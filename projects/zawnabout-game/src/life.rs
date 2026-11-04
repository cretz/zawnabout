//! Life around the world that is only for looks: gulls wheeling over the coasts, sheep
//! grazing the meadows, townsfolk strolling between houses, dolphins leaping offshore and
//! lighthouse beams sweeping at night. Everything is a pure function of the clock, so it
//! costs nothing to keep and looks the same for everyone.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::shape::hash;
use zawnabout_sim::terrain::CoverKind;
use zawnabout_sim::town::Kind;

use crate::mesh::Rgb;
use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// A flock of gulls circling a spot on the coast.
struct Flock {
    center: Vec3,
    radius: f32,
    count: u32,
}

/// Sheep, cows or horses grazing around a slowly wandering spot.
struct Herd {
    center: Vec2,
    count: u32,
    /// 0 sheep, 1 cows, 2 horses.
    kind: u8,
}

/// Someone walking back and forth between two doors.
struct Walker {
    a: Vec2,
    b: Vec2,
    kind: u8,
}

/// A ski run beside a chairlift: from its top station down to its bottom one.
struct Run {
    top: Vec2,
    bottom: Vec2,
}

/// Dolphins leaping now and then near a spot at sea.
struct Pod {
    center: Vec2,
}

pub struct Life {
    flocks: Vec<Flock>,
    herds: Vec<Herd>,
    walkers: Vec<Walker>,
    pods: Vec<Pod>,
    beams: Vec<Vec3>,
    /// Turbine hubs and the way their rotors face.
    turbines: Vec<(Vec3, Vec3)>,
    fires: Vec<Vec3>,
    /// Fairground wheels, rope swings and hot springs: each its landmark's frame.
    wheels: Vec<Mat4>,
    swings: Vec<Mat4>,
    springs: Vec<Vec3>,
    runs: Vec<Run>,
    skis: MeshId,
    /// Chimney tops, and flagpole tops with the way they face.
    chimneys: Vec<Vec3>,
    flags: Vec<Vec3>,
    flag: MeshId,
    pole: MeshId,
    wheel: MeshId,
    gondola: MeshId,
    seat: MeshId,
    steam: MeshId,
    rotor: MeshId,
    flame: MeshId,
    gull: MeshId,
    sheep: MeshId,
    cow: MeshId,
    horse: MeshId,
    dog: MeshId,
    body: MeshId,
    heads: [MeshId; 3],
    dolphin: MeshId,
    beam: MeshId,
}

/// Flag colors.
const FLAGS: [Rgb; 5] = [
    Rgb(0xE0564A),
    Rgb(0x4A90C8),
    Rgb(0xF2C14E),
    Rgb(0x6BBF7A),
    Rgb(0xF6F1E7),
];

/// Shirt colors for townsfolk.
const SHIRTS: [Rgb; 8] = [
    Rgb(0xE0564A),
    Rgb(0x4A90C8),
    Rgb(0xF2C14E),
    Rgb(0x6BBF7A),
    Rgb(0xE88AB0),
    Rgb(0x9A7FD1),
    Rgb(0xF6F1E7),
    Rgb(0x3E7F8C),
];

fn linear(c: Rgb) -> Vec3 {
    let f = |s: u32| ((s & 0xFF) as f32 / 255.0).powf(2.2);
    Vec3::new(f(c.0 >> 16), f(c.0 >> 8), f(c.0))
}

impl Life {
    pub fn new(renderer: &mut Renderer, world: &World) -> Self {
        let terrain = &world.terrain;
        let h = world.half_size;
        // Coasts, open grass and sea from a coarse look over the whole world, spaced out.
        let (mut flocks, mut herds, mut pods) =
            (Vec::<Flock>::new(), Vec::<Herd>::new(), Vec::<Pod>::new());
        let step = 120.0;
        let n = (h / step) as i32;
        for j in -n..n {
            for i in -n..n {
                let seed = ((i + 1000) as u32) * 7919 + (j + 1000) as u32;
                let x = (i as f32 + hash(seed)) * step;
                let z = (j as f32 + hash(seed + 1)) * step;
                let p = Vec2::new(x, z);
                let y = terrain.height(x, z);
                let roll = hash(seed + 2);
                if (-6.0..10.0).contains(&y) && roll < 0.4 {
                    if flocks
                        .iter()
                        .all(|f| Vec2::new(f.center.x, f.center.z).distance(p) > 450.0)
                    {
                        flocks.push(Flock {
                            center: Vec3::new(x, 0.0, z),
                            radius: 30.0 + hash(seed + 3) * 50.0,
                            count: 5 + (hash(seed + 4) * 8.0) as u32,
                        });
                    }
                } else if (12.0..350.0).contains(&y) && roll < 0.2 {
                    let open = terrain.normal(x, z).y > 0.9
                        && terrain.water(x, z, y).is_none()
                        && terrain.road_spot(x, z, 20.0).is_none()
                        && terrain
                            .cover(x, z)
                            .is_none_or(|c| c.kind == CoverKind::Meadow);
                    if open && herds.iter().all(|f| f.center.distance(p) > 700.0) {
                        let kind = (hash(seed + 11) * 2.6) as u8;
                        herds.push(Herd {
                            center: p,
                            count: if kind == 0 {
                                6 + (hash(seed + 9) * 10.0) as u32
                            } else {
                                3 + (hash(seed + 9) * 5.0) as u32
                            },
                            kind,
                        });
                    }
                } else if y < -25.0
                    && y > -120.0
                    && roll < 0.05
                    && pods.iter().all(|f| f.center.distance(p) > 900.0)
                {
                    pods.push(Pod { center: p });
                }
            }
        }
        // Townsfolk: every few houses someone walks over to a neighbor's door and back.
        let homes: Vec<_> = world
            .buildings
            .iter()
            .filter(|b| matches!(b.kind, Kind::House | Kind::Row))
            .collect();
        let mut walkers = Vec::new();
        for (i, w) in homes.windows(2).enumerate() {
            if i % 4 != 0 {
                continue;
            }
            let door = |b: &zawnabout_sim::town::Building| {
                let depth = if b.kind == Kind::Row {
                    b.size.x
                } else {
                    b.size.z
                };
                Vec2::new(b.pos.x, b.pos.z) + b.front() * (depth * 0.5 + 1.5)
            };
            let (a, b) = (door(w[0]), door(w[1]));
            if a.distance(b) > 3.0 && a.distance(b) < 120.0 {
                walkers.push(Walker {
                    a,
                    b,
                    kind: (hash(i as u32 * 17) * 3.0) as u8,
                });
            }
        }
        let beams: Vec<Vec3> = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Lighthouse)
            .map(|b| b.pos + Vec3::Y * (b.size.y + zawnabout_sim::town::LAMP_ROOM * 0.5))
            .collect();
        let turbines = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Turbine)
            .map(|b| {
                let r = zawnabout_sim::landmark::ROTOR;
                (b.at(r.x, r.y, r.z), Vec3::new(b.axis.x, 0.0, b.axis.y))
            })
            .collect();
        let fires = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Campsite)
            .map(|b| {
                let c = zawnabout_sim::landmark::CAMPFIRE;
                b.at(c.x, c.y + 0.2, c.z)
            })
            .collect();
        let frame = |b: &zawnabout_sim::town::Building| {
            let (a, f) = (b.axis, b.front());
            Mat4::from_cols(
                Vec4::new(a.x, 0.0, a.y, 0.0),
                Vec4::Y,
                Vec4::new(f.x, 0.0, f.y, 0.0),
                b.pos.extend(1.0),
            )
        };
        let of = |k: Kind| world.buildings.iter().filter(move |b| b.kind == k);
        let wheels = of(Kind::Ferris).map(frame).collect();
        let swings = of(Kind::Swing).map(frame).collect();
        let springs = of(Kind::HotSpring)
            .map(|b| {
                let c = zawnabout_sim::landmark::SPRING;
                b.at(c.x, c.y, c.z)
            })
            .collect();
        let runs = world
            .terrain
            .def
            .routes
            .iter()
            .filter(|r| r.kind == "chairlift")
            .map(|r| {
                let (a, b) = (r.pts[0], r.pts[r.pts.len() - 1]);
                Run {
                    top: Vec2::new(b[0], b[1]),
                    bottom: Vec2::new(a[0], a[1]),
                }
            })
            .collect();
        let chimneys = world
            .buildings
            .iter()
            .filter_map(|b| crate::town_mesh::chimney(b).map(|c| b.at(c.x, c.y, c.z)))
            .collect();
        // Flags fly over halls, terminals and lift stations.
        let flags = world
            .buildings
            .iter()
            .filter(|b| matches!(b.kind, Kind::Hall | Kind::Terminal | Kind::LiftStation))
            .map(|b| {
                b.at(
                    b.size.x * 0.42,
                    b.size.y + b.roof * 0.4 + 5.0,
                    b.size.z * 0.3,
                )
            })
            .collect();
        Self {
            wheels,
            swings,
            springs,
            chimneys,
            flags,
            flag: renderer.add_mesh(&models::flag()),
            pole: renderer.add_mesh(&models::flagpole()),
            runs,
            skis: renderer.add_mesh(&models::skis()),
            wheel: renderer.add_mesh(&models::ferris_wheel()),
            gondola: renderer.add_mesh(&models::gondola()),
            seat: renderer.add_mesh(&models::swing_seat()),
            steam: renderer.add_mesh(&models::pool()),
            turbines,
            fires,
            rotor: renderer.add_mesh(&models::rotor()),
            flame: renderer.add_mesh(&models::flame()),
            flocks,
            herds,
            walkers,
            pods,
            beams,
            gull: renderer.add_mesh(&models::gull()),
            sheep: renderer.add_mesh(&models::sheep()),
            cow: renderer.add_mesh(&models::cow()),
            horse: renderer.add_mesh(&models::horse()),
            dog: renderer.add_mesh(&models::dog()),
            body: renderer.add_mesh(&models::townsfolk_body()),
            heads: [0, 1, 2].map(|v| renderer.add_mesh(&models::townsfolk_head(v))),
            dolphin: renderer.add_mesh(&models::dolphin()),
            beam: renderer.add_mesh(&models::lighthouse_beam()),
        }
    }

    /// Everything alive near the camera this moment.
    pub fn draw(
        &self,
        world: &World,
        cam: Vec3,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let terrain = &world.terrain;
        let near = |p: Vec2, r: f32| p.distance(Vec2::new(cam.x, cam.z)) < r;
        // Gulls roost at night.
        if night < 0.6 {
            for (k, f) in self.flocks.iter().enumerate() {
                if !near(Vec2::new(f.center.x, f.center.z), 1600.0) {
                    continue;
                }
                for b in 0..f.count {
                    let s = k as u32 * 97 + b;
                    let r = f.radius * (0.6 + hash(s) * 0.8);
                    let speed = 9.0 + hash(s + 1) * 4.0;
                    let dir = if hash(s + 2) < 0.5 { 1.0 } else { -1.0 };
                    let a = time * speed / r * dir + hash(s + 3) * TAU;
                    let y = 25.0 + hash(s + 4) * 40.0 + (time * 0.3 + s as f32).sin() * 6.0;
                    let pos = f.center + Vec3::new(a.cos() * r, y, a.sin() * r);
                    let heading = (-a.sin() * dir).atan2(a.cos() * dir);
                    // Flap in bursts, glide between.
                    let burst = ((time * 0.4 + hash(s + 5) * 9.0).sin() * 0.5 + 0.5).powi(3);
                    let flap = 0.35 + 0.65 * burst * (time * 9.0 + s as f32).sin();
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(1.0, flap, 1.0) * 1.4,
                        Quat::from_rotation_y(heading) * Quat::from_rotation_z(0.35 * dir),
                        pos,
                    );
                    draws.push((self.gull, Instance::new(m)));
                }
            }
        }
        for (k, herd) in self.herds.iter().enumerate() {
            if !near(herd.center, 700.0) {
                continue;
            }
            let ks = k as f32;
            let drift =
                Vec2::new((time * 0.008 + ks).sin(), (time * 0.006 + ks * 2.0).cos()) * 40.0;
            for i in 0..herd.count {
                let s = k as u32 * 211 + i;
                let off = Vec2::new(hash(s) - 0.5, hash(s + 1) - 0.5) * 34.0;
                let amble = Vec2::new(
                    (time * 0.05 + hash(s + 2) * 20.0).sin(),
                    (time * 0.04 + hash(s + 3) * 20.0).cos(),
                ) * 3.0;
                let p = herd.center + drift + off + amble;
                let y = terrain.height(p.x, p.y);
                if terrain.water(p.x, p.y, y).is_some() {
                    continue;
                }
                // Facing the way it ambles, head down to graze now and then.
                let vel = Vec2::new(
                    (time * 0.05 + hash(s + 2) * 20.0).cos(),
                    -(time * 0.04 + hash(s + 3) * 20.0).sin(),
                );
                let graze = ((time * 0.3 + hash(s + 4) * 30.0).sin() * 0.5 + 0.5) * 0.25;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.9 + hash(s + 5) * 0.3),
                    Quat::from_rotation_y(vel.x.atan2(vel.y)) * Quat::from_rotation_x(graze),
                    Vec3::new(p.x, y, p.y),
                );
                let mesh = [self.sheep, self.cow, self.horse][herd.kind as usize % 3];
                draws.push((mesh, Instance::new(m)));
            }
        }
        // Skiers carve down the runs beside every chairlift, in wide S turns, crouched and
        // leaning into each one. Only on the snow.
        for (k, run) in self.runs.iter().enumerate() {
            if !near(run.top.lerp(run.bottom, 0.5), 1300.0) {
                continue;
            }
            let fall = run.bottom - run.top;
            let len = fall.length().max(1.0);
            let down = fall / len;
            let across = Vec2::new(-down.y, down.x);
            for i in 0..22u32 {
                let s = k as u32 * 401 + i;
                let speed = 7.0 + hash(s) * 6.0;
                let u = (time * speed / (len * 1.15) + hash(s + 1)).fract();
                let lane = (hash(s + 2) - 0.5) * 220.0;
                let width = 6.0 + hash(s + 3) * 8.0;
                let wave = len / (18.0 + hash(s + 4) * 14.0);
                let a = u * wave * PI + hash(s + 5) * TAU;
                let p = run.top + fall * u + across * (lane + a.sin() * width);
                if !near(p, 700.0) {
                    continue;
                }
                let y = terrain.height(p.x, p.y);
                if terrain
                    .cover(p.x, p.y)
                    .is_none_or(|c| c.kind != CoverKind::Snow)
                {
                    continue;
                }
                let sway = a.cos() * width * wave * PI / len;
                let dir = down + across * sway;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, 0.85, 1.0),
                    Quat::from_rotation_y(dir.x.atan2(dir.y))
                        * Quat::from_rotation_z(-sway.clamp(-0.8, 0.8) * 0.4)
                        * Quat::from_rotation_x(0.2),
                    Vec3::new(p.x, y, p.y),
                );
                let jacket = linear(SHIRTS[(hash(s + 6) * 8.0) as usize % 8]);
                draws.push((self.body, Instance::tinted(m, jacket.extend(1.0))));
                draws.push((self.heads[(s % 3) as usize], Instance::new(m)));
                draws.push((self.skis, Instance::new(m)));
            }
        }
        // Wood smoke from chimneys: a few homes by day, most in the evening and at night.
        let busy = 0.25 + 0.5 * night;
        for (k, c) in self.chimneys.iter().enumerate() {
            let s = k as u32 * 7;
            if hash(s) > busy || c.distance(cam) > 450.0 {
                continue;
            }
            for i in 0..5u32 {
                let u = (time * 0.08 + hash(s + i * 3 + 1)).fract();
                let at = *c
                    + Vec3::new(
                        u * 5.0,
                        0.4 + u * 11.0,
                        u * 2.0 + (time * 0.6 + (s + i) as f32).sin() * u * 1.5,
                    );
                let size = 0.6 + u * 2.6;
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let m = Mat4::from_scale_rotation_translation(Vec3::new(size, 1.0, size), face, at);
                let fade = (u * 6.0).min(1.0) * (1.0 - u) * 0.5;
                fx.push((
                    self.steam,
                    Instance::tinted(m, Vec4::new(0.62, 0.62, 0.66, fade)),
                ));
            }
        }
        // Flags stream out with the breeze and ripple.
        for (k, top) in self.flags.iter().enumerate() {
            if top.distance(cam) > 900.0 {
                continue;
            }
            draws.push((self.pole, Instance::new(Mat4::from_translation(*top))));
            let gust = (time * 0.4 + k as f32).sin() * 0.25;
            for seg in [0.0f32, 1.0, 2.0] {
                let wave = (time * 4.0 - seg * 1.3 + k as f32).sin() * 0.3;
                let m = Mat4::from_translation(*top - Vec3::Y * 0.4)
                    * Mat4::from_rotation_y(0.6 + gust + wave * 0.3 * seg)
                    * Mat4::from_translation(Vec3::X * seg * 0.75)
                    * Mat4::from_rotation_y(wave);
                let c = FLAGS[k % FLAGS.len()];
                draws.push((self.flag, Instance::tinted(m, linear(c).extend(1.0))));
            }
        }
        // Fewer people about at night.
        for (i, w) in self.walkers.iter().enumerate() {
            let s = i as u32 * 31;
            if (night > 0.5 && hash(s + 1) < 0.7) || !near(w.a, 450.0) {
                continue;
            }
            let len = w.a.distance(w.b);
            // Out, a pause at the door, and back.
            let u = (time * 1.3 / len + hash(s) * 10.0) % 2.0;
            let (t, toward) = if u < 1.0 {
                (u, w.b - w.a)
            } else {
                (2.0 - u, w.a - w.b)
            };
            let ease = t.clamp(0.0, 1.0);
            let p = w.a.lerp(w.b, ease);
            let y = world.support(p.x, p.y, terrain.height(p.x, p.y) + 0.8);
            let bob = (time * 9.0 + s as f32).sin().abs() * 0.07;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.95 + hash(s + 2) * 0.15),
                Quat::from_rotation_y(toward.x.atan2(toward.y)),
                Vec3::new(p.x, y + bob, p.y),
            );
            let shirt = linear(SHIRTS[(hash(s + 3) * 8.0) as usize % 8]);
            draws.push((self.body, Instance::tinted(m, shirt.extend(1.0))));
            draws.push((self.heads[w.kind as usize % 3], Instance::new(m)));
            // Some walk their dog, which trots alongside and noses about.
            if hash(s + 7) < 0.3 {
                let dir = toward.normalize_or(Vec2::Y);
                let side = Vec2::new(-dir.y, dir.x);
                let sniff = (time * 0.7 + s as f32).sin();
                let q = p + side * (0.9 + sniff * 0.3) + dir * (0.6 + sniff * 0.4);
                let dy = world.support(q.x, q.y, terrain.height(q.x, q.y) + 0.8);
                let trot = (time * 14.0 + s as f32).sin().abs() * 0.05;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::from_rotation_y(dir.x.atan2(dir.y) + sniff * 0.4),
                    Vec3::new(q.x, dy + trot, q.y),
                );
                draws.push((self.dog, Instance::new(m)));
            }
        }
        for (k, pod) in self.pods.iter().enumerate() {
            if !near(pod.center, 1200.0) {
                continue;
            }
            for d in 0..3u32 {
                let s = k as u32 * 53 + d;
                let period = 5.0 + hash(s) * 3.0;
                let phase = (time / period + hash(s + 1)) % 1.0;
                if phase > 0.22 {
                    continue;
                }
                let u = phase / 0.22;
                // Each leap heads off a new way, from a new spot.
                let round = ((time / period + hash(s + 1)).floor()) as u32;
                let a = hash(s * 7 + round) * TAU;
                let dir = Vec2::new(a.cos(), a.sin());
                let start = pod.center
                    + Vec2::new(hash(s + round) - 0.5, hash(s + round + 3) - 0.5) * 60.0
                    + Vec2::new(d as f32 * 4.0, 0.0);
                let p = start + dir * u * 14.0;
                let y = (u * PI).sin() * 3.2 - 0.8;
                let pitch = (u * PI).cos() * 0.9;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(1.0),
                    Quat::from_rotation_y(dir.x.atan2(dir.y)) * Quat::from_rotation_x(-pitch),
                    Vec3::new(p.x, y, p.y),
                );
                draws.push((self.dolphin, Instance::new(m)));
            }
        }
        for (k, (hub, facing)) in self.turbines.iter().enumerate() {
            if hub.distance(cam) > 4000.0 {
                continue;
            }
            let spin = time * 1.6 + k as f32 * 1.3;
            let m = Mat4::from_translation(*hub)
                * Mat4::from_quat(Quat::from_rotation_arc(Vec3::X, *facing))
                * Mat4::from_rotation_x(spin)
                * Mat4::from_scale(Vec3::splat(zawnabout_sim::landmark::BLADE));
            draws.push((self.rotor, Instance::new(m)));
        }
        for (k, fire) in self.fires.iter().enumerate() {
            if fire.distance(cam) > 1500.0 {
                continue;
            }
            for j in 0..3 {
                let s = k as u32 * 13 + j;
                let flick = (time * (7.0 + j as f32) + hash(s) * 9.0).sin() * 0.15;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.45, 0.42 + flick, 0.45) * (1.0 - j as f32 * 0.2),
                    Quat::from_rotation_y(time * 0.5 + j as f32 * 2.0),
                    *fire + Vec3::new((j as f32 - 1.0) * 0.25, 0.0, 0.0),
                );
                fx.push((self.flame, Instance::new(m)));
            }
        }
        // Fairground wheels turn slowly, their cars hanging level.
        use zawnabout_sim::landmark::{FERRIS_HUB, FERRIS_R, SWING_PIVOT, SWING_ROPE};
        let gondolas = [
            Rgb(0xE0564A),
            Rgb(0x4A90C8),
            Rgb(0xF2C14E),
            Rgb(0x6BBF7A),
            Rgb(0xE88AB0),
        ];
        for (k, f) in self.wheels.iter().enumerate() {
            if f.w_axis.truncate().distance(cam) > 3000.0 {
                continue;
            }
            let spin = time * 0.18 + k as f32;
            let hub = *f * Mat4::from_translation(FERRIS_HUB);
            draws.push((
                self.wheel,
                Instance::new(
                    hub * Mat4::from_rotation_z(spin) * Mat4::from_scale(Vec3::splat(FERRIS_R)),
                ),
            ));
            for i in 0..10 {
                let a = spin + i as f32 / 10.0 * TAU;
                let pin = Vec3::new(a.cos(), a.sin(), 0.0) * FERRIS_R;
                let m = hub * Mat4::from_translation(pin);
                let tint = linear(gondolas[i % 5]).extend(1.0);
                draws.push((self.gondola, Instance::tinted(m, tint)));
            }
        }
        // Rope swings sway in the breeze, as if someone just jumped off.
        for (k, f) in self.swings.iter().enumerate() {
            if f.w_axis.truncate().distance(cam) > 1500.0 {
                continue;
            }
            let sway = (time * 1.15 + k as f32).sin() * 0.35;
            let m = *f
                * Mat4::from_translation(SWING_PIVOT)
                * Mat4::from_rotation_x(sway)
                * Mat4::from_scale(Vec3::splat(SWING_ROPE));
            draws.push((self.seat, Instance::new(m)));
        }
        // Steam drifts up off hot springs.
        for (k, c) in self.springs.iter().enumerate() {
            if c.distance(cam) > 1500.0 {
                continue;
            }
            for i in 0..10u32 {
                let s = k as u32 * 17 + i;
                let u = (time * 0.12 + hash(s)).fract();
                let a = hash(s + 1) * TAU;
                let at =
                    *c + Vec3::new(
                        a.cos() * 4.0 * hash(s + 2),
                        0.5 + u * 9.0,
                        a.sin() * 4.0 * hash(s + 2),
                    ) + Vec3::new(u * 2.0, 0.0, u);
                let size = 1.5 + u * 4.0;
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let m = Mat4::from_scale_rotation_translation(Vec3::new(size, 1.0, size), face, at);
                let fade = (u * 5.0).min(1.0) * (1.0 - u) * 0.22;
                fx.push((
                    self.steam,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, fade)),
                ));
            }
        }
        if night > 0.15 {
            for (k, top) in self.beams.iter().enumerate() {
                if top.distance(cam) > 6000.0 {
                    continue;
                }
                for side in [0.0, PI] {
                    let a = time * 0.7 + k as f32 + side;
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(500.0, 22.0, 22.0),
                        Quat::from_rotation_y(a) * Quat::from_rotation_z(-0.03),
                        *top,
                    );
                    let glow = Vec4::new(1.0, 0.95, 0.8, night * 0.55);
                    fx.push((self.beam, Instance::tinted(m, glow)));
                }
            }
        }
    }
}
