//! Devices to dragon input. Keyboard, gamepad and touch all feed the same actions, and
//! every binding is data so players can change it.

use glam::Vec2;
use zawnabout_sim::DragonInput;

zawnabout_sim::params! {
    /// Player control preferences.
    pub struct ControlParams in "opt" {
        /// Up means nose down, like a flight stick.
        invert_pitch: bool = false, [0, 1],
        /// A flap press starts or stops continuous flapping.
        flap_toggle: bool = false, [0, 1],
        /// How fast held keys ramp to full turn or pitch (1/s).
        key_ramp: f32 = 4.0, [0.5, 30],
        /// How fast released keys ease back to center (1/s).
        key_return: f32 = 7.0, [0.5, 30],
        /// Gamepad stick deadzone.
        deadzone: f32 = 0.15, [0, 0.6],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    PitchUp,
    PitchDown,
    TurnLeft,
    TurnRight,
    Flap,
    Tuck,
    Brake,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Action::PitchUp,
        Action::PitchDown,
        Action::TurnLeft,
        Action::TurnRight,
        Action::Flap,
        Action::Tuck,
        Action::Brake,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Action::PitchUp => "up",
            Action::PitchDown => "down",
            Action::TurnLeft => "left",
            Action::TurnRight => "right",
            Action::Flap => "flap",
            Action::Tuck => "tuck",
            Action::Brake => "brake",
        }
    }

    pub fn from_name(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.name() == name)
    }
}

/// Key codes are W3C `KeyboardEvent.code` names (`KeyW`, `Space`, `ShiftLeft`), which
/// winit's `KeyCode` debug names also match. Gamepad indices follow the W3C standard
/// gamepad layout.
#[derive(Clone, Debug)]
pub struct Bindings {
    pub keys: Vec<(String, Action)>,
    pub buttons: Vec<(usize, Action)>,
    pub turn_axis: usize,
    pub pitch_axis: usize,
}

impl Default for Bindings {
    fn default() -> Self {
        use Action::*;
        let keys = [
            ("KeyW", PitchUp),
            ("ArrowUp", PitchUp),
            ("KeyS", PitchDown),
            ("ArrowDown", PitchDown),
            ("KeyA", TurnLeft),
            ("ArrowLeft", TurnLeft),
            ("KeyD", TurnRight),
            ("ArrowRight", TurnRight),
            ("Space", Flap),
            ("ShiftLeft", Tuck),
            ("ShiftRight", Tuck),
            ("KeyE", Brake),
            ("KeyX", Brake),
        ];
        let buttons = [
            (0, Flap),
            (1, Brake),
            (4, Brake),
            (6, Brake),
            (5, Tuck),
            (7, Tuck),
            (12, PitchUp),
            (13, PitchDown),
            (14, TurnLeft),
            (15, TurnRight),
        ];
        Self {
            keys: keys.iter().map(|(k, a)| (k.to_string(), *a)).collect(),
            buttons: buttons.to_vec(),
            turn_axis: 0,
            pitch_axis: 1,
        }
    }
}

impl Bindings {
    pub fn key_action(&self, code: &str) -> Option<Action> {
        self.keys.iter().find(|(k, _)| k == code).map(|(_, a)| *a)
    }
}

/// Scripted overrides, used by debug scripts and tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Overrides {
    pub pitch: Option<f32>,
    pub turn: Option<f32>,
    pub flap: Option<bool>,
    pub tuck: Option<bool>,
    pub brake: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct InputState {
    keys_down: Vec<String>,
    pad_axes: Vec<f32>,
    pad_buttons: Vec<f32>,
    pad_flap_was_down: bool,
    touch_stick: Vec2,
    touch_down: Vec<Action>,
    taps: u8,
    toggled_on: bool,
    key_axes: Vec2,
    pub overrides: Overrides,
}

impl InputState {
    pub fn key(
        &mut self,
        code: &str,
        down: bool,
        repeat: bool,
        binds: &Bindings,
        opts: &ControlParams,
    ) {
        if down {
            if !repeat && binds.key_action(code) == Some(Action::Flap) {
                self.press_flap(opts);
            }
            if !self.keys_down.iter().any(|k| k == code) {
                self.keys_down.push(code.to_string());
            }
        } else {
            self.keys_down.retain(|k| k != code);
        }
    }

    /// Releases everything, such as when the window loses focus.
    pub fn clear(&mut self) {
        self.keys_down.clear();
        self.touch_down.clear();
        self.touch_stick = Vec2::ZERO;
        self.pad_axes.clear();
        self.pad_buttons.clear();
    }

    pub fn press_flap(&mut self, opts: &ControlParams) {
        if opts.flap_toggle {
            self.toggled_on = !self.toggled_on;
        } else {
            self.taps = self.taps.saturating_add(1);
        }
    }

    /// Latest gamepad state, or empty slices when none is connected.
    pub fn gamepad(&mut self, axes: &[f32], buttons: &[f32]) {
        self.pad_axes.clear();
        self.pad_axes.extend_from_slice(axes);
        self.pad_buttons.clear();
        self.pad_buttons.extend_from_slice(buttons);
    }

    /// Touch stick in `[-1, 1]`, +Y is up on screen.
    pub fn touch_stick(&mut self, v: Vec2) {
        self.touch_stick = v.clamp_length_max(1.0);
    }

    pub fn touch_action(&mut self, action: Action, down: bool, opts: &ControlParams) {
        if down && !self.touch_down.contains(&action) {
            if action == Action::Flap {
                self.press_flap(opts);
            }
            self.touch_down.push(action);
        } else if !down {
            self.touch_down.retain(|a| *a != action);
        }
    }

    fn down(&self, action: Action, binds: &Bindings) -> bool {
        self.keys_down
            .iter()
            .any(|k| binds.key_action(k) == Some(action))
            || binds.buttons.iter().any(|(b, a)| {
                *a == action && self.pad_buttons.get(*b).copied().unwrap_or(0.0) > 0.5
            })
            || self.touch_down.contains(&action)
    }

    /// Builds this tick's dragon input and consumes queued presses.
    pub fn sample(&mut self, dt: f32, binds: &Bindings, opts: &ControlParams) -> DragonInput {
        let digital = |a: Action| if self.down(a, binds) { 1.0 } else { 0.0 };
        let target = Vec2::new(
            digital(Action::TurnRight) - digital(Action::TurnLeft),
            digital(Action::PitchUp) - digital(Action::PitchDown),
        );
        // Keys ramp so a tap is a nudge and a hold is a full turn.
        for i in 0..2 {
            let (cur, tgt) = (self.key_axes[i], target[i]);
            let rate = if tgt == 0.0 || tgt.signum() != cur.signum() && cur != 0.0 {
                opts.key_return
            } else {
                opts.key_ramp
            };
            let step = rate * dt;
            self.key_axes[i] = if (tgt - cur).abs() <= step {
                tgt
            } else {
                cur + step * (tgt - cur).signum()
            };
        }

        let axis = |i: usize| self.pad_axes.get(i).copied().unwrap_or(0.0);
        let mut stick = Vec2::new(axis(binds.turn_axis), -axis(binds.pitch_axis));
        let len = stick.length();
        stick = if len <= opts.deadzone {
            Vec2::ZERO
        } else {
            stick / len * ((len - opts.deadzone) / (1.0 - opts.deadzone)).min(1.0)
        };

        let pad_flap = binds.buttons.iter().any(|(b, a)| {
            *a == Action::Flap && self.pad_buttons.get(*b).copied().unwrap_or(0.0) > 0.5
        });
        if pad_flap && !self.pad_flap_was_down {
            self.press_flap(opts);
        }
        self.pad_flap_was_down = pad_flap;

        let mut v = (self.key_axes + stick + self.touch_stick).clamp(Vec2::NEG_ONE, Vec2::ONE);
        if opts.invert_pitch {
            v.y = -v.y;
        }
        let held = self.down(Action::Flap, binds);
        let o = self.overrides;
        let input = DragonInput {
            pitch: o.pitch.unwrap_or(v.y),
            turn: o.turn.unwrap_or(v.x),
            flap: o.flap.unwrap_or(if opts.flap_toggle {
                self.toggled_on
            } else {
                held
            }),
            flap_taps: self.taps,
            tuck: o.tuck.unwrap_or(self.down(Action::Tuck, binds)),
            brake: o.brake.unwrap_or(self.down(Action::Brake, binds)),
        };
        self.taps = 0;
        input
    }
}
