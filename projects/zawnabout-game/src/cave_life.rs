//! Life under the ground that is only for looks: bat colonies wheeling under the roofs of
//! big caverns and glowing motes drifting in the air around you. Like everything in
//! `life`, a pure function of the clock.

use std::f32::consts::TAU;

use glam::{Mat4, Quat, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::shape::hash;
use zawnabout_sim::terrain::VoidShape;

use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// Bats circling a spot under a cavern roof.
struct Colony {
    center: Vec3,
    radius: f32,
    count: u32,
}

pub struct CaveLife {
    colonies: Vec<Colony>,
    bat: MeshId,
}

/// Mote grid spacing and how far around the camera they show (m).
const MOTE_CELL: f32 = 11.0;
const MOTE_RANGE: f32 = 45.0;

impl CaveLife {
    pub fn new(renderer: &mut Renderer, world: &World) -> Self {
        let terrain = &world.terrain;
        let mut colonies = Vec::new();
        for (k, v) in terrain.def.voids.iter().enumerate() {
            if matches!(v.shape, VoidShape::Tube | VoidShape::Basin)
                || v.half[0].min(v.half[2]) < 120.0
            {
                continue;
            }
            let c = Vec3::from(v.center);
            let n = 1 + (v.half[0] * v.half[2] / 250_000.0) as u32;
            for i in 0..n.min(4) {
                let s = k as u32 * 101 + i;
                let (u, w) = (
                    (hash(s) - 0.5) * v.half[0] * 1.2,
                    (hash(s + 1) - 0.5) * v.half[2] * 1.2,
                );
                let x = c.x + u * v.cos - w * v.sin;
                let z = c.z + u * v.sin + w * v.cos;
                // Up under the roof, but still in open air.
                let mut y = c.y + v.half[1] * 0.5;
                while y > c.y && terrain.sdf(Vec3::new(x, y + 25.0, z)) < 0.0 {
                    y -= 10.0;
                }
                if terrain.sdf(Vec3::new(x, y, z)) < 20.0 {
                    continue;
                }
                colonies.push(Colony {
                    center: Vec3::new(x, y, z),
                    radius: 35.0 + hash(s + 2) * 60.0,
                    count: 14 + (hash(s + 3) * 16.0) as u32,
                });
            }
        }
        Self {
            colonies,
            bat: renderer.add_mesh(&models::bat()),
        }
    }

    /// Everything alive in the caves near the camera this moment. `tint` is the cave's own
    /// glow color, for the motes.
    pub fn draw(
        &self,
        world: &World,
        cam: Vec3,
        time: f32,
        tint: Vec3,
        pool: MeshId,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        for (k, c) in self.colonies.iter().enumerate() {
            if c.center.distance(cam) > 1600.0 {
                continue;
            }
            for b in 0..c.count {
                let s = k as u32 * 131 + b;
                // Bats dart: tight loops at speed, each on its own wobbling circle.
                let r = c.radius * (0.4 + hash(s) * 0.8);
                let speed = 14.0 + hash(s + 1) * 8.0;
                let dir = if hash(s + 2) < 0.5 { 1.0 } else { -1.0 };
                let a = time * speed / r * dir + hash(s + 3) * TAU;
                let wob = (time * 1.7 + s as f32).sin() * 8.0;
                let pos = c.center
                    + Vec3::new(
                        a.cos() * (r + wob),
                        (time * 0.9 + hash(s + 4) * 9.0).sin() * 10.0,
                        a.sin() * (r + wob),
                    );
                let heading = (-a.sin() * dir).atan2(a.cos() * dir);
                let flap = 0.25 + 0.75 * (time * 16.0 + s as f32 * 1.3).sin();
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, flap, 1.0) * 0.9,
                    Quat::from_rotation_y(heading) * Quat::from_rotation_z(0.45 * dir),
                    pos,
                );
                draws.push((self.bat, Instance::new(m)));
            }
        }
        // Motes sit on a fixed world grid, so they stay put as you move, and drift slowly
        // round their spots. Only where the air is open.
        let n = (MOTE_RANGE / MOTE_CELL).ceil() as i32;
        let base = (cam / MOTE_CELL).floor();
        let glow = (tint.normalize_or(Vec3::ONE) * 1.4 + Vec3::splat(0.5)).min(Vec3::ONE);
        for j in -n..=n {
            for k in -n..=n {
                for i in -n..=n {
                    let cell = base + Vec3::new(i as f32, j as f32, k as f32);
                    let seed = (cell.x as i32 as u32).wrapping_mul(73_856_093)
                        ^ (cell.y as i32 as u32).wrapping_mul(19_349_663)
                        ^ (cell.z as i32 as u32).wrapping_mul(83_492_791);
                    let r = |q: u32| hash(seed.wrapping_add(q));
                    if r(0) < 0.45 {
                        continue;
                    }
                    let home = (cell + Vec3::new(r(1), r(2), r(3))) * MOTE_CELL;
                    let drift = Vec3::new(
                        (time * 0.21 + r(4) * 9.0).sin(),
                        (time * 0.17 + r(5) * 9.0).sin() * 0.6,
                        (time * 0.19 + r(6) * 9.0).cos(),
                    ) * 2.5;
                    let at = home + drift;
                    let d = at.distance(cam);
                    if d > MOTE_RANGE || d < 1.5 || world.terrain.sdf(at) < 0.5 {
                        continue;
                    }
                    let fade = (1.0 - d / MOTE_RANGE).min(1.0)
                        * (0.55 + 0.45 * (time * 2.3 + r(7) * 20.0).sin());
                    let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(0.22, 1.0, 0.22) * (0.7 + r(8) * 0.6),
                        face,
                        at,
                    );
                    fx.push((
                        pool,
                        Instance::tinted(m, Vec4::new(glow.x, glow.y, glow.z, fade)),
                    ));
                }
            }
        }
    }
}
