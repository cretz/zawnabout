//! The chase camera. It trails the flight path, pulls back and widens with speed, and
//! leans a little into banks.

use glam::{Mat4, Quat, Vec2, Vec3};
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
        /// Extra height when hovering or creeping, to see the spot below (m).
        hover_height: f32 = 4.0, [0, 20],
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
    /// A look around from dragging: turn about the dragon (rad) and tilt up or down.
    pub free_look: Vec2,
    dir: Vec3,
    fresh: bool,
    /// How far out along the way to its spot the camera may go, eased back out after a
    /// wall or the ground pulled it in, so it never flickers in and out.
    reach: f32,
    /// The dragon's height on the ground, smoothed, so rough rock underfoot doesn't
    /// shake the view.
    ground_y: f32,
    /// How far the camera has swung round (rad) and up over the dragon to see past a wall
    /// behind it, eased toward `swing_to`.
    swing: Vec2,
    swing_to: Vec2,
    /// Time left until the next look round for the open side. Looking costs rock tests, a
    /// lot of them underground, so it is done a few times a second, not every frame.
    search_in: f32,
    /// A look round under way: the next way to try, and the best found so far.
    search: Option<(usize, f32, Vec2)>,
    /// How clear the view was last frame, along the way the camera actually looked.
    clear: f32,
}

/// How often the camera looks round for the open side while a wall is behind (s).
const SEARCH_EVERY: f32 = 0.2;
/// The ways the camera may swing: round (rad) and up over or down under the dragon.
const SWINGS: [Vec2; 21] = {
    let mut out = [Vec2::ZERO; 21];
    let yaws = [0.0f32, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0];
    let rises = [0.0f32, 0.7, -0.5];
    let mut i = 0;
    while i < 21 {
        out[i] = Vec2::new(yaws[i / 3], rises[i % 3]);
        i += 1;
    }
    out
};
/// How many of them are tried each frame, so no one frame pays for the whole look round.
const SWINGS_PER_FRAME: usize = 3;

/// How far the view from `from` toward `to` stays clear of the ground, rock and
/// anything standing still, as a fraction, and a small lift where moving up helps. Rock is
/// drawn in facets that jut a little inside its true surface, so keep a good margin off
/// it. Next to the dragon the margin is only what the dragon itself has, so a dragon up
/// against a wall still looks away from it. Tested at `steps` points along the way.
fn clear_way(world: &World, from: Vec3, to: Vec3, steps: u32) -> (f32, Vec3) {
    let margin = 1.6f32;
    let near = margin.min(world.terrain.sdf(from) - 0.1).max(0.3);
    let in_building = |q: Vec3| world.support_still(q.x, q.z, q.y + 0.5) > q.y - 0.8;
    // A dragon bumped into a building looks out of it, as it does out of rock.
    let inside = in_building(from);
    let len = (to - from).length();
    for i in 1..=steps {
        let f = i as f32 / steps as f32;
        let q = from.lerp(to, f);
        let close = f * len < 3.0;
        let need = if close { near } else { margin };
        if (!(inside && close) && in_building(q)) || world.terrain.sdf(q) < need {
            let clear = (i - 1) as f32 / steps as f32;
            let back = from.lerp(to, clear);
            let lift = if world.terrain.sdf(back + Vec3::Y * 0.5) >= margin {
                Vec3::Y * 0.5
            } else {
                Vec3::ZERO
            };
            return (clear, lift);
        }
    }
    (1.0, Vec3::ZERO)
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 10.0, -20.0),
            look: Vec3::ZERO,
            fov: 62.0,
            roll: 0.0,
            free_look: Vec2::ZERO,
            dir: Vec3::Z,
            fresh: true,
            reach: 1.0,
            ground_y: 0.0,
            swing: Vec2::ZERO,
            swing_to: Vec2::ZERO,
            search_in: 0.0,
            search: None,
            clear: 1.0,
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
        let mut t = t;
        if t.mode == Mode::Ground && !self.fresh {
            self.ground_y += (t.pos.y - self.ground_y) * ease(6.0);
            self.ground_y = self.ground_y.clamp(t.pos.y - 1.5, t.pos.y + 1.5);
            t.pos.y = self.ground_y;
        } else {
            self.ground_y = t.pos.y;
        }
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

        // Slow flight (hovering, creeping up on a spot) pulls in close and looks down.
        let slow = if t.mode == Mode::Air {
            1.0 - ((speed - 4.0) / 8.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let air = if t.mode == Mode::Air { 1.0 - slow } else { 0.0 };
        let dist = p.ground_distance
            + (p.distance - p.ground_distance) * air
            + speed.min(p.fast_speed) * p.distance_per_speed;
        let back = Quat::from_rotation_y(p.orbit.to_radians() + self.free_look.x) * self.dir;
        // Tilting swings the camera over or under the dragon.
        let side = Vec3::Y.cross(back).normalize_or(Vec3::X);
        // Looking up swings the camera low, then tilts the view up past that, since the
        // ground stops it going under the dragon.
        let tilt = self.free_look.y.max(-0.25);
        let up_more = (-0.25 - self.free_look.y).max(0.0);
        let back = Quat::from_axis_angle(side, -tilt) * back;
        let up = Vec3::Y * (p.height + slow * p.hover_height);
        // Look from just over the dragon's back, or from the dragon itself when its back is
        // up against a roof.
        let from = {
            let over = t.pos + Vec3::Y * 1.0;
            if world.terrain.sdf(over) < world.terrain.sdf(t.pos) {
                t.pos
            } else {
                over
            }
        };
        // When a wall, pillar or bank stands behind the dragon, swing round to the open
        // side, or up over it or down under it, rather than only moving in. Looking
        // around is left alone.
        let looking = (self.free_look.length() * 2.0).min(1.0);
        let swung = |a: Vec2| {
            let side = Vec3::Y.cross(back).normalize_or(Vec3::X);
            Quat::from_rotation_y(a.x) * (Quat::from_axis_angle(side, -a.y) * back)
        };
        // Swung down under the dragon, the camera drops its usual height too.
        let spot = |a: Vec2| t.pos - swung(a) * dist + up * (1.0 + a.y * 2.0).clamp(0.0, 1.0);
        // Not swung, the camera's own view last frame says whether a wall is behind.
        let behind = if self.swing == Vec2::ZERO && !self.fresh {
            self.clear
        } else {
            clear_way(world, from, spot(Vec2::ZERO), 16).0
        };
        self.search_in -= dt;
        let blocked = looking < 0.5 && (behind < 0.999 || self.swing != Vec2::ZERO);
        if !blocked {
            self.swing_to = Vec2::ZERO;
            self.search_in = 0.0;
            self.search = None;
        } else {
            // Clear distance counts most, then staying close to straight behind, then
            // keeping the swing already chosen so it never flips from side to side. Right
            // away when a wall first comes behind, then a few times a second.
            // Hemmed in right by the dragon, the camera would sit on it until the look
            // round ends: then it starts and finishes at once, rare as that is.
            let hemmed = behind * dist < 4.0;
            if self.search.is_none() && (self.search_in <= 0.0 || self.fresh || hemmed) {
                self.search = Some((0, f32::MIN, Vec2::ZERO));
            }
            if let Some((mut next, mut best, mut pick)) = self.search {
                let end = if self.fresh || hemmed {
                    SWINGS.len()
                } else {
                    (next + SWINGS_PER_FRAME).min(SWINGS.len())
                };
                for &a in &SWINGS[next..end] {
                    let c = clear_way(world, from, spot(a), 16).0;
                    let s = c - 0.12 * a.x.abs() - 0.2 * a.y.abs()
                        + if a == self.swing_to { 0.08 } else { 0.0 };
                    if s > best {
                        (best, pick) = (s, a);
                    }
                }
                next = end;
                if next == SWINGS.len() {
                    self.swing_to = pick;
                    self.search = None;
                    self.search_in = SEARCH_EVERY;
                } else {
                    self.search = Some((next, best, pick));
                }
            }
        }
        let want_swing = self.swing_to;
        let hemmed = self.clear * dist < 4.0;
        self.swing += (want_swing - self.swing) * ease(if hemmed { 10.0 } else { 3.0 });
        if self.swing.length() < 0.01 && want_swing == Vec2::ZERO {
            self.swing = Vec2::ZERO;
        }
        let want_pos = spot(self.swing);
        self.pos = self.pos.lerp(want_pos, ease(p.catch_up));
        // Stay out of spires and rocks, and above whatever is underneath.
        // Only what stands still: vehicles move tick by tick, and the view would shake.
        self.pos = world.push_out_still(self.pos, 1.0).0;
        let floor = world.support_still(self.pos.x, self.pos.z, self.pos.y) + 1.2;
        self.pos.y = self.pos.y.max(floor);
        // Whatever still stands in the way after that, move in past it.
        let (clear, lift) = clear_way(world, from, self.pos, 16);
        self.clear = clear;
        // In at once, so walls never hide the dragon; back out gently.
        self.reach = if clear < self.reach || self.fresh {
            clear
        } else {
            self.reach + (clear - self.reach) * ease(2.0)
        };
        if self.reach < 1.0 {
            self.pos = from.lerp(self.pos, self.reach) + lift;
        }
        // Looking around centers on the dragon instead of ahead of it.
        let ahead = if p.orbit == 0.0 {
            p.look_ahead * (1.0 - slow) * (1.0 - looking)
        } else {
            0.0
        };
        self.look = t.pos
            + self.dir * ahead
            + Vec3::Y * (1.0 - 2.0 * slow + up_more.min(1.2).tan() * dist * 1.2);

        let fast = ((speed - 12.0) / (p.fast_speed - 12.0)).clamp(0.0, 1.0);
        self.fov += (p.fov + (p.fov_fast - p.fov) * fast * fast - self.fov) * ease(3.0);
        self.roll += (t.bank * p.roll - self.roll) * ease(4.0);
        self.fresh = false;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let fwd = (self.look - self.pos).normalize_or(Vec3::Z);
        let up = Quat::from_axis_angle(fwd, self.roll) * Vec3::Y;
        let view = glam::camera::rh::view::look_at_mat4(self.pos, self.look, up);
        // Reversed depth (near and far swapped) keeps precision over the whole world.
        glam::camera::rh::proj::directx::perspective(
            self.fov.to_radians(),
            aspect.max(0.1),
            20000.0,
            0.3,
        ) * view
    }
}
