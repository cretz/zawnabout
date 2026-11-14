//! Rivers, roads and every water surface. River and road profiles are measured once, when
//! the terrain is built, from the ground they then cut into: rivers only ever run downhill,
//! and roads get a smooth grade with bridges wherever they cross water.

use glam::Vec2;

use super::{Cell, Kind, Stage, Terrain, mix, poly_sd, seg_dist, smoothstep};
use crate::terrain::def::NO_Y;

/// Water sits this far below the banks a river cuts.
const RIVER_DROP: f32 = 1.0;
/// Cave rivers run this deep over the cave floor.
const CAVE_RIVER_DEPTH: f32 = 3.0;
/// Banks stand this far above the water.
const BANK: f32 = 0.3;

/// One point of a road's profile.
#[derive(Clone, Copy, Debug)]
pub struct RoadPoint {
    /// Road surface height, or the deck height on a bridge.
    pub y: f32,
    pub bridge: bool,
    /// Runs through a tunnel: the ground overhead is left alone.
    pub tunnel: bool,
}

/// How far under the ground a road point must be to count as in a tunnel (m).
const UNDER: f32 = 4.0;
/// Steepest a road climbs, and a railway.
const ROAD_GRADE: f32 = 0.09;
const RAIL_GRADE: f32 = 0.03;
/// Widest a road's banks and cuttings spread beyond its shoulder (m).
const REACH: f32 = 40.0;
/// Dips up to twice this wide are bridged rather than followed (m).
const SPAN: f32 = 45.0;
/// A line through a cavern this far over its floor runs on a trestle (m).
const CAVE_TRESTLE: f32 = 3.0;
/// A road this far over the untouched ground runs on a viaduct (m).
const VIADUCT: f32 = 10.0;

/// The road nearest a spot, for coloring the ground under it.
#[derive(Clone, Copy, Debug)]
pub struct RoadSpot {
    /// Distance from the center line.
    pub d: f32,
    pub half: f32,
    pub runway: bool,
    pub bridge: bool,
}

/// Nearest point over a line entry's segments: distance, segment and how far along.
fn nearest(pts: &[[f32; 3]], segs: &[u32], p: Vec2) -> (f32, usize, f32) {
    let mut best = (f32::MAX, 0usize, 0.0f32);
    for &s in segs {
        let (a, b) = (pts[s as usize], pts[s as usize + 1]);
        let (d, t) = seg_dist(p, Vec2::new(a[0], a[1]), Vec2::new(b[0], b[1]));
        if d < best.0 {
            best = (d, s as usize, t);
        }
    }
    best
}

/// Nearest point over a whole path.
fn nearest_all(pts: &[[f32; 3]], p: Vec2) -> (f32, usize, f32) {
    let segs: Vec<u32> = (0..pts.len().saturating_sub(1) as u32).collect();
    nearest(pts, &segs, p)
}

impl Terrain {
    /// Surface lake level at a spot inside a lake.
    fn lake_at(&self, p: Vec2) -> Option<f32> {
        let (cell, center) = self.index.cell(p)?;
        cell.entries
            .iter()
            .filter(|e| e.kind == Kind::Lake)
            .map(|e| (e, &self.def.lakes[e.idx as usize]))
            .find(|(e, l)| poly_sd(e, l.pts, center, p, 1.0).0 > 0.0)
            .map(|(_, l)| l.level)
    }

    /// Still water over a spot: sea, lakes, cave lakes and canals, ignoring rivers.
    pub fn still_water(&self, x: f32, z: f32, ground: f32) -> Option<f32> {
        let p = Vec2::new(x, z);
        let mut best = (ground < self.def.sea_level).then_some(self.def.sea_level);
        let mut take = |level: f32| {
            if ground < level && best.is_none_or(|b| level > b) {
                best = Some(level);
            }
        };
        if let Some((cell, center)) = self.index.cell(p) {
            for e in &cell.entries {
                match e.kind {
                    Kind::Lake => {
                        let l = &self.def.lakes[e.idx as usize];
                        if poly_sd(e, l.pts, center, p, 10.0).0 > -10.0 {
                            take(l.level);
                        }
                    }
                    Kind::Pool => {
                        let l = &self.def.pools[e.idx as usize];
                        if ground > l.level - 80.0 && poly_sd(e, l.pts, center, p, 10.0).0 > -10.0 {
                            take(l.level);
                        }
                    }
                    Kind::Carve => {
                        let c = &self.def.carves[e.idx as usize];
                        if c.kind == super::CarveKind::Canal
                            && nearest(c.pts, &e.segs, p).0 < c.w * 0.5 + 2.0
                        {
                            take(c.level);
                        }
                    }
                    _ => {}
                }
            }
        }
        best
    }

    /// The water surface over `(x, z)` given the ground height there, if any.
    pub fn water(&self, x: f32, z: f32, ground: f32) -> Option<f32> {
        let p = Vec2::new(x, z);
        let mut best = self.still_water(x, z, ground);
        if let Some((cell, _)) = self.index.cell(p)
            && !self.levels.is_empty()
        {
            for e in cell.entries.iter().filter(|e| e.kind == Kind::River) {
                let r = &self.def.rivers[e.idx as usize];
                let (d, s, t) = nearest(r.pts, &e.segs, p);
                let lv = &self.levels[e.idx as usize];
                let level = mix(lv[s], lv[s + 1], t);
                let near = !r.under || ground > level - 40.0;
                if d < r.w * 0.5 + 2.0 && ground < level && near && best.is_none_or(|b| level > b) {
                    best = Some(level);
                }
            }
        }
        best
    }

    /// Whether `(x, z)` is on a canal's stone edge or bed.
    pub fn quay(&self, x: f32, z: f32) -> bool {
        let p = Vec2::new(x, z);
        let Some((cell, _)) = self.index.cell(p) else {
            return false;
        };
        cell.entries
            .iter()
            .filter(|e| e.kind == Kind::Carve)
            .any(|e| {
                let c = &self.def.carves[e.idx as usize];
                c.kind == super::CarveKind::Canal && nearest(c.pts, &e.segs, p).0 < c.w * 0.5 + 5.0
            })
    }

    /// Water in caves over a cave floor: cave lakes and cave rivers, never the surface's.
    pub fn cave_water(&self, x: f32, z: f32, ground: f32) -> Option<f32> {
        let p = Vec2::new(x, z);
        let (cell, center) = self.index.cell(p)?;
        let mut best: Option<f32> = None;
        let mut take = |level: f32| {
            if ground < level && ground > level - 40.0 && best.is_none_or(|b| level > b) {
                best = Some(level);
            }
        };
        for e in &cell.entries {
            match e.kind {
                Kind::Pool => {
                    let l = &self.def.pools[e.idx as usize];
                    if poly_sd(e, l.pts, center, p, 10.0).0 > -10.0 {
                        take(l.level);
                    }
                }
                Kind::River if !self.levels.is_empty() => {
                    let r = &self.def.rivers[e.idx as usize];
                    let (d, s, t) = nearest(r.pts, &e.segs, p);
                    let lv = &self.levels[e.idx as usize];
                    if r.under && d < r.w * 0.5 + 2.0 {
                        take(mix(lv[s], lv[s + 1], t));
                    }
                }
                _ => {}
            }
        }
        best
    }

    /// Water heights along river `i`, one per point.
    pub fn river_levels(&self, i: usize) -> &[f32] {
        &self.levels[i]
    }

    /// Profile of road `i`, one per point.
    pub fn road_points(&self, i: usize) -> &[RoadPoint] {
        &self.roads[i]
    }

    /// The road at `(x, z)`, if any is within its width plus `margin`.
    pub fn road_spot(&self, x: f32, z: f32, margin: f32) -> Option<RoadSpot> {
        let p = Vec2::new(x, z);
        let (cell, _) = self.index.cell(p)?;
        let mut best: Option<RoadSpot> = None;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Road) {
            let r = &self.def.roads[e.idx as usize];
            let (d, s, _) = nearest(r.pts, &e.segs, p);
            let half = r.w * 0.5;
            if d < half + margin && best.is_none_or(|b| d - half < b.d - b.half) {
                let rp = &self.roads[e.idx as usize];
                best = Some(RoadSpot {
                    d,
                    half,
                    runway: r.runway,
                    bridge: rp[s].bridge && rp[s + 1].bridge,
                });
            }
        }
        best
    }

    /// Road heights from the untouched ground: smoothed, never steeper than a real road
    /// or railway climbs, with bridges over water. Wider roads are laid first, and every
    /// road that joins one meets it at its height.
    pub(super) fn road_profiles(&self) -> Vec<Vec<RoadPoint>> {
        let def = self.def;
        let rank = |i: usize| {
            let r = &def.roads[i];
            (r.runway as u8, r.rail as u8, (r.w * 10.0) as u32)
        };
        let mut order: Vec<usize> = (0..def.roads.len()).collect();
        order.sort_by(|&a, &b| rank(b).cmp(&rank(a)).then(a.cmp(&b)));
        let mut out: Vec<Vec<RoadPoint>> = vec![Vec::new(); def.roads.len()];
        for (k, &ri) in order.iter().enumerate() {
            let r = &def.roads[ri];
            // Where it runs over a road already laid, it takes that road's height.
            let meets: Vec<Option<f32>> = r
                .pts
                .iter()
                .map(|q| {
                    let p = Vec2::new(q[0], q[1]);
                    order[..k].iter().find_map(|&oi| {
                        let o = &def.roads[oi];
                        let (d, s, t) = nearest_all(o.pts, p);
                        let op = &out[oi];
                        let y = mix(op[s].y, op[s + 1].y, t);
                        // Not where one passes over the other: through a tunnel under
                        // it, or on a bridge over it.
                        let level = (y - self.height_at(q[0], q[1], Stage::Base)).abs() < 20.0;
                        let plain = !op[s].bridge
                            && !op[s + 1].bridge
                            && !op[s].tunnel
                            && !op[s + 1].tunnel
                            && !o.subway;
                        // Its own width and a shoulder too, so the two lie flush where
                        // they overlap.
                        let reach = (o.w + r.w) * 0.5 + 6.0;
                        (d < reach && level && plain && !r.subway).then_some(y)
                    })
                })
                .collect();
            out[ri] = self.road_profile(r, &meets);
        }
        out
    }

    /// One road's profile, `meets` holding the heights it must take where it joins roads
    /// already laid.
    fn road_profile(&self, r: &super::def::Road, meets: &[Option<f32>]) -> Vec<RoadPoint> {
        let n = r.pts.len();
        let ground: Vec<f32> = r
            .pts
            .iter()
            .map(|q| self.height_at(q[0], q[1], Stage::Base))
            .collect();
        // A point set well below the ground runs in a tunnel: water overhead is no reason
        // for a bridge.
        let wet: Vec<bool> = r
            .pts
            .iter()
            .zip(&ground)
            .map(|(q, g)| {
                let under = q[2] > NO_Y + 1.0 && q[2] < g - UNDER;
                !under && self.still_water(q[0], q[1], *g - 0.5).is_some()
            })
            .collect();
        let held = |i: usize| (r.pts[i][2] > NO_Y + 1.0).then_some(r.pts[i][2]);
        let mut along = vec![0.0f32; n];
        for i in 1..n {
            let (a, b) = (r.pts[i - 1], r.pts[i]);
            along[i] = along[i - 1] + Vec2::new(b[0] - a[0], b[1] - a[1]).length();
        }
        let ys: Vec<f32> = if (0..n).all(|i| held(i).is_some()) {
            (0..n).map(|i| r.pts[i][2]).collect()
        } else if r.runway {
            // Dead flat, at the mean of the ground along it.
            let (a, b) = (r.pts[0], r.pts[n - 1]);
            let mean = (0..=8)
                .map(|k| {
                    let t = k as f32 / 8.0;
                    self.height_at(mix(a[0], b[0], t), mix(a[1], b[1], t), Stage::Base)
                })
                .sum::<f32>()
                / 9.0;
            vec![mean.max(2.0); n]
        } else {
            let mut fixed: Vec<Option<f32>> = (0..n).map(|i| held(i).or(meets[i])).collect();
            // The ground averaged over a stretch either side, skipping water.
            let window = |ys: &[f32], i: usize, reach: f32, skip_wet: bool| {
                let (mut sum, mut count) = (0.0, 0.0);
                let mut j = i;
                while j > 0 && along[i] - along[j - 1] <= reach {
                    j -= 1;
                }
                while j < n && along[j] - along[i] <= reach {
                    if !(skip_wet && wet[j]) {
                        sum += ys[j];
                        count += 1.0;
                    }
                    j += 1;
                }
                (count > 0.0).then(|| sum / count)
            };
            let smooth: Vec<f32> = (0..n)
                .map(|i| window(&ground, i, 25.0, true).unwrap_or(ground[i]).max(1.5))
                .collect();
            // Gullies and gorges narrower than a long bridge are spanned, not followed:
            // the highest ground within reach, then the lowest of those.
            let reach = |ys: &[f32], i: usize, pick: fn(f32, f32) -> f32| {
                let mut v = ys[i];
                for j in (0..i).rev().take_while(|&j| along[i] - along[j] <= SPAN) {
                    v = pick(v, ys[j]);
                }
                for j in (i + 1..n).take_while(|&j| along[j] - along[i] <= SPAN) {
                    v = pick(v, ys[j]);
                }
                v
            };
            let lay = |fixed: &[Option<f32>]| {
                let high: Vec<f32> = (0..n).map(|i| reach(&smooth, i, f32::max)).collect();
                let mut y: Vec<f32> = (0..n)
                    .map(|i| fixed[i].unwrap_or_else(|| reach(&high, i, f32::min)))
                    .collect();
                // No steeper than the grade a car or a train can take, both ways along.
                let grade = if r.rail { RAIL_GRADE } else { ROAD_GRADE };
                let limit = |y: &mut Vec<f32>| {
                    for _ in 0..2 {
                        for i in 1..n {
                            if fixed[i].is_none() {
                                let d = (along[i] - along[i - 1]) * grade;
                                y[i] = y[i].clamp(y[i - 1] - d, y[i - 1] + d);
                            }
                        }
                        for i in (0..n - 1).rev() {
                            if fixed[i].is_none() {
                                let d = (along[i + 1] - along[i]) * grade;
                                y[i] = y[i].clamp(y[i + 1] - d, y[i + 1] + d);
                            }
                        }
                    }
                };
                limit(&mut y);
                // Round off every change of grade into a gentle curve, then hold the grade
                // again where uneven spacing let the average steepen it.
                for _ in 0..2 {
                    let prev = y.clone();
                    for i in 0..n {
                        if fixed[i].is_none() {
                            y[i] = window(&prev, i, 15.0, false).unwrap_or(prev[i]);
                        }
                    }
                }
                limit(&mut y);
                y
            };
            let mut y = lay(&fixed);
            // A loop meets itself level: both ends take the height midway between them.
            let (first, last) = (r.pts[0], r.pts[n - 1]);
            if (first[0] - last[0]).abs() < 1.0 && (first[1] - last[1]).abs() < 1.0 {
                let mid = (y[0] + y[n - 1]) * 0.5;
                fixed[0] = fixed[0].or(Some(mid));
                fixed[n - 1] = fixed[n - 1].or(Some(mid));
                y = lay(&fixed);
            }
            y
        };
        let mut pts: Vec<RoadPoint> = ys
            .iter()
            .zip(&wet)
            .map(|(y, w)| RoadPoint {
                y: *y,
                bridge: *w && !r.runway,
                tunnel: false,
            })
            .collect();
        arch_bridges(&mut pts, r.pts, 30.0);
        pts
    }

    /// Where a road runs high over the untouched ground, it crosses on a viaduct rather
    /// than a towering bank of earth.
    pub(super) fn mark_viaducts(&self) -> Vec<Vec<RoadPoint>> {
        let mut out = (*self.roads).clone();
        for (r, pts) in self.def.roads.iter().zip(out.iter_mut()) {
            if r.runway || r.subway {
                continue;
            }
            let high: Vec<bool> = r
                .pts
                .iter()
                .zip(pts.iter())
                .map(|(q, p)| p.y - self.height_at(q[0], q[1], Stage::Base) > VIADUCT)
                .collect();
            let n = pts.len();
            for i in 0..n {
                // Each span starts one point early and ends one late, on its abutments.
                let near = high[i] || (i > 0 && high[i - 1]) || (i + 1 < n && high[i + 1]);
                if near && !pts[i].tunnel {
                    pts[i].bridge = true;
                }
            }
            // Through a cavern, a line high over the floor runs on a trestle.
            for (q, p) in r.pts.iter().zip(pts.iter_mut()) {
                if p.tunnel && p.y - self.support(q[0], q[1], p.y + 0.5) > CAVE_TRESTLE {
                    p.bridge = true;
                }
            }
        }
        out
    }

    /// Marks the road points that run inside a passage or cavern.
    pub(super) fn mark_tunnels(&self) -> Vec<Vec<RoadPoint>> {
        let mut out = (*self.roads).clone();
        for (r, pts) in self.def.roads.iter().zip(out.iter_mut()) {
            for (q, rp) in r.pts.iter().zip(pts.iter_mut()) {
                let p = Vec2::new(q[0], q[1]);
                // Only where the hill closes over it: short of that, a cutting leads in.
                let covered = self.height_at(p.x, p.y, Stage::Base) > rp.y + 6.0;
                rp.tunnel = covered
                    && self.index.cell(p).is_some_and(|(cell, _)| {
                        !cell.voids.is_empty()
                            && self.voids_sdf(glam::Vec3::new(p.x, rp.y + 2.0, p.y), &cell.voids)
                                < 0.0
                    });
            }
            // A point between two in the tunnel is in it too, round a loop's seam as well.
            let n = pts.len();
            let looped = r
                .pts
                .first()
                .zip(r.pts.last())
                .is_some_and(|(a, b)| (a[0] - b[0]).abs() < 1.0 && (a[1] - b[1]).abs() < 1.0);
            let was: Vec<bool> = pts.iter().map(|p| p.tunnel).collect();
            for i in 0..n {
                let (prev, next) = match (i, looped) {
                    (0, true) => (was[n - 2], was[1]),
                    (i, true) if i == n - 1 => (was[n - 2], was[1]),
                    (0, false) => (false, false),
                    (i, false) if i == n - 1 => (false, false),
                    _ => (was[i - 1], was[i + 1]),
                };
                if prev && next {
                    pts[i].tunnel = true;
                }
            }
        }
        out
    }

    /// Roads that cross rivers get bridges there too.
    pub(super) fn mark_bridges(&self) -> Vec<Vec<RoadPoint>> {
        let def = self.def;
        let mut out = (*self.roads).clone();
        for (ri, r) in def.roads.iter().enumerate() {
            if r.runway {
                continue;
            }
            let mut changed = false;
            for (i, q) in r.pts.iter().enumerate() {
                let p = Vec2::new(q[0], q[1]);
                for river in def.rivers {
                    if river.under {
                        continue;
                    }
                    let (d, _, _) = nearest_all(river.pts, p);
                    // Over the water, and over the banks wherever the river cuts them
                    // below the road, so the road never dives down beside its bridge.
                    let half = river.w * 0.5;
                    let over = d < half + 4.0
                        || (d < half + 60.0
                            && self.height_at(q[0], q[1], Stage::Full) < out[ri][i].y - 1.5);
                    let under = out[ri][i].y < self.height_at(q[0], q[1], Stage::Base) - UNDER;
                    if over && !under && !out[ri][i].bridge {
                        out[ri][i].bridge = true;
                        changed = true;
                    }
                }
            }
            if changed {
                // The profile already spans the dip, so the deck only gets a gentle hump.
                arch_bridges(&mut out[ri], r.pts, 4.0);
            }
        }
        out
    }

    /// Water heights along every river, never rising downstream.
    pub(super) fn measure_rivers(&self) -> Vec<Vec<f32>> {
        let def = self.def;
        let mut out: Vec<Vec<f32>> = Vec::with_capacity(def.rivers.len());
        for r in def.rivers {
            let parent = (r.parent >= 0).then_some(r.parent as usize);
            let at_parent = |p: Vec2, out: &Vec<Vec<f32>>| -> Option<f32> {
                let pi = parent?;
                let (_, s, t) = nearest_all(def.rivers[pi].pts, p);
                let lv = &out[pi];
                Some(mix(lv[s], lv[(s + 1).min(lv.len() - 1)], t))
            };
            let first = Vec2::new(r.pts[0][0], r.pts[0][1]);
            let mut run = if r.from_parent {
                at_parent(first, &out).unwrap_or(f32::MAX)
            } else {
                f32::MAX
            };
            let mut levels = Vec::with_capacity(r.pts.len());
            for q in r.pts {
                let p = Vec2::new(q[0], q[1]);
                let g = if r.under {
                    match self.pool_at(p) {
                        Some(level) => level,
                        None => self.support(q[0], q[1], q[2] + 40.0) + CAVE_RIVER_DEPTH,
                    }
                } else {
                    let g = self.height_at(q[0], q[1], Stage::Roads);
                    let g = match self.lake_at(p) {
                        Some(level) => g.max(level + RIVER_DROP),
                        None => g,
                    };
                    g - RIVER_DROP
                };
                run = run.min(g);
                levels.push(run);
            }
            let last = r.pts[r.pts.len() - 1];
            if !r.from_parent
                && let Some(floor) = at_parent(Vec2::new(last[0], last[1]), &out)
            {
                for v in &mut levels {
                    *v = v.max(floor);
                }
            }
            if r.under {
                plunge(&mut levels, r.pts);
            } else {
                for v in &mut levels {
                    *v = v.max(def.sea_level + 0.2);
                }
            }
            out.push(levels);
        }
        out
    }

    /// Distance into the slots cave rivers cut along their floors, negative inside.
    pub(super) fn cave_channels(&self, p: glam::Vec3) -> f32 {
        let mut d = f32::MAX;
        if self.levels.is_empty() {
            return d;
        }
        let Some((cell, _)) = self.index.cell(Vec2::new(p.x, p.z)) else {
            return d;
        };
        for e in cell.entries.iter().filter(|e| e.kind == Kind::River) {
            let r = &self.def.rivers[e.idx as usize];
            if !r.under {
                continue;
            }
            let (flat, s, t) = nearest(r.pts, &e.segs, Vec2::new(p.x, p.z));
            let lv = &self.levels[e.idx as usize];
            let level = mix(lv[s], lv[s + 1], t);
            let slot = (flat - r.w * 0.5)
                .max(level - CAVE_RIVER_DEPTH * 2.0 - p.y)
                .max(p.y - level - 25.0);
            d = d.min(slot);
        }
        d
    }

    /// Cave lake level at a spot inside one.
    fn pool_at(&self, p: Vec2) -> Option<f32> {
        let (cell, center) = self.index.cell(p)?;
        cell.entries
            .iter()
            .filter(|e| e.kind == Kind::Pool)
            .map(|e| (e, &self.def.pools[e.idx as usize]))
            .find(|(e, l)| poly_sd(e, l.pts, center, p, 1.0).0 > 0.0)
            .map(|(_, l)| l.level)
    }

    /// Pulls the ground to each road's grade, except under bridges. Where roads meet, the
    /// one a spot is most inside is laid last, so its surface is the one the ground keeps.
    pub(super) fn grade_roads(&self, cell: &Cell, p: Vec2, mut h: f32) -> f32 {
        if self.roads.is_empty() {
            return h;
        }
        let mut near: [(f32, f32, f32); 6] = [(0.0, 0.0, 0.0); 6];
        let mut count = 0;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::Road) {
            let r = &self.def.roads[e.idx as usize];
            let rp = &self.roads[e.idx as usize];
            let (d, s, t) = nearest(r.pts, &e.segs, p);
            let half = r.w * 0.5;
            // Over tunnels the ground stays as it is, and under bridges too, except that
            // a bank never rises through the deck.
            // At a portal the cutting leads in, but it never digs a deep shaft down to a
            // road that runs on in the hill.
            if (rp[s].tunnel && rp[s + 1].tunnel)
                || ((rp[s].tunnel || rp[s + 1].tunnel) && h - mix(rp[s].y, rp[s + 1].y, t) > 12.0)
            {
                continue;
            }
            if rp[s].bridge || rp[s + 1].bridge {
                let under = mix(rp[s].y, rp[s + 1].y, t) - 2.0;
                if d < half + 3.0 && h > under {
                    h = mix(h, under, 1.0 - smoothstep(half + 1.0, half + 3.0, d));
                }
                continue;
            }
            if d > half + 4.0 + REACH || count == near.len() {
                continue;
            }
            near[count] = (d - half, mix(rp[s].y, rp[s + 1].y, t), half);
            count += 1;
        }
        let near = &mut near[..count];
        near.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(out, y, _) in near.iter() {
            // A flat shoulder wider than a terrain cell, so the ground never pokes through.
            // Banks and cuttings slope back like real earthworks, but never carve away a
            // whole hillside.
            let reach = (4.0 + (h - y).abs() * 0.9).min(REACH);
            let f = 1.0 - smoothstep(4.0, 4.0 + reach, out);
            h = mix(h, y, f);
        }
        h
    }

    /// Cuts river channels, with walls that widen into gorges where the cut is deep, and
    /// raises low banks so water never spills.
    pub(super) fn cut_rivers(&self, cell: &Cell, p: Vec2, mut h: f32) -> f32 {
        if self.levels.is_empty() {
            return h;
        }
        let mut in_channel = false;
        let mut lips: [(f32, f32, f32); 4] = [(0.0, 0.0, 0.0); 4];
        let mut n_lips = 0;
        for e in cell.entries.iter().filter(|e| e.kind == Kind::River) {
            let r = &self.def.rivers[e.idx as usize];
            if r.under {
                continue;
            }
            let (d, s, t) = nearest(r.pts, &e.segs, p);
            let lv = &self.levels[e.idx as usize];
            let level = mix(lv[s], lv[s + 1], t);
            let half = r.w * 0.5;
            let depth = 1.2 + r.w * 0.05;
            let target = if d < half {
                in_channel = true;
                let u = d / half;
                mix(level - depth, level + BANK, u * u)
            } else {
                // A soft grassy bank first, then steeper walls where the cut runs deep.
                let out = d - half;
                level + BANK + out * 0.45 + (out - 8.0).max(0.0) * 0.8
            };
            h = h.min(target);
            if d < half + 6.0 && n_lips < lips.len() {
                lips[n_lips] = (d, half, level);
                n_lips += 1;
            }
        }
        if !in_channel {
            for &(d, half, level) in &lips[..n_lips] {
                let bank = level + BANK;
                if h < bank {
                    h = mix(bank, h, smoothstep(half, half + 6.0, d));
                }
            }
        }
        h
    }
}

/// Cave falls drop straight from their lip: wherever the water falls steeply on the
/// whole, it falls in one sheet and lands at the foot, ignoring ledges on the way.
fn plunge(levels: &mut [f32], pts: &[[f32; 3]]) {
    let n = levels.len();
    let flat =
        |a: usize, b: usize| Vec2::new(pts[b][0] - pts[a][0], pts[b][1] - pts[a][1]).length();
    let mut i = 0;
    while i + 1 < n {
        let starts = (levels[i] - levels[i + 1]) / flat(i, i + 1).max(0.1) > 0.6;
        let foot = (i + 2..n)
            .rev()
            .find(|&j| starts && (levels[i] - levels[j]) / flat(i, j).max(0.1) > 1.5);
        match foot {
            Some(j) => {
                let foot = levels[j];
                levels[i + 1..j].fill(foot);
                i = j;
            }
            None => i += 1,
        }
    }
}

/// Spans of bridge points get a gentle arch between their ends, adding at most half a
/// road's steepest grade at the ends, and at most `rise` meters at the middle. One extra point at
/// each end becomes the abutment.
fn arch_bridges(pts: &mut [RoadPoint], line: &[[f32; 3]], rise: f32) {
    let n = pts.len();
    let flat =
        |a: usize, b: usize| Vec2::new(line[b][0] - line[a][0], line[b][1] - line[a][1]).length();
    let mut i = 0;
    while i < n {
        if !pts[i].bridge {
            i += 1;
            continue;
        }
        let start = i.saturating_sub(1);
        let mut end = i;
        while end + 1 < n && pts[end + 1].bridge {
            end += 1;
        }
        let end = (end + 1).min(n - 1);
        let mut along = vec![0.0f32; end - start + 1];
        for k in 1..along.len() {
            along[k] = along[k - 1] + flat(start + k - 1, start + k);
        }
        let len = along[along.len() - 1].max(1.0);
        let top = rise.min(len * ROAD_GRADE * 0.125);
        let base: Vec<f32> = pts[start..=end].iter().map(|p| p.y).collect();
        for (k, p) in pts[start..=end].iter_mut().enumerate() {
            let u = along[k] / len;
            p.y = base[k] + top * 4.0 * u * (1.0 - u);
            p.bridge = true;
        }
        i = end + 1;
    }
}
