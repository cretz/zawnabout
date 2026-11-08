//! Turns game state into frames: registers every mesh once, then builds each frame's
//! draw list from the camera and dragon poses.

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::PropKind;

use crate::game::Game;
use crate::mesh::{MeshBuilder, Rgb};
use crate::models;
use crate::pose::Part;
use crate::render::{Cull, Frame, Group, Instance, Lighting, MeshId, Renderer};
use crate::terrain_mesh::TerrainMeshes;

struct DragonIds {
    body: MeshId,
    neck: MeshId,
    head: MeshId,
    eye: MeshId,
    tail: MeshId,
    plume: MeshId,
    whisker: MeshId,
    mane: MeshId,
    leg: MeshId,
    arm: [MeshId; 2],
    hand: [MeshId; 2],
    star: MeshId,
}

pub struct Gfx {
    pub renderer: Renderer,
    /// What the last frame drawn while the game was halted showed: nothing moves then,
    /// so the picture can stay up until the HUD over it changes.
    still: Option<u64>,
    /// Treasure models by kind, the glint star over a find, and the hoard's coin heap.
    treasures: Vec<(&'static str, MeshId)>,
    star: MeshId,
    heap: MeshId,
    /// The postcard pictures in the atlas, as of this version of them.
    pics_version: u32,
    /// Talk bubbles: plain, and for someone with a favor.
    bubbles: [MeshId; 2],
    /// The journal on its stand in the lair.
    lectern: MeshId,
    /// Bunting flags, paper lanterns and their poles, for what finished sets bring.
    flag: MeshId,
    paper_lantern: MeshId,
    pole: MeshId,
    /// A story's lamp: an old iron lamp on a stone post.
    brazier: MeshId,
    /// A passenger's basket seat, and the umbrella they float down under.
    saddle: MeshId,
    umbrella: MeshId,
    dragon: DragonIds,
    /// Each named dragon's own colors, built once they are known.
    kin_meshes: Vec<DragonIds>,
    terrain: TerrainMeshes,
    /// Water meshes with bounding spheres, and whether each is in a cave.
    water: Vec<(MeshId, Vec3, f32, bool)>,
    spray: MeshId,
    falls: Vec<(Vec3, f32)>,
    /// What can be heard where the camera is.
    listener: crate::audio::Listener,
    flame: MeshId,
    ember: MeshId,
    /// Light streaming down sky openings into caves, with bounding spheres and where
    /// each lands.
    shafts: Vec<(MeshId, Vec3, f32, Vec3)>,
    /// Big crystals that light their surroundings.
    glowers: Vec<Light>,
    life: crate::life::Life,
    cave_life: crate::cave_life::CaveLife,
    park_life: crate::park_life::ParkLife,
    sea_life: crate::sea_life::SeaLife,
    wildlife: crate::wildlife::Wildlife,
    weather: crate::weather::Weather,
    /// Dark plugs a little way into every cave mouth, so from outside a mouth looks like
    /// darkness instead of a hole onto the sky where the deep walls are not drawn.
    plugs: Vec<Mat4>,
    /// Where each cave mouth opens to the sky. Near one, the outside draws in full even
    /// from inside, so the view out is the real world.
    mouths: Vec<Vec3>,
    plug: MeshId,
    /// Grass tufts around the camera, rebuilt when it moves to another cell.
    tuft: MeshId,
    tufts: std::cell::RefCell<Lawn>,
    /// The sparser ring of tufts further out, rebuilt less often.
    far_tufts: std::cell::RefCell<Lawn>,
    /// One mesh per vehicle, in the order of the sim's traffic.
    vehicles: Vec<MeshId>,
    /// Every lamp's head and foot, and whether it is underground where it always burns.
    lamps: Vec<(Vec3, Vec3, bool)>,
    /// Every landmark's lamps and bulbs: where, how big, whether they burn all day, and
    /// whether they are a fair's colored bulbs that chase round.
    bulbs: Vec<(Vec3, f32, bool, bool)>,
    pool: MeshId,
    /// Terrain chunk steps per frame at most. Steps stop sooner once
    /// [`CHUNK_SECS`] have gone by.
    pub chunk_budget: usize,
    /// The HUD's world map, drawn on another thread the first time the HUD shows, and
    /// whether it is in the atlas yet.
    hud_map: (Option<std::sync::mpsc::Receiver<Vec<u8>>>, bool),
    /// The journal's smaller map, drawn a few rows a frame where the HUD's is not.
    book_map: Option<crate::map::MapJob>,
}

/// A point light: position, reach and color.
type Light = (Vec3, f32, Vec3);

/// Where to put a dark plug in each cave mouth: in from the open end along the tunnel to
/// where its walls are deep enough to stop drawing from outside, so from afar you see the
/// tunnel run back into darkness.
fn mouth_plugs(terrain: &zawnabout_sim::terrain::Terrain) -> (Vec<Mat4>, Vec<Vec3>) {
    use zawnabout_sim::terrain::VoidShape;
    let below = |p: Vec3| terrain.height(p.x, p.z) - p.y;
    // Tunnels are chains of short pieces named `id#k`: gather each chain's line.
    let mut chains: Vec<(&str, Vec<Vec3>, f32)> = Vec::new();
    for v in terrain.def.voids {
        if v.shape != VoidShape::Tube {
            continue;
        }
        let name = v.id.split('#').next().unwrap_or(v.id);
        let (a, b) = (Vec3::from(v.a), Vec3::from(v.b));
        match chains.last_mut() {
            Some((n, line, _))
                if *n == name && line.last().is_some_and(|l| l.distance(a) < 1.0) =>
            {
                line.push(b)
            }
            _ => chains.push((name, vec![a, b], v.r)),
        }
    }
    let mut out = Vec::new();
    let mut mouths = Vec::new();
    for (_, mut line, r) in chains {
        // Walk in from whichever end is open to the sky.
        if below(line[line.len() - 1]) > 0.0 {
            line.reverse();
        }
        if below(line[line.len() - 1]) > 0.0 || below(line[0]) <= 0.0 {
            continue;
        }
        mouths.push(line[line.len() - 1]);
        let mut at = line[0];
        for p in line.iter().rev() {
            at = *p;
            if below(*p) > 210.0 {
                break;
            }
        }
        out.push(Mat4::from_scale_rotation_translation(
            Vec3::splat(r * 1.35),
            Quat::IDENTITY,
            at,
        ));
    }
    (out, mouths)
}

/// Bounding sphere of a mesh.
fn bounds(mesh: &crate::mesh::MeshBuilder) -> (Vec3, f32) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &mesh.verts {
        let p = Vec3::from(v.pos);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    ((lo + hi) * 0.5, (hi - lo).length() * 0.5)
}

/// Lamps on the fronts of terraces and flats, by every third door, so city streets glow
/// at night: the lamp, the ground under it, and never underground.
fn wall_lamps(world: &zawnabout_sim::World) -> impl Iterator<Item = (Vec3, Vec3, bool)> + '_ {
    use zawnabout_sim::town::Kind;
    world
        .buildings
        .iter()
        .filter(|b| matches!(b.kind, Kind::Row | Kind::Flat) && b.seed % 3 == 0)
        .filter(|b| world.terrain.daylight(b.pos + Vec3::Y) >= 0.5)
        .map(|b| {
            let f = b.front();
            let face = b.pos + Vec3::new(f.x, 0.0, f.y) * (b.size.z * 0.5 + 0.3);
            let side = Vec3::new(b.axis.x, 0.0, b.axis.y) * b.size.x * 0.3;
            let lamp = face + side + Vec3::Y * 3.2;
            let foot = Vec3::new(lamp.x, world.terrain.height(lamp.x, lamp.z) + 0.25, lamp.z);
            (lamp, foot, false)
        })
}

/// Props are batched per square of this size (m), so far squares can be skipped.
/// Time a frame spends building terrain before leaving the rest for the next (s). At
/// least one step always runs, so terrain keeps coming on any device.
const CHUNK_SECS: f64 = 0.004;
const PROP_TILE: f32 = 1024.0;
const CLOUD_TILE: f32 = 3000.0;
/// Tiles per side of a far group, and how far out (times the detail range) the group
/// draws as one.
const GROUP: i32 = 4;
const GROUP_RANGE: f32 = 1.5;
/// Heavy buildings show their full detail within this many times the detail range of
/// their own bounds.
const HEAVY_DETAIL: f32 = 0.4;
/// Signs' letters draw out to this many times the prop range.
const LETTERS_REACH: f32 = 0.35;
/// People farther than this (m) draw as a speck.
const PERSON_FAR: f32 = 140.0;
/// Merged groups farther than this many times the detail range draw only their big
/// things.
const DISTANT: f32 = 3.0;
/// Flowers, bushes and small rocks bake per square of this size (m), and draw out to
/// this many times the prop range.
const SMALL_TILE: f32 = 256.0;
const SMALL_REACH: f32 = 0.14;
/// The journal's own map, when there is no HUD map, in pixels square.
const BOOK_MAP: u32 = 512;
/// Water merges per square of this size (m).
const WATER_TILE: f32 = 4096.0;

use zawnabout_sim::traffic::CAR_KINDS;

/// The car mesh painted white, which the tint colors.
const CAR_WHITE: u8 = 3;
/// Fairground colors as tints over a white ferris car or tour carriage: red, blue,
/// yellow, green, pink.
const FAIR_PAINT: [Vec3; 5] = [
    Vec3::new(0.752, 0.092, 0.066),
    Vec3::new(0.066, 0.284, 0.586),
    Vec3::new(0.891, 0.542, 0.074),
    Vec3::new(0.148, 0.530, 0.198),
    Vec3::new(0.812, 0.259, 0.442),
];
/// Car paints as tints over white: red, blue, yellow, white, green, pink.
const CAR_PAINT: [Vec3; 6] = [
    Vec3::new(0.78, 0.13, 0.08),
    Vec3::new(0.08, 0.3, 0.62),
    Vec3::new(0.95, 0.56, 0.08),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(0.17, 0.55, 0.22),
    Vec3::new(0.85, 0.28, 0.48),
];

fn linear(c: Rgb, scale: f32) -> Vec3 {
    let f = |s: u32| ((s & 0xFF) as f32 / 255.0).powf(2.2) * scale;
    Vec3::new(f(c.0 >> 16), f(c.0 >> 8), f(c.0))
}

impl Gfx {
    pub fn new(mut renderer: Renderer, game: &Game) -> Self {
        let world = &game.sim.world;
        // Water merges per tile, one draw each, instead of one per lake, river and patch
        // of sea.
        let mut water_tiles: HashMap<((i32, i32), bool), MeshBuilder> = HashMap::new();
        for part in crate::water::all(&world.terrain) {
            let (center, _) = bounds(&part.mesh);
            let key = (
                (
                    (center.x / WATER_TILE).floor() as i32,
                    (center.z / WATER_TILE).floor() as i32,
                ),
                part.cave,
            );
            water_tiles
                .entry(key)
                .or_default()
                .verts
                .extend_from_slice(&part.mesh.verts);
        }
        let mut water_keys: Vec<_> = water_tiles.keys().copied().collect();
        water_keys.sort();
        let water = water_keys
            .into_iter()
            .map(|key| {
                let mesh = &water_tiles[&key];
                let (center, radius) = bounds(mesh);
                (renderer.add_mesh(mesh), center, radius, key.1)
            })
            .collect();

        // Props are batched by tile and culled by distance. Clouds use bigger tiles and
        // reach farther.
        let tile = |p: Vec3, kind: PropKind| {
            let size = if kind == PropKind::Cloud || world.terrain.daylight(p + Vec3::Y) < 0.5 {
                CLOUD_TILE
            } else {
                PROP_TILE
            };
            ((p.x / size).floor() as i32, (p.z / size).floor() as i32)
        };
        // Cave props batch apart, so they draw only with the cave insides.
        type Key = (PropKind, u8, (i32, i32), bool);
        let mut batches: HashMap<Key, Vec<Instance>> = HashMap::new();
        type Shapes = Vec<(MeshBuilder, Option<MeshBuilder>, Option<MeshBuilder>)>;
        let shapes: HashMap<PropKind, Shapes> = PropKind::ALL
            .iter()
            .map(|k| {
                let near = models::prop_variants(*k);
                let all = near
                    .into_iter()
                    .enumerate()
                    .map(|(v, m)| {
                        // Boulders far off are just lumps.
                        let far = models::prop_far(*k, v as u32).or_else(|| {
                            matches!(*k, PropKind::Rock | PropKind::Palm)
                                .then(|| models::spire_of(&m))
                        });
                        let spire = far.as_ref().map(models::spire_of);
                        (m, far, spire)
                    })
                    .collect();
                (*k, all)
            })
            .collect();
        // Trees and flowers out in the open bake into their town tile's meshes: one draw
        // for everything on a tile, however much grows there.
        let (mut cave_tiles, open): (Vec<_>, Vec<_>) =
            crate::town_mesh::build(&world.terrain, &world.buildings, &world.bridges)
                .into_iter()
                .partition(|(_, t)| t.cave);
        let mut tiles: HashMap<(i32, i32), crate::town_mesh::Tile> = open.into_iter().collect();
        // Far meshes for the merged groups: buildings as they are, trees as spires.
        let mut group_far: HashMap<(i32, i32), MeshBuilder> = tiles
            .iter()
            .map(|(k, t)| {
                let mut far = t.far.clone();
                for (_, f) in &t.heavy {
                    far.verts.extend_from_slice(&f.verts);
                }
                (*k, far)
            })
            .collect();
        let mut group_distant: HashMap<(i32, i32), MeshBuilder> =
            tiles.iter().map(|(k, t)| (*k, t.distant.clone())).collect();
        let mut small_tiles: HashMap<(i32, i32), MeshBuilder> = HashMap::new();
        for p in &world.props {
            let variants = &shapes[&p.kind];
            let v = p.variant % variants.len() as u8;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(p.scale),
                Quat::from_rotation_y(p.yaw),
                p.pos,
            );
            // Clouds glow a little so they stay white from below. Alpha is daylight.
            let day = world.terrain.daylight(p.pos + Vec3::Y);
            // Clouds drift with the weather instead.
            if p.kind == PropKind::Cloud {
                continue;
            }
            let small = matches!(p.kind, PropKind::Flower | PropKind::Bush)
                || (p.kind == PropKind::Rock && p.scale < 2.5);
            if day >= 0.5 && small {
                // Small things only show up close, from small tiles of their own.
                let key = (
                    (p.pos.x / SMALL_TILE).floor() as i32,
                    (p.pos.z / SMALL_TILE).floor() as i32,
                );
                let tile = small_tiles.entry(key).or_default();
                let from = tile.verts.len();
                tile.place(&variants[v as usize].0, m);
                let alpha = (day * 255.0) as u8;
                tile.verts[from..]
                    .iter_mut()
                    .for_each(|v| v.color[3] = alpha);
                continue;
            }
            if day >= 0.5 {
                let key = crate::town_mesh::tile_key(Vec2::new(p.pos.x, p.pos.z));
                let tile = tiles.entry(key).or_insert_with(|| crate::town_mesh::Tile {
                    near: MeshBuilder::new(),
                    far: MeshBuilder::new(),
                    cave: false,
                    heavy: Vec::new(),
                    distant: MeshBuilder::new(),
                    letters: MeshBuilder::new(),
                });
                let (near, far, spire) = &variants[v as usize];
                let alpha = (day * 255.0) as u8;
                // Every tree its own green, and now and then one turning for autumn.
                let leaves = leaf_tint(p.kind, p.id);
                if let Some(spire) = spire {
                    for g in [
                        group_far.entry(key).or_default(),
                        group_distant.entry(key).or_default(),
                    ] {
                        let from = g.verts.len();
                        g.place(spire, m);
                        g.verts[from..].iter_mut().for_each(|v| v.color[3] = alpha);
                    }
                }
                let from = tile.near.verts.len();
                tile.near.place(near, m);
                tile.near.verts[from..].iter_mut().for_each(|v| {
                    v.color[3] = alpha;
                    tint_leaf(v, leaves);
                });
                // Flowers are too small to see from afar.
                if let Some(far) = far {
                    let from = tile.far.verts.len();
                    tile.far.place(far, m);
                    tile.far.verts[from..].iter_mut().for_each(|v| {
                        v.color[3] = alpha;
                        tint_leaf(v, leaves);
                    });
                }
                continue;
            }
            batches
                .entry((p.kind, v, tile(p.pos, p.kind), true))
                .or_default()
                .push(Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)));
        }
        let mut small_keys: Vec<_> = small_tiles.keys().copied().collect();
        small_keys.sort();
        for key in small_keys {
            let mesh = &small_tiles[&key];
            let (center, radius) = bounds(mesh);
            let id = renderer.add_mesh(mesh);
            renderer.add_static_culled(
                id,
                &[Instance::new(Mat4::IDENTITY)],
                Cull {
                    center,
                    radius,
                    reach: SMALL_REACH,
                    far: None,
                    detail: 1.0,
                    cave: false,
                    group: None,
                },
            );
        }
        let mut keys: Vec<_> = batches.keys().copied().collect();
        keys.sort_by_key(|(k, v, t, c)| (*k as u8, *v, *t, *c));
        let mut ids: HashMap<(PropKind, u8), MeshId> = HashMap::new();
        let mut far_ids: HashMap<(PropKind, u8), Option<MeshId>> = HashMap::new();
        for key in keys {
            let (kind, v, (tx, tz), cave) = key;
            let id = *ids
                .entry((kind, v))
                .or_insert_with(|| renderer.add_mesh(&shapes[&kind][v as usize].0));
            let far = *far_ids.entry((kind, v)).or_insert_with(|| {
                shapes[&kind][v as usize]
                    .1
                    .as_ref()
                    .map(|m| renderer.add_mesh(m))
            });
            let instances = &batches[&key];
            // Clouds reach farther and keep detail longer, flowers only show up close.
            let (size, y, reach, detail) = match kind {
                // Caves are big and hazy: few big batches that stop at the haze.
                _ if cave => (CLOUD_TILE, instances[0].model[3][1], 0.28, 0.8),
                PropKind::Cloud => (CLOUD_TILE, 700.0, 2.2, 1.0),
                PropKind::Flower | PropKind::Bush => (PROP_TILE, 300.0, 0.18, 1.0),
                _ => (PROP_TILE, 300.0, 1.0, 1.0),
            };
            let mut center = Vec3::new((tx as f32 + 0.5) * size, y, (tz as f32 + 0.5) * size);
            // Half the tile diagonal plus room for tall things: clouds spread high, trees
            // only stand on the ground below the center height.
            let mut radius = size * 0.71
                + if kind == PropKind::Cloud {
                    700.0
                } else {
                    300.0
                };
            if kind != PropKind::Cloud {
                // Props hug the ground they grow on, so bound them tightly.
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                let mut big = 0.0f32;
                for i in instances {
                    let p = Vec3::from_slice(&i.model[3][..3]);
                    lo = lo.min(p);
                    hi = hi.max(p);
                    big = big.max(Vec3::from_slice(&i.model[0][..3]).length());
                }
                center = (lo + hi) * 0.5;
                // Giant cave things reach far from where they stand.
                radius = (hi - lo).length() * 0.5 + 25.0f32.max(big * 8.0);
            }
            renderer.add_static_culled(
                id,
                instances,
                Cull {
                    center,
                    radius,
                    reach,
                    far,
                    detail,
                    cave,
                    group: None,
                },
            );
        }

        // Towns, roads, trees and flowers, a near and a far mesh per tile. Far away, each
        // block of GROUP by GROUP tiles draws as one merged mesh.
        let mut tiles: Vec<_> = tiles.into_iter().collect();
        tiles.sort_by_key(|(k, _)| *k);
        let group_of = |(x, z): (i32, i32)| (x.div_euclid(GROUP), z.div_euclid(GROUP));
        let mut merged: HashMap<(i32, i32), MeshBuilder> = HashMap::new();
        for (key, far) in &group_far {
            merged
                .entry(group_of(*key))
                .or_default()
                .verts
                .extend_from_slice(&far.verts);
        }
        let mut distant: HashMap<(i32, i32), MeshBuilder> = HashMap::new();
        for (key, d) in &group_distant {
            distant
                .entry(group_of(*key))
                .or_default()
                .verts
                .extend_from_slice(&d.verts);
        }
        let groups: HashMap<(i32, i32), Group> = merged
            .iter()
            .filter(|(_, m)| !m.verts.is_empty())
            .map(|(key, m)| {
                let (center, radius) = bounds(m);
                let id = renderer.add_mesh(m);
                let far = distant
                    .get(key)
                    .filter(|d| !d.verts.is_empty())
                    .map(|d| renderer.add_mesh(d));
                let group = Group {
                    center,
                    radius,
                    range: GROUP_RANGE,
                    merged: true,
                };
                renderer.add_static_culled(
                    id,
                    &[Instance::new(Mat4::IDENTITY)],
                    Cull {
                        center,
                        radius,
                        reach: 2.4,
                        far,
                        detail: DISTANT,
                        cave: false,
                        group: Some(group),
                    },
                );
                (*key, group)
            })
            .collect();
        tiles.append(&mut cave_tiles);
        for (key, tile) in tiles {
            let group = if tile.cave {
                None
            } else {
                groups.get(&group_of(key)).map(|g| Group {
                    merged: false,
                    ..*g
                })
            };
            // Signs' painted letters, up close only.
            if !tile.letters.verts.is_empty() {
                let (center, radius) = bounds(&tile.letters);
                let id = renderer.add_mesh(&tile.letters);
                renderer.add_letters(
                    id,
                    Cull {
                        center,
                        radius,
                        reach: LETTERS_REACH,
                        far: None,
                        detail: 1.0,
                        cave: tile.cave,
                        group,
                    },
                );
            }
            // Heavy buildings switch to their far mesh by their own bounds, not the tile's.
            for (near, far) in &tile.heavy {
                let (center, radius) = bounds(near);
                let id = renderer.add_mesh(near);
                let far = (!far.verts.is_empty()).then(|| renderer.add_mesh(far));
                renderer.add_static_culled(
                    id,
                    &[Instance::new(Mat4::IDENTITY)],
                    Cull {
                        center,
                        radius,
                        reach: 2.4,
                        far,
                        detail: HEAVY_DETAIL,
                        cave: tile.cave,
                        group,
                    },
                );
            }
            if tile.near.verts.is_empty() {
                continue;
            }
            let (center, radius) = bounds(&tile.near);
            let near = renderer.add_mesh(&tile.near);
            let far = (!tile.far.verts.is_empty()).then(|| renderer.add_mesh(&tile.far));
            renderer.add_static_culled(
                near,
                &[Instance::new(Mat4::IDENTITY)],
                Cull {
                    center,
                    radius,
                    reach: 2.4,
                    far,
                    detail: 0.55,
                    cave: tile.cave,
                    group,
                },
            );
        }

        // Suspension bridges: thick main cables over the towers, hangers down to the deck.
        for b in &world.bridges {
            let mut mesh = crate::mesh::MeshBuilder::new();
            let col = crate::town_mesh::BRIDGE_PAINT;
            let strand = |m: &mut crate::mesh::MeshBuilder, a: Vec3, b: Vec3, r: f32| {
                let along = (b - a).normalize_or(Vec3::X);
                let across = along.cross(Vec3::Y).normalize_or(Vec3::Z) * r;
                let up = along.cross(across).normalize_or(Vec3::Y) * r;
                m.quad(a - across, b - across, b + across, a + across, col);
                m.quad(a + across, b + across, b - across, a - across, col);
                m.quad(a - up, b - up, b + up, a + up, col);
                m.quad(a + up, b + up, b - up, a - up, col);
            };
            for line in &b.cables {
                for w in line.windows(2) {
                    strand(&mut mesh, w[0], w[1], 0.55);
                }
            }
            for &(top, foot) in &b.hangers {
                strand(&mut mesh, top, foot, 0.09);
            }
            let (center, radius) = bounds(&mesh);
            let id = renderer.add_mesh(&mesh);
            renderer.add_static_culled(
                id,
                &[Instance::new(Mat4::IDENTITY)],
                Cull {
                    center,
                    radius,
                    reach: 3.0,
                    far: None,
                    detail: 1.0,
                    cave: false,
                    group: None,
                },
            );
        }

        // Power lines: three dark conductors a side, sagging between the pylons.
        {
            use zawnabout_sim::landmark::PYLON_ARMS;
            let mut mesh = crate::mesh::MeshBuilder::new();
            let col = Rgb(0x3A3A40);
            for line in &world.power_lines {
                for w in line.windows(2) {
                    let (a, b) = (&world.buildings[w[0]], &world.buildings[w[1]]);
                    for (y, out) in PYLON_ARMS {
                        for s in [-1.0f32, 1.0] {
                            let hook = |p: &zawnabout_sim::town::Building| {
                                let f = p.front();
                                p.pos + Vec3::new(f.x * s * out, y - 0.7, f.y * s * out)
                            };
                            let (pa, pb) = (hook(a), hook(b));
                            let sag = pa.distance(pb) * 0.035;
                            let steps = 10;
                            for k in 0..steps {
                                let at =
                                    |t: f32| pa.lerp(pb, t) - Vec3::Y * sag * 4.0 * t * (1.0 - t);
                                let (p0, p1) = (
                                    at(k as f32 / steps as f32),
                                    at((k + 1) as f32 / steps as f32),
                                );
                                let along = (p1 - p0).normalize_or(Vec3::X);
                                let across = along.cross(Vec3::Y).normalize_or(Vec3::Z) * 0.06;
                                let up = Vec3::Y * 0.06;
                                mesh.quad(p0 - up, p1 - up, p1 + up, p0 + up, col);
                                mesh.quad(p0 + up, p1 + up, p1 - up, p0 - up, col);
                                mesh.quad(p0 - across, p1 - across, p1 + across, p0 + across, col);
                                mesh.quad(p0 + across, p1 + across, p1 - across, p0 - across, col);
                            }
                        }
                    }
                }
            }
            if !mesh.verts.is_empty() {
                let (center, radius) = bounds(&mesh);
                let id = renderer.add_mesh(&mesh);
                renderer.add_static_culled(
                    id,
                    &[Instance::new(Mat4::IDENTITY)],
                    Cull {
                        center,
                        radius,
                        reach: 3.0,
                        far: None,
                        detail: 1.0,
                        cave: false,
                        group: None,
                    },
                );
            }
        }

        // Lift cables: thin and dark, sagging nowhere, following each lift's line.
        for line in world.traffic.cables() {
            let mut cable = crate::mesh::MeshBuilder::new();
            let col = Rgb(0x3A3A40);
            for w in line.windows(2) {
                let (a, b) = (w[0], w[1]);
                let along = (b - a).normalize_or(Vec3::X);
                let across = along.cross(Vec3::Y).normalize_or(Vec3::Z) * 0.07;
                let up = Vec3::Y * 0.07;
                cable.quad(a - across, b - across, b + across, a + across, col);
                cable.quad(a - up, b - up, b + up, a + up, col);
            }
            let (center, radius) = bounds(&cable);
            let mesh = renderer.add_mesh(&cable);
            renderer.add_static_culled(
                mesh,
                &[Instance::new(Mat4::IDENTITY)],
                Cull {
                    center,
                    radius,
                    reach: 1.6,
                    far: None,
                    detail: 1.0,
                    cave: false,
                    group: None,
                },
            );
        }

        let renderer_spray = renderer.add_mesh(&models::spray());
        let treasures = crate::treasure::KINDS
            .iter()
            .map(|k| (*k, renderer.add_mesh(&crate::treasure::treasure(k))))
            .collect();
        let star = renderer.add_mesh(&crate::treasure::star());
        let lectern = renderer.add_mesh(&crate::treasure::lectern());
        let flag = renderer.add_mesh(&crate::treasure::flag());
        let paper_lantern = renderer.add_mesh(&crate::treasure::paper_lantern());
        let pole = renderer.add_mesh(&crate::treasure::pole());
        let brazier = renderer.add_mesh(&crate::treasure::brazier());
        let saddle = renderer.add_mesh(&crate::treasure::saddle());
        let umbrella = renderer.add_mesh(&crate::treasure::umbrella());
        let bubbles = [
            renderer.add_mesh(&crate::treasure::bubble(false)),
            renderer.add_mesh(&crate::treasure::bubble(true)),
        ];
        let heap = {
            let mut m = MeshBuilder::new();
            crate::treasure::coins(&mut m, 1.0);
            renderer.add_mesh(&m)
        };
        let flame = renderer.add_mesh(&models::flame());
        let pool = renderer.add_mesh(&models::pool());
        let tuft = renderer.add_mesh(&models::tuft());
        let life = crate::life::Life::new(&mut renderer, world, pool);
        let weather = crate::weather::Weather::new(&mut renderer, world, pool);
        let cave_life = crate::cave_life::CaveLife::new(&mut renderer, world);
        let park_life = crate::park_life::ParkLife::new(&mut renderer, world, life.people(), pool);
        let sea_life = crate::sea_life::SeaLife::new(&mut renderer, world);
        let wildlife = crate::wildlife::Wildlife::new(&mut renderer, world);
        let (plugs, mouths) = mouth_plugs(&world.terrain);
        let plug = {
            let mut m = crate::mesh::MeshBuilder::new();
            m.blob(Vec3::ZERO, Vec3::ONE, true, 0.0, 1, Rgb(0x1C2030));
            renderer.add_mesh(&m)
        };
        let lamps = world
            .buildings
            .iter()
            .filter(|b| b.kind == zawnabout_sim::town::Kind::Lantern)
            .map(|b| {
                let cave = world.terrain.daylight(b.pos + Vec3::Y) < 0.5;
                (
                    b.pos + Vec3::Y * (b.size.y + 0.2),
                    b.pos + Vec3::Y * 0.25,
                    cave,
                )
            })
            .chain(wall_lamps(world))
            .collect();
        let bulbs = world
            .buildings
            .iter()
            .flat_map(|b| {
                use zawnabout_sim::town::Kind;
                let fair = matches!(b.kind, Kind::Carnival | Kind::ThemePark | Kind::Pier);
                crate::town_mesh::lamp_spots(b)
                    .into_iter()
                    .map(move |(p, r, always)| (p, r, always, fair))
            })
            .collect();
        // Ordinary cars share one white mesh, painted per car by the instance tint, so
        // they draw in one go.
        // Vehicles that differ only in paint share one white mesh, tinted per vehicle, so
        // they cost one draw between them.
        let shared = |v: &zawnabout_sim::traffic::Vehicle| {
            use zawnabout_sim::town::Kind;
            use zawnabout_sim::traffic::TOUR_CAR;
            match v.kind {
                Kind::Car if v.variant < CAR_KINDS => CAR_WHITE,
                Kind::FerrisCar | Kind::Punt | Kind::RaceCar | Kind::CoasterCar => 0,
                Kind::Train if (TOUR_CAR..TOUR_CAR + 3).contains(&v.variant) => TOUR_CAR,
                // Balloons come in four stripes.
                Kind::Balloon => v.variant % 4,
                _ => v.variant,
            }
        };
        let mut vehicle_ids: HashMap<(u8, u8), MeshId> = HashMap::new();
        let vehicles = world
            .traffic
            .vehicles
            .iter()
            .map(|v| {
                *vehicle_ids
                    .entry((v.kind as u8, shared(v)))
                    .or_insert_with(|| {
                        let mesh = crate::town_mesh::vehicle(
                            &world.terrain,
                            v.kind,
                            v.size,
                            shared(v) as u32,
                        );
                        renderer.add_mesh(&mesh)
                    })
            })
            .collect();
        let ember = renderer.add_mesh(&models::ember());
        let shafts = light_shafts(&world.terrain)
            .iter()
            .map(|(m, foot)| {
                let (center, radius) = bounds(m);
                (renderer.add_mesh(m), center, radius, *foot)
            })
            .collect();
        let glowers = world
            .props
            .iter()
            .filter(|p| p.kind == PropKind::Crystal && p.scale >= 2.6)
            .map(|p| {
                let color = linear(
                    models::CRYSTAL[p.variant as usize % models::CRYSTAL.len()],
                    1.0,
                );
                (
                    p.pos + Vec3::Y * 1.6 * p.scale,
                    (p.scale * 9.0).min(70.0),
                    color * 1.1,
                )
            })
            .collect();
        let m = models::dragon();
        let dragon = DragonIds {
            body: renderer.add_mesh(&m.body),
            neck: renderer.add_mesh(&m.neck),
            head: renderer.add_mesh(&m.head),
            eye: renderer.add_mesh(&m.eye),
            tail: renderer.add_mesh(&m.tail),
            plume: renderer.add_mesh(&m.plume),
            whisker: renderer.add_mesh(&m.whisker),
            mane: renderer.add_mesh(&m.mane),
            leg: renderer.add_mesh(&m.leg),
            arm: [renderer.add_mesh(&m.arm[0]), renderer.add_mesh(&m.arm[1])],
            hand: [renderer.add_mesh(&m.hand[0]), renderer.add_mesh(&m.hand[1])],
            star: renderer.add_mesh(&m.star),
        };
        let mut gfx = Self {
            renderer,
            dragon,
            kin_meshes: Vec::new(),
            terrain: TerrainMeshes::default(),
            treasures,
            star,
            heap,
            pics_version: 0,
            bubbles,
            lectern,
            flag,
            paper_lantern,
            pole,
            brazier,
            saddle,
            umbrella,
            water,
            spray: renderer_spray,
            flame,
            ember,
            shafts,
            glowers,
            vehicles,
            lamps,
            bulbs,
            pool,
            tuft,
            life,
            cave_life,
            weather,
            park_life,
            sea_life,
            wildlife,
            plugs,
            mouths,
            plug,
            tufts: std::cell::RefCell::new(Lawn::default()),
            far_tufts: std::cell::RefCell::new(Lawn::default()),
            falls: crate::water::falls_feet(&game.sim.world.terrain),
            listener: crate::audio::Listener::new(game),
            chunk_budget: 12,
            still: None,
            hud_map: (None, false),
            book_map: None,
        };
        gfx.update_terrain(game, usize::MAX, f64::INFINITY);
        gfx
    }

    /// The weather, for scripts to ask about.
    pub fn weather(&self) -> &crate::weather::Weather {
        &self.weather
    }

    /// Streams terrain chunks around the camera, at most `budget` steps of building, and
    /// none started after `secs`.
    pub fn update_terrain(&mut self, game: &Game, budget: usize, secs: f64) {
        self.terrain.update(
            &mut self.renderer,
            &game.sim.world.terrain,
            game.camera.pos,
            budget,
            secs,
        );
    }

    fn mesh(&self, part: Part) -> MeshId {
        Self::mesh_of(&self.dragon, part)
    }

    fn mesh_of(d: &DragonIds, part: Part) -> MeshId {
        match part {
            Part::Body => d.body,
            Part::Neck(_) => d.neck,
            Part::Head => d.head,
            Part::Eye => d.eye,
            Part::Tail(_) => d.tail,
            Part::Plume => d.plume,
            Part::Whisker => d.whisker,
            Part::Mane => d.mane,
            Part::Leg => d.leg,
            Part::Arm(i) => d.arm[i],
            Part::Hand(i) => d.hand[i],
            Part::Star => d.star,
        }
    }

    pub fn frame(&self, game: &Game) -> Frame {
        let (w, h) = self.renderer.size();
        let look = &game.look;
        let sky = game.sky();
        // Low sun turns everything golden, night swaps the sun for a bright, cool moon
        // and dims the sky to deep blue.
        let night = sky.night;
        let blend = |day: Vec3, dusk: Vec3, dark: Vec3| day.lerp(dusk, sky.dusk).lerp(dark, night);
        let c = |hex: u32, k: f32| linear(Rgb(hex), k);
        let lighting = Lighting {
            sun_dir: sky.light_dir,
            sun_color: blend(c(0xFFF1DC, 1.05), c(0xFFB475, 0.95), c(0xC9D6FF, 0.55)),
            sky_top: blend(c(0x5FAEEB, 1.0), c(0x6E86C9, 1.0), c(0x0E1A3D, 1.0)),
            sky_horizon: blend(c(0xFFE6CF, 1.0), c(0xFFAE7A, 1.0), c(0x2B3F6E, 1.0)),
            ambient_sky: blend(c(0xA9CBF0, 0.62), c(0xB9A6D6, 0.55), c(0x5A6FA8, 0.45)),
            ambient_ground: blend(c(0xC9E0A6, 0.42), c(0xD8B28E, 0.38), c(0x3A4A6A, 0.3)),
            fog_start: look.fog_start,
            fog_end: look.fog_end,
            bands: look.bands,
            shadow: look.shadow,
            cave_ambient: cave_tint(&game.sim.world.terrain, game.camera.pos),
            underground: 1.0 - game.sim.world.terrain.daylight(game.camera.pos),
            sky_gloom: 0.0,
            night,
            // Mist lies in the low ground from first light, and burns off by mid morning.
            valley_fog: smooth(4.8, 6.2, sky.hour) * (1.0 - smooth(7.8, 10.0, sky.hour)),
            rainbow: 0.0,
            bow: Vec3::NEG_Y,
        };
        // Inside a cloud the world whites out into mist close round, and under a rain
        // cloud the light turns grey.
        // Under rock, as in a tunnel or a cave, it never rains.
        let open_sky = {
            let p = game.camera.pos;
            game.sim.world.terrain.height(p.x, p.z) < p.y + 1.5
        };
        let mut weather = self.weather.here(game.camera.pos, game.time as f32);
        if !open_sky {
            weather.rain = 0.0;
        }
        let mut lighting = lighting;
        {
            let (mist, rain) = (weather.mist, weather.rain);
            let cloud = c(0xF2F4F8, 1.0).lerp(c(0x2B3F6E, 1.0), night);
            let grey = c(0x9AA3B0, 1.0).lerp(c(0x1E2840, 1.0), night);
            let l = &mut lighting;
            l.sky_top = l.sky_top.lerp(grey, rain * 0.6).lerp(cloud, mist);
            l.sky_horizon = l.sky_horizon.lerp(grey, rain * 0.5).lerp(cloud, mist);
            l.sun_color *= 1.0 - rain * 0.45 - mist * 0.3;
            l.ambient_sky = l.ambient_sky.lerp(cloud * 0.7, mist * 0.5);
            // Rain closes in the distance a little, mist all of it.
            l.fog_end *= 1.0 - rain * 0.35;
            l.fog_start += (2.0 - l.fog_start) * mist;
            l.fog_end += (70.0 - l.fog_end) * mist;
            // A lightning flash whitens the sky and lights everything for a moment.
            // By day it is only a glint, by night the whole sky lights up.
            let flash = c(0xE8ECFF, 1.0) * weather.flash * (0.15 + 0.85 * night);
            l.sky_top += flash * 0.6;
            l.sky_horizon += flash * 0.8;
            l.ambient_sky += flash * 0.7;
            l.ambient_ground += flash * 0.4;
            // A rainbow when the sun is up and not too high, behind you, and rain falls
            // out in front, not on you.
            let anti = Vec2::new(-l.sun_dir.x, -l.sun_dir.z).normalize_or_zero();
            let sun_ok = smooth(0.03, 0.12, l.sun_dir.y) * (1.0 - smooth(0.55, 0.75, l.sun_dir.y));
            let cam = game.camera.pos;
            let showers = self
                .weather
                .nearest(cam, game.time as f32, 8)
                .into_iter()
                .filter(|(_, _, rain)| *rain)
                .map(|(at, _, _)| {
                    let to = Vec2::new(at.x - cam.x, at.z - cam.z);
                    let near = 1.0 - smooth(2500.0, 6000.0, to.length());
                    near * smooth(0.2, 0.6, to.normalize_or_zero().dot(anti))
                })
                .fold(0.0f32, f32::max);
            l.rainbow = showers
                * sun_ok
                * (1.0 - night)
                * (1.0 - rain)
                * (1.0 - mist)
                * open_sky as u8 as f32;
            l.bow = rainbow_center(-l.sun_dir.normalize_or(Vec3::Y), cam);
        }

        let alpha = game.alpha();
        let mut parts = Vec::new();
        let mut draws = Vec::new();
        let mut shadows = Vec::new();
        let world = &game.sim.world;
        for (_, visual) in &game.visuals {
            parts.clear();
            visual.parts(alpha, &mut parts);
            // Instance alpha carries daylight, so the dragon darkens in caves too.
            let (pos, _) = visual.interpolated(alpha);
            let day = world.terrain.daylight(pos);
            draws.extend(parts.iter().map(|(part, m, tint)| {
                (
                    self.mesh(*part),
                    Instance::tinted(*m, tint.truncate().extend(tint.w * day)),
                )
            }));
            let ground = world.support(pos.x, pos.z, pos.y);
            let height = (pos.y - ground).max(0.0);
            let normal = world.ground_normal(pos.x, pos.z);
            let radius = 2.6 + height * 0.015;
            let strength = 1.0 - (height / 160.0).clamp(0.0, 1.0);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(radius, 1.0, radius * 1.3),
                Quat::from_rotation_arc(Vec3::Y, normal)
                    * Quat::from_rotation_y(game.player_dragon().heading),
                Vec3::new(pos.x, ground + 0.12, pos.z),
            );
            shadows.push(Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, strength)));
        }
        // The other dragons in their own colors and sizes, and the hatchling. A named one
        // stands as tall as you, so it grows from its feet. The hatchling's body already
        // sits low, so it shrinks round its middle. A roll for fun turns it about its
        // length.
        let stand = game.sim.flight.stand_height;
        let flyers = game
            .flock
            .dragons
            .iter()
            .enumerate()
            .map(|(i, f)| (f, false, self.kin_meshes.get(i).unwrap_or(&self.dragon)))
            .chain(game.flock.hatchling.iter().map(|h| (h, true, &self.dragon)));
        for (f, small, ids) in flyers {
            let (pos, rot) = f.vis.interpolated(alpha);
            if pos.distance(game.camera.pos) > 1800.0 {
                continue;
            }
            parts.clear();
            f.vis.parts(alpha, &mut parts);
            let pivot = if small { pos } else { pos - Vec3::Y * stand };
            let fwd = (rot * Vec3::Z).normalize_or(Vec3::Z);
            let around = Mat4::from_translation(pivot)
                * Mat4::from_scale(Vec3::splat(f.size))
                * Mat4::from_translation(-pivot)
                * Mat4::from_translation(pos)
                * Mat4::from_axis_angle(fwd, f.spin)
                * Mat4::from_translation(-pos);
            let day = world.terrain.daylight(pos);
            draws.extend(parts.iter().map(|(part, m, tint)| {
                (
                    Self::mesh_of(ids, *part),
                    Instance::tinted(around * *m, tint.truncate().extend(tint.w * day)),
                )
            }));
            let ground = world.support(pos.x, pos.z, pos.y);
            let height = (pos.y - ground).max(0.0);
            let radius = (2.6 + height * 0.015) * f.size;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(radius, 1.0, radius * 1.3),
                Quat::from_rotation_arc(Vec3::Y, world.ground_normal(pos.x, pos.z))
                    * Quat::from_rotation_y(f.d.heading),
                Vec3::new(pos.x, ground + 0.12, pos.z),
            );
            let strength = 1.0 - (height / 160.0).clamp(0.0, 1.0);
            shadows.push(Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, strength)));
        }

        let t = game.time as f32;
        // Near a mouth, or looking out of one from further in, the outside draws too, so
        // the opening shows the world beyond and not just sky.
        let cam = &game.camera;
        let ahead = (cam.look - cam.pos).normalize_or(Vec3::Z);
        let peek = self.mouths.iter().any(|m| {
            let d = m.distance(cam.pos);
            d < 450.0 || (d < 1600.0 && (*m - cam.pos).normalize_or(Vec3::Z).dot(ahead) > 0.6)
        });
        let outside = lighting.underground < 0.5 || peek;
        lighting.sky_gloom = if peek {
            0.0
        } else {
            ((lighting.underground - 0.5) * 2.5).clamp(0.0, 1.0)
        };
        let close =
            lighting.underground < 0.5 || self.mouths.iter().any(|m| m.distance(cam.pos) < 450.0);
        use crate::prof::time;
        // Deep in a cave with no mouth close by, only cave vehicles draw: ships and cabins
        // out in the open would show through a far mouth for a lot of cost.
        let open_air = lighting.underground < 0.85 || close;
        time("vehicles", || self.vehicles(game, open_air, &mut draws));
        // Grass only shows close up, so not through a far mouth.
        if close {
            time("grass", || self.grass(game, &mut draws));
        }
        // Spray boils up at the foot of every falls nearby.
        for (k, (foot, w)) in self.falls.iter().enumerate() {
            if foot.distance(game.camera.pos) > 1500.0 {
                continue;
            }
            let day = world.terrain.daylight(*foot + Vec3::Y * 3.0);
            for i in 0..7 {
                let seed = (k * 7 + i) as f32;
                let phase = (t * 0.35 + seed * 0.618).fract();
                let spread = w * 0.5 * (0.4 + phase);
                let a = seed * 2.4 + t * 0.2;
                let at =
                    *foot + Vec3::new(a.cos() * spread, 1.0 + phase * w * 0.6, a.sin() * spread);
                let size = w * 0.32 * (1.0 - phase * 0.7) * (phase * 6.0).min(1.0);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(size),
                    Quat::from_rotation_y(a),
                    at,
                );
                draws.push((
                    self.spray,
                    Instance::tinted(m, Vec4::new(1.3, 1.3, 1.3, day)),
                ));
            }
        }

        // Landing rings: two that breathe in and out, warmer for the destination.
        let mut rings = Vec::new();
        let player = game.player_dragon().pos;
        for m in game.markers() {
            let fade = if game.feedback.active {
                1.0
            } else {
                1.0 - (m.pos.distance(player) / game.look.ring_range).clamp(0.0, 1.0)
            };
            let (color, strength) = if m.destination {
                (linear(Rgb(0xFFB089), 1.0), 0.95)
            } else {
                (linear(Rgb(0xFFE38A), 1.0), 0.95 * fade.sqrt())
            };
            for k in 0..2 {
                let phase = (t * 0.8 + k as f32 * 0.5).fract();
                // Halos face the camera and keep a minimum on-screen size, so spots stay
                // easy to see and tap from any angle and distance.
                let center = m.pos + Vec3::Y * 0.6;
                let to_cam = game.camera.pos - center;
                let size = m.radius.max(to_cam.length() * 0.035);
                let scale = size * (0.55 + 0.6 * phase);
                let alpha = strength * (1.0 - phase) * (phase * 4.0).min(1.0);
                let face = Quat::from_rotation_arc(Vec3::Y, to_cam.normalize_or(Vec3::Y));
                let model = Mat4::from_scale_rotation_translation(
                    Vec3::new(scale, 1.0, scale),
                    face,
                    center,
                );
                rings.push(Instance::tinted(model, color.extend(alpha)));
            }
        }

        let caves = lighting.underground > 0.05 || world.terrain.near_voids(game.camera.pos, 150.0);
        let dark: Vec<Vec4> = game
            .play
            .as_ref()
            .map(crate::play::dark)
            .unwrap_or_default();
        let (mut fx, lights) = time("effects", || self.effects(game, caves, t, &dark));
        time("play", || self.play(game, t, night, &mut draws, &mut fx));
        // People live underground too, so their towns' life draws near the caves.
        if outside || caves {
            time("life", || {
                let sim_time = (game.sim.tick as f32 + game.alpha()) * zawnabout_sim::DT;
                self.life.draw(
                    world,
                    game.camera.pos,
                    t,
                    sim_time,
                    night,
                    game.play.as_ref().and_then(crate::play::riding_folk),
                    &dark,
                    &mut draws,
                    &mut fx,
                )
            });
            time("park_life", || {
                self.park_life.draw(
                    &world.traffic,
                    (game.sim.tick as f32 + game.alpha()) * zawnabout_sim::DT,
                    game.sky().hour,
                    game.camera.pos,
                    t,
                    night,
                    &mut draws,
                    &mut fx,
                )
            });
        }
        // Nobody stands inside a wall: anyone whose way crosses one waits out of sight
        // until they are through. Whoever is left nearest the dragon can be talked to.
        {
            let (body, heads) = self.life.people();
            let cam = game.camera.pos;
            draws.retain(|(m, inst)| {
                if *m != body && !heads.contains(m) {
                    return true;
                }
                let at = Vec3::from_slice(&inst.model[3][..3]);
                at.distance(cam) > 200.0 || !world.in_building(at + Vec3::Y * 1.0)
            });
            let me = game.player_dragon().pos;
            let feet = me.y - game.sim.flight.stand_height;
            let near = draws
                .iter()
                .filter(|(m, _)| *m == body)
                .map(|(_, inst)| Vec3::from_slice(&inst.model[3][..3]))
                // Not a passenger on the dragon's own back.
                .filter(|p| {
                    (p.y - feet).abs() < 2.5 && Vec2::new(p.x - me.x, p.z - me.z).length() > 1.2
                })
                .min_by(|a, b| a.distance(me).total_cmp(&b.distance(me)))
                .filter(|p| p.distance(me) < 12.0)
                .map(|p| {
                    let q = |v: f32| (v / 3.0).round() as i32 as u32;
                    (
                        p,
                        (q(p.x).wrapping_mul(73_856_093) ^ q(p.z).wrapping_mul(19_349_663))
                            | 1 << 31,
                    )
                });
            game.near_drawn.set(near);
        }
        if outside {
            time("weather", || {
                self.weather.draw(
                    game.camera.pos,
                    game.time as f32,
                    night,
                    open_sky,
                    &mut draws,
                    &mut fx,
                )
            });
            time("sea_life", || {
                self.sea_life.draw(game.camera.pos, t, &mut draws)
            });
            time("wildlife", || {
                self.wildlife.draw(
                    world,
                    game.camera.pos,
                    t,
                    night,
                    self.pool,
                    &mut draws,
                    &mut fx,
                )
            });
        }
        if caves {
            self.cave_life.draw(
                world,
                game.camera.pos,
                t,
                lighting.cave_ambient,
                self.pool,
                self.life.people(),
                &mut draws,
                &mut fx,
            );
        }
        // People far off are a speck of shirt: a cheap body, and no head.
        {
            let (body, heads) = self.life.people();
            let far = self.life.person_far();
            let cam = game.camera.pos;
            draws.retain_mut(|(m, i)| {
                let away = Vec3::from_slice(&i.model[3][..3]).distance_squared(cam);
                if away < PERSON_FAR * PERSON_FAR {
                    return true;
                }
                if *m == body {
                    *m = far;
                }
                !heads.contains(m)
            });
        }
        if !caves {
            for m in &self.plugs {
                if m.w_axis.truncate().distance(game.camera.pos) < 4000.0 {
                    draws.push((self.plug, Instance::new(*m)));
                }
            }
        }
        // Seen through a far mouth, the outside is only its land and water: the props and
        // towns out there would cost a great deal for a small hole of view.
        let deep = lighting.underground > 0.85 && !peek;
        let near_mouth = self
            .mouths
            .iter()
            .any(|m| m.distance(game.camera.pos) < 450.0);
        let deep_statics = lighting.underground > 0.85 && !near_mouth;
        let water = self
            .water
            .iter()
            .filter(|(_, _, _, cave)| if *cave { caves } else { !deep })
            .map(|(id, c, r, _)| (*id, *c, *r))
            .collect();
        Frame {
            water,
            time: (game.time % 3600.0) as f32,
            view_proj: game.camera.view_proj(w as f32 / h.max(1) as f32),
            cam_pos: game.camera.pos,
            lighting,
            draws,
            // Cave insides are hidden unless underground or close enough to see in.
            terrain: self
                .terrain
                .draw_list(game.camera.pos, caves, lighting.underground, deep),
            prop_range: game.look.prop_range,
            detail_range: game.look.detail_range,
            deep: deep_statics,
            caves,
            shadows,
            rings,
            fx,
            lights,
            dark: dark.clone(),
            clouds: if outside {
                self.weather.shadows(game.camera.pos, game.time as f32)
            } else {
                Vec::new()
            },
            // A postcard's picture is the plain view.
            hud: if game.wants_snap() {
                Vec::new()
            } else {
                let d = game.player_dragon();
                // Only the open journal reads the hoard, and gathering it is not free.
                let hoard = if game.hud.book.showing() {
                    game.hoard_view()
                } else {
                    None
                };
                let view = crate::book::View {
                    hoard: hoard.as_ref(),
                    binds: &game.bindings,
                    muted: game.muted,
                    pause_away: game.pause_away,
                    hold_hour: game.hold_hour,
                    touch: game.touch,
                    hud: game.hud.on,
                    map: game.map_view(),
                };
                game.hud.build(
                    self.renderer.size(),
                    glam::Vec2::new(d.pos.x, d.pos.z),
                    d.heading,
                    game.sim.world.terrain.def.bounds,
                    self.hud_map.1,
                    game.feedback.active,
                    &view,
                )
            },
        }
    }

    /// Finds the townsperson close enough to talk to, where the townsfolk are known.
    /// Drawing does this every frame. Hosts that do not draw every frame call it.
    pub fn observe(&self, game: &Game) {
        let me = game.player_dragon();
        let near = (game.play.is_some() && me.mode == zawnabout_sim::Mode::Ground)
            .then(|| {
                self.life.nearest_person(
                    me.pos,
                    game.folk_search,
                    game.time as f32,
                    game.sky().night,
                )
            })
            .flatten();
        game.near_folk.set(near);
    }

    /// Gameplay on top: finds still lying about, each glinting now and then when the sun
    /// catches it (softly glowing at night and underground), and the hoard heaped up
    /// in its lair with every treasure set round it.
    /// A story's lamps, lit or waiting for fire.
    fn story_lamps(&self, game: &Game, draws: &mut Vec<(MeshId, Instance)>) {
        let Some(play) = &game.play else { return };
        let cam = game.camera.pos;
        for (foot, _) in crate::play::lamps(play) {
            if foot.distance(cam) > 600.0 {
                continue;
            }
            let day = game.sim.world.terrain.daylight(foot + Vec3::Y);
            let m = Mat4::from_translation(foot);
            draws.push((
                self.brazier,
                Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
            ));
        }
    }

    /// What finished sets bring: people come out, bunting and lanterns go up.
    /// A fireworks show over `top`: rockets climb, burst into rings of colored stars
    /// that drift down and fade, one after another on their own beats.
    fn fireworks(
        &self,
        top: Vec3,
        id: usize,
        t: f32,
        night: f32,
        cam: Vec3,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        const COLORS: [u32; 6] = [0xFF5A5A, 0xFFD24A, 0x6AD0FF, 0x9CFF7A, 0xFF8AE0, 0xFFFFFF];
        for k in 0..4 {
            let beat = 3.7 + k as f32 * 0.6;
            let phase = ((t + k as f32 * 1.3 + id as f32 * 0.7) / beat).fract();
            let spread = Vec3::new(
                ((k * 37 + id * 11) % 50) as f32 - 25.0,
                (k as f32 * 9.0) % 20.0,
                ((k * 53 + id * 7) % 50) as f32 - 25.0,
            );
            let center = top + spread;
            let color = linear(Rgb(COLORS[(k + id) % COLORS.len()]), 1.0);
            let star = |at: Vec3, size: f32, glow: f32, fx: &mut Vec<(MeshId, Instance)>| {
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let s = Mat4::from_scale_rotation_translation(Vec3::splat(size), face, at);
                fx.push((
                    self.star,
                    Instance::tinted(s, (color * 2.0).extend(glow * night)),
                ));
            };
            if phase < 0.25 {
                // The rocket going up.
                let u = phase / 0.25;
                let at = center - Vec3::Y * (1.0 - u) * top.y.max(60.0) * 0.6;
                star(at, 1.6, 0.8, fx);
                continue;
            }
            let u = (phase - 0.25) / 0.75;
            let r = 34.0 * u.sqrt();
            let fade = (1.0 - u) * (1.0 - u);
            for j in 0..28 {
                // Spread evenly round a sphere, golden-angle style.
                let y = 1.0 - (j as f32 + 0.5) / 14.0;
                let ring = (1.0 - y * y).max(0.0).sqrt();
                let a = j as f32 * 2.399;
                let dir = Vec3::new(a.cos() * ring, y, a.sin() * ring);
                let at = center + dir * r - Vec3::Y * u * u * 14.0;
                star(at, 3.2 * (1.0 - u * 0.5), fade, fx);
            }
        }
    }

    fn set_effects(
        &self,
        game: &Game,
        play: &crate::play::Play,
        t: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        const FLAGS: [u32; 5] = [0xE0564A, 0xF2C230, 0x4A7FE0, 0x5CC06A, 0xFFF6E6];
        const PAPER: [u32; 4] = [0xFF9A3C, 0xE84A4A, 0xFFD25A, 0xFF8FB0];
        let cam = game.camera.pos;
        let world = &game.sim.world;
        let me = game.player_dragon();
        let (body, heads) = self.life.people();
        for e in crate::play::effects(play) {
            // Fireworks light the sky at night, seen from far across the islands.
            if e.kind == "fireworks" {
                if night > 0.2 && e.a.distance(cam) < 4000.0 {
                    self.fireworks(e.a, e.id, t, night, cam, fx);
                }
                continue;
            }
            if e.a.distance(cam) > 500.0 {
                continue;
            }
            let day = world.terrain.daylight(e.a + Vec3::Y);
            if e.kind == "vane" {
                // A pole, and a little gold dragon on top turning slowly in the wind.
                let ground = world.support_still(e.a.x, e.a.z, e.a.y);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, e.a.y - ground, 1.0),
                    Quat::IDENTITY,
                    Vec3::new(e.a.x, ground, e.a.z),
                );
                draws.push((
                    self.pole,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
                ));
                let turn = (t * 0.15 + e.id as f32).sin() * 1.4 + e.id as f32;
                let little =
                    zawnabout_sim::Dragon::new(zawnabout_sim::EntityId(0), Vec3::ZERO, 0.0);
                let mut parts = Vec::new();
                crate::pose::DragonVisual::new(&little).parts(1.0, &mut parts);
                let at = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.4),
                    Quat::from_rotation_y(turn),
                    e.a + Vec3::Y * 0.4,
                );
                let gold = Vec4::new(1.25, 0.95, 0.35, day);
                draws.extend(
                    parts
                        .iter()
                        .map(|(part, m, _)| (self.mesh(*part), Instance::tinted(at * *m, gold))),
                );
                continue;
            }
            if e.kind == "folk" {
                let to = me.pos - e.a;
                let face = if to.length() < 15.0 {
                    to.x.atan2(to.z)
                } else {
                    e.facing
                };
                let bob = (t * 3.0 + e.id as f32 * 1.7).sin().abs() * 0.12;
                let sway = (t * 1.3 + e.id as f32).sin() * 0.06;
                let m = Mat4::from_rotation_translation(
                    Quat::from_rotation_y(face + sway),
                    e.a + Vec3::Y * bob,
                );
                let shirt = linear(Rgb(LOCAL_SHIRTS[e.look as usize % LOCAL_SHIRTS.len()]), 1.0);
                draws.push((body, Instance::tinted(m, shirt.extend(day))));
                draws.push((
                    heads[e.look as usize % 3],
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
                ));
                continue;
            }
            // A pole at each end, and the string sagging between.
            for top in [e.a, e.b] {
                let ground = world.support_still(top.x, top.z, top.y);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, top.y - ground + 0.3, 1.0),
                    Quat::IDENTITY,
                    Vec3::new(top.x, ground, top.z),
                );
                draws.push((
                    self.pole,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
                ));
            }
            let span = e.b - e.a;
            let len = span.length();
            let lanterns = e.kind == "lanterns";
            let gap = if lanterns { 2.6 } else { 1.3 };
            let n = (len / gap).max(1.0) as usize;
            let yaw = span.x.atan2(span.z) - std::f32::consts::FRAC_PI_2;
            for k in 1..n {
                let u = k as f32 / n as f32;
                let sag = len * 0.04 * 4.0 * u * (1.0 - u);
                let at = e.a + span * u - Vec3::Y * sag;
                let swing = (t * 1.6 + k as f32 * 0.9 + e.id as f32).sin() * 0.12;
                let m = Mat4::from_rotation_translation(
                    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(swing),
                    at,
                );
                if lanterns {
                    let c = linear(Rgb(PAPER[(k + e.id) % PAPER.len()]), 1.0);
                    let glow = (day + night * 1.5).min(2.0);
                    draws.push((self.paper_lantern, Instance::tinted(m, c.extend(glow))));
                    if night > 0.05 && at.distance(cam) < 300.0 {
                        let p = at - Vec3::Y * 0.45;
                        let face =
                            Quat::from_rotation_arc(Vec3::Y, (cam - p).normalize_or(Vec3::Y));
                        let s = Mat4::from_scale_rotation_translation(Vec3::splat(1.6), face, p);
                        fx.push((
                            self.star,
                            Instance::tinted(s, (c * 1.4).extend(0.5 * night)),
                        ));
                    }
                } else {
                    let c = linear(Rgb(FLAGS[(k + e.id) % FLAGS.len()]), 1.0);
                    draws.push((self.flag, Instance::tinted(m, c.extend(day))));
                }
            }
        }
    }

    fn play(
        &self,
        game: &Game,
        t: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let Some(play) = &game.play else { return };
        let cam = game.camera.pos;
        let world = &game.sim.world;
        self.observe(game);
        let me = game.player_dragon();
        let model = |kind: &str| {
            self.treasures
                .iter()
                .find(|(k, _)| *k == kind)
                .map_or(self.heap, |(_, m)| *m)
        };
        let mut shinies = Vec::new();
        crate::play::shinies(play, &mut shinies);
        let gold = linear(Rgb(0xFFE7A0), 1.0);
        for s in shinies {
            let dist = s.pos.distance(cam);
            if dist > 160.0 {
                continue;
            }
            let day = world.terrain.daylight(s.pos + Vec3::Y);
            let turn = s.id as f32 * 2.39;
            let m = Mat4::from_rotation_translation(Quat::from_rotation_y(turn), s.pos);
            draws.push((
                model(s.kind),
                Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
            ));
            // A quick twinkle every few seconds, each find on its own beat, seen only
            // from close by. In the dark, a soft steady glow instead.
            let fade = 1.0 - smooth(50.0, 80.0, dist);
            if fade <= 0.0 {
                continue;
            }
            let phase = (t * 0.4 + s.id as f32 * 0.377).fract();
            let twinkle = (1.0 - (phase - 0.5).abs() * 9.0).max(0.0);
            let sun = day * (1.0 - night);
            let k = (twinkle * sun + (1.0 - sun) * 0.4) * fade;
            if k < 0.02 {
                continue;
            }
            let at = s.pos + Vec3::Y * 1.3;
            let to_cam = (cam - at).normalize_or(Vec3::Y);
            let size = 0.5 + 1.6 * twinkle * sun + 0.004 * dist;
            let face = Quat::from_rotation_arc(Vec3::Y, to_cam) * Quat::from_rotation_y(t * 0.6);
            let m = Mat4::from_scale_rotation_translation(Vec3::splat(size), face, at);
            fx.push((self.star, Instance::tinted(m, (gold * 1.6).extend(k))));
        }
        // Named locals, turned toward the dragon when it is near, and lost things.
        let lit = |id: &str| game.sim.beacon(id).is_some_and(|(_, lit)| lit);
        let (mut locals, mut lost) = (Vec::new(), Vec::new());
        let carried = crate::play::people(play, &lit, &mut locals, &mut lost);
        let (body, heads) = self.life.people();
        for l in &locals {
            if l.pos.distance(cam) > 450.0 {
                continue;
            }
            let to = me.pos - l.pos;
            let face = if to.length() < 15.0 {
                to.x.atan2(to.z)
            } else {
                l.facing
            };
            let sway = (t * 1.3 + l.id as f32).sin() * 0.04;
            // Someone with a favor hops about to catch the dragon's eye.
            let hop = if l.ask && to.length() < 80.0 {
                (t * 4.0 + l.id as f32).sin().abs() * 0.3
            } else {
                0.0
            };
            let m = Mat4::from_rotation_translation(
                Quat::from_rotation_y(face + sway),
                l.pos + Vec3::Y * hop,
            );
            let day = world.terrain.daylight(l.pos + Vec3::Y);
            let shirt = linear(Rgb(LOCAL_SHIRTS[l.look as usize % LOCAL_SHIRTS.len()]), 1.0);
            draws.push((body, Instance::tinted(m, shirt.extend(day))));
            draws.push((
                heads[l.look as usize % 3],
                Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
            ));
        }
        for item in &lost {
            if item.pos.distance(cam) > 300.0 {
                continue;
            }
            let m = Mat4::from_translation(item.pos);
            draws.push((model(item.kind), Instance::tinted(m, Vec4::ONE)));
            let k = 0.5 + 0.5 * (t * 2.0).sin();
            let at = item.pos + Vec3::Y * 1.3;
            let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
            // Big enough to spot from a way off, so "around here" is easy.
            let size = 1.0 + k * 1.2 + item.pos.distance(cam) * 0.02;
            let s = Mat4::from_scale_rotation_translation(Vec3::splat(size), face, at);
            fx.push((self.star, Instance::tinted(s, (gold * 1.6).extend(0.8))));
        }
        // A passenger sits in a basket on the dragon's back, just behind the shoulders.
        if let Some(rider) = crate::play::rider(play)
            && let Some(v) = game.visual(game.player)
        {
            let mut parts = Vec::new();
            v.parts(game.alpha(), &mut parts);
            if let Some((_, body_m, _)) =
                parts.iter().find(|(p, _, _)| *p == crate::pose::Part::Body)
            {
                let day = world.terrain.daylight(me.pos);
                let seat = *body_m
                    * Mat4::from_scale_rotation_translation(
                        Vec3::new(0.9, 0.7, 0.9),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 0.3, 0.25),
                    );
                draws.push((
                    self.saddle,
                    Instance::tinted(seat, Vec4::new(1.0, 1.0, 1.0, day)),
                ));
                let seat = *body_m
                    * Mat4::from_scale_rotation_translation(
                        Vec3::new(0.75, 0.55, 0.75),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 0.42, 0.25),
                    );
                let shirt = linear(
                    Rgb(LOCAL_SHIRTS[rider.look as usize % LOCAL_SHIRTS.len()]),
                    1.0,
                );
                let day = world.terrain.daylight(me.pos);
                draws.push((body, Instance::tinted(seat, shirt.extend(day))));
                let head = *body_m
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.75),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 0.1, 0.25),
                    );
                draws.push((
                    heads[rider.look as usize % 3],
                    Instance::tinted(head, Vec4::new(1.0, 1.0, 1.0, day)),
                ));
            }
        }
        // Whatever the passenger points at twinkles big, so "over there" is easy to see.
        if let Some((at, left)) = game.point_at {
            let k = 0.5 + 0.5 * (t * 4.0).sin();
            let at = at + Vec3::Y * 1.5;
            let size = 1.5 + k * 1.5 + at.distance(cam) * 0.012;
            let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y))
                * Quat::from_rotation_y(t * 1.5);
            let m = Mat4::from_scale_rotation_translation(Vec3::splat(size), face, at);
            fx.push((
                self.star,
                Instance::tinted(m, (gold * 1.8).extend(left.min(1.0))),
            ));
        }
        // Passengers who hopped off mid-air, drifting down under their umbrellas.
        for f in &game.floaters {
            let (at, _) = f.at();
            let day = world.terrain.daylight(at + Vec3::Y);
            let sway = (f.age * 1.3).sin() * 0.15;
            let m = Mat4::from_rotation_translation(Quat::from_rotation_z(sway), at);
            let shirt = linear(Rgb(LOCAL_SHIRTS[f.look as usize % LOCAL_SHIRTS.len()]), 1.0);
            draws.push((body, Instance::tinted(m, shirt.extend(day))));
            draws.push((
                heads[f.look as usize % 3],
                Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, day)),
            ));
            let u = m * Mat4::from_translation(Vec3::new(0.3, 1.2, 0.0));
            draws.push((
                self.umbrella,
                Instance::tinted(u, Vec4::new(1.0, 1.0, 1.0, day)),
            ));
        }
        self.set_effects(game, play, t, night, draws, fx);
        self.story_lamps(game, draws);
        // A carried thing rides in the dragon's arms.
        if let Some(kind) = carried {
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.6),
                Quat::from_rotation_y(me.heading),
                me.pos + Vec3::new(me.heading.sin(), 0.0, me.heading.cos()) * 1.3 - Vec3::Y * 0.4,
            );
            draws.push((model(kind), Instance::new(m)));
        }
        // A bubble over whoever the dragon could talk to.
        if let Some(who) = game.listener() {
            let at = who.bubble() + Vec3::Y * (t * 2.5).sin() * 0.08;
            let to = (cam - at).normalize_or(Vec3::Z);
            let face =
                Quat::from_rotation_arc(Vec3::Z, Vec3::new(to.x, 0.0, to.z).normalize_or(Vec3::Z));
            let m = Mat4::from_scale_rotation_translation(Vec3::splat(1.3), face, at);
            let ask = game.listener_asks() as usize;
            draws.push((self.bubbles[ask], Instance::new(m)));
        }
        if let Some(heap) = crate::play::heap(play)
            && heap.at.distance(cam) < 400.0
        {
            let ground = |x: f32, z: f32| world.support_still(x, z, heap.at.y + 3.0);
            let day = world.terrain.daylight(heap.at + Vec3::Y * 2.0);
            let tint = Vec4::new(1.0, 1.0, 1.0, day);
            let size = (0.5 + (heap.gold as f32).sqrt() * 0.14).min(6.0);
            if heap.gold > 0 {
                let at = Vec3::new(heap.at.x, ground(heap.at.x, heap.at.z), heap.at.z);
                let m =
                    Mat4::from_scale_rotation_translation(Vec3::splat(size), Quat::IDENTITY, at);
                draws.push((self.heap, Instance::tinted(m, tint)));
            }
            // The journal on its stand beside the heap.
            let (x, z) = (heap.at.x - size - 2.2, heap.at.z);
            let m = Mat4::from_translation(Vec3::new(x, ground(x, z), z));
            draws.push((self.lectern, Instance::tinted(m, tint)));
            // Treasures in rings round the heap, the newest furthest out.
            for (k, kind) in heap.kinds.iter().enumerate() {
                let ring = k / 10;
                let a = (k % 10) as f32 / 10.0 * std::f32::consts::TAU + ring as f32 * 0.31;
                let r = size * 1.1 + 1.6 + ring as f32 * 1.7;
                let (x, z) = (heap.at.x + a.cos() * r, heap.at.z + a.sin() * r);
                let m = Mat4::from_rotation_translation(
                    Quat::from_rotation_y(-a + std::f32::consts::FRAC_PI_2),
                    Vec3::new(x, ground(x, z), z),
                );
                draws.push((model(kind), Instance::tinted(m, tint)));
            }
        }
    }

    /// Tufts of grass on every lawn near the camera, swaying. They sit on a fixed world
    /// grid, so they stay put as the camera moves, and shrink away at the edge.
    fn grass(&self, game: &Game, draws: &mut Vec<(MeshId, Instance)>) {
        let terrain = &game.sim.world.terrain;
        let cam = game.camera.pos;
        if cam.y - terrain.height(cam.x, cam.z) > 90.0 {
            return;
        }
        // Thick close by, and a sparser ring of bigger tufts out to where the eye still
        // reads the ground as grass, so it never seems to start right underfoot.
        let rings = [
            (&self.tufts, 6.0f32, 2.4f32, 0.0f32, 50.0f32, 1.0f32),
            (&self.far_tufts, 24.0, 5.0, 44.0, 100.0, 1.45),
        ];
        for (cache, cell, step, inner, range, grow) in rings {
            let key = ((cam.x / cell).floor() as i32, (cam.z / cell).floor() as i32);
            let mut cache = cache.borrow_mut();
            if cache.key != key {
                let lawn = &mut *cache;
                lawn.key = key;
                lawn.out.clear();
                let center = Vec2::new(key.0 as f32 + 0.5, key.1 as f32 + 0.5) * cell;
                // Where tufts stand only depends on the ground, so each tile of them is
                // found once and kept while it stays in range. Moving on only finds the
                // tiles coming into view.
                let side = step * LAWN_TILE as f32;
                let tile = |v: f32| (v / side).floor() as i32;
                let (lo, hi) = (center - Vec2::splat(range), center + Vec2::splat(range));
                lawn.tiles.retain(|&(tx, tz), _| {
                    (tile(lo.x) - 1..=tile(hi.x) + 1).contains(&tx)
                        && (tile(lo.y) - 1..=tile(hi.y) + 1).contains(&tz)
                });
                for tz in tile(lo.y)..=tile(hi.y) {
                    for tx in tile(lo.x)..=tile(hi.x) {
                        let tufts = lawn
                            .tiles
                            .entry((tx, tz))
                            .or_insert_with(|| lawn_tile(terrain, step, tx, tz));
                        for t in tufts.iter() {
                            let d = Vec2::new(t.at.x, t.at.z).distance(center);
                            if d > range || d < inner {
                                continue;
                            }
                            let fade = (1.0 - smooth(range * 0.7, range, d))
                                * smooth(inner * 0.9, inner + 8.0, d)
                                    .max((inner == 0.0) as u8 as f32);
                            let size = t.size * fade * grow;
                            let m = Mat4::from_scale_rotation_translation(
                                Vec3::new(size, size * t.tall, size),
                                Quat::from_rotation_y(t.turn),
                                t.at,
                            );
                            lawn.out.push(Instance::tinted(m, t.tint.extend(1.0)));
                        }
                    }
                }
            }
            draws.extend(cache.out.iter().map(|i| (self.tuft, *i)));
        }
    }

    /// Ferries, boats, planes and cars where they are this frame, between ticks.
    fn vehicles(&self, game: &Game, open_air: bool, draws: &mut Vec<(MeshId, Instance)>) {
        let traffic = &game.sim.world.traffic;
        let terrain = &game.sim.world.terrain;
        let time = (game.sim.tick as f32 + game.alpha()) * zawnabout_sim::DT;
        let cam = game.camera.pos;
        for (i, (v, mesh)) in traffic.vehicles.iter().zip(&self.vehicles).enumerate() {
            let range = if matches!(
                v.kind,
                zawnabout_sim::town::Kind::Car
                    | zawnabout_sim::town::Kind::Train
                    | zawnabout_sim::town::Kind::LiftChair
                    | zawnabout_sim::town::Kind::LiftCabin
                    | zawnabout_sim::town::Kind::FerrisCar
                    | zawnabout_sim::town::Kind::RaceCar
                    | zawnabout_sim::town::Kind::CoasterCar
            ) {
                1500.0
            } else {
                // Big ships and planes show a long way off, small boats not so far.
                (v.size.max_element() * 220.0).clamp(1500.0, 6000.0)
            };
            // Where it was last tick says whether it is in range, before the costlier
            // pose between ticks. It cannot have moved far since.
            let last = traffic.poses.get(i).map_or(Vec3::ZERO, |p| p.pos);
            if last.distance(cam) > range + 20.0 {
                continue;
            }
            let p = traffic.pose(v, time);
            if p.pos.distance(cam) > range {
                continue;
            }
            let a = p.axis();
            let basis = Mat4::from_cols(
                Vec4::new(a.x, 0.0, a.y, 0.0),
                Vec4::Y,
                Vec4::new(-a.y, 0.0, a.x, 0.0),
                p.pos.extend(1.0),
            );
            let m = basis * Mat4::from_rotation_z(p.pitch) * Mat4::from_rotation_x(p.bank);
            let day = terrain.daylight(p.pos + Vec3::Y * 2.0);
            if !open_air && day > 0.5 {
                continue;
            }
            let paint = {
                use zawnabout_sim::town::Kind;
                use zawnabout_sim::traffic::TOUR_CAR;
                match v.kind {
                    Kind::Car if v.variant < CAR_KINDS => {
                        CAR_PAINT[v.variant as usize % CAR_PAINT.len()]
                    }
                    Kind::FerrisCar | Kind::RaceCar | Kind::CoasterCar => {
                        FAIR_PAINT[v.variant as usize % FAIR_PAINT.len()]
                    }
                    Kind::Train if (TOUR_CAR..TOUR_CAR + 3).contains(&v.variant) => {
                        FAIR_PAINT[(v.variant - TOUR_CAR) as usize % 3]
                    }
                    _ => Vec3::ONE,
                }
            };
            draws.push((*mesh, Instance::tinted(m, paint.extend(day))));
        }
    }

    /// Fire, sparks and light shafts, and the point lights that go with them.
    fn effects(
        &self,
        game: &Game,
        caves: bool,
        t: f32,
        dark: &[Vec4],
    ) -> (Vec<(MeshId, Instance)>, Vec<Light>) {
        let cam = game.camera.pos;
        let world = &game.sim.world;
        let mut fx = Vec::new();
        let mut lights = Vec::new();
        let fire = linear(Rgb(0xFF9A4A), 1.0);

        // Breath lights everything in front of the snout, always first in line.
        let d = game.player_dragon();
        if d.breath > 0.02 {
            let (mouth, dir) = d.breath_ray();
            let flick = 1.0 + 0.15 * (t * 23.0).sin();
            lights.push((mouth + dir * 12.0, 45.0, fire * 3.0 * d.breath * flick));
        }

        let mut near = Vec::new();
        for (i, b) in world.beacons.iter().enumerate() {
            if !game.sim.lit[i] {
                continue;
            }
            let dist = b.flame.distance(cam);
            if dist > 1500.0 {
                continue;
            }
            let k = i as f32;
            let flick = 0.08 * (t * 13.0 + k).sin() + 0.06 * (t * 7.3 + k * 2.0).sin();
            for (scale, off, spin) in [(1.0, Vec3::ZERO, 0.0), (0.6, Vec3::new(0.5, 0.0, 0.3), 1.3)]
            {
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.3, 1.7 * (1.0 + flick * 1.6), 1.3) * scale,
                    Quat::from_rotation_y(t * 0.7 + k + spin),
                    b.flame - Vec3::Y * 0.4 + off,
                );
                fx.push((self.flame, Instance::new(m)));
            }
            near.push((b.flame + Vec3::Y * 1.5, 48.0, fire * 2.2 * (1.0 + flick)));
        }
        // A story's lamps, burning once lit.
        if let Some(play) = &game.play {
            for (k, (foot, lit)) in crate::play::lamps(play).into_iter().enumerate() {
                let dist = foot.distance(cam);
                if !lit || dist > 1500.0 {
                    continue;
                }
                let flame = foot + Vec3::Y * crate::play::LAMP_FLAME;
                let k = k as f32 * 1.7;
                let flick = 0.08 * (t * 13.0 + k).sin() + 0.06 * (t * 7.3 + k * 2.0).sin();
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.7, 1.0 * (1.0 + flick * 1.6), 0.7),
                    Quat::from_rotation_y(t * 0.7 + k),
                    flame - Vec3::Y * 0.3,
                );
                fx.push((self.flame, Instance::new(m)));
                near.push((flame + Vec3::Y * 1.0, 30.0, fire * 1.6 * (1.0 + flick)));
            }
        }
        // Street lamps at night, and cave lamps always: a warm pool on the ground and a
        // soft halo, and the nearest ones light what is around them.
        let night = game.sky().night;
        let warm = linear(Rgb(0xFFC874), 1.0);
        for (lamp, foot, under) in &self.lamps {
            let on = if *under {
                1.0
            } else if in_dark(dark, *lamp) {
                0.0
            } else {
                night
            };
            let dist = lamp.distance(cam);
            if on < 0.05 || (*under && !caves) || dist > 700.0 {
                continue;
            }
            let pool = Mat4::from_scale_rotation_translation(
                Vec3::new(9.0, 1.0, 9.0),
                Quat::IDENTITY,
                *foot,
            );
            fx.push((self.pool, Instance::tinted(pool, (warm * 0.5).extend(on))));
            let to_cam = (cam - *lamp).normalize_or(Vec3::Y);
            let size = 1.6 + dist * 0.004;
            let halo = Mat4::from_scale_rotation_translation(
                Vec3::new(size, 1.0, size),
                Quat::from_rotation_arc(Vec3::Y, to_cam),
                *lamp,
            );
            fx.push((self.pool, Instance::tinted(halo, (warm * 0.9).extend(on))));
            if dist < 250.0 {
                near.push((*lamp, 22.0, warm * 1.6 * on));
            }
        }
        // Headlights and tail lights on cars, buses and trains after dark.
        if night > 0.15 {
            use zawnabout_sim::town::Kind;
            let traffic = &world.traffic;
            let time = (game.sim.tick as f32 + game.alpha()) * zawnabout_sim::DT;
            let (head, tail) = (linear(Rgb(0xFFF2D0), 1.0), linear(Rgb(0xFF2A1A), 1.0));
            for v in &traffic.vehicles {
                if !matches!(v.kind, Kind::Car | Kind::Train) {
                    continue;
                }
                let p = traffic.pose(v, time);
                let dist = p.pos.distance(cam);
                if dist > 600.0 {
                    continue;
                }
                let a = p.axis();
                let (fwd, side) = (Vec3::new(a.x, 0.0, a.y), Vec3::new(-a.y, 0.0, a.x));
                let to_cam = (cam - p.pos).normalize_or(Vec3::Y);
                let face = Quat::from_rotation_arc(Vec3::Y, to_cam);
                let (len, wide) = (v.size.x * 0.5, v.size.z * 0.36);
                for (end, color, size) in [(len, head, 0.55), (-len, tail, 0.35)] {
                    // Only the end facing the camera shows its lights.
                    if fwd.dot(to_cam) * end < 0.0 {
                        continue;
                    }
                    for s in [-1.0f32, 1.0] {
                        let at = p.pos + fwd * end + side * wide * s + Vec3::Y * 0.75;
                        let r = size + dist * 0.0025;
                        let m =
                            Mat4::from_scale_rotation_translation(Vec3::new(r, 1.0, r), face, at);
                        fx.push((self.pool, Instance::tinted(m, color.extend(night))));
                    }
                }
            }
        }
        // Landmark lamps and fair bulbs: a soft halo each, the fair's in colors that chase
        // round the strings.
        const FAIR: [u32; 5] = [0xFFD27A, 0xFF6A5A, 0x6AB8FF, 0x7CE08A, 0xFFF4D6];
        let t = game.time as f32;
        for (k, &(p, r, always, fair)) in self.bulbs.iter().enumerate() {
            let on = if always { 1.0 } else { night };
            let dist = p.distance(cam);
            if on < 0.05 || (always && !caves) || dist > 900.0 {
                continue;
            }
            let (color, pulse) = if fair {
                let chase = (t * 3.0 - k as f32 * 0.7).sin() * 0.5 + 0.5;
                (linear(Rgb(FAIR[k % FAIR.len()]), 1.0), 0.45 + 0.55 * chase)
            } else {
                (warm, 1.0)
            };
            let to_cam = (cam - p).normalize_or(Vec3::Y);
            let size = (r * 5.0).clamp(0.6, 2.2) + dist * 0.003;
            let halo = Mat4::from_scale_rotation_translation(
                Vec3::new(size, 1.0, size),
                Quat::from_rotation_arc(Vec3::Y, to_cam),
                p,
            );
            fx.push((
                self.pool,
                Instance::tinted(halo, (color * 0.9).extend(on * pulse)),
            ));
        }
        if caves {
            near.extend(self.glowers.iter().copied());
            // A pool of daylight where each shaft lands.
            let day = linear(Rgb(0xFFF1D6), 1.0) * (1.0 - night * 0.75);
            for (_, _, _, foot) in &self.shafts {
                near.push((*foot + Vec3::Y * 12.0, 70.0, day * 1.3));
            }
        }
        // The nearest lights that reach the camera's surroundings win.
        near.retain(|(p, r, _)| p.distance(cam) - r < 350.0);
        near.sort_by(|a, b| (a.0.distance(cam) - a.1).total_cmp(&(b.0.distance(cam) - b.1)));
        lights.extend(
            near.into_iter()
                .take(crate::render::MAX_LIGHTS - lights.len()),
        );

        for p in &game.fx.puffs {
            let u = p.t();
            let (color, alpha, size) = if p.water {
                (
                    Vec3::new(0.85, 0.94, 1.0) * 1.1,
                    (1.0 - u) * 0.7,
                    0.3 + 0.25 * zawnabout_sim::shape::hash(p.seed),
                )
            } else if p.spark {
                (
                    Vec3::new(1.0, 0.8, 0.4) * 2.0,
                    (1.0 - u) * 0.9,
                    0.22 * (1.0 - u * 0.6),
                )
            } else {
                let hot = Vec3::new(1.0, 0.95, 0.65);
                let warm = Vec3::new(1.0, 0.5, 0.15);
                let cool = Vec3::new(0.75, 0.2, 0.12);
                let c = if u < 0.35 {
                    hot.lerp(warm, u / 0.35)
                } else {
                    warm.lerp(cool, (u - 0.35) / 0.65)
                };
                let wob = zawnabout_sim::shape::hash(p.seed) * 0.5;
                (c * 1.8, (1.0 - u).powf(1.4) * 0.75, 0.5 + u * 3.2 + wob)
            };
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(size),
                Quat::from_rotation_y(p.seed as f32 + u * 3.0),
                p.pos,
            );
            fx.push((self.ember, Instance::tinted(m, color.extend(alpha))));
        }

        if caves {
            let dim = 1.0 - night * 0.75;
            for (id, c, r, _) in &self.shafts {
                if c.distance(cam) - r < 1200.0 {
                    fx.push((
                        *id,
                        Instance::tinted(Mat4::IDENTITY, Vec4::new(dim, dim, dim, 1.0)),
                    ));
                }
            }
        }
        (fx, lights)
    }

    pub fn render(&mut self, game: &Game, target: &wgpu::TextureView) {
        let frame = self.prepare(game);
        crate::prof::time("encode", || self.renderer.render(target, &frame));
    }

    /// Whether `frame` shows just what the last one did, with the game halted (the
    /// journal, a card, the pause) so the host can leave the picture up instead of drawing
    /// it all again.
    pub fn unchanged(&mut self, game: &Game, frame: &Frame) -> bool {
        if !game.halted() || game.wants_snap() {
            self.still = None;
            return false;
        }
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytemuck::cast_slice::<_, u8>(&frame.hud).hash(&mut h);
        for v in frame.view_proj.to_cols_array() {
            v.to_bits().hash(&mut h);
        }
        self.renderer.size().hash(&mut h);
        let key = h.finish();
        let same = self.still == Some(key);
        self.still = Some(key);
        same
    }

    /// Draws the next frame even if nothing changed, so the host can read it back.
    pub fn redraw(&mut self) {
        self.still = None;
    }

    /// Everything to draw this frame.
    pub fn prepare(&mut self, game: &Game) -> Frame {
        use crate::prof::time;
        self.load_hud_map(game);
        if self.kin_meshes.len() != game.kin.len() {
            self.kin_meshes = game
                .kin
                .iter()
                .map(|k| {
                    let mut m = models::dragon();
                    m.recolor(k.tint);
                    let r = &mut self.renderer;
                    DragonIds {
                        body: r.add_mesh(&m.body),
                        neck: r.add_mesh(&m.neck),
                        head: r.add_mesh(&m.head),
                        eye: r.add_mesh(&m.eye),
                        tail: r.add_mesh(&m.tail),
                        plume: r.add_mesh(&m.plume),
                        whisker: r.add_mesh(&m.whisker),
                        mane: r.add_mesh(&m.mane),
                        leg: r.add_mesh(&m.leg),
                        arm: [r.add_mesh(&m.arm[0]), r.add_mesh(&m.arm[1])],
                        hand: [r.add_mesh(&m.hand[0]), r.add_mesh(&m.hand[1])],
                        star: r.add_mesh(&m.star),
                    }
                })
                .collect();
        }
        self.load_pics(game);
        // Halted, the view stands still and the chunks around it are long done.
        if !game.halted() {
            time("terrain", || {
                self.update_terrain(game, self.chunk_budget, CHUNK_SECS)
            });
        }
        time("frame", || self.frame(game))
    }

    /// What the world sounds like this frame, for the host's [`crate::audio::Synth`].
    pub fn sound(&mut self, game: &Game) -> crate::audio::Mix {
        self.listener.listen(game, &self.weather, &self.falls)
    }

    /// Puts postcard pictures in the atlas when they change.
    fn load_pics(&mut self, game: &Game) {
        let Some(play) = &game.play else { return };
        let Some((version, pics)) = crate::play::pics(play, self.pics_version) else {
            return;
        };
        self.pics_version = version;
        let c = crate::hud::CARD_CELL;
        for (i, rgba) in pics {
            let (x, y) = crate::hud::card_slot(i);
            self.renderer.write_hud_rect(x, y, c.0 - 4, c.1 - 4, &rgba);
        }
    }

    /// Draws the HUD's map the first time the HUD is on, off the main thread where there
    /// are threads, and puts it in the atlas when done.
    fn load_hud_map(&mut self, game: &Game) {
        if self.hud_map.1 {
            return;
        }
        if !game.hud.on {
            // No HUD map here, so the journal gets its own, a little at a time.
            let terrain = &game.sim.world.terrain;
            let job = self
                .book_map
                .get_or_insert_with(|| crate::map::MapJob::world(terrain, BOOK_MAP));
            if job.step(terrain, 6) {
                self.renderer
                    .write_hud_rect(0, 0, BOOK_MAP, BOOK_MAP, &job.out);
                self.book_map = None;
                self.hud_map.1 = true;
                game.map_px.set(BOOK_MAP as f32);
            }
            return;
        }
        let draw =
            |t: &zawnabout_sim::terrain::Terrain| crate::map::render(t, crate::hud::MAP, false);
        match &self.hud_map.0 {
            None => {
                let terrain = game.sim.world.terrain.clone();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(draw(&terrain));
                    });
                    self.hud_map.0 = Some(rx);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.renderer.set_hud_map(&draw(&terrain));
                    self.hud_map.1 = true;
                    game.map_px.set(crate::hud::MAP as f32);
                }
            }
            Some(rx) => {
                if let Ok(rgba) = rx.try_recv() {
                    self.renderer.set_hud_map(&rgba);
                    self.hud_map.1 = true;
                    game.map_px.set(crate::hud::MAP as f32);
                }
            }
        }
    }

    /// Plays `frames` frames of the game, rendering each offscreen, and reports the CPU
    /// time each part of a frame took on average. Native only.
    pub fn profile(&mut self, game: &mut Game, frames: u32) -> String {
        let (w, h) = self.renderer.size();
        let tex = self
            .renderer
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("profile"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.renderer.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
        let view = tex.create_view(&Default::default());
        crate::prof::enable();
        crate::prof::report();
        // Each frame: how long the GPU took, what was drawn, and what was uploaded.
        let mut each = Vec::new();
        for _ in 0..frames {
            crate::prof::begin_frame();
            crate::prof::time("sim", || game.advance(1.0 / 60.0));
            let sent = self.renderer.uploaded;
            self.renderer.largest = 0;
            self.render(game, &view);
            // Waiting for the GPU to finish counts too, so a slow GPU shows.
            let start = crate::prof::clock();
            crate::prof::time("gpu", || {
                self.renderer
                    .device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("poll device")
            });
            let s = &self.renderer.stats;
            each.push((
                (crate::prof::clock() - start) * 1000.0,
                s.triangles,
                s.draws,
                self.renderer.uploaded - sent,
                self.renderer.largest,
            ));
            crate::prof::end_frame();
        }
        let mut order: Vec<usize> = (0..each.len()).collect();
        order.sort_by(|a, b| each[*b].0.total_cmp(&each[*a].0));
        let mut out = crate::prof::report();
        let median = order.get(order.len() / 2).map_or(0.0, |&i| each[i].0);
        out.push_str(&format!(
            "
 gpu median {median:.1}ms, slowest frames:"
        ));
        for &i in order.iter().take(5) {
            let (ms, tris, draws, up, big) = each[i];
            out.push_str(&format!(
                " #{i} {ms:.0}ms {tris} tris {draws} draws {}kB up (biggest {}kB);",
                up / 1024,
                big / 1024
            ));
        }
        out
    }

    /// Renders one frame offscreen and reads it back as tightly packed RGBA8. Blocks,
    /// so native only.
    pub fn capture(&mut self, game: &Game) -> Vec<u8> {
        // Shots always show finished terrain.
        while !self
            .terrain
            .settled(&game.sim.world.terrain, game.camera.pos)
        {
            self.update_terrain(game, usize::MAX, f64::INFINITY);
        }
        let (w, h) = self.renderer.size();
        let device = &self.renderer.device;
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.renderer.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        self.render(game, &view);

        let device = &self.renderer.device;
        let row = (w * 4).div_ceil(256) * 256;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.renderer.queue.submit([enc.finish()]);
        buf.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("map readback"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll device");
        let data = buf.slice(..).get_mapped_range().expect("mapped range");
        let bgra = matches!(
            self.renderer.format,
            wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
        );
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let line = &data[(y * row) as usize..(y * row + w * 4) as usize];
            for px in line.as_chunks::<4>().0 {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        out
    }
}

/// Cave light near `p`: each kind of cavern has its own color, blended by how near each
/// one is, so passing from one to the next fades.
fn cave_tint(terrain: &zawnabout_sim::terrain::Terrain, p: Vec3) -> Vec3 {
    use zawnabout_sim::terrain::{Decor, VoidShape, void_sdf};
    let base = linear(Rgb(0xC6BCCE), 0.95);
    let reach = 250.0;
    let list = terrain.voids_in(p - Vec3::splat(reach), p + Vec3::splat(reach));
    let (mut sum, mut weight) = (Vec3::ZERO, 0.0);
    for i in list {
        let v = &terrain.def.voids[i as usize];
        if matches!(
            v.shape,
            VoidShape::Tube | VoidShape::Basin | VoidShape::Block
        ) {
            continue;
        }
        let w = (1.0 - void_sdf(v, p).max(0.0) / reach).max(0.0);
        if w <= 0.0 {
            continue;
        }
        let c = match v.decor {
            Decor::Crystal => linear(Rgb(0xA6D2F2), 0.95),
            Decor::Shroom => linear(Rgb(0xD2B2EC), 0.95),
            Decor::Ember => linear(Rgb(0xF2BC94), 0.95),
            Decor::Ruins => linear(Rgb(0xDCCCAE), 0.95),
            Decor::Tiled => linear(Rgb(0xFFF0D8), 1.0),
            Decor::Mixed | Decor::Bare => base,
        };
        sum += c * w;
        weight += w;
    }
    if weight <= 0.0 {
        return base;
    }
    // Between caverns the plain light fills in.
    base.lerp(sum / weight, weight.min(1.0))
}

/// Beams of daylight down the steep part of every mouth marked as letting light in, then
/// straight down to the floor below.
fn light_shafts(
    terrain: &zawnabout_sim::terrain::Terrain,
) -> Vec<(crate::mesh::MeshBuilder, Vec3)> {
    let mut mouths: Vec<(&str, Vec<Vec3>, f32)> = Vec::new();
    for v in terrain.def.voids.iter().filter(|v| v.light) {
        let name = v.id.split('#').next().unwrap_or(v.id);
        let (a, b) = (Vec3::from(v.a), Vec3::from(v.b));
        match mouths.last_mut() {
            Some((n, pts, _)) if *n == name => pts.push(b),
            _ => mouths.push((name, vec![a, b], v.r)),
        }
    }
    let mut out = Vec::new();
    for (_, mut pts, r) in mouths {
        // Light enters at the high end.
        if pts[0].y < pts[pts.len() - 1].y {
            pts.reverse();
        }
        let mut path = vec![pts[0]];
        for w in pts.windows(2) {
            let drop = w[0].y - w[1].y;
            if drop < (w[1] - w[0]).length() * 0.5 {
                break;
            }
            path.push(w[1]);
        }
        let end = path[path.len() - 1];
        let floor = terrain.support(end.x, end.z, end.y - 2.0);
        if end.y - floor > 8.0 && end.y - floor < 600.0 {
            path.push(Vec3::new(end.x, floor, end.z));
        }
        if path.len() < 2 {
            continue;
        }
        let total: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut m = crate::mesh::MeshBuilder::new();
        let mut along = 0.0;
        for w in path.windows(2) {
            let len = w[0].distance(w[1]);
            let fade = |s: f32| 0.16 * (1.0 - 0.7 * s / total.max(1.0));
            models::light_beam(&mut m, w[0], w[1], r * 0.55, fade(along), fade(along + len));
            along += len;
        }
        out.push((m, path[path.len() - 1]));
    }
    out
}

/// Named locals' clothes, by their look.
const LOCAL_SHIRTS: [u32; 9] = [
    0x7FB069, 0xE07A5F, 0x3D5A80, 0xF2CC8F, 0x9C6ADE, 0x2A2E4A, 0x81B29A, 0xC8553D, 0x5B8E7D,
];

/// Grass tufts kept around the camera: the tiles of them found so far, and this cell's
/// tufts faded to the ring.
#[derive(Default)]
struct Lawn {
    key: (i32, i32),
    tiles: HashMap<(i32, i32), Vec<Tuft>>,
    out: Vec<Instance>,
}

/// One grass tuft where it stands, before fading.
struct Tuft {
    at: Vec3,
    size: f32,
    tall: f32,
    turn: f32,
    tint: Vec3,
}

/// Grass spots along each side of a lawn tile.
const LAWN_TILE: i32 = 8;

/// The tufts in one tile of the grid `step` apart: on lawn, off water, roads, pads and
/// cliff edges.
fn lawn_tile(terrain: &zawnabout_sim::terrain::Terrain, step: f32, tx: i32, tz: i32) -> Vec<Tuft> {
    let mut out = Vec::new();
    for j in tz * LAWN_TILE..(tz + 1) * LAWN_TILE {
        for i in tx * LAWN_TILE..(tx + 1) * LAWN_TILE {
            let seed = (i as u32).wrapping_mul(73_856_093)
                ^ (j as u32).wrapping_mul(19_349_663)
                ^ (step * 10.0) as u32;
            let r = |k: u32| zawnabout_sim::shape::hash(seed.wrapping_add(k));
            let x = (i as f32 + r(1)) * step;
            let z = (j as f32 + r(2)) * step;
            let y = terrain.height(x, z);
            let at = Vec3::new(x, y, z);
            if terrain.water(x, z, y).is_some()
                || terrain.road_spot(x, z, 1.5).is_some()
                || terrain.on_pad(x, z)
            {
                continue;
            }
            let Some(color) = crate::terrain_mesh::lawn(terrain, at, terrain.normal(x, z)) else {
                continue;
            };
            // The drawn ground rounds off cliff edges, so keep back from them.
            if terrain.rim(x, z, 5.0) > 1.2 {
                continue;
            }
            out.push(Tuft {
                at: at - Vec3::Y * 0.05,
                size: 0.8 + r(3) * 0.7,
                tall: 0.8 + r(4) * 0.5,
                turn: r(5) * std::f32::consts::TAU,
                tint: linear(color, 1.0) * (0.85 + r(6) * 0.3),
            });
        }
    }
    out
}

fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much to shift a tree's leaves: a little each way for every tree, and for one
/// broadleaf in thirty, autumn gold or red.
fn leaf_tint(kind: PropKind, id: u32) -> Option<[f32; 3]> {
    if !matches!(
        kind,
        PropKind::RoundTree | PropKind::PineTree | PropKind::Palm | PropKind::Bush
    ) {
        return None;
    }
    let h = |k: u32| zawnabout_sim::shape::hash(id.wrapping_mul(2654435761).wrapping_add(k));
    if kind == PropKind::RoundTree && h(0) < 0.035 {
        return Some(if h(1) < 0.5 {
            [2.9, 1.0, 0.4]
        } else {
            [2.6, 0.5, 0.55]
        });
    }
    let warm = h(2) - 0.5;
    let bright = 0.88 + h(3) * 0.24;
    Some([
        bright * (1.0 + warm * 0.25),
        bright,
        bright * (1.0 - warm * 0.3),
    ])
}

/// Shifts a green vertex by a leaf tint, leaving trunks and fruit alone.
fn tint_leaf(v: &mut crate::mesh::Vertex, tint: Option<[f32; 3]>) {
    let Some(t) = tint else { return };
    let [r, g, b, _] = v.color;
    if g as u16 <= r as u16 + 8 || g <= b {
        return;
    }
    v.color[0] = (r as f32 * t[0]).min(255.0) as u8;
    v.color[1] = (g as f32 * t[1]).min(255.0) as u8;
    v.color[2] = (b as f32 * t[2]).min(255.0) as u8;
}

/// Where the rainbow's end is: an islet in the far corner of the sea.
pub const RAINBOW_END: Vec3 = Vec3::new(5150.0, 0.0, -5150.0);

/// Toward the middle of the rainbow's circle, seen from `cam`. A real one centers on
/// the point right away from the sun, `anti`. This one keeps that height, so it is just
/// as big, but swings round so one of its ends comes down on the rainbow's end.
pub fn rainbow_center(anti: Vec3, cam: Vec3) -> Vec3 {
    // Half way across the bands, in radians.
    const ARC: f32 = 0.72;
    let e = anti.y.clamp(-0.99, 0.99).asin();
    let reach = ARC.cos() / e.cos();
    if !(-1.0..=1.0).contains(&reach) {
        return anti;
    }
    let side = reach.acos();
    let to = RAINBOW_END - cam;
    let corner = to.x.atan2(to.z);
    let sun_side = anti.x.atan2(anti.z);
    let gap = |a: f32| {
        let d = (a - sun_side).rem_euclid(std::f32::consts::TAU);
        d.min(std::f32::consts::TAU - d)
    };
    let a = [corner + side, corner - side]
        .into_iter()
        .min_by(|x, y| gap(*x).total_cmp(&gap(*y)))
        .unwrap_or(sun_side);
    Vec3::new(e.cos() * a.sin(), e.sin(), e.cos() * a.cos())
}

/// Whether `p` is where a story has put the lights out.
pub fn in_dark(dark: &[Vec4], p: Vec3) -> bool {
    dark.iter()
        .any(|d| Vec2::new(d.x - p.x, d.y - p.z).length() < d.z)
}

#[cfg(test)]
mod rainbow_tests {
    use super::*;

    /// Wherever you watch from, one end of the rainbow comes down on the horizon right
    /// toward the rainbow's end, and the bow stays as high as a real one would be.
    #[test]
    fn one_end_on_the_corner() {
        for cam in [
            Vec3::new(-1800.0, 50.0, 800.0),
            Vec3::new(3000.0, 300.0, 2000.0),
        ] {
            for sun in [Vec3::new(0.6, 0.2, 0.3), Vec3::new(-0.5, 0.35, -0.6)] {
                let anti = -sun.normalize();
                let c = rainbow_center(anti, cam);
                let to = (RAINBOW_END - cam) * Vec3::new(1.0, 0.0, 1.0);
                let angle = c.dot(to.normalize()).acos();
                assert!((angle - 0.72).abs() < 0.01, "{angle}");
                assert!((c.y - anti.y).abs() < 1e-4);
            }
        }
    }
}
