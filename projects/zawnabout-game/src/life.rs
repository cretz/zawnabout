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

/// How far over the ground townsfolk look for their footing: up onto paving, steps and
/// terraces, but never a roof.
const STEP_UP: f32 = 1.2;

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
    /// Underground, the height to look down for the floor from: the cavern town's own
    /// ground rather than the hill far overhead.
    under: Option<f32>,
}

/// A ski run beside a chairlift: from its top station down to its bottom one.
struct Run {
    top: Vec2,
    bottom: Vec2,
}

/// A stretch of town road someone cycles up and down: its points along the right-hand
/// lane, and the distance along at each.
struct Ride {
    pts: Vec<Vec3>,
    along: Vec<f32>,
    /// How far right of the center line the cyclist keeps: by the curb, clear of cars.
    curb: f32,
}

impl Ride {
    fn len(&self) -> f32 {
        *self.along.last().unwrap_or(&0.0)
    }

    /// Position and direction `s` meters along.
    fn at(&self, s: f32) -> (Vec3, Vec3) {
        let i = self
            .along
            .partition_point(|a| *a <= s)
            .clamp(1, self.pts.len() - 1);
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let u = (s - self.along[i - 1]) / (self.along[i] - self.along[i - 1]).max(1e-3);
        (a.lerp(b, u.clamp(0.0, 1.0)), (b - a).normalize_or(Vec3::Z))
    }
}

/// Dolphins leaping now and then near a spot at sea.
struct Pod {
    center: Vec2,
}

pub struct Life {
    rides: Vec<Ride>,
    /// Where someone sits fishing off the end of a pier, facing the water.
    anglers: Vec<Mat4>,
    /// Piers, as where they meet the shore and where they end, for porters carrying
    /// crates out to the boats.
    piers: Vec<(Vec3, Vec3)>,
    crate_box: MeshId,
    bike: MeshId,
    rod: MeshId,
    flocks: Vec<Flock>,
    herds: Vec<Herd>,
    walkers: Vec<Walker>,
    pods: Vec<Pod>,
    beams: Vec<Vec3>,
    /// Each lighthouse's gallery: its middle and how far out the keeper walks round it.
    keepers: Vec<(Vec3, f32)>,
    /// Turbine hubs and the way their rotors face.
    turbines: Vec<(Vec3, Vec3)>,
    fires: Vec<Vec3>,
    /// Fairground wheels, rope swings and hot springs: each its landmark's frame.
    wheels: Vec<Mat4>,
    swings: Vec<Mat4>,
    springs: Vec<Vec3>,
    /// Chimneys and cooling towers letting off steam, and how wide each plume starts.
    stacks: Vec<(Vec3, f32)>,
    /// Water pouring down a dam's spillway: where it leaves and where it lands.
    spills: Vec<(Vec3, Vec3)>,
    runs: Vec<Run>,
    skis: MeshId,
    /// Chimney tops, and flagpole tops with the way they face.
    chimneys: Vec<Vec3>,
    flags: Vec<Vec3>,
    flag: MeshId,
    pole: MeshId,
    wheel: MeshId,
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
    /// Body and head as two boxes, for afar.
    body_far: MeshId,
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
pub(crate) const SHIRTS: [Rgb; 8] = [
    Rgb(0xE0564A),
    Rgb(0x4A90C8),
    Rgb(0xF2C14E),
    Rgb(0x6BBF7A),
    Rgb(0xE88AB0),
    Rgb(0x9A7FD1),
    Rgb(0xF6F1E7),
    Rgb(0x3E7F8C),
];

impl Life {
    /// The townsfolk meshes, body and heads, for others to draw people with.
    pub fn people(&self) -> (MeshId, [MeshId; 3]) {
        (self.body, self.heads)
    }

    /// A person seen from afar, standing in for body and head.
    pub fn person_far(&self) -> MeshId {
        self.body_far
    }

    /// Where walker `i` is at `time`, and which way they walk. None when they are
    /// indoors for the night.
    fn walker_at(&self, i: usize, world: &World, time: f32, night: f32) -> Option<(Vec3, Vec2)> {
        let w = &self.walkers[i];
        let s = i as u32 * 31;
        if night > 0.5 && hash(s + 1) < 0.7 {
            return None;
        }
        let len = w.a.distance(w.b);
        // Out, a pause at the door, and back.
        let u = (time * 1.3 / len + hash(s) * 10.0) % 2.0;
        let (t, toward) = if u < 1.0 {
            (u, w.b - w.a)
        } else {
            (2.0 - u, w.a - w.b)
        };
        let p = w.a.lerp(w.b, t.clamp(0.0, 1.0));
        let top = w
            .under
            .unwrap_or_else(|| world.terrain.height(p.x, p.y) + STEP_UP);
        let y = world.support(p.x, p.y, top);
        let bob = (time * 9.0 + s as f32).sin().abs() * 0.07;
        Some((Vec3::new(p.x, y + bob, p.y), toward))
    }

    /// The townsperson nearest `at` within `reach` meters, and a number that is theirs
    /// alone, for what they say.
    pub fn nearest_person(
        &self,
        world: &World,
        at: Vec3,
        reach: f32,
        time: f32,
        night: f32,
    ) -> Option<(Vec3, u32)> {
        let flat = Vec2::new(at.x, at.z);
        self.walkers
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                let len = w.a.distance(w.b);
                w.a.distance(flat) < len + reach && w.b.distance(flat) < len + reach
            })
            .filter_map(|(i, _)| {
                let (p, _) = self.walker_at(i, world, time, night)?;
                let d = Vec2::new(p.x, p.z).distance(flat);
                (d < reach && (p.y - at.y).abs() < 4.0).then_some((d, p, i as u32 * 31))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, p, s)| (p, s))
    }
}

pub(crate) fn linear(c: Rgb) -> Vec3 {
    let f = |s: u32| ((s & 0xFF) as f32 / 255.0).powf(2.2);
    Vec3::new(f(c.0 >> 16), f(c.0 >> 8), f(c.0))
}

impl Life {
    /// `pool` is the shared soft glow, for steam and lanterns.
    pub fn new(renderer: &mut Renderer, world: &World, pool: MeshId) -> Self {
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
            // A row house shows its gable to the street, so its door is at the end of
            // its ridge, along its axis.
            let door = |b: &zawnabout_sim::town::Building| {
                let (out, depth) = if b.kind == Kind::Row {
                    (b.axis, b.size.x)
                } else {
                    (b.front(), b.size.z)
                };
                Vec2::new(b.pos.x, b.pos.z) + out * (depth * 0.5 + 1.5)
            };
            let (a, b) = (door(w[0]), door(w[1]));
            let under = (w[0].style == zawnabout_sim::terrain::Style::Cavern)
                .then(|| w[0].pos.y.max(w[1].pos.y) + 3.0);
            // Cavern towns are busy at every hour, so they have twice the people.
            // Only a way with nothing solid across it: no walking through walls.
            // and no sudden drop or climb, which they would float over.
            let clear = |a: Vec2, b: Vec2| {
                let n = (a.distance(b) / 0.8).ceil() as usize;
                let mut last: Option<f32> = None;
                (0..=n).all(|k| {
                    let p = a.lerp(b, k as f32 / n.max(1) as f32);
                    let top = under.unwrap_or_else(|| world.terrain.height(p.x, p.y) + STEP_UP);
                    let y = world.support_still(p.x, p.y, top);
                    let step = last.is_none_or(|l| (y - l).abs() < 2.0);
                    last = Some(y);
                    // By the doors themselves they step in and out of the house.
                    let door = p.distance(a).min(p.distance(b)) < 3.0;
                    step && (door || !world.in_building(Vec3::new(p.x, y + 1.0, p.y)))
                })
            };
            if (i % 4 == 0 || (under.is_some() && i % 2 == 0))
                && a.distance(b) > 3.0
                && a.distance(b) < 120.0
                && clear(a, b)
            {
                walkers.push(Walker {
                    a,
                    b,
                    kind: (hash(i as u32 * 17) * 3.0) as u8,
                    under,
                });
            }
        }
        let beams: Vec<Vec3> = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Lighthouse)
            .map(|b| b.pos + Vec3::Y * (b.size.y + zawnabout_sim::town::LAMP_ROOM * 0.5))
            .collect();
        let keepers = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Lighthouse)
            .map(|b| (b.pos + Vec3::Y * b.size.y, b.size.x * 0.5 * 0.64 + 0.7))
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
        let stacks = world
            .buildings
            .iter()
            .flat_map(|b| {
                zawnabout_sim::landmark::stacks(b.kind)
                    .into_iter()
                    .map(|(c, w)| (b.at(c.x, c.y, c.z), w))
            })
            .collect();
        let spills = world
            .buildings
            .iter()
            .flat_map(|b| {
                zawnabout_sim::landmark::spills(b.kind)
                    .into_iter()
                    .map(|(a, c)| (b.at(a.x, a.y, a.z), b.at(c.x, c.y, c.z)))
            })
            .collect();
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
        // Cyclists ride stretches of road through towns, one per stretch, well apart.
        let mut rides = Vec::new();
        for (i, r) in terrain.def.roads.iter().enumerate() {
            if r.rail || r.runway {
                continue;
            }
            let prof = terrain.road_points(i);
            let mut run: Vec<Vec3> = Vec::new();
            let curb = r.w * 0.5 - 1.1;
            let flush = |run: &mut Vec<Vec3>, rides: &mut Vec<Ride>| {
                let mut along = vec![0.0];
                for w in run.windows(2) {
                    along.push(along[along.len() - 1] + w[0].distance(w[1]));
                }
                if along[along.len() - 1] > 80.0 && rides.len() < 80 {
                    rides.push(Ride {
                        pts: std::mem::take(run),
                        along,
                        curb,
                    });
                }
                run.clear();
            };
            for (q, p) in r.pts.iter().zip(prof) {
                let town = !p.bridge
                    && !p.tunnel
                    && terrain
                        .cover(q[0], q[1])
                        .is_some_and(|c| c.kind == CoverKind::Town);
                // Long roads are split into stretches of about 300 m.
                if town && run.len() < 40 {
                    run.push(Vec3::new(q[0], p.y + 0.25, q[1]));
                } else {
                    flush(&mut run, &mut rides);
                }
            }
            flush(&mut run, &mut rides);
        }
        let piers = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Pier && b.size.x > 60.0)
            .map(|b| {
                (
                    b.at(-b.size.x * 0.5 + 2.0, b.size.y, 0.0),
                    b.at(b.size.x * 0.5 - 6.0, b.size.y, 0.0),
                )
            })
            .collect();
        let anglers = world
            .buildings
            .iter()
            .filter(|b| b.kind == Kind::Pier)
            .flat_map(|b| {
                [-1.0f32, 1.0].map(|side| {
                    let a = Vec3::new(b.axis.x, 0.0, b.axis.y);
                    let at = b.at(
                        b.size.x * 0.5 - 0.6,
                        b.size.y,
                        side * (b.size.z * 0.5 - 1.0),
                    );
                    // Facing out along the pier, turned a little to the side.
                    let face =
                        Quat::from_rotation_arc(Vec3::Z, a) * Quat::from_rotation_y(side * 0.4);
                    Mat4::from_rotation_translation(face, at)
                })
            })
            .collect();
        Self {
            rides,
            anglers,
            piers,
            crate_box: renderer.add_mesh(&models::crate_box()),
            bike: renderer.add_mesh(&models::bicycle()),
            rod: renderer.add_mesh(&models::fishing_rod()),
            wheels,
            swings,
            springs,
            stacks,
            spills,
            chimneys,
            flags,
            flag: renderer.add_mesh(&models::flag()),
            pole: renderer.add_mesh(&models::flagpole()),
            runs,
            skis: renderer.add_mesh(&models::skis()),
            wheel: renderer.add_mesh(&models::ferris_wheel()),
            seat: renderer.add_mesh(&models::swing_seat()),
            steam: pool,
            turbines,
            fires,
            rotor: renderer.add_mesh(&models::rotor()),
            flame: renderer.add_mesh(&models::flame()),
            flocks,
            herds,
            walkers,
            pods,
            beams,
            keepers,
            gull: renderer.add_mesh(&models::gull()),
            sheep: renderer.add_mesh(&models::sheep()),
            cow: renderer.add_mesh(&models::cow()),
            horse: renderer.add_mesh(&models::horse()),
            dog: renderer.add_mesh(&models::dog()),
            body: renderer.add_mesh(&models::townsfolk_body()),
            body_far: renderer.add_mesh(&models::townsfolk_far()),
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
        sim_time: f32,
        night: f32,
        riding: Option<u32>,
        dark: &[glam::Vec4],
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
        // Gulls trail every fishing boat, wheeling low over its stern.
        for (i, v) in world.traffic.vehicles.iter().enumerate() {
            if v.kind != Kind::Trawler {
                continue;
            }
            let p = world.traffic.poses[i];
            if !near(Vec2::new(p.pos.x, p.pos.z), 1200.0) {
                continue;
            }
            let stern = p.pos - Vec3::new(p.axis().x, 0.0, p.axis().y) * v.size.x * 0.5;
            for b in 0..8u32 {
                let s = i as u32 * 131 + b;
                let r = 6.0 + hash(s) * 12.0;
                let dir = if hash(s + 2) < 0.5 { 1.0 } else { -1.0 };
                let a = time * (8.0 + hash(s + 1) * 4.0) / r * dir + hash(s + 3) * TAU;
                let y = 7.0 + hash(s + 4) * 8.0 + (time * 0.7 + s as f32).sin() * 2.0;
                let pos = stern + Vec3::new(a.cos() * r, y, a.sin() * r);
                let heading = (-a.sin() * dir).atan2(a.cos() * dir);
                let flap = 0.35 + 0.65 * (time * 9.0 + s as f32).sin();
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, flap, 1.0) * 1.3,
                    Quat::from_rotation_y(heading) * Quat::from_rotation_z(0.35 * dir),
                    pos,
                );
                draws.push((self.gull, Instance::new(m)));
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
        // Cyclists pedal up and down their stretch of road, keeping to the right, by day.
        if night < 0.6 {
            for (k, r) in self.rides.iter().enumerate() {
                let len = r.len();
                if !near(Vec2::new(r.pts[0].x, r.pts[0].z), 450.0 + len) {
                    continue;
                }
                let s = k as u32 * 59;
                let u =
                    (time * (4.0 + hash(s) * 2.0) + hash(s + 1) * len * 2.0).rem_euclid(len * 2.0);
                let (along, back) = if u < len {
                    (u, false)
                } else {
                    (len * 2.0 - u, true)
                };
                let (p, dir) = r.at(along);
                let dir = if back { -dir } else { dir };
                let right = Vec3::new(-dir.z, 0.0, dir.x).normalize_or(Vec3::X);
                let at = p + right * r.curb;
                if at.distance(cam) > 450.0 {
                    continue;
                }
                let face = Quat::from_rotation_y(dir.x.atan2(dir.z));
                let bike = Mat4::from_rotation_translation(face, at);
                let paint = linear(SHIRTS[(hash(s + 2) * 8.0) as usize % 8]);
                draws.push((self.bike, Instance::tinted(bike, paint.extend(1.0))));
                let pedal = (time * 6.0 + s as f32).sin() * 0.04;
                let rider = bike
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.9),
                        Quat::from_rotation_x(0.25),
                        Vec3::new(0.0, 0.18 + pedal, -0.1),
                    );
                let shirt = linear(SHIRTS[(hash(s + 3) * 8.0) as usize % 8]);
                draws.push((self.body, Instance::tinted(rider, shirt.extend(1.0))));
                draws.push((self.heads[(s % 3) as usize], Instance::new(rider)));
            }
        }
        // Anglers sit at the ends of piers with their rods out, legs over the edge.
        for (k, m) in self.anglers.iter().enumerate() {
            let s = k as u32 * 23 + 5;
            if hash(s) < 0.35 || m.w_axis.truncate().distance(cam) > 400.0 {
                continue;
            }
            let seated = *m * Mat4::from_translation(Vec3::new(0.0, -0.8, 0.0));
            let shirt = linear(SHIRTS[(hash(s + 1) * 8.0) as usize % 8]);
            draws.push((self.body, Instance::tinted(seated, shirt.extend(1.0))));
            draws.push((self.heads[(s % 3) as usize], Instance::new(seated)));
            // The rod twitches now and then as something nibbles.
            let nibble = ((time * 0.7 + s as f32).sin().max(0.0).powi(8)) * 0.15;
            let rod = *m
                * Mat4::from_rotation_translation(
                    Quat::from_rotation_x(-nibble),
                    Vec3::new(0.0, 0.8, 0.3),
                );
            draws.push((self.rod, Instance::new(rod)));
        }
        // Porters carry crates out along the long piers to the boats, and walk back for
        // more, by day.
        if night < 0.6 {
            for (k, (shore, end)) in self.piers.iter().enumerate() {
                if shore.distance(cam) > 450.0 {
                    continue;
                }
                let dir = (*end - *shore).normalize_or(Vec3::Z);
                let side = Vec3::new(-dir.z, 0.0, dir.x);
                let len = shore.distance(*end);
                for i in 0..3u32 {
                    let s = k as u32 * 37 + i;
                    let u = (time * 1.2 / len + hash(s) * 2.0) % 2.0;
                    let (t, out) = if u < 1.0 { (u, true) } else { (2.0 - u, false) };
                    let at = shore.lerp(*end, t) + side * (i as f32 - 1.0) * 1.6;
                    let face = if out { dir } else { -dir };
                    let m = Mat4::from_rotation_translation(
                        Quat::from_rotation_y(face.x.atan2(face.z)),
                        at,
                    );
                    let shirt = linear(SHIRTS[(hash(s + 1) * 8.0) as usize % 8]);
                    draws.push((self.body, Instance::tinted(m, shirt.extend(1.0))));
                    draws.push((self.heads[(s % 3) as usize], Instance::new(m)));
                    if out {
                        let held = m * Mat4::from_translation(Vec3::new(0.0, 1.05, 0.35));
                        draws.push((self.crate_box, Instance::new(held)));
                    }
                }
            }
        }
        // Each lighthouse keeper takes a slow turn round the gallery by day, stopping now
        // and then to look out to sea.
        if night < 0.6 {
            for (k, (mid, r)) in self.keepers.iter().enumerate() {
                if mid.distance(cam) > 450.0 {
                    continue;
                }
                let t = time * 0.05 + k as f32 * 1.7;
                let a = t + (t * 3.0).sin() * 0.3;
                let at = *mid + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let walk = Vec3::new(-a.sin(), 0.0, a.cos());
                let out = Vec3::new(a.cos(), 0.0, a.sin());
                let look = if (t * 3.0).cos() > 0.6 { out } else { walk };
                let m = Mat4::from_rotation_translation(
                    Quat::from_rotation_y(look.x.atan2(look.z)),
                    at,
                );
                let shirt = linear(Rgb(0x2E4A7A));
                draws.push((self.body, Instance::tinted(m, shirt.extend(1.0))));
                draws.push((self.heads[k % 3], Instance::new(m)));
            }
        }
        // Fewer people about at night.
        for (i, w) in self.walkers.iter().enumerate() {
            let s = i as u32 * 31;
            // Someone off for a ride on the dragon's back is not walking about.
            if !near(w.a, 450.0) || riding == Some(s) {
                continue;
            }
            let Some((at, toward)) = self.walker_at(i, world, time, night) else {
                continue;
            };
            let p = Vec2::new(at.x, at.z);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.95 + hash(s + 2) * 0.15),
                Quat::from_rotation_y(toward.x.atan2(toward.y)),
                at,
            );
            let shirt = linear(SHIRTS[(hash(s + 3) * 8.0) as usize % 8]);
            draws.push((self.body, Instance::tinted(m, shirt.extend(1.0))));
            draws.push((self.heads[w.kind as usize % 3], Instance::new(m)));
            // Underground, many carry a lantern, swinging as they go.
            if w.under.is_some() && hash(s + 9) < 0.5 {
                let swing = (time * 4.5 + s as f32).sin() * 0.06;
                let hand = m * Mat4::from_translation(Vec3::new(0.32, 0.72 + swing, 0.12));
                let face = Quat::from_rotation_arc(
                    Vec3::Y,
                    (cam - hand.w_axis.truncate()).normalize_or(Vec3::Y),
                );
                let glow = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.35, 1.0, 0.35),
                    face,
                    hand.w_axis.truncate(),
                );
                fx.push((
                    self.steam,
                    Instance::tinted(glow, Vec4::new(1.0, 0.75, 0.4, 0.9)),
                ));
            }
            // Some walk their dog, which trots alongside and noses about.
            if hash(s + 7) < 0.3 {
                let dir = toward.normalize_or(Vec2::Y);
                let side = Vec2::new(-dir.y, dir.x);
                let sniff = (time * 0.7 + s as f32).sin();
                let q = p + side * (0.9 + sniff * 0.3) + dir * (0.6 + sniff * 0.4);
                let dy = world.support(
                    q.x,
                    q.y,
                    w.under.unwrap_or_else(|| terrain.height(q.x, q.y) + 0.8),
                );
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
        // Fairground wheels turn slowly on the sim's clock. Their cars are vehicles, which
        // can be landed on and ridden, so the wheel keeps time with them.
        use zawnabout_sim::landmark::{FERRIS_HUB, FERRIS_R, FERRIS_SPIN, SWING_PIVOT, SWING_ROPE};
        for (k, f) in self.wheels.iter().enumerate() {
            if f.w_axis.truncate().distance(cam) > 3000.0 {
                continue;
            }
            let spin = sim_time * FERRIS_SPIN + k as f32;
            let hub = *f * Mat4::from_translation(FERRIS_HUB);
            draws.push((
                self.wheel,
                Instance::new(
                    hub * Mat4::from_rotation_z(spin) * Mat4::from_scale(Vec3::splat(FERRIS_R)),
                ),
            ));
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
        // Steam billows up off factory chimneys and cooling towers, drifting off on the wind.
        for (k, (c, w)) in self.stacks.iter().enumerate() {
            if c.distance(cam) > 3000.0 {
                continue;
            }
            for i in 0..8u32 {
                let s = k as u32 * 23 + i;
                let u = (time * 0.05 + i as f32 / 8.0 + hash(s) * 0.1).fract();
                let at = *c
                    + Vec3::new(
                        u * w * 3.0 + (time * 0.3 + i as f32).sin() * u * 2.0,
                        u * w * 4.0 + 1.0,
                        u * w * 1.2,
                    );
                let size = w * (1.0 + u * 1.8);
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let m = Mat4::from_scale_rotation_translation(Vec3::new(size, 1.0, size), face, at);
                let fade = (u * 6.0).min(1.0) * (1.0 - u) * 0.5;
                fx.push((
                    self.steam,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, fade)),
                ));
            }
        }
        // Water pours down dam spillways, white and rushing, and mist rises off the foot.
        for (k, (top, foot)) in self.spills.iter().enumerate() {
            if top.distance(cam) > 2500.0 {
                continue;
            }
            for i in 0..24u32 {
                let s = k as u32 * 31 + i;
                let u = (time * 0.35 + i as f32 / 24.0).fract();
                // Falling faster as it goes, spreading as it falls.
                let f = u * u;
                let at = top.lerp(*foot, f)
                    + Vec3::new(hash(s) - 0.5, 0.0, hash(s + 1) - 0.5) * (1.0 + f * 4.0);
                let size = 2.2 + f * 3.0;
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let m = Mat4::from_scale_rotation_translation(Vec3::new(size, 1.0, size), face, at);
                fx.push((
                    self.steam,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, 0.7)),
                ));
            }
            for i in 0..8u32 {
                let s = k as u32 * 37 + i;
                let u = (time * 0.1 + hash(s)).fract();
                let a = hash(s + 1) * TAU;
                let at = *foot + Vec3::new(a.cos() * 8.0 * u, 2.0 + u * 14.0, a.sin() * 8.0 * u);
                let size = 5.0 + u * 8.0;
                let face = Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                let m = Mat4::from_scale_rotation_translation(Vec3::new(size, 1.0, size), face, at);
                let fade = (u * 5.0).min(1.0) * (1.0 - u) * 0.35;
                fx.push((
                    self.steam,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, fade)),
                ));
            }
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
                if top.distance(cam) > 6000.0 || crate::gfx::in_dark(dark, *top) {
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
