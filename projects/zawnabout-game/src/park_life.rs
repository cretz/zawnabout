//! Life in the big places, only for looks: animals roaming their zoo pens, a roller
//! coaster train racing round its track, a carousel turning, a drop tower lifting and
//! dropping its riders, swimmers, tubers and sliders at the water park, children playing
//! at school, a tractor working the farm's field and cranes moving containers at the port.
//! Everything is a pure function of the clock.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::landmark::{
    CAROUSEL, CRANE_REACH, DROP, FIELDS, LAZY_RIVER, PARK_BENCHES, PLAYGROUND, PLAYING_FIELD,
    PORT_CRANES, SLIDES, STALLS, ZOO_PENS, coaster, slide,
};
use zawnabout_sim::shape::hash;
use zawnabout_sim::town::{Building, Kind};

use crate::life::{SHIRTS, linear};
use crate::mesh::Rgb;
use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// The coaster track sampled by distance along it, with the time the train reaches each
/// sample, so the train climbs the lift hill slowly and races down the drops.
struct Ride {
    /// Track points in the park's frame, evenly spaced along the track.
    pts: Vec<Vec3>,
    /// Spacing between points (m).
    step: f32,
    /// When the front of the train passes each point, from 0 to `lap`.
    at: Vec<f32>,
    lap: f32,
}

/// Points along the coaster track.
const RIDE_PTS: usize = 600;
/// Cars in a coaster train, and how far apart they ride (m).
const CARS: usize = 5;
const CAR_GAP: f32 = 2.7;
/// Speed up the lift hill, and how long the brake run before it is (m/s, m).
const LIFT_SPEED: f32 = 3.5;
const BRAKE_RUN: f32 = 30.0;
const G: f32 = 9.8;

impl Ride {
    fn new() -> Self {
        // Fine samples first, to measure the track.
        let fine = 4000;
        let raw: Vec<Vec3> = (0..=fine)
            .map(|k| coaster(k as f32 / fine as f32))
            .collect();
        let mut len = vec![0.0f32];
        for w in raw.windows(2) {
            len.push(len.last().unwrap() + w[0].distance(w[1]));
        }
        let total = *len.last().unwrap();
        let step = total / RIDE_PTS as f32;
        let mut pts = Vec::with_capacity(RIDE_PTS);
        let mut j = 0;
        for k in 0..RIDE_PTS {
            let s = k as f32 * step;
            while len[j + 1] < s {
                j += 1;
            }
            let u = (s - len[j]) / (len[j + 1] - len[j]).max(1e-6);
            pts.push(raw[j].lerp(raw[j + 1], u));
        }
        // The lift hill climbs from the lowest point before the top to the top.
        let top = (0..RIDE_PTS)
            .max_by(|&a, &b| pts[a].y.total_cmp(&pts[b].y))
            .unwrap();
        let mut foot = top;
        for k in 1..RIDE_PTS {
            let i = (top + RIDE_PTS - k) % RIDE_PTS;
            if pts[i].y <= pts[(i + RIDE_PTS - 1) % RIDE_PTS].y {
                foot = i;
                break;
            }
        }
        let peak = pts[top].y;
        let speed = |i: usize| {
            let on_lift = (i + RIDE_PTS - foot) % RIDE_PTS <= (top + RIDE_PTS - foot) % RIDE_PTS;
            if on_lift {
                return LIFT_SPEED;
            }
            // Free running from the top, losing a little to friction along the way, then
            // easing down on the brakes just before the lift.
            let along = ((i + RIDE_PTS - top) % RIDE_PTS) as f32 * step;
            let free = (LIFT_SPEED * LIFT_SPEED + 2.0 * G * (peak - pts[i].y) - along * 0.02)
                .max(1.0)
                .sqrt();
            let before = ((foot + RIDE_PTS - i) % RIDE_PTS) as f32 * step;
            if before < BRAKE_RUN {
                LIFT_SPEED + (free - LIFT_SPEED) * (before / BRAKE_RUN)
            } else {
                free
            }
        };
        let mut at = Vec::with_capacity(RIDE_PTS);
        let mut t = 0.0;
        for i in 0..RIDE_PTS {
            at.push(t);
            t += step * 2.0 / (speed(i) + speed((i + 1) % RIDE_PTS));
        }
        Self {
            pts,
            step,
            at,
            lap: t,
        }
    }

    /// Distance along the track of the train's front at `time`.
    fn front(&self, time: f32) -> f32 {
        let t = time.rem_euclid(self.lap);
        let i = self.at.partition_point(|&a| a <= t).max(1) - 1;
        let next = if i + 1 < RIDE_PTS {
            self.at[i + 1]
        } else {
            self.lap
        };
        (i as f32 + (t - self.at[i]) / (next - self.at[i]).max(1e-6)) * self.step
    }

    /// The track point `s` meters along.
    fn point(&self, s: f32) -> Vec3 {
        let f = (s / self.step).rem_euclid(RIDE_PTS as f32);
        let i = f as usize % RIDE_PTS;
        self.pts[i].lerp(self.pts[(i + 1) % RIDE_PTS], f.fract())
    }
}

/// A park's frame: x along its axis, y up, z toward its front.
fn frame(b: &Building) -> Mat4 {
    let (a, f) = (b.axis, b.front());
    Mat4::from_cols(
        Vec4::new(a.x, 0.0, a.y, 0.0),
        Vec4::Y,
        Vec4::new(f.x, 0.0, f.y, 0.0),
        b.pos.extend(1.0),
    )
}

/// A yaw turning +z toward `d` on the ground.
fn facing(d: Vec2) -> Quat {
    Quat::from_rotation_y(d.x.atan2(d.y))
}

/// Where a roaming animal is this moment.
#[derive(Clone, Copy)]
struct Roam {
    at: Vec2,
    face: Vec2,
    walking: bool,
    /// Seconds since it last stopped.
    rested: f32,
}

/// A roaming animal walks from one spot to the next at `speed`, then stands a while, a
/// fresh spot each `period`.
fn roam(spot: impl Fn(u32) -> Vec2, time: f32, period: f32, speed: f32) -> Roam {
    let n = (time / period).floor();
    let k = n as i64 as u32;
    let (a, b) = (spot(k), spot(k + 1));
    let walk = (a.distance(b) / speed).clamp(0.1, period * 0.8);
    let since = time - n * period;
    let u = (since / walk).min(1.0);
    let ease = u * u * (3.0 - 2.0 * u);
    let dir = (b - a).normalize_or(Vec2::Y);
    // Turn to the new way at the start of each walk, from the way of the last one.
    let was = (a - spot(k.wrapping_sub(1))).normalize_or(dir);
    let face = was.lerp(dir, (u * 4.0).min(1.0)).normalize_or(dir);
    Roam {
        at: a.lerp(b, ease),
        face,
        walking: u < 1.0,
        rested: (since - walk).max(0.0),
    }
}

pub struct ParkLife {
    zoos: Vec<Mat4>,
    parks: Vec<Mat4>,
    pools: Vec<Mat4>,
    schools: Vec<Mat4>,
    markets: Vec<Mat4>,
    /// Town parks, for the people resting on their benches.
    gardens: Vec<Mat4>,
    farms: Vec<Mat4>,
    ports: Vec<Mat4>,
    ride: Ride,
    tractor: MeshId,
    trolley: MeshId,
    cables: MeshId,
    container: MeshId,
    elephant: MeshId,
    giraffe: MeshId,
    zebra: MeshId,
    lion: MeshId,
    lioness: MeshId,
    penguin: MeshId,
    car: MeshId,
    horse: MeshId,
    drop_ring: MeshId,
    swim_ring: MeshId,
    slides: MeshId,
    body: MeshId,
    heads: [MeshId; 3],
    spray: MeshId,
}

/// Coaster car and swim ring colors.
const PAINT: [Rgb; 6] = [
    Rgb(0xE0564A),
    Rgb(0x4A90C8),
    Rgb(0xF2C14E),
    Rgb(0x6BBF7A),
    Rgb(0xE88AB0),
    Rgb(0x9A7FD1),
];
/// Carousel horse pastels.
const PASTEL: [Rgb; 5] = [
    Rgb(0xFFFFFF),
    Rgb(0xFFD6E4),
    Rgb(0xD6ECFF),
    Rgb(0xFFF1C8),
    Rgb(0xDDF5DD),
];

/// How far away each park's life shows (m).
const ZOO_NEAR: f32 = 600.0;
const PARK_NEAR: f32 = 1200.0;
const POOL_NEAR: f32 = 600.0;
const SLIDES_NEAR: f32 = 3000.0;

impl ParkLife {
    pub fn new(renderer: &mut Renderer, world: &World) -> Self {
        let of = |k: Kind| {
            world
                .buildings
                .iter()
                .filter(move |b| b.kind == k)
                .map(frame)
                .collect()
        };
        let mut add = |m: &crate::mesh::MeshBuilder| renderer.add_mesh(m);
        Self {
            zoos: of(Kind::Zoo),
            parks: of(Kind::ThemePark),
            pools: of(Kind::Waterpark),
            schools: of(Kind::School),
            markets: of(Kind::Market),
            gardens: of(Kind::Park),
            farms: of(Kind::Farm),
            ports: of(Kind::Port),
            ride: Ride::new(),
            tractor: add(&models::tractor()),
            trolley: add(&models::trolley()),
            cables: add(&models::cables()),
            container: add(&models::container()),
            elephant: add(&models::elephant()),
            giraffe: add(&models::giraffe()),
            zebra: add(&models::zebra()),
            lion: add(&models::lion(true)),
            lioness: add(&models::lion(false)),
            penguin: add(&models::penguin()),
            car: add(&models::coaster_car()),
            horse: add(&models::carousel_horse()),
            drop_ring: add(&models::drop_ring()),
            swim_ring: add(&models::swim_ring()),
            slides: add(&models::water_slides()),
            body: add(&models::townsfolk_body()),
            heads: [0, 1, 2].map(|v| add(&models::townsfolk_head(v))),
            spray: add(&models::spray()),
        }
    }

    pub fn draw(
        &self,
        cam: Vec3,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let near = |f: &Mat4, r: f32| f.w_axis.truncate().distance(cam) < r;
        for (k, f) in self.zoos.iter().enumerate() {
            if near(f, ZOO_NEAR) {
                self.zoo(f, k as u32, time, night, draws);
            }
        }
        for (k, f) in self.parks.iter().enumerate() {
            if near(f, PARK_NEAR) {
                self.theme_park(f, k as u32, cam, time, night, draws);
            }
        }
        // People rest on park benches by day, side by side or alone.
        if night < 0.6 {
            for (k, f) in self.gardens.iter().enumerate() {
                if !near(f, ZOO_NEAR) {
                    continue;
                }
                for (b, bench) in PARK_BENCHES.into_iter().enumerate() {
                    for seat in [-0.6f32, 0.6] {
                        let s = (k as u32 * 4 + b as u32 * 2 + (seat > 0.0) as u32)
                            .wrapping_mul(2_654_435_761);
                        // A sitter stays a while, then the seat is empty a while.
                        let slot = (time / 90.0 + hash(s) * 3.0) as u32;
                        if hash(s ^ slot.wrapping_mul(40_503)) < 0.4 {
                            continue;
                        }
                        let m = *f
                            * Mat4::from_rotation_translation(
                                Quat::from_rotation_y(PI),
                                bench + Vec3::new(seat, -0.3, 0.0),
                            );
                        self.person(m, s, draws);
                    }
                }
            }
        }
        // Stallholders behind their counters, and shoppers browsing along the lane.
        if night < 0.6 {
            for (k, f) in self.markets.iter().enumerate() {
                if near(f, ZOO_NEAR) {
                    self.market(f, k as u32, time, draws);
                }
            }
        }
        for (k, f) in self.schools.iter().enumerate() {
            if near(f, ZOO_NEAR) && night < 0.5 {
                self.school(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.farms.iter().enumerate() {
            if near(f, PARK_NEAR) {
                self.farm(f, k as u32, time, night, draws);
            }
        }
        for (k, f) in self.ports.iter().enumerate() {
            if near(f, PARK_NEAR) {
                self.port(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.pools.iter().enumerate() {
            if near(f, SLIDES_NEAR) {
                draws.push((self.slides, Instance::new(*f)));
            }
            if near(f, POOL_NEAR) {
                self.water_park(f, k as u32, time, night, draws, fx);
            }
        }
    }

    /// A person, `seed` picking shirt and head, standing at `m`'s origin.
    fn person(&self, m: Mat4, seed: u32, draws: &mut Vec<(MeshId, Instance)>) {
        let shirt = linear(SHIRTS[(hash(seed) * 8.0) as usize % 8]);
        draws.push((self.body, Instance::tinted(m, shirt.extend(1.0))));
        draws.push((
            self.heads[(hash(seed + 1) * 3.0) as usize % 3],
            Instance::new(m),
        ));
    }

    fn zoo(&self, f: &Mat4, z: u32, time: f32, night: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for (p, (c, r, kind)) in ZOO_PENS.into_iter().enumerate() {
            let pond = c + Vec2::new(r * 0.35, -r * 0.3);
            let pool = if kind == 4 { r * 0.45 } else { r * 0.22 };
            let tree = c + Vec2::new(-r * 0.45, -r * 0.35);
            // Spots to walk to: inside the fence, clear of the tree and the rocks. Penguins
            // like their pool, the rest keep mostly dry.
            let spot = |seed: u32| {
                move |k: u32| {
                    for t in 0..6u32 {
                        let s = seed
                            .wrapping_mul(7919)
                            .wrapping_add(k.wrapping_mul(131) + t);
                        let a = hash(s) * TAU;
                        let d = hash(s + 1).sqrt() * r * 0.72;
                        let q = c + Vec2::new(a.cos(), a.sin()) * d;
                        let rocks = (0..3u32).any(|j| {
                            let a = j as f32 * 2.1 + kind as f32;
                            let rock = c + Vec2::new(a.cos(), a.sin()) * r * 0.55;
                            q.distance(rock) < r * (0.1 + 0.03 * j as f32) + 1.2
                        });
                        let wet = q.distance(pond) < pool && kind != 0 && kind != 4;
                        if q.distance(tree) > 2.5 && !rocks && !wet {
                            return q;
                        }
                    }
                    c
                }
            };
            let (count, mesh, period, speed) = match kind {
                0 => (3, self.elephant, 26.0, 0.9),
                1 => (3, self.giraffe, 20.0, 1.1),
                2 => (6, self.zebra, 14.0, 1.3),
                3 => (3, self.lion, 45.0, 1.0),
                _ => (12, self.penguin, 9.0, 0.45),
            };
            // Animals rest more at night.
            let pace = if night > 0.6 { 0.35 } else { 1.0 };
            let seed_of = |i: u32| z * 1009 + p as u32 * 97 + i;
            let pose = |i: u32| {
                let seed = seed_of(i);
                roam(
                    spot(seed),
                    time * pace + hash(seed + 5) * period,
                    period,
                    speed,
                )
            };
            for i in 0..count {
                let seed = seed_of(i);
                let mut r = pose(i);
                let mut scale = Vec3::splat(0.9 + hash(seed + 6) * 0.2);
                let mut tilt = Quat::IDENTITY;
                let mut y = 0.16;
                let mut mesh = mesh;
                match kind {
                    // The last elephant is a calf, keeping close by its mother.
                    0 if i == count - 1 => {
                        let mom = pose(0);
                        let side = Vec2::new(-mom.face.y, mom.face.x);
                        r = Roam {
                            at: mom.at + side * 2.6 - mom.face * 0.6,
                            ..mom
                        };
                        scale = Vec3::splat(0.5);
                    }
                    3 => {
                        if i > 0 {
                            mesh = self.lioness;
                        }
                        // Lions lie about, getting up only to wander a little.
                        let down = ((r.rested - 4.0) * 0.6).clamp(0.0, 1.0);
                        scale.y *= 1.0 - down * 0.45;
                        scale.x *= 1.0 + down * 0.12;
                    }
                    4 => {
                        if r.at.distance(pond) < pool - 0.3 {
                            // Swimming: lying flat, low in the water.
                            y = 0.05;
                            tilt = Quat::from_rotation_x(1.35);
                        } else if r.walking {
                            tilt = Quat::from_rotation_z((time * 7.0 + i as f32).sin() * 0.14);
                        }
                    }
                    _ => {}
                }
                let bob = if r.walking && kind != 4 {
                    (time * speed * 3.0 + i as f32).sin().abs() * 0.04 * scale.y
                } else {
                    0.0
                };
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        scale,
                        facing(r.face) * tilt,
                        Vec3::new(r.at.x, y + bob, r.at.y),
                    );
                draws.push((mesh, Instance::new(m)));
            }
        }
        // Visitors stroll round the path loop, stopping now and then to look.
        if night < 0.6 {
            for i in 0..16u32 {
                let s = z * 331 + i;
                let dir = if hash(s) < 0.5 { 1.0 } else { -1.0 };
                let a = hash(s + 1) * TAU + dir * time * 1.1 / 33.0
                    - dir * ((time * 0.05 + hash(s + 2) * 9.0).sin().max(0.0) * 3.0);
                let lane = 1.0 + (hash(s + 3) - 0.5) * 2.4;
                let at = Vec3::new(a.cos() * (38.0 + lane), 0.1, a.sin() * (28.0 + lane));
                let tangent = Vec2::new(-a.sin() * 38.0, a.cos() * 28.0) * dir;
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.9 + hash(s + 4) * 0.15),
                        facing(tangent),
                        at,
                    );
                self.person(m, s, draws);
            }
        }
    }

    fn theme_park(
        &self,
        f: &Mat4,
        k: u32,
        cam: Vec3,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        let open = night < 0.75;
        // Two trains, half a lap apart, so there's always one racing somewhere.
        let ride = &self.ride;
        let lap = ride.step * RIDE_PTS as f32;
        for train in 0..2u32 {
            if !open && train == 1 {
                continue;
            }
            let when = time + train as f32 * ride.lap * 0.5 + k as f32 * 7.0;
            let front = ride.front(when);
            let speed = (ride.front(when + 0.05) - front).rem_euclid(lap) / 0.05;
            for c in 0..CARS {
                let s = front - c as f32 * CAR_GAP;
                let (a, b) = (
                    f.transform_point3(ride.point(s - 1.2)),
                    f.transform_point3(ride.point(s + 1.2)),
                );
                let at = f.transform_point3(ride.point(s));
                let fwd = (b - a).normalize_or(Vec3::Z);
                // Bank into the turns, the way the track is built for the speed.
                let (p0, p2) = (
                    f.transform_point3(ride.point(s - 6.0)),
                    f.transform_point3(ride.point(s + 6.0)),
                );
                let (d0, d1) = (
                    Vec2::new(at.x - p0.x, at.z - p0.z),
                    Vec2::new(p2.x - at.x, p2.z - at.z),
                );
                let bend = d0.perp_dot(d1) / (d0.length() * d1.length() * 6.0).max(1e-3);
                let bank = (speed * speed * bend / G).atan().clamp(-0.7, 0.7);
                let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
                let up = right.cross(fwd);
                let base = Mat4::from_cols(
                    (-right).extend(0.0),
                    up.extend(0.0),
                    fwd.extend(0.0),
                    at.extend(1.0),
                ) * Mat4::from_rotation_z(-bank);
                let paint = linear(PAINT[(c + train as usize * 2) % PAINT.len()]);
                draws.push((self.car, Instance::tinted(base, paint.extend(1.0))));
                if cam.distance(at) > 500.0 {
                    continue;
                }
                for seat in 0..4u32 {
                    let s = k * 977 + train * 101 + c as u32 * 7 + seat;
                    if hash(s) < 0.25 {
                        continue;
                    }
                    let x = if seat % 2 == 0 { -0.3 } else { 0.3 };
                    let z = if seat < 2 { -0.5 } else { 0.4 };
                    let m = base
                        * Mat4::from_scale_rotation_translation(
                            Vec3::splat(0.85),
                            Quat::IDENTITY,
                            Vec3::new(x, 0.35, z),
                        );
                    self.person(m, s, draws);
                }
            }
        }
        // The carousel turns, its horses rising and falling on their poles.
        let spin = time * 0.42 + k as f32;
        for ring in 0..2u32 {
            let (r, n) = if ring == 0 { (7.2, 12) } else { (4.6, 8) };
            for i in 0..n {
                let a = spin + (i as f32 + ring as f32 * 0.5) / n as f32 * TAU;
                let pos = CAROUSEL + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let lift = 0.8 + 1.4 + (spin * 3.0 + i as f32 * 2.1).sin() * 0.35;
                let dir = Vec2::new(-a.sin(), a.cos());
                let m = *f * Mat4::from_rotation_translation(facing(dir), pos + Vec3::Y * lift);
                let tint = linear(PASTEL[(i + ring * 2) as usize % PASTEL.len()]);
                draws.push((self.horse, Instance::tinted(m, tint.extend(1.0))));
                let s = k * 503 + ring * 31 + i;
                if open && hash(s) < 0.55 {
                    let kid = 0.6 + hash(s + 2) * 0.25;
                    let seat = m * Mat4::from_scale_rotation_translation(
                        Vec3::splat(kid),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 0.52 - 0.8 * kid, -0.05),
                    );
                    self.person(seat, s, draws);
                }
            }
        }
        // The drop tower: a slow ride up, a pause at the top, a free fall and the brakes.
        let cycle = 42.0;
        let t = (time + k as f32 * 11.0).rem_euclid(cycle);
        let (bottom, top) = (1.3, 54.0);
        let fall = 2.6;
        let fallen = 0.5 * G * fall * fall;
        let v = G * fall;
        let brake = v * v / (2.0 * (top - fallen - bottom));
        let brake_time = v / brake;
        let y = if t < 6.0 {
            bottom
        } else if t < 24.0 {
            let u = (t - 6.0) / 18.0;
            bottom + (top - bottom) * (u * u * (3.0 - 2.0 * u))
        } else if t < 28.0 {
            top
        } else if t < 28.0 + fall {
            let u = t - 28.0;
            top - 0.5 * G * u * u
        } else if t < 28.0 + fall + brake_time {
            let u = t - 28.0 - fall;
            top - fallen - (v * u - 0.5 * brake * u * u)
        } else {
            bottom
        };
        let ring = *f * Mat4::from_translation(DROP + Vec3::Y * y);
        draws.push((self.drop_ring, Instance::new(ring)));
        if open && cam.distance(ring.w_axis.truncate()) < 500.0 {
            for i in 0..8u32 {
                let s = k * 71 + i + (time / cycle).floor() as u32 * 8;
                if hash(s) < 0.3 {
                    continue;
                }
                let a = i as f32 / 8.0 * TAU;
                let at = Vec3::new(a.cos(), 0.0, a.sin()) * 3.75;
                let m = ring
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.85),
                        facing(Vec2::new(a.cos(), a.sin())),
                        at * 1.02 - Vec3::Y * (0.3 + 0.8 * 0.85),
                    );
                self.person(m, s, draws);
            }
        }
        // Crowds stroll up and down in front of the stalls by day.
        if open {
            let span = 130.0;
            for i in 0..24u32 {
                let s = k * 211 + i;
                let u = (hash(s + 1) * span * 2.0 + time * (0.9 + hash(s) * 0.5))
                    .rem_euclid(span * 2.0);
                let (x, way) = if u < span {
                    (u, 1.0)
                } else {
                    (span * 2.0 - u, -1.0)
                };
                let at = Vec3::new(x - span * 0.5, 0.15, 44.5 + hash(s + 2) * 4.0);
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.85 + hash(s + 3) * 0.2),
                        facing(Vec2::new(way, 0.0)),
                        at,
                    );
                self.person(m, s, draws);
            }
        }
    }

    fn market(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for (i, (at, face)) in STALLS.into_iter().enumerate() {
            let s = (k * 16 + i as u32).wrapping_mul(2_654_435_761);
            let sway = (time * 0.6 + i as f32).sin() * 0.2;
            let m = *f
                * Mat4::from_rotation_translation(
                    facing(Vec2::new(sway, face)),
                    Vec3::new(at.x + sway, 0.12, at.y - face * 1.6),
                );
            self.person(m, s, draws);
        }
        // Shoppers stroll the lane, stopping at a stall now and then to look.
        for i in 0..12u32 {
            let seed = (k * 64 + i).wrapping_mul(2_246_822_519);
            let spot = move |n: u32| {
                let (at, face) = STALLS[(hash(seed ^ n.wrapping_mul(40_503)) * 8.0) as usize % 8];
                Vec2::new(at.x + (hash(seed ^ n) - 0.5) * 3.0, at.y + face * 2.2)
            };
            let r = roam(spot, time + hash(seed) * 12.0, 12.0, 1.1);
            // Turned to the counter while stopped.
            let face = if r.walking {
                r.face
            } else {
                Vec2::new(0.0, if r.at.y > 0.0 { 1.0 } else { -1.0 })
            };
            let m = *f
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.9 + hash(seed + 1) * 0.15),
                    facing(face),
                    Vec3::new(r.at.x, 0.12, r.at.y),
                );
            self.person(m, seed, draws);
        }
    }

    /// Children run about the playing field and the playground.
    fn school(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for (place, (c, half), count, period, speed) in [
            (0u32, PLAYING_FIELD, 14u32, 5.0, 3.2),
            (1, PLAYGROUND, 8, 7.0, 1.6),
        ] {
            for i in 0..count {
                let seed = k * 977 + place * 131 + i;
                let spot = move |n: u32| {
                    let s = seed.wrapping_mul(7919).wrapping_add(n.wrapping_mul(613));
                    c + Vec2::new(hash(s) - 0.5, hash(s + 1) - 0.5) * half * 1.8
                };
                let r = roam(spot, time + hash(seed + 2) * period, period, speed);
                let hop = if r.walking {
                    (time * 11.0 + i as f32).sin().abs() * 0.12
                } else {
                    0.0
                };
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.55 + hash(seed + 3) * 0.12),
                        facing(r.face),
                        Vec3::new(r.at.x, 0.12 + hop, r.at.y),
                    );
                self.person(m, seed, draws);
            }
        }
    }

    /// The tractor works the first field row by row, turning at each end, by day.
    fn farm(&self, f: &Mat4, k: u32, time: f32, night: f32, draws: &mut Vec<(MeshId, Instance)>) {
        if night > 0.6 {
            return;
        }
        let (c, h, _) = FIELDS[0];
        let rows = (h.x * 2.0 / 3.0) as u32;
        let len = h.y * 2.0 - 4.0;
        let turn = 3.0;
        let lap = rows as f32 * (len + turn);
        let s = (time * 2.5 + k as f32 * 37.0).rem_euclid(lap);
        let row = (s / (len + turn)) as u32;
        let u = s - row as f32 * (len + turn);
        let x0 = c.x - h.x + 1.5 + row as f32 * 3.0;
        let way = if row.is_multiple_of(2) { 1.0 } else { -1.0 };
        let start = c.y - way * len * 0.5;
        let (at, dir) = if u < len {
            (Vec2::new(x0, start + way * u), Vec2::new(0.0, way))
        } else {
            // Swinging round onto the next row.
            let a = (u - len) / turn * PI;
            let end = start + way * len;
            (
                Vec2::new(x0 + 1.5 - a.cos() * 1.5, end + way * a.sin() * 1.5),
                Vec2::new(a.sin(), way * a.cos()),
            )
        };
        let bump = (time * 9.0).sin() * 0.03;
        let m =
            *f * Mat4::from_rotation_translation(facing(dir), Vec3::new(at.x, 0.1 + bump, at.y));
        draws.push((self.tractor, Instance::new(m)));
    }

    /// Each crane lifts a container off the stacks, runs it out to the quay's edge, sets
    /// it down, and brings it back again, over and over.
    fn port(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let (top, near, far) = (29.6, 14.0, 10.0 + CRANE_REACH - 6.0);
        for (c, x) in PORT_CRANES.into_iter().enumerate() {
            let cycle = 40.0;
            let t = (time + c as f32 * 17.0 + k as f32 * 5.0).rem_euclid(cycle);
            let ease = |u: f32| {
                let u = u.clamp(0.0, 1.0);
                u * u * (3.0 - 2.0 * u)
            };
            // Out with the box in the first half, back with it in the second.
            let (half, back) = if t < cycle * 0.5 {
                (t, false)
            } else {
                (t - cycle * 0.5, true)
            };
            let (from, to) = if back { (far, near) } else { (near, far) };
            // Containers rest on the quay.
            let low = 0.3;
            let (z, hook) = match half {
                h if h < 3.0 => (from, low + (top - 4.0 - low) * ease(h / 3.0)),
                h if h < 13.0 => (from + (to - from) * ease((h - 3.0) / 10.0), top - 4.0),
                h if h < 16.0 => (to, top - 4.0 - (top - 4.0 - low) * ease((h - 13.0) / 3.0)),
                _ => (to, low),
            };
            let trolley = *f * Mat4::from_translation(Vec3::new(x, top, z));
            draws.push((self.trolley, Instance::new(trolley)));
            let drop = top - 0.7 - (hook + 2.6);
            draws.push((
                self.cables,
                Instance::new(
                    *f * Mat4::from_scale_rotation_translation(
                        Vec3::new(1.0, drop.max(0.1), 1.0),
                        Quat::IDENTITY,
                        Vec3::new(x, top - 0.7, z),
                    ),
                ),
            ));
            let paint = linear(PAINT[(k as usize * 2 + c) % PAINT.len()]);
            let m = *f * Mat4::from_translation(Vec3::new(x, hook, z));
            draws.push((self.container, Instance::tinted(m, paint.extend(1.0))));
        }
    }

    fn water_park(
        &self,
        f: &Mat4,
        k: u32,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        if night > 0.6 {
            return;
        }
        let water = 0.36;
        let ring = |m: Mat4, s: u32| {
            let paint = linear(PAINT[(hash(s + 9) * 6.0) as usize % 6]);
            Instance::tinted(m, paint.extend(1.0))
        };
        // Tubers drift round the lazy river, turning slowly as they go.
        let (mid, radii) = LAZY_RIVER;
        let around = PI * (radii.x + radii.y);
        for i in 0..14u32 {
            let s = k * 401 + i;
            let a = (i as f32 + hash(s) * 0.6) / 14.0 * TAU + time * 0.7 / around * TAU;
            let lane = (hash(s + 1) - 0.5) * 1.6;
            let at = Vec3::new(
                mid.x + a.cos() * (radii.x + lane),
                water + (time * 1.3 + i as f32).sin() * 0.04,
                mid.y + a.sin() * (radii.y + lane),
            );
            let spin = Quat::from_rotation_y(time * 0.25 * (hash(s + 2) - 0.5) + hash(s + 3) * TAU);
            let m = *f * Mat4::from_rotation_translation(spin, at);
            draws.push((self.swim_ring, ring(m, s)));
            let rider = m * Mat4::from_translation(Vec3::new(0.0, -0.95, 0.0));
            self.person(rider, s, draws);
        }
        // Swimmers paddle about the pools, heads and shoulders above the water.
        let pools = [
            (Vec2::new(-25.0, -10.0), Vec2::new(19.0, 13.0), 12u32),
            (Vec2::new(30.0, -15.0), Vec2::new(10.0, 10.0), 8),
            (Vec2::new(30.0, 22.0), Vec2::new(5.5, 5.5), 5),
        ];
        for (p, (c, half, n)) in pools.into_iter().enumerate() {
            for i in 0..n {
                let s = k * 811 + p as u32 * 61 + i;
                let drift = Vec2::new(
                    (time * 0.07 + hash(s) * 20.0).sin(),
                    (time * 0.05 + hash(s + 1) * 20.0).cos(),
                );
                let at = c + drift * half;
                let dir = Vec2::new(
                    (time * 0.07 + hash(s) * 20.0).cos() * 0.07,
                    -(time * 0.05 + hash(s + 1) * 20.0).sin() * 0.05,
                );
                let small = if p == 2 {
                    0.65
                } else {
                    0.85 + hash(s + 2) * 0.2
                };
                let bob = (time * 2.0 + i as f32).sin() * 0.05;
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(small),
                        facing(dir),
                        Vec3::new(at.x, water - 1.3 * small + bob, at.y),
                    );
                self.person(m, s, draws);
                if p == 1 && i % 2 == 0 {
                    let rm = *f * Mat4::from_translation(Vec3::new(at.x, water + bob, at.y));
                    draws.push((self.swim_ring, ring(rm, s)));
                }
            }
        }
        // Sliders spin down each slide on a ring and splash into the pool.
        for sl in 0..SLIDES {
            let gap = 9.0 + sl as f32 * 2.0;
            let run = 6.5 + sl as f32 * 1.5;
            let t = (time + sl as f32 * 3.3 + k as f32).rem_euclid(gap);
            let s = k * 37 + sl * 5 + (time / gap).floor() as u32 * 3;
            if t < run {
                // Speeding up as they drop.
                let u = (t / run).powf(1.35).min(0.999);
                let at = slide(sl, u);
                let ahead = slide(sl, (u + 0.01).min(1.0)) - at;
                let m = *f
                    * Mat4::from_rotation_translation(
                        facing(Vec2::new(ahead.x, ahead.z)),
                        at + Vec3::Y * 0.2,
                    );
                draws.push((self.swim_ring, ring(m, s)));
                let rider = m * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.85),
                    Quat::IDENTITY,
                    Vec3::new(0.0, -0.7, 0.0),
                );
                self.person(rider, s, draws);
            } else if t < run + 1.2 {
                let u = (t - run) / 1.2;
                let at = f.transform_point3(slide(sl, 1.0) + Vec3::Y * water);
                for j in 0..5u32 {
                    let a = j as f32 / 5.0 * TAU + sl as f32;
                    let out = Vec3::new(a.cos(), 0.0, a.sin()) * u * 1.6;
                    let lift = (u * PI).sin() * (1.2 + hash(s + j) * 0.8);
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.5 + u * 0.4),
                        Quat::IDENTITY,
                        at + out + Vec3::Y * lift,
                    );
                    fx.push((
                        self.spray,
                        Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, (1.0 - u) * 0.7)),
                    ));
                }
            }
        }
    }
}
