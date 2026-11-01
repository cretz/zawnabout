//! Dragon state and flight. A light aerodynamic model: lift and drag from airspeed and
//! angle of attack, flap strokes as impulses, and a few assists so it stays friendly.
//!
//! Frame: +Y up, body forward is local +Z, body right is local -X. Positive bank rolls
//! the right wing down. Heading 0 faces +Z.

use glam::{Quat, Vec2, Vec3};

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
        body_cd: f32 = 0.025, [0, 0.2],
        /// Drag coefficient of open wings.
        wing_cd: f32 = 0.015, [0, 0.2],
        /// Induced drag per lift coefficient squared.
        induced_cd: f32 = 0.08, [0, 0.5],
        /// Hands-off angle of attack (rad). Sets the relaxed glide speed.
        trim_aoa: f32 = 0.1, [-0.1, 0.4],
        /// Speed (m/s) the dragon settles to with hands off.
        trim_speed: f32 = 17.0, [5, 60],
        /// Angle of attack added per unit of relative overspeed. Pulls out of dives and
        /// drops the nose when slow, like a real glider.
        speed_stability: f32 = 0.15, [0, 1],
        /// Angle of attack added per radian of descent with hands off, so letting go
        /// levels out instead of diving.
        level_assist: f32 = 0.3, [0, 1],
        /// Extra angle of attack at full pitch up (rad).
        aoa_up: f32 = 0.34, [0, 1],
        /// Angle of attack change at full pitch down (rad).
        aoa_down: f32 = 0.35, [0, 1],
        /// How fast the body reaches the commanded angle of attack (1/s).
        aoa_rate: f32 = 5.0, [0.5, 20],
        /// Max bank (rad).
        max_bank: f32 = 1.0, [0.2, 1.5],
        /// How fast bank follows input (1/s).
        bank_rate: f32 = 4.0, [0.5, 20],
        /// Extra lift in banks to hold altitude (0 is none, 1 is full).
        turn_assist: f32 = 0.8, [0, 1],
        /// Max load factor (g) lift can pull.
        max_load: f32 = 3.5, [1, 10],
        /// Speed (m/s) below which turn input yaws directly.
        slow_turn_speed: f32 = 12.0, [0, 40],
        /// Direct yaw rate at a standstill in the air (rad/s).
        slow_turn_rate: f32 = 1.6, [0, 5],
        /// Lift kept with wings tucked.
        tuck_lift: f32 = 0.25, [0, 1],
        /// Wing drag kept with wings tucked.
        tuck_drag: f32 = 0.3, [0, 1],
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
        /// Speed change (m/s) one flap stroke delivers.
        flap_impulse: f32 = 6.0, [0, 20],
        /// Seconds per relaxed flap stroke.
        stroke_time: f32 = 0.6, [0.2, 2],
        /// Fraction of the stroke that is the power downstroke.
        downstroke: f32 = 0.45, [0.2, 0.8],
        /// Upstroke speedup when another flap is queued (mashing flaps harder).
        mash_speedup: f32 = 3.0, [1, 6],
        /// Airspeed (m/s) where flapping stops adding much.
        flap_max_speed: f32 = 38.0, [5, 100],
        /// Flap effectiveness kept at high speed.
        flap_min_eff: f32 = 0.15, [0, 1],
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
        walk_speed: f32 = 7.0, [0, 30],
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
        /// Height (m) where lift starts fading out.
        ceiling: f32 = 420.0, [50, 2000],
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
    /// Distance walked, drives the walk cycle.
    pub stride: f32,
    /// Last tick's turn input, for visuals that lean into turns.
    pub turn_input: f32,
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
            stride: 0.0,
            turn_input: 0.0,
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
        self.turn_input = input.turn;
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
        self.pos.x += flat.x * dt;
        self.pos.z += flat.y * dt;
        self.pos.y = world.ground_height(self.pos.x, self.pos.z) + p.stand_height;
        self.vel = Vec3::new(flat.x, 0.0, flat.y);

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
        let pitch = input.pitch.clamp(-1.0, 1.0);
        let stability = (p.speed_stability * (speed - p.trim_speed) / p.trim_speed)
            .clamp(-0.15, p.stall_aoa * 0.9);
        let mut aoa_cmd = p.trim_aoa
            + stability
            + if pitch > 0.0 {
                pitch * p.aoa_up
            } else {
                pitch * p.aoa_down
            };
        let hands_off = (1.0 - pitch.abs()) * (1.0 - self.tuck);
        aoa_cmd += p.level_assist * (-path).max(0.0) * hands_off;
        aoa_cmd += self.brake * p.brake_aoa;
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
            let height = self.pos.y - p.stand_height - world.ground_height(self.pos.x, self.pos.z);
            let ground_effect = 1.0
                + p.ground_effect
                    * (1.0 - height / p.ground_effect_height.max(0.01)).clamp(0.0, 1.0);
            let thin_air = 1.0 - smoothstep(p.ceiling, p.ceiling + 150.0, self.pos.y);
            let wings = 1.0 - self.tuck * (1.0 - p.tuck_lift) + self.brake * p.brake_lift;
            let assist = 1.0 + (1.0 / self.bank.cos().max(0.35) - 1.0) * p.turn_assist;
            let cl = lift_coefficient(p, aoa);
            let q = p.lift_k * speed * speed;
            let max_lift = p.gravity * p.max_load;
            let lift =
                (q * cl * wings * ground_effect * assist * thin_air).clamp(-max_lift, max_lift);
            let lift_dir = (up - dir * up.dot(dir)).normalize_or_zero();
            let cd = p.body_cd
                + p.wing_cd * (1.0 - self.tuck * (1.0 - p.tuck_drag))
                + p.brake_cd * self.brake
                + p.induced_cd * cl * cl * wings;
            acc += lift_dir * lift - dir * q * cd;
            if self.vel.y < 0.0 {
                acc.y += self.brake * p.brake_sink * self.vel.y * self.vel.y;
            }
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
                let eff =
                    (1.0 - speed / p.flap_max_speed).clamp(p.flap_min_eff, 1.0) * (1.0 - self.tuck);
                let force = p.flap_impulse * std::f32::consts::FRAC_PI_2 / down_secs * shape * eff;
                // Thrust pushes along the heading rather than the pitched-up body, so
                // flapping from slow flight builds speed instead of only climbing.
                let thrust = self.forward_flat() * p.flap_fwd;
                acc += (up * p.flap_up + thrust).normalize_or_zero() * force;
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

        self.vel += acc * dt;
        self.pos += self.vel * dt;

        let ground = world.ground_height(self.pos.x, self.pos.z);
        if self.pos.y - p.stand_height <= ground {
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

    /// A soft invisible breeze turns the dragon back near the edge of the world.
    fn keep_in_bounds(&mut self, world: &World, dt: f32) {
        let limit = world.half_size - 100.0;
        for (pos, vel) in [(self.pos.x, &mut self.vel.x), (self.pos.z, &mut self.vel.z)] {
            let over = pos.abs() - limit;
            if over > 0.0 {
                *vel -= pos.signum() * over * 0.2 * dt;
            }
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
