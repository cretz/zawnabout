//! Suspension bridges: wherever a `bridge` site sits by a long road bridge, the span gets
//! two tall towers, a main cable each side sagging between them down to the deck and back
//! up to anchors at the banks, and hangers holding the deck up. A railway bridge running
//! right beside the road is carried too, between the same cables. The towers are solid
//! buildings, the cables only for looks.

use glam::{Vec2, Vec3};

use crate::terrain::Terrain;

/// Spans shorter than this keep their plain deck (m).
const MIN_SPAN: f32 = 400.0;
/// Where the towers stand along the span, as fractions of its length.
const TOWERS: [f32; 2] = [0.2, 0.8];
/// Tower height over the highest point of the deck, per meter of main span.
const RISE: f32 = 0.17;
/// How far apart hangers hang (m).
const HANGER_GAP: f32 = 8.0;
/// How far from the road a railway bridge may run and still be carried with it (m).
const ALONGSIDE: f32 = 25.0;

/// One tower: where it stands, the way across the deck, its feet and top, and half the
/// distance between its two legs.
#[derive(Clone, Copy, Debug)]
pub struct Tower {
    pub at: Vec2,
    pub across: Vec2,
    pub base: f32,
    pub top: f32,
    pub spread: f32,
}

#[derive(Clone, Debug)]
pub struct Suspension {
    /// The roads it carries, and where each deck hangs, in meters along that road.
    pub spans: Vec<(usize, [f32; 2])>,
    pub towers: [Tower; 2],
    /// The two main cables, bank to bank over the towers.
    pub cables: [Vec<Vec3>; 2],
    /// Each hanger's top on a cable and its foot on the deck.
    pub hangers: Vec<(Vec3, Vec3)>,
}

/// A stretch of road points that bridge water: which road, first and last point.
#[derive(Clone, Copy)]
struct Run {
    road: usize,
    s: usize,
    e: usize,
}

/// Every run of bridge points on roads (`rail` false) or railways (`rail` true).
fn runs(t: &Terrain, rail: bool) -> Vec<Run> {
    let mut out = Vec::new();
    for (road, r) in t.def.roads.iter().enumerate() {
        if r.rail != rail || r.runway {
            continue;
        }
        let prof = t.road_points(road);
        let mut k = 0;
        while k < prof.len() {
            if !prof[k].bridge {
                k += 1;
                continue;
            }
            let s = k;
            while k < prof.len() && prof[k].bridge {
                k += 1;
            }
            out.push(Run { road, s, e: k - 1 });
        }
    }
    out
}

fn point(t: &Terrain, road: usize, k: usize) -> Vec2 {
    let p = t.def.roads[road].pts[k];
    Vec2::new(p[0], p[1])
}

/// How far a run's nearest point is from `p`, and that point's index.
fn nearest(t: &Terrain, run: Run, p: Vec2) -> (f32, usize) {
    (run.s..=run.e)
        .map(|k| (point(t, run.road, k).distance(p), k))
        .fold((f32::MAX, run.s), |a, b| if b.0 < a.0 { b } else { a })
}

/// Meters along road `road` to its point `k`.
fn along_road(t: &Terrain, road: usize, k: usize) -> f32 {
    (0..k)
        .map(|j| point(t, road, j).distance(point(t, road, j + 1)))
        .sum()
}

/// A suspension bridge at every `bridge` site beside a long enough road bridge.
pub fn suspensions(t: &Terrain) -> Vec<Suspension> {
    let roads = runs(t, false);
    let rails = runs(t, true);
    let mut out = Vec::new();
    for m in t.def.marks.iter().filter(|m| m.kind == "site:bridge") {
        let site = Vec2::new(m.pos[0], m.pos[2]);
        let best = roads
            .iter()
            .map(|r| (nearest(t, *r, site).0, *r))
            .filter(|(d, _)| *d < 400.0)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some(b) = best.and_then(|(_, run)| suspension(t, run, &rails)) {
            out.push(b);
        }
    }
    out
}

/// The suspension bridge over `run`, with any railway run alongside it, if the span is
/// long enough.
fn suspension(t: &Terrain, run: Run, rails: &[Run]) -> Option<Suspension> {
    let road = &t.def.roads[run.road];
    let prof = t.road_points(run.road);
    let xz = |k: usize| point(t, run.road, k);
    let mut along = vec![0.0f32];
    for k in run.s..run.e {
        along.push(along[along.len() - 1] + xz(k).distance(xz(k + 1)));
    }
    let len = *along.last()?;
    if len < MIN_SPAN {
        return None;
    }
    // The centerline, deck height and way across at `d` meters along the span.
    let at = |d: f32| {
        let d = d.clamp(0.0, len);
        let i = along.partition_point(|a| *a <= d).clamp(1, along.len() - 1);
        let u = (d - along[i - 1]) / (along[i] - along[i - 1]).max(1e-3);
        let (a, b) = (run.s + i - 1, run.s + i);
        let p = xz(a).lerp(xz(b), u);
        let y = prof[a].y + (prof[b].y - prof[a].y) * u;
        let dir = (xz(b) - xz(a)).normalize_or(Vec2::X);
        (p, y, Vec2::new(-dir.y, dir.x))
    };
    // A railway running right beside the middle of the span shares the bridge.
    let (mid, _, mid_across) = at(len * 0.5);
    let rail = rails
        .iter()
        .find(|r| nearest(t, **r, mid).0 < ALONGSIDE)
        .copied();
    let rail_deck = |p: Vec2| {
        rail.map(|r| {
            let (_, k) = nearest(t, r, p);
            t.road_points(r.road)[k].y
        })
    };
    // The deck's edges either side of the road's centerline, rail included.
    let half = road.w * 0.5;
    let (mut lo, mut hi) = (-half, half);
    let mut rail_side = 0.0;
    if let Some(r) = rail {
        let (_, k) = nearest(t, r, mid);
        let o = (point(t, r.road, k) - mid).dot(mid_across);
        let rail_half = t.def.roads[r.road].w * 0.5 + 1.0;
        lo = lo.min(o - rail_half);
        hi = hi.max(o + rail_half);
        rail_side = o.signum();
    }
    let (lo, hi) = (lo - 1.5, hi + 1.5);
    let deck_top = prof[run.s..=run.e]
        .iter()
        .map(|p| p.y)
        .fold(f32::MIN, f32::max);
    let (t1, t2) = (len * TOWERS[0], len * TOWERS[1]);
    let top = deck_top + (t2 - t1) * RISE;
    let tower = |d: f32| {
        let (p, _, across) = at(d);
        Tower {
            at: p + across * (lo + hi) * 0.5,
            across,
            base: t.def.sea_level - 6.0,
            top,
            spread: (hi - lo) * 0.5 + 1.0,
        }
    };
    // The cable's height: rising from its anchor to the first tower, sagging to just over
    // the deck midway, and back up and down to the far anchor.
    let low = at(len * 0.5).1 + 3.0;
    let cable_y = |d: f32| {
        if d <= t1 {
            let u = d / t1;
            let a = at(0.0).1 + 1.0;
            a + (top - a) * u - t1 * 0.04 * 4.0 * u * (1.0 - u)
        } else if d >= t2 {
            let u = (len - d) / (len - t2);
            let a = at(len).1 + 1.0;
            a + (top - a) * u - (len - t2) * 0.04 * 4.0 * u * (1.0 - u)
        } else {
            let u = (d - t1) / (t2 - t1) * 2.0 - 1.0;
            low + (top - low) * u * u
        }
    };
    let mut stops: Vec<f32> = (0..=(len / 6.0) as u32).map(|k| k as f32 * 6.0).collect();
    stops.extend([t1, t2, len]);
    stops.sort_by(f32::total_cmp);
    stops.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let cables = [lo, hi].map(|off| {
        stops
            .iter()
            .map(|&d| {
                let (p, _, across) = at(d);
                let q = p + across * off;
                Vec3::new(q.x, cable_y(d), q.y)
            })
            .collect::<Vec<_>>()
    });
    let mut hangers = Vec::new();
    let n = (len / HANGER_GAP) as u32;
    for k in 1..n {
        let d = k as f32 * HANGER_GAP;
        if (d - t1).abs() < 4.0 || (d - t2).abs() < 4.0 {
            continue;
        }
        let (p, y, across) = at(d);
        let c = cable_y(d);
        for off in [lo, hi] {
            let q = p + across * off;
            // The deck under this cable: the railway's on its side, else the road's.
            let foot = if off.signum() == rail_side {
                rail_deck(q).unwrap_or(y)
            } else {
                y
            };
            if c - foot > 2.0 {
                hangers.push((Vec3::new(q.x, c, q.y), Vec3::new(q.x, foot + 0.2, q.y)));
            }
        }
    }
    let mut spans = vec![(
        run.road,
        [
            along_road(t, run.road, run.s),
            along_road(t, run.road, run.e),
        ],
    )];
    if let Some(r) = rail {
        spans.push((
            r.road,
            [along_road(t, r.road, r.s), along_road(t, r.road, r.e)],
        ));
    }
    Some(Suspension {
        spans,
        towers: [tower(t1), tower(t2)],
        cables,
        hangers,
    })
}
