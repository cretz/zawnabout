//! Flying for fun. Stunts are never announced: each gets its name the first time it is
//! done. Courses are natural ones, like flying under every bridge in a town, in any
//! order and over any number of visits. Races start at fixed spots as a train or boat
//! leaves, and no time is kept: win and it is great, fly off and nobody minds.

use glam::{Vec2, Vec3};
use zawnabout_sim::dragon::{Dragon, Mode};
use zawnabout_sim::world::World;

use crate::{Event, GiftDef, Play};

/// A place to pass. With `r` it is a column: anywhere within `r` of `a` between `lo` and
/// `hi`. Without, it is a line on the ground from `a` to `b`, crossed between `lo` and
/// `hi`, like the gap under a bridge.
pub struct GateDef {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub r: f32,
    pub lo: f32,
    pub hi: f32,
}

/// A stunt: through a gate (`gap`, `near`), a move (`dive`, `skim`, `glide`, `high`,
/// `spin`, `deep`, `cling`, `splash`, `comet`, `fast`), or riding a `vehicle` (`ride`).
pub struct StuntDef {
    pub id: &'static str,
    pub name: &'static str,
    pub line: &'static str,
    pub kind: &'static str,
    pub vehicle: &'static str,
    pub gate: Option<GateDef>,
}

/// A natural course: pass every gate, in any order, over any number of visits.
pub struct CourseDef {
    pub id: &'static str,
    pub name: &'static str,
    pub line: &'static str,
    /// What townsfolk say about it, so it can be heard of.
    pub rumor: &'static str,
    pub gold: u32,
    pub prize: GiftDef,
    pub gates: &'static [GateDef],
}

/// A race against a `vehicle` from one place to another, starting as it passes or
/// leaves while the dragon waits on the ground at the start.
pub struct RaceDef {
    pub id: &'static str,
    pub name: &'static str,
    pub vehicle: &'static str,
    pub from: &'static str,
    pub from_at: [f32; 2],
    pub to: &'static str,
    pub to_at: [f32; 2],
    /// How near counts as at the start or the finish.
    pub r: f32,
    pub rumor: &'static str,
    pub gold: u32,
    pub prize: GiftDef,
}

/// A race being run: which, against which vehicle, when it gets there, and whether the
/// dragon has flown off from the start. Staying put is not racing, so nothing is said.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub race: usize,
    pub vehicle: usize,
    pub due: f32,
    pub left: bool,
}

/// Stunt move thresholds. Flat out flapping reaches about 250 m/s, a tucked dive holds
/// a path of about -0.9 rad.
const DIVE_PITCH: f32 = -0.7;
const DIVE_SPEED: f32 = 70.0;
const DIVE_TIME: f32 = 2.5;
const COMET_TIME: f32 = 1.0;
const SKIM_HEIGHT: f32 = 8.0;
const SKIM_SPEED: f32 = 60.0;
const SKIM_TIME: f32 = 3.0;
const GLIDE_TIME: f32 = 30.0;
const HIGH: f32 = 1500.0;
const DEEP: f32 = -1000.0;
const SPINS: f32 = 2.0;
const SPLASH_SPEED: f32 = 60.0;
const FAST: f32 = 260.0;
const RIDE_TIME: f32 = 5.0;
const RIDE_MOVED: f32 = 20.0;
/// Farther than this in one tick is a jump, not flying.
const JUMP: f32 = 40.0;
/// Longest a race can be, looking ahead for the vehicle's arrival (s).
const RACE_LOOK: f32 = 1500.0;

/// One player's flying: what is done, and the moves under way.
#[derive(Clone, Debug, Default)]
pub struct Flying {
    pub stunts: Vec<bool>,
    pub gates: Vec<Vec<bool>>,
    pub courses: Vec<bool>,
    pub won: Vec<bool>,
    /// Races run at least once, won or not.
    pub tried: Vec<bool>,
    pub race: Option<Run>,
    /// A race just run, not started again until the dragon has left its start.
    rested: Option<usize>,
    last: Option<(Vec3, Mode, f32)>,
    time: f32,
    dive: f32,
    comet: f32,
    skim: f32,
    glide: f32,
    spin: f32,
    ride: Option<(usize, f32, Vec3)>,
    /// Touched down from the air: seconds since, and how fast it was going.
    landed: Option<(f32, f32)>,
}

impl Flying {
    pub fn new(stunts: usize, gates: Vec<usize>, races: usize) -> Self {
        Self {
            stunts: vec![false; stunts],
            courses: vec![false; gates.len()],
            gates: gates.into_iter().map(|n| vec![false; n]).collect(),
            won: vec![false; races],
            tried: vec![false; races],
            ..Self::default()
        }
    }

    /// Forgets everything done.
    pub fn reset(&mut self) {
        *self = Self::new(
            self.stunts.len(),
            self.gates.iter().map(Vec::len).collect(),
            self.won.len(),
        );
    }
}

fn flat(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Whether a step from `a` to `b` passes gate `g`.
pub fn passes(g: &GateDef, a: Vec3, b: Vec3) -> bool {
    if g.r > 0.0 {
        return flat(b).distance(Vec2::from(g.a)) < g.r && (g.lo..g.hi).contains(&b.y);
    }
    let (p, e) = (Vec2::from(g.a), Vec2::from(g.b) - Vec2::from(g.a));
    let d = flat(b) - flat(a);
    let den = d.perp_dot(e);
    if den.abs() < 1e-6 {
        return false;
    }
    let w = p - flat(a);
    let (t, u) = (w.perp_dot(e) / den, w.perp_dot(d) / den);
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return false;
    }
    (g.lo..g.hi).contains(&(a.y + (b.y - a.y) * t))
}

/// The water over the ground at `ground`, in caves or out.
fn water_over(world: &World, x: f32, z: f32, ground: f32) -> Option<f32> {
    if ground < world.terrain.height(x, z) - 1.0 {
        world.terrain.cave_water(x, z, ground)
    } else {
        world.terrain.water(x, z, ground)
    }
}

/// Whether a dragon could fly at `p`: not in rock, and clear of the ground and water.
fn open(world: &World, p: Vec3) -> bool {
    let ground = world.terrain.support(p.x, p.z, p.y);
    let water = water_over(world, p.x, p.z, ground).unwrap_or(f32::MIN);
    !world.solid_at(p) && p.y > ground + 4.5 && p.y > water + 4.5
}

/// A clear way through gate `g`: a point in it and the way across, with open air for
/// some way either side. Middles of the gate first, square across it first, and the
/// middle of the first clear band of heights.
pub fn way_through(world: &World, g: &GateDef) -> Option<(Vec3, Vec3)> {
    let (a, b) = (Vec2::from(g.a), Vec2::from(g.b));
    let (lo, hi) = (g.lo.max(-1500.0), g.hi.min(600.0));
    let (spots, across): (Vec<Vec2>, Vec2) = if g.r > 0.0 {
        let ring = (0..17).map(|k| {
            let ang = k as f32 * 0.785;
            let r = if k == 16 {
                0.0
            } else if k < 8 {
                0.5
            } else {
                0.9
            };
            a + Vec2::new(ang.cos(), ang.sin()) * g.r * r
        });
        (ring.rev().collect(), Vec2::X)
    } else {
        let mut ts: Vec<f32> = (0..=40).map(|k| k as f32 / 40.0).collect();
        ts.sort_by(|x, y| (x - 0.5).abs().total_cmp(&(y - 0.5).abs()));
        (
            ts.into_iter().map(|t| a.lerp(b, t)).collect(),
            (b - a).perp().normalize_or(Vec2::X),
        )
    };
    let reach = if g.r > 0.0 { 8.0 } else { 12.0 };
    for turn in [0.0f32, 0.5, -0.5, 1.0, -1.0] {
        let d = Vec2::from_angle(turn).rotate(across);
        let dir = Vec3::new(d.x, 0.0, d.y);
        for p in &spots {
            let clear = |y: f32| {
                let at = Vec3::new(p.x, y, p.y);
                (-5..=5).all(|k| open(world, at + dir * (k as f32 * reach / 5.0)))
            };
            let mut y = lo;
            while y <= hi && !clear(y) {
                y += 1.0;
            }
            if y > hi {
                continue;
            }
            let mut top = y;
            while top + 1.0 <= hi && clear(top + 1.0) {
                top += 1.0;
            }
            return Some((Vec3::new(p.x, (y + top) * 0.5, p.y), dir));
        }
    }
    None
}

/// A vehicle kind as play.kdl names it.
pub fn vehicle_name(world: &World, v: usize) -> String {
    format!("{:?}", world.traffic.vehicles[v].kind).to_lowercase()
}

impl Play {
    /// Watches the player's dragon fly, once a tick, at sim `time` seconds.
    pub fn fly(&mut self, me: &Dragon, world: &World, time: f32) -> Vec<Event> {
        let def = self.def;
        let mut out = Vec::new();
        let f = &mut self.flying;
        let dt = (time - f.time).clamp(0.0, 0.25);
        f.time = time;
        let air = me.mode == Mode::Air;
        let speed = me.vel.length();
        let last = f.last.replace((me.pos, me.mode, speed));
        let step = last
            .map(|(p, _, _)| p)
            .filter(|p| p.distance(me.pos) < JUMP);
        // Moves.
        let diving = air && me.path_pitch < DIVE_PITCH && speed > DIVE_SPEED;
        f.dive = if diving { f.dive + dt } else { 0.0 };
        f.comet = if diving && me.breath > 0.5 {
            f.comet + dt
        } else {
            0.0
        };
        let water = (air && speed > SKIM_SPEED)
            .then(|| {
                let ground = world.terrain.support(me.pos.x, me.pos.z, me.pos.y);
                water_over(world, me.pos.x, me.pos.z, ground)
            })
            .flatten();
        let skimming = water.is_some_and(|w| me.pos.y > w && me.pos.y - w < SKIM_HEIGHT);
        f.skim = if skimming { f.skim + dt } else { 0.0 };
        let gliding = air && !me.stroke.active && me.flapping < 0.05;
        f.glide = if gliding { f.glide + dt } else { 0.0 };
        if air {
            let turned = wrap(me.yaw_rate * dt);
            if turned != 0.0 && turned.signum() != f.spin.signum() && f.spin.abs() > 0.3 {
                f.spin = turned;
            } else {
                f.spin += turned;
            }
        } else {
            f.spin = 0.0;
        }
        if let Some((_, m, s)) = last
            && m == Mode::Air
            && !air
        {
            f.landed = Some((0.0, s));
        } else if let Some((t, _)) = &mut f.landed {
            *t += dt;
        }
        f.landed = f.landed.filter(|(t, _)| *t < 0.5 && !air);
        let splash = me.swimming && f.landed.is_some_and(|(_, s)| s > SPLASH_SPEED);
        let riding = me
            .riding
            .filter(|_| me.mode == Mode::Ground)
            .map(|v| {
                let (rv, t, from) = match f.ride {
                    Some((rv, t, from)) if rv == v => (rv, t + dt, from),
                    _ => (v, 0.0, me.pos),
                };
                f.ride = Some((rv, t, from));
                (t >= RIDE_TIME && from.distance(me.pos) > RIDE_MOVED)
                    .then(|| vehicle_name(world, v))
            })
            .unwrap_or_else(|| {
                f.ride = None;
                None
            });
        for (i, s) in def.stunts.iter().enumerate() {
            let done = match s.kind {
                "gap" | "near" => match (&s.gate, step) {
                    (Some(g), Some(a)) => passes(g, a, me.pos),
                    _ => false,
                },
                "dive" => f.dive >= DIVE_TIME,
                "comet" => f.comet >= COMET_TIME,
                "skim" => f.skim >= SKIM_TIME,
                "glide" => f.glide >= GLIDE_TIME,
                "high" => air && me.pos.y > HIGH,
                "deep" => air && me.pos.y < DEEP,
                "spin" => f.spin.abs() >= SPINS * std::f32::consts::TAU,
                "cling" => me.cling.is_some(),
                "splash" => splash,
                "fast" => air && speed > FAST,
                "ride" => riding.as_deref() == Some(s.vehicle),
                _ => false,
            };
            if done && !f.stunts[i] {
                f.stunts[i] = true;
                out.push(Event::Stunt(i));
            }
        }

        // Courses.
        if let Some(a) = step {
            for (c, course) in def.courses.iter().enumerate() {
                if f.courses[c] {
                    continue;
                }
                for (g, gate) in course.gates.iter().enumerate() {
                    if !f.gates[c][g] && passes(gate, a, me.pos) {
                        f.gates[c][g] = true;
                        if f.gates[c].iter().all(|d| *d) {
                            f.courses[c] = true;
                            out.push(Event::Course(c));
                        } else {
                            out.push(Event::Gate(c, g));
                        }
                    }
                }
            }
        }

        // Races.
        let grounded = me.mode == Mode::Ground && me.riding.is_none();
        if let Some(rest) = f.rested
            && flat(me.pos).distance(Vec2::from(def.races[rest].from_at)) > def.races[rest].r * 2.0
        {
            f.rested = None;
        }
        match f.race {
            Some(mut run) => {
                let r = &def.races[run.race];
                run.left |= flat(me.pos).distance(Vec2::from(r.from_at)) > r.r;
                f.race = Some(run);
                let near_end = flat(me.pos).distance(Vec2::from(r.to_at)) < r.r;
                let leg = Vec2::from(r.from_at).distance(Vec2::from(r.to_at));
                let away = flat(me.pos)
                    .distance(Vec2::from(r.from_at))
                    .min(flat(me.pos).distance(Vec2::from(r.to_at)));
                if me.mode == Mode::Ground && near_end {
                    f.race = None;
                    f.rested = Some(run.race);
                    let first = !f.won[run.race];
                    f.won[run.race] = true;
                    self.changed |= first;
                    out.push(Event::RaceWon(run.race, first));
                } else if time >= run.due {
                    f.race = None;
                    f.rested = Some(run.race);
                    if run.left {
                        out.push(Event::RaceLost(run.race));
                    }
                } else if away > leg * 0.5 + 800.0 {
                    f.race = None;
                }
            }
            None if grounded => {
                for (i, r) in def.races.iter().enumerate() {
                    if f.rested == Some(i) || flat(me.pos).distance(Vec2::from(r.from_at)) > r.r {
                        continue;
                    }
                    if let Some(run) = start(world, time, i, r) {
                        f.race = Some(run);
                        if !f.tried[i] {
                            f.tried[i] = true;
                            self.changed = true;
                        }
                        out.push(Event::RaceOn(i));
                        break;
                    }
                }
            }
            None => {}
        }
        if out
            .iter()
            .any(|e| matches!(e, Event::Stunt(_) | Event::Gate(..) | Event::Course(_)))
        {
            self.changed = true;
        }
        out
    }

    /// Stunts done, in the order of the file.
    pub fn stunts_done(&self) -> impl Iterator<Item = &'static StuntDef> + '_ {
        self.def
            .stunts
            .iter()
            .zip(&self.flying.stunts)
            .filter(|(_, d)| **d)
            .map(|(s, _)| s)
    }

    /// How many of course `c`'s gates are passed.
    pub fn gates_passed(&self, c: usize) -> usize {
        self.flying.gates[c].iter().filter(|g| **g).count()
    }

    /// Prizes from courses flown and races won, for the hoard.
    pub fn prizes(&self) -> impl Iterator<Item = &'static GiftDef> + '_ {
        let courses = self
            .def
            .courses
            .iter()
            .zip(&self.flying.courses)
            .filter(|(_, d)| **d)
            .map(|(c, _)| &c.prize);
        let races = self
            .def
            .races
            .iter()
            .zip(&self.flying.won)
            .filter(|(_, w)| **w)
            .map(|(r, _)| &r.prize);
        courses.chain(races)
    }

    /// Gold from courses flown and races won.
    pub fn prize_gold(&self) -> u32 {
        let courses: u32 = self
            .def
            .courses
            .iter()
            .zip(&self.flying.courses)
            .filter(|(_, d)| **d)
            .map(|(c, _)| c.gold)
            .sum();
        let races: u32 = self
            .def
            .races
            .iter()
            .zip(&self.flying.won)
            .filter(|(_, w)| **w)
            .map(|(r, _)| r.gold)
            .sum();
        courses + races
    }
}

/// Race `i` starts if one of its vehicles is at the start and moving off, and gets to
/// the finish in a while: the run, with when it gets there.
fn start(world: &World, time: f32, i: usize, r: &RaceDef) -> Option<Run> {
    let traffic = &world.traffic;
    let from = Vec2::from(r.from_at);
    let to = Vec2::from(r.to_at);
    traffic
        .vehicles
        .iter()
        .enumerate()
        .filter(|(v, _)| vehicle_name(world, *v) == r.vehicle)
        .filter(|(v, _)| {
            let p = traffic.poses[*v];
            let off = flat(p.pos) - from;
            off.length() < r.r && p.vel.length() > 1.0 && off.dot(flat(p.vel)) > 0.0
        })
        .find_map(|(v, veh)| {
            let mut t = 5.0;
            while t < RACE_LOOK {
                if flat(traffic.pose(veh, time + t).pos).distance(to) < r.r {
                    return Some(Run {
                        race: i,
                        vehicle: v,
                        due: time + t,
                        left: false,
                    });
                }
                t += 1.0;
            }
            None
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLAY;
    use zawnabout_sim::{EntityId, Sim};

    /// A step under a gap counts, over it or beside it does not, and a column counts
    /// anywhere inside.
    #[test]
    fn gates_pass_only_through() {
        let g = GateDef {
            a: [0.0, -10.0],
            b: [0.0, 10.0],
            r: 0.0,
            lo: 0.0,
            hi: 8.0,
        };
        let at = |x, y, z| Vec3::new(x, y, z);
        assert!(passes(&g, at(-1.0, 4.0, 0.0), at(1.0, 4.0, 0.0)));
        assert!(passes(&g, at(1.0, 4.0, 5.0), at(-1.0, 4.0, 5.0)));
        assert!(!passes(&g, at(-1.0, 12.0, 0.0), at(1.0, 12.0, 0.0)));
        assert!(!passes(&g, at(-1.0, 4.0, 20.0), at(1.0, 4.0, 20.0)));
        assert!(!passes(&g, at(-3.0, 4.0, 0.0), at(-1.0, 4.0, 0.0)));
        let c = GateDef {
            a: [0.0, 0.0],
            b: [0.0, 0.0],
            r: 5.0,
            lo: 0.0,
            hi: 10.0,
        };
        assert!(passes(&c, at(9.0, 5.0, 0.0), at(3.0, 5.0, 0.0)));
        assert!(!passes(&c, at(9.0, 5.0, 0.0), at(6.0, 5.0, 0.0)));
    }

    /// Every gate in the file has open air a dragon can fly through, or for a low one
    /// like a goal, ground to walk through on.
    #[test]
    fn gates_are_open() {
        let sim = Sim::new(1);
        let w = &sim.world;
        let gates = PLAY
            .stunts
            .iter()
            .filter_map(|s| s.gate.as_ref().map(|g| (s.id, g)))
            .chain(
                PLAY.courses
                    .iter()
                    .flat_map(|c| c.gates.iter().map(move |g| (c.id, g))),
            );
        for (id, g) in gates {
            let mid = (Vec2::from(g.a) + Vec2::from(g.b)) * 0.5;
            let body = w.support(mid.x, mid.y, g.hi) + 1.5;
            let walk = g.r == 0.0 && (g.lo..g.hi).contains(&body);
            assert!(
                walk || way_through(w, g).is_some(),
                "{id}: no clear way through gate {:?} {:?}",
                g.a,
                g.b
            );
        }
    }

    /// Every race has dry land at both ends, and its vehicle sets off from the start
    /// and gets to the finish in a while.
    #[test]
    fn races_can_be_run() {
        let mut sim = Sim::new(1);
        for (i, r) in PLAY.races.iter().enumerate() {
            for (name, at) in [("start", r.from_at), ("finish", r.to_at)] {
                let c = Vec2::from(at);
                let land = (0..64).any(|k| {
                    let a = k as f32 * 0.7;
                    let p = c + Vec2::new(a.cos(), a.sin()) * r.r * (k % 4 + 1) as f32 / 5.0;
                    let ground = sim.world.support(p.x, p.y, 5000.0);
                    !sim.world.afloat(p.x, p.y, ground)
                });
                assert!(land, "{}: no land near its {name}", r.id);
            }
            let mut t = 0.0;
            let mut run = None;
            while t < 4000.0 && run.is_none() {
                sim.world.traffic.update(t);
                run = start(&sim.world, t, i, r);
                t += 1.0;
            }
            let run = run.unwrap_or_else(|| panic!("{}: the {} never sets off", r.id, r.vehicle));
            let took = run.due - t;
            assert!((15.0..RACE_LOOK).contains(&took), "{}: {took}", r.id);
        }
    }

    /// Waiting at the start, the race comes on as the cable car sets off. Staying put
    /// until it gets there says nothing, and landing at the top first wins.
    #[test]
    fn race_on_won_or_quiet() {
        let mut sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let i = PLAY
            .races
            .iter()
            .position(|r| r.id == "gondola-race")
            .expect("race");
        let r = &PLAY.races[i];
        let ground = |sim: &Sim, at: [f32; 2]| {
            Vec3::new(at[0], sim.world.support(at[0], at[1], 5000.0) + 1.5, at[1])
        };
        let mut me = Dragon::new(EntityId(9), ground(&sim, r.from_at), 0.0);
        me.mode = Mode::Ground;
        let mut t = 0.0;
        let mut on = false;
        for round in 0..2 {
            while t < 3000.0 {
                sim.world.traffic.update(t);
                let events = play.fly(&me, &sim.world, t);
                t += 0.5;
                if events.contains(&Event::RaceOn(i)) {
                    on = true;
                    break;
                }
            }
            assert!(on, "never started");
            if round == 0 {
                // Stay put: it ends with nothing said.
                let due = play.flying.race.expect("running").due;
                while t <= due + 1.0 {
                    sim.world.traffic.update(t);
                    assert!(!play.fly(&me, &sim.world, t).contains(&Event::RaceLost(i)));
                    t += 0.5;
                }
                assert!(play.flying.race.is_none());
                // Step away and back, so it can start again.
                me.pos.x += r.r * 3.0;
                play.fly(&me, &sim.world, t);
                me.pos = ground(&sim, r.from_at);
                on = false;
            }
        }
        me.pos = ground(&sim, r.to_at);
        t += 0.5;
        assert!(
            play.fly(&me, &sim.world, t)
                .contains(&Event::RaceWon(i, true))
        );
        assert!(play.flying.won[i]);
    }

    /// A course fills gate by gate and finishes with a prize, and a stunt is named once.
    #[test]
    fn courses_and_stunts() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let c = PLAY
            .courses
            .iter()
            .position(|c| c.id == "needle-weave")
            .expect("course");
        let gates = PLAY.courses[c].gates;
        let gold = play.gold();
        let mut me = Dragon::new(EntityId(9), Vec3::ZERO, 0.0);
        me.mode = Mode::Air;
        let mut t = 0.0;
        for (k, g) in gates.iter().enumerate() {
            let (a, b) = (Vec2::from(g.a), Vec2::from(g.b));
            let mid = a.lerp(b, 0.5);
            let side = (b - a).perp().normalize() * 3.0;
            let y = g.lo.max(0.0) + 20.0;
            me.pos = Vec3::new(mid.x - side.x, y, mid.y - side.y);
            t += 1.0 / 60.0;
            play.fly(&me, &sim.world, t);
            me.pos = Vec3::new(mid.x + side.x, y, mid.y + side.y);
            t += 1.0 / 60.0;
            let events = play.fly(&me, &sim.world, t);
            if k + 1 < gates.len() {
                assert!(events.contains(&Event::Gate(c, k)), "{events:?}");
            } else {
                assert!(events.contains(&Event::Course(c)), "{events:?}");
            }
        }
        assert!(play.gold() > gold);
        assert!(play.gifts().any(|g| g.name == PLAY.courses[c].prize.name));
        assert_eq!(play.gates_passed(c), gates.len());

        // Up high, the stunt is named once.
        let high = PLAY
            .stunts
            .iter()
            .position(|s| s.kind == "high")
            .expect("stunt");
        me.pos = Vec3::new(0.0, HIGH + 10.0, 0.0);
        t += 1.0;
        assert!(play.fly(&me, &sim.world, t).contains(&Event::Stunt(high)));
        t += 1.0 / 60.0;
        assert!(!play.fly(&me, &sim.world, t).contains(&Event::Stunt(high)));
    }
}
