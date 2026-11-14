//! Terrain from the compiled world: a height for any x, z, built from the island
//! outlines, ranges, mountains, plateaus and carves in the world files.
//!
//! Everything here is deterministic: only add, multiply, divide, sqrt and floor, so
//! native and wasm agree bit for bit. A coarse grid lists which features can touch each
//! cell, so a query only looks at nearby features.

pub mod def;
mod water;

use std::sync::Arc;

use glam::{Vec2, Vec3};

pub use def::{
    CarveKind, Cover, CoverKind, Decor, Density, FLOWER_VARIANTS, Flowers, Mark, MountShape, NO_Y,
    Road, Route, Style, Tech, Town, Trees, Void, VoidShape, WORLD, WorldDef,
};
pub use water::{RoadPoint, RoadSpot};

/// Index cell size (m).
const CELL: f32 = 128.0;
/// How far land shapes the seabed offshore and the slopes inland (m).
const LAND_RANGE: f32 = 600.0;
const PLATEAU_RANGE: f32 = 140.0;
const LAKE_RANGE: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Land,
    Plateau,
    Lake,
    Cover,
    Ridge,
    Carve,
    River,
    Pool,
    Town,
    Shelf,
    Road,
}

/// Which shaping a height query includes. River and road profiles are measured on the
/// ground before they cut it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Base,
    Roads,
    Full,
}

/// One feature's presence in a cell: the outline or path segments near the cell, and for
/// outlines whether the cell center is inside.
#[derive(Clone, Debug)]
struct Entry {
    kind: Kind,
    idx: u16,
    inside: bool,
    segs: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
struct Cell {
    entries: Vec<Entry>,
    mounts: Vec<u16>,
    spires: Vec<u16>,
    voids: Vec<u16>,
    pillars: Vec<u16>,
}

/// A void's bounds, for quick rejection.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

/// Bounds of a void, loose enough for its rough walls: the noise in [`void_sdf`] pushes
/// walls out by up to 1.4 times its amplitude.
pub fn void_bounds(v: &Void) -> Bounds {
    let he = Vec3::from(v.half);
    let rough = match v.shape {
        VoidShape::Hall => 1.4 * (he.y * 0.14 + 1.25),
        VoidShape::Dome | VoidShape::Shaft => 1.4 * 0.12 * he.min_element().max(he.x.min(he.z)),
        VoidShape::Tube => 1.4 * v.r * 0.16,
        VoidShape::Carved | VoidShape::Block | VoidShape::Basin => 0.0,
    };
    let pad = 8.0 + rough;
    match v.shape {
        VoidShape::Tube => {
            let (a, b) = (Vec3::from(v.a), Vec3::from(v.b));
            let r = Vec3::splat(v.r * 1.2 + pad);
            Bounds {
                min: a.min(b) - r,
                max: a.max(b) + r,
            }
        }
        VoidShape::Basin => {
            let c = Vec3::from(v.center);
            let (up, wide) = basin_opening(v.half[0].min(v.half[2]));
            let e = Vec3::new(v.half[0] + pad + wide, 0.0, v.half[2] + pad + wide);
            Bounds {
                min: c - e - Vec3::Y * (v.half[1] + pad),
                max: c + e + Vec3::Y * (up + pad),
            }
        }
        _ => {
            let c = Vec3::from(v.center);
            let flat = v.half[0].max(v.half[2]) * 1.42 + pad;
            let e = Vec3::new(flat, v.half[1] + pad, flat);
            Bounds {
                min: c - e,
                max: c + e,
            }
        }
    }
}

/// Smooth 3D value noise in `[-1, 1]`.
fn noise3(seed: u32, p: Vec3) -> f32 {
    let f = p.floor();
    let t = p - f;
    let s = t * t * (Vec3::splat(3.0) - 2.0 * t);
    let (ix, iy, iz) = (f.x as i32, f.y as i32, f.z as i32);
    let h = |dx: i32, dy: i32, dz: i32| {
        hash(
            seed ^ ((iy + dy) as u32).wrapping_mul(0x27D4_EB2F),
            ix + dx,
            iz + dz,
        )
    };
    let x00 = mix(h(0, 0, 0), h(1, 0, 0), s.x);
    let x10 = mix(h(0, 1, 0), h(1, 1, 0), s.x);
    let x01 = mix(h(0, 0, 1), h(1, 0, 1), s.x);
    let x11 = mix(h(0, 1, 1), h(1, 1, 1), s.x);
    mix(mix(x00, x10, s.y), mix(x01, x11, s.y), s.z) * 2.0 - 1.0
}

/// How wide a lake's banks slope down to it from higher ground round it (m).
const LAKE_BANK: f32 = 25.0;

/// How far a cave lake basin opens up above its water, so it always meets the cavern,
/// and how far it widens on the way up. Bumps in a cavern floor would otherwise leave flat
/// skins of rock hanging over the lake. Small pools open less, so they stay sealed under
/// the seabed.
fn basin_opening(small: f32) -> (f32, f32) {
    let up = (small * 0.4).clamp(20.0, 60.0);
    (up, (up * 0.8).min(small * 0.4))
}

/// Signed distance to one void, negative inside.
pub fn void_sdf(v: &Void, p: Vec3) -> f32 {
    let rough = |scale: f32, amp: f32| {
        noise3(17, p / scale) * amp + noise3(29, p / (scale * 0.37)) * amp * 0.4
    };
    match v.shape {
        VoidShape::Tube => {
            let (a, b) = (Vec3::from(v.a), Vec3::from(v.b));
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            // A third, finer layer keeps holes from looking drilled.
            (p - (a + ab * t)).length() - v.r
                + rough(v.r.max(10.0), v.r * 0.1)
                + noise3(41, p / (v.r * 0.2).max(3.0)) * v.r * 0.06
        }
        _ => {
            let d = p - Vec3::from(v.center);
            let q = Vec3::new(d.x * v.cos + d.z * v.sin, d.y, -d.x * v.sin + d.z * v.cos);
            let he = Vec3::from(v.half);
            match v.shape {
                VoidShape::Basin => {
                    // Both halves use the same scale, so they meet at the waterline
                    // without a step that the mesher would read as a thin ledge.
                    let k = he.y.min(he.x.min(he.z));
                    let flat = Vec2::new(q.x / he.x, q.z / he.z).length() - 1.0;
                    if q.y < 0.0 {
                        ((q / he).length() - 1.0) * k
                    } else {
                        {
                            let (up, wide) = basin_opening(he.x.min(he.z));
                            (flat * k - (q.y * 0.8).min(wide)).max(q.y - up)
                        }
                    }
                }
                VoidShape::Block => {
                    let e = q.abs() - he;
                    e.max(Vec3::ZERO).length() + e.max_element().min(0.0)
                }
                VoidShape::Carved => {
                    let k = (q.abs() - he).max(Vec3::ZERO).length();
                    let block = k + (q.abs() - he).max_element().min(0.0);
                    // A barrel vault across the short way over straight walls, with a rib
                    // every 45m that comes down the walls as a pillar.
                    let (along, across, wide) = if he.x >= he.z {
                        (q.x, q.z, he.z)
                    } else {
                        (q.z, q.x, he.x)
                    };
                    let spring = he.y * 0.1;
                    let rise = he.y - spring;
                    let rib =
                        1.0 - smoothstep(3.0, 7.0, ((along / 45.0).round() * 45.0 - along).abs());
                    let r = 1.0 - 0.07 * rib;
                    let e = Vec2::new(across / (wide * r), (q.y - spring).max(0.0) / (rise * r));
                    block.max((e.length() - 1.0) * rise.min(wide))
                }
                VoidShape::Dome => {
                    let small = he.min_element();
                    let ell = ((q / he).length() - 1.0) * small;
                    let floor = -(q.y + he.y * 0.55);
                    ell.max(floor) + rough(small * 0.8, small * 0.12)
                }
                VoidShape::Shaft => {
                    let flat = Vec2::new(q.x / he.x, q.z / he.z).length() - 1.0;
                    let side = flat * he.x.min(he.z);
                    side.max(q.y.abs() - he.y) + rough(he.x.min(he.z) * 0.8, he.x.min(he.z) * 0.12)
                }
                _ => {
                    // A rounded box: flattish floor and roof, round corners.
                    let r = he.min_element() * 0.5;
                    let k = (q.abs() - (he - Vec3::splat(r))).max(Vec3::ZERO).length() - r;
                    k + rough(he.y * 0.9, he.y * 0.14) + rough(18.0, 2.5) * 0.5
                }
            }
        }
    }
}

#[derive(Debug)]
struct Index {
    x0: f32,
    z0: f32,
    nx: usize,
    nz: usize,
    cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct Terrain {
    pub def: &'static WorldDef,
    index: Arc<Index>,
    /// Water height at every river point.
    levels: Arc<Vec<Vec<f32>>>,
    /// Graded height and bridge flag at every road point.
    roads: Arc<Vec<Vec<RoadPoint>>>,
    /// Bounds of every void, in the same order.
    bounds: Arc<Vec<Bounds>>,
    /// Level ground under big landmarks.
    pads: Arc<Vec<Pad>>,
}

/// Level ground under a big landmark: a rectangle in the landmark's frame.
#[derive(Clone, Debug)]
struct Pad {
    at: Vec2,
    axis: Vec2,
    half: Vec2,
    y: f32,
}

/// Landmarks narrower than this (half width, m) sit on the ground as it is.
const PAD_MIN: f32 = 20.0;
/// The farthest a pad's slope eases back to the hillside (m).
const PAD_REACH_MAX: f32 = 40.0;

pub fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smooth maximum that rounds the join over `k` meters.
fn smax(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.max(b) + h * h * k * 0.25
}

fn hash(seed: u32, x: i32, z: i32) -> f32 {
    let mut h = seed ^ (x as u32).wrapping_mul(0x8DA6_B343) ^ (z as u32).wrapping_mul(0xD816_3841);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth value noise in `[-1, 1]`.
pub fn noise(seed: u32, x: f32, z: f32) -> f32 {
    let (fx, fz) = (x.floor(), z.floor());
    let (tx, tz) = (x - fx, z - fz);
    let (sx, sz) = (tx * tx * (3.0 - 2.0 * tx), tz * tz * (3.0 - 2.0 * tz));
    let (ix, iz) = (fx as i32, fz as i32);
    let a = hash(seed, ix, iz);
    let b = hash(seed, ix + 1, iz);
    let c = hash(seed, ix, iz + 1);
    let d = hash(seed, ix + 1, iz + 1);
    (mix(mix(a, b, sx), mix(c, d, sx), sz)) * 2.0 - 1.0
}

fn fbm(seed: u32, p: Vec2, scale: f32) -> f32 {
    let q = p / scale;
    noise(seed, q.x, q.y) * 0.55
        + noise(seed ^ 0x1F, q.x * 2.13, q.y * 2.13) * 0.3
        + noise(seed ^ 0x2E, q.x * 4.37, q.y * 4.37) * 0.15
}

/// Distance from `p` to segment `a b`, and how far along it the nearest point is.
fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a + ab * t - p).length(), t)
}

fn crosses(p1: Vec2, p2: Vec2, q1: Vec2, q2: Vec2) -> bool {
    let o = |a: Vec2, b: Vec2, c: Vec2| (b - a).perp_dot(c - a);
    let (d1, d2) = (o(q1, q2, p1), o(q1, q2, p2));
    let (d3, d4) = (o(p1, p2, q1), o(p1, p2, q2));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

fn v2(p: [f32; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

/// Shore height, inland ramp width and offshore slope for each coast kind.
const COAST: [[f32; 3]; 3] = [[1.2, 160.0, 0.16], [2.5, 45.0, 0.35], [0.0, 10.0, 0.8]];

impl Default for Terrain {
    fn default() -> Self {
        Self::new(&WORLD)
    }
}

impl Terrain {
    pub fn new(def: &'static WorldDef) -> Self {
        let margin = LAND_RANGE + CELL;
        let x0 = def.bounds[0] - margin;
        let z0 = def.bounds[1] - margin;
        let nx = ((def.bounds[2] + margin - x0) / CELL).ceil() as usize;
        let nz = ((def.bounds[3] + margin - z0) / CELL).ceil() as usize;
        let mut index = Index {
            x0,
            z0,
            nx,
            nz,
            cells: vec![Cell::default(); nx * nz],
        };
        for (i, l) in def.lands.iter().enumerate() {
            index.add_poly(Kind::Land, i, l.pts, LAND_RANGE);
        }
        for (i, p) in def.plateaus.iter().enumerate() {
            index.add_poly(Kind::Plateau, i, p.pts, PLATEAU_RANGE);
        }
        for (i, l) in def.lakes.iter().enumerate() {
            index.add_poly(Kind::Lake, i, l.pts, LAKE_RANGE);
        }
        for (i, c) in def.covers.iter().enumerate() {
            index.add_poly(Kind::Cover, i, c.pts, 0.0);
        }
        for (i, r) in def.ridges.iter().enumerate() {
            index.add_line(Kind::Ridge, i, r.pts, r.width);
        }
        for (i, c) in def.carves.iter().enumerate() {
            index.add_line(Kind::Carve, i, c.pts, c.w * 0.5 + 12.0);
        }
        for (i, m) in def.mounts.iter().enumerate() {
            let r = m.r * m.stretch.max(1.0);
            index.each_cell(Vec2::new(m.x, m.z), Vec2::new(m.x, m.z), r, |c| {
                c.mounts.push(i as u16)
            });
        }
        for (i, s) in def.spires.iter().enumerate() {
            let at = Vec2::new(s.x, s.z);
            index.each_cell(at, at, s.r * 1.6, |c| c.spires.push(i as u16));
        }
        for (i, v) in def.voids.iter().enumerate() {
            let b = void_bounds(v);
            let (lo, hi) = (Vec2::new(b.min.x, b.min.z), Vec2::new(b.max.x, b.max.z));
            index.each_cell(lo, hi, 0.0, |c| c.voids.push(i as u16));
        }
        for (i, p) in def.pillars.iter().enumerate() {
            let at = Vec2::new(p.x, p.z);
            index.each_cell(at, at, p.r * 1.3, |c| c.pillars.push(i as u16));
        }
        for (i, r) in def.rivers.iter().enumerate() {
            // Deep cuts widen into gorges, so reach well past the banks.
            index.add_line(Kind::River, i, r.pts, r.w * 0.5 + 90.0);
        }
        for (i, l) in def.pools.iter().enumerate() {
            index.add_poly(Kind::Pool, i, l.pts, LAKE_RANGE);
        }
        for (i, t) in def.towns.iter().enumerate() {
            if !t.tiers.is_empty() {
                index.add_poly(Kind::Town, i, t.pts, 0.0);
            }
        }
        for (i, s) in def.shelves.iter().enumerate() {
            index.add_line2(Kind::Shelf, i, s.pts, s.w * 0.5 + 4.0);
        }
        for (i, r) in def.roads.iter().enumerate() {
            index.add_line(Kind::Road, i, r.pts, r.w * 0.5 + 40.0);
        }
        let mut t = Self {
            def,
            index: Arc::new(index),
            levels: Arc::new(Vec::new()),
            roads: Arc::new(Vec::new()),
            bounds: Arc::new(def.voids.iter().map(void_bounds).collect()),
            pads: Arc::new(Vec::new()),
        };
        t.roads = Arc::new(t.road_profiles());
        t.levels = Arc::new(t.measure_rivers());
        t.roads = Arc::new(t.mark_bridges());
        t.roads = Arc::new(t.mark_tunnels());
        t.roads = Arc::new(t.mark_viaducts());
        t.pads = Arc::new(t.landmark_pads());
        t
    }

    /// Level ground under every big landmark, at the mean height of its footprint, so
    /// parks, stadiums and malls stand flat on a hillside.
    fn landmark_pads(&self) -> Vec<Pad> {
        let mut out = Vec::new();
        for m in self.def.marks.iter().filter(|m| m.ground) {
            let Some(rest) = m.kind.strip_prefix("landmark:") else {
                continue;
            };
            let mut parts = rest.splitn(3, ':');
            let name = parts.next().unwrap_or_default();
            let rot: f32 = parts.next().and_then(|r| r.parse().ok()).unwrap_or(0.0);
            let text = parts.next().unwrap_or_default();
            let Some(&(_, kind)) = crate::landmark::KINDS.iter().find(|(n, _)| *n == name) else {
                continue;
            };
            if kind == crate::town::Kind::Sign {
                continue;
            }
            let half = crate::landmark::footprint(&crate::landmark::pieces(kind, 0, text));
            if half.min_element() < PAD_MIN {
                continue;
            }
            let r = rot.to_radians();
            let axis = Vec2::new(r.cos(), -r.sin());
            let front = Vec2::new(-axis.y, axis.x);
            let at = Vec2::new(m.pos[0], m.pos[2]);
            let mut sum = 0.0;
            for i in -2..=2 {
                for j in -2..=2 {
                    let q = at + axis * half.x * i as f32 * 0.45 + front * half.y * j as f32 * 0.45;
                    sum += self.height_at(q.x, q.y, Stage::Full);
                }
            }
            out.push(Pad {
                at,
                axis,
                half: half + Vec2::splat(4.0),
                y: sum / 25.0,
            });
        }
        out
    }

    /// Whether `(x, z)` is on the level ground under a big landmark.
    pub fn on_pad(&self, x: f32, z: f32) -> bool {
        let p = Vec2::new(x, z);
        self.pads.iter().any(|pad| {
            let d = p - pad.at;
            let q = Vec2::new(d.dot(pad.axis), pad.axis.perp_dot(d));
            q.x.abs() < pad.half.x && q.y.abs() < pad.half.y
        })
    }

    /// Pulls the ground level under big landmarks, easing back to the hillside round them.
    fn level_pads(&self, p: Vec2, mut h: f32) -> f32 {
        for pad in self.pads.iter() {
            let d = p - pad.at;
            if d.length_squared() > (pad.half.length() + PAD_REACH_MAX).powi(2) {
                continue;
            }
            let q = Vec2::new(d.dot(pad.axis), pad.axis.perp_dot(d));
            let out = (q.abs() - pad.half).max(Vec2::ZERO).length();
            let reach = (6.0 + (h - pad.y).abs() * 1.2).min(PAD_REACH_MAX);
            h = mix(h, pad.y, 1.0 - smoothstep(0.0, reach, out));
        }
        h
    }

    pub fn half_size(&self) -> f32 {
        (self.def.bounds[2] - self.def.bounds[0]) * 0.5
    }

    pub fn spawn(&self) -> Vec2 {
        v2(self.def.spawn)
    }

    fn sea_floor(&self, p: Vec2) -> f32 {
        self.def.sea_floor + fbm(7, p, 400.0) * 8.0
    }

    /// Height of the terrain surface at `(x, z)`.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        self.height_at(x, z, Stage::Full)
    }

    /// A road tunnel's bore for a road `w` wide: its radius, and how far above the road
    /// its center runs, so the whole width of the road fits with a little floor under it.
    /// The world compiler bores them the same way.
    pub fn bore(w: f32) -> (f32, f32) {
        let r = (w * 0.5 + 5.0).max(10.0);
        (r, r - 3.0)
    }

    /// The ground before roads and rivers shape it.
    pub fn natural_height(&self, x: f32, z: f32) -> f32 {
        self.height_at(x, z, Stage::Base)
    }

    fn height_at(&self, x: f32, z: f32, stage: Stage) -> f32 {
        let p = Vec2::new(x, z);
        let Some((cell, center)) = self.index.cell(p) else {
            return self.sea_floor(p);
        };
        let def = self.def;
        let adds = self.adds(cell, center, p);
        let mut best = self.sea_floor(p);
        let rolling = fbm(3, p, 380.0) * 9.0 + fbm(4, p, 90.0) * 2.5;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Land) {
            let land = &def.lands[e.idx as usize];
            let (sd, seg, t) = poly_sd(e, land.pts, center, p, LAND_RANGE);
            let n = land.edges.len();
            let k0 = COAST[land.edges[seg] as usize];
            let k1 = COAST[land.edges[(seg + 1) % n] as usize];
            let (shore, width, slope) = (
                mix(k0[0], k1[0], t),
                mix(k0[1], k1[1], t),
                mix(k0[2], k1[2], t),
            );
            let y = if sd >= 0.0 {
                let inland = self.carve(cell, center, p, smax(land.base + rolling, adds, 30.0));
                mix(shore, inland, smoothstep(0.0, width, sd))
            } else {
                shore + sd * slope
            };
            best = best.max(y);
        }
        for &i in &cell.spires {
            let s = &def.spires[i as usize];
            // Lumpy, not round: real stacks are split, slumped and undercut.
            let lumps = 1.0 + fbm(13, p, s.r * 0.9) * 0.22 + fbm(14, p, s.r * 0.3) * 0.1;
            let d = (p - Vec2::new(s.x, s.z)).length() / lumps.max(0.5);
            let f = 1.0 - smoothstep(s.r * 0.8, s.r * 1.15, d);
            if f > 0.0 {
                let top = if s.flat {
                    s.h
                } else {
                    s.h * (1.0 - 0.45 * d / s.r)
                } + fbm(11, p, 12.0) * s.r * 0.08
                    + fbm(12, p, s.r * 0.5) * s.r * 0.12;
                best = best.max(mix(best, top, f));
            }
        }
        self.finish(cell, center, p, best, stage)
    }

    /// Everything that raises land: ranges, mountains, hills and plateaus.
    fn adds(&self, cell: &Cell, center: Vec2, p: Vec2) -> f32 {
        let def = self.def;
        let mut a = -1.0e6;
        let crag = fbm(5, p, 110.0) + fbm(6, p, 37.0) * 0.5;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Ridge) {
            let r = &def.ridges[e.idx as usize];
            let mut best = -1.0e6f32;
            for &s in &e.segs {
                let (a3, b3) = (r.pts[s as usize], r.pts[s as usize + 1]);
                let (d, t) = seg_dist(p, Vec2::new(a3[0], a3[1]), Vec2::new(b3[0], b3[1]));
                let u = d / r.width;
                if u < 1.0 {
                    let h = mix(a3[2], b3[2], t);
                    let v = h * (1.0 - u) * (1.0 - u) * (1.0 + u);
                    let rough = if r.smooth { 0.0 } else { crag * 0.1 };
                    best = best.max(v + rough * v);
                }
            }
            if best > -1.0e5 {
                a = smax(a, best, 40.0);
            }
        }
        for &i in &cell.mounts {
            let m = &def.mounts[i as usize];
            let d = p - Vec2::new(m.x, m.z);
            let lx = d.x * m.cos + d.y * m.sin;
            let lz = -d.x * m.sin + d.y * m.cos;
            let (rx, rz) = (lx / (m.r * m.stretch), lz / m.r);
            let u = (rx * rx + rz * rz).sqrt();
            if u >= 1.0 {
                continue;
            }
            let cone = (1.0 - u) * (1.0 - 0.3 * u);
            let v = match m.shape {
                MountShape::Cone => m.h * cone,
                MountShape::Dome => {
                    let q = 1.0 - u * u;
                    m.h * q * q
                }
                MountShape::Crag => m.h * cone * (1.0 + crag * 0.18),
                MountShape::Crater => {
                    let ri = m.inner / m.r;
                    if u < ri {
                        mix(m.floor, m.rim, smoothstep(ri * 0.7, ri, u))
                    } else {
                        let v = (u - ri) / (1.0 - ri);
                        m.rim * (1.0 - v) * (1.0 - v) * (1.0 + v) * (1.0 + crag * 0.1)
                    }
                }
            };
            a = smax(a, v, 40.0);
        }
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Plateau) {
            let pl = &def.plateaus[e.idx as usize];
            let (sd, _, _) = poly_sd(e, pl.pts, center, p, PLATEAU_RANGE);
            let f = if pl.cliff {
                (1.0 + sd / 14.0).clamp(0.0, 1.0)
            } else {
                smoothstep(-120.0, 0.0, sd)
            };
            if f > 0.0 {
                a = smax(a, pl.top * f + crag * 4.0 * f, 20.0);
            }
        }
        a
    }

    /// Everything that cuts land: valleys, gorges, river beds, canals and lake basins.
    fn carve(&self, cell: &Cell, center: Vec2, p: Vec2, mut h: f32) -> f32 {
        let def = self.def;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Carve) {
            let c = &def.carves[e.idx as usize];
            let half = c.w * 0.5;
            // A gorge ends square at its last point, so it can open out through a cliff
            // face without digging into the ground at the foot.
            if c.kind == CarveKind::Gorge
                && let [.., a3, b3] = c.pts
            {
                let (a, b) = (Vec2::new(a3[0], a3[1]), Vec2::new(b3[0], b3[1]));
                if (p - b).dot(b - a) > 0.0 {
                    continue;
                }
            }
            let mut cut = 0.0f32;
            let mut nearest = f32::MAX;
            for &s in &e.segs {
                let (a3, b3) = (c.pts[s as usize], c.pts[s as usize + 1]);
                let (d, t) = seg_dist(p, Vec2::new(a3[0], a3[1]), Vec2::new(b3[0], b3[1]));
                nearest = nearest.min(d);
                let depth = mix(a3[2], b3[2], t);
                let f = match c.kind {
                    CarveKind::Valley => 1.0 - smoothstep(0.25, 1.0, d / half),
                    CarveKind::Gorge => 1.0 - smoothstep(0.55, 1.0, d / half),
                    CarveKind::River => 1.0 - smoothstep(0.6, 1.0, d / (half + 6.0)),
                    CarveKind::Canal => 0.0,
                };
                cut = cut.max(depth * f);
            }
            // A gorge never cuts down below the sea, so where it opens through a cliff its
            // cut stops at the foot instead of leaving a pit full of seawater.
            h = if c.kind == CarveKind::Gorge {
                (h - cut).max(h.min(def.sea_level + 2.0))
            } else {
                h - cut
            };
            if c.kind == CarveKind::Canal && nearest < half + 4.0 {
                let floor = c.level - 3.0;
                h = h.min(mix(floor, h, smoothstep(half, half + 4.0, nearest)));
            }
        }
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Lake) {
            let l = &def.lakes[e.idx as usize];
            let (sd, _, _) = poly_sd(e, l.pts, center, p, LAKE_RANGE);
            if sd > 0.0 {
                h = h.min(l.level - (1.5 + sd * 0.3).min(16.0));
            } else {
                // High ground slopes down to the shore instead of dropping sheer into the
                // water, which a height grid can only draw as a ragged step.
                if sd > -LAKE_BANK {
                    h = h.min(mix(l.level + 0.6, h, smoothstep(0.0, LAKE_BANK, -sd)));
                }
                // A lip so the water never spills over its shore.
                if sd > -10.0 {
                    h = h.max(l.level + 0.6);
                }
            }
        }
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Town) {
            let t = &def.towns[e.idx as usize];
            let (sd, _, _) = poly_sd(e, t.pts, center, p, 80.0);
            if sd > 0.0 {
                h = mix(h, terrace(h, t.tiers), smoothstep(0.0, 80.0, sd));
            }
        }
        h
    }

    /// Ledges, graded roads and river beds, which shape coasts and cliffs as well as land.
    fn finish(&self, cell: &Cell, center: Vec2, p: Vec2, mut h: f32, stage: Stage) -> f32 {
        let _ = center;
        let def = self.def;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Shelf) {
            let s = &def.shelves[e.idx as usize];
            let mut d = f32::MAX;
            for &i in &e.segs {
                let (a, b) = (v2(s.pts[i as usize]), v2(s.pts[i as usize + 1]));
                d = d.min(seg_dist(p, a, b).0);
            }
            let half = s.w * 0.5;
            let f = 1.0 - smoothstep(half - 2.0, half + 1.0, d);
            h = mix(h, s.y, f);
        }
        h = self.level_pads(p, h);
        if stage >= Stage::Roads {
            h = self.grade_roads(cell, p, h);
        }
        if stage >= Stage::Full {
            // River banks are cut back, but never into a road: the road keeps its grade
            // on an embankment, and only bridges cross the water.
            h = self.cut_rivers(cell, p, h);
            h = self.grade_roads(cell, p, h);
        }
        h
    }

    pub fn normal(&self, x: f32, z: f32) -> Vec3 {
        let e = 1.0;
        let dx = self.height(x + e, z) - self.height(x - e, z);
        let dz = self.height(x, z + e) - self.height(x, z - e);
        Vec3::new(-dx, 2.0 * e, -dz).normalize()
    }

    /// The land cover at `(x, z)`: towns win over forests, forests over meadows.
    pub fn cover(&self, x: f32, z: f32) -> Option<&'static Cover> {
        let p = Vec2::new(x, z);
        let (cell, center) = self.index.cell(p)?;
        let mut best: Option<&'static Cover> = None;
        let rank = |c: &Cover| match c.kind {
            CoverKind::Town => 4,
            CoverKind::Snow => 3,
            CoverKind::Forest => 2,
            CoverKind::Meadow => 1,
        };
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Cover) {
            let c = &self.def.covers[e.idx as usize];
            if inside(e, c.pts, center, p) && best.is_none_or(|b| rank(c) > rank(b)) {
                best = Some(c);
            }
        }
        best
    }

    /// Named places at and around a spot, for design notes: what it is inside or on,
    /// then the nearest marks.
    /// The climate of the island at `(x, z)`: 0 temperate (and open sea), 1 lush, 2 dry,
    /// 3 cold.
    pub fn climate(&self, x: f32, z: f32) -> u8 {
        let Some((cell, _)) = self.index.cell(Vec2::new(x, z)) else {
            return 0;
        };
        cell.entries
            .iter()
            .find(|e| e.kind == Kind::Land)
            .map(|e| self.def.lands[e.idx as usize].climate)
            .unwrap_or(0)
    }

    pub fn describe(&self, x: f32, z: f32) -> Vec<String> {
        let p = Vec2::new(x, z);
        let def = self.def;
        let mut out = vec![format!("ground {:.0}m", self.height(x, z))];
        if let Some((cell, center)) = self.index.cell(p) {
            for e in &cell.entries {
                let (id, what, pts) = match e.kind {
                    Kind::Land => (
                        def.lands[e.idx as usize].id,
                        "island",
                        def.lands[e.idx as usize].pts,
                    ),
                    Kind::Plateau => {
                        let pl = &def.plateaus[e.idx as usize];
                        (pl.id, "plateau", pl.pts)
                    }
                    Kind::Lake => (
                        def.lakes[e.idx as usize].id,
                        "lake",
                        def.lakes[e.idx as usize].pts,
                    ),
                    Kind::Cover => {
                        let c = &def.covers[e.idx as usize];
                        let what = match c.kind {
                            CoverKind::Forest => "forest",
                            CoverKind::Meadow => "meadow",
                            CoverKind::Town => "town",
                            CoverKind::Snow => "snowfield",
                        };
                        (c.id, what, c.pts)
                    }
                    Kind::Ridge
                    | Kind::Carve
                    | Kind::River
                    | Kind::Pool
                    | Kind::Town
                    | Kind::Shelf
                    | Kind::Road => continue,
                };
                if inside(e, pts, center, p) {
                    out.push(format!("in {id} ({what})"));
                }
            }
            for e in cell.entries.iter().filter(|e| e.kind == Kind::Ridge) {
                let r = &def.ridges[e.idx as usize];
                let on = e.segs.iter().any(|&s| {
                    let (a, b) = (r.pts[s as usize], r.pts[s as usize + 1]);
                    seg_dist(p, Vec2::new(a[0], a[1]), Vec2::new(b[0], b[1])).0 < r.width
                });
                if on {
                    out.push(format!("on {} (ridge)", r.id));
                }
            }
            for &i in &cell.mounts {
                let m = &def.mounts[i as usize];
                if (p - Vec2::new(m.x, m.z)).length() < m.r * m.stretch {
                    out.push(format!("on {} (mountain)", m.id));
                }
            }
        }
        let mut marks: Vec<(f32, &def::Mark)> = def
            .marks
            .iter()
            .map(|m| ((p - Vec2::new(m.pos[0], m.pos[2])).length(), m))
            .filter(|(d, _)| *d < 800.0)
            .collect();
        marks.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (d, m) in marks.iter().take(6) {
            out.push(format!("{d:.0}m from {} ({})", m.id, m.kind));
        }
        out
    }

    /// Voids whose bounds touch the box, for meshing a chunk.
    pub fn voids_in(&self, min: Vec3, max: Vec3) -> Vec<u16> {
        (0..self.def.voids.len() as u16)
            .filter(|&i| {
                let b = &self.bounds[i as usize];
                b.min.cmple(max).all() && b.max.cmpge(min).all()
            })
            .collect()
    }

    /// Whether any cave space is within `dist` of `p`.
    pub fn near_voids(&self, p: Vec3, dist: f32) -> bool {
        // Bounds are loose (the great hall's cover most of home), so check the real
        // distance to the cave space too.
        self.bounds.iter().zip(self.def.voids).any(|(b, v)| {
            let gap = (b.min - p).max(p - b.max).max(Vec3::ZERO);
            gap.length() < dist && void_sdf(v, p) < dist
        })
    }

    /// Distance into the cave spaces in `list`, negative inside one. Pillars count as rock.
    pub fn voids_sdf(&self, p: Vec3, list: &[u16]) -> f32 {
        let def = self.def;
        let mut d = f32::MAX;
        for &i in list {
            let b = &self.bounds[i as usize];
            if p.y < b.min.y || p.y > b.max.y {
                continue;
            }
            // A void whose loose bounds are farther than the nearest one so far cannot be
            // nearer, so skip its costly rough surface.
            let gap = (b.min - p).max(p - b.max).max(Vec3::ZERO).length();
            if gap > 0.0 && gap >= d {
                continue;
            }
            d = d.min(void_sdf(&def.voids[i as usize], p));
        }
        d = d.min(self.cave_channels(p));
        if d < 0.0
            && let Some((cell, _)) = self.index.cell(Vec2::new(p.x, p.z))
        {
            for &i in &cell.pillars {
                let pl = &def.pillars[i as usize];
                if p.y > pl.y0 && p.y < pl.y1 {
                    let wobble = noise3(41, p / 25.0) * pl.r * 0.2;
                    let rock =
                        (Vec2::new(p.x, p.z) - Vec2::new(pl.x, pl.z)).length() - pl.r - wobble;
                    d = d.max(-rock);
                }
            }
        }
        d
    }

    /// Signed distance to rock, positive in open air. Solid means under the surface and
    /// outside every cave. `ground` is the surface height at `p`.
    pub fn sdf_with(&self, p: Vec3, ground: f32, list: &[u16]) -> f32 {
        let above = p.y - ground;
        if list.is_empty() {
            return above;
        }
        above.max(-self.voids_sdf(p, list))
    }

    /// Cave spaces that might touch the column at `(x, z)`.
    fn cell_voids(&self, x: f32, z: f32) -> &[u16] {
        match self.index.cell(Vec2::new(x, z)) {
            Some((cell, _)) => &cell.voids,
            None => &[],
        }
    }

    pub fn sdf(&self, p: Vec3) -> f32 {
        let ground = self.height(p.x, p.z);
        self.sdf_with(p, ground, self.cell_voids(p.x, p.z))
    }

    /// How far the ground falls away within `r` of `(x, z)`. Big at cliff edges, where the
    /// drawn surface rounds off and anything set right on the edge would hang in the air.
    pub fn rim(&self, x: f32, z: f32, r: f32) -> f32 {
        const AROUND: [(f32, f32); 6] = [
            (1.0, 0.0),
            (0.5, 0.866),
            (-0.5, 0.866),
            (-1.0, 0.0),
            (-0.5, -0.866),
            (0.5, -0.866),
        ];
        let h = self.height(x, z);
        AROUND
            .iter()
            .map(|(dx, dz)| h - self.height(x + dx * r, z + dz * r))
            .fold(0.0, f32::max)
    }

    /// The highest solid surface at `(x, z)` not above `below`: the ground, or a cave
    /// floor when the column passes through caves.
    pub fn support(&self, x: f32, z: f32, below: f32) -> f32 {
        let ground = self.height(x, z);
        let list = self.cell_voids(x, z);
        if list.is_empty() {
            return ground;
        }
        let at = |y: f32| self.sdf_with(Vec3::new(x, y, z), ground, list);
        let mut y = below.min(ground + 0.5);
        let mut d = at(y);
        let floor = self.def.deep;
        while d > 0.0 && y > floor {
            let step = (d * 0.8).max(0.3);
            y -= step;
            d = at(y);
        }
        // Refine between the solid sample and the open one above it.
        let (mut lo, mut hi) = (y, y + (d.abs() * 0.8).max(0.3));
        for _ in 0..10 {
            let mid = (lo + hi) * 0.5;
            if at(mid) <= 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Pushes a sphere out of rock. Returns the outward normal and how far.
    pub fn push_out(&self, p: Vec3, r: f32) -> Option<(Vec3, f32)> {
        let ground = self.height(p.x, p.z);
        let list = self.cell_voids(p.x, p.z);
        if list.is_empty() {
            if p.y - ground > r * 4.0 {
                return None;
            }
            let n = self.normal(p.x, p.z);
            let dist = (p.y - ground) * n.y;
            return (dist < r).then_some((n, r - dist));
        }
        let d = self.sdf_with(p, ground, list);
        if d > r * 2.0 {
            return None;
        }
        let e = 0.5;
        let s = |o: Vec3| self.sdf(p + o);
        let n = Vec3::new(
            s(Vec3::X * e) - s(-Vec3::X * e),
            s(Vec3::Y * e) - s(-Vec3::Y * e),
            s(Vec3::Z * e) - s(-Vec3::Z * e),
        )
        .normalize_or(Vec3::Y);
        (d < r).then_some((n, r - d))
    }

    /// How much sky light reaches `p`: 1 in the open, fading toward 0 deep underground.
    pub fn daylight(&self, p: Vec3) -> f32 {
        let depth = self.height(p.x, p.z) - p.y;
        1.0 - smoothstep(4.0, 45.0, depth)
    }
}

/// Steps a height onto terraces: flat at each tier, with short steep risers between.
fn terrace(h: f32, tiers: &[f32]) -> f32 {
    let Some(k) = tiers.iter().rposition(|t| *t <= h) else {
        return h;
    };
    let flat = tiers[k];
    match tiers.get(k + 1) {
        Some(&next) => flat + smoothstep(next - 2.5, next, h) * (next - flat),
        None => flat + (h - flat) * 0.15,
    }
}

/// Whether `p` is inside a cell entry's outline: the cell center's side, flipped by every
/// outline segment between the center and `p`.
fn inside(e: &Entry, pts: &[[f32; 2]], center: Vec2, p: Vec2) -> bool {
    let n = pts.len();
    let mut inside = e.inside;
    for &s in &e.segs {
        let (a, b) = (v2(pts[s as usize]), v2(pts[(s as usize + 1) % n]));
        if crosses(center, p, a, b) {
            inside = !inside;
        }
    }
    inside
}

/// Signed distance to an outline, positive inside, capped at `cap`, plus the nearest
/// segment and how far along it.
fn poly_sd(e: &Entry, pts: &[[f32; 2]], center: Vec2, p: Vec2, cap: f32) -> (f32, usize, f32) {
    let n = pts.len();
    let (mut best, mut seg, mut along) = (cap, 0usize, 0.0);
    for &s in &e.segs {
        let (a, b) = (v2(pts[s as usize]), v2(pts[(s as usize + 1) % n]));
        let (d, t) = seg_dist(p, a, b);
        if d < best {
            (best, seg, along) = (d, s as usize, t);
        }
    }
    let d = if inside(e, pts, center, p) {
        best
    } else {
        -best
    };
    (d, seg, along)
}

impl Index {
    fn cell(&self, p: Vec2) -> Option<(&Cell, Vec2)> {
        let fx = ((p.x - self.x0) / CELL).floor();
        let fz = ((p.y - self.z0) / CELL).floor();
        if fx < 0.0 || fz < 0.0 {
            return None;
        }
        let (ix, iz) = (fx as usize, fz as usize);
        if ix >= self.nx || iz >= self.nz {
            return None;
        }
        Some((&self.cells[iz * self.nx + ix], self.center(ix, iz)))
    }

    fn center(&self, ix: usize, iz: usize) -> Vec2 {
        Vec2::new(
            self.x0 + (ix as f32 + 0.5) * CELL,
            self.z0 + (iz as f32 + 0.5) * CELL,
        )
    }

    fn range(&self, lo: f32, hi: f32, origin: f32, n: usize) -> std::ops::Range<usize> {
        let a = ((lo - origin) / CELL).floor().max(0.0) as usize;
        let b = (((hi - origin) / CELL).floor().max(-1.0) + 1.0) as usize;
        a.min(n)..b.min(n)
    }

    fn each_cell(&mut self, a: Vec2, b: Vec2, r: f32, mut f: impl FnMut(&mut Cell)) {
        let (lo, hi) = (a.min(b) - Vec2::splat(r), a.max(b) + Vec2::splat(r));
        for iz in self.range(lo.y, hi.y, self.z0, self.nz) {
            for ix in self.range(lo.x, hi.x, self.x0, self.nx) {
                f(&mut self.cells[iz * self.nx + ix]);
            }
        }
    }

    fn add_seg(&mut self, kind: Kind, idx: usize, s: usize, a: Vec2, b: Vec2, r: f32) {
        self.each_cell(a, b, r, |c| match c.entries.last_mut() {
            Some(e) if e.kind == kind && e.idx as usize == idx => e.segs.push(s as u32),
            _ => c.entries.push(Entry {
                kind,
                idx: idx as u16,
                inside: false,
                segs: vec![s as u32],
            }),
        });
    }

    fn add_line2(&mut self, kind: Kind, idx: usize, pts: &[[f32; 2]], r: f32) {
        for s in 0..pts.len().saturating_sub(1) {
            self.add_seg(kind, idx, s, v2(pts[s]), v2(pts[s + 1]), r);
        }
    }

    fn add_line(&mut self, kind: Kind, idx: usize, pts: &[[f32; 3]], r: f32) {
        for s in 0..pts.len().saturating_sub(1) {
            let (a, b) = (pts[s], pts[s + 1]);
            self.add_seg(
                kind,
                idx,
                s,
                Vec2::new(a[0], a[1]),
                Vec2::new(b[0], b[1]),
                r,
            );
        }
    }

    fn add_poly(&mut self, kind: Kind, idx: usize, pts: &[[f32; 2]], r: f32) {
        let n = pts.len();
        if n < 3 {
            return;
        }
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for s in 0..n {
            let (a, b) = (v2(pts[s]), v2(pts[(s + 1) % n]));
            lo = lo.min(a);
            hi = hi.max(a);
            self.add_seg(kind, idx, s, a, b, r);
        }
        // Mark the cells whose centers are inside, by counting crossings to the left.
        for iz in self.range(lo.y, hi.y, self.z0, self.nz) {
            for ix in self.range(lo.x, hi.x, self.x0, self.nx) {
                let c = self.center(ix, iz);
                let far = Vec2::new(lo.x - 10.0, c.y);
                let mut inside = false;
                for s in 0..n {
                    if crosses(far, c, v2(pts[s]), v2(pts[(s + 1) % n])) {
                        inside = !inside;
                    }
                }
                if !inside {
                    continue;
                }
                let cell = &mut self.cells[iz * self.nx + ix];
                match cell
                    .entries
                    .iter_mut()
                    .find(|e| e.kind == kind && e.idx as usize == idx)
                {
                    Some(e) => e.inside = true,
                    None => cell.entries.push(Entry {
                        kind,
                        idx: idx as u16,
                        inside: true,
                        segs: Vec::new(),
                    }),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_is_on_land() {
        let t = Terrain::default();
        let s = t.spawn();
        let h = t.height(s.x, s.y);
        assert!(h > 5.0, "spawn height {h}");
    }

    #[test]
    fn open_sea_is_deep() {
        let t = Terrain::default();
        let h = t.height(5000.0, -5000.0);
        assert!(h < -60.0, "sea height {h}");
    }

    #[test]
    fn summit_is_high() {
        let t = Terrain::default();
        let m = WORLD
            .marks
            .iter()
            .find(|m| m.id == "home/summit-beacon")
            .unwrap();
        let h = t.height(m.pos[0], m.pos[2]);
        assert!(h > 600.0, "summit height {h}");
    }

    #[test]
    fn rivers_run_downhill() {
        let t = Terrain::default();
        for (i, r) in WORLD.rivers.iter().enumerate() {
            let lv = t.river_levels(i);
            assert_eq!(lv.len(), r.pts.len(), "{}", r.id);
            for w in lv.windows(2) {
                assert!(w[1] <= w[0] + 1e-3, "{} climbs", r.id);
            }
            let big = lv.windows(2).map(|w| w[0] - w[1]).fold(0.0f32, f32::max);
            eprintln!(
                "{} {:.0} to {:.0}, biggest drop {big:.0}",
                r.id,
                lv[0],
                lv[lv.len() - 1]
            );
            let q = r.pts[r.pts.len() / 2];
            let ground = t.height(q[0], q[1]);
            if !r.under {
                let mid = lv[r.pts.len() / 2];
                assert!(ground < mid, "{} bed {ground} above water {mid}", r.id);
            }
        }
    }

    #[test]
    fn heights_are_deterministic() {
        let a = Terrain::default();
        let b = Terrain::default();
        for i in 0..200 {
            let (x, z) = (i as f32 * 37.3 - 3000.0, i as f32 * -19.1 + 1500.0);
            assert_eq!(a.height(x, z).to_bits(), b.height(x, z).to_bits());
        }
    }
}
