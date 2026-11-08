//! The client game: one local player driving a dragon in the sim, plus camera, visuals
//! and the debug command language. No GPU here, see [`crate::gfx`].

use std::fmt::Write;

use glam::{Vec2, Vec3};
use zawnabout_sim::{DT, Dragon, EntityId, Mode, ParamDef, Params, Sim};

use crate::book::{Do, HoardView, View};
use crate::camera::{Camera, CameraParams, Target};
use crate::feedback::{Feedback, FreeCam, Pin, json_str};
use crate::fx::Fx;
use crate::hud::{HudKey, NotesKey};
use crate::input::{Action, Bindings, ControlParams, InputState};
use crate::notes::Note;
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

/// Something for the host to do for the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Gameplay changed: write [`Game::save_text`] to the save slot.
    Save,
    /// Sound, pausing or the journal's page changed: keep [`Game::prefs_text`].
    Prefs,
    /// The keys changed: keep [`Bindings::to_text`].
    Keys,
    /// Offer [`Game::save_text`] as a file to keep.
    Export,
    /// Ask for a save file and hand it to [`Game::load_save`].
    Import,
    /// Open the game's source code page, [`crate::book::SOURCE`].
    Source,
}

/// Someone to talk to, close by the dragon standing on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Listener {
    /// A named local, by index.
    Local(usize, Vec3),
    /// A townsperson walking by, with the number that is theirs.
    Folk(Vec3, u32),
    /// Someone a finished set brought, by their number among its effects.
    Guest(usize, Vec3),
    /// A named dragon, by its number.
    Kin(usize, Vec3),
}

impl Listener {
    /// Where their bubble floats.
    pub fn bubble(&self) -> Vec3 {
        match self {
            Listener::Local(_, p) | Listener::Folk(p, _) | Listener::Guest(_, p) => {
                *p + Vec3::Y * 2.6
            }
            Listener::Kin(_, p) => *p + Vec3::Y * 4.0,
        }
    }
}

/// How near the dragon must stand to talk to someone.
pub const TALK_REACH: f32 = 9.0;

/// A postcard waiting for its picture: the game shows the view from a postcard camera
/// for a frame with nothing over it, and the host hands that frame back.
#[derive(Clone, Debug)]
pub struct Snap {
    pub card: usize,
    /// The camera to put back after.
    camera: Camera,
    /// Frames waited, to give up on a host that never sends one.
    waited: u32,
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
    /// The game's own heads-up display, off where the host draws its own.
    pub hud: crate::hud::Hud,
    /// Sound turned off with U.
    pub muted: bool,
    pub controls: ControlParams,
    pub cam: CameraParams,
    pub look: LookParams,
    pub camera: Camera,
    pub visuals: Vec<(EntityId, DragonVisual)>,
    /// A landing spot picked by a tap, sent with the next tick's input.
    pub land_request: Option<Vec3>,
    /// Seconds left before a dragged look eases back.
    look_hold: f32,
    /// Look keys held: I up, K down, J left, L right.
    look_keys: [bool; 4],
    /// Notes mode: frozen world, free camera, pins.
    pub feedback: Feedback,
    pub paused: bool,
    pub time: f64,
    acc: f32,
    trace: Option<Trace>,
    /// Fire and sparks, client only.
    pub fx: Fx,
    /// Gameplay on top of the islands. None when it is compiled out or switched off.
    pub play: Option<crate::play::Play>,
    /// Played by touch, so help shows the touch controls.
    pub touch: bool,
    /// Leaving the window opens the journal, which pauses.
    pub pause_away: bool,
    /// The day held at one hour, set in the journal. None lets it pass.
    pub hold_hour: Option<f32>,
    /// Things for the host to do, taken each frame.
    pub asks: Vec<Ask>,
    /// Gamepad buttons held as of the last poll, to spot presses.
    pad_was: Vec<bool>,
    /// Postcards waiting for their picture, the first being taken now.
    pub snaps: Vec<Snap>,
    /// The townsperson nearest the dragon, found while drawing them: where, and who.
    pub near_folk: std::cell::Cell<Option<(Vec3, u32)>>,
    /// Anyone else drawn nearby, as of the last frame: fishers on the piers, shoppers at
    /// a market, people on a boat's deck. Who is a number from where they are.
    pub near_drawn: std::cell::Cell<Option<(Vec3, u32)>>,
    /// Townsfolk talked to so far, so lines change.
    chats: u32,
    /// How far to look for the nearest townsperson. Scripts widen it to find someone.
    pub folk_search: f32,
    /// A townsperson's small ask on the card showing now, to take on if accepted.
    chore_offer: Option<crate::play::Chore>,
    /// The passenger's chatter: seconds till they next say something, and what they
    /// have already pointed out this ride.
    chatter: f32,
    pointed: Vec<Vec3>,
    /// The other dragons and the hatchling, flying and resting.
    pub flock: crate::kin::Flock,
    /// Who the named dragons are, from the play layer, once there is one.
    pub kin: Vec<crate::kin::KinInfo>,
    /// Times a dragon has been talked to, to pick its line.
    kin_talks: u32,
    /// The racer being raced, if one is.
    racing: Option<usize>,
    /// Fire on the egg so far (s), time flown with the hatchling (s), and fire breathed
    /// beside it (s).
    warm: f32,
    together: f32,
    fired: f32,
    /// A gift was looked for on this visit to the lair.
    gift_looked: bool,
    /// Lines waiting to show at the bottom, one after another: who or what, and the line.
    toasts: Vec<(String, String)>,
    /// How big the map in the atlas is, once drawn, for the journal's map page.
    pub map_px: std::cell::Cell<f32>,
    /// What the passenger is pointing at, and for how many seconds more.
    pub point_at: Option<(Vec3, f32)>,
    /// Passengers who hopped off in the air, floating down under their umbrellas.
    pub floaters: Vec<Floater>,
}

/// A passenger floating down under an umbrella: where they let go, how long ago, and
/// how they look.
#[derive(Clone, Debug)]
pub struct Floater {
    pub from: Vec3,
    pub age: f32,
    pub look: u32,
    /// Where they touch down.
    pub ground: f32,
}

impl Floater {
    /// How fast they drift down (m/s).
    pub const SINK: f32 = 3.5;

    /// Where they are now, and whether they have landed yet.
    pub fn at(&self) -> (Vec3, bool) {
        let y = self.from.y - self.age * Self::SINK;
        let drift = Vec3::new((self.age * 0.7).sin() * 1.5, 0.0, self.age * 0.6);
        (
            Vec3::new(self.from.x, y.max(self.ground), self.from.z) + drift,
            y <= self.ground,
        )
    }
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
        let play = crate::play::start(&sim.world);
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
            hud: crate::hud::Hud::default(),
            muted: false,
            controls: ControlParams::default(),
            cam: CameraParams::default(),
            look: LookParams::default(),
            camera: Camera::default(),
            visuals,
            land_request: None,
            look_hold: 0.0,
            look_keys: [false; 4],
            feedback: Feedback::default(),
            paused: false,
            time: 0.0,
            acc: 0.0,
            trace: None,
            fx: Fx::default(),
            play,
            touch: false,
            pause_away: true,
            hold_hour: None,
            asks: Vec::new(),
            pad_was: Vec::new(),
            snaps: Vec::new(),
            near_folk: std::cell::Cell::new(None),
            near_drawn: std::cell::Cell::new(None),
            chats: 0,
            folk_search: TALK_REACH,
            chore_offer: None,
            chatter: 4.0,
            pointed: Vec::new(),
            flock: crate::kin::Flock::default(),
            kin: Vec::new(),
            kin_talks: 0,
            racing: None,
            warm: 0.0,
            together: 0.0,
            fired: 0.0,
            gift_looked: false,
            toasts: Vec::new(),
            map_px: std::cell::Cell::new(0.0),
            point_at: None,
            floaters: Vec::new(),
        };
        game.update_camera(0.0);
        game
    }

    /// Time of day from the sim's clock, so everyone sees the same sky.
    /// Whether it is dark enough for the things that only come out at night.
    fn sky_night(&self) -> bool {
        self.sky().night > 0.5
    }

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
        let hour = self
            .hold_hour
            .unwrap_or_else(|| clock(cycle(l.start_hour) + secs / l.day_length));
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

    /// Lets go of every key, as when the window loses focus.
    pub fn release_keys(&mut self) {
        self.input.clear();
        self.look_keys = [false; 4];
    }

    /// Offers a key press, and the text it types, to the game's own screens first: a
    /// card, the journal, the big map, and notes mode (N) with its list and the note
    /// being written. Pass means the key is for flying.
    pub fn ui_key(&mut self, code: &str, text: Option<&str>) -> HudKey {
        if self.feedback.active {
            if !self.hud.on {
                return HudKey::Pass;
            }
            let used = match self.hud.notes_key(code, text) {
                NotesKey::Leave => {
                    self.set_feedback(false);
                    HudKey::Used
                }
                NotesKey::Go(v) => {
                    self.feedback_view(Vec3::new(v[0], v[1], v[2]), v[3], v[4]);
                    HudKey::Used
                }
                NotesKey::Used => HudKey::Used,
                NotesKey::Pass => HudKey::Pass,
            };
            self.sync_pins();
            return used;
        }
        if self.hud.open() {
            return self.hud.key(code);
        }
        let hoard = self.hoard_view();
        let view = View {
            hoard: hoard.as_ref(),
            binds: &self.bindings,
            muted: self.muted,
            pause_away: self.pause_away,
            hold_hour: self.hold_hour,
            touch: self.touch,
            hud: self.hud.on,
            map: self.map_view(),
        };
        let was = self.hud.book.showing();
        if let Some(dos) = self.hud.book.key(code, &view) {
            self.apply(dos, was);
            return HudKey::Used;
        }
        if matches!(code, "Enter" | "NumpadEnter" | "KeyT") && self.talk() {
            return HudKey::Used;
        }
        if !self.hud.on {
            return HudKey::Pass;
        }
        if code == "KeyN" {
            self.set_feedback(true);
            return HudKey::Used;
        }
        self.hud.key(code)
    }

    /// Whether beacon `id` burns.
    fn lit(&self) -> impl Fn(&str) -> bool + '_ {
        |id: &str| self.sim.beacon(id).is_some_and(|(_, lit)| lit)
    }

    /// Who the dragon could talk to now: someone close by while it stands on the ground.
    /// None with gameplay off, flying, or with the journal or a card up.
    pub fn listener(&self) -> Option<Listener> {
        let play = self.play.as_ref()?;
        let d = self.player_dragon();
        if d.mode != Mode::Ground || self.halted() || self.feedback.active {
            return None;
        }
        if let Some((i, f)) = self
            .flock
            .dragons
            .iter()
            .enumerate()
            .filter(|(i, f)| crate::play::kin_met(play, *i) && f.settled())
            .min_by(|a, b| {
                a.1.d
                    .pos
                    .distance(d.pos)
                    .total_cmp(&b.1.d.pos.distance(d.pos))
            })
            && f.d.pos.distance(d.pos) < TALK_REACH * 1.6
        {
            return Some(Listener::Kin(i, f.d.pos));
        }
        if let Some((i, p)) = crate::play::nearest_local(play, d.pos, TALK_REACH) {
            return Some(Listener::Local(i, p));
        }
        if let Some((k, p)) = crate::play::guest_near(play, d.pos, TALK_REACH) {
            return Some(Listener::Guest(k, p));
        }
        let dist = |(p, _): &(Vec3, u32)| (p.x - d.pos.x).hypot(p.z - d.pos.z);
        let (p, seed) = [self.near_folk.get(), self.near_drawn.get()]
            .into_iter()
            .flatten()
            .filter(|f| dist(f) < TALK_REACH)
            .min_by(|a, b| dist(a).total_cmp(&dist(b)))?;
        Some(Listener::Folk(p, seed))
    }

    /// Whether the one to talk to has a favor to ask or be thanked for.
    pub fn listener_asks(&self) -> bool {
        match (self.listener(), &self.play) {
            (Some(Listener::Local(i, _)), Some(play)) => crate::play::asks(play, i, &self.lit()),
            (Some(Listener::Folk(p, seed)), Some(play)) => crate::play::folk_asks(play, seed, p),
            _ => false,
        }
    }

    /// Talks to whoever is close: their card comes up. False when nobody is.
    pub fn talk(&mut self) -> bool {
        let Some(who) = self.listener() else {
            return self.ask_rider();
        };
        let card = match who {
            Listener::Local(i, _) => {
                let sim = &self.sim;
                let lit = |id: &str| sim.beacon(id).is_some_and(|(_, lit)| lit);
                let Some(play) = self.play.as_mut() else {
                    return false;
                };
                let card = crate::play::talk_card(play, i, &lit);
                if crate::play::take_changed(play) {
                    self.asks.push(Ask::Save);
                }
                card
            }
            Listener::Kin(i, _) => {
                let Some(play) = self.play.as_ref() else {
                    return false;
                };
                self.kin_talks += 1;
                crate::play::kin_talk(play, i, self.kin_talks)
            }
            Listener::Guest(k, _) => {
                let Some(card) = self
                    .play
                    .as_ref()
                    .and_then(|p| crate::play::guest_card(p, k))
                else {
                    return false;
                };
                card
            }
            Listener::Folk(p, seed) => {
                let world = &self.sim.world;
                // Somewhere a dragon can walk up to: on the ground, not up on a roof or
                // in the water, so a dropped thing can always be found.
                let rest = |x: f32, z: f32| {
                    let at =
                        |x: f32, z: f32| Vec3::new(x, world.support_still(x, z, p.y + 30.0), z);
                    (0..24)
                        .map(|k| {
                            let a = k as f32 * 2.4;
                            let r = k as f32 * 1.2;
                            at(x + a.cos() * r, z + a.sin() * r)
                        })
                        .find(|q| {
                            !world.afloat(q.x, q.z, q.y)
                                && !world.in_building(*q - Vec3::Y * 0.5)
                                && !world.in_building(*q + Vec3::Y)
                                && (q.y - p.y).abs() < 6.0
                        })
                        .unwrap_or_else(|| at(x, z))
                };
                let Some(play) = self.play.as_mut() else {
                    return false;
                };
                if let Some((card, offer)) =
                    crate::play::chore_card(play, seed, p, &rest, crate::talk::name(seed))
                {
                    self.chore_offer = offer;
                    self.hud.book.cards.push(card);
                    self.release_keys();
                    return true;
                }
                let sky = self.sky();
                let Some(play) = self.play.as_ref() else {
                    return false;
                };
                let scene = crate::talk::Scene {
                    hour: sky.hour,
                    rain: false,
                    under: self.sim.world.terrain.daylight(p + Vec3::Y) < 0.5,
                    kind: crate::play::kindness(play),
                    rumor: crate::play::rumor(play, seed / 31 + self.chats),
                    dare: crate::play::dare(play, seed / 7 + self.chats),
                    dark: crate::play::dark_here(play, p),
                    lair: crate::play::lair_tip(play, seed),
                };
                self.chats += 1;
                crate::book::Card {
                    title: crate::talk::name(seed).into(),
                    body: crate::talk::line(seed, self.chats, &scene),
                    foot: String::new(),
                    kind: crate::book::CardKind::Say,
                }
            }
        };
        self.hud.book.cards.push(card);
        self.release_keys();
        true
    }

    /// Asks the passenger, if there is one, where they would like to go: show the way,
    /// hop off, or keep flying.
    pub fn ask_rider(&mut self) -> bool {
        let Some(rider) = self.play.as_ref().and_then(crate::play::rider) else {
            return false;
        };
        let landed = self.player_dragon().mode == Mode::Ground;
        let wants = match rider.wants.strip_prefix("The ") {
            Some(rest) => format!("the {rest}"),
            None => rider.wants.clone(),
        };
        let about = rider.about;
        let body = format!(
            "I'd love to see {wants}! {about} Just follow the red arrow. {}",
            if landed {
                "Or I can hop off right here, if you like."
            } else {
                "Or I can hop off right here. I brought my umbrella!"
            }
        );
        self.hud.book.cards.push(crate::book::Card {
            title: rider.name,
            body,
            foot: String::new(),
            kind: crate::book::CardKind::Rider,
        });
        self.release_keys();
        true
    }

    /// Where `p` shows on the screen last drawn, in pixels, and whether it is in view.
    /// Behind the camera it is flipped, so an arrow still points the right way round.
    fn to_screen(&self, p: Vec3) -> (Vec2, bool) {
        let size = self.hud.book.screen().max(Vec2::ONE);
        let clip = self.camera.view_proj(size.x / size.y) * p.extend(1.0);
        let w = clip.w.abs().max(1e-3);
        let mut n = Vec2::new(clip.x / w, clip.y / w);
        if clip.w < 0.0 {
            n = -n * 10.0;
        }
        let at = Vec2::new((n.x * 0.5 + 0.5) * size.x, (0.5 - n.y * 0.5) * size.y);
        let seen = clip.w > 0.0 && at.cmpge(Vec2::ZERO).all() && at.cmple(size).all();
        (at, seen)
    }

    /// The passenger's bubble, button and pointing, where they show this frame.
    fn update_rider_bits(&mut self, dt: f32) {
        if let Some((_, left)) = &mut self.hud.say {
            *left -= dt;
            if *left <= 0.0 {
                self.hud.say = None;
            }
        }
        if let Some((_, left)) = &mut self.point_at {
            *left -= dt;
            if *left <= 0.0 {
                self.point_at = None;
            }
        }
        let riding = self.play.as_ref().and_then(crate::play::rider).is_some();
        if !riding {
            self.hud.say = None;
            self.point_at = None;
        }
        if self.halted() {
            self.hud.goal = None;
        }
        let head = self
            .visual(self.player)
            .map_or(self.player_dragon().pos, |v| v.interpolated(self.alpha()).0)
            + Vec3::Y * 1.9;
        self.hud.rider_at = riding
            .then(|| self.to_screen(head))
            .filter(|(_, seen)| *seen)
            .map(|(at, _)| at);
        self.hud.point = self
            .point_at
            .map(|(p, _)| self.to_screen(p + Vec3::Y * 1.5));
        let me = self.player_dragon().pos;
        let flock_goal = self.flock.busy().and_then(|(i, task)| {
            let k = self.kin.get(i)?;
            let f = &self.flock.dragons[i];
            Some(match task {
                crate::kin::Task::Race => {
                    let (to, at, r) = k.race.clone()?;
                    crate::play::Goal { name: to, at, r }
                }
                _ => crate::play::Goal {
                    name: format!("Follow {}", k.name),
                    at: f.d.pos,
                    r: 0.0,
                },
            })
        });
        self.hud.goal = flock_goal
            .or_else(|| self.play.as_ref().and_then(crate::play::goal))
            .filter(|g| g.at.distance(me) > g.r)
            .map(|g| {
                let (mut at, seen) = self.to_screen(g.at + Vec3::Y * 2.0);
                if !seen {
                    // Which way to turn, worked out from the camera, so a spot behind
                    // points down and round rather than wherever its flipped image falls.
                    let cam = &self.camera;
                    let fwd = (cam.look - cam.pos).normalize_or(Vec3::Z);
                    let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
                    let up = right.cross(fwd);
                    let to = g.at - cam.pos;
                    let (x, y, z) = (to.dot(right), to.dot(up), to.dot(fwd));
                    let mut dir = glam::Vec2::new(x, -y);
                    if z < 0.0 {
                        dir.y = dir.y.abs().max(x.abs() * 0.5 + 1.0);
                    }
                    at = self.hud.book.screen() * 0.5 + dir.normalize_or(glam::Vec2::Y) * 10000.0;
                }
                let far = me.distance(g.at);
                let far = if far < 1000.0 {
                    format!("{:.0} m", (far / 10.0).round() * 10.0)
                } else {
                    format!("{:.1} km", far / 1000.0)
                };
                (at, seen, format!("{}  {far}", g.name))
            });
        let world = &self.sim.world;
        for f in &mut self.floaters {
            f.age += dt;
            if f.ground == f32::MIN {
                f.ground = world.support_still(f.from.x, f.from.z, f.from.y);
            }
        }
        self.floaters.retain(|f| {
            let (_, down) = f.at();
            !down || f.age < (f.from.y - f.ground) / Floater::SINK + 4.0
        });
    }

    /// Whether a click or tap at a fraction of the screen falls on the passenger's
    /// button.
    fn on_rider_button(&self, nx: f32, ny: f32) -> bool {
        let size = self.hud.book.screen();
        self.hud
            .rider_button(size)
            .is_some_and(|(c, r)| (Vec2::new(nx, ny) * size).distance(c) < r * 1.3)
    }

    /// Where the talk bubble shows on screen, as a fraction of it, and how far round it
    /// a click still counts.
    pub fn bubble_on_screen(&self) -> Option<(Vec2, f32)> {
        let who = self.listener()?;
        let size = self.hud.book.screen();
        let aspect = size.x / size.y.max(1.0);
        let clip = self.camera.view_proj(aspect) * who.bubble().extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let at = Vec2::new(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5);
        Some((at, (1.6 / clip.w).max(0.04)))
    }

    /// Queues lines to show, a newer one about the same thing taking the older one's
    /// place, and only the last few kept so nothing shows long after it happened.
    fn toast_all(&mut self, lines: Vec<(String, String)>) {
        for (who, text) in lines {
            self.toasts.retain(|(w, _)| *w != who);
            self.toasts.push((who, text));
        }
        let over = self.toasts.len().saturating_sub(3);
        self.toasts.drain(..over);
    }

    /// A passenger chats as you fly: their sharp eyes spot glints you might miss, they
    /// point out beautiful places, and now and then just enjoy the ride. Never pauses.
    fn chatter(&mut self, dt: f32) {
        if let Some((_, _, left)) = &mut self.hud.toast {
            *left -= dt;
            if *left <= 0.0 {
                self.hud.toast = None;
            }
        }
        if self.hud.toast.is_none() && !self.toasts.is_empty() {
            let (who, text) = self.toasts.remove(0);
            self.hud.toast = Some((who, text, 5.0));
            self.chatter = self.chatter.max(3.0);
            return;
        }
        let Some(play) = &self.play else { return };
        let Some(rider) = crate::play::rider(play) else {
            self.hatchling_chatter(dt);
            return;
        };
        let d = self.player_dragon().clone();
        self.chatter -= dt;
        if self.chatter > 0.0 || self.hud.say.is_some() {
            return;
        }
        let mut said: Option<String> = None;
        let mut point: Option<Vec3> = None;
        // Sharp eyes: a glint within reach of a good look.
        let mut shinies = Vec::new();
        crate::play::shinies(play, &mut shinies);
        if let Some(s) = shinies
            .iter()
            .filter(|s| s.kind != "coins" && s.pos.distance(d.pos) < 220.0)
            .find(|s| !self.pointed.contains(&s.pos))
        {
            self.pointed.push(s.pos);
            point = Some(s.pos);
            said = Some(
                if s.pos.y < d.pos.y - 10.0 {
                    "Look! Something's glinting down there! I'll point."
                } else {
                    "Ooh, something shiny, just over there! See where I'm pointing?"
                }
                .into(),
            );
        }
        let goal = crate::play::goal(play);
        if said.is_none()
            && let Some(g) = &goal
            && d.mode == Mode::Air
            && ((self.time * 3.0) as u32).is_multiple_of(2)
        {
            // Which way to go, said the way a passenger would.
            let fwd = glam::Vec2::new(d.heading.sin(), d.heading.cos());
            let right = glam::Vec2::new(-fwd.y, fwd.x);
            let to = glam::Vec2::new(g.at.x - d.pos.x, g.at.z - d.pos.z);
            let (ahead, side) = (to.dot(fwd), to.dot(right));
            said = Some(if to.length() < 150.0 {
                format!("That's {}, right down there! Let's land!", g.name)
            } else if ahead > side.abs() * 2.5 {
                "Straight ahead! Keep going!".into()
            } else if ahead < 0.0 && side.abs() < -ahead {
                "Oops, it's behind us! Let's turn round.".into()
            } else if side > 0.0 {
                "It's off to the right, I think!".into()
            } else {
                "It's off to the left, I think!".into()
            });
        }
        if said.is_none()
            && goal.is_none()
            && let Some((name, at)) = crate::play::spot_near(play, d.pos, 450.0)
            && !self.pointed.contains(&at)
        {
            self.pointed.push(at);
            point = Some(at);
            said = Some(format!("Oh, look! It's {name}, over there!"));
        }
        if said.is_none() && d.mode == Mode::Air {
            const RIDE: [&str; 8] = [
                "Wheee!",
                "Look how tiny the houses are!",
                "I can see my house from here!",
                "Not so fast! No, wait, faster!",
                "The wind smells like the sea up here.",
                "I'm never walking anywhere again.",
                "Can we do that bit again?",
                "Hello, clouds!",
            ];
            let k = (self.time * 7.0) as usize % RIDE.len();
            said = Some(if d.speed() > 45.0 { "Whoooa!" } else { RIDE[k] }.into());
        }
        if let Some(text) = said {
            let _ = rider;
            self.hud.say = Some((text, 5.0));
            if let Some(p) = point {
                self.point_at = Some((p, 8.0));
            }
            self.chatter = 9.0 + (self.time as f32 * 3.1).sin().abs() * 6.0;
        } else {
            self.chatter = 2.0;
        }
    }

    /// The hatchling has sharp little eyes: it squeaks at glints you might fly past.
    fn hatchling_chatter(&mut self, dt: f32) {
        let Some(play) = &self.play else { return };
        if crate::play::egg(play) != 2 || self.flock.hatchling.is_none() {
            return;
        }
        self.chatter -= dt;
        if self.chatter > 0.0 {
            return;
        }
        self.chatter = 6.0;
        let me = self.player_dragon().pos;
        let mut shinies = Vec::new();
        crate::play::shinies(play, &mut shinies);
        if let Some(s) = shinies
            .iter()
            .filter(|s| s.kind != "coins" && s.pos.distance(me) < 160.0)
            .find(|s| !self.pointed.contains(&s.pos))
        {
            self.pointed.push(s.pos);
            self.point_at = Some((s.pos, 8.0));
            self.toast_all(vec![(
                "Little one".into(),
                "Squeak! Squeak! It's spotted something shiny, over there!".into(),
            )]);
            self.chatter = 15.0;
        }
    }

    /// Answers a dragon's ask: follow it, race it, swap, or take the egg.
    fn kin_answer(&mut self, i: usize, ask: crate::book::KinAsk) {
        use crate::book::KinAsk;
        let Some(k) = self.kin.get(i) else { return };
        match ask {
            KinAsk::Follow => {
                self.flock.lead(i, k);
                self.toast_all(vec![(
                    k.name.clone(),
                    "This way! Follow the red arrow.".into(),
                )]);
            }
            KinAsk::Race => {
                self.flock.race(i, k);
                self.racing = Some(i);
                let to = k.race.as_ref().map_or(String::new(), |r| r.0.clone());
                self.toast_all(vec![(k.name.clone(), format!("Go! First to {to} wins!"))]);
            }
            KinAsk::Trade | KinAsk::Egg => {
                if let Some(play) = &mut self.play
                    && let Some(card) = crate::play::kin_do(play, i, ask)
                {
                    self.hud.book.cards.push(card);
                    self.asks.push(Ask::Save);
                }
            }
            KinAsk::Chat => {}
        }
    }

    /// The other dragons and the family, a step: they fly about, meet you, lead and
    /// race, leave gifts, and the egg and hatchling do their things.
    fn kin_step(&mut self) {
        let Some(play) = &mut self.play else {
            self.flock = crate::kin::Flock::default();
            return;
        };
        let world = &self.sim.world;
        if self.kin.is_empty() {
            self.kin = crate::play::kin_info(play);
            for k in &mut self.kin {
                if let Some((_, at, _)) = &mut k.race {
                    at.y = world.support_still(at.x, at.z, 5000.0);
                }
            }
        }
        if self.kin.is_empty() {
            return;
        }
        let p = &self.sim.flight;
        self.flock.ensure(&self.kin, p.stand_height);
        let me = self
            .sim
            .dragon(self.player)
            .expect("player dragon exists")
            .clone();
        let egg = crate::play::egg(play);
        let size = crate::play::hatchling_size(play);
        let side = Vec3::new(me.heading.cos(), 0.0, -me.heading.sin());
        let beside = me.pos + side * 3.0;
        self.flock
            .set_hatchling((egg == 2).then_some(beside), size, p.stand_height * size);
        let met: Vec<bool> = (0..self.kin.len())
            .map(|i| crate::play::kin_met(play, i))
            .collect();
        let news = self.flock.tick(&self.kin, &met, &me, p, world, DT);
        let mut lines = Vec::new();
        let mut cards = Vec::new();
        for n in news {
            match n {
                crate::kin::FlockNews::Shown(i) => {
                    lines.push(crate::play::kin_shown(play, i));
                    self.flock.home(i, &self.kin[i]);
                }
                crate::kin::FlockNews::Finished(i) => {
                    if self.racing == Some(i) {
                        self.racing = None;
                        match crate::play::kin_raced(play, i, false) {
                            Ok(c) => cards.push(c),
                            Err(l) => lines.push(l),
                        }
                    }
                    self.flock.home(i, &self.kin[i]);
                }
            }
        }
        // Beat the racer to the finish.
        if let Some(i) = self.racing
            && let Some((_, at, r)) = &self.kin[i].race
            && (at.x - me.pos.x).hypot(at.z - me.pos.z) < *r
            && (at.y - me.pos.y).abs() < r.max(40.0)
        {
            self.racing = None;
            match crate::play::kin_raced(play, i, true) {
                Ok(c) => cards.push(c),
                Err(l) => lines.push(l),
            }
            self.flock.home(i, &self.kin[i]);
        }
        // Coming upon a dragon not yet met.
        for i in 0..self.kin.len() {
            if crate::play::kin_met(play, i) {
                continue;
            }
            // The shy one is only caught sitting still. The others can be met anywhere.
            let f = &self.flock.dragons[i];
            let shy = self.kin[i].kind == "shy";
            if (shy && !f.settled()) || f.d.pos.distance(me.pos) > 70.0 {
                continue;
            }
            match crate::play::find_kin(play, i) {
                Some(crate::play::Meet::Card(c)) => cards.push(c),
                Some(crate::play::Meet::Flee(who, line)) => {
                    lines.push((who, line));
                    if let Some(to) = self.flock.flee(i, &self.kin[i]) {
                        self.point_at = Some((to, 8.0));
                    }
                }
                None => {}
            }
        }
        // Home in the lair: a friend may have left something, and the egg wants fire.
        let heap = crate::play::heap(play).map(|h| h.at);
        let home = heap.is_some_and(|h| h.distance(me.pos) < 25.0);
        if home && me.mode == Mode::Ground && !self.gift_looked {
            self.gift_looked = true;
            cards.extend(crate::play::kin_gift(play));
        }
        if heap.is_none_or(|h| h.distance(me.pos) > 80.0) {
            self.gift_looked = false;
        }
        if egg == 1 && heap.is_some_and(|h| h.distance(me.pos) < 14.0) && me.breath > 0.5 {
            self.warm += DT;
            if self.warm > 2.0 {
                self.warm = 0.0;
                cards.extend(crate::play::hatch(play));
            }
        }
        // Firsts with the hatchling.
        if egg == 2 {
            if me.mode == Mode::Air {
                self.together += DT;
            }
            if me.breath > 0.5 {
                self.fired += DT;
            }
            let water = world
                .terrain
                .water(me.pos.x, me.pos.z, me.pos.y - 13.0)
                .is_some_and(|l| me.pos.y < l + 1.5);
            for (name, now) in [
                ("flight", self.together > 8.0),
                ("splash", me.swimming || water),
                ("fire", self.fired > 1.0),
            ] {
                if now {
                    lines.extend(crate::play::first(play, name));
                }
            }
        }
        if crate::play::take_changed(play) {
            self.asks.push(Ask::Save);
        }
        self.toast_all(lines);
        if !cards.is_empty() {
            self.hud.book.cards.extend(cards);
            self.release_keys();
        }
    }

    /// Whether a click or tap at a fraction of the screen falls on the talk bubble.
    fn on_bubble(&self, nx: f32, ny: f32) -> bool {
        let Some((at, r)) = self.bubble_on_screen() else {
            return false;
        };
        let size = self.hud.book.screen();
        let aspect = size.x / size.y.max(1.0);
        Vec2::new((at.x - nx) * aspect, at.y - ny).length() < r
    }

    /// The hoard as the journal shows it, none with gameplay off.
    /// What the journal's map page needs: the map, the world and where the dragon is.
    pub fn map_view(&self) -> crate::book::MapView {
        let d = self.player_dragon();
        crate::book::MapView {
            px: self.map_px.get(),
            bounds: self.sim.world.terrain.def.bounds,
            me: Vec2::new(d.pos.x, d.pos.z),
            heading: d.heading,
        }
    }

    pub fn hoard_view(&self) -> Option<HoardView> {
        let world = &self.sim.world;
        let beacon = |id: &str| world.beacons.iter().find(|b| b.id == id).map(|b| b.flame);
        self.play.as_ref().map(|p| crate::play::view(p, &beacon))
    }

    /// Whether the world waits: paused, the journal, a card or the big map is up, or a
    /// postcard is being taken.
    pub fn halted(&self) -> bool {
        self.paused || self.hud.book.showing() || self.hud.open() || !self.snaps.is_empty()
    }

    /// Whether this frame is a postcard's view, for the host to hand back with
    /// [`Game::take_snap`]. Nothing is drawn over it.
    pub fn wants_snap(&self) -> bool {
        !self.snaps.is_empty()
    }

    /// Points the camera for a postcard: up behind the dragon, looking out past it the
    /// way it faces, so the place fills the picture with the dragon small in it.
    fn snap_camera(&mut self) {
        // The best view of the place itself, at this time of day and in this weather.
        if let Some(snap) = self.snaps.first()
            && let Some(play) = self.play.as_ref()
            && let Some((at, r)) = crate::play::card_spot(play, snap.card)
            && let Some((pos, look)) = self.best_view(at, r, crate::play::card_low(play, snap.card))
        {
            self.camera.pos = pos;
            self.camera.look = look;
            self.camera.roll = 0.0;
            self.camera.fov = 62.0;
            return;
        }
        let d = self.player_dragon().clone();
        let mut dir = self.camera.look - self.camera.pos;
        dir.y = 0.0;
        let dir = dir
            .try_normalize()
            .unwrap_or(Vec3::new(d.heading.sin(), 0.0, d.heading.cos()));
        let mut pos = d.pos - dir * 15.0 + Vec3::Y * 6.0;
        let floor = self.sim.world.support_still(pos.x, pos.z, pos.y + 4.0) + 3.0;
        pos.y = pos.y.max(floor);
        self.camera.pos = pos;
        self.camera.look = d.pos + dir * 80.0 + Vec3::Y * 2.0;
        self.camera.roll = 0.0;
        self.camera.fov = 62.0;
    }

    /// Where to stand to see the place at `at`, about `r` across, at its best: a little
    /// way off and up, with nothing in the way, the light from behind if it can be, and
    /// open air under the camera. None when no way round is clear.
    fn best_view(&self, at: Vec3, r: f32, low: bool) -> Option<(Vec3, Vec3)> {
        let world = &self.sim.world;
        let terrain = &world.terrain;
        // A falls is seen from its pool: aim halfway down the drop from its foot.
        let (at, r) = match crate::water::falls_feet(terrain)
            .into_iter()
            .map(|(foot, _)| foot)
            .filter(|f| low && f.y < at.y - 20.0)
            .min_by(|a, b| {
                let d = |p: &Vec3| Vec2::new(p.x - at.x, p.z - at.z).length();
                d(a).total_cmp(&d(b))
            })
            .filter(|f| Vec2::new(f.x - at.x, f.z - at.z).length() < 500.0)
        {
            Some(foot) => (
                Vec3::new(foot.x, (foot.y + at.y) * 0.5, foot.z),
                r.max((at.y - foot.y) * 0.9),
            ),
            None => (at, r),
        };
        let under = terrain.daylight(at + Vec3::Y * 2.0) < 0.5;
        let subject = at + Vec3::Y * (r * 0.08).clamp(2.0, 8.0);
        let far = if under {
            (r * 1.1).clamp(25.0, 80.0)
        } else {
            (r * 1.45).clamp(35.0, 180.0)
        };
        // Which way the ground falls away from the place: out in front of a falls.
        let n = terrain.normal(at.x, at.z);
        let downhill = Vec3::new(n.x, 0.0, n.z).normalize_or_zero();
        let sun = self.sky().light_dir;
        let sun_flat = Vec3::new(sun.x, 0.0, sun.z).normalize_or_zero();
        let (room, gap) = if under { (2.0, 0.6) } else { (4.0, 1.5) };
        let clear = |from: Vec3, to: Vec3| {
            let n = 28;
            (1..n).all(|k| {
                let p = from.lerp(to, k as f32 / n as f32);
                // The last stretch is the place itself.
                p.distance(to) < r.min(25.0) || terrain.sdf(p) > gap
            })
        };
        let want = if low { -0.2 } else { 0.24 };
        let mut best: Option<(f32, Vec3)> = None;
        // Further off first, closer in where caves or crags close round.
        let rises: &[f32] = if low {
            &[-0.2, -0.35, -0.08, 0.1]
        } else {
            &[0.32, 0.18, 0.5, 0.08]
        };
        for (e, dist) in rises
            .iter()
            .copied()
            .flat_map(|e| [(e, far), (e, far * 0.6), (e, far * 0.35)])
        {
            let e = if under { e * 0.5 } else { e };
            for k in 0..20 {
                let a = k as f32 / 20.0 * std::f32::consts::TAU;
                let out = Vec3::new(a.sin(), 0.0, a.cos());
                let cam = subject + out * dist * e.cos() + Vec3::Y * dist * e.sin();
                if terrain.sdf(cam) < room
                    || terrain.water(cam.x, cam.z, cam.y).is_some()
                    || !clear(cam, subject)
                {
                    continue;
                }
                // The sun over the shoulder lights the place; looking into it washes out.
                let light = out.dot(sun_flat);
                let front = if low { out.dot(downhill) * 1.2 } else { 0.0 };
                let score = light * 0.6 + front - (e - want).abs() - (1.0 - dist / far) * 0.5
                    + if world.solid_at(cam) { -5.0 } else { 0.0 };
                if best.is_none_or(|(s, _)| score > s) {
                    best = Some((score, cam));
                }
            }
            if best.is_some_and(|(s, _)| s > 0.1) {
                break;
            }
        }
        best.map(|(_, cam)| (cam, subject))
    }

    /// The postcard's picture: a frame of `w` by `h` RGBA pixels from the postcard
    /// camera. The camera goes back, and the card shows with the picture on it.
    pub fn take_snap(&mut self, rgba: &[u8], w: usize, h: usize) {
        if self.snaps.is_empty() {
            return;
        }
        let snap = self.snaps.remove(0);
        if let Some(play) = &mut self.play {
            crate::play::set_pic(play, snap.card, rgba, w, h);
        }
        self.finish_snap(snap);
    }

    fn finish_snap(&mut self, snap: Snap) {
        self.camera = snap.camera;
        if let Some(play) = &self.play {
            self.hud
                .book
                .cards
                .push(crate::play::postcard_card(play, snap.card));
        }
        self.asks.push(Ask::Save);
        if !self.snaps.is_empty() {
            self.snap_camera();
        }
    }

    /// Opens the journal where it was left, which pauses the game.
    pub fn open_book(&mut self) {
        let hoard = self.hoard_view();
        let view = View {
            hoard: hoard.as_ref(),
            binds: &self.bindings,
            muted: self.muted,
            pause_away: self.pause_away,
            hold_hour: self.hold_hour,
            touch: self.touch,
            hud: self.hud.on,
            map: self.map_view(),
        };
        self.hud.book.show(&view);
        self.release_keys();
        self.asks.push(Ask::Save);
    }

    /// Does what the journal asked. `was` says whether it was showing before.
    fn apply(&mut self, dos: Vec<Do>, was: bool) {
        for d in dos {
            match d {
                Do::Close | Do::Turned => {}
                Do::Source => self.asks.push(Ask::Source),
                Do::Decline(i) => {
                    if let Some(play) = &mut self.play {
                        crate::play::decline(play, i);
                    }
                }
                Do::Kin(i, ask) => self.kin_answer(i, ask),
                Do::TakeChore => {
                    if let (Some(play), Some(ch)) = (&mut self.play, self.chore_offer.take()) {
                        crate::play::take_chore(play, ch);
                        self.pointed.clear();
                        self.chatter = 3.0;
                    }
                }
                Do::RiderMap => {
                    self.open_book();
                    let hoard = self.hoard_view();
                    let view = View {
                        hoard: hoard.as_ref(),
                        binds: &self.bindings,
                        muted: self.muted,
                        pause_away: self.pause_away,
                        hold_hour: self.hold_hour,
                        touch: self.touch,
                        hud: self.hud.on,
                        map: self.map_view(),
                    };
                    self.hud.book.show_me("favors", &view);
                }
                Do::HopOff => {
                    let rider = self.play.as_ref().and_then(crate::play::rider);
                    if let (Some(play), Some(rider)) = (&mut self.play, rider) {
                        crate::play::hop_off(play);
                        self.asks.push(Ask::Save);
                        let d = self.sim.dragon(self.player).expect("player dragon exists");
                        let flying = d.mode == Mode::Air;
                        if flying {
                            self.floaters.push(Floater {
                                from: d.pos + Vec3::Y * 1.2,
                                age: 0.0,
                                look: rider.look,
                                ground: f32::MIN,
                            });
                        }
                        self.toast_all(vec![(
                            rider.name,
                            if flying {
                                "Wheee! Good thing I brought my umbrella. Thanks for the lift!"
                            } else {
                                "Thanks for the lift! I'll find my own way from here."
                            }
                            .into(),
                        )]);
                    }
                }
                Do::Accept(i) => {
                    if let Some(play) = &mut self.play {
                        crate::play::accept(play, i);
                        self.asks.push(Ask::Save);
                        self.pointed.clear();
                        self.chatter = 3.0;
                    }
                }
                Do::Begin(s) => {
                    if let Some(play) = &mut self.play {
                        crate::play::begin(play, s);
                        self.asks.push(Ask::Save);
                    }
                }
                Do::SetAside => {
                    if let Some(play) = &mut self.play {
                        crate::play::set_aside(play);
                        self.asks.push(Ask::Save);
                    }
                }
                Do::GiveBack(i) => {
                    if let Some(play) = &mut self.play {
                        let card = crate::play::give_back(play, i);
                        self.hud.book.cards.push(card);
                        self.asks.push(Ask::Save);
                    }
                }
                Do::Sound => {
                    self.muted = !self.muted;
                    self.asks.push(Ask::Prefs);
                }
                Do::PauseAway => {
                    self.pause_away = !self.pause_away;
                    self.asks.push(Ask::Prefs);
                }
                Do::TimeOfDay => {
                    let hours = crate::book::HOLD_HOURS;
                    let next = match self.hold_hour {
                        None => Some(hours[0].1),
                        Some(h) => hours
                            .iter()
                            .position(|t| t.1 == h)
                            .and_then(|k| hours.get(k + 1))
                            .map(|t| t.1),
                    };
                    self.hold(next);
                    self.asks.push(Ask::Prefs);
                }
                Do::Export => self.asks.push(Ask::Export),
                Do::Import => self.asks.push(Ask::Import),
                Do::ResetKeys => {
                    self.bindings = Bindings::default();
                    self.asks.push(Ask::Keys);
                }
                Do::Bind(action, code) => {
                    self.bindings.rebind(action, &code);
                    self.asks.push(Ask::Keys);
                }
                Do::Claim(i) => {
                    if let Some(play) = &mut self.play {
                        crate::play::claim(play, i);
                        self.asks.push(Ask::Save);
                    }
                }
            }
        }
        // Keys held when the journal opened or closed must not stick. A pause keeps the
        // game, so it carries on from here next time.
        if was != self.hud.book.showing() {
            self.release_keys();
            self.asks.push(Ask::Prefs);
            self.asks.push(Ask::Save);
        }
    }

    /// A click or tap on the game's own screens, at a fraction of the screen. False when
    /// it is for the world, such as tap to land.
    pub fn ui_click(&mut self, nx: f32, ny: f32) -> bool {
        if self.feedback.active || self.hud.open() {
            return false;
        }
        let hoard = self.hoard_view();
        let view = View {
            hoard: hoard.as_ref(),
            binds: &self.bindings,
            muted: self.muted,
            pause_away: self.pause_away,
            hold_hour: self.hold_hour,
            touch: self.touch,
            hud: self.hud.on,
            map: self.map_view(),
        };
        let was = self.hud.book.showing();
        match self.hud.book.click(Vec2::new(nx, ny), &view) {
            Some(dos) => {
                self.apply(dos, was);
                true
            }
            None if self.on_rider_button(nx, ny) => self.ask_rider(),
            None => self.on_bubble(nx, ny) && self.talk(),
        }
    }

    /// The pointer moved, at a fraction of the screen.
    pub fn ui_hover(&mut self, nx: f32, ny: f32) {
        self.hud.book.hover(Vec2::new(nx, ny));
    }

    /// Gamepad axes and buttons, W3C standard order. Start opens and closes the journal,
    /// and while it or a card is up the pad works it instead of flying.
    pub fn gamepad(&mut self, axes: &[f32], buttons: &[f32]) {
        let down: Vec<bool> = buttons.iter().map(|b| *b > 0.5).collect();
        let pressed: Vec<usize> = (0..down.len())
            .filter(|i| down[*i] && !self.pad_was.get(*i).copied().unwrap_or(false))
            .collect();
        self.pad_was = down;
        let ui = self.hud.book.showing();
        let card = !self.hud.book.cards.is_empty();
        for b in pressed {
            // Y talks to whoever is close.
            if b == 3 && !ui && self.talk() {
                continue;
            }
            let code = match b {
                9 if card => "Enter",
                9 => "Escape",
                0 if ui => "Enter",
                1 if ui => "Escape",
                4 if ui => "PageUp",
                5 if ui => "PageDown",
                12 if ui => "ArrowUp",
                13 if ui => "ArrowDown",
                14 if ui => "ArrowLeft",
                15 if ui => "ArrowRight",
                _ => continue,
            };
            self.ui_key(code, None);
        }
        if ui || self.hud.book.showing() {
            self.input.gamepad(&[], &[]);
        } else {
            self.input.gamepad(axes, buttons);
        }
    }

    /// Sound, pausing, the time of day and the journal's page, one setting a line.
    pub fn prefs_text(&self) -> String {
        format!(
            "sound {}\npauseaway {}\ntime {}\npage {}\n",
            if self.muted { "off" } else { "on" },
            if self.pause_away { "on" } else { "off" },
            self.hold_hour
                .map_or("passing".to_string(), |h| format!("{h}")),
            self.hud.book.page.name()
        )
    }

    /// Holds the day at `hour`, or with none lets it pass again from where it is.
    pub fn hold(&mut self, hour: Option<f32>) {
        if hour.is_none()
            && let Some(h) = self.hold_hour
        {
            let secs = (self.sim.tick as f64 * DT as f64) as f32;
            let len = self.look.day_length.max(1.0);
            self.look.start_hour = clock(cycle(h) - secs / len);
        }
        self.hold_hour = hour;
    }

    pub fn set_prefs_text(&mut self, text: &str) {
        for line in text.lines() {
            match line.split_whitespace().collect::<Vec<_>>()[..] {
                ["sound", v] => self.muted = v == "off",
                ["pauseaway", v] => self.pause_away = v != "off",
                ["time", v] => self.hold(v.parse().ok()),
                ["page", v] => {
                    if let Some(p) = crate::book::Page::from_name(v) {
                        self.hud.book.page = p;
                    }
                }
                _ => {}
            }
        }
    }

    /// The game as a save file, none with gameplay off.
    pub fn save_text(&self) -> Option<String> {
        let play = self.play.as_ref()?;
        Some(crate::play::save(play) + &self.where_text())
    }

    /// Where the game was left, as the save's `[where]` section: the dragon's place and
    /// heading, whether it stood, the clock (which sets the time of day and the
    /// weather), and any cards not yet read.
    fn where_text(&self) -> String {
        let d = self.player_dragon();
        let clean = |s: &str| s.replace(['|', '\n'], " ");
        let mut out = format!(
            "[where]\nat {:.2} {:.2} {:.2} {:.3} {}\ntime {:.1}\n",
            d.pos.x,
            d.pos.y,
            d.pos.z,
            d.heading,
            (d.mode == Mode::Ground) as u8,
            self.time
        );
        for c in &self.hud.book.cards {
            let kind = match c.kind {
                crate::book::CardKind::Find => "find".to_string(),
                crate::book::CardKind::News => "news".to_string(),
                crate::book::CardKind::Say => "say".to_string(),
                crate::book::CardKind::Carry => "carry".to_string(),
                crate::book::CardKind::Postcard(Some(i)) => format!("postcard {i}"),
                _ => continue,
            };
            out += &format!(
                "card {kind}|{}|{}|{}\n",
                clean(&c.title),
                clean(&c.body),
                clean(&c.foot)
            );
        }
        out
    }

    /// Replaces the game with a save file's, and carries on where it was left: in the
    /// same spot at the same time of day, with any unread cards. A race is off, and a
    /// passenger has gone home to wait for another ride.
    pub fn load_save(&mut self, text: &str) -> Result<(), String> {
        let (game, place) = match text.find("[where]") {
            Some(i) => {
                let rest = &text[i + "[where]".len()..];
                let end = rest.find("\n[").map_or(rest.len(), |e| e + 1);
                (
                    format!("{}{}", &text[..i], &rest[end..]),
                    Some(rest[..end].to_string()),
                )
            }
            None => (text.to_string(), None),
        };
        let play = self.play.as_mut().ok_or("gameplay is off")?;
        crate::play::load(play, &game)?;
        crate::play::hop_off(play);
        if let Some(place) = place {
            self.resume(&place);
        }
        Ok(())
    }

    /// Puts the game back as a save's `[where]` section left it.
    fn resume(&mut self, text: &str) {
        for line in text.lines() {
            let mut w = line.split_whitespace();
            match w.next() {
                Some("at") => {
                    let v: Vec<f32> = w.filter_map(|s| s.parse().ok()).collect();
                    if let [x, y, z, heading, stood] = v[..] {
                        self.place_player(Vec3::new(x, y, z), heading, stood > 0.5);
                    }
                }
                Some("time") => {
                    if let Some(t) = w.next().and_then(|s| s.parse().ok()) {
                        self.time = t;
                    }
                }
                Some("card") => {
                    let rest = line["card ".len()..].to_string();
                    let mut parts = rest.split('|');
                    let (Some(kind), Some(title), Some(body), Some(foot)) =
                        (parts.next(), parts.next(), parts.next(), parts.next())
                    else {
                        continue;
                    };
                    let kind = match kind.split_once(' ') {
                        Some(("postcard", i)) => match i.parse() {
                            Ok(i) => crate::book::CardKind::Postcard(Some(i)),
                            Err(_) => continue,
                        },
                        _ => match kind {
                            "find" => crate::book::CardKind::Find,
                            "news" => crate::book::CardKind::News,
                            "carry" => crate::book::CardKind::Carry,
                            _ => crate::book::CardKind::Say,
                        },
                    };
                    self.hud.book.cards.push(crate::book::Card {
                        title: title.into(),
                        body: body.into(),
                        foot: foot.into(),
                        kind,
                    });
                }
                _ => {}
            }
        }
    }

    /// Puts the dragon exactly at `pos` facing `heading`, standing or gliding on.
    fn place_player(&mut self, pos: Vec3, heading: f32, stood: bool) {
        let player = self.player;
        let d = self.sim.dragon_mut(player).expect("player dragon exists");
        *d = Dragon::new(player, pos, heading);
        if !stood {
            d.mode = Mode::Air;
            d.vel = Vec3::new(heading.sin(), 0.0, heading.cos()) * 14.0;
        }
        let d = d.clone();
        if let Some((_, v)) = self.visuals.iter_mut().find(|(id, _)| *id == player) {
            v.snap(&d);
        }
        self.camera.reset();
        self.update_camera(0.0);
    }

    pub fn key(&mut self, code: &str, down: bool, repeat: bool) {
        // U turns the sound off and on.
        if code == "KeyU" {
            if down && !repeat {
                self.muted = !self.muted;
                self.asks.push(Ask::Prefs);
            }
            return;
        }
        // I, J, K and L look round while held, as dragging does.
        if let Some(k) = ["KeyI", "KeyK", "KeyJ", "KeyL"]
            .iter()
            .position(|c| *c == code)
        {
            self.look_keys[k] = down;
            return;
        }
        self.input
            .key(code, down, repeat, &self.bindings, &self.controls);
    }

    pub fn touch_action(&mut self, action: Action, down: bool) {
        self.input.touch_action(action, down, &self.controls);
    }

    /// Advances by real elapsed time in fixed ticks.
    pub fn advance(&mut self, dt: f32) {
        // A host that never sends the postcard's picture: carry on without one.
        if let Some(snap) = self.snaps.first_mut() {
            snap.waited += 1;
            if snap.waited > 30 {
                let snap = self.snaps.remove(0);
                self.finish_snap(snap);
            }
        }
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
        if self.halted() {
            return;
        }
        let dt = dt.clamp(0.0, 0.25);
        self.time += dt as f64;
        self.acc += dt;
        while self.acc >= DT {
            self.acc -= DT;
            self.tick();
            // A card that just came up stops the world where it is.
            if self.halted() {
                self.acc = 0.0;
                break;
            }
        }
        // A postcard's camera, just pointed, stays put for its picture.
        if !self.snaps.is_empty() {
            return;
        }
        self.chatter(dt);
        crate::prof::time("camera", || self.update_camera(dt));
        self.update_rider_bits(dt);
    }

    fn tick(&mut self) {
        let mut input = self.input.sample(DT, &self.bindings, &self.controls);
        input.land_at = self.land_request.take();
        let player = self.player;
        crate::prof::time("step", || self.sim.step(&[(player, input)]));
        crate::prof::time("fx", || self.fx.tick(&self.sim, DT));
        crate::prof::time("kin", || self.kin_step());
        let night = self.sky_night();
        if let Some(play) = &mut self.play {
            crate::play::set_night(play, night);
            let me = self.sim.dragon(self.player).expect("player dragon exists");
            let time = self.sim.tick as f32 * DT;
            let breath = self.sim.flight.breath_range;
            let news = crate::play::tick(play, me, &self.sim.world, time, breath);
            let mut lines = news.toasts;
            if news.cheer && crate::play::egg(play) == 2 {
                // The hatchling copies the stunt with a roll of its own.
                if let Some(h) = &mut self.flock.hatchling {
                    h.spin = std::f32::consts::TAU;
                }
                lines.extend(crate::play::first(play, "stunt"));
            }
            if news.cheer
                && let Some(rider) = crate::play::rider(play)
            {
                const CHEER: [&str; 4] = [
                    "Again! Again!",
                    "That was amazing!",
                    "Woohoo! Did you see that?",
                    "My tummy's still up there!",
                ];
                let k = self.sim.tick as usize % CHEER.len();
                lines.push((rider.name, CHEER[k].into()));
            }
            if crate::play::take_changed(play) {
                self.asks.push(Ask::Save);
            }
            self.toast_all(lines);
            if !news.cards.is_empty() || !news.snaps.is_empty() {
                self.hud.book.cards.extend(news.cards);
                self.release_keys();
            }
            for card in news.snaps {
                let first = self.snaps.is_empty();
                self.snaps.push(Snap {
                    card,
                    camera: self.camera.clone(),
                    waited: 0,
                });
                if first {
                    self.snap_camera();
                }
            }
        }
        crate::prof::time("visuals", || {
            for d in &self.sim.dragons {
                match self.visuals.iter_mut().find(|(id, _)| *id == d.id) {
                    Some((_, v)) => v.tick(d, &self.sim.flight, &self.sim.world, DT),
                    None => self.visuals.push((d.id, DragonVisual::new(d))),
                }
            }
        });
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
        f.y = (f.y - dy * 0.004).clamp(-1.3, 1.1);
        self.look_hold = 1.0;
    }

    fn update_camera(&mut self, dt: f32) {
        let [up, down, left, right] = self.look_keys;
        if up || down || left || right {
            let axis = |a: bool, b: bool| (a as i8 - b as i8) as f32;
            // About a quarter turn a second, as a steady drag would.
            self.look_drag(axis(left, right) * 260.0 * dt, axis(down, up) * 260.0 * dt);
        }
        // Standing still on the ground, a look round stays put until you move.
        let still = {
            let d = self.player_dragon();
            d.mode == zawnabout_sim::Mode::Ground && d.vel.length() < 0.5
        };
        if self.look_hold > 0.0 {
            self.look_hold -= dt;
        } else if !still {
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
                        // Its top is found below any rock overhead, so in a cave it is
                        // not the ground far above the roof.
                        let reach = if world.terrain.sdf(q) < 0.0 {
                            2.0
                        } else {
                            let mut up = 1.0;
                            while up < 500.0 && world.terrain.sdf(q + Vec3::Y * up) > 0.0 {
                                up += 2.0;
                            }
                            up
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
            // On the pool deck, aft of the sky deck and clear of the funnels.
            zawnabout_sim::town::Kind::Cruise => (-0.1, 0.25),
            // Beside the mast, clear of the sails along the middle.
            zawnabout_sim::town::Kind::Sailboat => (-0.2, 0.3),
            // Out on the seats, clear of the tower up the middle.
            zawnabout_sim::town::Kind::DropRing => (0.0, 0.45),
            zawnabout_sim::town::Kind::Car
            | zawnabout_sim::town::Kind::Train
            | zawnabout_sim::town::Kind::LiftChair
            | zawnabout_sim::town::Kind::LiftCabin
            | zawnabout_sim::town::Kind::FerrisCar => (0.0, 0.0),
            _ => (0.1, 0.0),
        };
        let p = pose.pos
            + Vec3::new(a.x, 0.0, a.y) * v.size.x * along
            + Vec3::new(-a.y, 0.0, a.x) * v.size.z * across;
        // Chairs and cabins hang below the cable: look down from just above them. Cars
        // and trains may be in a tunnel, so look down from just over their roofs.
        let above = match v.kind {
            zawnabout_sim::town::Kind::LiftChair
            | zawnabout_sim::town::Kind::LiftCabin
            | zawnabout_sim::town::Kind::FerrisCar => 0.5,
            zawnabout_sim::town::Kind::Car | zawnabout_sim::town::Kind::Train => v.size.y + 2.0,
            // Afloat in caves, under a low roof.
            zawnabout_sim::town::Kind::Punt => 4.0,
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
        if let Some((t, i)) = vehicle {
            // Where the ray first comes down onto the vehicle's top, so a tap on a big
            // ship's sky deck lands there, not on its usual spot. A near miss of
            // something small or quick still lands on its usual spot.
            let reach = traffic.vehicles[i].size.x.max(4.0);
            let mut s = (t - reach).max(0.0);
            while s < t + reach {
                let p = near + dir * s;
                if let Some(top) = world.deck_of(i, p.x, p.z)
                    && p.y <= top + 0.05
                    && p.y > top - 1.5
                {
                    return Some((Vec3::new(p.x, top, p.z), 3.0));
                }
                s += 0.25;
            }
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
        let n = self.spot_note(nx, ny, aspect)?;
        self.feedback.draft = Some(n.spot);
        let near: Vec<String> = n.near.iter().map(|s| json_str(s)).collect();
        let [x, y, z, yaw, pitch] = n.view;
        Some(format!(
            r#"{{"spot":[{:.1},{:.1},{:.1}],"view":[{x:.1},{y:.1},{z:.1},{yaw:.4},{pitch:.4}],"near":[{}]}}"#,
            n.spot.x,
            n.spot.y,
            n.spot.z,
            near.join(",")
        ))
    }

    /// An empty note on the surface under the cursor, seen from the free camera.
    fn spot_note(&self, nx: f32, ny: f32, aspect: f32) -> Option<Note> {
        let (near, dir) = self.ray(nx, ny, aspect);
        let spot = self.surface_hit(near, dir)?;
        let cam = self.feedback.cam;
        Some(Note {
            spot,
            view: [cam.pos.x, cam.pos.y, cam.pos.z, cam.yaw, cam.pitch],
            near: self.sim.world.terrain.describe(spot.x, spot.z),
            ..Default::default()
        })
    }

    /// Starts writing a note at the spot under the cursor, in the HUD's notes mode.
    pub fn notes_pick(&mut self, nx: f32, ny: f32, aspect: f32) -> bool {
        if !self.feedback.active || self.hud.writing.is_some() {
            return false;
        }
        self.hud.writing = self.spot_note(nx, ny, aspect);
        self.sync_pins();
        self.hud.writing.is_some()
    }

    /// The notes the HUD lists, from the host's files.
    pub fn set_notes(&mut self, notes: Vec<Note>) {
        self.hud.set_notes(notes);
        self.sync_pins();
    }

    /// Pins and the draft pin follow the HUD's notes.
    fn sync_pins(&mut self) {
        self.feedback.pins = self
            .hud
            .notes
            .iter()
            .map(|n| Pin {
                pos: n.spot,
                done: n.done,
            })
            .collect();
        self.feedback.draft = self.hud.writing.as_ref().map(|n| n.spot);
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

    /// A vehicle named in a script: its number, or a kind with an optional count, such as
    /// `cruise`, `ferry.1` or `metro` (the lead car of a subway train). Names keep playbooks
    /// working when the number of cars on the roads changes.
    /// Vehicles far from any dragon move only now and then, so one about to have a dragon
    /// put on it is brought up to now first.
    fn refresh_vehicle(&mut self, i: usize) {
        let time = self.sim.tick as f32 * DT;
        self.sim.world.traffic.refresh(i, time);
    }

    pub fn vehicle(&self, name: &str) -> Result<usize, String> {
        if let Ok(i) = name.parse::<usize>() {
            return Ok(i);
        }
        let (kind, n) = match name.split_once('.') {
            Some((k, n)) => (k, n.parse::<usize>().map_err(|e| e.to_string())?),
            None => (name, 0),
        };
        use zawnabout_sim::traffic::METRO_CAB;
        self.sim
            .world
            .traffic
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| match kind {
                // A train's engine, or a subway train's lead car.
                "train" => v.kind == zawnabout_sim::town::Kind::Train && v.variant == 0,
                "metro" => v.kind == zawnabout_sim::town::Kind::Train && v.variant == METRO_CAB,
                // A mine train's engine.
                "mine" => {
                    v.kind == zawnabout_sim::town::Kind::Train
                        && v.variant == zawnabout_sim::traffic::MINE_ENGINE
                }
                // A cave tour train's engine.
                "tour" => {
                    v.kind == zawnabout_sim::town::Kind::Train
                        && v.variant == zawnabout_sim::traffic::TOUR_ENGINE
                }
                _ => format!("{:?}", v.kind).eq_ignore_ascii_case(kind),
            })
            .nth(n)
            .map(|(i, _)| i)
            .ok_or_else(|| format!("no vehicle {name}"))
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
            ["not", rest @ ..] => !self.check(rest)?,
            ["landed"] => {
                d.mode == Mode::Ground && d.swoop.is_none() && self.land_request.is_none()
            }
            ["ground"] => d.mode == Mode::Ground,
            ["air"] => d.mode == Mode::Air,
            ["near", ..] => feet.distance(Vec3::new(num(1)?, num(2)?, num(3)?)) <= num(4)?,
            ["found", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::is_found(play, id).ok_or("no such find")?
            }
            ["lair", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::lair_id(play) == Some(*id)
            }
            ["card"] => !self.hud.book.cards.is_empty(),
            ["can-talk"] => self.listener().is_some(),
            ["riding"] => self.play.as_ref().and_then(crate::play::rider).is_some(),
            // met N: named dragon N is met. egg N: the egg is 0 not had, 1 given, 2
            // hatched. busy: a dragon is leading the way or racing.
            ["met", n] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::kin_met(play, n.parse::<usize>().map_err(|_| "a number")?)
            }
            ["egg", n] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::egg(play).to_string() == *n
            }
            ["busy"] => self.flock.busy().is_some(),
            // guide: the way to where what the dragon carries is going shows.
            ["guide"] => self.hud.goal.is_some(),
            ["say"] => self.hud.say.is_some(),
            ["toast"] => self.hud.toast.is_some(),
            // flown ID: a stunt done, a course flown or a race won.
            ["flown", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::flown(play, id).ok_or("no such stunt, course or race")?
            }
            ["racing"] => self.play.as_ref().is_some_and(crate::play::racing),
            // story-done ID: story ID is told.
            ["story-done", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::story_done(play, id).ok_or("no such story")?
            }
            // dark: the lights are out where the dragon is.
            ["dark"] => {
                self.play
                    .as_ref()
                    .and_then(|p| crate::play::dark_here(p, d.pos))
                    == Some(true)
            }
            // gate ID N: course ID's gate N is passed.
            ["gate", id, _] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::gate_passed(play, id, num(2)? as usize).ok_or("no such gate")?
            }
            ["helped", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                crate::play::helped(play, id).ok_or("no such local")?
            }
            ["halted"] => self.halted(),
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
            ["riding", which] => {
                d.mode == Mode::Ground
                    && self
                        .sim
                        .world
                        .carrier_keeping(d.riding, d.pos.x, d.pos.z, feet.y)
                        == Some(self.vehicle(which)?)
            }
            ["speed-below", _] => d.speed() < num(1)?,
            // feet-above y: the feet are higher than y, such as up on a ship's top deck.
            ["feet-above", _] => feet.y > num(1)?,
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
            ["land-on", which] => {
                let i = self.vehicle(which)?;
                self.refresh_vehicle(i);
                self.land_request = Some(self.deck(i).ok_or("no such vehicle")?);
                Ok(String::new())
            }
            ["ride", which] => {
                let i = self.vehicle(which)?;
                self.refresh_vehicle(i);
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
            ["sound", on] => {
                self.muted = *on == "off";
                Ok(String::new())
            }
            ["hud", on] => {
                self.hud.on = *on == "on";
                Ok(String::new())
            }
            ["hud-key", code] | ["ui-key", code] => {
                self.ui_key(code, None);
                Ok(String::new())
            }
            // ui-click nx ny: clicks the game's own screens at a fraction of the screen.
            ["ui-click", ..] => Ok(format!("{}", self.ui_click(num(1)?, num(2)?))),
            // hud-type words: types into the note being written.
            ["hud-type", ..] => {
                let text = line.split_once(' ').map(|(_, t)| t).unwrap_or("");
                self.ui_key("", Some(text));
                Ok(String::new())
            }
            ["book", "open"] => {
                self.open_book();
                Ok(String::new())
            }
            ["book", "close"] => {
                self.hud.book.open = false;
                Ok(String::new())
            }
            ["book", page] => {
                let p = crate::book::Page::from_name(page).ok_or("no such page")?;
                self.hud.book.page = p;
                self.open_book();
                Ok(String::new())
            }
            ["touch", on] => {
                self.touch = *on == "on";
                Ok(String::new())
            }
            // play on|off: gameplay on top of the islands, or the islands alone.
            ["play", "off"] => {
                self.play = None;
                self.hud.book.cards.clear();
                Ok(String::new())
            }
            ["play", "on"] => {
                if self.play.is_none() {
                    self.play = crate::play::start(&self.sim.world);
                }
                Ok(if self.play.is_some() {
                    String::new()
                } else {
                    "gameplay is compiled out".into()
                })
            }
            ["play", "reset"] => {
                let play = self.play.as_mut().ok_or("gameplay is off")?;
                crate::play::reset(play);
                Ok(String::new())
            }
            ["play"] => Ok(match &self.play {
                Some(p) => crate::play::status(p),
                None => "gameplay off".into(),
            }),
            ["finds"] => Ok(self
                .play
                .as_ref()
                .map_or("gameplay off".into(), crate::play::list)),
            // folk-search R: looks this far for the nearest townsperson, for goto-folk.
            ["folk-search", _] => {
                self.folk_search = num(1)?;
                Ok(String::new())
            }
            // goto-folk: stands the dragon right by the townsperson found.
            ["goto-folk"] => {
                let (p, _) = self.near_folk.get().ok_or("nobody found nearby")?;
                let d = self.player_dragon().pos;
                let away = Vec3::new(d.x - p.x, 0.0, d.z - p.z).normalize_or(Vec3::X) * 4.0;
                self.stand_at(p + away);
                Ok(format!("folk at ({:.1},{:.1},{:.1})", p.x, p.y, p.z))
            }
            ["rider-bits"] => Ok(format!(
                "at {:?} say {:?} point {:?} screen {:?} folk {:?} drawn {:?}",
                self.hud.rider_at,
                self.hud.say,
                self.hud.point,
                self.hud.book.screen(),
                self.near_folk.get(),
                self.near_drawn.get()
            )),
            // best-view ID: where a postcard's picture would be taken from.
            ["best-view", id] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                let i = crate::play::card_index(play, id).ok_or("no such postcard")?;
                let (at, r) = crate::play::card_spot(play, i).ok_or("no spot")?;
                let low = crate::play::card_low(play, i);
                Ok(format!("{:?}", self.best_view(at, r, low)))
            }
            ["ask-rider"] => {
                if self.ask_rider() {
                    Ok("asked".into())
                } else {
                    Err("nobody riding".into())
                }
            }
            ["bubble"] => {
                let (at, _) = self.bubble_on_screen().ok_or("no bubble")?;
                Ok(format!("bubble {:.4} {:.4}", at.x, at.y))
            }
            ["play-save"] => self
                .save_text()
                .ok_or_else(|| "gameplay is off".to_string()),
            // goto-find ID [AWAY]: stands the dragon AWAY meters (8 unless given) back
            // from a find or a lair, facing it. 0 stands right on it.
            ["goto-find", id, ..] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                let at = crate::play::find(play, id)
                    .or_else(|| crate::play::lair_at(play, id))
                    .ok_or("no such find or lair")?;
                let away = if words.len() > 2 { num(2)? } else { 8.0 };
                let heading = self.player_dragon().heading;
                self.stand_at(at - Vec3::new(heading.sin(), 0.0, heading.cos()) * away);
                Ok(self.state())
            }
            // thanks all: every town's thanks goes up, for a look at them.
            ["thanks", "all"] => {
                let play = self.play.as_mut().ok_or("gameplay is off")?;
                crate::play::thank_all(play);
                Ok(String::new())
            }
            // kin-state: where every other dragon is and what it is doing.
            ["kin-state"] => Ok(self
                .flock
                .dragons
                .iter()
                .chain(self.flock.hatchling.iter())
                .map(|f| {
                    format!(
                        "({:.0},{:.0},{:.0}) {:?} {:?} settled={}",
                        f.d.pos.x,
                        f.d.pos.y,
                        f.d.pos.z,
                        f.d.mode,
                        f.task,
                        f.settled()
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")),
            // goto-kin N [AWAY]: standing near named dragon N (or `little` for the
            // hatchling), wherever it is now.
            ["goto-kin", which, ..] => {
                let f = if *which == "little" {
                    self.flock.hatchling.as_ref()
                } else {
                    self.flock
                        .dragons
                        .get(which.parse::<usize>().map_err(|_| "a number")?)
                };
                let at =
                    f.ok_or("no such dragon yet")?.d.pos - Vec3::Y * self.sim.flight.stand_height;
                let away = if words.len() > 2 { num(2)? } else { 8.0 };
                let heading = self.player_dragon().heading;
                self.stand_at(at - Vec3::new(heading.sin(), 0.0, heading.cos()) * away);
                Ok(self.state())
            }
            // try-gate ID [N]: flying at a stunt's gate (or course ID's gate N), a little
            // way back, headed through it.
            ["try-gate", id, ..] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                let n = if words.len() > 2 { num(2)? as usize } else { 0 };
                let (at, dir) = crate::play::gate_way(play, &self.sim.world, id, n)
                    .ok_or("no such gate, or no clear way through")?;
                let player = self.player;
                let d = self.sim.dragon_mut(player).expect("player dragon exists");
                *d = Dragon::new(player, at - dir * 6.0, dir.x.atan2(dir.z));
                d.mode = Mode::Air;
                d.vel = dir * 25.0;
                let d = d.clone();
                if let Some((_, v)) = self.visuals.iter_mut().find(|(id, _)| *id == player) {
                    v.snap(&d);
                }
                self.camera.reset();
                self.update_camera(0.0);
                Ok(format!("through ({:.1},{:.1},{:.1})", at.x, at.y, at.z))
            }
            // story ID start|stop: starts story ID, or sets it aside.
            ["story", id, what] => {
                let play = self.play.as_mut().ok_or("gameplay is off")?;
                let s = crate::play::story_index(play, id).ok_or("no such story")?;
                if *what == "start" {
                    crate::play::begin(play, s);
                } else {
                    crate::play::set_aside(play);
                }
                Ok(String::new())
            }
            // goto-lamp K: standing beside lamp K of the story under way, facing it.
            ["goto-lamp", _] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                let at = crate::play::lamp_at(play, num(1)? as usize).ok_or("no such lamp")?;
                let heading = self.player_dragon().heading;
                self.stand_at(at - Vec3::new(heading.sin(), 0.0, heading.cos()) * 4.0);
                Ok(self.state())
            }
            // goto-race ID start|end: standing at a race's start or finish.
            ["goto-race", id, end] => {
                let play = self.play.as_ref().ok_or("gameplay is off")?;
                let (x, z) =
                    crate::play::race_spot(play, id, *end == "end").ok_or("no such race")?;
                self.stand_at(Vec3::new(x, 5000.0, z));
                Ok(self.state())
            }
            // note-pick nx ny: starts a note at a spot on a 16:9 screen, in notes mode.
            ["note-pick", ..] => {
                if self.notes_pick(num(1)?, num(2)?, 16.0 / 9.0) {
                    Ok(String::new())
                } else {
                    Err("no surface there".into())
                }
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
            // sdf x y z: how far that point is from rock, negative inside it.
            ["sdf", _, _, _] => {
                let p = Vec3::new(num(1)?, num(2)?, num(3)?);
                Ok(format!("sdf {p} {:.1}", self.sim.world.terrain.sdf(p)))
            }
            // props-near x y z r: every prop within r of the point, nearest first.
            ["props-near", _, _, _, _] => {
                let (p, r) = (Vec3::new(num(1)?, num(2)?, num(3)?), num(4)?);
                let mut near: Vec<_> = self
                    .sim
                    .world
                    .props
                    .iter()
                    .filter(|q| q.pos.distance(p) < r)
                    .collect();
                near.sort_by(|a, b| a.pos.distance(p).total_cmp(&b.pos.distance(p)));
                Ok(near
                    .iter()
                    .map(|q| {
                        format!(
                            "{:?} v{} x{:.1} at ({:.1},{:.1},{:.1})",
                            q.kind, q.variant, q.scale, q.pos.x, q.pos.y, q.pos.z
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            // skip secs: moves the clock on that far at once, every vehicle to where it
            // is by then, without playing the time out. For setting up a test quickly.
            ["skip", _] => {
                let n = (num(1)?.max(0.0) / DT).round() as u64;
                self.sim.tick += n;
                self.time += (n as f32 * DT) as f64;
                let t = self.sim.tick as f32 * DT;
                let traffic = &mut self.sim.world.traffic;
                traffic.update(t);
                for i in 0..traffic.vehicles.len() {
                    traffic.refresh(i, t);
                }
                Ok(self.state())
            }
            // ground x z: the height of the top surface there.
            ["ground", _, _] => {
                let (x, z) = (num(1)?, num(2)?);
                Ok(format!(
                    "ground {x} {z} {:.1}",
                    self.sim.world.terrain.height(x, z)
                ))
            }
            // camera: how far the camera is from the dragon and from rock.
            ["camera"] => {
                let c = self.camera.pos;
                let d = self.player_dragon().pos + Vec3::Y;
                Ok(format!(
                    "camera off dragon {:.1} m, off rock {:.1} m, at {:.1} {:.1} {:.1}",
                    (c - d).length(),
                    self.sim.world.terrain.sdf(c),
                    c.x,
                    c.y,
                    c.z
                ))
            }
            _ => Err(format!("unknown command: {line}")),
        }
    }

    /// Stands the dragon on the first surface below `at`, keeping its heading.
    fn stand_at(&mut self, at: Vec3) {
        let heading = self.player_dragon().heading;
        let ground = self.sim.world.support(at.x, at.z, at.y + 3.0);
        let flight = self.sim.flight.clone();
        let player = self.player;
        let d = self.sim.dragon_mut(player).expect("player dragon exists");
        *d = Dragon::new(
            player,
            Vec3::new(at.x, ground + flight.stand_height, at.z),
            heading,
        );
        let d = d.clone();
        if let Some((_, v)) = self.visuals.iter_mut().find(|(id, _)| *id == player) {
            v.snap(&d);
        }
        self.camera.reset();
        self.update_camera(0.0);
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
