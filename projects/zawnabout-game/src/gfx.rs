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
    dragon: DragonIds,
    terrain: TerrainMeshes,
    /// Water meshes with bounding spheres, and whether each is in a cave.
    water: Vec<(MeshId, Vec3, f32, bool)>,
    spray: MeshId,
    falls: Vec<(Vec3, f32)>,
    flame: MeshId,
    ember: MeshId,
    /// Light streaming down sky openings into caves, with bounding spheres and where
    /// each lands.
    shafts: Vec<(MeshId, Vec3, f32, Vec3)>,
    /// Big crystals that light their surroundings.
    glowers: Vec<Light>,
    life: crate::life::Life,
    cave_life: crate::cave_life::CaveLife,
    sea_life: crate::sea_life::SeaLife,
    wildlife: crate::wildlife::Wildlife,
    /// Dark plugs a little way into every cave mouth, so from outside a mouth looks like
    /// darkness instead of a hole onto the sky where the deep walls are not drawn.
    plugs: Vec<Mat4>,
    /// Where each cave mouth opens to the sky. Near one, the outside draws in full even
    /// from inside, so the view out is the real world.
    mouths: Vec<Vec3>,
    plug: MeshId,
    /// Grass tufts around the camera, rebuilt when it moves to another cell.
    tuft: MeshId,
    tufts: std::cell::RefCell<((i32, i32), Vec<Instance>)>,
    /// One mesh per vehicle, in the order of the sim's traffic.
    vehicles: Vec<MeshId>,
    /// Every lamp's head and foot, and whether it is underground where it always burns.
    lamps: Vec<(Vec3, Vec3, bool)>,
    pool: MeshId,
    /// Terrain chunks built per frame at most. Slow devices want 1.
    pub chunk_budget: usize,
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

/// Props are batched per square of this size (m), so far squares can be skipped.
const PROP_TILE: f32 = 1024.0;
const CLOUD_TILE: f32 = 3000.0;
/// Tiles per side of a far group, and how far out (times the detail range) the group
/// draws as one.
const GROUP: i32 = 4;
const GROUP_RANGE: f32 = 1.5;
/// Flowers, bushes and small rocks bake per square of this size (m), and draw out to
/// this many times the prop range.
const SMALL_TILE: f32 = 256.0;
const SMALL_REACH: f32 = 0.14;
/// Water merges per square of this size (m).
const WATER_TILE: f32 = 2048.0;

use zawnabout_sim::traffic::CAR_KINDS;

/// The car mesh painted white, which the tint colors.
const CAR_WHITE: u8 = 3;
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
            crate::town_mesh::build(&world.terrain, &world.buildings)
                .into_iter()
                .partition(|(_, t)| t.cave);
        let mut tiles: HashMap<(i32, i32), crate::town_mesh::Tile> = open.into_iter().collect();
        // Far meshes for the merged groups: buildings as they are, trees as spires.
        let mut group_far: HashMap<(i32, i32), MeshBuilder> =
            tiles.iter().map(|(k, t)| (*k, t.far.clone())).collect();
        let mut cloud_tiles: HashMap<(i32, i32), (MeshBuilder, MeshBuilder)> = HashMap::new();
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
            if p.kind == PropKind::Cloud {
                // Clouds bake per big tile too, every shape in one mesh.
                let key = (
                    (p.pos.x / CLOUD_TILE).floor() as i32,
                    (p.pos.z / CLOUD_TILE).floor() as i32,
                );
                let (near, far, _) = &variants[v as usize];
                let tile = cloud_tiles.entry(key).or_default();
                tile.0.place(near, m);
                if let Some(far) = far {
                    tile.1.place(far, m);
                }
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
                });
                let (near, far, spire) = &variants[v as usize];
                let alpha = (day * 255.0) as u8;
                // Every tree its own green, and now and then one turning for autumn.
                let leaves = leaf_tint(p.kind, p.id);
                if let Some(spire) = spire {
                    let g = group_far.entry(key).or_default();
                    let from = g.verts.len();
                    g.place(spire, m);
                    g.verts[from..].iter_mut().for_each(|v| v.color[3] = alpha);
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
        let mut cloud_keys: Vec<_> = cloud_tiles.keys().copied().collect();
        cloud_keys.sort();
        for key in cloud_keys {
            let (near, far) = &cloud_tiles[&key];
            let (center, radius) = bounds(near);
            let id = renderer.add_mesh(near);
            let far = (!far.verts.is_empty()).then(|| renderer.add_mesh(far));
            renderer.add_static_culled(
                id,
                &[Instance::tinted(
                    Mat4::IDENTITY,
                    Vec4::new(1.5, 1.5, 1.5, 1.0),
                )],
                Cull {
                    center,
                    radius,
                    reach: 2.2,
                    far,
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
                _ if cave => (CLOUD_TILE, instances[0].model[3][1], 0.35, 1.0),
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
        let groups: HashMap<(i32, i32), Group> = merged
            .iter()
            .filter(|(_, m)| !m.verts.is_empty())
            .map(|(key, m)| {
                let (center, radius) = bounds(m);
                let id = renderer.add_mesh(m);
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
                        far: None,
                        detail: 1.0,
                        cave: false,
                        group: Some(group),
                    },
                );
                (*key, group)
            })
            .collect();
        tiles.append(&mut cave_tiles);
        for (key, tile) in tiles {
            if tile.near.verts.is_empty() {
                continue;
            }
            let (center, radius) = bounds(&tile.near);
            let near = renderer.add_mesh(&tile.near);
            let far = (!tile.far.verts.is_empty()).then(|| renderer.add_mesh(&tile.far));
            let group = if tile.cave {
                None
            } else {
                groups.get(&group_of(key)).map(|g| Group {
                    merged: false,
                    ..*g
                })
            };
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
        let flame = renderer.add_mesh(&models::flame());
        let pool = renderer.add_mesh(&models::pool());
        let tuft = renderer.add_mesh(&models::tuft());
        let life = crate::life::Life::new(&mut renderer, world);
        let cave_life = crate::cave_life::CaveLife::new(&mut renderer, world);
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
            .collect();
        // Ordinary cars share one white mesh, painted per car by the instance tint, so
        // they draw in one go.
        let shared = |v: &zawnabout_sim::traffic::Vehicle| {
            if v.kind == zawnabout_sim::town::Kind::Car && v.variant < CAR_KINDS {
                CAR_WHITE
            } else {
                v.variant
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
            terrain: TerrainMeshes::default(),
            water,
            spray: renderer_spray,
            flame,
            ember,
            shafts,
            glowers,
            vehicles,
            lamps,
            pool,
            tuft,
            life,
            cave_life,
            sea_life,
            wildlife,
            plugs,
            mouths,
            plug,
            tufts: std::cell::RefCell::new(((i32::MAX, 0), Vec::new())),
            falls: crate::water::falls_feet(&game.sim.world.terrain),
            chunk_budget: 3,
        };
        gfx.update_terrain(game, usize::MAX);
        gfx
    }

    /// Streams terrain chunks around the camera, building at most `budget` new ones.
    pub fn update_terrain(&mut self, game: &Game, budget: usize) {
        self.terrain.update(
            &mut self.renderer,
            &game.sim.world.terrain,
            game.camera.pos,
            budget,
        );
    }

    fn mesh(&self, part: Part) -> MeshId {
        let d = &self.dragon;
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
            night,
        };

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

        let t = game.time as f32;
        let peek = self
            .mouths
            .iter()
            .any(|m| m.distance(game.camera.pos) < 450.0);
        let outside = lighting.underground < 0.5 || peek;
        self.vehicles(game, &mut draws);
        if outside {
            self.grass(game, &mut draws);
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
        let (mut fx, lights) = self.effects(game, caves, t);
        if outside {
            self.life
                .draw(world, game.camera.pos, t, night, &mut draws, &mut fx);
            self.sea_life.draw(game.camera.pos, t, &mut draws);
            self.wildlife.draw(
                world,
                game.camera.pos,
                t,
                night,
                self.pool,
                &mut draws,
                &mut fx,
            );
        }
        if caves {
            self.cave_life.draw(
                world,
                game.camera.pos,
                t,
                lighting.cave_ambient,
                self.pool,
                &mut draws,
                &mut fx,
            );
        }
        if !caves {
            for m in &self.plugs {
                if m.w_axis.truncate().distance(game.camera.pos) < 4000.0 {
                    draws.push((self.plug, Instance::new(*m)));
                }
            }
        }
        let deep = lighting.underground > 0.85 && !peek;
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
            deep,
            caves,
            shadows,
            rings,
            fx,
            lights,
        }
    }

    /// Tufts of grass on every lawn near the camera, swaying. They sit on a fixed world
    /// grid, so they stay put as the camera moves, and shrink away at the edge.
    fn grass(&self, game: &Game, draws: &mut Vec<(MeshId, Instance)>) {
        const STEP: f32 = 2.4;
        const RANGE: f32 = 46.0;
        let terrain = &game.sim.world.terrain;
        let cam = game.camera.pos;
        if cam.y - terrain.height(cam.x, cam.z) > 70.0 {
            return;
        }
        let key = ((cam.x / 6.0).floor() as i32, (cam.z / 6.0).floor() as i32);
        let mut cache = self.tufts.borrow_mut();
        if cache.0 != key {
            cache.0 = key;
            cache.1.clear();
            let center = Vec2::new(key.0 as f32 + 0.5, key.1 as f32 + 0.5) * 6.0;
            let n = (RANGE / STEP).ceil() as i32;
            let (ci, cj) = ((center.x / STEP) as i32, (center.y / STEP) as i32);
            for j in cj - n..=cj + n {
                for i in ci - n..=ci + n {
                    let seed =
                        (i as u32).wrapping_mul(73_856_093) ^ (j as u32).wrapping_mul(19_349_663);
                    let r = |k: u32| zawnabout_sim::shape::hash(seed.wrapping_add(k));
                    let x = (i as f32 + r(1)) * STEP;
                    let z = (j as f32 + r(2)) * STEP;
                    let d = Vec2::new(x, z).distance(center);
                    if d > RANGE {
                        continue;
                    }
                    let y = terrain.height(x, z);
                    let at = Vec3::new(x, y, z);
                    if terrain.water(x, z, y).is_some() || terrain.road_spot(x, z, 1.5).is_some() {
                        continue;
                    }
                    let Some(color) = crate::terrain_mesh::lawn(terrain, at, terrain.normal(x, z))
                    else {
                        continue;
                    };
                    // The drawn ground rounds off cliff edges, so keep back from them.
                    if terrain.rim(x, z, 5.0) > 1.2 {
                        continue;
                    }
                    let fade = 1.0 - smooth(RANGE * 0.65, RANGE, d);
                    let size = (0.8 + r(3) * 0.7) * fade;
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(size, size * (0.8 + r(4) * 0.5), size),
                        Quat::from_rotation_y(r(5) * std::f32::consts::TAU),
                        at - Vec3::Y * 0.05,
                    );
                    let tint = linear(color, 1.0) * (0.85 + r(6) * 0.3);
                    cache.1.push(Instance::tinted(m, tint.extend(1.0)));
                }
            }
        }
        draws.extend(cache.1.iter().map(|i| (self.tuft, *i)));
    }

    /// Ferries, boats, planes and cars where they are this frame, between ticks.
    fn vehicles(&self, game: &Game, draws: &mut Vec<(MeshId, Instance)>) {
        let traffic = &game.sim.world.traffic;
        let terrain = &game.sim.world.terrain;
        let time = (game.sim.tick as f32 + game.alpha()) * zawnabout_sim::DT;
        let cam = game.camera.pos;
        for (v, mesh) in traffic.vehicles.iter().zip(&self.vehicles) {
            let p = traffic.pose(v, time);
            let range = if matches!(
                v.kind,
                zawnabout_sim::town::Kind::Car
                    | zawnabout_sim::town::Kind::Train
                    | zawnabout_sim::town::Kind::LiftChair
                    | zawnabout_sim::town::Kind::LiftCabin
            ) {
                1500.0
            } else {
                6000.0
            };
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
            let paint = if v.kind == zawnabout_sim::town::Kind::Car && v.variant < CAR_KINDS {
                CAR_PAINT[v.variant as usize % CAR_PAINT.len()]
            } else {
                Vec3::ONE
            };
            draws.push((*mesh, Instance::tinted(m, paint.extend(day))));
        }
    }

    /// Fire, sparks and light shafts, and the point lights that go with them.
    fn effects(&self, game: &Game, caves: bool, t: f32) -> (Vec<(MeshId, Instance)>, Vec<Light>) {
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
        // Street lamps at night, and cave lamps always: a warm pool on the ground and a
        // soft halo, and the nearest ones light what is around them.
        let night = game.sky().night;
        let warm = linear(Rgb(0xFFC874), 1.0);
        for (lamp, foot, under) in &self.lamps {
            let on = if *under { 1.0 } else { night };
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
            let (color, alpha, size) = if p.spark {
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
        self.update_terrain(game, self.chunk_budget);
        let frame = self.frame(game);
        self.renderer.render(target, &frame);
    }

    /// Renders one frame offscreen and reads it back as tightly packed RGBA8. Blocks,
    /// so native only.
    pub fn capture(&mut self, game: &Game) -> Vec<u8> {
        // Shots always show finished terrain.
        while !self
            .terrain
            .settled(&game.sim.world.terrain, game.camera.pos)
        {
            self.update_terrain(game, usize::MAX);
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
    let base = linear(Rgb(0xB8B4D8), 0.95);
    let reach = 250.0;
    let list = terrain.voids_in(p - Vec3::splat(reach), p + Vec3::splat(reach));
    let (mut sum, mut weight) = (Vec3::ZERO, 0.0);
    for i in list {
        let v = &terrain.def.voids[i as usize];
        if matches!(v.shape, VoidShape::Tube | VoidShape::Basin) {
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
