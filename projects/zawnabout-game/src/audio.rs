//! Sound, all made here from noise and simple tones, no recordings: wind that rises with
//! speed and height, surf along the shore, rain, waterfalls, the murmur of towns,
//! birdsong over the land by day, church bells on the hour, a fairground organ, and
//! thunder that follows the lightning late by the speed of sound.
//!
//! [`Listener`] runs with the game and says what can be heard this frame, as a [`Mix`].
//! [`Synth`] runs wherever the host makes sound and turns the latest mix into samples.

use glam::{Vec2, Vec3};

use crate::game::Game;
use crate::weather::Weather;

/// How loud each sound is, 0 to 1, and the one-off sounds that start now.
#[derive(Clone, Debug, Default)]
pub struct Mix {
    pub wind: f32,
    /// Higher and brighter with speed, 0 to 1.
    pub rush: f32,
    pub surf: f32,
    pub rain: f32,
    pub falls: f32,
    pub bustle: f32,
    pub birds: f32,
    pub organ: f32,
    /// Everything at once: 0 when the sound is off.
    pub master: f32,
    pub events: Vec<Event>,
}

/// A sound that plays once.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// Thunder: after `delay` seconds, how loud, and whether it is near enough to crack.
    Thunder { delay: f32, loud: f32, crack: bool },
    /// A church bell striking `count` times.
    Bell { count: u32, loud: f32 },
}

/// The speed of sound (m/s), for thunder.
const SOUND: f32 = 343.0;
/// How far the hour bells carry (m).
const BELL_REACH: f32 = 250.0;

/// What the game's world sounds like where the camera is.
pub struct Listener {
    /// The last lightning heard of, by when it struck.
    heard: f32,
    hour: f32,
    towers: Vec<Vec3>,
    fairs: Vec<Vec3>,
    homes: Vec<Vec2>,
}

impl Listener {
    pub fn new(game: &Game) -> Self {
        use zawnabout_sim::town::Kind;
        let world = &game.sim.world;
        let of = |kinds: &[Kind]| {
            world
                .buildings
                .iter()
                .filter(|b| kinds.contains(&b.kind))
                .map(|b| b.pos)
                .collect::<Vec<_>>()
        };
        Self {
            heard: game.time as f32,
            hour: game.sky().hour,
            towers: of(&[Kind::Church, Kind::ClockTower]),
            fairs: of(&[Kind::Carnival, Kind::ThemePark]),
            homes: world
                .buildings
                .iter()
                .filter(|b| matches!(b.kind, Kind::House | Kind::Row | Kind::Flat))
                .map(|b| Vec2::new(b.pos.x, b.pos.z))
                .collect(),
        }
    }

    /// This frame's mix. `falls` are the feet of the waterfalls.
    pub fn listen(&mut self, game: &Game, weather: &Weather, falls: &[(Vec3, f32)]) -> Mix {
        let world = &game.sim.world;
        let t = &world.terrain;
        let cam = game.camera.pos;
        let time = game.time as f32;
        let sky = game.sky();
        let ground = t.height(cam.x, cam.z);
        let up = (cam.y - ground.max(0.0)).max(0.0);
        let outside = t.daylight(cam).clamp(0.0, 1.0);
        let here = weather.here(cam, time);
        let speed = game.player_dragon().vel.length();
        let near = |d: f32, lo: f32, hi: f32| 1.0 - smooth(lo, hi, d);

        // Shore: water and land both close round, and low enough to hear the waves.
        let (mut wet, mut dry) = (0, 0);
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let reach = if k < 8 { 80.0 } else { 240.0 };
            let p = Vec2::new(cam.x, cam.z) + Vec2::new(a.cos(), a.sin()) * reach;
            if t.height(p.x, p.y) < world.terrain.def.sea_level {
                wet += 1;
            } else {
                dry += 1;
            }
        }
        // Breakers along a shore, gentler waves out on open water.
        let low = near(up, 40.0, 260.0);
        let shore = ((wet.min(dry) as f32 / 5.0).min(1.0)).max(wet as f32 / 16.0 * 0.35) * low;
        let homes = self
            .homes
            .iter()
            .filter(|h| h.distance_squared(Vec2::new(cam.x, cam.z)) < 220.0 * 220.0)
            .count() as f32;
        let fair = self
            .fairs
            .iter()
            .map(|p| p.distance(cam))
            .fold(f32::MAX, f32::min);
        let roar = falls
            .iter()
            // Loud only right by the falls, falling away fast.
            .map(|(p, w)| near(p.distance(cam), 15.0 + w, 90.0 + w * 3.0).powi(2))
            .fold(0.0f32, f32::max);
        let day = 1.0 - sky.night;
        let land = (ground > world.terrain.def.sea_level + 1.0) as u8 as f32;

        let mut mix = Mix {
            // Gentle at rest, and capped low however fast or high you fly.
            wind: (0.18
                + (speed / 55.0).min(1.0) * 0.17
                + smooth(150.0, 900.0, up) * 0.08
                + here.mist * 0.07)
                * (0.25 + 0.75 * outside),
            rush: (speed / 70.0).min(1.0),
            surf: shore * outside,
            rain: here.rain,
            falls: roar,
            bustle: (homes / 60.0).min(1.0) * near(up, 30.0, 220.0) * (0.4 + 0.6 * day),
            birds: day * land * near(up, 25.0, 160.0) * outside * (1.0 - here.rain),
            // Falling away with distance like a real sound, faint from a few hundred
            // meters and gone past 600.
            organ: (40.0 / fair.max(40.0)).powf(1.5) * near(fair, 300.0, 600.0) * outside,
            // Silent while muted or paused.
            master: if game.muted || game.paused { 0.0 } else { 1.0 },
            events: Vec::new(),
        };

        // Thunder from every strike since the last frame, late by its distance.
        for s in weather.strikes(time, (time - self.heard).clamp(0.0, 2.0)) {
            if s.time <= self.heard {
                continue;
            }
            let d = s.at.distance(cam);
            if d < 9000.0 {
                mix.events.push(Event::Thunder {
                    delay: d / SOUND,
                    loud: (1.0 - d / 9000.0).powf(1.5),
                    crack: d < 1800.0,
                });
            }
        }
        self.heard = time;

        // Bells ring out the hour, heard only close to a tower.
        if sky.hour.floor() != self.hour.floor() && (sky.hour - self.hour).rem_euclid(24.0) < 1.0 {
            let d = self
                .towers
                .iter()
                .map(|p| p.distance(cam))
                .fold(f32::MAX, f32::min);
            if d < BELL_REACH {
                let h = sky.hour.floor() as u32 % 12;
                mix.events.push(Event::Bell {
                    count: if h == 0 { 12 } else { h },
                    loud: (1.0 - d / BELL_REACH).powi(2),
                });
            }
        }
        self.hour = sky.hour;
        mix
    }
}

fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A one-pole low-pass filter.
#[derive(Clone, Copy, Default)]
struct Low(f32);

impl Low {
    fn run(&mut self, x: f32, a: f32) -> f32 {
        self.0 += a * (x - self.0);
        self.0
    }
}

/// The filter step for a cut-off frequency at a sample rate.
fn coef(hz: f32, rate: f32) -> f32 {
    1.0 - (-std::f32::consts::TAU * hz / rate).exp()
}

/// A sound playing out: what it is, how far in (s), and how loud.
#[derive(Clone, Copy)]
struct Voice {
    event: Event,
    at: f32,
}

/// The organ's tune: a lilting fairground waltz, in semitones from A4, and a rest as
/// `None`, three beats to the bar.
const TUNE: [Option<i32>; 24] = [
    Some(3),
    Some(7),
    Some(10),
    Some(15),
    Some(10),
    Some(7),
    Some(5),
    Some(8),
    Some(12),
    Some(17),
    Some(12),
    Some(8),
    Some(3),
    Some(7),
    Some(10),
    Some(14),
    Some(12),
    Some(10),
    Some(8),
    Some(7),
    Some(5),
    Some(3),
    None,
    None,
];

/// Turns mixes into stereo samples, smoothly following each new mix.
pub struct Synth {
    target: Mix,
    now: [f32; 8],
    /// The master level, gliding to the mix's.
    master: f32,
    seed: u32,
    lows: [Low; 16],
    time: f64,
    voices: Vec<Voice>,
    /// The next chirp: when it starts (s) and its pitch.
    chirp: (f64, f32, u32),
    pub volume: f32,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            target: Mix::default(),
            now: [0.0; 8],
            master: 0.0,
            seed: 0x1234_5678,
            lows: [Low::default(); 16],
            time: 0.0,
            voices: Vec::new(),
            chirp: (0.5, 3000.0, 3),
            volume: 0.6,
        }
    }

    /// Takes this frame's mix: levels to glide toward, and one-off sounds to start.
    pub fn set(&mut self, mut mix: Mix) {
        // Sounds that start while it is off are not heard later either.
        if mix.master <= 0.0 {
            mix.events.clear();
        }
        for e in mix.events.drain(..) {
            if self.voices.len() < 24 {
                self.voices.push(Voice { event: e, at: 0.0 });
            }
        }
        self.target = mix;
    }

    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// Fills `out` with interleaved stereo samples at `rate` per second.
    pub fn fill(&mut self, out: &mut [f32], rate: f32) {
        let dt = 1.0 / rate;
        let glide = coef(2.0, rate);
        let m = &self.target;
        let goal = [
            m.wind, m.rush, m.surf, m.rain, m.falls, m.bustle, m.birds, m.organ,
        ];
        let c = |hz: f32| coef(hz, rate);
        for frame in out.chunks_mut(2) {
            for (n, g) in self.now.iter_mut().zip(goal) {
                *n += (g - *n) * glide;
            }
            let [wind, rush, surf, rain, falls, bustle, birds, organ] = self.now;
            let t = self.time as f32;
            let (wa, wb, n3) = (self.noise(), self.noise(), self.noise());
            // Wind: low rushing noise with slow gusts, brighter at speed.
            let gust = 0.6 + 0.4 * (self.lows[0].run(n3, c(0.4)) * 6.0).clamp(-1.0, 1.0);
            let bright = c(180.0 + rush * 900.0);
            let wl = self.lows[2].run(wa, bright);
            let wl = self.lows[1].run(wl, bright) * 2.4;
            let wr = self.lows[4].run(wb, bright);
            let wr = self.lows[3].run(wr, bright) * 2.4;
            let mut l = (wl * wind * gust) * 0.9;
            let mut r = (wr * wind * gust) * 0.9;
            // Surf: swells that build and break every few seconds.
            // Breakers with near quiet between, not a steady hiss.
            let swell = ((t * std::f32::consts::TAU / 7.0).sin().max(0.0)).powi(3) * 0.9 + 0.05;
            let sea = self.lows[5].run(wa + wb, c(600.0)) * surf * swell * 0.65;
            l += sea;
            r += sea * 0.9;
            // Rain: a hiss with patters, and the falls' steady roar.
            let hiss = n3 - self.lows[6].run(n3, c(2500.0));
            let drop = if self.noise() > 0.995 {
                self.noise()
            } else {
                0.0
            };
            let wet = (hiss * 0.3 + drop) * rain * 0.5;
            let roar = self.lows[7].run(wa, c(900.0)) * falls * 0.9;
            l += wet + roar;
            r += wet + roar;
            // Towns: a soft murmur of voices and goings on.
            // Broken up by an uneven chatter of voices, so it reads as people, not hiss.
            let band = self.lows[8].run(wb, c(700.0)) - self.lows[9].run(wb, c(250.0));
            let chatter = (self.lows[13].run(n3, c(5.0)) * 14.0).clamp(0.0, 1.0);
            let busy = band * bustle * (0.25 + 0.75 * chatter) * 0.45;
            l += busy;
            r += busy;
            // Birds: little runs of chirps, now one side, now the other.
            if birds > 0.02 {
                let (start, pitch, notes) = self.chirp;
                let since = (self.time - start) as f32;
                if since > 0.0 {
                    let k = (since / 0.12) as u32;
                    let p = since - k as f32 * 0.12;
                    if k < notes && p < 0.07 {
                        let f = pitch * (1.0 + p * 4.0) * (1.0 + 0.05 * k as f32);
                        let s =
                            (std::f32::consts::TAU * f * p).sin() * (1.0 - p / 0.07) * birds * 0.12;
                        let pan = ((start * 7.3).sin() as f32) * 0.5 + 0.5;
                        l += s * (1.0 - pan);
                        r += s * pan;
                    } else if k >= notes {
                        let a = self.noise() * 0.5 + 0.5;
                        let b = self.noise() * 0.5 + 0.5;
                        self.chirp = (
                            self.time + 0.3 + a as f64 * 2.5,
                            2400.0 + b * 2200.0,
                            2 + (a * 4.0) as u32,
                        );
                    }
                }
            }
            // The fairground organ, round and reedy.
            if organ > 0.01 {
                let beat = 0.32;
                let step = ((t / beat) as usize) % TUNE.len();
                if let Some(semi) = TUNE[step] {
                    let f = 440.0 * 2f32.powf(semi as f32 / 12.0);
                    let p = t % beat;
                    let env = (p / 0.02).min(1.0) * (1.0 - p / beat * 0.6);
                    let ph = std::f32::consts::TAU * f * t;
                    let tone = ph.sin() + (ph * 3.0).sin() * 0.3 + (ph * 2.0).sin() * 0.2;
                    let bass = (std::f32::consts::TAU * f * 0.25 * t).sin() * 0.5;
                    let s = (tone * 0.14 + bass * 0.08) * env * organ;
                    l += s;
                    r += s;
                }
            }
            // One-off sounds.
            let mut i = 0;
            while i < self.voices.len() {
                let v = self.voices[i];
                let (s, done) = match v.event {
                    Event::Thunder { delay, loud, crack } => {
                        let p = v.at - delay;
                        if p < 0.0 {
                            (0.0, false)
                        } else {
                            let env = (p / 0.06).min(1.0) * (-p / 2.2).exp();
                            let low = self.lows[11].run(n3, c(90.0));
                            let rumble = self.lows[10].run(low, c(90.0)) * 9.0;
                            let snap = if crack && p < 0.4 {
                                self.lows[12].run(wa, c(1800.0)) * (1.0 - p / 0.4) * 1.5
                            } else {
                                0.0
                            };
                            (
                                (rumble * (0.7 + 0.3 * (p * 3.1).sin()) + snap) * env * loud,
                                p > 8.0,
                            )
                        }
                    }
                    Event::Bell { count, loud } => {
                        let k = (v.at / 1.8) as u32;
                        let p = v.at - k as f32 * 1.8;
                        let f = 220.0;
                        let tone = [1.0f32, 2.0, 2.76, 5.4]
                            .iter()
                            .zip([0.6f32, 0.35, 0.25, 0.12])
                            .map(|(h, a)| (std::f32::consts::TAU * f * h * p).sin() * a)
                            .sum::<f32>();
                        let s = if k < count {
                            tone * (-p * 1.6).exp() * loud * 0.3
                        } else {
                            0.0
                        };
                        (s, k >= count + 2)
                    }
                };
                l += s;
                r += s;
                self.voices[i].at += dt;
                if done {
                    self.voices.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            self.master += (self.target.master - self.master) * glide * 4.0;
            let gain = self.volume * self.master;
            frame[0] = (l * gain).tanh();
            if frame.len() > 1 {
                frame[1] = (r * gain).tanh();
            }
            self.time += dt as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every sound at once stays within range and is not silent.
    #[test]
    fn synth_is_bounded() {
        let mut s = Synth::new();
        s.set(Mix {
            wind: 1.0,
            rush: 1.0,
            surf: 1.0,
            rain: 1.0,
            falls: 1.0,
            bustle: 1.0,
            birds: 1.0,
            organ: 1.0,
            master: 1.0,
            events: vec![
                Event::Thunder {
                    delay: 0.1,
                    loud: 1.0,
                    crack: true,
                },
                Event::Bell {
                    count: 3,
                    loud: 1.0,
                },
            ],
        });
        let mut out = vec![0.0f32; 48_000 * 2];
        s.fill(&mut out, 48_000.0);
        assert!(out.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        let loud = out.iter().map(|v| v * v).sum::<f32>() / out.len() as f32;
        assert!(loud > 1e-4, "too quiet: {loud}");
    }
}

#[cfg(all(test, feature = "probes"))]
mod levels {
    use super::*;

    /// Probe: how loud each sound is on its own at full level, as RMS.
    /// `cargo test --release -p zawnabout-game --features probes levels -- --nocapture`
    #[test]
    #[cfg(feature = "probes")]
    fn each_sound_alone() {
        for k in 0..8 {
            let mut v = [0.0f32; 8];
            v[k] = 1.0;
            let mut s = Synth::new();
            s.set(Mix {
                wind: v[0],
                rush: v[1],
                surf: v[2],
                rain: v[3],
                falls: v[4],
                bustle: v[5],
                birds: v[6],
                organ: v[7],
                master: 1.0,
                events: Vec::new(),
            });
            let mut out = vec![0.0f32; 48_000 * 6];
            s.fill(&mut out, 48_000.0);
            let tail = &out[48_000 * 2..];
            let rms = (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt();
            let name = [
                "wind", "rush", "surf", "rain", "falls", "bustle", "birds", "organ",
            ][k];
            println!("{name:7} rms {rms:.3}");
        }
    }
}
