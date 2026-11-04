//! The client game: one local player driving a dragon in the sim, plus camera, visuals
//! and the debug command language. No GPU here, see [`crate::gfx`].

use std::fmt::Write;

use glam::{Vec2, Vec3};
use zawnabout_sim::{DT, Dragon, EntityId, Mode, ParamDef, Params, Sim};

use crate::camera::{Camera, CameraParams, Target};
use crate::feedback::{Feedback, FreeCam, Pin, json_str};
use crate::fx::Fx;
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
        prop_range: f32 = 2200.0, [200, 9000],
        /// Trees nearer than this get their full detail mesh (m).
        detail_range: f32 = 450.0, [0, 4000],
        /// Blob shadow strength.
        shadow: f32 = 0.5, [0, 1],
        /// Show rings on nearby landing spots.
        landing_rings: bool = true, [0, 1],
        /// How far away landing rings show (m).
        ring_range: f32 = 450.0, [50, 2000],
        /// Day (0) to night (1) when the day stands still. At night a bright moon lights
        /// the world and windows and lamps glow.
        night: f32 = 0.0, [0, 1],
        /// Seconds for a whole day and night, 0 to stand still with the sun above. Night
        /// takes about a quarter of it.
        day_length: f32 = 1440.0, [0, 7200],
        /// Hour of day at the start.
        start_hour: f32 = 9.5, [0, 24],
    }
}

/// The sun and sky for a moment of the day.
#[derive(Clone, Copy, Debug)]
pub struct Sky {
    /// Toward the light that casts: the sun by day, the moon by night.
    pub light_dir: Vec3,
    /// 0 by day to 1 at night.
    pub night: f32,
    /// 1 when the sun is low and golden, at dawn and dusk.
    pub dusk: f32,
    pub hour: f32,
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
    /// Seconds left before a dragged look eases back.
    look_hold: f32,
    /// Notes mode: frozen world, free camera, pins.
    pub feedback: Feedback,
    pub paused: bool,
    pub time: f64,
    acc: f32,
    trace: Option<Trace>,
    /// Fire and sparks, client only.
    pub fx: Fx,
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
            look_hold: 0.0,
            feedback: Feedback::default(),
            paused: false,
            time: 0.0,
            acc: 0.0,
            trace: None,
            fx: Fx::default(),
        };
        game.update_camera(0.0);
        game
    }

    /// Time of day from the sim's clock, so everyone sees the same sky.
    pub fn sky(&self) -> Sky {
        let l = &self.look;
        let dir = |yaw: f32, pitch: f32| {
            Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            )
        };
        if l.day_length <= 0.0 {
            return Sky {
                light_dir: dir(l.sun_yaw.to_radians(), l.sun_height.to_radians()),
                night: l.night.clamp(0.0, 1.0),
                dusk: 0.0,
                hour: 12.0,
            };
        }
        let secs = (self.sim.tick as f64 * DT as f64) as f32;
        let hour = clock(cycle(l.start_hour) + secs / l.day_length);
        // Up in the east at six, highest at noon, down in the west at six.
        let turn = (hour - 6.0) / 24.0 * std::f32::consts::TAU;
        let elev = turn.sin() * 62f32.to_radians();
        let yaw = std::f32::consts::FRAC_PI_2 + (hour - 6.0) / 12.0 * std::f32::consts::PI;
        let deg = elev.to_degrees();
        let night = smoothstep(3.0, -7.0, deg);
        // The moon rides opposite the sun, a little off so its light slants.
        let light_dir = if deg > -4.0 {
            dir(yaw, elev.max(2f32.to_radians()))
        } else {
            dir(
                yaw + std::f32::consts::PI + 0.4,
                (-elev).max(15f32.to_radians()),
            )
        };
        Sky {
            light_dir,
            night,
            dusk: 1.0 - smoothstep(2.0, 22.0, deg.abs()),
            hour,
        }
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
        self.fx.tick(&self.sim, DT);
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

    /// Looks around by a drag of `dx`, `dy` pixels. The view eases back behind the dragon
    /// a moment after the drag stops.
    pub fn look_drag(&mut self, dx: f32, dy: f32) {
        let f = &mut self.camera.free_look;
        // Turning wraps all the way round, kept within half a turn so it eases back the
        // short way.
        let pi = std::f32::consts::PI;
        f.x = (f.x - dx * 0.006 + pi).rem_euclid(2.0 * pi) - pi;
        f.y = (f.y - dy * 0.004).clamp(-0.6, 1.1);
        self.look_hold = 1.0;
    }

    fn update_camera(&mut self, dt: f32) {
        if self.look_hold > 0.0 {
            self.look_hold -= dt;
        } else {
            self.camera.free_look *= (-2.5 * dt).exp();
        }
        let d = self.player_dragon();
        // Follow the pose as drawn this frame, not the last tick, so turns stay smooth
        // whatever the frame rate.
        let (pos, heading) = match self.visual(self.player) {
            Some(v) => {
                let (pos, rot) = v.interpolated(self.alpha());
                let fwd = rot * Vec3::Z;
                let heading = if fwd.x.abs() + fwd.z.abs() > 0.1 {
                    fwd.x.atan2(fwd.z)
                } else {
                    d.heading
                };
                (pos, heading)
            }
            None => (d.pos, d.heading),
        };
        let target = Target {
            pos,
            vel: d.vel,
            heading,
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
        let mut above = near.y;
        while t < 9000.0 {
            let at = near + dir * t;
            // What stands under the last sample: once a step drops below it, the ray went
            // through it. Asking under this sample would skip a roof it is now inside.
            // Into the side of something taller than the ray: find the face.
            if world.solid_at(at) {
                let back = 1.0 + t * 0.01;
                let mut k = (t - back).max(0.0);
                while k <= t {
                    let q = near + dir * k;
                    if world.solid_at(q) {
                        if world.wall_normal(q).is_some() {
                            return Some(q);
                        }
                        // Ground is landed on where it was hit, so a tap through a cave
                        // mouth lands inside. A building or prop is landed on top of.
                        let reach = if world.terrain.sdf(q) < 0.0 {
                            2.0
                        } else {
                            500.0
                        };
                        let top = world.support(q.x, q.z, q.y + reach);
                        return Some(Vec3::new(q.x, top, q.z));
                    }
                    k += 0.2;
                }
            }
            let floor = world.support(at.x, at.z, above.max(at.y));
            if at.y <= floor {
                // Well below the top means the ray went in through a side: find the face and
                // cling to it if it is a wall.
                if floor - at.y > 2.5 {
                    let back = 1.0 + t * 0.01;
                    let mut k = (t - back).max(0.0);
                    while k < t {
                        let q = near + dir * k;
                        if world.support(q.x, q.z, q.y + 300.0) > q.y + 0.5 {
                            if world.wall_normal(q).is_some() {
                                return Some(q);
                            }
                            break;
                        }
                        k += 0.2;
                    }
                }
                return Some(Vec3::new(at.x, floor, at.z));
            }
            above = at.y;
            t += 1.0 + t * 0.01;
        }
        None
    }

    /// Where to stand on vehicle `i`: toward the bow, clear of cabins and funnels.
    pub fn deck(&self, i: usize) -> Option<Vec3> {
        let t = &self.sim.world.traffic;
        let v = t.vehicles.get(i)?;
        let pose = t.poses[i];
        let a = pose.axis();
        let (along, across) = match v.kind {
            zawnabout_sim::town::Kind::Ferry => (0.36, 0.0),
            // Beside the mast, clear of the sails along the middle.
            zawnabout_sim::town::Kind::Sailboat => (-0.2, 0.3),
            zawnabout_sim::town::Kind::Car
            | zawnabout_sim::town::Kind::Train
            | zawnabout_sim::town::Kind::LiftChair
            | zawnabout_sim::town::Kind::LiftCabin => (0.0, 0.0),
            _ => (0.1, 0.0),
        };
        let p = pose.pos
            + Vec3::new(a.x, 0.0, a.y) * v.size.x * along
            + Vec3::new(-a.y, 0.0, a.x) * v.size.z * across;
        // Chairs and cabins hang below the cable: look down from just above them.
        let above = match v.kind {
            zawnabout_sim::town::Kind::LiftChair | zawnabout_sim::town::Kind::LiftCabin => 0.5,
            _ => 50.0,
        };
        let top = self.sim.world.support(p.x, p.z, p.y + above);
        Some(Vec3::new(p.x, top, p.z))
    }

    /// The world point under normalized screen coordinates (0..1, origin top left): a
    /// boat or plane, a perch when one is near the ray, else the first surface hit. Returns
    /// the spot and how close counts as arriving.
    pub fn pick(&self, nx: f32, ny: f32, aspect: f32) -> Option<(Vec3, f32)> {
        let (near, dir) = self.ray(nx, ny, aspect);
        let world = &self.sim.world;
        let ground = self.surface_hit(near, dir);
        let ground_t = ground.map_or(f32::MAX, |g| (g - near).length());
        let traffic = &world.traffic;
        let vehicle = (0..traffic.vehicles.len())
            .filter_map(|i| {
                let v = &traffic.vehicles[i];
                // Aim at where it stands, which for chairs and cabins is under the cable.
                let c = self.deck(i)? - Vec3::Y * v.size.y * 0.5;
                let t = (c - near).dot(dir);
                let miss = (near + dir * t - c).length();
                // Generous, since they move: half their length plus a bit with distance,
                // and never less than a couple of meters, so small chairs are easy to hit.
                let reach = (v.size.x * 0.5).max(2.5) + t * 0.025;
                (t > 0.0 && t < ground_t + 5.0 && miss < reach).then_some((t, i))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, i)) = vehicle {
            return self.deck(i).map(|p| (p, 3.0));
        }
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
        ground.map(|p| (p, 2.0))
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
        let swoop = self
            .player_dragon()
            .swoop
            .as_ref()
            .map(|s| s.goal(&self.sim.world));
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
            ["lit", id] => self.sim.beacon(id).ok_or("no such beacon")?.1,
            ["riding", _] => {
                d.mode == Mode::Ground
                    && self.sim.world.carrier(d.pos.x, d.pos.z, feet.y) == Some(num(1)? as usize)
            }
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
            "t={:.2} mode={:?} pos=({:.1},{:.1},{:.1}) speed={:.1} vy={:.1} heading={:.2} path={:.2} aoa={:.2} bank={:.2} flap={} tuck={:.2} brake={:.2} breath={:.2}",
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
            d.brake,
            d.breath
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
            // land-at x y z: an exact spot, such as one on a wall.
            ["land-at", _, _, _] => {
                let p = Vec3::new(num(1)?, num(2)?, num(3)?);
                self.land_request = Some(p);
                Ok(format!("flying to {p}"))
            }
            // tap-screen nx ny: a tap at normalized screen coordinates, as on a phone.
            ["tap-screen", _, _] => {
                let aspect = 16.0 / 9.0;
                if self.tap(num(1)?, num(2)?, aspect) {
                    Ok(format!("tapped, landing at {:?}", self.land_request))
                } else {
                    Err("nothing there".into())
                }
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
            ["hold", "breath"] => {
                o.breath = Some(true);
                Ok(String::new())
            }
            ["vehicles", ..] => {
                let n = if words.len() > 1 {
                    num(1)? as usize
                } else {
                    10
                };
                let at = self.player_dragon().pos;
                let t = &self.sim.world.traffic;
                let mut list: Vec<usize> = (0..t.vehicles.len()).collect();
                list.sort_by(|a, b| {
                    t.poses[*a]
                        .pos
                        .distance(at)
                        .total_cmp(&t.poses[*b].pos.distance(at))
                });
                Ok(list
                    .iter()
                    .take(n)
                    .map(|&i| {
                        let p = t.poses[i];
                        format!(
                            "vehicle {i} {:?} at ({:.1},{:.1},{:.1}) heading {:.2} dist={:.0}",
                            t.vehicles[i].kind,
                            p.pos.x,
                            p.pos.y,
                            p.pos.z,
                            p.heading,
                            p.pos.distance(at)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(
                        "
",
                    ))
            }
            // ride n: stands the dragon on top of vehicle n.
            ["land-on", _] => {
                let i = num(1)? as usize;
                self.land_request = Some(self.deck(i).ok_or("no such vehicle")?);
                Ok(String::new())
            }
            ["ride", _] => {
                let i = num(1)? as usize;
                let Vec3 { x, y: top, z } = self.deck(i).ok_or("no such vehicle")?;
                let p = Vec3::new(x, top, z);
                let stand = self.sim.flight.stand_height;
                let d = self
                    .sim
                    .dragon_mut(self.player)
                    .expect("player dragon exists");
                d.pos = Vec3::new(p.x, top + stand, p.z);
                d.vel = Vec3::ZERO;
                d.mode = Mode::Ground;
                d.swoop = None;
                Ok(format!("riding vehicle {i}"))
            }
            // time [hour]: shows the time of day, or jumps to an hour.
            ["time"] => Ok(format!("{:.2}", self.sky().hour)),
            ["time", _] => {
                let want = num(1)?;
                let secs = (self.sim.tick as f64 * DT as f64) as f32;
                let len = self.look.day_length.max(1.0);
                self.look.start_hour = clock(cycle(want) - secs / len);
                Ok(format!("{:.2}", self.sky().hour))
            }
            // light x: lights a beacon by id, or `all` of them, for testing.
            ["light", id] => {
                if *id == "all" {
                    self.sim.lit.iter_mut().for_each(|l| *l = true);
                    return Ok(format!("{} beacons lit", self.sim.lit.len()));
                }
                let (i, _) = self.sim.beacon(id).ok_or("no such beacon")?;
                self.sim.lit[i] = true;
                Ok(format!("lit {id}"))
            }
            ["beacons", ..] => {
                let n = if words.len() > 1 { num(1)? as usize } else { 5 };
                let at = self.player_dragon().pos;
                let mut list: Vec<_> = self.sim.world.beacons.iter().enumerate().collect();
                list.sort_by(|a, b| a.1.flame.distance(at).total_cmp(&b.1.flame.distance(at)));
                Ok(list
                    .iter()
                    .take(n)
                    .map(|(i, b)| {
                        format!(
                            "beacon {} at ({:.1},{:.1},{:.1}) {} dist={:.0}",
                            b.id,
                            b.flame.x,
                            b.flame.y,
                            b.flame.z,
                            if self.sim.lit[*i] { "lit" } else { "dark" },
                            b.flame.distance(at)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            ["release", what] => {
                match *what {
                    "pitch" => o.pitch = None,
                    "turn" => o.turn = None,
                    "flap" => o.flap = None,
                    "tuck" => o.tuck = None,
                    "brake" => o.brake = None,
                    "breath" => o.breath = None,
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

/// Daytime, 5:30 to 19:30, takes this much of each cycle, so nights pass quicker.
const DAYTIME: f32 = 0.72;
const DAWN: f32 = 5.5;
const DUSK: f32 = 19.5;

/// The hour at a fraction of the way round a cycle that starts at dawn.
fn clock(f: f32) -> f32 {
    let f = f.rem_euclid(1.0);
    let hour = if f < DAYTIME {
        DAWN + f / DAYTIME * (DUSK - DAWN)
    } else {
        DUSK + (f - DAYTIME) / (1.0 - DAYTIME) * (24.0 - DUSK + DAWN)
    };
    hour.rem_euclid(24.0)
}

/// How far round a cycle that starts at dawn `hour` is: the inverse of [`clock`].
fn cycle(hour: f32) -> f32 {
    let since = (hour - DAWN).rem_euclid(24.0);
    if since < DUSK - DAWN {
        since / (DUSK - DAWN) * DAYTIME
    } else {
        DAYTIME + (since - (DUSK - DAWN)) / (24.0 - DUSK + DAWN) * (1.0 - DAYTIME)
    }
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
