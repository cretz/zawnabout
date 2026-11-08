//! Vehicles on fixed paths: ferries, yachts, seaplanes and airliners on the routes in the
//! world files, chairs and cabins on ski lifts and gondolas, and cars on the roads. Where each one is depends only on the time, so every
//! peer agrees without sending anything.

use glam::{Quat, Vec2, Vec3};

use crate::shape::{Block, Shape};
use crate::terrain::{NO_Y, Terrain};
use crate::town::Kind;

/// A path to follow, with the distance along it at every point.
#[derive(Clone, Debug)]
pub struct Path {
    pub pts: Vec<Vec3>,
    along: Vec<f32>,
    /// Goes round instead of back and forth.
    pub looped: bool,
    /// Where trains on a looped line halt, front first, in order along it.
    pub stops: Vec<f32>,
    /// How hard what runs on it speeds up and brakes (m/s^2), and how long it waits at
    /// each stop (s).
    pub accel: f32,
    pub dwell: f32,
}

/// How hard trains speed up and brake (m/s^2), and how long they wait at a station (s).
pub const TRAIN_ACCEL: f32 = 0.9;
pub const TRAIN_DWELL: f32 = 20.0;
/// A metro's top speed (m/s), how hard it speeds up and brakes, and its wait at a station.
const METRO_TOP: f32 = 22.0;
const METRO_ACCEL: f32 = 1.3;
const METRO_DWELL: f32 = 9.0;
/// Cars: top speed, how hard they speed up and brake, and how long they stop at the end
/// of a trip.
const CAR_SPEED: f32 = 15.0;
const CAR_ACCEL: f32 = 2.2;
const CAR_DWELL: f32 = 8.0;

impl Path {
    pub fn new(pts: Vec<Vec3>, looped: bool) -> Self {
        let mut along = vec![0.0];
        for w in pts.windows(2) {
            along.push(along[along.len() - 1] + w[0].distance(w[1]));
        }
        Self {
            pts,
            along,
            looped,
            stops: Vec::new(),
            accel: TRAIN_ACCEL,
            dwell: TRAIN_DWELL,
        }
    }

    /// Time to run `len` meters from a halt to a halt, topping out at `top`.
    fn run_time(&self, len: f32, top: f32) -> f32 {
        let a = self.accel;
        if len >= top * top / a {
            len / top + top / a
        } else {
            2.0 * (len / a).sqrt()
        }
    }

    /// Seconds for one lap of a line with stops, waits included.
    pub fn lap_time(&self, top: f32) -> f32 {
        let n = self.stops.len();
        (0..n)
            .map(|i| self.dwell + self.run_time(self.stop_gap(i), top))
            .sum()
    }

    fn stop_gap(&self, i: usize) -> f32 {
        let n = self.stops.len();
        // A lone stop is a whole lap round to itself.
        let gap = (self.stops[(i + 1) % n] - self.stops[i]).rem_euclid(self.len());
        if n == 1 { self.len() } else { gap.max(1.0) }
    }

    /// How far along a train's front is `t` seconds into its timetable, and how fast it
    /// goes: waiting at each stop, then speeding up, running at `top` and braking into the
    /// next one.
    pub fn timetable(&self, t: f32, top: f32) -> (f32, f32) {
        let a = self.accel;
        let mut t = t.rem_euclid(self.lap_time(top));
        for (i, &from) in self.stops.iter().enumerate() {
            let len = self.stop_gap(i);
            let run = self.run_time(len, top);
            if t < self.dwell {
                return (from, 0.0);
            }
            t -= self.dwell;
            if t < run {
                let peak = top.min((len * a).sqrt());
                let ramp = peak / a;
                let (d, v) = if t < ramp {
                    (0.5 * a * t * t, a * t)
                } else if t < run - ramp {
                    (0.5 * a * ramp * ramp + peak * (t - ramp), peak)
                } else {
                    let left = run - t;
                    (len - 0.5 * a * left * left, a * left)
                };
                return (from + d, v);
            }
            t -= run;
        }
        (self.stops[0], 0.0)
    }

    /// How long since a train `t` seconds into its timetable halted, if it is halted.
    pub fn halted_for(&self, t: f32, top: f32) -> Option<f32> {
        if self.stops.is_empty() {
            return None;
        }
        let mut t = t.rem_euclid(self.lap_time(top));
        for i in 0..self.stops.len() {
            if t < self.dwell {
                return Some(t);
            }
            t -= self.dwell + self.run_time(self.stop_gap(i), top);
            if t < 0.0 {
                return None;
            }
        }
        None
    }

    pub fn len(&self) -> f32 {
        self.along[self.along.len() - 1]
    }

    pub fn is_empty(&self) -> bool {
        self.pts.len() < 2
    }

    /// Position and unit direction at distance `s` along the path.
    pub fn at(&self, s: f32) -> (Vec3, Vec3) {
        let s = s.clamp(0.0, self.len());
        let i = self
            .along
            .partition_point(|a| *a <= s)
            .clamp(1, self.pts.len() - 1);
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let span = (self.along[i] - self.along[i - 1]).max(1e-3);
        let t = (s - self.along[i - 1]) / span;
        (a.lerp(b, t), (b - a).normalize_or(Vec3::Z))
    }
}

/// One vehicle: what it is, which path it follows and how.
#[derive(Clone, Copy, Debug)]
pub struct Vehicle {
    /// Stable id, for saves and the network.
    pub id: u32,
    pub kind: Kind,
    pub path: usize,
    /// Meters per second.
    pub speed: f32,
    /// Where along its path it starts.
    pub offset: f32,
    /// Length (local x), height and width (local z), as for buildings.
    pub size: Vec3,
    /// Sideways from the path center, to the right (cars keep to their lane).
    pub lane: f32,
    pub variant: u8,
    /// How far behind its train's front a carriage runs, on lines with stops.
    pub lead: f32,
}

/// Where a vehicle is right now.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pose {
    pub pos: Vec3,
    /// Heading 0 faces +z, as for the dragon.
    pub heading: f32,
    pub pitch: f32,
    pub bank: f32,
    pub vel: Vec3,
    /// Turn rate (rad/s), for carrying riders round with it.
    pub yaw_rate: f32,
}

impl Pose {
    /// The vehicle's world direction of local x (its length) on the ground.
    pub fn axis(&self) -> Vec2 {
        Vec2::new(self.heading.sin(), self.heading.cos())
    }

    /// A world point in this vehicle's own frame.
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        Quat::from_rotation_y(-self.heading) * (p - self.pos)
    }

    /// A point in this vehicle's own frame, back in the world.
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.pos + Quat::from_rotation_y(self.heading) * local
    }

    /// How far it has turned since `was` (rad).
    pub fn heading_since(&self, was: &Pose) -> f32 {
        wrap(self.heading - was.heading)
    }
}

/// Vehicles nearer a dragon than this move every tick (m). Further off they move a tick
/// in a few, which nothing can see: drawing finds its own pose between ticks.
pub const STIR: f32 = 2000.0;

#[derive(Clone, Debug, Default)]
pub struct Traffic {
    pub paths: Vec<Path>,
    pub vehicles: Vec<Vehicle>,
    pub poses: Vec<Pose>,
    /// Poses one tick ago, for carrying riders exactly.
    pub prev: Vec<Pose>,
    /// When a coaster train reaches each point round its track, shared by every theme
    /// park.
    pub coaster: crate::landmark::Coaster,
}

/// Car body sizes, picked by variant.
pub const CAR_KINDS: u8 = 6;
/// Car variants past the paint colors: a bus, a delivery truck and a parcel van.
pub const BUS: u8 = CAR_KINDS;
pub const TRUCK: u8 = CAR_KINDS + 1;
pub const VAN: u8 = CAR_KINDS + 2;

fn heading_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}

fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// To the right of travel along `dir`, flat.
/// What runs on a lift: the chairs or cabins, the cable's height over the ground, the
/// gap between carriers, their speed, and half the gap between the up and down cables.
#[derive(Clone, Copy, Debug)]
pub struct LiftSpec {
    pub carrier: Kind,
    pub size: Vec3,
    pub clearance: f32,
    pub spacing: f32,
    pub speed: f32,
    pub gauge: f32,
}

/// The lift kinds a route can be.
pub fn lift_spec(kind: &str) -> Option<LiftSpec> {
    match kind {
        "chairlift" => Some(LiftSpec {
            carrier: Kind::LiftChair,
            size: Vec3::new(0.9, 0.2, 1.9),
            clearance: 9.0,
            spacing: 22.0,
            speed: 4.5,
            gauge: 2.6,
        }),
        "gondola" => Some(LiftSpec {
            carrier: Kind::LiftCabin,
            size: Vec3::new(3.2, 2.8, 2.8),
            clearance: 18.0,
            spacing: 70.0,
            speed: 6.0,
            gauge: 3.6,
        }),
        _ => None,
    }
}

/// A cruise ship's length, height of its top deck over the water, and beam.
pub const CRUISE_SIZE: Vec3 = Vec3::new(250.0, 42.0, 36.0);
/// A cruise ship's decks, bow to stern, as (middle along its length, half length, base
/// height, height, half beam): the hull, then three tiers of cabins stepping back, the
/// last the sky deck forward of the funnels.
pub const CRUISE_DECKS: [(f32, f32, f32, f32, f32); 4] = [
    (0.0, 125.0, 0.0, 14.0, 18.0),
    (-8.0, 98.0, 14.0, 12.0, 17.0),
    (-16.0, 78.0, 26.0, 10.0, 15.5),
    (22.0, 34.0, 36.0, 6.0, 11.0),
];
/// Where along a cruise ship its two funnels stand.
pub const CRUISE_FUNNELS: [f32; 2] = [-48.0, -70.0];

/// Cars in a train, engine included, and the distance between their middles.
pub const TRAIN_CARS: u32 = 7;
pub const TRAIN_GAP: f32 = 21.0;
/// Cars in a metro train, the distance between their middles, and their variants: the
/// leading cab, the middle cars and the trailing cab.
pub const METRO_CARS: u32 = 4;
pub const METRO_GAP: f32 = 17.0;
pub const METRO_CAB: u8 = 4;
pub const METRO_MID: u8 = 5;
pub const METRO_TAIL: u8 = 6;
/// A cave tour train: its cars, the distance between their middles, and its variants:
/// the engine, then open carriages in three colors.
pub const TOUR_CARS: u32 = 5;
pub const TOUR_GAP: f32 = 7.5;
pub const TOUR_ENGINE: u8 = 7;
pub const TOUR_CAR: u8 = 8;
/// A mine line's engine, and its carts.
pub const MINE_ENGINE: u8 = 11;
pub const MINE_CART: u8 = 12;

/// Where trains on railway `road` halt: each station that snaps to this line, with the
/// train centered on its platform. Matches where [`crate::World`] puts the stations.
fn station_stops(
    def: &crate::terrain::WorldDef,
    road: usize,
    path: &Path,
    cars: u32,
    gap: f32,
) -> Vec<f32> {
    let mut stops: Vec<f32> = def
        .marks
        .iter()
        .filter(|m| m.kind == "station")
        .filter_map(|m| {
            let at = Vec2::new(m.pos[0], m.pos[2]);
            let (r, k) = crate::world::nearest_rail(def, at)?;
            (r == road).then(|| path.along[k] + (cars - 1) as f32 * gap * 0.5)
        })
        .map(|s| s.rem_euclid(path.len()))
        .collect();
    stops.sort_by(f32::total_cmp);
    stops
}

/// How far under the cable a chair's seat and a cabin's floor hang.
pub const CHAIR_DROP: f32 = 3.7;
pub const CABIN_DROP: f32 = 4.6;

/// Height the cable comes down to at each station.
pub const STATION_CABLE: f32 = 6.5;

/// A lift's cable line, along the route from the bottom station to the top: high enough
/// over the ground everywhere, running smooth, and coming down to each station.
pub fn lift_line(terrain: &Terrain, pts: &[[f32; 3]], spec: &LiftSpec) -> Vec<Vec3> {
    let n = pts.len();
    let ground: Vec<f32> = pts.iter().map(|p| terrain.height(p[0], p[1])).collect();
    let mut along = vec![0.0f32; n];
    for i in 1..n {
        let (a, b) = (pts[i - 1], pts[i]);
        along[i] = along[i - 1] + Vec2::new(b[0] - a[0], b[1] - a[1]).length();
    }
    let total = along[n - 1];
    // The highest ground nearby, then smoothed, so the cable spans dips.
    let high: Vec<f32> = (0..n)
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(3), (i + 3).min(n - 1));
            ground[lo..=hi].iter().copied().fold(f32::MIN, f32::max)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(3), (i + 3).min(n - 1));
            let mean = high[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
            let end = along[i].min(total - along[i]);
            let lift = STATION_CABLE + (spec.clearance - STATION_CABLE) * smooth(0.0, 45.0, end);
            let y = if end < 1.0 {
                ground[i] + STATION_CABLE
            } else {
                (mean + lift).max(ground[i] + STATION_CABLE)
            };
            Vec3::new(pts[i][0], y, pts[i][1])
        })
        .collect()
}

fn right(dir: Vec3) -> Vec3 {
    let n = Vec2::new(dir.x, dir.z).normalize_or(Vec2::Y);
    Vec3::new(-n.y, 0.0, n.x)
}

fn flat(dir: Vec3) -> Vec3 {
    Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z)
}

/// Where helicopters set down: every helipad, and every hospital's roof.
fn pads(terrain: &Terrain) -> Vec<Vec3> {
    terrain
        .def
        .marks
        .iter()
        .filter_map(|m| {
            let (x, z) = (m.pos[0], m.pos[2]);
            if let Some(rest) = m.kind.strip_prefix("landmark:oilrig:") {
                // Out at sea, its pad hung off a corner of the deck.
                let r = rest.split(':').next()?.parse::<f32>().ok()?.to_radians();
                let (axis, pad) = (Vec2::new(r.cos(), -r.sin()), crate::landmark::RIG_PAD);
                let p = Vec2::new(x, z) + axis * pad.x + Vec2::new(-axis.y, axis.x) * pad.z;
                return Some(Vec3::new(p.x, m.pos[1] + pad.y, p.y));
            }
            let up = if m.kind.starts_with("landmark:helipad:") {
                crate::landmark::HELIPAD_TOP
            } else if m.kind.starts_with("landmark:hospital:") {
                crate::landmark::HOSPITAL_PAD
            } else {
                return None;
            };
            Some(Vec3::new(x, terrain.height(x, z) + up, z))
        })
        .collect()
}

/// Where every oil rig stands, on the map.
fn rigs(terrain: &Terrain) -> Vec<Vec2> {
    terrain
        .def
        .marks
        .iter()
        .filter(|m| m.kind.starts_with("landmark:oilrig:"))
        .map(|m| Vec2::new(m.pos[0], m.pos[2]))
        .collect()
}

/// Landing spots on every balloon field, on the ground.
fn balloon_spots(terrain: &Terrain) -> Vec<Vec3> {
    terrain
        .def
        .marks
        .iter()
        .filter_map(|m| {
            let rest = m.kind.strip_prefix("landmark:balloonfield:")?;
            let rot: f32 = rest.split(':').next()?.parse().ok()?;
            Some((m, rot.to_radians()))
        })
        .flat_map(|(m, r)| {
            let axis = Vec2::new(r.cos(), -r.sin());
            let front = Vec2::new(-axis.y, axis.x);
            crate::landmark::BALLOON_SPOTS.map(|s| {
                let p = Vec2::new(m.pos[0], m.pos[2]) + axis * s.x + front * s.y;
                Vec3::new(p.x, terrain.height(p.x, p.y) + 0.12, p.y)
            })
        })
        .collect()
}

/// Where along a bus's path it pulls in at a bus stop: each stop it passes close by, once
/// for each time it goes by, clear of the stops it already makes.
fn bus_calls(p: &Path, bus_stops: &[Vec2]) -> Vec<f32> {
    let mut out: Vec<f32> = Vec::new();
    for s in bus_stops {
        // The nearest spot on each pass, a pass being a run of segments close by.
        let mut best: Option<(f32, f32)> = None;
        for (i, w) in p.pts.windows(2).enumerate() {
            let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
            let ab = b - a;
            let u = ((*s - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let d = (a + ab * u).distance(*s);
            let along = p.along[i] + ab.length() * u;
            if d < 9.0 {
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, along));
                }
            } else if let Some((_, at)) = best.take() {
                out.push(at);
            }
        }
        if let Some((_, at)) = best {
            out.push(at);
        }
    }
    out.retain(|&a| {
        p.stops
            .iter()
            .all(|&s| (s - a).abs() > 20.0 && (s - a).abs() < p.len() - 20.0)
    });
    out.sort_by(f32::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 20.0);
    out
}

fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Traffic {
    /// How far a drawbridge at `at` has its leaves lifted (radians): up as a sailing boat
    /// or a trawler comes near, down again once it has gone by.
    pub fn draw_lift(&self, at: Vec2) -> f32 {
        let near = self
            .vehicles
            .iter()
            .zip(&self.poses)
            .filter(|(v, _)| matches!(v.kind, Kind::Sailboat | Kind::Trawler | Kind::Supply))
            .map(|(_, p)| Vec2::new(p.pos.x - at.x, p.pos.z - at.y).length())
            .fold(f32::MAX, f32::min);
        (1.0 - smooth(70.0, 150.0, near)) * 1.3
    }

    /// `places` are where buildings stand, for cars to call at, and `bus_stops` where
    /// buses pull in on their way.
    pub fn new(terrain: &Terrain, places: &[Vec2], bus_stops: &[Vec2]) -> Self {
        let def = terrain.def;
        let mut t = Traffic::default();
        let push = |t: &mut Traffic, v: Vehicle| {
            t.vehicles.push(Vehicle {
                id: t.vehicles.len() as u32,
                ..v
            });
        };
        // Routes in file order, but cruise ships come after the trains below, so adding one
        // keeps the numbers of everything else.
        let spots = balloon_spots(terrain);
        let route_vehicles = |t: &mut Traffic, cruise: bool| {
            for r in def.routes {
                // Lifts have their own cars, and power lines none.
                if lift_spec(r.kind).is_some()
                    || r.kind == "power"
                    || (r.kind == "cruise") != cruise
                {
                    continue;
                }
                let mut pts: Vec<Vec3> = r
                    .pts
                    .iter()
                    .map(|p| Vec3::new(p[0], if p[2] <= NO_Y { def.sea_level } else { p[2] }, p[1]))
                    .collect();
                // A balloon route through a balloon field's landing spot comes down onto the
                // grass there, waits a while and lifts off again.
                let mut landings = Vec::new();
                if r.kind == "balloon" {
                    for (i, p) in pts.iter_mut().enumerate() {
                        if let Some(s) = spots
                            .iter()
                            .find(|s| Vec2::new(s.x - p.x, s.z - p.z).length() < 6.0)
                        {
                            *p = *s;
                            landings.push(i);
                        }
                    }
                }
                let path = t.paths.len();
                t.paths.push(Path::new(pts, r.looped));
                if !landings.is_empty() {
                    let p = &mut t.paths[path];
                    // A looped route ends where it starts: that is one landing, not two.
                    let len = p.len();
                    p.stops = landings
                        .iter()
                        .map(|&i| p.along[i])
                        .filter(|&d| d < len - 1.0 || !p.looped)
                        .collect();
                    // Points next to each other on one spot are one landing.
                    p.stops.dedup_by(|a, b| (*a - *b).abs() < 10.0);
                    p.accel = 0.4;
                    p.dwell = 45.0;
                }
                // Supply ships tie up where they start, in the harbor, and alongside every
                // oil rig they pass, and take their time loading.
                if r.kind == "supply" {
                    let p = &mut t.paths[path];
                    let mut stops = vec![0.0];
                    for rig in rigs(terrain) {
                        let (i, d) = p
                            .pts
                            .iter()
                            .enumerate()
                            .map(|(i, q)| (i, Vec2::new(q.x - rig.x, q.z - rig.y).length()))
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .expect("a route has points");
                        if d < 60.0 {
                            stops.push(p.along[i]);
                        }
                    }
                    p.stops = stops;
                    p.accel = 0.5;
                    p.dwell = 50.0;
                }
                // Helicopters set down at every helipad on their way round, and wait.
                if r.kind == "heli" {
                    let p = &mut t.paths[path];
                    p.stops = pads(terrain)
                        .into_iter()
                        .filter_map(|pad| {
                            let (i, d) = p
                                .pts
                                .iter()
                                .enumerate()
                                .map(|(i, q)| (i, q.distance(pad)))
                                .min_by(|a, b| a.1.total_cmp(&b.1))?;
                            (d < 25.0).then(|| p.along[i])
                        })
                        .collect();
                    p.stops.sort_by(f32::total_cmp);
                    p.accel = 2.5;
                    p.dwell = 30.0;
                }
                let len = t.paths[path].len();
                let (kind, speed, size, count) = match r.kind {
                    "yacht" => (Kind::Sailboat, 9.0, Vec3::new(14.0, 2.4, 4.6), 2),
                    "cruise" => (Kind::Cruise, 9.0, CRUISE_SIZE, 1),
                    "seaplane" => (Kind::Seaplane, 42.0, Vec3::new(12.0, 3.5, 14.0), 1),
                    "plane" => (Kind::Airliner, 70.0, Vec3::new(34.0, 4.0, 32.0), 1),
                    "balloon" => (Kind::Balloon, 5.0, Vec3::new(16.0, 22.0, 16.0), 2),
                    "punt" => (Kind::Punt, 2.2, Vec3::new(8.0, 1.0, 2.4), 3),
                    "trawler" => (Kind::Trawler, 5.0, Vec3::new(15.0, 2.6, 5.0), 1),
                    "speedboat" => (Kind::Speedboat, 15.0, Vec3::new(7.0, 0.8, 2.6), 3),
                    "motoryacht" => (Kind::MotorYacht, 6.0, Vec3::new(26.0, 6.0, 6.5), 1),
                    "heli" => (Kind::Helicopter, 32.0, Vec3::new(12.0, 3.2, 3.0), 1),
                    "supply" => (Kind::Supply, 7.0, Vec3::new(34.0, 3.0, 9.0), 1),
                    _ => (Kind::Ferry, 14.0, Vec3::new(42.0, 6.0, 12.0), 1),
                };
                for k in 0..count {
                    push(
                        t,
                        Vehicle {
                            id: 0,
                            kind,
                            path,
                            speed,
                            offset: len * k as f32 / count as f32,
                            size,
                            lane: 0.0,
                            lead: 0.0,
                            variant: if kind == Kind::Balloon {
                                (path * 2 + k) as u8
                            } else {
                                k as u8
                            },
                        },
                    );
                }
            }
        };
        route_vehicles(&mut t, false);
        // Cars on trips over the road network, about one per 220m of road: out to a far
        // dead end, a moment's wait, round and home again.
        for (k, trip) in crate::trips::car_trips(terrain, places)
            .into_iter()
            .enumerate()
        {
            let path = t.paths.len();
            let mut p = Path::new(trip.pts, true);
            p.stops = trip.stops.to_vec();
            p.accel = CAR_ACCEL;
            p.dwell = CAR_DWELL;
            t.paths.push(p);
            let k = k as u32;
            let seed = crate::shape::hash(k * 977 + 3);
            // Mostly cars, now and then a bus or a truck, more of them on the big roads.
            let big =
                crate::shape::hash(k * 977 + 5003) * if trip.widest >= 14.0 { 0.7 } else { 1.0 };
            let variant = if big < 0.1 {
                BUS
            } else if big < 0.2 {
                TRUCK
            } else if big < 0.3 {
                VAN
            } else {
                (seed * 997.0) as u8 % CAR_KINDS
            };
            if variant == BUS {
                let p = &mut t.paths[path];
                let extra = bus_calls(p, bus_stops);
                p.stops.extend(extra);
                p.stops.sort_by(f32::total_cmp);
            }
            let lap = t.paths[path].lap_time(CAR_SPEED);
            push(
                &mut t,
                Vehicle {
                    id: 0,
                    kind: Kind::Car,
                    path,
                    speed: CAR_SPEED,
                    // Spread over the trip, a time into its timetable.
                    offset: crate::shape::hash(k * 977 + 11) * lap * CAR_SPEED,
                    size: match variant {
                        BUS => Vec3::new(10.5, 3.0, 2.5),
                        TRUCK => Vec3::new(7.0, 2.8, 2.3),
                        VAN => Vec3::new(5.4, 2.4, 2.0),
                        _ => Vec3::new(4.2, 1.5, 1.9),
                    },
                    lane: 0.0,
                    lead: 0.0,
                    variant,
                },
            );
        }
        // Trains on every railway, going round and round and halting at its stations: three
        // on a main line (an engine and six carriages), four on a subway (four metro cars each)
        // and three on a tour or mine line, so there is never long to wait.
        for (i, road) in def.roads.iter().enumerate() {
            if !road.rail {
                continue;
            }
            let profile = terrain.road_points(i);
            let pts: Vec<Vec3> = road
                .pts
                .iter()
                .zip(profile)
                .map(|(p, rp)| Vec3::new(p[0], rp.y, p[1]))
                .collect();
            let looped = pts[0].distance(pts[pts.len() - 1]) < 2.0;
            let path = t.paths.len();
            let mut line = Path::new(pts, looped);
            // A metro is brisk: quick off the mark, fast between stations, and only a
            // short stop at each.
            if road.subway {
                line.accel = METRO_ACCEL;
                line.dwell = METRO_DWELL;
            }
            t.paths.push(line);
            let (cars, gap, top, trains) = if road.tour {
                (TOUR_CARS, TOUR_GAP, 10.0, 3)
            } else if road.subway {
                (METRO_CARS, METRO_GAP, METRO_TOP, 4)
            } else {
                (TRAIN_CARS, TRAIN_GAP, 17.0, 3)
            };
            if looped {
                let stops = station_stops(def, i, &t.paths[path], cars, gap);
                t.paths[path].stops = stops;
            }
            let p = &t.paths[path];
            let len = p.len();
            let lap = if p.stops.is_empty() {
                len / top
            } else {
                p.lap_time(top)
            };
            for train in 0..trains {
                for car in 0..cars {
                    let variant = match (road.tour, road.subway, car) {
                        (true, _, 0) if road.mine => MINE_ENGINE,
                        (true, _, _) if road.mine => MINE_CART,
                        (true, _, 0) => TOUR_ENGINE,
                        (true, _, c) => TOUR_CAR + (c % 3) as u8,
                        (_, false, 0) => 0,
                        (_, false, _) => 1 + (train * 3 + car) as u8 % 3,
                        (_, true, 0) => METRO_CAB,
                        (_, true, c) if c == cars - 1 => METRO_TAIL,
                        (_, true, _) => METRO_MID,
                    };
                    let lead = car as f32 * gap;
                    push(
                        &mut t,
                        Vehicle {
                            id: 0,
                            kind: Kind::Train,
                            path,
                            speed: top,
                            // Spread evenly round in time.
                            offset: lap * top * train as f32 / trains as f32,
                            size: if road.tour {
                                Vec3::new(gap - 1.0, 2.4, 2.2)
                            } else {
                                Vec3::new(gap - 1.2, 4.2, 3.2)
                            },
                            lane: 0.0,
                            variant,
                            lead,
                        },
                    );
                }
            }
        }
        route_vehicles(&mut t, true);
        // Lifts last, so the other vehicles keep their numbers.
        for r in def.routes {
            // Lifts: chairs or cabins round a loop of cable, up one side and down the
            // other, turning round the wheels in each station.
            if let Some(spec) = lift_spec(r.kind) {
                let path = t.paths.len();
                t.paths
                    .push(Path::new(lift_line(terrain, r.pts, &spec), false));
                let len = t.paths[path].len();
                let round = 2.0 * (len + std::f32::consts::PI * spec.gauge);
                let count = (round / spec.spacing).floor().max(2.0) as u32;
                for k in 0..count {
                    push(
                        &mut t,
                        Vehicle {
                            id: 0,
                            kind: spec.carrier,
                            path,
                            speed: spec.speed,
                            offset: round * k as f32 / count as f32,
                            size: spec.size,
                            lane: spec.gauge,
                            variant: (k % 4) as u8,
                            lead: 0.0,
                        },
                    );
                }
            }
        }
        // Fairground wheels: their cars go round a circle standing upright, after the
        // lifts so the other vehicles keep their numbers.
        let wheels = def.marks.iter().filter_map(|m| {
            let rest = m.kind.strip_prefix("landmark:ferris:")?;
            let rot: f32 = rest.split(':').next()?.parse().ok()?;
            Some((m, rot.to_radians()))
        });
        for (k, (m, rot)) in wheels.enumerate() {
            use crate::landmark::{FERRIS_CAR, FERRIS_CARS, FERRIS_HUB, FERRIS_R, FERRIS_SPIN};
            let (x, z) = (m.pos[0], m.pos[2]);
            let base = Vec3::new(x, terrain.height(x, z), z);
            let axis = Vec3::new(rot.cos(), 0.0, -rot.sin());
            let steps = 64;
            let pts: Vec<Vec3> = (0..=steps)
                .map(|i| {
                    let a = i as f32 / steps as f32 * std::f32::consts::TAU;
                    base + FERRIS_HUB + (axis * a.cos() + Vec3::Y * a.sin()) * FERRIS_R
                })
                .collect();
            let path = t.paths.len();
            t.paths.push(Path::new(pts, true));
            // Arc length runs a touch short of the true circle, so scale to match the
            // wheel's own turn.
            let round = t.paths[path].len();
            let per_rad = round / std::f32::consts::TAU;
            for i in 0..FERRIS_CARS {
                let a = k as f32 + i as f32 / FERRIS_CARS as f32 * std::f32::consts::TAU;
                push(
                    &mut t,
                    Vehicle {
                        id: 0,
                        kind: Kind::FerrisCar,
                        path,
                        speed: FERRIS_SPIN * per_rad,
                        offset: a.rem_euclid(std::f32::consts::TAU) * per_rad,
                        size: FERRIS_CAR,
                        lane: 0.0,
                        variant: (i % 5) as u8,
                        lead: 0.0,
                    },
                );
            }
        }
        // Racing cars lap every racetrack, last so the other vehicles keep their numbers:
        // two lanes, each lane's cars at one pace so none drives through another.
        let tracks = def.marks.iter().filter_map(|m| {
            let rest = m.kind.strip_prefix("landmark:racetrack:")?;
            let rot: f32 = rest.split(':').next()?.parse().ok()?;
            Some((m, rot.to_radians()))
        });
        for (k, (m, rot)) in tracks.enumerate() {
            let (x, z) = (m.pos[0], m.pos[2]);
            let base = Vec3::new(x, terrain.height(x, z), z);
            let (axis, front) = (
                Vec2::new(rot.cos(), -rot.sin()),
                Vec2::new(rot.sin(), rot.cos()),
            );
            for (lane, (out, speed)) in [(-3.5f32, 31.0f32), (3.5, 33.0)].into_iter().enumerate() {
                let steps = 320;
                let pts: Vec<Vec3> = (0..=steps)
                    .map(|i| {
                        let u = i as f32 / steps as f32;
                        let (p, _) = crate::landmark::race_line(u, out);
                        let w = axis * p.x + front * p.y;
                        base + Vec3::new(w.x, crate::landmark::race_height(u) + 0.14, w.y)
                    })
                    .collect();
                let path = t.paths.len();
                t.paths.push(Path::new(pts, true));
                let len = t.paths[path].len();
                for c in 0..3u32 {
                    push(
                        &mut t,
                        Vehicle {
                            id: 0,
                            kind: Kind::RaceCar,
                            path,
                            speed,
                            offset: len * (c as f32 / 3.0 + lane as f32 * 0.17 + k as f32 * 0.1),
                            size: Vec3::new(4.6, 1.1, 2.0),
                            lane: 0.0,
                            variant: (lane as u32 * 3 + c) as u8,
                            lead: 0.0,
                        },
                    );
                }
            }
        }
        // Roller coaster trains, two to a theme park half a lap apart, last of all.
        let parks = def.marks.iter().filter_map(|m| {
            let rest = m.kind.strip_prefix("landmark:themepark:")?;
            let rot: f32 = rest.split(':').next()?.parse().ok()?;
            Some((m, rot.to_radians()))
        });
        for (k, (m, rot)) in parks.enumerate() {
            use crate::landmark::{CAR_GAP, COASTER_CARS};
            let (x, z) = (m.pos[0], m.pos[2]);
            let base = Vec3::new(x, terrain.height(x, z), z);
            let (axis, front) = (
                Vec2::new(rot.cos(), -rot.sin()),
                Vec2::new(rot.sin(), rot.cos()),
            );
            let pts: Vec<Vec3> = t
                .coaster
                .pts
                .iter()
                .map(|p| {
                    let w = axis * p.x + front * p.z;
                    base + Vec3::new(w.x, p.y, w.y)
                })
                .collect();
            let path = t.paths.len();
            t.paths.push(Path::new(pts, true));
            let lap = t.coaster.lap;
            for train in 0..2u32 {
                for c in 0..COASTER_CARS {
                    push(
                        &mut t,
                        Vehicle {
                            id: 0,
                            kind: Kind::CoasterCar,
                            path,
                            speed: 0.0,
                            offset: train as f32 * lap * 0.5 + k as f32 * 7.0,
                            size: Vec3::new(2.6, 1.4, 1.8),
                            lane: 0.0,
                            variant: ((c + train as usize * 2) % 6) as u8,
                            lead: c as f32 * CAR_GAP,
                        },
                    );
                }
            }
        }
        // Drop tower rings, one at each theme park, last of all.
        let parks = def.marks.iter().filter_map(|m| {
            let rest = m.kind.strip_prefix("landmark:themepark:")?;
            let rot: f32 = rest.split(':').next()?.parse().ok()?;
            Some((m, rot.to_radians()))
        });
        for (k, (m, rot)) in parks.enumerate() {
            let (x, z) = (m.pos[0], m.pos[2]);
            let d = crate::landmark::DROP;
            let (axis, front) = (
                Vec2::new(rot.cos(), -rot.sin()),
                Vec2::new(rot.sin(), rot.cos()),
            );
            let w = axis * d.x + front * d.z;
            let base = Vec3::new(x + w.x, terrain.height(x, z), z + w.y);
            let path = t.paths.len();
            t.paths
                .push(Path::new(vec![base, base + Vec3::Y * 60.0], false));
            push(
                &mut t,
                Vehicle {
                    id: 0,
                    kind: Kind::DropRing,
                    path,
                    speed: 0.0,
                    offset: k as f32 * 11.0,
                    size: Vec3::new(7.4, 1.2, 7.4),
                    lane: 0.0,
                    variant: 0,
                    lead: 0.0,
                },
            );
        }
        t.poses = vec![Pose::default(); t.vehicles.len()];
        t
    }

    /// Where a coaster car is: up the lift hill slowly, racing down the drops, banked
    /// into the turns the way the track is built for the speed.
    fn coaster_pose(&self, v: &Vehicle, time: f32) -> Pose {
        let ride = &self.coaster;
        let path = &self.paths[v.path];
        let n = path.pts.len();
        let point = |s: f32| {
            let f = (s / ride.step).rem_euclid(n as f32);
            let i = f as usize % n;
            path.pts[i].lerp(path.pts[(i + 1) % n], f.fract())
        };
        let when = time + v.offset;
        let lap = ride.step * n as f32;
        let front = ride.front(when);
        let speed = (ride.front(when + 0.05) - front).rem_euclid(lap) / 0.05;
        let s = front - v.lead;
        let pos = point(s);
        let dir = (point(s + 1.2) - point(s - 1.2)).normalize_or(Vec3::Z);
        let (p0, p2) = (point(s - 6.0), point(s + 6.0));
        let (d0, d1) = (
            Vec2::new(pos.x - p0.x, pos.z - p0.z),
            Vec2::new(p2.x - pos.x, p2.z - pos.z),
        );
        let bend = d0.perp_dot(d1) / (d0.length() * d1.length() * 6.0).max(1e-3);
        Pose {
            pos,
            heading: heading_of(dir),
            pitch: dir.y.clamp(-1.0, 1.0).asin(),
            // Leaning into the bend. Vehicles roll about their nose, so the sign flips.
            bank: -(speed * speed * bend / 9.8).atan().clamp(-0.7, 0.7),
            vel: dir * speed,
            yaw_rate: bend * speed,
        }
    }

    /// Where vehicle `v` is `u` meters into its trip. Open paths go out and back, with a
    /// U-turn round each end so nothing ever spins on the spot.
    fn spot(&self, v: &Vehicle, u: f32) -> Vec3 {
        let path = &self.paths[v.path];
        let len = path.len().max(1.0);
        if path.looped {
            let (pos, dir) = path.at(u.rem_euclid(len));
            return pos + right(dir) * v.lane;
        }
        let r = if v.lane != 0.0 {
            v.lane.abs()
        } else {
            (v.size.x * 0.6).clamp(4.0, 40.0)
        };
        let arc = std::f32::consts::PI * r;
        let u = u.rem_euclid(2.0 * (len + arc));
        // Boats and planes keep to their line and only swing out near the ends to turn.
        let side = |s: f32| {
            if v.lane != 0.0 {
                r
            } else {
                r * (1.0 - smooth(0.0, r * 3.0, s.min(len - s)))
            }
        };
        if u < len {
            let (pos, dir) = path.at(u);
            pos + right(dir) * side(u)
        } else if u < len + arc {
            let (end, dir) = path.at(len);
            let a = (u - len) / r;
            end + (right(dir) * a.cos() + flat(dir) * a.sin()) * r
        } else if u < 2.0 * len + arc {
            let s = 2.0 * len + arc - u;
            let (pos, dir) = path.at(s);
            pos - right(dir) * side(s)
        } else {
            let (start, dir) = path.at(0.0);
            let a = (u - 2.0 * len - arc) / r;
            start - (right(dir) * a.cos() + flat(dir) * a.sin()) * r
        }
    }

    /// Where vehicle `v` is at `time` seconds. The path is rounded off, so corners are
    /// curves and riders are never jolted.
    /// How long `v` has been halted at a stop at `time`, if it is.
    pub fn halted_for(&self, v: &Vehicle, time: f32) -> Option<f32> {
        self.paths[v.path].halted_for(time + v.offset / v.speed, v.speed)
    }

    pub fn pose(&self, v: &Vehicle, time: f32) -> Pose {
        if v.kind == Kind::CoasterCar {
            return self.coaster_pose(v, time);
        }
        if v.kind == Kind::DropRing {
            let base = self.paths[v.path].pts[0];
            let (y, y2) = (
                crate::landmark::drop_height(time + v.offset),
                crate::landmark::drop_height(time + v.offset + 0.05),
            );
            return Pose {
                pos: base + Vec3::Y * (y - 0.6),
                heading: 0.0,
                pitch: 0.0,
                bank: 0.0,
                vel: Vec3::Y * (y2 - y) / 0.05,
                yaw_rate: 0.0,
            };
        }
        let path = &self.paths[v.path];
        let len = path.len();
        // A ferris car hangs level from its pin, broadside to the wheel, whichever way
        // the wheel carries it.
        if v.kind == Kind::FerrisCar {
            let u = (v.offset + v.speed * time).rem_euclid(len.max(1.0));
            let (pos, dir) = path.at(u);
            let n = path.pts.len();
            let axle = path.pts[0] - path.pts[(n - 1) / 2];
            return Pose {
                pos,
                heading: heading_of(axle),
                pitch: 0.0,
                bank: 0.0,
                vel: dir * v.speed,
                yaw_rate: 0.0,
            };
        }
        let (u, speed) = if path.stops.is_empty() {
            (v.offset + v.speed * time, v.speed)
        } else {
            let (front, speed) = path.timetable(time + v.offset / v.speed, v.speed);
            (front - v.lead, speed)
        };
        // Cars turn round in tight spots, so they smooth over less of their path.
        let reach: f32 = match v.kind {
            Kind::Car => 5.0,
            Kind::Balloon => 2.0,
            _ => 12.0,
        };
        let look = reach.min(len * 0.25).max(0.5);
        let at = |k: f32| self.spot(v, u + k * look);
        let (back2, back, here, ahead, ahead2) = (at(-2.0), at(-1.0), at(0.0), at(1.0), at(2.0));
        let pos = (back + here * 2.0 + ahead) * 0.25;
        let dir = (ahead - back).normalize_or(Vec3::Z);
        let turn = wrap(heading_of(ahead2 - here) - heading_of(here - back2));
        let yaw_rate = turn / (2.0 * look) * speed;
        let flying = matches!(v.kind, Kind::Seaplane | Kind::Airliner | Kind::Helicopter);
        let bank = if flying && pos.y > 4.0 {
            (yaw_rate * v.speed / 9.81).atan().clamp(-0.6, 0.6)
        } else {
            0.0
        };
        Pose {
            pos,
            heading: heading_of(dir),
            // Planes climb and dive, cars tilt with the road's grade.
            pitch: if v.kind == Kind::Helicopter {
                -0.12 * speed / v.speed.max(1.0)
            } else if flying || matches!(v.kind, Kind::Car | Kind::Train | Kind::RaceCar) {
                dir.y.asin()
            } else {
                0.0
            },
            bank,
            vel: dir * speed,
            yaw_rate,
        }
    }

    /// Moves every vehicle to `time`.
    pub fn update(&mut self, time: f32) {
        self.update_near(time, &[], 0, 1);
    }

    /// Brings vehicle `i` up to `time` at once, still, as when a dragon is put straight
    /// onto it from far off.
    pub fn refresh(&mut self, i: usize, time: f32) {
        if let Some(v) = self.vehicles.get(i) {
            let p = self.pose(v, time);
            self.poses[i] = p;
            self.prev[i] = p;
        }
    }

    /// Moves the vehicles within [`STIR`] of any of `near` to `time`, and of the rest only
    /// one in `every`, in turn by `tick`: nothing far off can touch them, and drawing
    /// finds its own pose for each.
    pub fn update_near(&mut self, time: f32, near: &[Vec3], tick: u64, every: usize) {
        std::mem::swap(&mut self.prev, &mut self.poses);
        if self.prev.len() != self.vehicles.len() {
            self.prev = (0..self.vehicles.len())
                .map(|i| self.pose(&self.vehicles[i], time))
                .collect();
        }
        self.poses.clone_from(&self.prev);
        let turn = tick as usize % every.max(1);
        for i in 0..self.vehicles.len() {
            let close = || {
                let at = self.prev[i].pos;
                near.iter().any(|p| p.distance_squared(at) < STIR * STIR)
            };
            if i % every.max(1) == turn || close() {
                self.poses[i] = self.pose(&self.vehicles[i], time);
            }
        }
    }

    /// The two runs of cable on every lift, as points along them.
    pub fn cables(&self) -> Vec<Vec<Vec3>> {
        let mut out = Vec::new();
        let mut done = Vec::new();
        for v in &self.vehicles {
            if !matches!(v.kind, Kind::LiftChair | Kind::LiftCabin) || done.contains(&v.path) {
                continue;
            }
            done.push(v.path);
            let path = &self.paths[v.path];
            let len = path.len();
            for side in [1.0, -1.0] {
                let steps = (len / 6.0).ceil().max(1.0) as usize;
                out.push(
                    (0..=steps)
                        .map(|k| {
                            let (p, dir) = path.at(len * k as f32 / steps as f32);
                            p + right(dir) * v.lane * side
                        })
                        .collect(),
                );
            }
        }
        out
    }

    /// The solid parts of vehicle `i` where it is now. They follow the
    /// meshes: a ferry is a deck with two cabins and a funnel on it.
    pub fn shapes(&self, i: usize) -> Vec<Shape> {
        let v = &self.vehicles[i];
        let p = &self.poses[i];
        let block = |half: Vec3, y: f32| {
            Shape::Block(Block {
                base: p.pos + Vec3::Y * y,
                half,
                axis: p.axis(),
                roof: 0.0,
            })
        };
        let (w, d) = (v.size.x, v.size.z);
        match v.kind {
            // The basket hangs under the envelope.
            Kind::Balloon => vec![
                block(Vec3::new(1.3, 1.4, 1.3), 0.0),
                self.shape(i).expect("balloons are solid"),
            ],
            Kind::Cruise => {
                let a = p.axis();
                let along = Vec3::new(a.x, 0.0, a.y);
                let mut out: Vec<Shape> = CRUISE_DECKS
                    .iter()
                    .map(|&(x, hx, y, h, hz)| {
                        Shape::Block(Block {
                            base: p.pos + along * x + Vec3::Y * y,
                            half: Vec3::new(hx, h, hz),
                            axis: a,
                            roof: 0.0,
                        })
                    })
                    .collect();
                for x in CRUISE_FUNNELS {
                    out.push(Shape::Frustum(crate::shape::Frustum {
                        base: p.pos + along * x + Vec3::Y * 36.0,
                        height: 14.0,
                        r0: 5.0,
                        r1: 4.4,
                        spin: 0.0,
                    }));
                }
                out
            }
            // The hull, and the bridge standing on its fore deck.
            Kind::Supply => vec![
                block(Vec3::new(w * 0.5, v.size.y, d * 0.5), 0.0),
                Shape::Block(Block {
                    base: p.pos
                        + Vec3::new(p.axis().x, 0.0, p.axis().y) * w * 0.275
                        + Vec3::Y * 3.0,
                    half: Vec3::new(3.2, 6.4, d * 0.45),
                    axis: p.axis(),
                    roof: 0.0,
                }),
            ],
            // Hull, saloon and the sun deck on the flybridge, all on the middle.
            Kind::MotorYacht => vec![
                block(Vec3::new(w * 0.5, 2.0, d * 0.5), 0.0),
                block(Vec3::new(w * 0.3, 2.2, d * 0.4), 2.0),
                block(Vec3::new(w * 0.16, 1.8, d * 0.34), 4.2),
            ],
            Kind::Ferry => vec![
                block(Vec3::new(w * 0.5, 2.0, d * 0.5), 0.0),
                block(Vec3::new(w * 0.275, 3.0, d * 0.4), 2.0),
                block(Vec3::new(w * 0.15, 2.5, d * 0.3), 5.0),
                Shape::Frustum(crate::shape::Frustum {
                    base: p.pos + Vec3::Y * 7.5,
                    height: 3.5,
                    r0: 1.4,
                    r1: 1.2,
                    spin: 0.0,
                }),
            ],
            _ => self.shape(i).into_iter().collect(),
        }
    }

    /// The main solid box of vehicle `i` where it is now.
    pub fn shape(&self, i: usize) -> Option<Shape> {
        let v = &self.vehicles[i];
        let p = &self.poses[i];
        // Planes are solid around the body only, wings aside. The body's middle sits at
        // the height the plane meshes put it.
        let r = v.size.x * 0.065;
        let (half, lift) = match v.kind {
            Kind::Airliner => (Vec3::new(v.size.x * 0.5, r * 2.0, r), v.size.y - r),
            Kind::Seaplane => (Vec3::new(v.size.x * 0.5, r * 2.0, r), v.size.y * 0.7 - r),
            // The envelope, with a flat top to land on.
            Kind::Balloon => (Vec3::new(6.5, v.size.y - 6.0, 6.5), 6.0),
            // Chairs and cabins hang under the cable.
            Kind::LiftChair => (
                Vec3::new(v.size.x * 0.5, v.size.y, v.size.z * 0.5),
                -CHAIR_DROP,
            ),
            Kind::LiftCabin => (
                Vec3::new(v.size.x * 0.5, v.size.y, v.size.z * 0.5),
                -CABIN_DROP,
            ),
            Kind::FerrisCar => (
                Vec3::new(v.size.x * 0.5, v.size.y, v.size.z * 0.5),
                -crate::landmark::FERRIS_DROP,
            ),
            _ => (Vec3::new(v.size.x * 0.5, v.size.y, v.size.z * 0.5), 0.0),
        };
        Some(Shape::Block(Block {
            base: p.pos + Vec3::Y * lift,
            half,
            axis: p.axis(),
            roof: 0.0,
        }))
    }
}
