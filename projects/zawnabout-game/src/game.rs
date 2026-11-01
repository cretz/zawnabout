//! The client game: one local player driving a dragon in the sim, plus camera, visuals
//! and the debug command language. No GPU here, see [`crate::gfx`].

use std::fmt::Write;

use glam::Vec3;
use zawnabout_sim::{DT, Dragon, EntityId, Mode, ParamDef, Params, Sim};

use crate::camera::{Camera, CameraParams, Target};
use crate::input::{Action, Bindings, ControlParams, InputState};
use crate::pose::DragonVisual;

zawnabout_sim::params! {
    /// How the world looks.
    pub struct LookParams in "look" {
        /// Sun compass angle (degrees).
        sun_yaw: f32 = 35.0, [0, 360],
        /// Sun height above the horizon (degrees).
        sun_height: f32 = 50.0, [2, 90],
        /// Light bands for the toon look (0 is smooth).
        bands: f32 = 4.0, [0, 8],
        /// Fog start (m).
        fog_start: f32 = 300.0, [10, 3000],
        /// Fog end (m).
        fog_end: f32 = 1400.0, [50, 6000],
        /// Blob shadow strength.
        shadow: f32 = 0.5, [0, 1],
    }
}

struct Trace {
    every: f32,
    since: f32,
    csv: String,
}

pub struct Game {
    pub sim: Sim,
    pub player: EntityId,
    pub input: InputState,
    pub bindings: Bindings,
    pub controls: ControlParams,
    pub cam: CameraParams,
    pub look: LookParams,
    pub camera: Camera,
    pub visuals: Vec<(EntityId, DragonVisual)>,
    pub paused: bool,
    pub time: f64,
    acc: f32,
    trace: Option<Trace>,
}

fn list<P: Params>(p: &P, out: &mut Vec<(String, f32, ParamDef)>) {
    for d in P::defs() {
        out.push((
            format!("{}.{}", P::GROUP, d.name),
            p.get(d.name).unwrap_or(0.0),
            *d,
        ));
    }
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut sim = Sim::new(seed);
        let player = sim.spawn_dragon(0.0, 0.0, 0.0);
        let visuals = sim
            .dragons
            .iter()
            .map(|d| (d.id, DragonVisual::new(d)))
            .collect();
        let mut game = Self {
            sim,
            player,
            input: InputState::default(),
            bindings: Bindings::default(),
            controls: ControlParams::default(),
            cam: CameraParams::default(),
            look: LookParams::default(),
            camera: Camera::default(),
            visuals,
            paused: false,
            time: 0.0,
            acc: 0.0,
            trace: None,
        };
        game.update_camera(0.0);
        game
    }

    pub fn player_dragon(&self) -> &Dragon {
        self.sim.dragon(self.player).expect("player dragon exists")
    }

    pub fn visual(&self, id: EntityId) -> Option<&DragonVisual> {
        self.visuals.iter().find(|(v, _)| *v == id).map(|(_, v)| v)
    }

    /// Fraction of a tick since the last one, for interpolating visuals.
    pub fn alpha(&self) -> f32 {
        self.acc / DT
    }

    pub fn key(&mut self, code: &str, down: bool, repeat: bool) {
        self.input
            .key(code, down, repeat, &self.bindings, &self.controls);
    }

    pub fn touch_action(&mut self, action: Action, down: bool) {
        self.input.touch_action(action, down, &self.controls);
    }

    /// Advances by real elapsed time in fixed ticks.
    pub fn advance(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        let dt = dt.clamp(0.0, 0.25);
        self.time += dt as f64;
        self.acc += dt;
        while self.acc >= DT {
            self.acc -= DT;
            self.tick();
        }
        self.update_camera(dt);
    }

    fn tick(&mut self) {
        let input = self.input.sample(DT, &self.bindings, &self.controls);
        self.sim.step(&[(self.player, input)]);
        for d in &self.sim.dragons {
            match self.visuals.iter_mut().find(|(id, _)| *id == d.id) {
                Some((_, v)) => v.tick(d, &self.sim.flight, &self.sim.world, DT),
                None => self.visuals.push((d.id, DragonVisual::new(d))),
            }
        }
        if let Some(t) = &mut self.trace {
            t.since += DT;
            if t.since >= t.every {
                t.since = 0.0;
                let d = self.sim.dragon(self.player).expect("player dragon exists");
                let _ = writeln!(
                    t.csv,
                    "{:.3},{:.2},{:.2},{:.2},{:.2},{:.2},{:.3},{:.3},{:.3},{:.3},{},{},{:.2},{:.2},{:.2},{:.2}",
                    self.sim.tick as f32 * DT,
                    d.pos.x,
                    d.pos.y,
                    d.pos.z,
                    d.speed(),
                    d.vel.y,
                    d.heading,
                    d.path_pitch,
                    d.aoa,
                    d.bank,
                    if d.mode == Mode::Air { "air" } else { "ground" },
                    d.stroke.active as u8,
                    d.tuck,
                    d.brake,
                    input.pitch,
                    input.turn,
                );
            }
        }
    }

    fn update_camera(&mut self, dt: f32) {
        let d = self.player_dragon();
        let pos = self
            .visual(self.player)
            .map(|v| v.interpolated(self.alpha()).0)
            .unwrap_or(d.pos);
        let target = Target {
            pos,
            vel: d.vel,
            heading: d.heading,
            bank: d.bank,
            mode: d.mode,
        };
        self.camera.update(dt, target, &self.sim.world, &self.cam);
    }

    pub fn params(&self) -> Vec<(String, f32, ParamDef)> {
        let mut out = Vec::new();
        list(&self.sim.flight, &mut out);
        list(&self.cam, &mut out);
        list(&self.controls, &mut out);
        list(&self.look, &mut out);
        out
    }

    pub fn set_param(&mut self, full: &str, value: f32) -> bool {
        let Some((group, name)) = full.split_once('.') else {
            return false;
        };
        match group {
            "flight" => self.sim.flight.set(name, value),
            "cam" => self.cam.set(name, value),
            "opt" => self.controls.set(name, value),
            "look" => self.look.set(name, value),
            _ => false,
        }
    }

    /// Every tunable as JSON, for debug panels.
    pub fn params_json(&self) -> String {
        let mut s = String::from("[");
        for (i, (name, value, d)) in self.params().iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let doc = d.doc.trim().replace('\\', "\\\\").replace('"', "\\\"");
            let _ = write!(
                s,
                r#"{{"name":"{name}","value":{value},"min":{},"max":{},"toggle":{},"doc":"{doc}"}}"#,
                d.min, d.max, d.toggle
            );
        }
        s.push(']');
        s
    }

    /// Tunables that differ from defaults, as `set` lines that restore them.
    pub fn dump(&self) -> String {
        let mut base = Vec::new();
        list(&zawnabout_sim::FlightParams::default(), &mut base);
        list(&CameraParams::default(), &mut base);
        list(&ControlParams::default(), &mut base);
        list(&LookParams::default(), &mut base);
        let mut s = String::new();
        for ((name, value, _), (_, default, _)) in self.params().iter().zip(&base) {
            if value != default {
                let _ = writeln!(s, "set {name} {value}");
            }
        }
        s
    }

    pub fn state(&self) -> String {
        let d = self.player_dragon();
        format!(
            "t={:.2} mode={:?} pos=({:.1},{:.1},{:.1}) speed={:.1} vy={:.1} heading={:.2} path={:.2} aoa={:.2} bank={:.2} flap={} tuck={:.2} brake={:.2}",
            self.sim.tick as f32 * DT,
            d.mode,
            d.pos.x,
            d.pos.y,
            d.pos.z,
            d.speed(),
            d.vel.y,
            d.heading,
            d.path_pitch,
            d.aoa,
            d.bank,
            d.stroke.active,
            d.tuck,
            d.brake
        )
    }

    /// Takes the recorded trace as CSV.
    pub fn take_trace(&mut self) -> Option<String> {
        self.trace.take().map(|t| t.csv)
    }

    /// Runs one debug command. Unknown commands come back as `Err`.
    pub fn command(&mut self, line: &str) -> Result<String, String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let num = |i: usize| -> Result<f32, String> {
            words
                .get(i)
                .ok_or_else(|| format!("missing argument {i}"))?
                .parse::<f32>()
                .map_err(|e| e.to_string())
        };
        let o = &mut self.input.overrides;
        match words.as_slice() {
            [] => Ok(String::new()),
            ["set", name, _] => {
                let v = num(2)?;
                if self.set_param(name, v) {
                    Ok(format!("{name} = {v}"))
                } else {
                    Err(format!("unknown param {name}"))
                }
            }
            ["get", name] => self
                .params()
                .into_iter()
                .find(|(n, _, _)| n == name)
                .map(|(n, v, _)| format!("{n} = {v}"))
                .ok_or_else(|| format!("unknown param {name}")),
            ["params"] => Ok(self
                .params()
                .iter()
                .map(|(n, v, d)| format!("{n} = {v}  [{}, {}]  {}", d.min, d.max, d.doc.trim()))
                .collect::<Vec<_>>()
                .join("\n")),
            ["dump"] => Ok(self.dump()),
            ["defaults"] => {
                self.sim.flight = Default::default();
                self.cam = Default::default();
                self.controls = Default::default();
                self.look = Default::default();
                Ok("all tunables back to defaults".into())
            }
            ["state"] => Ok(self.state()),
            ["hold", "pitch", _] => {
                o.pitch = Some(num(2)?);
                Ok(String::new())
            }
            ["hold", "turn", _] => {
                o.turn = Some(num(2)?);
                Ok(String::new())
            }
            ["hold", "flap"] => {
                o.flap = Some(true);
                Ok(String::new())
            }
            ["hold", "tuck"] => {
                o.tuck = Some(true);
                Ok(String::new())
            }
            ["hold", "brake"] => {
                o.brake = Some(true);
                Ok(String::new())
            }
            ["release", what] => {
                match *what {
                    "pitch" => o.pitch = None,
                    "turn" => o.turn = None,
                    "flap" => o.flap = None,
                    "tuck" => o.tuck = None,
                    "brake" => o.brake = None,
                    "all" => *o = Default::default(),
                    _ => return Err(format!("unknown input {what}")),
                }
                Ok(String::new())
            }
            ["tap", "flap", ..] => {
                let n = if words.len() > 2 { num(2)? as u32 } else { 1 };
                for _ in 0..n {
                    self.input.press_flap(&self.controls);
                }
                Ok(String::new())
            }
            ["key", code, state] => {
                self.key(code, *state == "down", false);
                Ok(String::new())
            }
            // spawn x y z heading speed: y is height above ground, 0 stands on the ground.
            ["spawn", ..] => {
                let (x, h, z) = (num(1)?, num(2)?, num(3)?);
                let heading = num(4).unwrap_or(0.0).to_radians();
                let speed = num(5).unwrap_or(0.0);
                self.spawn_player(Vec3::new(x, h, z), heading, speed);
                Ok(self.state())
            }
            ["pause"] => {
                self.paused = true;
                Ok(String::new())
            }
            ["resume"] => {
                self.paused = false;
                Ok(String::new())
            }
            ["step", ..] => {
                let n = if words.len() > 1 { num(1)? as u32 } else { 1 };
                for _ in 0..n {
                    self.tick();
                }
                self.update_camera(DT * n as f32);
                Ok(self.state())
            }
            ["trace", _] => {
                self.trace = Some(Trace {
                    every: num(1)?,
                    since: f32::MAX,
                    csv: "t,x,y,z,speed,vy,heading,path,aoa,bank,mode,stroke,tuck,brake,in_pitch,in_turn\n".into(),
                });
                Ok(String::new())
            }
            ["camera", "reset"] => {
                self.camera.reset();
                Ok(String::new())
            }
            _ => Err(format!("unknown command: {line}")),
        }
    }

    /// Moves the player. `pos.y` is height above the ground, and anything above zero
    /// starts in flight at `speed` along `heading`.
    pub fn spawn_player(&mut self, pos: Vec3, heading: f32, speed: f32) {
        let flight = self.sim.flight.clone();
        let ground = self.sim.world.ground_height(pos.x, pos.z);
        let player = self.player;
        let d = self.sim.dragon_mut(player).expect("player dragon exists");
        *d = Dragon::new(
            player,
            Vec3::new(pos.x, ground + flight.stand_height + pos.y, pos.z),
            heading,
        );
        if pos.y > 0.5 {
            d.mode = Mode::Air;
            d.vel = Vec3::new(heading.sin(), 0.0, heading.cos()) * speed;
        }
        let d = d.clone();
        if let Some((_, v)) = self.visuals.iter_mut().find(|(id, _)| *id == player) {
            v.snap(&d);
        }
        self.camera.reset();
        self.update_camera(0.0);
    }
}
