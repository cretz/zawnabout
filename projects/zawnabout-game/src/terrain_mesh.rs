//! Streams terrain meshes around the camera. A quadtree picks chunks that are finer near
//! the camera and coarser far away. Each chunk is meshed with surface nets from the sim's
//! terrain, flat shaded and colored by slope, height and land cover.
//!
//! Neighboring chunks overlap by a cell, and coarser chunks sit slightly lower, so the
//! finer one always shows and no cracks open between levels.
//!
//! Far chunks change rarely and each would cost a draw, so their outside meshes merge,
//! one mesh per square of [`GROUP`], rebuilt only when the chunks in it change.

use std::collections::{HashMap, HashSet};

use glam::{Vec2, Vec3};
use zawnabout_sim::terrain::{CoverKind, Density, Terrain, noise, smoothstep, void_bounds};

use crate::mesh::{MeshBuilder, Rgb};
use crate::render::{MeshId, Renderer};

/// Cells along a chunk side.
const N: usize = 32;
/// Cell size at the finest level (m).
const CELL0: f32 = 6.0;
const MAX_LOD: u8 = 5;
/// A node splits while the camera is within this many node sizes of it.
const SPLIT: f32 = 0.65;
/// Coarse chunks far off split less readily than near ones, so far land, seen small,
/// costs less.
fn far_split(lod: u8) -> f32 {
    if lod >= 3 {
        0.7f32.powi(lod as i32 - 2)
    } else {
        1.0
    }
}

/// Chunks this coarse and coarser merge into one mesh per square of `GROUP` meters.
const MERGE_LOD: u8 = 2;
const GROUP: f32 = N as f32 * CELL0 * 8.0;
/// Chunks entirely below this are under opaque sea and never drawn.
const UNDERWATER: f32 = -crate::water::OPAQUE_DEPTH - 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub lod: u8,
    pub x: i32,
    pub z: i32,
}

impl Key {
    fn size(self) -> f32 {
        N as f32 * CELL0 * (1u32 << self.lod) as f32
    }

    fn min(self) -> Vec2 {
        Vec2::new(self.x as f32, self.z as f32) * self.size()
    }

    /// Whether this chunk and `other` cover any of the same ground.
    fn overlaps(self, other: Key) -> bool {
        let (a0, a1) = (self.min(), self.min() + self.size());
        let (b0, b1) = (other.min(), other.min() + other.size());
        a0.x < b1.x && b0.x < a1.x && a0.y < b1.y && b0.y < a1.y
    }
}

/// A built chunk: its outside mesh (with mouths and the first stretch into caves), its
/// deep cave mesh, and bounding spheres.
#[derive(Clone, Default)]
struct Built {
    outer: Option<(MeshId, Vec3, f32)>,
    /// A far chunk's outside mesh, kept here for its group to merge instead of uploaded.
    far: Option<MeshBuilder>,
    inner: Option<(MeshId, Vec3, f32)>,
    /// Caves deep under the ground left out, to mesh when the camera comes near them.
    skipped: Vec<u16>,
}

/// Cave walls shallower than this draw with the outside, so mouths show their tunnels
/// from afar instead of holes onto the sky.
const SHALLOW: f32 = 220.0;
/// How far away deep cave walls draw, even underground.
const CAVE_RANGE: f32 = 1800.0;
/// How far away deep cave walls draw from outside, when only peeking in a mouth.
const PEEK_RANGE: f32 = 400.0;

fn upload(renderer: &mut Renderer, mesh: &MeshBuilder) -> Option<(MeshId, Vec3, f32)> {
    if mesh.verts.is_empty() {
        return None;
    }
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &mesh.verts {
        let p = Vec3::from(v.pos);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    Some((
        renderer.add_mesh(mesh),
        (lo + hi) * 0.5,
        (hi - lo).length() * 0.5,
    ))
}

/// Far chunks drawn as one: which chunks, and their merged mesh.
struct Group {
    members: Vec<Key>,
    mesh: Option<(MeshId, Vec3, f32)>,
}

#[derive(Default)]
pub struct TerrainMeshes {
    built: HashMap<Key, Built>,
    drawn: Vec<Key>,
    groups: HashMap<(i32, i32), Group>,
}

impl Key {
    /// The square of far chunks this one merges into.
    fn group(self) -> (i32, i32) {
        let c = self.min() + Vec2::splat(self.size() * 0.5);
        ((c.x / GROUP).floor() as i32, (c.y / GROUP).floor() as i32)
    }
}

impl TerrainMeshes {
    /// Meshes to draw this frame. Cave insides only draw when `caves` is set, since they
    /// are hidden from outside. When `deep` underground the surface far overhead is
    /// hidden, so only nearby surface chunks draw.
    pub fn draw_list(
        &self,
        cam: Vec3,
        caves: bool,
        underground: f32,
        deep: bool,
    ) -> Vec<(MeshId, Vec3, f32)> {
        let cave_range = PEEK_RANGE + (CAVE_RANGE - PEEK_RANGE) * (underground * 2.0).min(1.0);
        let surface = |(_, c, r): &(MeshId, Vec3, f32)| !deep || c.distance(cam) - r < 300.0;
        self.drawn
            .iter()
            .filter_map(|k| self.built.get(k))
            .flat_map(|b| {
                let outer = b.outer.filter(surface);
                // Caves are dim and twisty, so far cave walls are not worth drawing.
                let inner = b
                    .inner
                    .filter(|(_, c, r)| caves && c.distance(cam) - r < cave_range);
                [outer, inner]
            })
            .flatten()
            // Far chunks are never near enough to show from deep underground.
            .chain(
                self.groups
                    .values()
                    .filter_map(|g| g.mesh)
                    .filter(|_| !deep),
            )
            .collect()
    }

    fn select(terrain: &Terrain, cam: Vec3) -> Vec<Key> {
        let reach = terrain.half_size() + 600.0;
        let root = Key {
            lod: MAX_LOD,
            x: 0,
            z: 0,
        }
        .size();
        let (lo, hi) = (
            (-reach / root).floor() as i32,
            (reach / root).floor() as i32,
        );
        // Deep in a cave, haze hides fine detail and a fine chunk overhead would mesh the
        // whole roof far above at full detail, so split less.
        let split = SPLIT * (1.0 - 0.4 * (1.0 - terrain.daylight(cam)));
        let mut out = Vec::new();
        let mut stack = Vec::new();
        for z in lo..=hi {
            for x in lo..=hi {
                stack.push(Key { lod: MAX_LOD, x, z });
            }
        }
        while let Some(k) = stack.pop() {
            let (a, b) = (k.min(), k.min() + k.size());
            if b.x < -reach || b.y < -reach || a.x > reach || a.y > reach {
                continue;
            }
            let flat = Vec2::new(cam.x, cam.z).clamp(a, b) - Vec2::new(cam.x, cam.z);
            let up = (cam.y - 400.0).max(0.0);
            let d = (flat.length_squared() + up * up).sqrt();
            if k.lod > 0 && d < k.size() * split * far_split(k.lod) {
                for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    stack.push(Key {
                        lod: k.lod - 1,
                        x: k.x * 2 + dx,
                        z: k.z * 2 + dz,
                    });
                }
            } else {
                out.push(k);
            }
        }
        let center = |k: Key| k.min() + k.size() * 0.5;
        let at = Vec2::new(cam.x, cam.z);
        out.sort_by(|a, b| center(*a).distance(at).total_cmp(&center(*b).distance(at)));
        out
    }

    /// Builds up to `budget` missing chunks, nearest first. Until a wanted chunk is
    /// built, whatever was drawn there before keeps drawing, so nothing pops out.
    /// Caves far under the ground only show from inside them, so chunks leave them out
    /// until the camera comes near one, and build much faster.
    pub fn update(&mut self, renderer: &mut Renderer, terrain: &Terrain, cam: Vec3, budget: usize) {
        let wanted = Self::select(terrain, cam);
        let near = |v: u16| near_void(terrain, v, cam);
        let ready = |b: Option<&Built>| b.is_some_and(|b| !b.skipped.iter().any(|v| near(*v)));
        let mut left = budget;
        for k in &wanted {
            if left == 0 {
                break;
            }
            if !ready(self.built.get(k)) {
                let (outer, inner, skipped) =
                    crate::prof::time("chunk", || build(terrain, *k, &near));
                let merge = k.lod >= MERGE_LOD && !outer.verts.is_empty();
                let built = Built {
                    outer: if merge {
                        None
                    } else {
                        upload(renderer, &outer)
                    },
                    far: merge.then_some(outer),
                    inner: upload(renderer, &inner),
                    skipped,
                };
                if let Some(old) = self.built.insert(*k, built) {
                    for (mesh, _, _) in [old.outer, old.inner].into_iter().flatten() {
                        renderer.free_mesh(mesh);
                    }
                }
                // A far chunk built afresh changes its square's merged mesh.
                if let Some(g) = self.groups.remove(&k.group())
                    && let Some((mesh, _, _)) = g.mesh
                {
                    renderer.free_mesh(mesh);
                }
                left -= 1;
            }
        }
        let missing: Vec<Key> = wanted
            .iter()
            .copied()
            .filter(|k| !ready(self.built.get(k)))
            .collect();
        let mut drawn: Vec<Key> = wanted
            .iter()
            .copied()
            .filter(|k| self.built.contains_key(k))
            .collect();
        for old in &self.drawn {
            if missing.iter().any(|m| m.overlaps(*old)) && !drawn.contains(old) {
                drawn.push(*old);
            }
        }
        let keep: HashSet<Key> = drawn.iter().copied().collect();
        self.built.retain(|k, id| {
            let stay = keep.contains(k);
            if !stay {
                for (mesh, _, _) in [id.outer, id.inner].into_iter().flatten() {
                    renderer.free_mesh(mesh);
                }
            }
            stay
        });
        self.drawn = drawn;
        self.merge(renderer);
    }

    /// Rebuilds the merged mesh of every square whose far chunks changed.
    fn merge(&mut self, renderer: &mut Renderer) {
        let mut want: HashMap<(i32, i32), Vec<Key>> = HashMap::new();
        for k in &self.drawn {
            if self.built.get(k).is_some_and(|b| b.far.is_some()) {
                want.entry(k.group()).or_default().push(*k);
            }
        }
        for members in want.values_mut() {
            members.sort_by_key(|k| (k.lod, k.x, k.z));
        }
        self.groups.retain(|cell, g| {
            let keep = want.get(cell).is_some_and(|m| *m == g.members);
            if !keep && let Some((mesh, _, _)) = g.mesh {
                renderer.free_mesh(mesh);
            }
            keep
        });
        for (cell, members) in want {
            if self.groups.contains_key(&cell) {
                continue;
            }
            let mut all = MeshBuilder::new();
            for k in &members {
                if let Some(far) = self.built.get(k).and_then(|b| b.far.as_ref()) {
                    all.verts.extend_from_slice(&far.verts);
                }
            }
            let mesh = upload(renderer, &all);
            self.groups.insert(cell, Group { members, mesh });
        }
    }

    /// Whether every wanted chunk is built, with the deep caves near the camera.
    pub fn settled(&self, terrain: &Terrain, cam: Vec3) -> bool {
        Self::select(terrain, cam).iter().all(|k| {
            self.built
                .get(k)
                .is_some_and(|b| !b.skipped.iter().any(|v| near_void(terrain, *v, cam)))
        })
    }
}

const GRASS: [Rgb; 4] = [Rgb(0x86C95A), Rgb(0x7CC152), Rgb(0x91CF63), Rgb(0x78B94E)];
const MEADOW: Rgb = Rgb(0xB8DC72);
const FOREST: [Rgb; 2] = [Rgb(0x5E9E48), Rgb(0x568F42)];
const RAIN: Rgb = Rgb(0x4A8A3E);
const SAND: [Rgb; 2] = [Rgb(0xEADBA2), Rgb(0xE0D094)];
const ROCK: [Rgb; 3] = [Rgb(0x9C8F84), Rgb(0x8E8379), Rgb(0xA89C8E)];
const ALPINE: Rgb = Rgb(0xA6B47E);
const SNOW: Rgb = Rgb(0xF0F2F5);
const TOWN: Rgb = Rgb(0xD9CBA8);
const CAVE_ROCK: [Rgb; 3] = [Rgb(0x947760), Rgb(0x806856), Rgb(0xA0846A)];
const CAVE_FLOOR: [Rgb; 2] = [Rgb(0x9C9070), Rgb(0x7F9A68)];
/// Dressed stone along canals.
const QUAY: [Rgb; 2] = [Rgb(0xC2B59E), Rgb(0xB5A890)];
/// Sand and pebbles under water.
const BED: [Rgb; 3] = [Rgb(0xD9C99A), Rgb(0xC9BC92), Rgb(0xB8B08C)];

fn hash(p: Vec3) -> f32 {
    let x = (p.x * 0.37).floor() as i32;
    let z = (p.z * 0.37).floor() as i32;
    let y = (p.y * 0.37).floor() as i32;
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (z as u32).wrapping_mul(0xD816_3841)
        ^ (y as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65536.0
}

/// The grass color at a spot out in the open, or none where it is sand, rock, snow, town
/// or quay.
pub fn lawn(terrain: &Terrain, c: Vec3, n: Vec3) -> Option<Rgb> {
    if c.y < 3.5 || c.y > 680.0 || n.y < 0.75 || terrain.quay(c.x, c.z) {
        return None;
    }
    let h = hash(c);
    let pick = |list: &[Rgb]| list[(h * list.len() as f32) as usize % list.len()];
    Some(match terrain.cover(c.x, c.z) {
        Some(cv) => match (cv.kind, cv.density) {
            (CoverKind::Town | CoverKind::Snow, _) => return None,
            (CoverKind::Forest, Density::Dense) => pick(&FOREST),
            (CoverKind::Forest, Density::Rain) => RAIN.mix(pick(&FOREST), h * 0.4),
            (CoverKind::Forest, Density::Open) => pick(&GRASS).mix(FOREST[0], 0.35),
            (CoverKind::Meadow, _) => MEADOW.mix(pick(&GRASS), h * 0.4),
        },
        None => pick(&GRASS),
    })
}

/// Cream wall tiles and the stone floor of a tiled hall.
const TILE: [Rgb; 2] = [Rgb(0xEFE8D8), Rgb(0xE6DECB)];
const TILE_FLOOR: Rgb = Rgb(0x9A958C);

/// The ground's color at `c`. `tiled` when it walls a tiled hall.
fn color(terrain: &Terrain, c: Vec3, n: Vec3, inside: bool, wet: bool, tiled: bool) -> Rgb {
    let h = hash(c);
    let pick = |list: &[Rgb]| list[(h * list.len() as f32) as usize % list.len()];
    // Cold seas, rivers, quays and town canals get a grey pebbly bed under the water, too pale
    // for the shader's coral, which needs warm sand.
    let plain = || {
        c.y < -0.5
            && (wet
                || terrain.climate(c.x, c.z) == 3
                || terrain
                    .cover(c.x, c.z)
                    .is_some_and(|cv| cv.kind == CoverKind::Town))
    };
    let pebbles = |sand: Rgb| sand.mix(Rgb(0xB8B6AE), 0.85);
    if !inside && terrain.quay(c.x, c.z) {
        return if c.y < -0.5 {
            pebbles(pick(&QUAY))
        } else {
            pick(&QUAY)
        };
    }
    if wet {
        return if plain() {
            pebbles(pick(&BED))
        } else {
            pick(&BED)
        };
    }
    if tiled {
        if n.y > 0.7 {
            return TILE_FLOOR.mix(Rgb(0x000000), h * 0.05);
        }
        return pick(&TILE);
    }
    if inside {
        // Floors gather moss in broad patches and fade into the walls with the slope, so
        // the ground reads as one surface rather than a checkerboard of triangles.
        let moss = smoothstep(-0.15, 0.35, noise(97, c.x / 80.0, c.z / 80.0));
        let floor = CAVE_FLOOR[0].mix(CAVE_FLOOR[1], moss);
        let rock = CAVE_ROCK[0].mix(CAVE_ROCK[2], noise(98, c.y / 40.0, c.x / 90.0) * 0.5 + 0.5);
        let ground = rock.mix(floor, smoothstep(0.55, 0.85, n.y));
        return ground.mix(Rgb(0x000000), h * 0.07);
    }
    if c.y < 0.6 {
        return if plain() {
            pebbles(pick(&SAND))
        } else {
            pick(&SAND)
        };
    }
    // Snow lies on ski runs even where they steepen, rock only shows on real drops.
    let snow = n.y > 0.4
        && terrain
            .cover(c.x, c.z)
            .is_some_and(|cv| cv.kind == CoverKind::Snow);
    if snow {
        // Groomed runs: bright, faintly blue in the hollows.
        return SNOW.mix(Rgb(0xD6E0EE), h * 0.5);
    }
    if n.y < 0.55 {
        return pick(&ROCK);
    }
    if c.y < 3.5 && n.y > 0.8 {
        return pick(&SAND);
    }
    // Cold islands are snowy above a far lower line.
    let climate = terrain.climate(c.x, c.z);
    let snowline = if climate == 3 {
        230.0 + h * 40.0
    } else {
        880.0
    };
    if c.y > snowline {
        return SNOW.mix(pick(&ROCK), h * 0.3);
    }
    let base = match terrain.cover(c.x, c.z) {
        Some(cv) => match (cv.kind, cv.density) {
            (CoverKind::Town, _) => TOWN.mix(pick(&GRASS), 0.15 + h * 0.15),
            (CoverKind::Snow, _) => SNOW,
            (CoverKind::Forest, Density::Dense) => pick(&FOREST),
            (CoverKind::Forest, Density::Rain) => RAIN.mix(pick(&FOREST), h * 0.4),
            (CoverKind::Forest, Density::Open) => pick(&GRASS).mix(FOREST[0], 0.35),
            (CoverKind::Meadow, _) => MEADOW.mix(pick(&GRASS), h * 0.4),
        },
        None => pick(&GRASS),
    };
    // Each island's own climate: deep greens in the rainforest, golden grass on the dry
    // island, pale frosty grass on the cold one. Towns keep their colors.
    let town = terrain
        .cover(c.x, c.z)
        .is_some_and(|cv| cv.kind == CoverKind::Town);
    let base = match (climate, town) {
        (_, true) | (0, _) => base,
        (1, _) => base.mix(Rgb(0x2F7A34), 0.35),
        (2, _) => base.mix(Rgb(0xD4B862).mix(Rgb(0xC9A452), h), 0.6),
        _ => base.mix(Rgb(0xB9C0AA), 0.68),
    };
    let base = if c.y > 680.0 {
        base.mix(ALPINE, ((c.y - 680.0) / 150.0).clamp(0.0, 1.0))
    } else {
        base
    };
    if n.y < 0.75 {
        base.mix(pick(&ROCK), 0.55)
    } else {
        base
    }
}

/// Surface nets over one chunk. Returns the outside surface, with mouths and shallow cave
/// walls, and the deep cave walls.
/// Whether the camera is close enough to cave space `v` to need its walls: in it, or
/// near enough to fly in soon.
fn near_void(terrain: &Terrain, v: u16, cam: Vec3) -> bool {
    let void = &terrain.def.voids[v as usize];
    let b = void_bounds(void);
    // From inside one cave you can see a long way across into the next, so underground
    // the reach is much longer.
    let reach = if terrain.daylight(cam) < 0.5 {
        DEEP_SEEN
    } else {
        DEEP_NEAR
    };
    (b.min - cam).max(cam - b.max).max(Vec3::ZERO).length() < reach
        && zawnabout_sim::terrain::void_sdf(void, cam) < reach
}

/// How near a deep cave the camera must come before chunks mesh its walls (m). Cave
/// insides start drawing at 150m, so this leaves time to build them.
const DEEP_NEAR: f32 = 300.0;
/// How near the camera must be when it is underground itself (m).
const DEEP_SEEN: f32 = 1200.0;

/// A chunk's outside and deep cave meshes, and the deep caves it left out because the
/// camera is not near them (`near` says which are).
fn build(
    terrain: &Terrain,
    key: Key,
    near: &dyn Fn(u16) -> bool,
) -> (MeshBuilder, MeshBuilder, Vec<u16>) {
    let mut m = MeshBuilder::new();
    let mut inner = MeshBuilder::new();
    let cell = CELL0 * (1u32 << key.lod) as f32;
    let origin = key.min();
    // Corners run one cell past each side, so neighbors overlap.
    let nc = N + 3;
    let corner_xz = |i: usize| -> f32 { (i as f32 - 1.0) * cell };
    let mut heights = vec![0.0f32; nc * nc];
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for j in 0..nc {
        for i in 0..nc {
            let h = terrain.height(origin.x + corner_xz(i), origin.y + corner_xz(j));
            heights[j * nc + i] = h;
            lo = lo.min(h);
            hi = hi.max(h);
        }
    }
    let pad = cell * 2.0;
    let size = key.size();
    let list = terrain.voids_in(
        Vec3::new(origin.x - pad, -1.0e5, origin.y - pad),
        Vec3::new(origin.x + size + pad, 1.0e5, origin.y + size + pad),
    );
    if hi < UNDERWATER && list.is_empty() {
        return (m, inner, Vec::new());
    }
    let tiled: Vec<&zawnabout_sim::terrain::Void> = list
        .iter()
        .map(|&i| &terrain.def.voids[i as usize])
        .filter(|v| v.decor == zawnabout_sim::terrain::Decor::Tiled)
        .collect();
    // Only bands of height where a surface can be get meshed: round the ground, and round
    // each cave space. The rock between them is solid, so a cavern far below costs only
    // its own band, not a whole column of rock down to it.
    let mut bands: Vec<(f32, f32, Vec<u16>)> = vec![(lo - pad * 2.0, hi + pad, Vec::new())];
    for &i in &list {
        let b = void_bounds(&terrain.def.voids[i as usize]);
        bands.push((b.min.y - pad, b.max.y.min(hi) + pad, vec![i]));
    }
    bands.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32, Vec<u16>)> = Vec::new();
    for (a, b, v) in bands {
        match merged.last_mut() {
            Some(last) if a <= last.1 + cell * 3.0 => {
                last.1 = last.1.max(b);
                last.2.extend(v);
            }
            _ => merged.push((a, b, v)),
        }
    }
    let mut skipped = Vec::new();
    // For each column of corners, the cave spaces that might reach it and how high and low
    // they go, so rock far from any of them skips the costly 3D test.
    let bounds: Vec<(u16, zawnabout_sim::terrain::Bounds)> = list
        .iter()
        .map(|&i| (i, void_bounds(&terrain.def.voids[i as usize])))
        .collect();
    let mut col_voids: Vec<Vec<u16>> = vec![Vec::new(); nc * nc];
    let mut col_spans: Vec<Vec<(f32, f32)>> = vec![Vec::new(); nc * nc];
    if !bounds.is_empty() {
        for j in 0..nc {
            for i in 0..nc {
                let (x, z) = (origin.x + corner_xz(i), origin.y + corner_xz(j));
                let at = j * nc + i;
                for (v, b) in &bounds {
                    if x > b.min.x - pad
                        && x < b.max.x + pad
                        && z > b.min.z - pad
                        && z < b.max.z + pad
                    {
                        col_voids[at].push(*v);
                        col_spans[at].push((b.min.y - pad, b.max.y + pad));
                    }
                }
            }
        }
    }
    // Bands of cave far under the ground only ever join the deep mesh, so they wait
    // until the camera comes near one of their caves.
    let ground = lo;
    for (lo, hi, voids) in merged {
        if hi < ground - SHALLOW && !voids.iter().any(|v| near(*v)) {
            skipped.extend(voids);
            continue;
        }
        let y0 = (lo / cell).floor() * cell - cell;
        let ny = ((hi - y0) / cell).ceil() as usize + 2;
        let at = |i: usize, k: usize, j: usize| -> Vec3 {
            Vec3::new(
                origin.x + corner_xz(i),
                y0 + k as f32 * cell,
                origin.y + corner_xz(j),
            )
        };
        // Rock distance at every corner. Only corners near caves need the full 3D test, and
        // of those only the ones near a surface: a coarse pass first finds where that is.
        let ncy = ny + 1;
        let mut field = vec![0.0f32; nc * ncy * nc];
        let cave_zone = |col: usize, y: f32, h: f32| {
            y < h + pad && col_spans[col].iter().any(|&(lo, hi)| y > lo && y < hi)
        };
        const C: usize = 4;
        let coarse = |n: usize| (n - 1).div_ceil(C) + 1;
        let fine = |c: usize, n: usize| (c * C).min(n - 1);
        let (cx, cy) = (coarse(nc), coarse(ncy));
        let mut rough = vec![0.0f32; cx * cy * cx];
        for cj in 0..cx {
            for ck in 0..cy {
                for ci in 0..cx {
                    let (i, k, j) = (fine(ci, nc), fine(ck, ncy), fine(cj, nc));
                    let h = heights[j * nc + i];
                    let y = y0 + k as f32 * cell;
                    let col = j * nc + i;
                    rough[(cj * cy + ck) * cx + ci] = if cave_zone(col, y, h) {
                        terrain.sdf_with(at(i, k, j), h, &col_voids[col])
                    } else {
                        y - h
                    };
                }
            }
        }
        // Coarse values are only trusted this far from a surface.
        let band = C as f32 * cell * 1.5;
        let lerp_axis = |f: usize, n: usize| {
            let c = (f / C).min(coarse(n) - 2);
            let (a, b) = (fine(c, n), fine(c + 1, n));
            (c, (f - a) as f32 / (b - a).max(1) as f32)
        };
        for j in 0..nc {
            let (cj, tj) = lerp_axis(j, nc);
            for k in 0..ncy {
                let (ck, tk) = lerp_axis(k, ncy);
                for i in 0..nc {
                    let h = heights[j * nc + i];
                    let y = y0 + k as f32 * cell;
                    let col = j * nc + i;
                    let mut d = y - h;
                    if cave_zone(col, y, h) {
                        let (ci, ti) = lerp_axis(i, nc);
                        let r = |a: usize, b: usize, c: usize| {
                            rough[((cj + c) * cy + ck + b) * cx + ci + a]
                        };
                        let mixf = |a: f32, b: f32, t: f32| a + (b - a) * t;
                        let guess = mixf(
                            mixf(
                                mixf(r(0, 0, 0), r(1, 0, 0), ti),
                                mixf(r(0, 1, 0), r(1, 1, 0), ti),
                                tk,
                            ),
                            mixf(
                                mixf(r(0, 0, 1), r(1, 0, 1), ti),
                                mixf(r(0, 1, 1), r(1, 1, 1), ti),
                                tk,
                            ),
                            tj,
                        );
                        // Trust the guess only when the whole coarse cell is far on one side.
                        let corners = [
                            r(0, 0, 0),
                            r(1, 0, 0),
                            r(0, 1, 0),
                            r(1, 1, 0),
                            r(0, 0, 1),
                            r(1, 0, 1),
                            r(0, 1, 1),
                            r(1, 1, 1),
                        ];
                        let clear =
                            corners.iter().all(|c| *c > band) || corners.iter().all(|c| *c < -band);
                        d = if clear {
                            guess
                        } else {
                            terrain.sdf_with(at(i, k, j), h, &col_voids[col])
                        };
                    }
                    field[(j * ncy + k) * nc + i] = d;
                }
            }
        }
        let sdf = |i: usize, k: usize, j: usize| -> f32 { field[(j * ncy + k) * nc + i] };
        // For each column, the lowest corner with open air all the way up to the sky. Below
        // the ground it means a sinkhole, shaft or skylight opens there.
        let mut open_from = vec![ncy; nc * nc];
        let mut skylight = false;
        for j in 0..nc {
            for i in 0..nc {
                let mut k = ncy;
                while k > 0 && sdf(i, k - 1, j) > 0.0 {
                    k -= 1;
                }
                // A band whose top is rock (a cave band under the ground) is closed to
                // the sky in that column.
                if k == ncy {
                    continue;
                }
                open_from[j * nc + i] = k;
                skylight |= y0 + k as f32 * cell < heights[j * nc + i] - cell * 2.0;
            }
        }
        // Whether open sky lies straight above `p`.
        let sky_above = |p: Vec3| {
            let i = ((p.x - origin.x) / cell + 1.0)
                .round()
                .clamp(0.0, (nc - 1) as f32) as usize;
            let j = ((p.z - origin.y) / cell + 1.0)
                .round()
                .clamp(0.0, (nc - 1) as f32) as usize;
            let k = ((p.y - y0) / cell).ceil().max(0.0) as usize;
            k >= open_from[j * nc + i]
        };

        // One vertex per cell that the surface passes through, at the mean edge crossing.
        let nx = nc - 1;
        let mut verts = vec![u32::MAX; nx * ny * nx];
        let mut points: Vec<Vec3> = Vec::new();
        let edges = [
            ((0, 0, 0), (1, 0, 0)),
            ((0, 1, 0), (1, 1, 0)),
            ((0, 0, 1), (1, 0, 1)),
            ((0, 1, 1), (1, 1, 1)),
            ((0, 0, 0), (0, 1, 0)),
            ((1, 0, 0), (1, 1, 0)),
            ((0, 0, 1), (0, 1, 1)),
            ((1, 0, 1), (1, 1, 1)),
            ((0, 0, 0), (0, 0, 1)),
            ((1, 0, 0), (1, 0, 1)),
            ((0, 1, 0), (0, 1, 1)),
            ((1, 1, 0), (1, 1, 1)),
        ];
        for j in 0..nx {
            for k in 0..ny {
                for i in 0..nx {
                    // Most cells are all rock or all air: no surface, nothing to do.
                    let first = sdf(i, k, j) < 0.0;
                    let mixed = [
                        (1, 0, 0),
                        (0, 1, 0),
                        (1, 1, 0),
                        (0, 0, 1),
                        (1, 0, 1),
                        (0, 1, 1),
                        (1, 1, 1),
                    ]
                    .iter()
                    .any(|&(a, b, c)| (sdf(i + a, k + b, j + c) < 0.0) != first);
                    if !mixed {
                        continue;
                    }
                    let mut sum = Vec3::ZERO;
                    let mut count = 0;
                    for ((ai, ak, aj), (bi, bk, bj)) in edges {
                        let (da, db) = (sdf(i + ai, k + ak, j + aj), sdf(i + bi, k + bk, j + bj));
                        if (da < 0.0) != (db < 0.0) {
                            let t = da / (da - db);
                            sum += at(i + ai, k + ak, j + aj).lerp(at(i + bi, k + bk, j + bj), t);
                            count += 1;
                        }
                    }
                    if count > 0 {
                        verts[(j * ny + k) * nx + i] = points.len() as u32;
                        points.push(sum / count as f32);
                    }
                }
            }
        }
        let vert = |i: usize, k: usize, j: usize| -> Option<Vec3> {
            let v = verts[(j * ny + k) * nx + i];
            (v != u32::MAX).then(|| points[v as usize])
        };
        let sink = Vec3::Y * (0.3 * ((1u32 << key.lod) - 1) as f32);
        let mut quad = |q: [Option<Vec3>; 4], outward: Vec3| {
            let [Some(a), Some(b), Some(c), Some(d)] = q else {
                return;
            };
            let mid = (a + b + c + d) * 0.25;
            // A cave wall sits on a cave space well under the real ground. A cliff face or a
            // slope over a shallow cave only looks deep on the coarse grid, so it is not one.
            let depth = if list.is_empty() {
                0.0
            } else {
                terrain.height(mid.x, mid.z) - mid.y
            };
            let near = {
                let i = ((mid.x - origin.x) / cell + 1.0)
                    .round()
                    .clamp(0.0, (nc - 1) as f32) as usize;
                let j = ((mid.z - origin.y) / cell + 1.0)
                    .round()
                    .clamp(0.0, (nc - 1) as f32) as usize;
                &col_voids[j * nc + i]
            };
            let cave = depth > 4.0 + cell * 0.5 && terrain.voids_sdf(mid, near) < cell * 0.75 + 2.0;
            // Walls of a tiled hall.
            let tile = cave
                && tiled
                    .iter()
                    .any(|v| zawnabout_sim::terrain::void_sdf(v, mid) < cell * 0.75 + 2.0);
            // Rock under a skylight is lit and seen from outside like the open ground.
            let lit = cave && sky_above(mid);
            let dark = ((1.0 - smoothstep(4.0, 45.0, depth)) * 255.0) as u8;
            let depth = if cave && !lit { depth } else { 0.0 };
            // The first stretch into a cave stays with the outside, so mouths look like
            // openings from afar instead of holes onto the sky.
            let inside = cave && depth > 60.0;
            // Open ground deep under the sea is hidden by the water, but cave walls down there
            // (a subway, a sea cave) are seen from inside.
            if !cave && a.y.max(b.y).max(c.y).max(d.y) < UNDERWATER {
                return;
            }
            // Sky light fades with depth. The shader reads it from vertex alpha.
            let light = ((1.0 - smoothstep(4.0, 45.0, depth)) * 255.0) as u8;
            // How high each corner sits above the water near here, for the shore in the shader.
            // Caves only know their own pools and rivers, never the sea or lakes far above.
            let level = if cave {
                terrain.cave_water(mid.x, mid.z, mid.y - 13.0)
            } else {
                terrain.water(mid.x, mid.z, mid.y - 13.0)
            };
            // Shallow walls, and walls of a cave that opens to the sky in this chunk, can be
            // seen from outside, so they join the outside mesh.
            let out = if !inside || depth < SHALLOW || skylight {
                &mut m
            } else {
                &mut inner
            };
            let (a, b, c, d) = (a - sink, b - sink, c - sink, d - sink);
            let n = (b - a).cross(c - a);
            let (a, b, c, d) = if n.dot(outward) < 0.0 {
                (a, d, c, b)
            } else {
                (a, b, c, d)
            };
            for (p, q, r) in [(a, b, c), (a, c, d)] {
                let n = (q - p).cross(r - p).normalize_or_zero();
                let wet = level.is_some_and(|l| (p.y + q.y + r.y) / 3.0 < l);
                out.tri(
                    p,
                    q,
                    r,
                    color(terrain, (p + q + r) / 3.0, n, cave, wet, tile),
                );
                let at = out.verts.len() - 3;
                for v in &mut out.verts[at..] {
                    // Under a skylight each corner asks for itself, so the patch of daylight
                    // fades out at its edge instead of ending in a hard square.
                    v.color[3] = if cave {
                        if sky_above(Vec3::from(v.pos) + sink) {
                            dark.max(170)
                        } else {
                            dark
                        }
                    } else {
                        light
                    };
                    v.normal[3] = match level {
                        Some(l) => ((v.pos[1] + sink.y - l) * 10.0).clamp(-127.0, 127.0) as i8,
                        None => 127,
                    };
                }
            }
        };
        // A quad for each grid edge the surface crosses, joining the four cells around it.
        for j in 1..nx {
            for k in 1..ny {
                for i in 1..nx {
                    let s = sdf(i, k, j);
                    let solid = s < 0.0;
                    if (sdf(i + 1, k, j) < 0.0) != solid {
                        let out = if solid { Vec3::X } else { -Vec3::X };
                        quad(
                            [
                                vert(i, k - 1, j - 1),
                                vert(i, k, j - 1),
                                vert(i, k, j),
                                vert(i, k - 1, j),
                            ],
                            out,
                        );
                    }
                    if (sdf(i, k + 1, j) < 0.0) != solid {
                        let out = if solid { Vec3::Y } else { -Vec3::Y };
                        quad(
                            [
                                vert(i - 1, k, j - 1),
                                vert(i, k, j - 1),
                                vert(i, k, j),
                                vert(i - 1, k, j),
                            ],
                            out,
                        );
                    }
                    if (sdf(i, k, j + 1) < 0.0) != solid {
                        let out = if solid { Vec3::Z } else { -Vec3::Z };
                        quad(
                            [
                                vert(i - 1, k - 1, j),
                                vert(i, k - 1, j),
                                vert(i, k, j),
                                vert(i - 1, k, j),
                            ],
                            out,
                        );
                    }
                }
            }
        }
    }
    (m, inner, skipped)
}
