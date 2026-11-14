//! Weather, only for looks and a pure function of the clock: fair weather clouds and big
//! towering cumulus drifting on the wind across the whole world, some of them dark rain
//! clouds trailing grey shafts of rain. Flying into a cloud whites everything out in
//! mist, flying under a rain cloud brings rain all round, and the big clouds cast soft
//! shadows on the ground below. A few rain clouds are thunderstorms, towering up to an
//! anvil, with lightning that flashes across the sky.

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::World;
use zawnabout_sim::shape::hash;
use zawnabout_sim::world::PropKind;

use crate::mesh::{MeshBuilder, Rgb};
use crate::models;
use crate::render::{Instance, MeshId, Renderer};

/// The wind the clouds drift on (m/s).
const WIND: Vec2 = Vec2::new(3.4, 1.2);
/// A cloud piece at unit scale: its half size across, and how far it reaches below and
/// above its origin, as the cloud meshes build it (a flat belly, a rounded top).
const PIECE: Vec2 = Vec2::new(24.0, 13.0);
const PIECE_BELOW: f32 = 0.5;
const PIECE_ABOVE: f32 = 11.0;
/// Beyond this a cloud piece draws as its far mesh (m).
const FAR: f32 = 1200.0;
/// Big clouds keep their puffs out to here, and are round lumps beyond (m).
const MID: f32 = 3500.0;
/// Clouds farther than this are not drawn at all (m).
const RANGE: f32 = 7000.0;
/// Big clouds whose shadows the ground shows at once, nearest first.
pub const SHADOWS: usize = 16;
/// Big towering clouds, and how many of them rain.
const BIG: u32 = 36;
const RAINY: u32 = 14;
/// The first rain clouds are thunderstorms.
const STORMS: u32 = 4;
/// How long a bolt shows (s), and how far its flash lights the sky (m).
const BOLT: f32 = 0.4;
const FLASH_REACH: f32 = 7000.0;
/// Rain streaks fill cells of this size round the camera, out to this far (m).
const STREAK_CELL: f32 = 5.0;
const STREAK_RANGE: f32 = 40.0;

/// One cloud: where it starts, what it is made of, and whether it rains.
struct Cloud {
    home: Vec3,
    yaw: f32,
    /// Its pieces: offset from its middle (m), scale and mesh variant.
    pieces: Vec<(Vec3, f32, u8)>,
    /// Its body, for flying into it: the middle, above its home, and half its size.
    mid: f32,
    half: Vec3,
    /// How far its belly hangs below its home, where shadows and rain fall from.
    below: f32,
    rain: bool,
    storm: bool,
}

/// A lightning strike: where it comes down, when, and its seed for its shape.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub at: Vec3,
    pub time: f32,
    pub seed: u32,
}

/// What the weather is doing where the camera is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Here {
    /// How far inside a cloud, 0 out in clear air to 1 deep in.
    pub mist: f32,
    /// How hard it is raining, 0 to 1.
    pub rain: f32,
    /// How bright a lightning flash lights everything this moment, 0 to 1.
    pub flash: f32,
}

pub struct Weather {
    clouds: Vec<Cloud>,
    near: MeshId,
    /// Big clouds farther off, still round; small far ones are lumps.
    mid: MeshId,
    far: MeshId,
    shaft: MeshId,
    streak: MeshId,
    /// A soft glowing disc, for wisps of mist.
    wisp: MeshId,
    /// Clouds drift across this square and come round again (half its side, m).
    half: f32,
}

impl Weather {
    /// `wisp` is the shared soft glow disc.
    pub fn new(renderer: &mut Renderer, world: &World, wisp: MeshId) -> Self {
        // One shape per detail, so every cloud in sight costs three draws at most: each
        // piece is stretched and turned its own way instead.
        let v = 1;
        let near_mesh = &models::prop_variants(PropKind::Cloud)[v as usize];
        let near = renderer.add_mesh(near_mesh);
        let mid = renderer.add_mesh(&models::cloud_mid(v));
        let far = match models::prop_far(PropKind::Cloud, v) {
            Some(m) => renderer.add_mesh(&m),
            None => near,
        };
        let half = world.terrain.half_size() + 1500.0;
        // The world's scatter of fair weather clouds, each one piece.
        let mut clouds: Vec<Cloud> = world
            .props
            .iter()
            .filter(|p| p.kind == PropKind::Cloud)
            .map(|p| {
                let (below, above) = (PIECE_BELOW * p.scale, PIECE_ABOVE * p.scale);
                Cloud {
                    home: p.pos,
                    yaw: p.yaw,
                    pieces: vec![(Vec3::ZERO, p.scale, p.variant)],
                    mid: (above - below) * 0.5,
                    half: Vec3::new(PIECE.x, 0.0, PIECE.y) * p.scale
                        + Vec3::Y * (above + below) * 0.5,
                    below,
                    rain: false,
                    storm: false,
                }
            })
            .collect();
        // Big towering cumulus: a broad base of pieces with more heaped on top, some with
        // a dark belly that rains.
        for k in 0..BIG {
            let r = |q: u32| hash(k.wrapping_mul(2_654_435_761).wrapping_add(q * 40_503));
            let home = Vec3::new(
                (r(1) * 2.0 - 1.0) * half,
                320.0 + r(2) * 260.0,
                (r(3) * 2.0 - 1.0) * half,
            );
            let s = 5.0 + r(4) * 4.0;
            let mut pieces = Vec::new();
            for i in 0..4u32 {
                let a = i as f32 / 4.0 * std::f32::consts::TAU + r(10 + i);
                let off = Vec3::new(a.cos() * 18.0, 0.0, a.sin() * 14.0);
                pieces.push((off * s, s * (0.8 + r(20 + i) * 0.3), (i % 3) as u8));
            }
            let storm = k < STORMS;
            let tiers = if storm { 4 } else { 1 + (r(5) * 3.0) as u32 };
            let mut top = PIECE_ABOVE * s;
            if storm {
                // A storm builds up solid: tiers close together, two pieces wide.
                for t in 0..5u32 {
                    let up = 9.0 + t as f32 * 8.0;
                    for side in [-1.0f32, 1.0] {
                        let off = Vec3::new(
                            side * 9.0 + (r(30 + t) - 0.5) * 6.0,
                            up,
                            (r(40 + t) - 0.5) * 8.0,
                        );
                        pieces.push((off * s, s * 0.95, ((t + 1) % 3) as u8));
                    }
                    top = top.max(up * s + PIECE_ABOVE * s);
                }
            }
            for t in 0..if storm { 0 } else { tiers } {
                let up = 12.0 + t as f32 * 11.0;
                let off = Vec3::new((r(30 + t) - 0.5) * 16.0, up, (r(40 + t) - 0.5) * 10.0);
                let ps = s * (0.85 - t as f32 * 0.12);
                pieces.push((off * s, ps, ((t + 1) % 3) as u8));
                top = top.max(up * s + PIECE_ABOVE * ps);
            }
            if storm {
                // The anvil: a broad flat top spreading out where the storm meets the
                // ceiling.
                let up = 9.0 + 5.0 * 8.0;
                for (dx, dz) in [(-22.0f32, 0.0f32), (22.0, 4.0), (0.0, -10.0), (0.0, 10.0)] {
                    pieces.push((Vec3::new(dx, up, dz) * s, s * 1.2, 0));
                }
                top = top.max(up * s + PIECE_ABOVE * s * 1.2);
            }
            let below = PIECE_BELOW * s;
            clouds.push(Cloud {
                home,
                yaw: r(6) * std::f32::consts::TAU,
                pieces,
                mid: (top - below) * 0.5,
                half: Vec3::new(42.0 * s, (top + below) * 0.5, 32.0 * s),
                below,
                rain: k < RAINY,
                storm,
            });
        }
        Self {
            clouds,
            near,
            mid,
            far,
            shaft: renderer.add_mesh(&shaft()),
            streak: renderer.add_mesh(&streak()),
            wisp,
            half,
        }
    }

    /// Where cloud `c` is at `time`, and how grown it is: clouds drift with the wind and
    /// come round again, forming out of clear air at one edge and fading at the other.
    fn at(&self, c: &Cloud, time: f32) -> (Vec3, f32) {
        let h = self.half;
        let p = Vec2::new(c.home.x, c.home.z) + WIND * time;
        let wrap = |v: f32| (v + h).rem_euclid(2.0 * h) - h;
        let (x, z) = (wrap(p.x), wrap(p.y));
        let edge = |v: f32| ((h - v.abs()) / 900.0).clamp(0.0, 1.0);
        (Vec3::new(x, c.home.y, z), edge(x) * edge(z))
    }

    /// How deep inside cloud `c` (at `at`, `grown`) a spot is: 0 at its edge or outside,
    /// 1 well in.
    fn inside(c: &Cloud, at: Vec3, grown: f32, p: Vec3) -> f32 {
        let half = c.half * grown;
        let body = at + Vec3::Y * c.mid * grown;
        if (p - body).abs().cmpgt(half).any() {
            return 0.0;
        }
        let d = (Quat::from_rotation_y(-c.yaw) * (p - body) / half).length();
        ((1.0 - d) / 0.35).clamp(0.0, 1.0)
    }

    /// Lightning that struck from the storms between `time - back` and `time`, oldest
    /// first. Each storm strikes now and then, a pure function of the clock.
    pub fn strikes(&self, time: f32, back: f32) -> Vec<Strike> {
        let mut out = Vec::new();
        for (k, c) in self.clouds.iter().enumerate().filter(|(_, c)| c.storm) {
            let k = k as u32;
            let period = 7.0 + hash(k * 31 + 5) * 6.0;
            let first = ((time - back) / period).floor() as i64;
            for slot in first..=(time / period).floor() as i64 {
                let seed = (slot as u32).wrapping_mul(2_654_435_761) ^ k.wrapping_mul(97);
                if hash(seed) > 0.75 {
                    continue;
                }
                let t = slot as f32 * period + hash(seed + 1) * period * 0.6;
                if t > time || t < time - back {
                    continue;
                }
                let (at, grown) = self.at(c, t);
                if grown < 0.6 {
                    continue;
                }
                let r = c.half.x.min(c.half.z) * 0.5;
                let a = hash(seed + 2) * std::f32::consts::TAU;
                let off = Vec3::new(a.cos(), 0.0, a.sin()) * r * hash(seed + 3);
                out.push(Strike {
                    at: Vec3::new(at.x, at.y - c.below, at.z) + off,
                    time: t,
                    seed,
                });
            }
        }
        out.sort_by(|a, b| a.time.total_cmp(&b.time));
        out
    }

    /// How bright a strike flashes `since` seconds after it: two quick pulses.
    fn flicker(since: f32) -> f32 {
        if !(0.0..BOLT).contains(&since) {
            0.0
        } else if since < 0.08 || (0.16..0.28).contains(&since) {
            1.0
        } else {
            0.3
        }
    }

    /// The weather at the camera this moment.
    pub fn here(&self, cam: Vec3, time: f32) -> Here {
        let mut out = Here::default();
        for s in self.strikes(time, BOLT) {
            let near = 1.0 - (s.at.distance(cam) / FLASH_REACH).clamp(0.0, 1.0);
            out.flash = out.flash.max(Self::flicker(time - s.time) * near * near);
        }
        for c in &self.clouds {
            let (at, grown) = self.at(c, time);
            if grown < 0.05 {
                continue;
            }
            out.mist = out.mist.max(Self::inside(c, at, grown, cam));
            // Under a rain cloud's dark belly it rains, harder toward the middle.
            if c.rain && grown > 0.5 && cam.y < at.y {
                let d = Vec2::new(cam.x - at.x, cam.z - at.z) / (c.half.x.min(c.half.z) * grown);
                out.rain = out.rain.max(((1.0 - d.length()) / 0.4).clamp(0.0, 1.0));
            }
        }
        out
    }

    /// The big clouds nearest `cam`: middle, half size and whether it rains.
    pub fn nearest(&self, cam: Vec3, time: f32, n: usize) -> Vec<(Vec3, Vec3, bool)> {
        let mut out: Vec<(Vec3, Vec3, bool)> = self
            .clouds
            .iter()
            .filter(|c| c.pieces.len() > 1)
            .map(|c| {
                let (at, grown) = self.at(c, time);
                (at + Vec3::Y * c.mid * grown, c.half * grown, c.rain)
            })
            .collect();
        out.sort_by(|a, b| a.0.distance(cam).total_cmp(&b.0.distance(cam)));
        out.truncate(n);
        out
    }

    /// The big clouds nearest the camera, for their shadows: middle (x, z), radius, and
    /// the height of their base.
    pub fn shadows(&self, cam: Vec3, time: f32) -> Vec<Vec4> {
        let mut out: Vec<(f32, Vec4)> = self
            .clouds
            .iter()
            .filter(|c| c.pieces.len() > 1)
            .filter_map(|c| {
                let (at, grown) = self.at(c, time);
                let r = c.half.x.min(c.half.z) * grown * 0.85;
                let d = Vec2::new(at.x - cam.x, at.z - cam.z).length();
                let base = at.y - c.below * grown;
                (grown > 0.1 && d < 4000.0).then(|| (d, Vec4::new(at.x, at.z, r, base)))
            })
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.into_iter().take(SHADOWS).map(|(_, v)| v).collect()
    }

    /// Clouds, rain and mist near `cam`. `night` dims the rain and mist, which only
    /// glow by day. Without `open_sky` overhead no rain falls round the camera.
    pub fn draw(
        &self,
        cam: Vec3,
        time: f32,
        night: f32,
        open_sky: bool,
        draws: &mut Vec<(MeshId, Instance)>,
        fx: &mut Vec<(MeshId, Instance)>,
    ) {
        let dim = 1.0 - night * 0.8;
        // Clouds glow a little so they stay white from below; rain clouds are grey.
        let white = Vec4::new(1.5, 1.5, 1.5, 1.0);
        let grey = Vec4::new(0.55, 0.58, 0.66, 1.0);
        let dark = Vec4::new(0.4, 0.43, 0.52, 1.0);
        // Lightning: a jagged glowing line from the belly to the ground, with a fork,
        // and the storm lit up inside for a moment.
        for s in self.strikes(time, BOLT) {
            let glow = Self::flicker(time - s.time);
            if glow <= 0.0 || s.at.distance(cam) > RANGE {
                continue;
            }
            let r = |q: u32| hash(s.seed.wrapping_add(q * 7919));
            let mut bolt = |from: Vec3, to_y: f32, steps: u32, wide: f32, q: u32| {
                let mut p = from;
                let fall = (from.y - to_y) / steps as f32;
                for i in 0..steps {
                    let j = q + i * 3;
                    let next = p + Vec3::new(
                        (r(j) - 0.5) * fall * 0.9,
                        -fall,
                        (r(j + 1) - 0.5) * fall * 0.9,
                    );
                    let d = next - p;
                    let arc = Quat::from_rotation_arc(Vec3::NEG_Y, d.normalize());
                    // Two quads crossed, so it shows from every side.
                    for spin in [0.0, std::f32::consts::FRAC_PI_2] {
                        let m = Mat4::from_scale_rotation_translation(
                            Vec3::new(wide, d.length(), wide),
                            arc * Quat::from_rotation_y(spin),
                            p,
                        );
                        fx.push((
                            self.streak,
                            Instance::tinted(m, Vec4::new(2.6, 2.6, 3.2, glow)),
                        ));
                    }
                    p = next;
                }
                p
            };
            let mid = bolt(s.at, s.at.y * 0.45, 5, 200.0, 0);
            bolt(mid, 0.0, 4, 200.0, 40);
            bolt(mid, s.at.y * 0.2, 3, 110.0, 80);
        }
        let mut mist = 0.0f32;
        for c in &self.clouds {
            let (at, grown) = self.at(c, time);
            // Small fair weather clouds fade into the haze sooner than big ones.
            let range = if c.pieces.len() > 1 {
                RANGE
            } else {
                RANGE * 0.7
            };
            if grown < 0.02 || at.distance(cam) > range {
                continue;
            }
            // From inside, a cloud is mist all round rather than its own shell.
            let deep = Self::inside(c, at, grown, cam);
            mist = mist.max(deep);
            if deep > 0.2 {
                continue;
            }
            let frame = Mat4::from_rotation_translation(Quat::from_rotation_y(c.yaw), at);
            for (off, s, v) in &c.pieces {
                let stretch = Vec3::new(0.8 + 0.22 * *v as f32, 1.0, 1.2 - 0.15 * *v as f32);
                let m = frame
                    * Mat4::from_scale_rotation_translation(
                        stretch * s * grown,
                        Quat::from_rotation_y(*v as f32 * 1.1),
                        *off * grown,
                    );
                let d = m.w_axis.truncate().distance(cam);
                let mesh = if d < FAR {
                    self.near
                } else if c.pieces.len() > 1 && d < MID {
                    self.mid
                } else {
                    self.far
                };
                let tint = if c.storm {
                    dark
                } else if c.rain {
                    grey
                } else {
                    white
                };
                draws.push((mesh, Instance::tinted(m, tint)));
            }
            // Grey streaks of rain from the belly down to the sea, thinning as they fall.
            // Seen from in under it, the rain falling round you shows instead.
            let r = c.half.x.min(c.half.z) * grown * 0.6;
            // Curtains read best from afar: closing in, they thin away and the rain all
            // round takes over.
            let away = Vec2::new(cam.x - at.x, cam.z - at.z).length();
            let fade = ((away - r * 1.2) / (r * 2.0)).clamp(0.0, 1.0);
            if c.rain && grown > 0.3 && fade > 0.01 {
                let base = at.y - c.below * grown;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(r, base, r),
                    Quat::from_rotation_y(c.yaw),
                    Vec3::new(at.x, 0.0, at.z),
                );
                fx.push((
                    self.shaft,
                    Instance::tinted(
                        m,
                        (Vec3::new(0.7, 0.74, 0.82) * dim).extend(0.45 * dim * fade),
                    ),
                ));
            }
        }
        // Wisps of cloud streaming past while flying through one.
        if mist > 0.01 {
            let cell = 14.0;
            let n = 4;
            let base = (cam / cell).floor();
            for j in -n..=n {
                for k in -n..=n {
                    for i in -n..=n {
                        let p = base + Vec3::new(i as f32, j as f32, k as f32);
                        let seed = (p.x as i32 as u32).wrapping_mul(73_856_093)
                            ^ (p.y as i32 as u32).wrapping_mul(19_349_663)
                            ^ (p.z as i32 as u32).wrapping_mul(83_492_791);
                        let r = |q: u32| hash(seed.wrapping_add(q));
                        if r(0) < 0.5 {
                            continue;
                        }
                        let at = (p + Vec3::new(r(1), r(2), r(3))) * cell;
                        let d = at.distance(cam);
                        if !(3.0..cell * n as f32).contains(&d) {
                            continue;
                        }
                        let face =
                            Quat::from_rotation_arc(Vec3::Y, (cam - at).normalize_or(Vec3::Y));
                        let m = Mat4::from_scale_rotation_translation(
                            Vec3::new(6.0, 1.0, 6.0) * (0.6 + r(4)),
                            face,
                            at,
                        );
                        let fade = mist * (1.0 - d / (cell * n as f32)) * 0.35;
                        fx.push((
                            self.wisp,
                            Instance::tinted(m, Vec3::splat(dim).extend(fade * dim)),
                        ));
                    }
                }
            }
        }
        // Rain streaks all round the camera when under a rain cloud.
        let rain = if open_sky {
            self.here(cam, time).rain
        } else {
            0.0
        };
        if rain > 0.01 {
            let n = (STREAK_RANGE / STREAK_CELL).ceil() as i32;
            let base = (cam / STREAK_CELL).floor();
            let face = Quat::from_rotation_y((cam.x - base.x).atan2(cam.z - base.z));
            for j in -n..=n {
                for k in -n..=n {
                    for i in -n / 2..=n / 2 {
                        let cell = base + Vec3::new(i as f32, j as f32, k as f32);
                        let seed = (cell.x as i32 as u32).wrapping_mul(73_856_093)
                            ^ (cell.y as i32 as u32).wrapping_mul(19_349_663)
                            ^ (cell.z as i32 as u32).wrapping_mul(83_492_791);
                        let r = |q: u32| hash(seed.wrapping_add(q));
                        if r(0) > rain * 0.8 {
                            continue;
                        }
                        // Falling fast through its cell, over and over.
                        let fall = (time * 2.2 + r(1)).fract();
                        let at = (cell + Vec3::new(r(2), 1.0 - fall, r(3))) * STREAK_CELL;
                        let d = at.distance(cam);
                        if d > STREAK_RANGE || d < 1.0 {
                            continue;
                        }
                        let m = Mat4::from_scale_rotation_translation(
                            Vec3::new(1.0, 2.2, 1.0),
                            face,
                            at,
                        );
                        let fade = (1.0 - d / STREAK_RANGE) * 0.5;
                        fx.push((
                            self.streak,
                            Instance::tinted(m, (Vec3::new(0.8, 0.85, 0.95) * dim).extend(fade)),
                        ));
                    }
                }
            }
        }
    }
}

/// A rain shaft of unit radius from the ground up to unit height: streaks hanging from
/// the belly at uneven lengths, thickest up top and thinning out as they fall, so the
/// rain looks like curtains rather than a tube.
fn shaft() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = Rgb(0xFFFFFF);
    let n = 40;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU + hash(k * 7) * 0.2;
        let r = 0.35 + hash(k * 13) * 0.65;
        let w = 0.05 + hash(k * 17) * 0.05;
        // Down to the sea, or giving out on the way.
        let reach = if hash(k * 19) < 0.6 {
            0.0
        } else {
            0.2 + hash(k * 23) * 0.5
        };
        let mid = Vec3::new(a.cos(), 0.0, a.sin()) * r;
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * w;
        let (top, foot) = (mid + Vec3::Y, mid + Vec3::Y * reach);
        let from = m.verts.len();
        m.quad(foot - side, foot + side, top + side, top - side, c);
        m.quad(top - side, top + side, foot + side, foot - side, c);
        // Fades from the belly down: vertex alpha is how much rain.
        for v in &mut m.verts[from..] {
            let up = (v.pos[1] - reach) / (1.0 - reach).max(0.01);
            v.color[3] = (up.clamp(0.0, 1.0).powf(0.7) * 255.0) as u8;
        }
    }
    m
}

/// A streak of rain: a thin quad hanging down from the origin, unit long, seen from both
/// sides.
fn streak() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = Rgb(0xFFFFFF);
    let w = Vec3::X * 0.02;
    let down = Vec3::NEG_Y;
    m.quad(-w, w, w + down, -w + down, c);
    m.quad(-w + down, w + down, w, -w, c);
    m
}
