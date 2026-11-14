//! Tap-to-land: a planned flight along a smooth curve that arcs over everything between
//! the dragon and the spot, rushes there fast and settles from above.

use glam::Vec3;

use crate::world::World;

/// Samples kept for converting distance along the curve to a curve parameter.
const SAMPLES: usize = 48;
/// Gentlest braking at the end of a swoop (m/s^2). Birds stop hard with a flurry of
/// back-flaps.
const BRAKING: f32 = 30.0;
/// Fast swoops brake harder, so the stop always takes about this distance (m).
const BRAKE_DISTANCE: f32 = 90.0;
/// Speed at touchdown (m/s).
const TOUCHDOWN: f32 = 2.5;

#[derive(Clone, Debug)]
pub struct Swoop {
    /// Where the feet land.
    pub target: Vec3,
    /// Bezier control points for the body center.
    points: [Vec3; 4],
    /// Cumulative length at each sample.
    lengths: [f32; SAMPLES + 1],
    /// Distance travelled along the curve.
    pub travelled: f32,
    pub speed: f32,
    /// Braking for this swoop's cruise speed (m/s^2).
    braking: f32,
    /// Landing on a moving boat or plane: the flight bends to follow it.
    pub carrier: Option<Carrier>,
    /// How far the flight was bent last tick, for its speed.
    pub shift: Vec3,
    /// Landing on the side of something: which way the wall faces.
    pub wall: Option<Vec3>,
}

/// A vehicle a swoop lands on: which one, where on it, and where that was at the start.
#[derive(Clone, Copy, Debug)]
pub struct Carrier {
    pub index: usize,
    /// The landing spot in the vehicle's frame.
    local: Vec3,
    start: Vec3,
}

fn bezier(p: &[Vec3; 4], t: f32) -> Vec3 {
    let u = 1.0 - t;
    p[0] * (u * u * u) + p[1] * (3.0 * u * u * t) + p[2] * (3.0 * u * t * t) + p[3] * (t * t * t)
}

impl Swoop {
    /// Plans a flight from `pos` (body center, moving at `vel`) to feet at `target`.
    pub fn plan(world: &World, pos: Vec3, vel: Vec3, target: Vec3, stand_height: f32) -> Self {
        let carrier = world.carrier(target.x, target.z, target.y);
        let wall = carrier
            .is_none()
            .then(|| world.wall_normal(target))
            .flatten();
        // Belly to the wall, or feet on the ground.
        let end = target + wall.unwrap_or(Vec3::Y) * stand_height;
        let flat = Vec3::new(end.x - pos.x, 0.0, end.z - pos.z);
        let dist = flat.length().max(1.0);
        let across = flat / dist;

        // Clear the tallest thing under the straight line, with room to spare, but only
        // things near the line: the hill over a cave is no reason to climb. And never
        // rise into a roof overhead, which only a sample in open air can have: one inside
        // a hill has the hill above it, and that is to be cleared.
        let mut top = pos.y.max(end.y);
        let mut roof = f32::MAX;
        let reach_up = top + 60.0;
        for i in 0..=24 {
            let p = pos.lerp(end, i as f32 / 24.0);
            top = top.max(world.support(p.x, p.z, reach_up) + stand_height);
            if world.solid_at(p) {
                continue;
            }
            if let Some(k) = (1..=20).find(|k| world.solid_at(p + Vec3::Y * (*k as f32 * 4.0))) {
                roof = roof.min(p.y + k as f32 * 4.0);
            }
        }
        let clear = (top + 8.0 + dist * 0.06)
            .min(roof - stand_height - 4.0)
            .max(pos.y.max(end.y));

        let speed = vel.length();
        // Leave along the current motion, already leaning toward the target. From a
        // standstill, leap up and toward it.
        let leave = if speed > 3.0 {
            (vel / speed * 0.6 + across * 0.4 + Vec3::Y * 0.15).normalize()
        } else {
            (across + Vec3::Y * 0.9).normalize()
        };
        let reach = (dist / 3.0).clamp(4.0, 120.0);
        let mut p1 = pos + leave * reach;
        // Arrive from above and a little behind, so the last stretch drops onto the spot.
        let mut p2 = match wall {
            // Fly straight at a wall, from out in front of it.
            Some(n) => end + n * (reach * 0.7).max(6.0) + Vec3::Y * reach * 0.1,
            None => end - across * reach * 0.5 + Vec3::Y * (reach * 0.6).max(6.0),
        };
        p1.y = p1.y.max(clear);
        if wall.is_none() {
            p2.y = p2.y.max(clear.min(end.y + reach * 1.5));
        }

        let mut s = Self {
            target,
            points: [pos, p1, p2, end],
            lengths: [0.0; SAMPLES + 1],
            travelled: 0.0,
            speed: speed.max(6.0),
            braking: BRAKING,
            carrier: carrier.map(|index| Carrier {
                index,
                local: world.traffic.poses[index].to_local(target),
                start: target,
            }),
            shift: Vec3::ZERO,
            wall,
        };
        s.measure();
        s
    }

    fn measure(&mut self) {
        let mut prev = self.points[0];
        let mut total = 0.0;
        for i in 1..=SAMPLES {
            let p = bezier(&self.points, i as f32 / SAMPLES as f32);
            total += p.distance(prev);
            self.lengths[i] = total;
            prev = p;
        }
    }

    pub fn length(&self) -> f32 {
        self.lengths[SAMPLES]
    }

    fn param(&self, s: f32) -> f32 {
        let s = s.clamp(0.0, self.length());
        let i = self.lengths.partition_point(|l| *l < s).clamp(1, SAMPLES);
        let (a, b) = (self.lengths[i - 1], self.lengths[i]);
        let f = if b > a { (s - a) / (b - a) } else { 0.0 };
        (i as f32 - 1.0 + f) / SAMPLES as f32
    }

    /// Advances one tick. Returns the new body center, velocity and whether it arrived.
    pub fn step(&mut self, dt: f32, top_speed: f32) -> (Vec3, Vec3, bool) {
        let left = self.length() - self.travelled;
        // Short hops stay gentle, long trips go all out, and very long ones rush.
        let top = top_speed.max((self.length() * 0.06).min(320.0));
        // At least 18 m/s, unless the top speed itself is lower.
        let cruise = (self.length() * 0.18).max(18.0).min(top);
        // Rush in, then brake hard enough to touch down gently.
        self.braking = (cruise * cruise / (2.0 * BRAKE_DISTANCE)).max(BRAKING);
        let brake_limit = (2.0 * self.braking * left + TOUCHDOWN * TOUCHDOWN).sqrt();
        let want = cruise.min(brake_limit);
        self.speed = if want > self.speed {
            (self.speed + 30.0 * dt).min(want)
        } else {
            want
        };
        self.travelled = (self.travelled + self.speed * dt).min(self.length());
        let t = self.param(self.travelled);
        let pos = bezier(&self.points, t);
        let ahead =
            bezier(&self.points, (t + 0.01).min(1.0)) - bezier(&self.points, (t - 0.01).max(0.0));
        let vel = ahead.normalize_or_zero() * self.speed;
        (pos, vel, self.length() - self.travelled < 0.05)
    }

    /// How far the landing spot has moved with its vehicle since the start, blended in
    /// over the flight so it starts in still air and ends on the deck.
    pub fn bend(&self, world: &World) -> Vec3 {
        let Some(c) = self.carrier else {
            return Vec3::ZERO;
        };
        let now = world.traffic.poses[c.index].to_world(c.local);
        let t = (self.progress() / 0.7).clamp(0.0, 1.0);
        (now - c.start) * (t * t * (3.0 - 2.0 * t))
    }

    /// Where the feet will land right now: on a moving deck, wherever it has got to.
    pub fn goal(&self, world: &World) -> Vec3 {
        match self.carrier {
            Some(c) => world.traffic.poses[c.index].to_world(c.local),
            None => self.target,
        }
    }

    /// Whether this is the final hard stop.
    pub fn landing(&self) -> bool {
        let left = self.length() - self.travelled;
        left < self.speed * self.speed / (2.0 * self.braking) + 4.0
    }

    /// Fraction of the way there.
    pub fn progress(&self) -> f32 {
        self.travelled / self.length().max(1e-3)
    }
}
