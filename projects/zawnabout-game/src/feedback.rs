//! Notes mode, the level design feedback tool. Everything freezes and a free camera flies
//! anywhere. Clicking a surface describes the spot (position, view, nearby named places)
//! so the host can save a note there. The host owns storage, this only knows pins.

use glam::{Vec2, Vec3};

use crate::input::InputState;

/// A camera that flies freely, ignoring the world.
#[derive(Clone, Copy, Debug, Default)]
pub struct FreeCam {
    pub pos: Vec3,
    /// Radians, 0 looking toward +z like dragon heading.
    pub yaw: f32,
    pub pitch: f32,
    /// Builds up while moving, so long trips get fast.
    pub boost: f32,
}

impl FreeCam {
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(sy * cp, sp, cy * cp)
    }

    /// Looks around by an amount in radians.
    pub fn look(&mut self, d: Vec2) {
        self.yaw += d.x;
        self.pitch = (self.pitch + d.y).clamp(-1.5, 1.5);
    }

    pub fn update(&mut self, input: &InputState, dt: f32) {
        let held = |codes: &[&str]| codes.iter().any(|c| input.held(c));
        let axis = |neg: &[&str], pos: &[&str]| held(pos) as i32 as f32 - held(neg) as i32 as f32;
        let (stick, look_stick, rise) = input.free_axes();
        let ahead = axis(&["KeyS"], &["KeyW"]) + stick.y;
        let side = axis(&["KeyA"], &["KeyD"]) + stick.x;
        let up = axis(&["ShiftLeft", "ShiftRight"], &["Space"]) + rise;
        let turn = axis(&["ArrowLeft"], &["ArrowRight"]) + look_stick.x;
        let tilt = axis(&["ArrowDown"], &["ArrowUp"]) + look_stick.y;
        self.look(Vec2::new(-turn, tilt) * 1.6 * dt);
        let fwd = self.forward();
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or(Vec3::Z);
        // Screen right, with the camera looking along `flat`.
        let right = Vec3::new(-flat.z, 0.0, flat.x);
        let dir = fwd * ahead + right * side + Vec3::Y * up;
        if dir.length_squared() > 0.01 {
            self.boost = (self.boost + dt).min(4.0);
            let speed = 25.0 * (1.0 + self.boost * self.boost * 1.5);
            self.pos += dir.normalize() * speed * dt;
        } else {
            self.boost = 0.0;
        }
    }
}

/// A saved note's place, for drawing.
#[derive(Clone, Copy, Debug)]
pub struct Pin {
    pub pos: Vec3,
    pub done: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Feedback {
    pub active: bool,
    pub cam: FreeCam,
    pub pins: Vec<Pin>,
    /// Where the note being written goes.
    pub draft: Option<Vec3>,
}

/// Escapes a string for a JSON string literal.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
