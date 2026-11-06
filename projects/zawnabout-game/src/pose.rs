//! Procedural dragon animation. Pure visuals derived from sim state, so it lives on the
//! client and never feeds back into the simulation.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::{Dragon, FlightParams, Mode, World};

use crate::mesh::hash;
use crate::models::{
    ELBOW, EYES, HIPS, NECK_BASE, NECK_LEN, SHOULDER, SNOUT, TAIL_BASE, TAIL_LEN, TAIL_ROOT,
    TAIL_SEGMENTS, TAIL_TAPER, WHISKER_LEN, WHISKER_SEGMENTS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Body,
    Neck(usize),
    Head,
    Eye,
    Tail(usize),
    Leg,
    Arm(usize),
    Hand(usize),
    /// Dizzy stars after a bonk.
    Star,
    /// The feather fan at the tail's tip.
    Plume,
    /// One piece of a whisker.
    Whisker,
    /// A ruff of feathers on the neck or head.
    Mane,
}

/// Wing joint angles (left side, mirrored for right).
#[derive(Clone, Copy, Debug, Default)]
struct Wing {
    flap: f32,
    sweep: f32,
    fold: f32,
    flex: f32,
    twist: f32,
}

impl Wing {
    const fn new(flap: f32, sweep: f32, fold: f32, flex: f32, twist: f32) -> Self {
        Self {
            flap,
            sweep,
            fold,
            flex,
            twist,
        }
    }

    fn lerp(self, o: Wing, t: f32) -> Wing {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Wing::new(
            l(self.flap, o.flap),
            l(self.sweep, o.sweep),
            l(self.fold, o.fold),
            l(self.flex, o.flex),
            l(self.twist, o.twist),
        )
    }
}

const GLIDE: Wing = Wing::new(0.12, 0.06, 0.12, 0.0, 0.0);
const FOLDED: Wing = Wing::new(0.75, 1.3, 2.55, 0.25, 0.1);
const TUCKED: Wing = Wing::new(0.2, 1.05, 2.1, 0.0, 0.0);
const BRAKING: Wing = Wing::new(0.32, -0.3, 0.0, 0.15, -0.45);

/// Per-dragon animation state, ticked with the sim.
#[derive(Clone, Debug)]
pub struct DragonVisual {
    prev_pos: Vec3,
    prev_rot: Quat,
    pos: Vec3,
    rot: Quat,
    prev_heading: f32,
    wing: Wing,
    tail: [Vec2; TAIL_SEGMENTS],
    neck: Vec2,
    /// 1 holds the neck up like when standing, 0 stretches it forward for flight.
    upright: f32,
    legs: f32,
    stride: f32,
    roll: f32,
    squash: f32,
    bonk: f32,
    time: f32,
    was_ground: bool,
    seed: u32,
    /// Airspeed, smoothed, so whiskers and feathers stream back as it builds.
    speed: f32,
    /// Leaning with the slope underfoot, so the feet sit on a hillside.
    tilt: Quat,
}

impl DragonVisual {
    pub fn new(d: &Dragon) -> Self {
        let rot = d.rotation();
        Self {
            prev_pos: d.pos,
            prev_rot: rot,
            pos: d.pos,
            rot,
            prev_heading: d.heading,
            wing: FOLDED,
            tail: [Vec2::ZERO; TAIL_SEGMENTS],
            neck: Vec2::ZERO,
            upright: 1.0,
            legs: 1.0,
            stride: d.stride,
            roll: 0.0,
            squash: 0.0,
            bonk: f32::MAX,
            time: 0.0,
            was_ground: d.mode == Mode::Ground,
            seed: d.id.0,
            speed: 0.0,
            tilt: Quat::IDENTITY,
        }
    }

    /// Snaps interpolation to the current state, for teleports.
    pub fn snap(&mut self, d: &Dragon) {
        *self = Self::new(d);
    }

    pub fn tick(&mut self, d: &Dragon, p: &FlightParams, world: &World, dt: f32) {
        self.time += dt;
        self.prev_pos = self.pos;
        self.prev_rot = self.rot;
        self.pos = d.pos;
        // Afloat, bobbing gently on the water.
        if d.swimming {
            self.pos.y += (self.time * 2.2).sin() * 0.07;
        }
        self.rot = d.rotation();
        let ease = |rate: f32| 1.0 - (-rate * dt).exp();
        let mut turn_rate = (d.heading - self.prev_heading + PI).rem_euclid(TAU) - PI;
        turn_rate /= dt;
        self.prev_heading = d.heading;

        self.bonk = d.bonk;
        let ground = d.mode == Mode::Ground;
        let feet = d.pos.y - p.stand_height;
        let height = feet - world.support(d.pos.x, d.pos.z, feet);
        let speed = d.speed();
        let landing = !ground && height < 7.0 && d.vel.y < 1.0 && speed < 16.0;

        // Wings.
        let mut want = if ground { FOLDED } else { GLIDE };
        if ground && d.mode_time < 0.35 {
            want = BRAKING.lerp(FOLDED, d.mode_time / 0.35);
        }
        let backflap = d.brake > 0.5;
        if !ground && d.stroke.active && backflap {
            // Back-flaps: wings sweep forward and down, faces angled to push air ahead.
            let ph = d.stroke.phase;
            let s = smooth(if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 });
            want = Wing::new(
                0.9 - 1.3 * s,
                -0.7 + 0.5 * s,
                0.1 + 0.3 * (1.0 - s),
                0.2,
                -0.7,
            );
        } else if !ground && d.stroke.active {
            let down = p.downstroke;
            let ph = d.stroke.phase;
            let (top, bottom) = (1.0, -0.75);
            want = if ph < down {
                let s = smooth(ph / down);
                Wing::new(
                    top + (bottom - top) * s,
                    -0.05,
                    0.05,
                    0.3 * (PI * s).sin(),
                    -0.1,
                )
            } else {
                let s = smooth((ph - down) / (1.0 - down));
                let arc = (PI * s).sin();
                Wing::new(
                    bottom + (top - bottom) * s,
                    0.15 * arc,
                    1.1 * arc,
                    -0.35 * arc,
                    0.2,
                )
            };
        }
        if !ground {
            if landing {
                want = want.lerp(BRAKING, 0.45);
            }
            if !(d.stroke.active && backflap) {
                want = want.lerp(BRAKING, d.brake);
            }
            want = want.lerp(TUCKED, d.tuck);
        }
        let wing_rate = if d.stroke.active { 45.0 } else { 9.0 };
        self.wing = self.wing.lerp(want, ease(wing_rate));

        // Legs drop for landing and stand on the ground.
        let legs = if ground || landing { 1.0 } else { 0.0 };
        self.legs += (legs - self.legs) * ease(6.0);
        self.stride = d.stride;

        self.speed += (speed - self.speed) * ease(2.0);
        // Stand along a hillside, partly: fully level looks like sinking into the
        // slope, fully tilted looks like falling over.
        let tilt = if ground && d.cling.is_none() && !world.on_structure(d.pos.x, d.pos.z, feet) {
            let n = world.ground_normal(d.pos.x, d.pos.z);
            Quat::from_rotation_arc(Vec3::Y, n.lerp(Vec3::Y, 0.35).normalize())
        } else {
            Quat::IDENTITY
        };
        self.tilt = self.tilt.slerp(tilt, ease(6.0));
        // The tail flows: a slow wave runs down it, growing toward the tip, it lags turns
        // and climbs, and wags when standing around.
        let wag = if ground && speed < 1.0 {
            0.16
        } else if ground {
            0.05
        } else {
            0.07
        };
        let droop = if ground { -0.1 } else { 0.02 };
        let lift = if ground {
            0.0
        } else {
            (-d.vel.y * 0.012).clamp(-0.12, 0.12)
        };
        for (i, seg) in self.tail.iter_mut().enumerate() {
            let fi = i as f32;
            let reach = 0.6 + fi * 0.12;
            let yaw = (-turn_rate * 0.1).clamp(-0.28, 0.28)
                + (self.time * 2.0 - fi * 0.75).sin() * wag * reach;
            let pitch = if ground && i >= 4 { 0.1 } else { droop }
                + lift
                + (self.time * 1.3 - fi * 0.6).sin() * 0.035 * reach;
            *seg += (Vec2::new(yaw, pitch) - *seg) * ease((5.0 - fi * 0.4).max(1.6));
        }

        // Neck looks into turns, and around now and then when idle.
        let idle = if ground && speed < 1.0 {
            (self.time * 0.45).sin() * 0.5
        } else {
            0.0
        };
        let look = Vec2::new(
            -d.turn_input * 0.35 + idle,
            if landing { -0.25 } else { 0.0 },
        );
        self.neck += (look - self.neck) * ease(4.0);
        let upright = if ground || landing { 1.0 } else { 0.25 };
        self.upright += (upright - self.upright) * ease(3.0);

        // A happy little barrel roll for tumbles, and a squash on touchdown.
        self.roll = if d.tumble > 0.0 {
            (1.0 - d.tumble / p.tumble_time.max(0.01)) * TAU
        } else {
            0.0
        };
        if ground && !self.was_ground {
            self.squash = 1.0;
        }
        self.was_ground = ground;
        self.squash *= 1.0 - ease(7.0);
    }

    pub fn interpolated(&self, alpha: f32) -> (Vec3, Quat) {
        (
            self.prev_pos.lerp(self.pos, alpha),
            self.prev_rot.slerp(self.rot, alpha),
        )
    }

    /// World transforms for every part.
    pub fn parts(&self, alpha: f32, out: &mut Vec<(Part, Mat4, Vec4)>) {
        let (pos, rot) = self.interpolated(alpha);
        let s = self.squash * (self.time * 18.0).cos().abs();
        // Slow breaths swell the chest.
        let breath = (self.time * 1.6).sin() * 0.015;
        let scale = Vec3::new(
            1.0 + 0.1 * s + breath,
            1.0 - 0.16 * s + breath,
            1.0 + 0.06 * s,
        );
        // Whiskers and feathers stream back as the speed builds, and flutter.
        let trail = (self.speed / 30.0).clamp(0.0, 1.0);
        let flutter = |k: f32| (self.time * (3.0 + trail * 9.0) + k).sin() * (0.05 + trail * 0.05);
        let root = Mat4::from_translation(pos)
            * Mat4::from_quat(self.tilt * rot * Quat::from_rotation_z(self.roll))
            * Mat4::from_scale(scale);
        let white = Vec4::ONE;
        let mut push = |part, m: Mat4| out.push((part, m, white));
        push(Part::Body, root);

        let pitches = [0.55, -0.15, -0.38].map(|p| p * (0.3 + 0.7 * self.upright));
        let mut m = root * Mat4::from_translation(NECK_BASE);
        for (i, base) in pitches.iter().enumerate() {
            let yaw = self.neck.x / 3.0;
            let pitch = base + self.neck.y / 3.0;
            m *= Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(-pitch);
            if i < 2 {
                push(Part::Neck(i), m);
                push(
                    Part::Mane,
                    m * Mat4::from_translation(Vec3::new(0.0, 0.3, NECK_LEN * 0.55))
                        * Mat4::from_rotation_x(-0.25 * trail + flutter(i as f32 * 1.7)),
                );
                m *= Mat4::from_translation(Vec3::Z * NECK_LEN);
            }
        }
        // A slightly big head reads as cute.
        let head = m
            * Mat4::from_translation(Vec3::new(0.0, -0.26, -0.1))
            * Mat4::from_scale(Vec3::splat(1.25));
        push(Part::Head, head);
        push(
            Part::Mane,
            head * Mat4::from_translation(Vec3::new(0.0, 0.52, -0.12))
                * Mat4::from_rotation_x(0.15 - 0.3 * trail + flutter(4.0)),
        );
        for side in [1.0f32, -1.0] {
            let mut w = head
                * Mat4::from_translation(SNOUT * Vec3::new(side, 1.0, 1.0))
                * Mat4::from_rotation_y(side * (1.75 + 0.6 * trail))
                * Mat4::from_rotation_x(0.12 - 0.12 * trail);
            for k in 0..WHISKER_SEGMENTS {
                let wave = (self.time * (2.2 + trail * 6.0) - k as f32 * 1.1 + side).sin();
                // Each piece curls a little further back, so they sweep like ribbons.
                w *= Mat4::from_rotation_y(side * (0.35 + wave * 0.2))
                    * Mat4::from_rotation_x(0.06 * (1.0 - trail) + wave * 0.1);
                push(Part::Whisker, w);
                w *= Mat4::from_translation(Vec3::Z * WHISKER_LEN);
            }
        }
        if self.bonk < 1.4 {
            // Three little stars circle the head, shrinking away.
            let size = 0.3 * (1.0 - self.bonk / 1.4).sqrt();
            let center = head.transform_point3(Vec3::new(0.0, 0.75, 0.35));
            for k in 0..3 {
                let a = self.time * 5.0 + k as f32 * TAU / 3.0;
                let at = center + Vec3::new(a.cos(), 0.15 * (a * 2.0).sin(), a.sin()) * 0.8;
                push(
                    Part::Star,
                    Mat4::from_scale_rotation_translation(
                        Vec3::splat(size),
                        Quat::from_rotation_y(a * 2.0),
                        at,
                    ),
                );
            }
        }
        let blink = {
            let cycle = 3.5 + hash(self.seed) * 2.0;
            if (self.time % cycle) < 0.13 {
                0.12
            } else {
                1.0
            }
        };
        push(
            Part::Eye,
            head * Mat4::from_translation(EYES) * Mat4::from_scale(Vec3::new(1.0, blink, 1.0)),
        );
        push(
            Part::Eye,
            head * Mat4::from_translation(EYES * Vec3::new(-1.0, 1.0, 1.0))
                * Mat4::from_scale(Vec3::new(-1.0, blink, 1.0)),
        );

        let mut m = root * Mat4::from_translation(TAIL_BASE);
        let mut size = TAIL_ROOT;
        for (i, seg) in self.tail.iter().enumerate() {
            m *= Mat4::from_rotation_y(seg.x) * Mat4::from_rotation_x(seg.y);
            push(
                Part::Tail(i),
                m * Mat4::from_scale(Vec3::new(size, size, 1.0)),
            );
            m *= Mat4::from_translation(Vec3::NEG_Z * TAIL_LEN);
            size *= TAIL_TAPER;
        }
        let last = self.tail[TAIL_SEGMENTS - 1];
        push(
            Part::Plume,
            m * Mat4::from_rotation_y(last.x * 1.5 + flutter(2.3))
                * Mat4::from_rotation_x(last.y * 1.5),
        );

        let walk = (self.stride * 1.4).sin();
        for (pair, hip) in HIPS.iter().enumerate() {
            for side in [1.0f32, -1.0] {
                let phase = if (pair == 0) == (side > 0.0) {
                    1.0
                } else {
                    -1.0
                };
                let swing = walk * phase * 0.45 * self.legs;
                let angle = 1.25 * (1.0 - self.legs) + swing;
                push(
                    Part::Leg,
                    root * Mat4::from_translation(*hip * Vec3::new(side, 1.0, 1.0))
                        * Mat4::from_rotation_x(angle),
                );
            }
        }

        let w = self.wing;
        for (i, side) in [1.0f32, -1.0].into_iter().enumerate() {
            let mirror = Vec3::new(side, 1.0, 1.0);
            let arm = root
                * Mat4::from_translation(SHOULDER * mirror)
                * Mat4::from_rotation_z(w.flap * side)
                * Mat4::from_rotation_y(w.sweep * side)
                * Mat4::from_rotation_x(w.twist);
            push(Part::Arm(i), arm);
            let hand = arm
                * Mat4::from_translation(ELBOW * mirror)
                * Mat4::from_rotation_y(w.fold * side)
                * Mat4::from_rotation_z(w.flex * side);
            push(Part::Hand(i), hand);
        }
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
