//! The chase camera. It trails the flight path, pulls back and widens with speed, and
//! leans a little into banks.

use glam::{Mat4, Quat, Vec3};
use zawnabout_sim::{Mode, World};

zawnabout_sim::params! {
    /// Chase camera tunables.
    pub struct CameraParams in "cam" {
        /// Distance behind in flight (m).
        distance: f32 = 13.0, [3, 60],
        /// Extra distance per m/s of speed.
        distance_per_speed: f32 = 0.1, [0, 1],
        /// Height above the dragon (m).
        height: f32 = 3.2, [-5, 20],
        /// How far ahead of the dragon to look (m).
        look_ahead: f32 = 6.0, [0, 40],
        /// How fast the camera swings behind the flight direction (1/s).
        follow: f32 = 2.6, [0.2, 20],
        /// How fast the camera position catches up (1/s).
        catch_up: f32 = 9.0, [0.5, 40],
        /// How much the camera tilts with the climb or dive.
        tilt: f32 = 0.55, [0, 1],
        /// Field of view at rest (degrees).
        fov: f32 = 62.0, [30, 110],
        /// Field of view at full speed (degrees).
        fov_fast: f32 = 80.0, [30, 120],
        /// Speed where the view is widest (m/s).
        fast_speed: f32 = 55.0, [10, 150],
        /// How much the camera leans into banks.
        roll: f32 = 0.22, [0, 1],
        /// Swings the camera around the dragon (degrees). For looking around and screenshots.
        orbit: f32 = 0.0, [-180, 180],
        /// Distance behind when on the ground (m).
        ground_distance: f32 = 10.0, [3, 60],
    }
}

/// What the camera needs to know about its target, already interpolated.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub pos: Vec3,
    pub vel: Vec3,
    pub heading: f32,
    pub bank: f32,
    pub mode: Mode,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub look: Vec3,
    pub fov: f32,
    pub roll: f32,
    dir: Vec3,
    fresh: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 10.0, -20.0),
            look: Vec3::ZERO,
            fov: 62.0,
            roll: 0.0,
            dir: Vec3::Z,
            fresh: true,
        }
    }
}

impl Camera {
    /// Snaps behind the target on the next update.
    pub fn reset(&mut self) {
        self.fresh = true;
    }

    pub fn update(&mut self, dt: f32, t: Target, world: &World, p: &CameraParams) {
        let ease = |rate: f32| {
            if self.fresh {
                1.0
            } else {
                1.0 - (-rate * dt).exp()
            }
        };
        let speed = t.vel.length();
        let flat = Vec3::new(t.heading.sin(), 0.0, t.heading.cos());
        let mut want = flat;
        if t.mode == Mode::Air && speed > 3.0 {
            // Climbs tilt less than dives so the camera never drops below the dragon.
            let slope = (t.vel.y / speed).clamp(-0.95, 0.95);
            let climb = slope * p.tilt * if slope > 0.0 { 0.35 } else { 1.0 };
            want = (flat * (1.0 - climb * climb).sqrt() + Vec3::Y * climb).normalize();
        }
        self.dir = self.dir.lerp(want, ease(p.follow)).normalize_or(want);

        let air = if t.mode == Mode::Air { 1.0 } else { 0.0 };
        let dist = p.ground_distance
            + (p.distance - p.ground_distance) * air
            + speed * p.distance_per_speed;
        let back = Quat::from_rotation_y(p.orbit.to_radians()) * self.dir;
        let want_pos = t.pos - back * dist + Vec3::Y * p.height;
        self.pos = self.pos.lerp(want_pos, ease(p.catch_up));
        let floor = world.ground_height(self.pos.x, self.pos.z) + 1.2;
        self.pos.y = self.pos.y.max(floor);
        let ahead = if p.orbit == 0.0 { p.look_ahead } else { 0.0 };
        self.look = t.pos + self.dir * ahead + Vec3::Y * 1.0;

        let fast = ((speed - 12.0) / (p.fast_speed - 12.0)).clamp(0.0, 1.0);
        self.fov += (p.fov + (p.fov_fast - p.fov) * fast * fast - self.fov) * ease(3.0);
        self.roll += (t.bank * p.roll - self.roll) * ease(4.0);
        self.fresh = false;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let fwd = (self.look - self.pos).normalize_or(Vec3::Z);
        let up = Quat::from_axis_angle(fwd, self.roll) * Vec3::Y;
        let view = glam::camera::rh::view::look_at_mat4(self.pos, self.look, up);
        glam::camera::rh::proj::directx::perspective(
            self.fov.to_radians(),
            aspect.max(0.1),
            0.3,
            6000.0,
        ) * view
    }
}
