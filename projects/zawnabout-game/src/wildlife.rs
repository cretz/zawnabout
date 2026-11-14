//! Small life that makes the land feel lived in: deer grazing at forest edges, eagles
//! circling the big peaks, songbirds flitting about the trees, butterflies over the
//! meadows by day and fireflies by night, and rowboats and kayaks out on the water. All
//! for looks, and a pure function of the clock.

use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::shape::hash;
use zawnabout_sim::terrain::{CoverKind, Density};

use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// A lake with room to row on: its middle, how far out boats go, and its level.
struct Pond {
    center: Vec2,
    reach: f32,
}

/// A river kayaks run down: its points and the distance along at each.
struct Run {
    pts: Vec<Vec2>,
    along: Vec<f32>,
}

pub struct Wildlife {
    deer: Vec<Vec2>,
    eagles: Vec<(Vec3, f32)>,
    ponds: Vec<Pond>,
    runs: Vec<Run>,
    deer_mesh: MeshId,
    eagle: MeshId,
    songbird: MeshId,
    butterfly: MeshId,
    rowboat: MeshId,
    kayak: MeshId,
    /// What each grid cell of songbirds or butterflies found out about its ground, kept
    /// while the camera stays about. Cells stay put, so each only has to be found once.
    spots: RefCell<HashMap<(u8, i32, i32), Spot>>,
}

/// The ground height and slope where small life lives, or none for no life there.
type Spot = Option<(f32, Vec3)>;

/// Cells of small life kept in [`Wildlife::spots`] before they are all found afresh.
const SPOTS_KEPT: usize = 4096;

/// Butterfly wing colors.
const WINGS: [Vec3; 5] = [
    Vec3::new(1.0, 0.75, 0.2),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(0.45, 0.6, 1.0),
    Vec3::new(1.0, 0.5, 0.6),
    Vec3::new(1.0, 0.95, 0.4),
];

impl Wildlife {
    pub fn new(renderer: &mut Renderer, world: &World) -> Self {
        let terrain = &world.terrain;
        let def = terrain.def;
        // Deer where a wood meets open ground, spread well apart.
        let mut deer: Vec<Vec2> = Vec::new();
        let step = 260.0;
        let n = (world.half_size / step) as i32;
        for j in -n..n {
            for i in -n..n {
                let s = (j * 977 + i) as u32;
                let p = Vec2::new((i as f32 + hash(s)) * step, (j as f32 + hash(s + 1)) * step);
                let y = terrain.height(p.x, p.y);
                let wooded = terrain
                    .cover(p.x, p.y)
                    .is_some_and(|c| c.kind == CoverKind::Forest && c.density != Density::Rain);
                if wooded
                    && y > 5.0
                    && terrain.normal(p.x, p.y).y > 0.85
                    && terrain.water(p.x, p.y, y).is_none()
                    && hash(s + 2) < 0.5
                    && deer.iter().all(|d| d.distance(p) > 500.0)
                {
                    deer.push(p);
                }
            }
        }
        // An eagle or two over every big peak, riding the air in wide circles.
        let eagles = def
            .mounts
            .iter()
            .filter(|m| m.h > 300.0)
            .flat_map(|m| {
                let top = m.h.max(m.rim);
                (0..2).map(move |k| {
                    let r = m.r * (0.35 + 0.25 * k as f32);
                    (Vec3::new(m.x, top + 60.0 + k as f32 * 70.0, m.z), r)
                })
            })
            .collect();
        // Lakes open to the sky, big enough to row on.
        let ponds = def
            .lakes
            .iter()
            .filter_map(|l| {
                let c = l
                    .pts
                    .iter()
                    .fold(Vec2::ZERO, |a, p| a + Vec2::new(p[0], p[1]))
                    / l.pts.len().max(1) as f32;
                let reach = l
                    .pts
                    .iter()
                    .map(|p| Vec2::new(p[0], p[1]).distance(c))
                    .fold(f32::MAX, f32::min);
                let sky = terrain.daylight(Vec3::new(c.x, l.level + 2.0, c.y)) > 0.5;
                (sky && reach > 40.0).then_some(Pond {
                    center: c,
                    reach: reach * 0.55,
                })
            })
            .collect();
        let runs = def
            .rivers
            .iter()
            .filter(|r| !r.under && r.w >= 14.0)
            .map(|r| {
                let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
                let mut along = vec![0.0];
                for w in pts.windows(2) {
                    along.push(along[along.len() - 1] + w[0].distance(w[1]));
                }
                Run { pts, along }
            })
            .collect();
        Self {
            deer,
            eagles,
            ponds,
            runs,
            deer_mesh: renderer.add_mesh(&models::deer()),
            eagle: renderer.add_mesh(&models::eagle()),
            songbird: renderer.add_mesh(&models::songbird()),
            butterfly: renderer.add_mesh(&models::butterfly()),
            rowboat: renderer.add_mesh(&models::rowboat()),
            kayak: renderer.add_mesh(&models::kayak()),
            spots: RefCell::default(),
        }
    }

    /// Everything near the camera this moment. Fireflies go in `fx` as glows.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        world: &World,
        cam: Vec3,
        time: f32,
        night: f32,
        pool: MeshId,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let terrain = &world.terrain;
        let flat_cam = Vec2::new(cam.x, cam.z);
        let near = |p: Vec2, r: f32| p.distance(flat_cam) < r;

        // Deer graze in a loose group, heads down, and look up now and then.
        for (k, c) in self.deer.iter().enumerate() {
            if !near(*c, 600.0) {
                continue;
            }
            let drift = Vec2::new((time * 0.01 + k as f32).sin(), (time * 0.008).cos()) * 25.0;
            for i in 0..4 + (hash(k as u32) * 3.0) as u32 {
                let s = k as u32 * 53 + i;
                let off = Vec2::new(hash(s) - 0.5, hash(s + 1) - 0.5) * 22.0;
                let p = *c + drift + off;
                let y = terrain.height(p.x, p.y);
                let face = hash(s + 2) * TAU + (time * 0.05 + s as f32).sin() * 0.6;
                let look = ((time * 0.25 + hash(s + 3) * 20.0).sin() * 0.5 + 0.5).powi(3);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.9 + hash(s + 4) * 0.25),
                    Quat::from_rotation_y(face) * Quat::from_rotation_x(0.3 * (1.0 - look)),
                    Vec3::new(p.x, y, p.y),
                );
                draws.push((self.deer_mesh, Instance::new(m)));
            }
        }

        // Eagles circle slowly, banked, with a few lazy wingbeats.
        for (k, (c, r)) in self.eagles.iter().enumerate() {
            if c.distance(cam) > 2500.0 {
                continue;
            }
            let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
            let a = time * 11.0 / r * dir + k as f32 * 2.1;
            let pos = *c
                + Vec3::new(
                    a.cos() * r,
                    (time * 0.2 + k as f32).sin() * 15.0,
                    a.sin() * r,
                );
            let heading = (-a.sin() * dir).atan2(a.cos() * dir);
            let beat = ((time * 0.3 + k as f32 * 3.0).sin() * 0.5 + 0.5).powi(8);
            let flap = 1.0 - 0.5 * beat * (time * 5.0).sin();
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(1.0, flap, 1.0) * 2.2,
                Quat::from_rotation_y(heading) * Quat::from_rotation_z(0.3 * dir),
                pos,
            );
            draws.push((self.eagle, Instance::new(m)));
        }

        // Grid cells round the camera for the small things, fixed in the world so they
        // stay put as you move.
        let cells = |size: f32, range: f32, f: &mut dyn FnMut(Vec2, u32)| {
            let n = (range / size).ceil() as i32;
            let base = (flat_cam / size).floor();
            for j in -n..=n {
                for i in -n..=n {
                    let cell = base + Vec2::new(i as f32, j as f32);
                    let seed = (cell.x as i32 as u32).wrapping_mul(73_856_093)
                        ^ (cell.y as i32 as u32).wrapping_mul(19_349_663);
                    f(cell * size, seed);
                }
            }
        };
        let day = 1.0 - night;
        let mut spots = self.spots.borrow_mut();
        if spots.len() > SPOTS_KEPT {
            spots.clear();
        }
        let key = |tag: u8, at: Vec2, size: f32| {
            (
                tag,
                (at.x / size).round() as i32,
                (at.y / size).round() as i32,
            )
        };

        // Songbirds flit about in little flocks wherever trees grow.
        if day > 0.3 {
            cells(70.0, 280.0, &mut |at, seed| {
                if hash(seed) > 0.35 {
                    return;
                }
                let c = at + Vec2::new(hash(seed + 1), hash(seed + 2)) * 70.0;
                if !near(c, 280.0) {
                    return;
                }
                let spot = *spots.entry(key(0, at, 70.0)).or_insert_with(|| {
                    if terrain
                        .cover(c.x, c.y)
                        .is_none_or(|cv| cv.kind != CoverKind::Forest)
                    {
                        return None;
                    }
                    let ground = terrain.height(c.x, c.y);
                    terrain
                        .water(c.x, c.y, ground)
                        .is_none()
                        .then_some((ground, Vec3::Y))
                });
                let Some((ground, _)) = spot else {
                    return;
                };
                // The flock hops from tree to tree: it rests, then darts on together.
                let hop = (time * 0.15 + hash(seed + 3) * 9.0).floor();
                let t = (time * 0.15 + hash(seed + 3) * 9.0).fract();
                let spot = |h: f32| {
                    Vec2::new(
                        hash(seed + h as u32 * 7) - 0.5,
                        hash(seed + h as u32 * 13) - 0.5,
                    ) * 40.0
                };
                let dart = ((t - 0.75) / 0.25).clamp(0.0, 1.0);
                let center = c + spot(hop).lerp(spot(hop + 1.0), dart * dart * (3.0 - 2.0 * dart));
                for b in 0..6u32 {
                    let s = seed + b * 31;
                    let wob = Vec3::new(
                        (time * 3.0 + s as f32).sin(),
                        (time * 4.3 + s as f32).sin() * 0.5,
                        (time * 2.6 + s as f32).cos(),
                    ) * (1.5 + dart * 2.0);
                    let pos = Vec3::new(center.x, ground + 7.0 + hash(s) * 6.0, center.y) + wob;
                    let flap = 0.3 + 0.7 * (time * 24.0 + s as f32).sin().abs();
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(1.0, flap, 1.0) * 1.8,
                        Quat::from_rotation_y(hash(s + 1) * TAU + time * (0.5 + dart)),
                        pos,
                    );
                    draws.push((self.songbird, Instance::new(m)));
                }
            });
        }

        // Butterflies by day and fireflies by night, low over grass and meadows.
        cells(22.0, 80.0, &mut |at, seed| {
            if hash(seed) > 0.45 {
                return;
            }
            let c = at + Vec2::new(hash(seed + 1), hash(seed + 2)) * 22.0;
            if !near(c, 80.0) {
                return;
            }
            let spot = *spots.entry(key(1, at, 22.0)).or_insert_with(|| {
                let ground = terrain.height(c.x, c.y);
                let open = terrain
                    .cover(c.x, c.y)
                    .is_none_or(|cv| matches!(cv.kind, CoverKind::Meadow | CoverKind::Forest));
                let normal = terrain.normal(c.x, c.y);
                let lives = open
                    && ground >= 3.0
                    && terrain.road_spot(c.x, c.y, 8.0).is_none()
                    && normal.y >= 0.8
                    && terrain.water(c.x, c.y, ground).is_none()
                    && terrain.daylight(Vec3::new(c.x, ground + 2.0, c.y)) >= 0.5;
                lives.then_some((ground, normal))
            });
            let Some((ground, normal)) = spot else {
                return;
            };
            for b in 0..2u32 {
                let s = seed + b * 17;
                let wander = Vec2::new(
                    (time * 0.31 + hash(s) * 9.0).sin() + (time * 0.83 + s as f32).sin() * 0.3,
                    (time * 0.27 + hash(s + 1) * 9.0).cos() + (time * 0.71 + s as f32).cos() * 0.3,
                ) * 5.0;
                let p = c + wander;
                // Along the slope at the middle, near enough a few meters out.
                let y = ground - (normal.x * wander.x + normal.z * wander.y) / normal.y;
                if day > 0.5 {
                    let lift = 0.6 + (time * 1.7 + s as f32).sin().abs() * 1.2;
                    let flap = 0.15 + 0.85 * (time * 14.0 + s as f32).sin().abs();
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(flap, 1.0, 1.0) * 2.2,
                        Quat::from_rotation_y(time * 0.6 + hash(s + 2) * TAU),
                        Vec3::new(p.x, y + lift, p.y),
                    );
                    let wing = WINGS[(hash(s + 3) * 5.0) as usize % 5];
                    draws.push((self.butterfly, Instance::tinted(m, wing.extend(1.0))));
                } else if night > 0.5 {
                    let blink = ((time * 1.3 + hash(s + 4) * 20.0).sin() * 0.5 + 0.5).powi(3);
                    let at = Vec3::new(p.x, y + 0.8 + (time * 0.5 + s as f32).sin() * 0.6, p.y);
                    let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                    let m =
                        Mat4::from_scale_rotation_translation(Vec3::new(0.4, 1.0, 0.4), face, at);
                    fx.push((
                        pool,
                        Instance::tinted(m, Vec4::new(0.9, 1.0, 0.5, (0.15 + blink) * night)),
                    ));
                }
            }
        });

        // Rowboats out on the lakes, drifting round with a lazy stroke.
        for (k, pond) in self.ponds.iter().enumerate() {
            if !near(pond.center, 1500.0) {
                continue;
            }
            for b in 0..2u32 {
                let s = k as u32 * 29 + b;
                let r = pond.reach * (0.4 + hash(s) * 0.6);
                let a = time * 1.2 / r.max(1.0) + hash(s + 1) * TAU;
                let p = pond.center + Vec2::new(a.cos(), a.sin()) * r;
                let ground = terrain.height(p.x, p.y);
                let Some(level) = terrain.water(p.x, p.y, ground) else {
                    continue;
                };
                let stroke = (time * 1.6 + s as f32).sin();
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(a + PI) * Quat::from_rotation_z(stroke * 0.03),
                    Vec3::new(p.x, level + (time * 1.1 + s as f32).sin() * 0.05, p.y),
                );
                draws.push((self.rowboat, Instance::new(m)));
            }
        }

        // Kayaks run down the bigger rivers, paddles going.
        for (k, run) in self.runs.iter().enumerate() {
            let len = run.along[run.along.len() - 1];
            for b in 0..3u32 {
                let s = k as u32 * 37 + b;
                let d = (time * (2.0 + hash(s) * 1.5) + hash(s + 1) * len).rem_euclid(len);
                let i = run
                    .along
                    .partition_point(|a| *a < d)
                    .clamp(1, run.pts.len() - 1);
                let (a, c) = (run.pts[i - 1], run.pts[i]);
                let seg = (run.along[i] - run.along[i - 1]).max(0.01);
                let dir = (c - a) / seg;
                let side = Vec2::new(-dir.y, dir.x) * (hash(s + 2) - 0.5) * 6.0;
                let p = a.lerp(c, (d - run.along[i - 1]) / seg) + side;
                if !near(p, 900.0) {
                    continue;
                }
                let ground = terrain.height(p.x, p.y);
                let Some(level) = terrain.water(p.x, p.y, ground) else {
                    continue;
                };
                let paddle = (time * 3.0 + s as f32).sin();
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(dir.x.atan2(dir.y) + paddle * 0.05)
                        * Quat::from_rotation_z(paddle * 0.05),
                    Vec3::new(p.x, level, p.y),
                );
                draws.push((self.kayak, Instance::new(m)));
            }
        }
    }
}
