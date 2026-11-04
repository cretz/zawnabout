//! Life on and in the water that is only for looks: fish leaping all along the coasts,
//! whales surfacing to blow and now and then breaching far out, sea turtles coming up for
//! air, seals basking on the shore rocks, bobbing buoys, duck families paddling on the
//! lakes and lily pads in their shallows. A pure function of the clock, like `life`.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::shape::hash;

use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// A spot on the water and its surface height.
#[derive(Clone, Copy)]
struct Spot {
    at: Vec2,
    level: f32,
}

pub struct SeaLife {
    fish: Vec<Spot>,
    whales: Vec<Spot>,
    turtles: Vec<Spot>,
    buoys: Vec<Spot>,
    /// Seals lying on the rocks: where, facing which way.
    seals: Vec<(Vec3, f32)>,
    ducks: Vec<Spot>,
    lilies: Vec<Spot>,
    fish_mesh: MeshId,
    whale: MeshId,
    turtle: MeshId,
    seal: MeshId,
    duck: MeshId,
    lily: [MeshId; 2],
    buoy: [MeshId; 3],
    splash: MeshId,
}

fn far_from(list: &[Spot], p: Vec2, d: f32) -> bool {
    list.iter().all(|s| s.at.distance(p) > d)
}

impl SeaLife {
    pub fn new(renderer: &mut Renderer, world: &World) -> Self {
        let terrain = &world.terrain;
        let sea = terrain.def.sea_level;
        let step = 80.0;
        let n = (world.half_size / step) as i32;
        let mut s = Self {
            fish: Vec::new(),
            whales: Vec::new(),
            turtles: Vec::new(),
            buoys: Vec::new(),
            seals: Vec::new(),
            ducks: Vec::new(),
            lilies: Vec::new(),
            fish_mesh: renderer.add_mesh(&models::fish()),
            whale: renderer.add_mesh(&models::whale()),
            turtle: renderer.add_mesh(&models::turtle()),
            seal: renderer.add_mesh(&models::seal()),
            duck: renderer.add_mesh(&models::duck()),
            lily: [0, 1].map(|v| renderer.add_mesh(&models::lily(v))),
            buoy: [0, 1, 2].map(|v| renderer.add_mesh(&models::buoy(v))),
            splash: renderer.add_mesh(&models::spray()),
        };
        for j in -n..n {
            for i in -n..n {
                let seed = ((i + 5000) as u32) * 7907 + (j + 5000) as u32;
                let r = |k: u32| hash(seed.wrapping_mul(3) + k);
                let at = Vec2::new((i as f32 + r(1)) * step, (j as f32 + r(2)) * step);
                let ground = terrain.height(at.x, at.y);
                let roll = r(3);
                let Some(level) = terrain.water(at.x, at.y, ground) else {
                    // Seals haul out on low rocks right by the sea.
                    let by_sea = [(40.0, 0.0), (-40.0, 0.0), (0.0, 40.0), (0.0, -40.0)]
                        .iter()
                        .find(|(dx, dz)| terrain.height(at.x + dx, at.y + dz) < sea - 2.0);
                    if let Some((dx, dz)) = by_sea
                        && (sea + 0.3..sea + 3.0).contains(&ground)
                        && roll < 0.05
                        && s.seals
                            .iter()
                            .all(|(p, _)| Vec2::new(p.x, p.z).distance(at) > 500.0)
                    {
                        for k in 0..3 + (r(4) * 4.0) as u32 {
                            let p = at + Vec2::new(r(10 + k) - 0.5, r(20 + k) - 0.5) * 14.0;
                            let y = terrain.height(p.x, p.y);
                            if y > sea + 0.1 {
                                let face = dx.atan2(*dz) + (r(30 + k) - 0.5) * 2.0;
                                s.seals.push((Vec3::new(p.x, y, p.y), face));
                            }
                        }
                    }
                    continue;
                };
                let depth = level - ground;
                let spot = Spot { at, level };
                if level > sea + 1.0 {
                    // Lakes and rivers up in the hills.
                    if roll < 0.25 && far_from(&s.ducks, at, 150.0) {
                        s.ducks.push(spot);
                    } else if depth < 3.0 && roll < 0.6 {
                        s.lilies.push(spot);
                    }
                } else if depth > 60.0 {
                    if roll < 0.006 && far_from(&s.whales, at, 1500.0) {
                        s.whales.push(spot);
                    }
                } else if depth > 2.0 && depth < 35.0 {
                    if roll < 0.25 {
                        s.fish.push(spot);
                    } else if roll < 0.27 {
                        s.turtles.push(spot);
                    } else if roll < 0.28 && depth > 4.0 && far_from(&s.buoys, at, 300.0) {
                        s.buoys.push(spot);
                    }
                }
            }
        }
        s
    }

    /// Everything on the water near the camera this moment.
    pub fn draw(&self, cam: Vec3, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let near = |p: Vec2, r: f32| p.distance(Vec2::new(cam.x, cam.z)) < r;
        let splash = |draws: &mut Vec<(MeshId, Instance)>, at: Vec3, size: f32| {
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(size, size * 0.6, size),
                Quat::IDENTITY,
                at,
            );
            draws.push((
                self.splash,
                Instance::tinted(m, Vec4::new(1.25, 1.3, 1.35, 1.0)),
            ));
        };
        // Fish: a few at each spot, each leaping now and then in a short silver arc.
        for (k, f) in self.fish.iter().enumerate() {
            if !near(f.at, 450.0) {
                continue;
            }
            for d in 0..3u32 {
                let s = k as u32 * 37 + d;
                let period = 3.0 + hash(s) * 5.0;
                let cycle = time / period + hash(s + 1);
                let phase = cycle.fract();
                let round = cycle.floor() as u32;
                if phase > 0.2 {
                    continue;
                }
                let u = phase / 0.2;
                let a = hash(s * 5 + round) * TAU;
                let dir = Vec2::new(a.cos(), a.sin());
                let start =
                    f.at + Vec2::new(hash(s + round * 3) - 0.5, hash(s + round * 7) - 0.5) * 40.0;
                let p = start + dir * u * 2.4;
                let y = f.level + (u * PI).sin() * 1.1 - 0.1;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(1.0 + hash(s + 2) * 0.5),
                    Quat::from_rotation_y(dir.x.atan2(dir.y))
                        * Quat::from_rotation_x(-(u * PI).cos() * 1.1),
                    Vec3::new(p.x, y, p.y),
                );
                draws.push((self.fish_mesh, Instance::new(m)));
                if !(0.08..=0.92).contains(&u) {
                    splash(draws, Vec3::new(p.x, f.level, p.y), 0.35);
                }
            }
        }
        // Whales: a long roll at the surface with a blow or two, a dive showing the tail,
        // and every few rounds a breach right out of the water.
        for (k, w) in self.whales.iter().enumerate() {
            if !near(w.at, 3500.0) {
                continue;
            }
            let s = k as u32 * 71;
            let period = 46.0;
            let cycle = time / period + hash(s);
            let phase = cycle.fract();
            let round = cycle.floor() as u32;
            let a = hash(s + round) * TAU;
            let dir = Vec2::new(a.cos(), a.sin());
            let start =
                w.at + Vec2::new(hash(s + round * 3) - 0.5, hash(s + round * 5) - 0.5) * 300.0;
            let heading = dir.x.atan2(dir.y);
            let breach = round % 3 == 1;
            if phase < 0.35 {
                let u = phase / 0.35;
                let p = start + dir * u * 60.0;
                let rise = (u * PI).sin().min(0.25) * 4.0;
                let y = w.level - 1.6 + rise;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(heading)
                        * Quat::from_rotation_x(((u * 9.0).sin()) * 0.04),
                    Vec3::new(p.x, y, p.y),
                );
                draws.push((self.whale, Instance::new(m)));
                // The blow: a column of mist rising from the blowhole and drifting off.
                for b in [0.15f32, 0.55] {
                    let t = (u - b) / 0.12;
                    if (0.0..1.0).contains(&t) {
                        let head = p + dir * 4.5;
                        for i in 0..4 {
                            let h = t * 9.0 * (0.5 + i as f32 * 0.2);
                            let size = (1.0 + t * 2.5) * (1.0 - i as f32 * 0.15) * (1.0 - t * 0.6);
                            splash(draws, Vec3::new(head.x, w.level + 1.0 + h, head.y), size);
                        }
                    }
                }
            } else if phase < 0.43 {
                // Down she goes, flukes up last.
                let u = (phase - 0.35) / 0.08;
                let p = start + dir * (60.0 + u * 14.0);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(heading) * Quat::from_rotation_x(0.2 + u * 0.9),
                    Vec3::new(p.x, w.level - 1.5 - u * 9.0, p.y),
                );
                draws.push((self.whale, Instance::new(m)));
            } else if breach && (0.6..0.68).contains(&phase) {
                let u = (phase - 0.6) / 0.08;
                let p = start + dir * (90.0 + u * 20.0);
                let y = w.level - 9.0 + (u * PI).sin() * 17.0;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(heading)
                        * Quat::from_rotation_x(-1.2 + u * 2.2)
                        * Quat::from_rotation_z(u * 1.4),
                    Vec3::new(p.x, y, p.y),
                );
                draws.push((self.whale, Instance::new(m)));
                if u > 0.75 {
                    let t = (u - 0.75) / 0.25;
                    for i in 0..6 {
                        let a = i as f32 / 6.0 * TAU;
                        let at = p + dir * 4.0 + Vec2::new(a.cos(), a.sin()) * 5.0 * t;
                        splash(
                            draws,
                            Vec3::new(at.x, w.level + t * 5.0, at.y),
                            2.5 + t * 2.0,
                        );
                    }
                }
            }
        }
        // Turtles come up for a breath and paddle a little circle before sinking again.
        for (k, t) in self.turtles.iter().enumerate() {
            if !near(t.at, 400.0) {
                continue;
            }
            let s = k as u32 * 19;
            let phase = (time / 25.0 + hash(s)).fract();
            if phase > 0.5 {
                continue;
            }
            let a = time * 0.15 + hash(s + 1) * TAU;
            let p = t.at + Vec2::new(a.cos(), a.sin()) * 8.0;
            let up = (phase / 0.5 * PI).sin().min(0.3) / 0.3;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(1.0 + hash(s + 2) * 0.4),
                Quat::from_rotation_y(a + PI) * Quat::from_rotation_z((time * 1.3).sin() * 0.06),
                Vec3::new(p.x, t.level - 0.6 + up * 0.5, p.y),
            );
            draws.push((self.turtle, Instance::new(m)));
        }
        // Seals doze on the rocks, now and then lifting a head.
        for (k, (at, face)) in self.seals.iter().enumerate() {
            if !near(Vec2::new(at.x, at.z), 500.0) {
                continue;
            }
            let lift = ((time * 0.4 + k as f32 * 1.7).sin() * 0.5 + 0.5).powi(6) * 0.4;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(1.6 + hash(k as u32) * 0.5),
                Quat::from_rotation_y(*face) * Quat::from_rotation_x(-lift),
                *at,
            );
            draws.push((self.seal, Instance::new(m)));
        }
        // Buoys bob and lean in the swell, in lines of three like a string of pots.
        for (k, b) in self.buoys.iter().enumerate() {
            if !near(b.at, 1500.0) {
                continue;
            }
            for i in 0..3u32 {
                let s = k as u32 * 13 + i;
                let p = b.at + Vec2::new(i as f32 * 18.0, hash(s) * 10.0);
                let bob = (time * 1.1 + s as f32).sin() * 0.2;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_x((time * 0.9 + s as f32).sin() * 0.12)
                        * Quat::from_rotation_z((time * 0.7 + s as f32 * 2.0).cos() * 0.12),
                    Vec3::new(p.x, b.level + bob, p.y),
                );
                draws.push((self.buoy[(k + i as usize) % 3], Instance::new(m)));
            }
        }
        // Ducks: a mother and her ducklings in a line, wandering round the lake.
        for (k, d) in self.ducks.iter().enumerate() {
            if !near(d.at, 400.0) {
                continue;
            }
            let s = k as u32 * 29;
            let r = 10.0 + hash(s) * 12.0;
            let dir = if hash(s + 1) < 0.5 { 1.0 } else { -1.0 };
            let count = 3 + (hash(s + 2) * 4.0) as u32;
            for i in 0..=count {
                let a = time * 0.9 / r * dir + hash(s + 3) * TAU - i as f32 * 0.9 / r * dir * 1.2;
                let p = d.at + Vec2::new(a.cos(), a.sin()) * r;
                let heading = (-a.sin() * dir).atan2(a.cos() * dir);
                let size = if i == 0 { 1.3 } else { 0.65 };
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(size),
                    Quat::from_rotation_y(heading)
                        * Quat::from_rotation_z((time * 2.0 + i as f32).sin() * 0.06),
                    Vec3::new(p.x, d.level, p.y),
                );
                draws.push((self.duck, Instance::new(m)));
            }
        }
        // Lily pads in the shallows, gently rocking.
        for (k, l) in self.lilies.iter().enumerate() {
            if !near(l.at, 300.0) {
                continue;
            }
            for i in 0..8u32 {
                let s = k as u32 * 41 + i;
                let p = l.at + Vec2::new(hash(s) - 0.5, hash(s + 1) - 0.5) * 22.0;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.5 + hash(s + 2) * 0.6),
                    Quat::from_rotation_y(hash(s + 3) * TAU)
                        * Quat::from_rotation_x((time * 0.8 + s as f32).sin() * 0.03),
                    Vec3::new(p.x, l.level, p.y),
                );
                let v = (hash(s + 4) < 0.18) as usize;
                draws.push((self.lily[v], Instance::new(m)));
            }
        }
    }
}
