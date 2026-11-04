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
}

/// How hard trains speed up and brake (m/s^2), and how long they wait at a station (s).
pub const TRAIN_ACCEL: f32 = 0.9;
pub const TRAIN_DWELL: f32 = 20.0;

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
        }
    }

    /// Time to run `len` meters from a halt to a halt, topping out at `top`.
    fn run_time(len: f32, top: f32) -> f32 {
        let a = TRAIN_ACCEL;
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
            .map(|i| TRAIN_DWELL + Self::run_time(self.stop_gap(i), top))
            .sum()
    }

    fn stop_gap(&self, i: usize) -> f32 {
        let n = self.stops.len();
        (self.stops[(i + 1) % n] - self.stops[i])
            .rem_euclid(self.len())
            .max(1.0)
    }

    /// How far along a train's front is `t` seconds into its timetable, and how fast it
    /// goes: waiting at each stop, then speeding up, running at `top` and braking into the
    /// next one.
    pub fn timetable(&self, t: f32, top: f32) -> (f32, f32) {
        let a = TRAIN_ACCEL;
        let mut t = t.rem_euclid(self.lap_time(top));
        for (i, &from) in self.stops.iter().enumerate() {
            let len = self.stop_gap(i);
            let run = Self::run_time(len, top);
            if t < TRAIN_DWELL {
                return (from, 0.0);
            }
            t -= TRAIN_DWELL;
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

#[derive(Clone, Debug, Default)]
pub struct Traffic {
    pub paths: Vec<Path>,
    pub vehicles: Vec<Vehicle>,
    pub poses: Vec<Pose>,
    /// Poses one tick ago, for carrying riders exactly.
    pub prev: Vec<Pose>,
}

/// Car body sizes, picked by variant.
pub const CAR_KINDS: u8 = 6;
/// Car variants past the paint colors: a bus and a delivery truck.
pub const BUS: u8 = CAR_KINDS;
pub const TRUCK: u8 = CAR_KINDS + 1;

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

fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Traffic {
    pub fn new(terrain: &Terrain) -> Self {
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
        let route_vehicles = |t: &mut Traffic, cruise: bool| {
            for r in def.routes {
                if lift_spec(r.kind).is_some() || (r.kind == "cruise") != cruise {
                    continue;
                }
                let pts = r
                    .pts
                    .iter()
                    .map(|p| Vec3::new(p[0], if p[2] <= NO_Y { def.sea_level } else { p[2] }, p[1]))
                    .collect();
                let path = t.paths.len();
                t.paths.push(Path::new(pts, r.looped));
                let len = t.paths[path].len();
                let (kind, speed, size, count) = match r.kind {
                    "yacht" => (Kind::Sailboat, 9.0, Vec3::new(14.0, 2.4, 4.6), 2),
                    "cruise" => (Kind::Cruise, 9.0, CRUISE_SIZE, 1),
                    "seaplane" => (Kind::Seaplane, 42.0, Vec3::new(12.0, 3.5, 14.0), 1),
                    "plane" => (Kind::Airliner, 70.0, Vec3::new(34.0, 4.0, 32.0), 1),
                    "balloon" => (Kind::Balloon, 5.0, Vec3::new(16.0, 22.0, 16.0), 2),
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
        // Cars on every road, both ways, about one per 220m each way.
        for (i, road) in def.roads.iter().enumerate() {
            if road.runway || road.rail {
                continue;
            }
            let profile = terrain.road_points(i);
            let pts: Vec<Vec3> = road
                .pts
                .iter()
                .zip(profile)
                .map(|(p, rp)| Vec3::new(p[0], rp.y, p[1]))
                .collect();
            let path = t.paths.len();
            t.paths.push(Path::new(pts, false));
            let len = t.paths[path].len();
            let count = (len / 220.0).floor().max(1.0) as u32;
            for k in 0..count {
                let seed = crate::shape::hash(i as u32 * 977 + k);
                // Mostly cars, now and then a bus or a truck.
                let big = crate::shape::hash(i as u32 * 977 + k + 5003);
                let variant = if big < 0.1 {
                    BUS
                } else if big < 0.2 {
                    TRUCK
                } else {
                    (seed * 997.0) as u8 % CAR_KINDS
                };
                push(
                    &mut t,
                    Vehicle {
                        id: 0,
                        kind: Kind::Car,
                        path,
                        speed: match variant {
                            BUS => 10.0,
                            TRUCK => 11.0,
                            _ if road.w >= 14.0 => 20.0 + seed * 6.0,
                            _ => 12.0 + seed * 6.0,
                        },
                        // Spread over the round trip, so half drive each way.
                        offset: len * 2.0 * k as f32 / count as f32 + seed * 40.0,
                        size: match variant {
                            BUS => Vec3::new(10.5, 3.0, 2.5),
                            TRUCK => Vec3::new(7.0, 2.8, 2.3),
                            _ => Vec3::new(4.2, 1.5, 1.9),
                        },
                        lane: road.w * 0.22,
                        lead: 0.0,
                        variant,
                    },
                );
            }
        }
        // Trains on every railway: two on each, going round and round and halting at its
        // stations. Main lines run an engine and six carriages, subways four metro cars.
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
            t.paths.push(Path::new(pts, looped));
            let (cars, gap, top) = if road.subway {
                (METRO_CARS, METRO_GAP, 15.0)
            } else {
                (TRAIN_CARS, TRAIN_GAP, 17.0)
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
            for train in 0..2u32 {
                for car in 0..cars {
                    let variant = match (road.subway, car) {
                        (false, 0) => 0,
                        (false, _) => 1 + (train * 3 + car) as u8 % 3,
                        (true, 0) => METRO_CAB,
                        (true, c) if c == cars - 1 => METRO_TAIL,
                        (true, _) => METRO_MID,
                    };
                    let lead = car as f32 * gap;
                    push(
                        &mut t,
                        Vehicle {
                            id: 0,
                            kind: Kind::Train,
                            path,
                            speed: top,
                            // Half a lap apart in time.
                            offset: lap * top * train as f32 / 2.0,
                            size: Vec3::new(gap - 1.2, 4.2, 3.2),
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
        t.poses = vec![Pose::default(); t.vehicles.len()];
        t
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
    pub fn pose(&self, v: &Vehicle, time: f32) -> Pose {
        let path = &self.paths[v.path];
        let len = path.len();
        let (u, speed) = if path.stops.is_empty() {
            (v.offset + v.speed * time, v.speed)
        } else {
            let (front, speed) = path.timetable(time + v.offset / v.speed, v.speed);
            (front - v.lead, speed)
        };
        let look = 12.0f32.min(len * 0.25).max(0.5);
        let at = |k: f32| self.spot(v, u + k * look);
        let (back2, back, here, ahead, ahead2) = (at(-2.0), at(-1.0), at(0.0), at(1.0), at(2.0));
        let pos = (back + here * 2.0 + ahead) * 0.25;
        let dir = (ahead - back).normalize_or(Vec3::Z);
        let turn = wrap(heading_of(ahead2 - here) - heading_of(here - back2));
        let yaw_rate = turn / (2.0 * look) * speed;
        let flying = matches!(v.kind, Kind::Seaplane | Kind::Airliner);
        let bank = if flying && pos.y > 4.0 {
            (yaw_rate * v.speed / 9.81).atan().clamp(-0.6, 0.6)
        } else {
            0.0
        };
        Pose {
            pos,
            heading: heading_of(dir),
            // Planes climb and dive, cars tilt with the road's grade.
            pitch: if flying || matches!(v.kind, Kind::Car | Kind::Train) {
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
        std::mem::swap(&mut self.prev, &mut self.poses);
        self.poses.resize(self.vehicles.len(), Pose::default());
        for i in 0..self.vehicles.len() {
            self.poses[i] = self.pose(&self.vehicles[i], time);
        }
        if self.prev.len() != self.poses.len() {
            self.prev = self.poses.clone();
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
