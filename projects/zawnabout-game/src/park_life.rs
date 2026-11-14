//! Life in the big places, only for looks: animals roaming their zoo pens, a roller
//! coaster train racing round its track, a carousel turning, a drop tower lifting and
//! dropping its riders, swimmers, tubers and sliders at the water park, children playing
//! at school, a tractor working the farm's field, cranes moving containers at the port and
//! shipyard cranes swinging planks into the hull on the slip.
//! Everything is a pure function of the clock.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::landmark::{
    AMPHI_SEATS, BUS_BENCH, CAFE_TABLES, CAROUSEL, CINEMA_QUEUE, CRANE_REACH, DRAW_DECK,
    DRAW_HINGE, DROP, FARM_TRAIL, FIELDS, GOLF_HOLES, LAZY_RIVER, LIBRARY_BENCHES, METRO_WAIT,
    MINE_TRACK, MUSEUM_STEPS, PARK_BENCHES, PASTURE, PLAYGROUND, PLAYING_FIELD, PORT_CRANES,
    Platform, QUAD, RACE_OVAL, RESTAURANT_TABLES, SHROOM_BED_LEN, SHROOM_BEDS, SKATE_BOXES,
    SKATE_LINES, SLIDES, STAGE, STALLS, STATION_PLATFORM, ZOO_PENS, picnic_tables, slide,
};
use zawnabout_sim::shape::hash;
use zawnabout_sim::town::{Building, Kind};

use crate::life::{SHIRTS, linear};
use crate::mesh::Rgb;
use crate::models;
use crate::render::{Instance, MeshId, Renderer};

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

/// A shipyard crane's jib length and how far out its hook hangs (m), and how near it
/// works. Past that only the tower is drawn.
const YARD_JIB: f32 = 24.0;
const YARD_REACH: f32 = 17.0;
const YARD_NEAR: f32 = 3000.0;

/// A shipyard crane: its frame at the tower foot, the tower's height, and where it picks
/// up (a turn of the jib from +z, and the pile's foot) and drops (over the hull's deck).
/// Heights are above the tower foot.
struct YardCrane {
    frame: Mat4,
    height: f32,
    pick: f32,
    pick_y: f32,
    drop: f32,
    drop_y: f32,
}

/// Every shipyard crane, turning from a dry clear pile beside it to the hull's deck.
fn yard_cranes(world: &World) -> Vec<YardCrane> {
    let t = &world.terrain;
    let flat = |v: Vec3| Vec2::new(v.x, v.z);
    world
        .buildings
        .iter()
        .filter(|b| b.kind == Kind::Crane)
        .filter_map(|b| {
            let hull = world
                .buildings
                .iter()
                .filter(|h| h.kind == Kind::Hull)
                .min_by(|a, c| {
                    flat(a.pos - b.pos)
                        .length()
                        .total_cmp(&flat(c.pos - b.pos).length())
                })?;
            let frame = frame(b);
            let spot = |turn: f32| {
                frame.transform_point3(Quat::from_rotation_y(turn) * Vec3::Z * YARD_REACH)
            };
            // Over the hull: the turn whose hook hangs nearest the hull's middle line.
            let drop = [FRAC_PI_2, -FRAC_PI_2]
                .into_iter()
                .min_by(|a, c| {
                    flat(spot(*a) - hull.pos)
                        .length()
                        .total_cmp(&flat(spot(*c) - hull.pos).length())
                })
                .expect("two turns");
            // The pile goes on dry ground clear of other buildings.
            let clear = |p: Vec3| {
                let h = t.height(p.x, p.z);
                t.water(p.x, p.z, h).is_none()
                    && world.buildings.iter().all(|o| {
                        std::ptr::eq(o, b)
                            || flat(o.pos - p).length() > o.size.x.max(o.size.z) * 0.5 + 3.0
                    })
            };
            let pick = [0.0, PI, drop + PI].into_iter().find(|&a| clear(spot(a)))?;
            let ground = spot(pick);
            Some(YardCrane {
                frame,
                height: b.size.y,
                pick,
                pick_y: t.height(ground.x, ground.z) - b.pos.y,
                drop,
                drop_y: hull.pos.y + hull.size.y - b.pos.y,
            })
        })
        .collect()
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

/// Eases 0 to 1 in and out.
fn smooth01(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
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
    carnivals: Vec<Mat4>,
    seal_colonies: Vec<Mat4>,
    ports: Vec<Mat4>,
    yard_cranes: Vec<YardCrane>,
    /// Underground: mushroom farms, crystal mines and cafes.
    shroom_farms: Vec<Mat4>,
    mines: Vec<Mat4>,
    cafes: Vec<Mat4>,
    restaurants: Vec<Mat4>,
    drawbridges: Vec<Mat4>,
    skateparks: Vec<Mat4>,
    board: MeshId,
    leaf: MeshId,
    /// Which markets are underground, where it is never night.
    deep_markets: Vec<bool>,
    /// Station platforms people wait on, bus stops, picnic spots (with their seeds).
    platforms: Vec<(Mat4, &'static Platform)>,
    cinemas: Vec<Mat4>,
    racetracks: Vec<Mat4>,
    aquariums: Vec<Mat4>,
    universities: Vec<Mat4>,
    stations: Vec<Mat4>,
    pumps: Vec<Mat4>,
    theaters: Vec<Mat4>,
    golf: Vec<Mat4>,
    museums: Vec<Mat4>,
    libraries: Vec<Mat4>,
    bus_stops: Vec<Mat4>,
    picnics: Vec<(Mat4, u32)>,
    /// Where fireworks go up over the harbors at night.
    shows: Vec<Vec3>,
    kite: MeshId,
    line: MeshId,
    water_skis: MeshId,
    parasail: MeshId,
    cow: MeshId,
    swing_crown: MeshId,
    swing_chair: MeshId,
    bumper_car: MeshId,
    parcel: MeshId,
    seal: MeshId,
    sheep: MeshId,
    mount: MeshId,
    ore_cart: MeshId,
    glow: MeshId,
    /// Coaster cars in the traffic, for their riders.
    coaster_cars: Vec<usize>,
    /// Helicopters in the traffic, for their turning rotors.
    helicopters: Vec<usize>,
    /// Every clock face: middle, which way it looks, and its minute hand's length.
    clocks: Vec<(Vec3, Vec3, f32)>,
    hand: MeshId,
    rotor: MeshId,
    tractor: MeshId,
    trolley: MeshId,
    cables: MeshId,
    yard_jib: MeshId,
    planks: MeshId,
    container: MeshId,
    elephant: MeshId,
    giraffe: MeshId,
    zebra: MeshId,
    lion: MeshId,
    lioness: MeshId,
    penguin: MeshId,
    horse: MeshId,
    swim_ring: MeshId,
    bucket: MeshId,
    slides: MeshId,
    slides_far: MeshId,
    track: MeshId,
    /// The track as one beam, for afar.
    track_far: MeshId,
    race: MeshId,
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
/// Crowds that are only people draw nearer than the places they are at: past this they
/// are a few pixels each (m).
const PEOPLE_NEAR: f32 = 450.0;
const CROWD_NEAR: f32 = 600.0;
const POOL_NEAR: f32 = 600.0;
const SLIDES_NEAR: f32 = 3000.0;

impl ParkLife {
    /// Draws people with the townsfolk meshes `people`, and lamps with the shared glow.
    pub fn new(
        renderer: &mut Renderer,
        world: &World,
        people: (MeshId, [MeshId; 3]),
        glow: MeshId,
    ) -> Self {
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
            carnivals: of(Kind::Carnival),
            seal_colonies: of(Kind::Seals),
            ports: of(Kind::Port),
            yard_cranes: yard_cranes(world),
            shroom_farms: of(Kind::ShroomFarm),
            mines: of(Kind::Mine),
            cafes: of(Kind::Cafe),
            restaurants: of(Kind::Restaurant),
            drawbridges: of(Kind::Drawbridge),
            skateparks: of(Kind::SkatePark),
            board: add(&models::skateboard()),
            leaf: add(&models::bridge_leaf()),
            deep_markets: world
                .buildings
                .iter()
                .filter(|b| b.kind == Kind::Market)
                .map(|b| b.style == zawnabout_sim::terrain::Style::Cavern)
                .collect(),
            platforms: world
                .buildings
                .iter()
                .filter_map(|b| match b.kind {
                    Kind::Station => Some((frame(b), &STATION_PLATFORM)),
                    Kind::Metro => Some((frame(b), &METRO_WAIT)),
                    _ => None,
                })
                .collect(),
            bus_stops: of(Kind::BusStop),
            cinemas: of(Kind::Cinema),
            racetracks: of(Kind::Racetrack),
            aquariums: of(Kind::Aquarium),
            universities: of(Kind::University),
            stations: of(Kind::Police),
            pumps: of(Kind::GasStation),
            theaters: of(Kind::Amphitheater),
            golf: of(Kind::Golf),
            museums: of(Kind::Museum),
            libraries: of(Kind::Library),
            picnics: world
                .buildings
                .iter()
                .filter(|b| b.kind == Kind::Picnic)
                .map(|b| (frame(b), b.seed))
                .collect(),
            shows: world
                .terrain
                .def
                .marks
                .iter()
                .filter(|m| m.kind == "site:marina" || m.kind == "site:docks")
                .map(|m| {
                    Vec3::new(
                        m.pos[0],
                        world.terrain.height(m.pos[0], m.pos[2]).max(0.0),
                        m.pos[2],
                    )
                })
                .collect(),
            kite: add(&models::kite()),
            line: add(&models::line()),
            water_skis: add(&models::water_skis()),
            parasail: add(&models::parasail()),
            cow: add(&models::cow()),
            swing_crown: add(&models::swing_crown()),
            swing_chair: add(&models::swing_chair(zawnabout_sim::landmark::SWING_REACH)),
            bumper_car: add(&models::bumper_car()),
            parcel: add(&{
                let mut m = crate::mesh::MeshBuilder::new();
                m.cube(Vec3::ZERO, Vec3::new(0.22, 0.16, 0.18), Rgb(0xC9935A));
                m.cube(Vec3::Y * 0.161, Vec3::new(0.23, 0.002, 0.04), Rgb(0xE8DCC0));
                m
            }),
            seal: add(&models::seal()),
            sheep: add(&models::sheep()),
            mount: add(&models::horse()),
            ore_cart: add(&models::ore_cart()),
            glow,
            helicopters: world
                .traffic
                .vehicles
                .iter()
                .enumerate()
                .filter(|(_, v)| v.kind == Kind::Helicopter)
                .map(|(i, _)| i)
                .collect(),
            rotor: add(&models::heli_rotor()),
            clocks: world
                .buildings
                .iter()
                .flat_map(|b| {
                    let f = frame(b);
                    zawnabout_sim::landmark::clocks(b.kind)
                        .into_iter()
                        .map(move |(at, n, len)| {
                            (f.transform_point3(at), f.transform_vector3(n), len)
                        })
                })
                .collect(),
            hand: add(&models::clock_hand()),
            coaster_cars: world
                .traffic
                .vehicles
                .iter()
                .enumerate()
                .filter(|(_, v)| v.kind == Kind::CoasterCar)
                .map(|(i, _)| i)
                .collect(),
            tractor: add(&models::tractor()),
            trolley: add(&models::trolley()),
            cables: add(&models::cables()),
            yard_jib: add(&models::yard_jib(YARD_JIB, YARD_REACH)),
            planks: add(&models::plank_bundle()),
            container: add(&models::container()),
            elephant: add(&models::elephant()),
            giraffe: add(&models::giraffe()),
            zebra: add(&models::zebra()),
            lion: add(&models::lion(true)),
            lioness: add(&models::lion(false)),
            penguin: add(&models::penguin()),
            horse: add(&models::carousel_horse()),
            swim_ring: add(&models::swim_ring()),
            bucket: add(&models::splash_bucket()),
            slides: add(&models::water_slides()),
            slides_far: add(&models::water_slides_far()),
            track: add(&models::coaster_track(&world.traffic.coaster)),
            track_far: add(&models::coaster_track_far(&world.traffic.coaster)),
            race: add(&models::race_track()),
            body: people.0,
            heads: people.1,
            spray: add(&models::spray()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        traffic: &zawnabout_sim::traffic::Traffic,
        sim_time: f32,
        hour: f32,
        cam: Vec3,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let near = |f: &Mat4, r: f32| f.w_axis.truncate().distance(cam) < r;
        self.couriers(traffic, sim_time, cam, draws);
        // Clocks keep the game's time: an hour hand and a minute hand on every face.
        for &(at, n, len) in &self.clocks {
            if at.distance(cam) > 900.0 {
                continue;
            }
            let right = Vec3::Y.cross(n);
            for (turn, long, wide, out) in [
                ((hour % 12.0) / 12.0, len * 0.65, 0.16, 0.12),
                (hour.fract(), len, 0.11, 0.16),
            ] {
                let a = turn * TAU;
                let dir = Vec3::Y * a.cos() + right * a.sin();
                let m = Mat4::from_cols(
                    (dir.cross(n) * wide).extend(0.0),
                    (dir * long).extend(0.0),
                    n.extend(0.0),
                    (at + n * out).extend(1.0),
                );
                draws.push((self.hand, Instance::new(m)));
            }
        }
        // Helicopter rotors, always turning, blurred into a disc.
        for &i in &self.helicopters {
            let v = &traffic.vehicles[i];
            let p = traffic.pose(v, sim_time);
            if p.pos.distance(cam) > 3000.0 {
                continue;
            }
            let a = p.axis();
            let base = Mat4::from_cols(
                Vec4::new(a.x, 0.0, a.y, 0.0),
                Vec4::Y,
                Vec4::new(-a.y, 0.0, a.x, 0.0),
                p.pos.extend(1.0),
            ) * Mat4::from_rotation_z(p.pitch)
                * Mat4::from_rotation_x(p.bank);
            let spin = sim_time * 21.0;
            let m = base
                * Mat4::from_rotation_translation(
                    Quat::from_rotation_y(spin),
                    Vec3::new(0.3, 3.0, 0.0),
                );
            draws.push((self.rotor, Instance::new(m)));
            let tail = base
                * Mat4::from_translation(Vec3::new(-5.9, 2.4, 0.4))
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.18),
                    Quat::from_rotation_x(spin * 1.7) * Quat::from_rotation_z(FRAC_PI_2),
                    Vec3::ZERO,
                );
            draws.push((self.rotor, Instance::new(tail)));
        }
        // Riders in the coaster cars, two by two, fewer late at night.
        for &i in &self.coaster_cars {
            let v = &traffic.vehicles[i];
            let p = traffic.pose(v, sim_time);
            if p.pos.distance(cam) > 400.0 {
                continue;
            }
            let a = p.axis();
            let base = Mat4::from_cols(
                Vec4::new(a.x, 0.0, a.y, 0.0),
                Vec4::Y,
                Vec4::new(-a.y, 0.0, a.x, 0.0),
                p.pos.extend(1.0),
            ) * Mat4::from_rotation_z(p.pitch)
                * Mat4::from_rotation_x(p.bank);
            for seat in 0..4u32 {
                let s = i as u32 * 977 + seat;
                if hash(s) < if night > 0.75 { 0.8 } else { 0.25 } {
                    continue;
                }
                let x = if seat < 2 { 0.5 } else { -0.4 };
                let z = if seat % 2 == 0 { -0.3 } else { 0.3 };
                let m = base
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.85),
                        Quat::from_rotation_y(FRAC_PI_2),
                        Vec3::new(x, 0.35, z),
                    );
                self.person(m, s, draws);
            }
        }
        for (k, f) in self.zoos.iter().enumerate() {
            if near(f, ZOO_NEAR) {
                self.zoo(f, k as u32, time, night, draws);
            }
        }
        for f in &self.racetracks {
            if near(f, 2000.0) {
                draws.push((self.race, Instance::new(*f)));
            }
        }
        for (k, f) in self.parks.iter().enumerate() {
            // The coaster's track stands out over the trees from a long way off.
            if near(f, 400.0) {
                draws.push((self.track, Instance::new(*f)));
            } else if near(f, 1500.0) {
                draws.push((self.track_far, Instance::new(*f)));
            }
            if near(f, PARK_NEAR) {
                self.theme_park(f, k as u32, cam, time, sim_time, night, draws);
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
        // Stallholders behind their counters, and shoppers browsing along the lane. By
        // day up top, always underground.
        for (k, f) in self.markets.iter().enumerate() {
            let deep = self.deep_markets.get(k).copied().unwrap_or(false);
            if (night < 0.6 || deep) && near(f, ZOO_NEAR) {
                self.market(f, k as u32, time, draws);
            }
        }
        // People waiting for trains on the platforms, for buses at the stops, and at
        // picnics by day, with a kite up over most picnics and parks.
        for (k, (f, p)) in self.platforms.iter().enumerate() {
            if near(f, PEOPLE_NEAR) {
                self.platform(f, p, k as u32, time, night, draws);
            }
        }
        // A queue for the pictures in the evening, visitors at the museum and readers at
        // the library by day.
        for (k, f) in self.cinemas.iter().enumerate() {
            if near(f, PEOPLE_NEAR) {
                self.cinema(f, k as u32, time, night, draws);
            }
        }
        // Someone on duty at the police station door, someone filling up at the pumps.
        for f in &self.stations {
            if near(f, PEOPLE_NEAR) {
                let m =
                    *f * Mat4::from_rotation_translation(facing(Vec2::Y), Vec3::new(1.8, 0.0, 9.0));
                self.person(m, 901, draws);
            }
        }
        for (k, f) in self.pumps.iter().enumerate() {
            if near(f, PEOPLE_NEAR) {
                let side = if ((time / 40.0 + k as f32) as u32).is_multiple_of(2) {
                    -1.0
                } else {
                    1.0
                };
                let m = *f
                    * Mat4::from_rotation_translation(
                        facing(Vec2::new(-side, 0.0)),
                        Vec3::new(4.0 * side + side * 1.2, 0.1, 4.5),
                    );
                self.person(m, 700 + k as u32, draws);
            }
        }
        // A show at the open air theater from afternoon into the evening.
        for (k, f) in self.theaters.iter().enumerate() {
            if near(f, CROWD_NEAR) {
                self.theater(f, k as u32, time, draws);
            }
        }
        if night < 0.6 {
            // Visitors on the aquarium's forecourt, students crossing the quad.
            for (k, f) in self.aquariums.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.strollers(
                        f,
                        k as u32 + 40,
                        Vec2::new(0.0, 22.0),
                        Vec2::new(16.0, 8.0),
                        10,
                        time,
                        draws,
                    );
                }
            }
            for (k, f) in self.universities.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    let (c, half) = QUAD;
                    self.strollers(
                        f,
                        k as u32 + 80,
                        c,
                        half - Vec2::splat(1.0),
                        16,
                        time,
                        draws,
                    );
                }
            }
            for (k, f) in self.golf.iter().enumerate() {
                if near(f, CROWD_NEAR) {
                    self.golfers(f, k as u32, time, draws);
                }
            }
            for (k, f) in self.racetracks.iter().enumerate() {
                if near(f, CROWD_NEAR) {
                    self.grandstand(f, k as u32, time, draws);
                }
            }
            for (k, f) in self.museums.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.museum(f, k as u32, time, draws);
                }
            }
            for (k, f) in self.libraries.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.library(f, k as u32, time, draws);
                }
            }
        }
        if night < 0.6 {
            for (k, f) in self.bus_stops.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.bus_stop(f, k as u32, time, draws);
                }
            }
            for (k, (f, seed)) in self.picnics.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.picnic(f, *seed, k as u32, time, draws);
                }
            }
            for (k, f) in self.gardens.iter().enumerate() {
                if near(f, PEOPLE_NEAR) {
                    self.kite_flier(f, Vec3::new(-14.0, 0.0, -10.0), k as u32 + 500, time, draws);
                }
            }
        }
        // Fireworks over the harbors on the clock's nights.
        if night > 0.55 {
            for (k, at) in self.shows.iter().enumerate() {
                if at.distance(cam) < 6000.0 {
                    self.fireworks(*at, k as u32, time, night, fx);
                }
            }
        }
        // Underground: farmers tending mushroom beds, miners pushing ore carts, and people
        // at the cafes.
        for (k, f) in self.shroom_farms.iter().enumerate() {
            if near(f, ZOO_NEAR) {
                self.shroom_farm(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.mines.iter().enumerate() {
            if near(f, ZOO_NEAR) {
                self.mine(f, k as u32, cam, time, draws, fx);
            }
        }
        // Drawbridges lift their leaves as a boat comes up the strait, and lower them once
        // it has gone through. People stroll the decks and wait at the gap.
        for (k, f) in self.drawbridges.iter().enumerate() {
            let at = f.w_axis.truncate();
            if at.distance(cam) > 1500.0 {
                continue;
            }
            let lift = traffic.draw_lift(Vec2::new(at.x, at.z));
            for s in [-1.0f32, 1.0] {
                let m = *f
                    * Mat4::from_translation(Vec3::new(-s * DRAW_HINGE, DRAW_DECK, 0.0))
                    * Mat4::from_rotation_y(if s < 0.0 { PI } else { 0.0 })
                    * Mat4::from_rotation_z(lift);
                draws.push((self.leaf, Instance::new(m)));
            }
            if at.distance(cam) < PEOPLE_NEAR && night < 0.7 {
                for i in 0..8u32 {
                    let seed = (k as u32 * 64 + i).wrapping_mul(2_246_822_519);
                    let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let spot = move |n: u32| {
                        let r =
                            |q: u32| hash(seed ^ n.wrapping_mul(40_503) ^ q.wrapping_mul(7_919));
                        Vec2::new(side * (DRAW_HINGE + 2.0 + r(1) * 80.0), (r(2) - 0.5) * 3.0)
                    };
                    let r = roam(spot, sim_time + hash(seed) * 30.0, 22.0, 1.2);
                    let m = *f
                        * Mat4::from_rotation_translation(
                            facing(r.face),
                            Vec3::new(r.at.x, DRAW_DECK, r.at.y),
                        );
                    self.person(m, seed, draws);
                }
            }
        }
        // Speedboats tow a water skier, swinging out behind on the rope, or a parasail
        // riding high on its line. Every third one just zips about.
        for v in &traffic.vehicles {
            if v.kind != Kind::Speedboat || v.variant % 3 == 2 {
                continue;
            }
            let p = traffic.pose(v, sim_time);
            if p.pos.distance(cam) > 1200.0 {
                continue;
            }
            let fwd = Vec3::new(p.heading.sin(), 0.0, p.heading.cos());
            let side = Vec3::new(fwd.z, 0.0, -fwd.x);
            let stern = p.pos - fwd * v.size.x * 0.5 + Vec3::Y * 0.9;
            let seed = (v.path as u32 * 31 + v.variant as u32).wrapping_mul(2_654_435_761);
            let face = Quat::from_rotation_y(p.heading);
            let (hand, end) = if v.variant % 3 == 0 {
                // The skier carves from side to side across the wake, leaning back.
                let swing = (sim_time * 0.6 + hash(seed) * 6.0).sin();
                let at = p.pos - fwd * 20.0 + side * swing * 7.0;
                let at = Vec3::new(at.x, p.pos.y, at.z);
                let lean = Quat::from_rotation_y(p.heading - swing * 0.25)
                    * Quat::from_rotation_x(-0.35)
                    * Quat::from_rotation_z(-swing * 0.3);
                draws.push((
                    self.water_skis,
                    Instance::new(Mat4::from_rotation_translation(face, at)),
                ));
                let body = Mat4::from_rotation_translation(lean, at + Vec3::Y * 0.1);
                self.person(body, seed, draws);
                let hand = (body * Mat4::from_translation(Vec3::new(0.0, 1.1, 0.5)))
                    .w_axis
                    .truncate();
                (hand, stern)
            } else {
                // The flier hangs in a harness under the canopy, high astern.
                let sway = (sim_time * 0.4 + hash(seed) * 6.0).sin();
                let at = p.pos - fwd * 48.0 + side * sway * 3.0 + Vec3::Y * 30.0;
                let toward = (stern - at).normalize_or(Vec3::NEG_Y);
                let tilt = Quat::from_rotation_arc(Vec3::NEG_Y, toward).slerp(Quat::IDENTITY, 0.7);
                draws.push((
                    self.parasail,
                    Instance::new(Mat4::from_rotation_translation(tilt * face, at)),
                ));
                self.person(
                    Mat4::from_rotation_translation(face, at - Vec3::Y * 1.6),
                    seed,
                    draws,
                );
                (at, stern)
            };
            let len = hand.distance(end);
            draws.push((
                self.line,
                Instance::new(Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, len, 1.0),
                    Quat::from_rotation_arc(Vec3::Y, (end - hand) / len.max(0.01)),
                    hand,
                )),
            ));
        }
        // Skaters ride back and forth over the boxes, popping up as they go over.
        for (k, f) in self.skateparks.iter().enumerate() {
            if !near(f, PEOPLE_NEAR) || night > 0.7 {
                continue;
            }
            for (i, (a, b)) in SKATE_LINES.into_iter().enumerate() {
                let s = (k as u32 * 16 + i as u32).wrapping_mul(2_654_435_761);
                let len = a.distance(b);
                let pace = 4.5 + hash(s) * 2.0;
                let u = (time * pace / len + hash(s + 1) * 2.0) % 2.0;
                let (t, dir) = if u < 1.0 {
                    (u, b - a)
                } else {
                    (2.0 - u, a - b)
                };
                let t = t * t * (3.0 - 2.0 * t);
                let at = a.lerp(b, t);
                let mut y = 0.2;
                for (bx, h) in SKATE_BOXES {
                    let d = at.distance(bx);
                    if d < 3.0 {
                        y += h * (1.0 - (d / 3.0).powi(2)) + 0.3 * (1.0 - d / 3.0);
                    }
                }
                let face = facing(dir);
                let board = *f * Mat4::from_rotation_translation(face, Vec3::new(at.x, y, at.y));
                draws.push((self.board, Instance::new(board)));
                // Riding side on, arms a little out.
                let m = board
                    * Mat4::from_rotation_translation(
                        Quat::from_rotation_y(FRAC_PI_2),
                        Vec3::Y * 0.12,
                    );
                self.person(m, s, draws);
            }
            // Friends on the benches watching.
            for (j, x) in [-8.0f32, 8.0].into_iter().enumerate() {
                let m = *f
                    * Mat4::from_rotation_translation(
                        Quat::from_rotation_y(PI),
                        Vec3::new(x + 0.4, -0.3, zawnabout_sim::landmark::SKATE_PAD.y + 2.0),
                    );
                self.person(m, k as u32 * 7 + j as u32 + 300, draws);
            }
        }
        for (k, f) in self.restaurants.iter().enumerate() {
            if near(f, ZOO_NEAR) && night < 0.8 {
                self.restaurant(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.cafes.iter().enumerate() {
            if near(f, ZOO_NEAR) {
                self.cafe(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.schools.iter().enumerate() {
            if near(f, ZOO_NEAR) && night < 0.5 {
                self.school(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.seal_colonies.iter().enumerate() {
            if near(f, PARK_NEAR) {
                self.seals(f, k as u32, time, draws);
            }
        }
        for (k, f) in self.carnivals.iter().enumerate() {
            if near(f, PARK_NEAR) {
                self.carnival(f, k as u32, time, draws);
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
        for (k, c) in self.yard_cranes.iter().enumerate() {
            if near(&c.frame, YARD_NEAR) {
                self.yard_crane(c, k as u32, time, draws);
            }
        }
        for (k, f) in self.pools.iter().enumerate() {
            if near(f, 400.0) {
                draws.push((self.slides, Instance::new(*f)));
            } else if near(f, SLIDES_NEAR) {
                draws.push((self.slides_far, Instance::new(*f)));
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
            let path = zawnabout_sim::landmark::zoo_path();
            let n = path.len();
            let lens: Vec<f32> = (0..n)
                .map(|j| path[j].distance(path[(j + 1) % n]))
                .collect();
            let total: f32 = lens.iter().sum();
            for i in 0..16u32 {
                let s = z * 331 + i;
                let dir = if hash(s) < 0.5 { 1.0 } else { -1.0 };
                let mut d = (hash(s + 1) * total + dir * time * 1.1
                    - dir * ((time * 0.05 + hash(s + 2) * 9.0).sin().max(0.0) * 100.0))
                    .rem_euclid(total);
                let mut j = 0;
                while d > lens[j] {
                    d -= lens[j];
                    j = (j + 1) % n;
                }
                let (a, b) = (path[j], path[(j + 1) % n]);
                let along = (b - a).normalize_or(Vec2::X);
                let side = Vec2::new(-along.y, along.x) * (hash(s + 3) - 0.5) * 2.4;
                let p = a.lerp(b, d / lens[j]) + side;
                let at = Vec3::new(p.x, 0.14, p.y);
                let tangent = along * dir;
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

    #[allow(clippy::too_many_arguments)]
    fn theme_park(
        &self,
        f: &Mat4,
        k: u32,
        cam: Vec3,
        time: f32,
        sim_time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        let open = night < 0.75;
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
        // The drop tower's ring rides the sim's clock, as a vehicle you can land on.
        let cycle = zawnabout_sim::landmark::DROP_CYCLE;
        let y = zawnabout_sim::landmark::drop_height(sim_time + k as f32 * 11.0);
        let ring = *f * Mat4::from_translation(DROP + Vec3::Y * y);
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

    /// People wait along a platform facing the track, some pacing a little, fewer late
    /// at night.
    fn platform(
        &self,
        f: &Mat4,
        p: &Platform,
        k: u32,
        time: f32,
        night: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        let count = if night > 0.6 { 5 } else { 14 };
        for i in 0..count {
            let seed = (k * 64 + i).wrapping_mul(2_654_435_761);
            // Each waits a while, then a new one is there in their place.
            let slot = ((time / 75.0 + hash(seed) * 9.0) as u32).wrapping_mul(40_503);
            let s = seed ^ slot;
            if hash(s) < 0.25 {
                continue;
            }
            let x = (hash(s + 1) * 2.0 - 1.0) * p.half;
            let z = p.near + hash(s + 2) * (p.far - p.near);
            let pace =
                (time * 0.3 + hash(s + 3) * 9.0).sin() * 1.2 * (hash(s + 4) < 0.3) as u32 as f32;
            let m = *f
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.92 + hash(s + 5) * 0.14),
                    facing(Vec2::new((hash(s + 6) - 0.5) * 0.8, -1.0)),
                    Vec3::new(x + pace, p.y, z),
                );
            self.person(m, s, draws);
        }
    }

    /// Evenings, a queue for the pictures shuffles toward the doors; by day someone or
    /// other looks over the posters.
    fn cinema(&self, f: &Mat4, k: u32, time: f32, night: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let (start, dir) = CINEMA_QUEUE;
        let count = if night > 0.3 { 9 } else { 2 };
        // The whole line steps forward now and then as the doors take someone in.
        let shuffle = ((time * 0.25 + k as f32).fract() * 4.0).min(1.0) * 1.1;
        for i in 0..count {
            let s = (k * 32 + i + ((time / 4.0) as u32 / 9) * 7).wrapping_mul(2_654_435_761);
            let at = start
                + dir * (i as f32 * 1.1 + 1.1 - shuffle)
                + Vec2::new(0.0, (hash(s) - 0.5) * 0.4);
            let face = if night > 0.3 {
                -dir
            } else {
                Vec2::new(0.0, -1.0)
            };
            let m = *f
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.92 + hash(s + 1) * 0.14),
                    facing(face),
                    Vec3::new(at.x, 0.0, at.y),
                );
            self.person(m, s, draws);
        }
    }

    /// An audience round the tiers and players on the stage, moving about as they act.
    fn theater(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let (inner, outer, rise) = AMPHI_SEATS;
        let rows = ((outer - inner) / 1.8) as u32;
        for i in 0..50u32 {
            let s = (k * 128 + i).wrapping_mul(2_654_435_761);
            let row = (hash(s) * rows as f32) as u32 % rows;
            let rad = inner + row as f32 * 1.8;
            let a = 0.15 + hash(s + 1) * (PI - 0.3);
            let at = Vec3::new(a.cos() * rad, rise * (row + 1) as f32 - 0.8, a.sin() * rad);
            let to = Vec2::new(STAGE.x - at.x, STAGE.z - at.z);
            let m = *f * Mat4::from_rotation_translation(facing(to), at);
            self.person(m, s, draws);
        }
        for i in 0..3u32 {
            let s = (k * 16 + i + 900).wrapping_mul(2_246_822_519);
            let x = (time * 0.3 + i as f32 * 2.1).sin() * 5.0;
            let z = (time * 0.23 + i as f32).cos() * 2.0;
            let m = *f
                * Mat4::from_rotation_translation(
                    facing(Vec2::new((time * 0.5 + i as f32).sin(), 1.0)),
                    STAGE + Vec3::new(x, 0.0, z),
                );
            self.person(m, s, draws);
        }
    }

    /// Golfers play their way round the holes in twos and threes: off the tee, walking
    /// up the fairway, putting out on the green.
    fn golfers(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let round = 240.0;
        for g in 0..4u32 {
            // Each group a hole ahead of the last, all the way round.
            let t = time / round + g as f32 / 4.0 + k as f32 * 0.3;
            let hole = (t.floor() as i64 as u32 + g) as usize % GOLF_HOLES.len();
            let u = t.fract();
            let (tee, green) = GOLF_HOLES[hole];
            let walk = ((u - 0.15) / 0.7).clamp(0.0, 1.0);
            let walk = walk * walk * (3.0 - 2.0 * walk);
            let at = tee.lerp(green, walk);
            let dir = (green - tee).normalize_or(Vec2::Y);
            let side = Vec2::new(-dir.y, dir.x);
            for p in 0..3u32 {
                let s = (k * 64 + g * 4 + p).wrapping_mul(2_654_435_761);
                if p == 2 && hash(s) < 0.5 {
                    continue;
                }
                let spot = at + side * (p as f32 - 1.0) * 2.5 - dir * (p as f32 * 1.2);
                // Up on the raised green once they reach it.
                let y = if spot.distance(green) < 15.0 {
                    0.55
                } else {
                    0.1
                };
                let m =
                    *f * Mat4::from_rotation_translation(facing(dir), Vec3::new(spot.x, y, spot.y));
                self.person(m, s, draws);
            }
        }
    }

    /// People wandering about a square `half` either side of `c` (in the frame `f`),
    /// stopping now and then to look about or chat.
    #[allow(clippy::too_many_arguments)]
    fn strollers(
        &self,
        f: &Mat4,
        k: u32,
        c: Vec2,
        half: Vec2,
        count: u32,
        time: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        for i in 0..count {
            let seed = (k * 64 + i).wrapping_mul(2_246_822_519);
            let spot = move |n: u32| {
                let r = |q: u32| hash(seed ^ n.wrapping_mul(40_503) ^ q.wrapping_mul(7_919));
                c + Vec2::new((r(1) * 2.0 - 1.0) * half.x, (r(2) * 2.0 - 1.0) * half.y)
            };
            let r = roam(spot, time + hash(seed) * 25.0, 16.0, 1.1);
            let m = *f
                * Mat4::from_rotation_translation(facing(r.face), Vec3::new(r.at.x, 0.1, r.at.y));
            self.person(m, seed, draws);
        }
    }

    /// A crowd up the grandstand tiers watching the race, bobbing and cheering.
    fn grandstand(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let (_, r, hw) = RACE_OVAL;
        let back = -r - hw - 4.0;
        for i in 0..60u32 {
            let s = (k * 128 + i).wrapping_mul(2_654_435_761);
            let tier = (hash(s) * 5.0) as u32 % 5;
            let x = (hash(s + 1) * 2.0 - 1.0) * 66.0;
            let z = back - tier as f32 * 2.4;
            let y = 1.2 + tier as f32 * 1.4;
            let cheer = ((time * 3.0 + hash(s + 2) * 20.0).sin().max(0.0)).powi(4) * 0.25;
            let m =
                *f * Mat4::from_rotation_translation(facing(Vec2::Y), Vec3::new(x, y + cheer, z));
            self.person(m, s, draws);
        }
    }

    /// People sit about the museum's front steps in the sun, and others stroll the
    /// square before it, stopping by the sculpture.
    fn museum(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let (half, front, rise) = MUSEUM_STEPS;
        for i in 0..10u32 {
            let s = (k * 64 + i).wrapping_mul(2_654_435_761)
                ^ ((time / 90.0 + i as f32 * 0.3) as u32).wrapping_mul(40_503);
            if hash(s) < 0.3 {
                continue;
            }
            // On one of the four steps, part way along.
            let step = (hash(s + 1) * 4.0) as u32 % 4;
            let x = (hash(s + 2) * 2.0 - 1.0) * (half - 1.0);
            let z = front + step as f32 * rise;
            let y = 2.4 - step as f32 * 0.6;
            let m = *f * Mat4::from_rotation_translation(facing(Vec2::Y), Vec3::new(x, y - 0.8, z));
            self.person(m, s, draws);
        }
        for i in 0..8u32 {
            let seed = (k * 32 + i + 300).wrapping_mul(2_246_822_519);
            let spot = move |n: u32| {
                let r = |q: u32| hash(seed ^ n.wrapping_mul(40_503) ^ q);
                Vec2::new((r(1) - 0.5) * 30.0, front + 4.0 + r(2) * 16.0)
            };
            let r = roam(spot, time + hash(seed) * 30.0, 20.0, 1.0);
            let m = *f
                * Mat4::from_rotation_translation(facing(r.face), Vec3::new(r.at.x, 0.1, r.at.y));
            self.person(m, seed, draws);
        }
    }

    /// Readers on the library's benches out front, and people going in and out.
    fn library(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for (b, bench) in LIBRARY_BENCHES.into_iter().enumerate() {
            for seat in [-0.6f32, 0.6] {
                let s = (k * 16 + b as u32 * 2 + (seat > 0.0) as u32).wrapping_mul(2_654_435_761);
                if hash(s ^ ((time / 100.0 + hash(s) * 3.0) as u32).wrapping_mul(40_503)) < 0.5 {
                    continue;
                }
                let m = *f
                    * Mat4::from_rotation_translation(
                        facing(Vec2::Y),
                        bench + Vec3::new(seat, -0.3, 0.0),
                    );
                self.person(m, s, draws);
            }
        }
        for i in 0..3u32 {
            let s = (k * 8 + i + 77).wrapping_mul(2_246_822_519);
            let u = ((time * 0.9 / 16.0 + hash(s)) % 2.0 - 1.0).abs();
            let at = Vec2::new((hash(s + 1) - 0.5) * 6.0, 11.0 + u * 14.0);
            let going = ((time * 0.9 / 16.0 + hash(s)) % 2.0) < 1.0;
            let m = *f
                * Mat4::from_rotation_translation(
                    facing(Vec2::new(0.0, if going { 1.0 } else { -1.0 })),
                    Vec3::new(at.x, 0.1, at.y),
                );
            self.person(m, s, draws);
        }
    }

    /// One to three people wait at a bus stop, on the bench or standing by the curb.
    fn bus_stop(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for i in 0..3u32 {
            let seed = (k * 8 + i).wrapping_mul(2_654_435_761);
            let s = seed ^ ((time / 60.0 + hash(seed) * 5.0) as u32).wrapping_mul(40_503);
            if hash(s) < 0.45 {
                continue;
            }
            let sit = i < 2;
            // Seated low on the bench, legs over its edge, or standing by the curb.
            let at = if sit {
                BUS_BENCH + Vec3::new((i as f32 - 0.5) * 1.6, -0.8, 0.0)
            } else {
                Vec3::new(1.4, 0.0, 0.6)
            };
            let m = *f * Mat4::from_rotation_translation(facing(Vec2::Y), at);
            self.person(m, s, draws);
        }
    }

    /// A family or friends round the picnic tables, now and then, one of them flying a
    /// kite.
    fn picnic(&self, f: &Mat4, seed: u32, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        self.kite_flier(f, Vec3::new(8.0, 0.0, 6.0), k, time, draws);
        let slot = ((time / 120.0 + hash(seed) * 3.0) as u32).wrapping_mul(40_503);
        if hash(seed ^ slot) < 0.15 {
            return;
        }
        for (t, table) in picnic_tables(seed).into_iter().enumerate() {
            for (j, side) in [-1.0f32, 1.0].into_iter().enumerate() {
                for x in [-0.55f32, 0.55] {
                    let s = (k * 32 + t as u32 * 8 + j as u32 * 2 + (x > 0.0) as u32)
                        .wrapping_mul(2_246_822_519)
                        ^ slot;
                    if hash(s) < 0.4 {
                        continue;
                    }
                    let m = *f
                        * Mat4::from_rotation_translation(
                            facing(Vec2::new(0.0, -side)),
                            table + Vec3::new(x, -0.35, side * 0.85),
                        );
                    self.person(m, s, draws);
                }
            }
        }
    }

    /// Someone standing holding a kite up on the breeze, high over them, its string down
    /// to their hand.
    fn kite_flier(
        &self,
        f: &Mat4,
        at: Vec3,
        k: u32,
        time: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        let seed = k.wrapping_mul(2_654_435_761);
        let person = *f * Mat4::from_rotation_translation(facing(Vec2::new(0.3, 1.0)), at);
        self.person(person, seed, draws);
        let hand = (person * Mat4::from_translation(Vec3::new(0.25, 1.3, 0.3)))
            .w_axis
            .truncate();
        // Up and downwind, drifting about on the gusts.
        let sway = Vec3::new(
            (time * 0.37 + k as f32).sin() * 3.0,
            (time * 0.53 + k as f32).sin() * 2.0,
            (time * 0.29 + k as f32).cos() * 2.0,
        );
        let kite = hand + Vec3::new(9.0, 22.0, 6.0) + sway;
        let toward = (hand - kite).normalize_or(Vec3::NEG_Y);
        let face = Quat::from_rotation_arc(
            Vec3::Z,
            Vec3::new(-toward.x, 0.0, -toward.z).normalize_or(Vec3::Z),
        ) * Quat::from_rotation_z((time * 1.3 + k as f32).sin() * 0.3);
        let paint = linear(SHIRTS[(hash(seed + 3) * 8.0) as usize % 8]);
        draws.push((
            self.kite,
            Instance::tinted(
                Mat4::from_rotation_translation(face, kite),
                paint.extend(1.0),
            ),
        ));
        let len = kite.distance(hand);
        let string = Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, len, 1.0),
            Quat::from_rotation_arc(Vec3::Y, (kite - hand) / len.max(0.01)),
            hand,
        );
        draws.push((self.line, Instance::new(string)));
    }

    /// A fireworks show over a harbor: rockets climbing on a trail of sparks, then
    /// bursting into falling, fading stars of color, one after another.
    fn fireworks(&self, at: Vec3, k: u32, time: f32, night: f32, fx: &mut Vec<(MeshId, Instance)>) {
        const COLORS: [Vec3; 6] = [
            Vec3::new(1.0, 0.35, 0.3),
            Vec3::new(0.35, 0.75, 1.0),
            Vec3::new(1.0, 0.85, 0.3),
            Vec3::new(0.5, 1.0, 0.5),
            Vec3::new(1.0, 0.45, 0.9),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        let (rise, burn) = (1.6, 2.8);
        let cycle = rise + burn;
        let strength = ((night - 0.55) / 0.2).clamp(0.0, 1.0);
        for i in 0..5u32 {
            let t = time + i as f32 * cycle / 5.0 + k as f32 * 3.1;
            let n = (t / cycle).floor() as i64 as u32;
            let age = t - (t / cycle).floor() * cycle;
            let s = (k * 97 + i * 13).wrapping_add(n.wrapping_mul(2_654_435_761));
            let top = at
                + Vec3::new(
                    (hash(s) - 0.5) * 120.0,
                    130.0 + hash(s + 1) * 80.0,
                    (hash(s + 2) - 0.5) * 120.0,
                );
            let color = COLORS[(hash(s + 3) * 6.0) as usize % 6];
            let mut glow = |p: Vec3, size: f32, c: Vec3, a: f32| {
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(size, 1.0, size),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    p,
                );
                fx.push((
                    self.glow,
                    Instance::tinted(m, (c * 2.0).extend(a * strength)),
                ));
            };
            if age < rise {
                let u = age / rise;
                let p = at.lerp(top, u * (2.0 - u));
                glow(p, 2.4, Vec3::new(1.0, 0.8, 0.5), 0.9);
                continue;
            }
            let a = age - rise;
            let fade = 1.0 - a / burn;
            let sparks = 26;
            for j in 0..sparks {
                // Spread evenly round a sphere, each falling as it slows.
                let y = 1.0 - 2.0 * (j as f32 + 0.5) / sparks as f32;
                let r = (1.0 - y * y).sqrt();
                let phi = j as f32 * 2.399_963 + hash(s + 4) * TAU;
                let dir = Vec3::new(phi.cos() * r, y, phi.sin() * r);
                let out = 34.0 * (1.0 - (-a * 1.6).exp()) / 1.6;
                let p = top + dir * out - Vec3::Y * 3.0 * a * a;
                glow(p, 4.5 * (0.6 + fade * 0.6), color, fade.sqrt());
            }
        }
    }

    /// Farmers walk the rows of a mushroom farm, stopping to crouch and pick.
    fn shroom_farm(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        for i in 0..5u32 {
            let seed = (k * 32 + i).wrapping_mul(2_654_435_761);
            let spot = move |n: u32| {
                let bed = SHROOM_BEDS[(hash(seed ^ n.wrapping_mul(40_503)) * 6.0) as usize % 6];
                let side = if hash(seed ^ n.wrapping_mul(7_919)) < 0.5 {
                    -1.3
                } else {
                    1.3
                };
                let x = (hash(seed ^ n) - 0.5) * (SHROOM_BED_LEN - 2.0);
                Vec2::new(x, bed + side)
            };
            let r = roam(spot, time + hash(seed) * 20.0, 14.0, 0.9);
            // Bent over the bed while picking, facing it.
            let (face, stoop) = if r.walking {
                (r.face, 1.0)
            } else {
                let to_bed = if (r.at.y.rem_euclid(3.6) - 1.8).abs() < 0.9 {
                    1.0
                } else {
                    -1.0
                };
                (Vec2::new(0.0, to_bed), 0.72)
            };
            let m = *f
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, stoop, 1.0) * (0.9 + hash(seed + 1) * 0.15),
                    facing(face),
                    Vec3::new(r.at.x, 0.1, r.at.y),
                );
            self.person(m, seed, draws);
        }
    }

    /// A miner pushes the ore cart out of the adit to the tip, tips it, and pushes it back
    /// empty to load again, a headlamp lit.
    fn mine(
        &self,
        f: &Mat4,
        k: u32,
        cam: Vec3,
        time: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let (from, to) = (MINE_TRACK.0 + 1.5, MINE_TRACK.1 - 1.5);
        let cycle = 50.0;
        let u = (time + k as f32 * 13.0).rem_euclid(cycle) / cycle;
        // Loading, out, tipping, back.
        let (z, out, tip) = match u {
            u if u < 0.2 => (from, true, 0.0),
            u if u < 0.5 => (from + (to - from) * smooth01((u - 0.2) / 0.3), true, 0.0),
            u if u < 0.7 => (to, true, ((u - 0.5) / 0.2 * PI).sin() * 0.7),
            u => (to - (to - from) * smooth01((u - 0.7) / 0.3), false, 0.0),
        };
        let cart = *f
            * Mat4::from_rotation_translation(
                Quat::from_rotation_y(-PI * 0.5) * Quat::from_rotation_z(tip),
                Vec3::new(0.0, 0.12, z),
            );
        draws.push((self.ore_cart, Instance::new(cart)));
        // The miner walks behind the cart, pushing it the way it goes.
        let way = if out { 1.0 } else { -1.0 };
        let seed = k.wrapping_mul(2_246_822_519);
        let m = *f
            * Mat4::from_rotation_translation(
                facing(Vec2::new(0.0, way)),
                Vec3::new(0.0, 0.1, z - way * 1.6),
            );
        self.person(m, seed, draws);
        let lamp = (m * Mat4::from_translation(Vec3::new(0.0, 1.72, 0.2)))
            .w_axis
            .truncate();
        let face = Quat::from_rotation_arc(Vec3::Y, (cam - lamp).normalize_or(Vec3::Y));
        let g = Mat4::from_scale_rotation_translation(Vec3::new(0.3, 1.0, 0.3), face, lamp);
        fx.push((
            self.glow,
            Instance::tinted(g, Vec4::new(1.0, 0.95, 0.8, 1.0)),
        ));
        // A second miner chips at the rock by the adit.
        let chip = (time * 2.2 + k as f32).sin() * 0.15;
        let m = *f
            * Mat4::from_rotation_translation(
                facing(Vec2::new(0.6, -1.0)),
                Vec3::new(-3.6, 0.1, MINE_TRACK.0 + 1.2 + chip),
            );
        self.person(m, seed.wrapping_add(5), draws);
    }

    /// People sit at the cafe's tables a while, then the seats are empty a while; someone
    /// minds the counter.
    fn cafe(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        self.diners(f, &CAFE_TABLES, k, time, draws);
        let m = *f * Mat4::from_rotation_translation(facing(Vec2::Y), Vec3::new(0.0, 0.0, -2.8));
        self.person(m, k.wrapping_mul(97) + 3, draws);
    }

    /// Up the mountain: diners out on the deck, a waiter going between the tables, and
    /// someone at the telescope looking out at the view.
    fn restaurant(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        self.diners(f, &RESTAURANT_TABLES, k + 500, time, draws);
        let seed = k.wrapping_mul(2_246_822_519) ^ 77;
        let spot = move |n: u32| {
            let at = RESTAURANT_TABLES[(hash(seed ^ n.wrapping_mul(40_503)) * 6.0) as usize % 6];
            at + Vec2::new(1.6, -0.8)
        };
        let r = roam(spot, time, 9.0, 1.3);
        let m =
            *f * Mat4::from_rotation_translation(facing(r.face), Vec3::new(r.at.x, 0.0, r.at.y));
        self.person(m, seed, draws);
        let m = *f
            * Mat4::from_rotation_translation(
                facing(Vec2::new(0.5, 1.0)),
                Vec3::new(11.0, 0.0, 18.3),
            );
        self.person(m, seed + 9, draws);
    }

    /// People sat round `tables` a while, then the seats empty a while.
    fn diners(
        &self,
        f: &Mat4,
        tables: &[Vec2],
        k: u32,
        time: f32,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        for (t, &at) in tables.iter().enumerate() {
            for seat in 0..4u32 {
                let s = (k * 64 + t as u32 * 4 + seat).wrapping_mul(2_654_435_761);
                if hash(s ^ ((time / 70.0 + hash(s) * 4.0) as u32).wrapping_mul(40_503)) < 0.4 {
                    continue;
                }
                let a = seat as f32 * PI * 0.5 + 0.4;
                let p = at + Vec2::new(a.cos(), a.sin());
                let lean = (time * 0.8 + s as f32).sin() * 0.04;
                let m = *f
                    * Mat4::from_rotation_translation(
                        facing(at - p) * Quat::from_rotation_x(lean),
                        Vec3::new(p.x, -0.2, p.y),
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
        // Cows and sheep grazing the pasture, wandering a few steps now and then.
        let (mid, half) = PASTURE;
        for i in 0..9u32 {
            let seed = (k * 64 + i).wrapping_mul(2_246_822_519);
            let spot = move |n: u32| {
                let r = |q: u32| hash(seed ^ n.wrapping_mul(40_503) ^ q.wrapping_mul(7_919));
                mid + Vec2::new((r(1) * 2.0 - 1.0) * half.x, (r(2) * 2.0 - 1.0) * half.y)
            };
            let r = roam(spot, time + hash(seed) * 40.0, 25.0, 0.8);
            let mesh = if i % 3 == 0 { self.cow } else { self.sheep };
            let m = *f
                * Mat4::from_rotation_translation(facing(r.face), Vec3::new(r.at.x, 0.0, r.at.y));
            draws.push((mesh, Instance::new(m)));
        }
        // Riders on horseback round the bridle path, at a walk, one now and then at a
        // trot.
        let corners = FARM_TRAIL;
        let lens: Vec<f32> = (0..4)
            .map(|c| corners[c].distance(corners[(c + 1) % 4]))
            .collect();
        let lap: f32 = lens.iter().sum();
        for i in 0..5u32 {
            let seed = (k * 16 + i).wrapping_mul(2_654_435_761);
            let pace = if i == 2 { 3.2 } else { 1.6 };
            let mut u = (time * pace + i as f32 / 5.0 * lap).rem_euclid(lap);
            let mut c = 0;
            while u > lens[c] {
                u -= lens[c];
                c += 1;
            }
            let (a, b) = (corners[c], corners[(c + 1) % 4]);
            let dir = (b - a).normalize_or(Vec2::X);
            let at = a + dir * u + Vec2::new(-dir.y, dir.x) * 0.6;
            let gait = (time * pace * 2.4 + i as f32).sin().abs() * 0.08 * pace;
            let m = *f * Mat4::from_rotation_translation(facing(dir), Vec3::new(at.x, gait, at.y));
            draws.push((self.mount, Instance::new(m)));
            self.person(
                m * Mat4::from_translation(Vec3::new(0.0, 1.05, -0.1)),
                seed,
                draws,
            );
        }
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

    /// A shipyard crane hoists a bundle of planks off its pile, swings round over the hull,
    /// lowers it onto the deck, and swings back empty for the next.
    fn yard_crane(&self, c: &YardCrane, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        let cycle = 52.0;
        let t = (time + k as f32 * 23.0).rem_euclid(cycle);
        let ease = |u: f32| {
            let u = u.clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u)
        };
        let top = c.height - 3.0;
        // The bundle on the hook sits on the pile at the pick, on the deck at the drop.
        let (pick, drop) = (c.pick_y + 1.0, c.drop_y + 0.5);
        let (turn, low, carrying) = match t {
            t if t < 4.0 => (c.pick, pick + (top - pick) * ease(t / 4.0), true),
            t if t < 18.0 => (
                c.pick + (c.drop - c.pick) * ease((t - 4.0) / 14.0),
                top,
                true,
            ),
            t if t < 23.0 => (c.drop, top - (top - drop) * ease((t - 18.0) / 5.0), true),
            t if t < 26.0 => (c.drop, drop, true),
            t if t < 30.0 => (c.drop, drop + (top - drop) * ease((t - 26.0) / 4.0), false),
            t if t < 44.0 => (
                c.drop + (c.pick - c.drop) * ease((t - 30.0) / 14.0),
                top,
                false,
            ),
            t if t < 49.0 => (c.pick, top - (top - pick) * ease((t - 44.0) / 5.0), false),
            _ => (c.pick, pick, true),
        };
        let spin = Quat::from_rotation_y(turn);
        let head = c.frame * Mat4::from_translation(Vec3::Y * c.height);
        draws.push((self.yard_jib, Instance::new(head * Mat4::from_quat(spin))));
        let out = spin * Vec3::Z * YARD_REACH;
        let hang = c.height - 0.4 - (low + 1.2);
        draws.push((
            self.cables,
            Instance::new(
                c.frame
                    * Mat4::from_scale_rotation_translation(
                        Vec3::new(0.4, hang.max(0.1), 1.0),
                        spin,
                        out + Vec3::Y * (c.height - 0.4),
                    ),
            ),
        ));
        let hook = Vec3::new(out.x, low + 1.2, out.z);
        draws.push((
            self.trolley,
            Instance::new(
                c.frame * Mat4::from_scale_rotation_translation(Vec3::splat(0.3), spin, hook),
            ),
        ));
        if carrying {
            let m = Mat4::from_rotation_translation(spin, Vec3::new(out.x, low, out.z));
            draws.push((self.planks, Instance::new(c.frame * m)));
        }
        // The pile it takes from, and the bundles already on the deck.
        let at = |turn: f32, y: f32| {
            let q = Quat::from_rotation_y(turn);
            c.frame * Mat4::from_rotation_translation(q, q * Vec3::Z * YARD_REACH + Vec3::Y * y)
        };
        for y in [c.pick_y, c.pick_y + 0.5] {
            draws.push((self.planks, Instance::new(at(c.pick, y))));
        }
        draws.push((self.planks, Instance::new(at(c.drop, c.drop_y))));
    }

    /// The splash playground: the big bucket fills and tips over the deck, a rush of
    /// water drops onto the children under it, and they run about the pad between soakings.
    fn splash(
        &self,
        f: &Mat4,
        k: u32,
        time: f32,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        use zawnabout_sim::landmark::{SPLASH, SPLASH_DECK, SPLASH_FILL, splash_tip};
        let (tip, pouring) = splash_tip(time);
        let bucket = *f * Mat4::from_rotation_translation(Quat::from_rotation_x(tip), SPLASH);
        draws.push((self.bucket, Instance::new(bucket)));
        // The bucket's mouth, where the water leaves it.
        let lip = bucket.transform_point3(Vec3::new(0.0, 0.75, 0.0));
        let floor = f.transform_point3(Vec3::new(SPLASH.x, SPLASH_DECK, SPLASH.z + 0.8));
        if pouring {
            let t = (time.rem_euclid(SPLASH_FILL + 4.0) - SPLASH_FILL - 0.8) / 1.8;
            for j in 0..10u32 {
                // Parcels of water falling from the lip to the deck, spreading as they go.
                let u = (time * 2.2 + j as f32 / 10.0).fract();
                let at = lip.lerp(floor, u * u);
                let spread = 0.45 + u * 0.9;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(spread, 1.4, spread),
                    Quat::IDENTITY,
                    at + Vec3::new(hash(j) - 0.5, 0.0, hash(j + 7) - 0.5) * u * 1.5,
                );
                fx.push((
                    self.spray,
                    Instance::tinted(m, Vec4::new(0.75, 0.9, 1.0, 0.75 * (1.0 - t * 0.5))),
                ));
            }
            for j in 0..6u32 {
                let a = j as f32 / 6.0 * TAU + time * 3.0;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.9),
                    Quat::IDENTITY,
                    floor + Vec3::new(a.cos() * 2.0, 0.3, a.sin() * 2.0),
                );
                fx.push((
                    self.spray,
                    Instance::tinted(m, Vec4::new(1.0, 1.0, 1.0, 0.6)),
                ));
            }
        }
        // Children: some wait on the deck under the bucket, looking up, the rest run
        // about the pad and back.
        for i in 0..9u32 {
            let s = k * 677 + i;
            let small = 0.55 + hash(s) * 0.15;
            let (at, face) = if i < 4 {
                let a = i as f32 * 1.7;
                let p = Vec3::new(
                    SPLASH.x + a.cos() * 1.6,
                    SPLASH_DECK,
                    SPLASH.z + 1.6 + a.sin() * 0.8,
                );
                let hop = if pouring {
                    (time * 9.0 + i as f32).sin().abs() * 0.25
                } else {
                    0.0
                };
                (p + Vec3::Y * hop, Vec2::new(-a.cos(), -a.sin()))
            } else {
                let a = time * (0.25 + hash(s + 1) * 0.2) + hash(s + 2) * TAU;
                let r = 5.0 + (time * 0.4 + i as f32).sin() * 1.5;
                let p = Vec3::new(SPLASH.x + a.cos() * r, 0.26, SPLASH.z + a.sin() * r);
                (p, Vec2::new(-a.sin(), a.cos()))
            };
            let m =
                *f * Mat4::from_scale_rotation_translation(Vec3::splat(small), facing(face), at);
            self.person(m, s, draws);
        }
    }

    /// Seals on their rocks: most lie about, lifting heads and tails now and then, one
    /// scratching, while others swim round the rocks, heads up, and dive under.
    fn seals(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        use zawnabout_sim::landmark::SEAL_ROCKS;
        let tint = |m: Mat4, s: u32| {
            let shade = 0.75 + hash(s + 11) * 0.45;
            Instance::tinted(m, Vec4::new(shade, shade, shade, 1.0))
        };
        for (r, (c, rad, top)) in SEAL_ROCKS.into_iter().enumerate() {
            let n = (rad * 1.1) as u32;
            for i in 0..n {
                let s = k * 523 + r as u32 * 41 + i;
                let a = i as f32 / n as f32 * TAU + hash(s) * 0.6;
                let mut at = c + Vec2::new(a.cos(), a.sin()) * rad * (0.25 + hash(s + 1) * 0.35);
                // Clear of the rock's boulder: over to the other side.
                let b = r as f32 * 2.3;
                let boulder = c + Vec2::new(b.cos(), b.sin()) * rad * 0.8;
                if at.distance(boulder) < rad * 0.35 + 1.0 {
                    at = c * 2.0 - at;
                }
                // Heads and tails up in a banana, now and then.
                let up = ((time * 0.3 + hash(s + 2) * 20.0).sin() - 0.6).max(0.0) * 0.9;
                let m = *f
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.9 + hash(s + 3) * 0.35),
                        Quat::from_rotation_y(a + hash(s + 4) * 2.0) * Quat::from_rotation_x(-up),
                        Vec3::new(at.x, top, at.y),
                    );
                draws.push((self.seal, tint(m, s)));
            }
        }
        // Swimmers going round the rocks, heads up between dives.
        for i in 0..5u32 {
            let s = k * 811 + i;
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            let a = time * 0.06 * dir + hash(s) * TAU;
            let r = 14.0 + hash(s + 1) * 8.0 + (time * 0.2 + i as f32).sin() * 2.0;
            let dive = (time * 0.25 + hash(s + 2) * TAU).sin();
            let y = if dive < -0.4 {
                -2.0
            } else {
                -0.35 + dive.max(0.0) * 0.05
            };
            let m = *f
                * Mat4::from_rotation_translation(
                    Quat::from_rotation_y(-a + if dir > 0.0 { 0.0 } else { PI }),
                    Vec3::new(a.cos() * r, y, a.sin() * r),
                );
            draws.push((self.seal, tint(m, s)));
        }
    }

    /// Parcel vans pulled up at a stop: the driver hops out of the side door with a
    /// parcel, walks it to the door, and comes back without it.
    fn couriers(
        &self,
        traffic: &zawnabout_sim::traffic::Traffic,
        sim_time: f32,
        cam: Vec3,
        draws: &mut Vec<(MeshId, Instance)>,
    ) {
        use zawnabout_sim::town::Kind;
        use zawnabout_sim::traffic::VAN;
        for (i, v) in traffic.vehicles.iter().enumerate() {
            if v.kind != Kind::Car || v.variant != VAN {
                continue;
            }
            let Some(t) = traffic.halted_for(v, sim_time) else {
                continue;
            };
            let p = traffic.pose(v, sim_time);
            if p.pos.distance(cam) > 300.0 {
                continue;
            }
            // Out over 2.5 s, a moment at the door, and back.
            let out = if t < 0.5 {
                0.0
            } else if t < 3.0 {
                (t - 0.5) / 2.5
            } else if t < 4.5 {
                1.0
            } else {
                (1.0 - (t - 4.5) / 2.5).max(0.0)
            };
            if t < 0.5 || t > 7.0 {
                continue;
            }
            let ahead = Vec2::new(p.heading.sin(), p.heading.cos());
            let right = Vec2::new(-ahead.y, ahead.x);
            let base = Vec2::new(p.pos.x, p.pos.z) + right * (1.3 + out * 4.5) + ahead * 0.4;
            let face = if t < 3.0 { right } else { -right };
            let at = Vec3::new(base.x, p.pos.y, base.y);
            let m = Mat4::from_rotation_translation(facing(face), at);
            self.person(m, 9001 + i as u32 * 7, draws);
            if t < 3.6 {
                let box_at = m * Mat4::from_translation(Vec3::new(0.0, 1.05, 0.3));
                draws.push((self.parcel, Instance::new(box_at)));
            } else if t < 4.5 {
                let left = m * Mat4::from_translation(Vec3::new(0.0, 0.18, 0.7));
                draws.push((self.parcel, Instance::new(left)));
            }
        }
    }

    /// A carnival's rides: the chair swing's crown spins with its chairs flying out on
    /// their chains, and bumper cars wander their floor, bumping about.
    fn carnival(&self, f: &Mat4, k: u32, time: f32, draws: &mut Vec<(MeshId, Instance)>) {
        use zawnabout_sim::landmark::{BUMPER, CHAIR_SWING, SWING_REACH, SWING_TOP, chair_swing};
        let (turn, speed) = chair_swing(time + k as f32 * 17.0);
        let top = *f
            * Mat4::from_rotation_translation(
                Quat::from_rotation_y(-turn),
                CHAIR_SWING + Vec3::Y * SWING_TOP,
            );
        draws.push((self.swing_crown, Instance::new(top)));
        // Chairs swing out as it speeds up, as far as the spin throws them.
        let lean = (speed * speed * 5.0 / 9.81).atan();
        for i in 0..14u32 {
            let a = i as f32 / 14.0 * TAU;
            let hang = top
                * Mat4::from_rotation_y(-a)
                * Mat4::from_translation(Vec3::new(5.0, -0.4, 0.0))
                * Mat4::from_rotation_z(lean);
            let paint = linear(PAINT[i as usize % 6]);
            draws.push((self.swing_chair, Instance::tinted(hang, paint.extend(1.0))));
            if hash(k * 71 + i) < 0.7 {
                let seat = hang
                    * Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.75),
                        Quat::IDENTITY,
                        Vec3::new(0.0, -SWING_REACH - 0.6, 0.05),
                    );
                self.person(seat, k * 131 + i, draws);
            }
        }
        // Bumper cars: each wanders the floor on its own loop, turning to where it goes.
        let (mid, half) = BUMPER;
        let room = half - Vec2::splat(1.4);
        for i in 0..9u32 {
            let s = k * 59 + i;
            let (fx, fz) = (0.13 + hash(s) * 0.12, 0.11 + hash(s + 1) * 0.12);
            let (px, pz) = (hash(s + 2) * TAU, hash(s + 3) * TAU);
            let at = |t: f32| {
                mid + Vec2::new((t * fx + px).sin() * room.x, (t * fz + pz).sin() * room.y)
            };
            let p = at(time);
            let dir = at(time + 0.3) - p;
            let m = *f * Mat4::from_rotation_translation(facing(dir), Vec3::new(p.x, 0.3, p.y));
            let paint = linear(PAINT[(i as usize + 2) % 6]);
            draws.push((self.bumper_car, Instance::tinted(m, paint.extend(1.0))));
            let rider = m * Mat4::from_scale_rotation_translation(
                Vec3::splat(0.8),
                Quat::IDENTITY,
                Vec3::new(0.0, 0.0, -0.2),
            );
            self.person(rider, s + 400, draws);
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
            (Vec2::new(-25.0, -10.0), Vec2::new(15.0, 9.0), 12u32),
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
        self.splash(f, k, time, draws, fx);
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
                // Sat down in the ring, folded short, feet on the slide's floor.
                let rider = m * Mat4::from_scale_rotation_translation(
                    Vec3::new(0.85, 0.6, 0.85),
                    Quat::IDENTITY,
                    Vec3::new(0.0, -0.17, 0.0),
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
