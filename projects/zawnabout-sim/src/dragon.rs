//! Dragon state and flight. A light aerodynamic model: lift and drag from airspeed and
//! angle of attack, flap strokes as impulses, and a few assists so it stays friendly.
//!
//! Frame: +Y up, body forward is local +Z, body right is local -X. Positive bank rolls
//! the right wing down. Heading 0 faces +Z.

use glam::{Quat, Vec2, Vec3};

use crate::swoop::Swoop;
use crate::world::World;

crate::params! {
    /// Flight and ground movement tunables.
    pub struct FlightParams in "flight" {
        /// Gravity (m/s^2).
        gravity: f32 = 9.81, [2, 25],
        /// Lift and drag scale per (m/s)^2.
        lift_k: f32 = 0.06, [0.01, 0.2],
        /// Lift coefficient per radian of angle of attack.
        cl_slope: f32 = 5.0, [1, 10],
        /// Angle of attack (rad) where the wing stalls.
        stall_aoa: f32 = 0.3, [0.1, 0.8],
        /// Fraction of peak lift kept deep in a stall.
        post_stall: f32 = 0.55, [0, 1],
        /// Drag coefficient of the body.
        body_cd: f32 = 0.011, [0, 0.2],
        /// Drag coefficient of open wings.
        wing_cd: f32 = 0.0075, [0, 0.2],
        /// Induced drag per lift coefficient squared.
        induced_cd: f32 = 0.08, [0, 0.5],
        /// Climb angle (rad) at full pitch up. Pitch input steers the flight path.
        path_up: f32 = 0.85, [0.1, 1.5],
        /// Dive angle (rad) at full pitch down.
        path_down: f32 = 1.1, [0.1, 1.5],
        /// Hands-off glide angle (rad, negative sinks).
        glide_path: f32 = -0.08, [-0.5, 0.2],
        /// Speed (m/s) a hands-off glide settles to.
        trim_speed: f32 = 34.0, [5, 80],
        /// Climb angle per unit of relative overspeed when gliding. Fast glides climb and
        /// slow ones dip, like a real glider.
        speed_stability: f32 = 0.25, [0, 1],
        /// Angle of attack per radian of flight path error.
        path_gain: f32 = 1.4, [0.1, 5],
        /// How fast the body reaches the commanded angle of attack (1/s).
        aoa_rate: f32 = 5.0, [0.5, 20],
        /// Max bank (rad).
        max_bank: f32 = 1.35, [0.2, 1.5],
        /// How fast bank follows input (1/s).
        bank_rate: f32 = 8.0, [0.5, 20],
        /// Extra lift (in g) pulled into the bank while turning, for sharp turns at speed.
        turn_pull: f32 = 5.0, [0, 12],
        /// Extra lift in banks to hold altitude (0 is none, 1 is full).
        turn_assist: f32 = 0.8, [0, 1],
        /// Max load factor (g) lift can pull.
        max_load: f32 = 9.0, [1, 15],
        /// Speed (m/s) below which turn input yaws directly.
        slow_turn_speed: f32 = 12.0, [0, 40],
        /// Direct yaw rate at a standstill in the air (rad/s).
        slow_turn_rate: f32 = 1.6, [0, 5],
        /// Lift kept with wings tucked.
        tuck_lift: f32 = 0.25, [0, 1],
        /// Wing drag kept with wings tucked.
        tuck_drag: f32 = 0.2, [0, 1],
        /// How fast wings tuck and open (1/s).
        tuck_rate: f32 = 5.0, [0.5, 20],
        /// Extra drag coefficient with wings braking.
        brake_cd: f32 = 0.5, [0, 2],
        /// Extra lift fraction with wings braking.
        brake_lift: f32 = 0.35, [0, 2],
        /// Extra angle of attack while braking (rad).
        brake_aoa: f32 = 0.05, [0, 0.6],
        /// Sink drag with wings cupped for landing. Higher means a softer, slower descent.
        brake_sink: f32 = 1.5, [0, 5],
        /// Speed (m/s) above which wingbeats push mostly forward.
        flap_tilt_speed: f32 = 28.0, [5, 80],
        /// Horizontal slowdown when braking (1/s). Lets brake plus flap hover in place.
        hover_damping: f32 = 1.6, [0, 6],
        /// Top speed of a tap-to-land swoop (m/s).
        swoop_speed: f32 = 90.0, [10, 200],
        /// Body radius for bumping into things (m).
        body_radius: f32 = 1.1, [0.3, 3],
        /// Speed change (m/s) one flap stroke delivers.
        flap_impulse: f32 = 21.0, [0, 40],
        /// Flap strength kept while braking, so brake and flap hovers rather than climbs.
        hover_flap: f32 = 0.2, [0, 1.5],
        /// Seconds per relaxed flap stroke.
        stroke_time: f32 = 0.42, [0.2, 2],
        /// Fraction of the stroke that is the power downstroke.
        downstroke: f32 = 0.45, [0.2, 0.8],
        /// Upstroke speedup when another flap is queued (mashing flaps harder).
        mash_speedup: f32 = 3.0, [1, 6],
        /// Airspeed (m/s) where flapping stops adding much.
        flap_max_speed: f32 = 230.0, [5, 300],
        /// Seconds of steady flapping to build a full rush. It kicks in about a third of
        /// the way.
        rush_time: f32 = 7.0, [0.5, 20],
        /// Top speed (m/s) of a full rush, for crossing the world.
        rush_speed: f32 = 320.0, [50, 600],
        /// Flap effectiveness kept at high speed.
        flap_min_eff: f32 = 0.1, [0, 1],
        /// Upward share of flap force.
        flap_up: f32 = 0.75, [0, 2],
        /// Forward share of flap force.
        flap_fwd: f32 = 0.65, [0, 2],
        /// Extra lift fraction right at the ground.
        ground_effect: f32 = 0.25, [0, 1],
        /// Height (m) where ground effect fades out.
        ground_effect_height: f32 = 8.0, [0, 30],
        /// Body center height above the feet when standing (m).
        stand_height: f32 = 1.5, [0.5, 4],
        /// Walk speed (m/s).
        walk_speed: f32 = 9.0, [0, 30],
        /// Steepest ground you can walk up, as the up part of its normal (0.3 is about 72
        /// degrees).
        walk_slope: f32 = 0.3, [0, 1],
        /// Walk acceleration (m/s^2).
        walk_accel: f32 = 14.0, [0, 60],
        /// Walk turn rate (rad/s).
        walk_turn_rate: f32 = 2.4, [0, 8],
        /// Slowdown on the ground when not walking (m/s^2).
        ground_friction: f32 = 7.0, [0, 40],
        /// Upward speed of the takeoff leap (m/s).
        leap_up: f32 = 5.0, [0, 20],
        /// Forward speed of the takeoff leap (m/s).
        leap_fwd: f32 = 5.0, [0, 20],
        /// Sink speed (m/s) above which a landing becomes a tumble.
        tumble_sink: f32 = 9.0, [1, 40],
        /// Seconds a tumble lasts.
        tumble_time: f32 = 1.0, [0, 4],
    }
}

/// Stable identity of an entity across the sim, saves and the network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u32);

/// What one dragon's controller asks for during one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragonInput {
    /// -1 nose down (or walk back) to 1 nose up (or walk forward).
    pub pitch: f32,
    /// -1 left to 1 right.
    pub turn: f32,
    /// Flapping continuously while held.
    pub flap: bool,
    /// Separate flap presses since the last tick. Each queues one stroke.
    pub flap_taps: u8,
    /// Wings tucked for diving.
    pub tuck: bool,
    /// Wings spread wide to slow down and flare.
    pub brake: bool,
    /// Asks for a swoop that lands the feet here. Sent once, steering cancels it.
    pub land_at: Option<Vec3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Ground,
    Air,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stroke {
    pub active: bool,
    /// Phase in `[0, 1)`. The downstroke comes first.
    pub phase: f32,
    pub queued: bool,
}

#[derive(Clone, Debug)]
pub struct Dragon {
    pub id: EntityId,
    /// Body center.
    pub pos: Vec3,
    pub vel: Vec3,
    pub heading: f32,
    /// Climb angle of the flight path (rad).
    pub path_pitch: f32,
    pub aoa: f32,
    pub bank: f32,
    pub mode: Mode,
    pub stroke: Stroke,
    /// 0 open to 1 fully tucked.
    pub tuck: f32,
    /// 0 to 1 braking.
    pub brake: f32,
    /// Seconds of tumble left.
    pub tumble: f32,
    /// Seconds in the current mode.
    pub mode_time: f32,
    /// Builds from 0 to 1 with steady flapping: more thrust, less drag, higher top speed.
    pub rush: f32,
    /// Distance walked, drives the walk cycle.
    pub stride: f32,
    /// Last tick's turn input, for visuals that lean into turns.
    pub turn_input: f32,
    /// Seconds since the last bump into something, for visuals.
    pub bonk: f32,
    /// 1 while flapping, easing to 0 when gliding.
    pub flapping: f32,
    /// A tap-to-land flight in progress.
    pub swoop: Option<Swoop>,
}

impl Dragon {
    pub fn new(id: EntityId, pos: Vec3, heading: f32) -> Self {
        Self {
            id,
            pos,
            vel: Vec3::ZERO,
            heading,
            path_pitch: 0.0,
            aoa: 0.0,
            bank: 0.0,
            mode: Mode::Ground,
            stroke: Stroke::default(),
            tuck: 0.0,
            brake: 0.0,
            tumble: 0.0,
            mode_time: 0.0,
            rush: 0.0,
            stride: 0.0,
            turn_input: 0.0,
            bonk: f32::MAX,
            flapping: 0.0,
            swoop: None,
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.heading)
            * Quat::from_rotation_x(-self.path_pitch)
            * Quat::from_rotation_z(self.bank)
            * Quat::from_rotation_x(-self.aoa)
    }

    pub fn speed(&self) -> f32 {
        self.vel.length()
    }

    fn forward_flat(&self) -> Vec3 {
        Vec3::new(self.heading.sin(), 0.0, self.heading.cos())
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.mode_time = 0.0;
    }

    pub fn step(&mut self, p: &FlightParams, world: &World, input: DragonInput, dt: f32) {
        self.mode_time += dt;
        self.bonk += dt;
        self.turn_input = input.turn;

        if let Some(target) = input.land_at
            && self.swoop.as_ref().is_none_or(|s| s.target != target)
        {
            self.swoop = Some(Swoop::plan(
                world,
                self.pos,
                self.vel,
                target,
                p.stand_height,
            ));
            self.set_mode(Mode::Air);
        }
        let steering = input.pitch.abs() > 0.15
            || input.turn.abs() > 0.15
            || input.flap
            || input.flap_taps > 0
            || input.tuck
            || input.brake;
        if steering && input.land_at.is_none() {
            self.swoop = None;
        }
        if self.swoop.is_some() {
            self.step_swoop(p, dt);
            return;
        }
        let toward = |v: f32, target: f32, rate: f32| v + (target - v) * (rate * dt).min(1.0);
        self.tuck = toward(
            self.tuck,
            if input.tuck && !self.stroke.active {
                1.0
            } else {
                0.0
            },
            p.tuck_rate,
        );
        self.brake = toward(self.brake, if input.brake { 1.0 } else { 0.0 }, p.tuck_rate);

        match self.mode {
            Mode::Ground => self.step_ground(p, world, input, dt),
            Mode::Air => self.step_air(p, world, input, dt),
        }
        self.keep_in_bounds(world, dt);
    }

    fn step_swoop(&mut self, p: &FlightParams, dt: f32) {
        let toward = |v: f32, target: f32, rate: f32| v + (target - v) * (rate * dt).min(1.0);
        let Some(swoop) = &mut self.swoop else { return };
        let (pos, vel, arrived) = swoop.step(dt, p.swoop_speed);
        let (progress, target, rushing, landing) = (
            swoop.progress(),
            swoop.target,
            swoop.speed < p.swoop_speed * 0.95,
            swoop.landing(),
        );
        let before = self.heading;
        self.pos = pos;
        self.vel = vel;
        let speed = vel.length();
        if Vec2::new(vel.x, vel.z).length() > 0.5 {
            self.heading = vel.x.atan2(vel.z);
        }
        let path = if speed > 0.5 {
            (vel.y / speed).asin()
        } else {
            0.0
        };
        self.path_pitch = (path * smoothstep(1.0, 6.0, speed)).clamp(-1.4, 1.4);
        let turn_rate = ((self.heading - before + std::f32::consts::PI)
            .rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI)
            / dt;
        let bank = (-turn_rate * speed / p.gravity)
            .atan()
            .clamp(-p.max_bank, p.max_bank);
        self.bank = toward(self.bank, bank, 6.0);

        // Look the part: flap while speeding up or climbing, tuck in steep drops, and finish
        // with a flurry of fast back-flaps, body pitched up.
        self.brake = toward(self.brake, if landing { 1.0 } else { 0.0 }, p.tuck_rate);
        self.tuck = toward(
            self.tuck,
            if vel.y < -15.0 && !landing { 0.6 } else { 0.0 },
            p.tuck_rate,
        );
        self.aoa = toward(self.aoa, if landing { 1.2 } else { 0.05 }, p.aoa_rate * 1.5);
        let want_flap = (rushing && progress < 0.6) || vel.y > 3.0 || landing;
        if self.stroke.active {
            let quick = if landing { 2.8 } else { 1.0 };
            self.stroke.phase += dt * quick / p.stroke_time.max(0.05);
            if self.stroke.phase >= 1.0 {
                self.stroke = Stroke::default();
            }
        }
        if want_flap && !self.stroke.active {
            self.start_stroke();
        }

        if arrived {
            self.pos = target + Vec3::Y * p.stand_height;
            self.vel = Vec3::ZERO;
            self.swoop = None;
            self.stroke = Stroke::default();
            self.set_mode(Mode::Ground);
        }
    }

    fn start_stroke(&mut self) {
        self.stroke = Stroke {
            active: true,
            phase: 0.0,
            queued: false,
        };
    }

    fn step_ground(&mut self, p: &FlightParams, world: &World, input: DragonInput, dt: f32) {
        let toward = |v: f32, target: f32, rate: f32| v + (target - v) * (rate * dt).min(1.0);
        self.bank = toward(self.bank, 0.0, 10.0);
        self.aoa = toward(self.aoa, 0.0, 10.0);
        self.path_pitch = toward(self.path_pitch, 0.0, 10.0);
        self.stroke.active = false;

        let mut flat = Vec2::new(self.vel.x, self.vel.z);
        if self.tumble > 0.0 {
            self.tumble -= dt;
            flat *= (1.0 - 2.5 * dt).max(0.0);
        } else {
            self.heading -= input.turn * p.walk_turn_rate * dt;
            let fwd = Vec2::new(self.heading.sin(), self.heading.cos());
            let along = flat.dot(fwd);
            let side = flat - fwd * along;
            let target = input.pitch.max(-0.4) * p.walk_speed;
            // Faster than walking means a run-out from a landing: slide down gently.
            let accel = if along.abs() > p.walk_speed && target.abs() <= along.abs() {
                p.ground_friction
            } else if input.pitch.abs() > 0.05 {
                p.walk_accel
            } else {
                p.ground_friction.max(p.walk_accel * 0.5)
            };
            let dv = (target - along).clamp(-accel * dt, accel * dt);
            flat = fwd * (along + dv) + side * (1.0 - 8.0 * dt).max(0.0);
            self.stride += (along + dv).abs() * dt;
        }
        self.rush = 0.0;
        let (old_x, old_z) = (self.pos.x, self.pos.z);
        self.pos.x += flat.x * dt;
        self.pos.z += flat.y * dt;
        // Walk up any slope short of a cliff. Cliffs stop you, the ground never shoves.
        if world.ground_normal(self.pos.x, self.pos.z).y < p.walk_slope
            && world.ground_height(self.pos.x, self.pos.z) > self.pos.y - p.stand_height + 0.3
        {
            (self.pos.x, self.pos.z) = (old_x, old_z);
            flat = Vec2::ZERO;
        }
        let (pushed, n) = world.push_out_props(self.pos + Vec3::Y * 0.4, p.body_radius);
        if n != Vec3::ZERO {
            self.pos.x = pushed.x;
            self.pos.z = pushed.z;
            let n = Vec2::new(n.x, n.z).normalize_or_zero();
            flat -= n * flat.dot(n).min(0.0);
        }
        self.vel = Vec3::new(flat.x, 0.0, flat.y);
        let feet = self.pos.y - p.stand_height;
        let support = world.support(self.pos.x, self.pos.z, feet + 0.6);
        if support < feet - 0.6 {
            // Walked off a ledge.
            self.set_mode(Mode::Air);
            return;
        }
        self.pos.y = support + p.stand_height;

        let wants_off = input.flap_taps > 0 || (input.flap && self.mode_time > 0.4);
        if wants_off && self.tumble <= 0.0 && self.mode_time > 0.15 {
            self.vel += Vec3::Y * p.leap_up + self.forward_flat() * p.leap_fwd;
            self.pos.y += 0.05;
            self.aoa = 0.3;
            self.set_mode(Mode::Air);
            self.start_stroke();
        }
    }

    fn step_air(&mut self, p: &FlightParams, world: &World, input: DragonInput, dt: f32) {
        let toward = |v: f32, target: f32, rate: f32| v + (target - v) * (rate * dt).min(1.0);

        // Flap strokes.
        if input.flap_taps > 0 {
            if self.stroke.active {
                self.stroke.queued = true;
            } else {
                self.start_stroke();
            }
        }
        if input.flap && !self.stroke.active {
            self.start_stroke();
        }
        let flapping = if self.stroke.active { 1.0 } else { 0.0 };
        self.flapping = toward(
            self.flapping,
            flapping,
            if flapping > 0.0 { 4.0 } else { 0.7 },
        );

        let speed = self.vel.length();
        // Heading follows the flight path, except when drifting backward (a stall over the
        // top), where turning around would be wrong.
        let flat = Vec2::new(self.vel.x, self.vel.z);
        let facing = Vec2::new(self.heading.sin(), self.heading.cos());
        if flat.length() > 0.5 && flat.dot(facing) > 0.0 {
            self.heading = self.vel.x.atan2(self.vel.z);
        }
        // At low speed the path angle means little, so the body eases back to level.
        let mut path = if speed > 0.01 {
            (self.vel.y / speed).asin()
        } else {
            0.0
        };
        if flat.dot(facing) < 0.0 {
            path = path.signum() * std::f32::consts::FRAC_PI_2;
        }
        let settle = smoothstep(2.0, 9.0, speed);
        self.path_pitch = (path * settle).clamp(-1.4, 1.4);

        let turn = input.turn.clamp(-1.0, 1.0);
        self.bank = toward(self.bank, turn * p.max_bank, p.bank_rate);
        // Steady flapping builds a rush. Letting go, braking or diving lets it fade.
        if input.flap && self.brake < 0.1 && self.tuck < 0.1 {
            self.rush = (self.rush + dt / p.rush_time.max(0.1)).min(1.0);
        } else {
            self.rush = (self.rush - dt * 0.5).max(0.0);
        }
        // Nothing for the first stretch of flapping, so ordinary flight feels the same.
        let rush = smoothstep(0.3, 1.0, self.rush);
        // Pitch steers the flight path. Hands off, a glide sinks gently and trades speed
        // for height, while flapping holds level so the extra power becomes speed.
        let pitch = input.pitch.clamp(-1.0, 1.0);
        let glide = (p.glide_path + p.speed_stability * (speed - p.trim_speed) / p.trim_speed)
            .clamp(-0.4, 0.3);
        let mut path_cmd = glide * (1.0 - self.flapping);
        path_cmd += if pitch > 0.0 {
            pitch * p.path_up
        } else {
            pitch * p.path_down
        };
        let wings = 1.0 - self.tuck * (1.0 - p.tuck_lift) + self.brake * p.brake_lift;
        let q = p.lift_k * speed * speed;
        // Angle of attack that holds the current path, from the linear part of the lift.
        let hold = p.gravity * path.cos() / (q * p.cl_slope * wings).max(1e-3);
        let aoa_cmd = (hold + p.path_gain * (path_cmd - path)).clamp(-0.3, p.stall_aoa * 1.15)
            + self.brake * p.brake_aoa;
        self.aoa = toward(self.aoa, aoa_cmd, p.aoa_rate);

        // Direct yaw when slow, so hovering and slow flight can still turn.
        if p.slow_turn_speed > 0.0 && speed < p.slow_turn_speed {
            let yaw = -turn * p.slow_turn_rate * (1.0 - speed / p.slow_turn_speed) * dt;
            self.heading += yaw;
            self.vel = Quat::from_rotation_y(yaw) * self.vel;
        }

        let rot = self.rotation();
        let (fwd, up) = (rot * Vec3::Z, rot * Vec3::Y);
        let mut acc = Vec3::NEG_Y * p.gravity;

        if speed > 0.01 {
            let dir = self.vel / speed;
            let aoa = (-dir.dot(up)).atan2(dir.dot(fwd));
            let feet = self.pos.y - p.stand_height;
            let height = feet - world.support(self.pos.x, self.pos.z, feet);
            let ground_effect = 1.0
                + p.ground_effect
                    * (1.0 - height / p.ground_effect_height.max(0.01)).clamp(0.0, 1.0);
            // Lift fades out above the world's sky ceiling.
            let thin_air = 1.0 - smoothstep(world.sky, world.sky + 300.0, self.pos.y);
            let assist = 1.0 + (1.0 / self.bank.cos().max(0.35) - 1.0) * p.turn_assist;
            let cl = lift_coefficient(p, aoa);
            let max_lift = p.gravity * p.max_load;
            let lift =
                (q * cl * wings * ground_effect * assist * thin_air).clamp(-max_lift, max_lift);
            let lift_dir = (up - dir * up.dot(dir)).normalize_or_zero();
            // Tucking streamlines the whole dragon, so a dive outruns flapping.
            let streamline = 1.0 - self.tuck * (1.0 - p.tuck_drag);
            let cd = p.body_cd * streamline
                + p.wing_cd * streamline
                + p.brake_cd * self.brake
                + p.induced_cd * cl * cl * wings;
            acc += lift_dir * lift - dir * q * cd;
            // Pulling into a turn: a sideways force toward the low wing, so turns stay sharp
            // at speed. Sideways only, so the flight path hold never cancels it.
            let side = Vec3::new(lift_dir.x, 0.0, lift_dir.z).normalize_or_zero();
            acc += side * turn.abs() * p.turn_pull * p.gravity * smoothstep(8.0, 25.0, speed);
            if rush > 0.0 {
                // A rush slips through the air, so long flights get really fast.
                acc += dir * q * cd * 0.7 * rush;
            }
            if self.vel.y < 0.0 {
                // Braking drops fast when high and settles softly near the ground, like a
                // bird dropping in and flaring. Flapping while braking always hovers.
                let settle = if input.flap || self.stroke.active {
                    1.0
                } else {
                    0.04 + 0.96 * (1.0 - smoothstep(15.0, 70.0, height))
                };
                acc.y += self.brake * p.brake_sink * settle * self.vel.y * self.vel.y;
            }
            acc.x -= self.vel.x * self.brake * p.hover_damping;
            acc.z -= self.vel.z * self.brake * p.hover_damping;
        }

        if self.stroke.active {
            let down = p.downstroke.clamp(0.05, 0.95);
            let mut rate = 1.0 / p.stroke_time.max(0.05);
            if self.stroke.queued && self.stroke.phase >= down {
                rate *= p.mash_speedup;
            }
            if self.stroke.phase < down {
                // Half-sine over the downstroke integrates to exactly one impulse.
                let down_secs = down * p.stroke_time;
                let shape = (std::f32::consts::PI * self.stroke.phase / down).sin();
                let top = p.flap_max_speed + (p.rush_speed - p.flap_max_speed) * rush;
                let eff = (1.0 - speed / top).clamp(p.flap_min_eff, 1.0) * (1.0 - self.tuck);
                let force = p.flap_impulse * std::f32::consts::FRAC_PI_2 / down_secs
                    * shape
                    * eff
                    * (1.0 - self.brake * (1.0 - p.hover_flap))
                    * (1.0 + 1.5 * rush);
                // Slow strokes lift, fast strokes push forward along the path. Braking
                // turns them into a hover, with pitch nudging forward or back.
                let tilt = smoothstep(4.0, p.flap_tilt_speed, speed) * (1.0 - self.brake);
                let along = if speed > 1.0 {
                    self.vel / speed
                } else {
                    self.forward_flat()
                };
                let thrust = self.forward_flat().lerp(along, tilt) * p.flap_fwd * (1.0 + tilt);
                let lift = up * p.flap_up * (1.0 - 0.8 * tilt);
                let flight = (lift + thrust).normalize_or_zero();
                let hover = (Vec3::Y + self.forward_flat() * pitch * 0.5).normalize();
                acc += flight.lerp(hover, self.brake).normalize_or_zero() * force;
            }
            self.stroke.phase += rate * dt;
            if self.stroke.phase >= 1.0 {
                if self.stroke.queued || input.flap {
                    self.start_stroke();
                } else {
                    self.stroke.active = false;
                    self.stroke.phase = 0.0;
                }
            }
        }

        let feet_before = self.pos.y - p.stand_height;
        self.vel += acc * dt;
        self.pos += self.vel * dt;

        let (pushed, n) = world.push_out(self.pos, p.body_radius);
        if n != Vec3::ZERO {
            self.pos = pushed;
            let into = self.vel.dot(n);
            if into < 0.0 {
                self.vel -= n * into * 1.2;
                if into < -6.0 {
                    // A soft bonk: lose some speed, keep flying.
                    self.vel *= 0.7;
                    self.bonk = 0.0;
                }
            }
        }

        let ground = world.support(self.pos.x, self.pos.z, feet_before + 0.3);
        if self.pos.y - p.stand_height <= ground && input.flap && self.brake < 0.1 {
            // Flapping hard into rising ground skims along it instead of landing.
            self.pos.y = ground + p.stand_height + 0.05;
            let n = world.ground_normal(self.pos.x, self.pos.z);
            let into = self.vel.dot(n);
            if into < 0.0 {
                self.vel -= n * into;
            }
            self.vel.y = self.vel.y.max(2.0);
        } else if self.pos.y - p.stand_height <= ground {
            self.pos.y = ground + p.stand_height;
            if self.vel.y < -p.tumble_sink {
                self.tumble = p.tumble_time;
                self.vel.x *= 0.6;
                self.vel.z *= 0.6;
            }
            self.vel.y = 0.0;
            self.stroke = Stroke::default();
            self.set_mode(Mode::Ground);
        }
    }

    /// A soft breeze turns the dragon back in the band at the edge of the world, and the
    /// air pushes down above the sky ceiling.
    fn keep_in_bounds(&mut self, world: &World, dt: f32) {
        let limit = world.half_size - world.edge;
        for (pos, vel) in [(self.pos.x, &mut self.vel.x), (self.pos.z, &mut self.vel.z)] {
            let over = pos.abs() - limit;
            if over > 0.0 {
                *vel -= pos.signum() * over * 0.05 * dt;
            }
        }
        let over = self.pos.y - world.sky;
        if over > 0.0 {
            self.vel.y -= over * 0.05 * dt;
        }
    }
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear up to the stall, then a gentle fall to `post_stall` of the peak. The negative
/// side is a weaker mirror.
pub fn lift_coefficient(p: &FlightParams, aoa: f32) -> f32 {
    let a = aoa.abs();
    let mag = if a <= p.stall_aoa {
        p.cl_slope * a
    } else {
        let peak = p.cl_slope * p.stall_aoa;
        let t = smoothstep(p.stall_aoa, p.stall_aoa + 0.6, a);
        peak * (1.0 + (p.post_stall - 1.0) * t)
    };
    if aoa >= 0.0 { mag } else { -mag * 0.7 }
}
