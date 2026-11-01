//! The client game: one local player driving a dragon in the sim, plus camera, visuals
//! and the debug command language. No GPU here, see [`crate::gfx`].

use std::fmt::Write;

use glam::{Vec2, Vec3};
use zawnabout_sim::{DT, Dragon, EntityId, Mode, ParamDef, Params, Sim};

use crate::camera::{Camera, CameraParams, Target};
use crate::feedback::{Feedback, FreeCam, Pin, json_str};
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
        fog_start: f32 = 900.0, [10, 6000],
        /// Fog end (m).
        fog_end: f32 = 9000.0, [50, 16000],
        /// How far trees, flowers and clouds draw (m).
        prop_range: f32 = 1600.0, [200, 9000],
        /// Trees nearer than this get their full detail mesh (m).
        detail_range: f32 = 450.0, [0, 4000],
        /// Blob shadow strength.
        shadow: f32 = 0.5, [0, 1],
        /// Show rings on nearby landing spots.
        landing_rings: bool = true, [0, 1],
        /// How far away landing rings show (m).
        ring_range: f32 = 450.0, [50, 2000],
    }
}

/// A ring drawn on the ground or a perch.
#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub pos: Vec3,
    pub radius: f32,
    /// True for where a swoop is heading.
    pub destination: bool,
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
    /// A landing spot picked by a tap, sent with the next tick's input.
    pub land_request: Option<Vec3>,
    /// Notes mode: frozen world, free camera, pins.
    pub feedback: Feedback,
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
        // Face north, toward the mountains.
        let spawn = sim.world.terrain.spawn();
        let player = sim.spawn_dragon(spawn.x, spawn.y, std::f32::consts::PI);
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
            land_request: None,
            feedback: Feedback::default(),
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
        if self.feedback.active {
            // The world stays frozen. Only the free camera moves.
            let dt = dt.clamp(0.0, 0.25);
            self.feedback.cam.update(&self.input, dt);
            // Stay above ground and water, so the view never ends up inside rock.
            let p = self.feedback.cam.pos;
            let floor = self.sim.world.support(p.x, p.z, p.y) + 2.0;
            self.feedback.cam.pos.y = p.y.max(floor);
            self.sync_free_cam();
            return;
        }
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
        let mut input = self.input.sample(DT, &self.bindings, &self.controls);
        input.land_at = self.land_request.take();
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

    /// The camera ray through normalized screen coordinates (0..1, origin top left).
    fn ray(&self, nx: f32, ny: f32, aspect: f32) -> (Vec3, Vec3) {
        let inv = self.camera.view_proj(aspect).inverse();
        let (x, y) = (nx * 2.0 - 1.0, 1.0 - ny * 2.0);
        // Reversed depth: 1 is the near plane, 0 the far one.
        let near = inv.project_point3(Vec3::new(x, y, 1.0));
        let dir = (inv.project_point3(Vec3::new(x, y, 0.0)) - near).normalize();
        (near, dir)
    }

    /// The first surface along a ray, water included.
    fn surface_hit(&self, near: Vec3, dir: Vec3) -> Option<Vec3> {
        let world = &self.sim.world;
        let mut t = 0.0;
        while t < 9000.0 {
            let at = near + dir * t;
            let floor = world.support(at.x, at.z, at.y);
            if at.y <= floor {
                return Some(Vec3::new(at.x, floor, at.z));
            }
            t += 1.0 + t * 0.01;
        }
        None
    }

    /// The world point under normalized screen coordinates (0..1, origin top left): a
    /// perch when one is near the ray, else the first surface hit. Returns the spot and how
    /// close counts as arriving.
    pub fn pick(&self, nx: f32, ny: f32, aspect: f32) -> Option<(Vec3, f32)> {
        let (near, dir) = self.ray(nx, ny, aspect);
        let world = &self.sim.world;
        let perch = world
            .perches
            .iter()
            .filter_map(|p| {
                let t = (p.pos - near).dot(dir);
                let miss = (near + dir * t - p.pos).length();
                (t > 0.0 && miss < p.radius + t * 0.04).then_some((t, p))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, p)) = perch {
            return Some((p.pos, p.radius));
        }
        self.surface_hit(near, dir).map(|p| (p, 2.0))
    }

    /// Enters or leaves notes mode. Entering freezes everything and starts the free
    /// camera where the game camera is.
    pub fn set_feedback(&mut self, on: bool) {
        if on == self.feedback.active {
            return;
        }
        self.input.clear();
        self.feedback.draft = None;
        if on {
            let dir = (self.camera.look - self.camera.pos).normalize_or(Vec3::Z);
            self.feedback.cam = FreeCam {
                pos: self.camera.pos,
                yaw: dir.x.atan2(dir.z),
                pitch: dir.y.clamp(-1.0, 1.0).asin(),
                boost: 0.0,
            };
            self.feedback.active = true;
            self.sync_free_cam();
        } else {
            self.feedback.active = false;
            self.camera.reset();
            self.update_camera(0.0);
        }
    }

    fn sync_free_cam(&mut self) {
        let cam = self.feedback.cam;
        self.camera.pos = cam.pos;
        self.camera.look = cam.pos + cam.forward();
        self.camera.roll = 0.0;
    }

    /// Turns the free camera by a mouse or finger drag in pixels.
    pub fn feedback_look(&mut self, dx: f32, dy: f32) {
        self.feedback.cam.look(Vec2::new(-dx, -dy) * 0.004);
        self.sync_free_cam();
    }

    /// Places the free camera exactly, in notes mode: position, yaw and pitch (radians).
    pub fn feedback_view(&mut self, pos: Vec3, yaw: f32, pitch: f32) {
        self.set_feedback(true);
        self.feedback.cam = FreeCam {
            pos,
            yaw,
            pitch,
            boost: 0.0,
        };
        self.sync_free_cam();
    }

    /// Picks the surface under the cursor in notes mode and describes it as JSON:
    /// `spot`, `view` (x, y, z, yaw, pitch) and `near` (named places around it).
    pub fn feedback_pick(&mut self, nx: f32, ny: f32, aspect: f32) -> Option<String> {
        let (near, dir) = self.ray(nx, ny, aspect);
        let spot = self.surface_hit(near, dir)?;
        self.feedback.draft = Some(spot);
        let cam = self.feedback.cam;
        let near: Vec<String> = self
            .sim
            .world
            .terrain
            .describe(spot.x, spot.z)
            .iter()
            .map(|s| json_str(s))
            .collect();
        Some(format!(
            r#"{{"spot":[{:.1},{:.1},{:.1}],"view":[{:.1},{:.1},{:.1},{:.4},{:.4}],"near":[{}]}}"#,
            spot.x,
            spot.y,
            spot.z,
            cam.pos.x,
            cam.pos.y,
            cam.pos.z,
            cam.yaw,
            cam.pitch,
            near.join(",")
        ))
    }

    /// Replaces the drawn pins: positions as x, y, z triples and one done flag each.
    pub fn set_pins(&mut self, xyz: &[f32], done: &[u8]) {
        self.feedback.draft = None;
        self.feedback.pins = xyz
            .as_chunks::<3>()
            .0
            .iter()
            .zip(done)
            .map(|(p, d)| Pin {
                pos: Vec3::new(p[0], p[1], p[2]),
                done: *d != 0,
            })
            .collect();
    }

    /// Taps or clicks the screen: flies to and lands on whatever is there.
    pub fn tap(&mut self, nx: f32, ny: f32, aspect: f32) -> bool {
        match self.pick(nx, ny, aspect) {
            Some((target, _)) => {
                self.land_request = Some(target);
                true
            }
            None => false,
        }
    }

    /// Rings to draw: nearby perches and the swoop destination, or note pins in notes
    /// mode (open ones warm, done ones pale).
    pub fn markers(&self) -> Vec<Marker> {
        let mut out = Vec::new();
        if self.feedback.active {
            let pins = self.feedback.pins.iter().map(|p| (p.pos, !p.done));
            let draft = self.feedback.draft.map(|p| (p, true));
            for (pos, open) in pins.chain(draft) {
                out.push(Marker {
                    pos,
                    radius: 4.0,
                    destination: open,
                });
            }
            return out;
        }
        let at = self.player_dragon().pos;
        if self.look.landing_rings {
            for p in &self.sim.world.perches {
                if p.pos.distance(at) < self.look.ring_range && p.pos.distance(at) > 4.0 {
                    out.push(Marker {
                        pos: p.pos,
                        radius: p.radius,
                        destination: false,
                    });
                }
            }
        }
        let swoop = self.player_dragon().swoop.as_ref().map(|s| s.target);
        if let Some(target) = swoop.or(self.land_request) {
            out.push(Marker {
                pos: target,
                radius: 2.5,
                destination: true,
            });
        }
        out
    }

    /// Evaluates a playbook condition such as `landed` or `near 0 0 0 5`.
    pub fn check(&self, words: &[&str]) -> Result<bool, String> {
        let d = self.player_dragon();
        let feet = d.pos - Vec3::Y * self.sim.flight.stand_height;
        let num = |i: usize| -> Result<f32, String> {
            words
                .get(i)
                .ok_or_else(|| format!("missing argument {i}"))?
                .parse::<f32>()
                .map_err(|e| e.to_string())
        };
        Ok(match words {
            ["landed"] => {
                d.mode == Mode::Ground && d.swoop.is_none() && self.land_request.is_none()
            }
            ["ground"] => d.mode == Mode::Ground,
            ["air"] => d.mode == Mode::Air,
            ["near", ..] => feet.distance(Vec3::new(num(1)?, num(2)?, num(3)?)) <= num(4)?,
            ["on-perch", id] => {
                let p = self
                    .sim
                    .world
                    .perches
                    .iter()
                    .find(|p| p.id == *id)
                    .ok_or("no such perch")?;
                d.mode == Mode::Ground
                    && Vec2::new(feet.x - p.pos.x, feet.z - p.pos.z).length() <= p.radius + 1.0
                    && (feet.y - p.pos.y).abs() < 1.5
            }
            ["speed-above", _] => d.speed() > num(1)?,
            ["speed-below", _] => d.speed() < num(1)?,
            ["height-above", _] => {
                feet.y - self.sim.world.support(feet.x, feet.z, feet.y) > num(1)?
            }
            _ => return Err(format!("unknown condition: {}", words.join(" "))),
        })
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
            // land-at x z: fly to the top surface at (x, z) and land.
            ["land-at", _, _] => {
                let (x, z) = (num(1)?, num(2)?);
                let y = self.sim.world.support(x, z, 1.0e4);
                self.land_request = Some(Vec3::new(x, y, z));
                Ok(format!("flying to ({x:.1},{y:.1},{z:.1})"))
            }
            ["land-perch", id] => {
                let p = *self
                    .sim
                    .world
                    .perches
                    .iter()
                    .find(|p| p.id == *id)
                    .ok_or("no such perch")?;
                self.land_request = Some(p.pos);
                Ok(format!("flying to perch {id}"))
            }
            ["perches", ..] => {
                let n = if words.len() > 1 { num(1)? as usize } else { 5 };
                let at = self.player_dragon().pos;
                let mut list: Vec<_> = self.sim.world.perches.iter().collect();
                list.sort_by(|a, b| a.pos.distance(at).total_cmp(&b.pos.distance(at)));
                Ok(list
                    .iter()
                    .take(n)
                    .map(|p| {
                        format!(
                            "perch {} at ({:.1},{:.1},{:.1}) r={:.1} dist={:.0}",
                            p.id,
                            p.pos.x,
                            p.pos.y,
                            p.pos.z,
                            p.radius,
                            p.pos.distance(at)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
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
            // place x y z heading: in the air at an exact spot, such as inside a cave.
            ["place", ..] => {
                let pos = Vec3::new(num(1)?, num(2)?, num(3)?);
                let heading = num(4).unwrap_or(0.0).to_radians();
                let player = self.player;
                let d = self.sim.dragon_mut(player).expect("player dragon exists");
                *d = Dragon::new(player, pos, heading);
                d.mode = Mode::Air;
                let d = d.clone();
                if let Some((_, v)) = self.visuals.iter_mut().find(|(id, _)| *id == player) {
                    v.snap(&d);
                }
                self.camera.reset();
                self.update_camera(0.0);
                Ok(self.state())
            }
            // goto x z height heading: flying at height over the ground, for looking around.
            ["goto", ..] => {
                let (x, z, h) = (num(1)?, num(2)?, num(3).unwrap_or(60.0));
                let heading = num(4).unwrap_or(180.0).to_radians();
                self.spawn_player(Vec3::new(x, h, z), heading, 25.0);
                Ok(self.state())
            }
            // goto-mark id height: same, over a named place from the world files.
            ["goto-mark", id, ..] => {
                let m = self
                    .sim
                    .world
                    .terrain
                    .def
                    .marks
                    .iter()
                    .find(|m| m.id == *id)
                    .ok_or("no such mark")?;
                let h = num(2).unwrap_or(60.0);
                let heading = num(3).unwrap_or(180.0).to_radians();
                self.spawn_player(Vec3::new(m.pos[0], h, m.pos[2]), heading, 25.0);
                Ok(self.state())
            }
            // notes on|off: notes mode, everything frozen with a free camera.
            ["notes", state] => {
                self.set_feedback(*state == "on");
                Ok(String::new())
            }
            // view x y z yaw pitch: the free camera exactly, as saved in a note.
            ["view", ..] => {
                let pos = Vec3::new(num(1)?, num(2)?, num(3)?);
                self.feedback_view(pos, num(4)?, num(5)?);
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
        let ground = self.sim.world.support(pos.x, pos.z, 1.0e4);
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
