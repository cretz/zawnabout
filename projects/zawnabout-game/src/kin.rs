//! The other dragons in flight and at rest, and the hatchling at your side. The play
//! layer says who they are and what they have done with you. Here they sit on their
//! perches, hop about their islands, lead the way, race, and get drawn. They fly as
//! real dragons do: the named ones by swoops at their own easy pace, the hatchling
//! tagging along beside you and landing when you land.

use glam::{Vec2, Vec3};
use zawnabout_sim::EntityId;
use zawnabout_sim::dragon::{Dragon, DragonInput, FlightParams, Mode};
use zawnabout_sim::world::World;

use crate::pose::DragonVisual;

/// What the game needs to know of a named dragon.
pub struct KinInfo {
    pub name: String,
    pub kind: &'static str,
    pub tint: u32,
    pub size: f32,
    /// Home first, then the other perches.
    pub spots: Vec<Vec3>,
    /// The way to its secret, stop by stop.
    pub way: Vec<Vec3>,
    /// A racer's finish: its name, where, and how near counts.
    pub race: Option<(String, Vec3, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
    /// Sitting on a perch.
    Rest,
    /// Flying to the next perch.
    Hop,
    /// Leading the way: flying to, or waiting at, stop `n`.
    Lead(usize),
    /// Racing to the finish.
    Race,
}

/// Something worth telling that happened while flying.
#[derive(Clone, Debug, PartialEq)]
pub enum FlockNews {
    /// Dragon `i` waits at the last stop with you there: its secret is right here.
    Shown(usize),
    /// Dragon `i` reached the race finish.
    Finished(usize),
}

/// Easy flying speeds (m/s): hopping about, leading the way, fleeing, racing.
const HOP: f32 = 32.0;
const LEAD: f32 = 34.0;
const FLEE: f32 = 55.0;
const RACE: f32 = 52.0;
/// A leader waits until you are this near before flying on (m).
const WAIT_FOR: f32 = 130.0;

pub struct Flyer {
    pub d: Dragon,
    pub vis: DragonVisual,
    pub size: f32,
    pub task: Task,
    /// The perch it sits on, or last flew to.
    pub spot: usize,
    /// Seconds until it hops to another perch.
    pub rest: f32,
    /// Top speed of its flights (m/s).
    pub speed: f32,
    /// A landing asked for, sent with its next step.
    go: Option<Vec3>,
    /// What is left of a barrel roll done for fun (rad).
    pub spin: f32,
}

impl Flyer {
    fn new(id: u32, at: Vec3, size: f32, stand: f32) -> Self {
        let d = Dragon::new(
            EntityId(id),
            at + Vec3::Y * stand,
            (id as f32 * 1.7).sin() * 3.0,
        );
        let mut d = d;
        d.mode = Mode::Ground;
        Self {
            vis: DragonVisual::new(&d),
            d,
            size,
            task: Task::Rest,
            spot: 0,
            rest: 30.0 + (id % 7) as f32 * 9.0,
            speed: HOP,
            go: None,
            spin: 0.0,
        }
    }

    /// Flies off to land at `to`.
    pub fn fly_to(&mut self, to: Vec3, speed: f32) {
        self.go = Some(to);
        self.speed = speed;
    }

    /// Whether it is down and still, not on its way anywhere.
    pub fn settled(&self) -> bool {
        self.go.is_none() && self.d.mode == Mode::Ground && self.d.swoop.is_none()
    }

    /// Its feet: where it stands, scaled down from its body.
    pub fn feet(&self, stand: f32) -> Vec3 {
        self.d.pos - Vec3::Y * stand
    }

    fn step(&mut self, p: &FlightParams, world: &World, dt: f32) {
        let mut input = DragonInput::default();
        if let Some(to) = self.go.take() {
            input.land_at = Some(to);
        }
        let mut p = p.clone();
        p.swoop_speed = self.speed;
        self.d.step(&p, world, input, dt);
        self.vis.tick(&self.d, &p, world, dt);
        self.spin = (self.spin - dt * 7.0).max(0.0);
    }
}

/// Past this a resting dragon stops moving at all, to save its steps (m).
const STILL_FAR: f32 = 300.0;

#[derive(Default)]
pub struct Flock {
    pub dragons: Vec<Flyer>,
    pub hatchling: Option<Flyer>,
    /// The hatchling is on its way down to land beside you.
    landing: bool,
}

impl Flock {
    /// Puts each named dragon at home, once.
    pub fn ensure(&mut self, kin: &[KinInfo], stand: f32) {
        if self.dragons.len() == kin.len() {
            return;
        }
        self.dragons = kin
            .iter()
            .enumerate()
            .map(|(i, k)| Flyer::new(9000 + i as u32, k.spots[0], k.size, stand))
            .collect();
    }

    /// Hatches the hatchling beside `at`, or takes it away with none.
    pub fn set_hatchling(&mut self, at: Option<Vec3>, size: f32, stand: f32) {
        match (at, &mut self.hatchling) {
            (None, _) => self.hatchling = None,
            (Some(_), Some(h)) => h.size = size,
            (Some(at), None) => self.hatchling = Some(Flyer::new(9900, at, size, stand)),
        }
    }

    /// Dragon `i` flies off to its next perch in a fright.
    pub fn flee(&mut self, i: usize, kin: &KinInfo) -> Option<Vec3> {
        let f = self.dragons.get_mut(i)?;
        f.spot = (f.spot + 1) % kin.spots.len();
        let to = kin.spots[f.spot];
        f.fly_to(to, FLEE);
        f.task = Task::Hop;
        Some(to)
    }

    /// Dragon `i` starts leading the way to its secret.
    pub fn lead(&mut self, i: usize, kin: &KinInfo) {
        if let (Some(f), Some(first)) = (self.dragons.get_mut(i), kin.way.first()) {
            f.fly_to(*first, LEAD);
            f.task = Task::Lead(0);
        }
    }

    /// The racer sets off for the finish.
    pub fn race(&mut self, i: usize, kin: &KinInfo) {
        if let (Some(f), Some((_, at, _))) = (self.dragons.get_mut(i), &kin.race) {
            f.fly_to(*at, RACE);
            f.task = Task::Race;
        }
    }

    /// Back to its perches, the race or the lead over.
    pub fn home(&mut self, i: usize, kin: &KinInfo) {
        if let Some(f) = self.dragons.get_mut(i) {
            f.spot = 0;
            f.fly_to(kin.spots[0], HOP);
            f.task = Task::Hop;
        }
    }

    /// Which dragon is leading the way or racing, if one is.
    pub fn busy(&self) -> Option<(usize, Task)> {
        self.dragons
            .iter()
            .enumerate()
            .find(|(_, f)| matches!(f.task, Task::Lead(_) | Task::Race))
            .map(|(i, f)| (i, f.task))
    }

    /// One step for everyone: perch hops, the way led, the race, the hatchling.
    pub fn tick(
        &mut self,
        kin: &[KinInfo],
        met: &[bool],
        me: &Dragon,
        p: &FlightParams,
        world: &World,
        dt: f32,
    ) -> Vec<FlockNews> {
        let mut news = Vec::new();
        for (i, (f, k)) in self.dragons.iter_mut().zip(kin).enumerate() {
            let near = f.d.pos.distance(me.pos);
            match f.task {
                Task::Rest => {
                    f.rest -= dt;
                    // Hops about only while nobody is close, so it never leaves mid-chat.
                    // Until met it stays home, so it can be found where folk say.
                    let roams = met.get(i).copied().unwrap_or(false);
                    if f.rest <= 0.0 && near > 260.0 && roams && k.spots.len() > 1 {
                        f.spot = (f.spot + 1) % k.spots.len();
                        f.fly_to(k.spots[f.spot], HOP);
                        f.task = Task::Hop;
                        f.rest = 45.0 + (i as f32 * 13.0 + f.spot as f32 * 7.0) % 50.0;
                    } else if near < 40.0 && f.settled() {
                        // Turns to look at you.
                        let to = me.pos - f.d.pos;
                        let want = to.x.atan2(to.z);
                        let diff = (want - f.d.heading + std::f32::consts::PI)
                            .rem_euclid(std::f32::consts::TAU)
                            - std::f32::consts::PI;
                        f.d.heading += diff * (dt * 2.0).min(1.0);
                    }
                }
                Task::Hop => {
                    if f.settled() {
                        f.task = Task::Rest;
                    }
                }
                Task::Lead(n) => {
                    if f.settled() && near < WAIT_FOR {
                        if n + 1 < k.way.len() {
                            f.fly_to(k.way[n + 1], LEAD);
                            f.task = Task::Lead(n + 1);
                        } else if near < 45.0 {
                            news.push(FlockNews::Shown(i));
                        }
                    }
                }
                Task::Race => {
                    if f.settled() {
                        news.push(FlockNews::Finished(i));
                    }
                }
            }
            // Resting far off it is only a speck, and still: it waits until you come near.
            if matches!(f.task, Task::Rest) && f.settled() && near > STILL_FAR {
                continue;
            }
            f.step(p, world, dt);
        }
        self.tick_hatchling(me, p, world, dt);
        news
    }

    /// The hatchling keeps close: off your shoulder in flight, landing beside you when
    /// you land, and hopping after you on the ground.
    fn tick_hatchling(&mut self, me: &Dragon, p: &FlightParams, world: &World, dt: f32) {
        let Some(h) = &mut self.hatchling else { return };
        let fwd = Vec3::new(me.heading.sin(), 0.0, me.heading.cos());
        let side = Vec3::new(fwd.z, 0.0, -fwd.x);
        let stand = p.stand_height * h.size;
        // Left far behind, as when you travel by the map, it simply catches up.
        if h.d.pos.distance(me.pos) > 300.0 {
            h.d.pos = me.pos + side * 5.0;
            h.d.vel = me.vel;
            h.d.mode = me.mode;
            h.vis.snap(&h.d);
        }
        if me.mode == Mode::Air {
            self.landing = false;
            h.go = None;
            h.d.swoop = None;
            let want = me.pos + fwd * 0.5 + side * 5.0 + Vec3::Y * 0.8;
            let to = want - h.d.pos;
            let top = me.speed() + 25.0;
            let vel = (to * 2.5).clamp_length_max(top);
            let before = h.d.heading;
            h.d.vel = vel;
            h.d.pos += vel * dt;
            let floor = world.support(h.d.pos.x, h.d.pos.z, h.d.pos.y + 2.0) + stand;
            h.d.pos.y = h.d.pos.y.max(floor);
            let speed = vel.length();
            if Vec2::new(vel.x, vel.z).length() > 1.0 {
                h.d.heading = vel.x.atan2(vel.z);
            } else {
                h.d.heading = me.heading;
            }
            h.d.path_pitch = if speed > 1.0 {
                (vel.y / speed).asin().clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let turn = ((h.d.heading - before + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI)
                / dt;
            let bank = (-turn * speed / p.gravity).atan().clamp(-1.0, 1.0);
            h.d.bank += (bank - h.d.bank) * (dt * 5.0).min(1.0);
            h.d.mode = Mode::Air;
            h.d.tuck = me.tuck;
            // Small wings work hard: it flaps whenever it climbs or you do.
            let flap = vel.y > 1.0 || me.stroke.active || speed < 25.0;
            if h.d.stroke.active {
                h.d.stroke.phase += dt / 0.4;
                if h.d.stroke.phase >= 1.0 {
                    h.d.stroke = Default::default();
                }
            } else if flap {
                h.d.stroke.active = true;
                h.d.stroke.phase = 0.0;
            }
            h.vis.tick(&h.d, p, world, dt);
            h.spin = (h.spin - dt * 7.0).max(0.0);
            return;
        }
        // On the ground it trots after you along the ground, bounding when it is far
        // behind, and never swoops: a swoop climbs over the roof of a cave.
        let beside = me.pos - Vec3::Y * p.stand_height + side * 3.5 - fwd * 1.5;
        let feet = h.feet(stand);
        let to = Vec3::new(beside.x - feet.x, 0.0, beside.z - feet.z);
        let far = to.length();
        h.go = None;
        h.d.swoop = None;
        h.d.mode = Mode::Ground;
        h.d.stroke = Default::default();
        h.d.bank *= 1.0 - (dt * 6.0).min(1.0);
        h.d.path_pitch *= 1.0 - (dt * 6.0).min(1.0);
        if far > 2.5 {
            let speed = if far > 40.0 {
                far * 1.5
            } else {
                (far * 1.2).clamp(3.0, 12.0)
            };
            let step = to / far * (speed * dt).min(far);
            let at = feet + step;
            let ground = world.support_still(at.x, at.z, feet.y + 4.0);
            h.d.pos = Vec3::new(at.x, ground + stand, at.z);
            h.d.vel = step / dt;
            h.d.heading = to.x.atan2(to.z);
            h.d.stride += step.length() / h.size;
            h.d.run = (speed / 12.0).min(1.0);
        } else {
            h.d.vel = Vec3::ZERO;
            h.d.run = 0.0;
            let want = me.heading;
            let diff = (want - h.d.heading + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            h.d.heading += diff * (dt * 2.0).min(1.0);
        }
        self.landing = false;
        h.vis.tick(&h.d, p, world, dt);
        h.spin = (h.spin - dt * 7.0).max(0.0);
    }
}
