//! Streams terrain meshes around the camera. A quadtree picks chunks that are finer near
//! the camera and coarser far away. Each chunk is meshed with surface nets from the sim's
//! terrain, flat shaded and colored by slope, height and land cover.
//!
//! Neighboring chunks overlap by a cell, and coarser chunks sit slightly lower, so the
//! finer one always shows and no cracks open between levels.

use std::collections::{HashMap, HashSet};

use glam::{Vec2, Vec3};
use zawnabout_sim::terrain::{CoverKind, Density, Terrain, smoothstep, void_bounds};

use crate::mesh::{MeshBuilder, Rgb};
use crate::render::{MeshId, Renderer};

/// Cells along a chunk side.
const N: usize = 32;
/// Cell size at the finest level (m).
const CELL0: f32 = 6.0;
const MAX_LOD: u8 = 5;
/// A node splits while the camera is within this many node sizes of it.
const SPLIT: f32 = 1.25;
/// Chunks entirely below this are under the opaque sea and never drawn.
const UNDERWATER: f32 = -2.5;

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

/// A built chunk: its outside mesh, its cave inside mesh, and bounding spheres.
#[derive(Clone, Copy, Default)]
struct Built {
    outer: Option<(MeshId, Vec3, f32)>,
    inner: Option<(MeshId, Vec3, f32)>,
}

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

#[derive(Default)]
pub struct TerrainMeshes {
    built: HashMap<Key, Built>,
    drawn: Vec<Key>,
}

impl TerrainMeshes {
    /// Meshes to draw this frame. Cave insides only draw when `caves` is set, since they
    /// are hidden from outside. Deep underground (`underground` near 1) the surface far
    /// overhead is hidden, so only nearby surface chunks draw.
    pub fn draw_list(&self, cam: Vec3, caves: bool, underground: f32) -> Vec<(MeshId, Vec3, f32)> {
        let deep = underground > 0.85;
        self.drawn
            .iter()
            .filter_map(|k| self.built.get(k))
            .flat_map(|b| {
                let outer = b
                    .outer
                    .filter(|(_, c, r)| !deep || c.distance(cam) - r < 300.0);
                [outer, b.inner.filter(|_| caves)]
            })
            .flatten()
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
            if k.lod > 0 && d < k.size() * SPLIT {
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
    pub fn update(&mut self, renderer: &mut Renderer, terrain: &Terrain, cam: Vec3, budget: usize) {
        let wanted = Self::select(terrain, cam);
        let mut left = budget;
        for k in &wanted {
            if left == 0 {
                break;
            }
            if !self.built.contains_key(k) {
                let (outer, inner) = build(terrain, *k);
                let built = Built {
                    outer: upload(renderer, &outer),
                    inner: upload(renderer, &inner),
                };
                self.built.insert(*k, built);
                left -= 1;
            }
        }
        let missing: Vec<Key> = wanted
            .iter()
            .copied()
            .filter(|k| !self.built.contains_key(k))
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
    }

    /// Whether every wanted chunk is built.
    pub fn settled(&self, terrain: &Terrain, cam: Vec3) -> bool {
        Self::select(terrain, cam)
            .iter()
            .all(|k| self.built.contains_key(k))
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
const CAVE_ROCK: [Rgb; 3] = [Rgb(0x8C8078), Rgb(0x7E746E), Rgb(0x988A7C)];
const CAVE_FLOOR: [Rgb; 2] = [Rgb(0x9C9070), Rgb(0x7F9A68)];

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

fn color(terrain: &Terrain, c: Vec3, n: Vec3, inside: bool) -> Rgb {
    let h = hash(c);
    let pick = |list: &[Rgb]| list[(h * list.len() as f32) as usize % list.len()];
    if inside {
        return if n.y > 0.65 {
            pick(&CAVE_FLOOR)
        } else {
            pick(&CAVE_ROCK)
        };
    }
    if c.y < 0.6 {
        return pick(&SAND);
    }
    if n.y < 0.55 {
        return pick(&ROCK);
    }
    if c.y < 3.5 && n.y > 0.8 {
        return pick(&SAND);
    }
    if c.y > 880.0 {
        return SNOW.mix(pick(&ROCK), h * 0.3);
    }
    let base = match terrain.cover(c.x, c.z) {
        Some(cv) => match (cv.kind, cv.density) {
            (CoverKind::Town, _) => TOWN.mix(pick(&GRASS), 0.15 + h * 0.15),
            (CoverKind::Forest, Density::Dense) => pick(&FOREST),
            (CoverKind::Forest, Density::Rain) => RAIN.mix(pick(&FOREST), h * 0.4),
            (CoverKind::Forest, Density::Open) => pick(&GRASS).mix(FOREST[0], 0.35),
            (CoverKind::Meadow, _) => MEADOW.mix(pick(&GRASS), h * 0.4),
        },
        None => pick(&GRASS),
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

/// Surface nets over one chunk. Returns the outside surface and the cave insides.
fn build(terrain: &Terrain, key: Key) -> (MeshBuilder, MeshBuilder) {
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
    let (mut cave_lo, mut cave_hi) = (f32::MAX, f32::MIN);
    for &i in &list {
        let b = void_bounds(&terrain.def.voids[i as usize]);
        cave_lo = cave_lo.min(b.min.y);
        cave_hi = cave_hi.max(b.max.y);
    }
    if hi < UNDERWATER && list.is_empty() {
        return (m, inner);
    }
    if !list.is_empty() {
        lo = lo.min(cave_lo);
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
    // Rock distance at every corner. Only corners near caves need the full 3D test.
    let ncy = ny + 1;
    let mut field = vec![0.0f32; nc * ncy * nc];
    for j in 0..nc {
        for k in 0..ncy {
            for i in 0..nc {
                let h = heights[j * nc + i];
                let y = y0 + k as f32 * cell;
                let mut d = y - h;
                if !list.is_empty() && y > cave_lo - pad && y < cave_hi + pad && y < h + pad {
                    d = terrain.sdf_with(at(i, k, j), h, &list);
                }
                field[(j * ncy + k) * nc + i] = d;
            }
        }
    }
    let sdf = |i: usize, k: usize, j: usize| -> f32 { field[(j * ncy + k) * nc + i] };
    let ground_at = |p: Vec3| -> f32 {
        let i = (((p.x - origin.x) / cell).round() as i64 + 1).clamp(0, nc as i64 - 1) as usize;
        let j = (((p.z - origin.y) / cell).round() as i64 + 1).clamp(0, nc as i64 - 1) as usize;
        heights[j * nc + i]
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
        let depth = ground_at(mid) - mid.y;
        // A cave wall sits on a cave space. A cliff face only looks deep, so it is not one.
        let inside = depth > 4.0 && !list.is_empty() && terrain.voids_sdf(mid, &list) < cell * 1.5;
        let depth = if inside { depth } else { 0.0 };
        if !inside && a.y.max(b.y).max(c.y).max(d.y) < UNDERWATER {
            return;
        }
        // Sky light fades with depth. The shader reads it from vertex alpha.
        let light = ((1.0 - smoothstep(4.0, 45.0, depth)) * 255.0) as u8;
        let out = if inside { &mut inner } else { &mut m };
        let (a, b, c, d) = (a - sink, b - sink, c - sink, d - sink);
        let n = (b - a).cross(c - a);
        let (a, b, c, d) = if n.dot(outward) < 0.0 {
            (a, d, c, b)
        } else {
            (a, b, c, d)
        };
        for (p, q, r) in [(a, b, c), (a, c, d)] {
            let n = (q - p).cross(r - p).normalize_or_zero();
            out.tri(p, q, r, color(terrain, (p + q + r) / 3.0, n, inside));
            let at = out.verts.len() - 3;
            for v in &mut out.verts[at..] {
                v.color[3] = light;
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
    (m, inner)
}
