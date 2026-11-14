//! Car trips over the road network. Roads that end on another road join it there, so the
//! roads make a graph of junctions and dead ends. Each car drives from one dead end to a
//! far one by the shortest way, turning onto other roads at the junctions, waits there a
//! moment, turns round and drives home again.

use glam::{Vec2, Vec3};

use crate::terrain::Terrain;

/// One car's round trip, already in its lane: out to the far end, round, and back.
pub struct Trip {
    pub pts: Vec<Vec3>,
    /// Where it halts, as distances along `pts`: at the far end, then at home.
    pub stops: [f32; 2],
    /// The widest road it takes, for picking buses and trucks for the big roads.
    pub widest: f32,
}

/// A place on a road: how far along its points (point index plus fraction).
#[derive(Clone, Copy)]
struct Spot {
    at: f32,
}

struct Edge {
    a: usize,
    b: usize,
    road: usize,
    from: f32,
    to: f32,
    len: f32,
}

/// The nearest point of a polyline to `p`: distance, and where along it.
fn nearest(pts: &[Vec3], p: Vec2) -> (f32, f32) {
    let mut best = (f32::MAX, 0.0);
    for i in 0..pts.len().saturating_sub(1) {
        let (a, b) = (
            Vec2::new(pts[i].x, pts[i].z),
            Vec2::new(pts[i + 1].x, pts[i + 1].z),
        );
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
        let d = (a + ab * t).distance(p);
        if d < best.0 {
            best = (d, i as f32 + t);
        }
    }
    best
}

fn point(pts: &[Vec3], at: f32) -> Vec3 {
    let i = (at.floor() as usize).min(pts.len() - 2);
    pts[i].lerp(pts[i + 1], at - i as f32)
}

/// The points of a road from `from` to `to` (either way round), ends included.
fn stretch(pts: &[Vec3], from: f32, to: f32) -> Vec<Vec3> {
    let mut out = vec![point(pts, from)];
    if from < to {
        let first = from.floor() as usize + 1;
        out.extend((first..).take_while(|&i| (i as f32) < to).map(|i| pts[i]));
    } else {
        let first = from.ceil() as usize;
        let first = if first as f32 == from {
            first.saturating_sub(1)
        } else {
            first - 1
        };
        let mut i = first as i64;
        while i >= 0 && (i as f32) > to {
            out.push(pts[i as usize]);
            i -= 1;
        }
    }
    out.push(point(pts, to));
    out
}

fn find(parent: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while parent[r] != r {
        r = parent[r];
    }
    let mut i = i;
    while parent[i] != r {
        let next = parent[i];
        parent[i] = r;
        i = next;
    }
    r
}

/// Shifts a line to the right by `lane`, which can change along it.
fn keep_right(pts: &[Vec3], lane: &[f32]) -> Vec<Vec3> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let d = pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)];
            let f = Vec2::new(d.x, d.z).normalize_or(Vec2::Y);
            pts[i] + Vec3::new(-f.y, 0.0, f.x) * lane[i]
        })
        .collect()
}

/// How many cars can call at one stop, each at its own spot along the curb, and how far
/// apart those are (m).
const SLOTS: u8 = 3;
const SLOT_GAP: f32 = 14.0;

/// A turn at the end of a lane: from `lane` right of the middle at `end`, heading along
/// `dir`, it pulls in to the curb, then swings round a half circle to the other side
/// heading back, wide enough to look like a car turning in a square or a driveway. Also
/// gives where it pulls in, as a distance along the turn, which is where it waits, out of
/// the way of anything driving by.
fn turn(end: Vec3, dir: Vec2, lane: f32) -> (Vec<Vec3>, f32) {
    let side = Vec3::new(-dir.y, 0.0, dir.x);
    let fwd = Vec3::new(dir.x, 0.0, dir.y);
    let r = lane.max(6.5);
    let mut out = vec![
        end + fwd * 2.0 + side * (lane + (r - lane) * 0.3),
        end + fwd * 4.0 + side * (lane + (r - lane) * 0.75),
    ];
    let steps = 12;
    for k in 0..=steps {
        let a = k as f32 / steps as f32 * std::f32::consts::PI;
        out.push(end + fwd * (6.0 + r * a.sin()) + side * (r * a.cos()));
    }
    let wait =
        (end + side * lane).distance(out[0]) + out[0].distance(out[1]) + out[1].distance(out[2]);
    (out, wait)
}

/// Lays a turn's points on the road under them, so a turn on a hill climbs or dips with
/// it instead of running flat into the slope. Only roads level with the turn count, so
/// a bridge over or a tunnel under is left alone.
fn lay_on_road(round: &mut [Vec3], pts: &[Vec<Vec3>], w: &[f32], end: Vec3) {
    for p in round {
        let flat = Vec2::new(p.x, p.z);
        let mut best: Option<(f32, f32)> = None;
        for (o, line) in pts.iter().enumerate() {
            if line.len() < 2 {
                continue;
            }
            let (d, at) = nearest(line, flat);
            let y = point(line, at).y;
            if d < w[o] * 0.5 + 8.0 && (y - end.y).abs() < 4.0 && best.is_none_or(|b| d < b.0) {
                best = Some((d, y));
            }
        }
        if let Some((_, y)) = best {
            p.y = y;
        }
    }
}

/// Cuts `len` meters off the end of a line, and its lanes with it.
fn trim_end(line: &mut Vec<Vec3>, lane: &mut Vec<f32>, mut len: f32) {
    while len > 0.0 && line.len() > 2 {
        let n = line.len();
        let d = line[n - 1].distance(line[n - 2]);
        if d <= len {
            line.pop();
            lane.pop();
            len -= d;
        } else {
            line[n - 1] = line[n - 1].lerp(line[n - 2], len / d);
            len = 0.0;
        }
    }
}

/// Every car's trip, about one for every 220 m of road, calling at the `places` given
/// (where buildings stand) and at the dead ends.
pub fn car_trips(terrain: &Terrain, places: &[Vec2]) -> Vec<Trip> {
    let def = terrain.def;
    let roads: Vec<usize> = (0..def.roads.len())
        .filter(|&i| !def.roads[i].runway && !def.roads[i].rail)
        .collect();
    let pts: Vec<Vec<Vec3>> = roads
        .iter()
        .map(|&i| {
            def.roads[i]
                .pts
                .iter()
                .zip(terrain.road_points(i))
                .map(|(p, rp)| Vec3::new(p[0], rp.y, p[1]))
                .collect()
        })
        .collect();
    let w: Vec<f32> = roads.iter().map(|&i| def.roads[i].w).collect();
    let lines = pts.clone();
    // Spots on each road where something happens: its ends, where others join it, and
    // the places cars stop at.
    let mut spots: Vec<Spot> = Vec::new();
    let mut on_road: Vec<Vec<usize>> = vec![Vec::new(); roads.len()];
    let add = |spots: &mut Vec<Spot>, on_road: &mut Vec<Vec<usize>>, road: usize, at: f32| {
        spots.push(Spot { at });
        on_road[road].push(spots.len() - 1);
        spots.len() - 1
    };
    let mut ends = Vec::new();
    for (r, line) in pts.iter().enumerate() {
        let n = line.len();
        if n < 2 {
            continue;
        }
        let a = add(&mut spots, &mut on_road, r, 0.0);
        let b = add(&mut spots, &mut on_road, r, (n - 1) as f32);
        ends.push((r, a, line[0]));
        ends.push((r, b, line[n - 1]));
    }
    // A road end on another road joins it there, and a road whose ends meet is a loop.
    let mut pairs = Vec::new();
    for &(r, s, p) in &ends {
        let flat = Vec2::new(p.x, p.z);
        let mut best: Option<(f32, usize, f32)> = None;
        for o in 0..roads.len() {
            if o == r || pts[o].len() < 2 {
                continue;
            }
            let (d, at) = nearest(&pts[o], flat);
            let level = (point(&pts[o], at).y - p.y).abs() < 4.0;
            if d < (w[o] + w[r]) * 0.5 + 6.0 && level && best.is_none_or(|b| d < b.0) {
                best = Some((d, o, at));
            }
        }
        if let Some((_, o, at)) = best {
            pairs.push((s, add(&mut spots, &mut on_road, o, at)));
        }
        let n = pts[r].len();
        if s % 2 == 1 && pts[r][0].distance(pts[r][n - 1]) < 3.0 {
            pairs.push((s - 1, s));
        }
    }
    // Places to call at: the road nearest each, one stop per stretch of road.
    let mut calls = Vec::new();
    for (k, place) in places.iter().enumerate() {
        let mut best: Option<(f32, usize, f32)> = None;
        for r in 0..roads.len() {
            if pts[r].len() < 2 {
                continue;
            }
            let (d, at) = nearest(&pts[r], *place);
            if d < w[r] * 0.5 + 35.0 && best.is_none_or(|b| d < b.0) {
                best = Some((d, r, at));
            }
        }
        let Some((_, r, at)) = best else {
            continue;
        };
        let p = point(&pts[r], at);
        let crowded = calls
            .iter()
            .any(|&(cr, cat): &(usize, f32)| cr == r && point(&pts[r], cat).distance(p) < 60.0);
        // Not right at a junction or an end either, where turning round would block it.
        let busy = on_road[r]
            .iter()
            .any(|&sp| point(&pts[r], spots[sp].at).distance(p) < 30.0);
        if !crowded && !busy && crate::shape::hash(k as u32 * 7919 + 13) < 0.8 {
            calls.push((r, at));
        }
    }
    let mut stops: Vec<usize> = calls
        .iter()
        .map(|&(r, at)| add(&mut spots, &mut on_road, r, at))
        .collect();
    let mut parent: Vec<usize> = (0..spots.len()).collect();
    for (a, b) in pairs {
        let (x, y) = (find(&mut parent, a), find(&mut parent, b));
        if x != y {
            parent[x] = y;
        }
    }
    let node: Vec<usize> = (0..spots.len()).map(|i| find(&mut parent, i)).collect();
    // Edges between neighboring spots along each road.
    let mut edges: Vec<Edge> = Vec::new();
    let mut links: Vec<Vec<usize>> = vec![Vec::new(); spots.len()];
    for (r, list) in on_road.iter_mut().enumerate() {
        list.sort_by(|&a, &b| spots[a].at.total_cmp(&spots[b].at));
        for pair in list.windows(2) {
            let (from, to) = (spots[pair[0]].at, spots[pair[1]].at);
            let line = stretch(&pts[r], from, to);
            let len: f32 = line.windows(2).map(|q| q[0].distance(q[1])).sum();
            if len < 1.0 {
                continue;
            }
            let (a, b) = (node[pair[0]], node[pair[1]]);
            if a == b {
                continue;
            }
            links[a].push(edges.len());
            links[b].push(edges.len());
            edges.push(Edge {
                a,
                b,
                road: r,
                from,
                to,
                len,
            });
        }
    }
    // Dead ends, road ends that join nothing, are stops too.
    stops.extend((0..spots.len()).filter(|&i| node[i] == i && links[i].len() == 1));
    let stops: Vec<usize> = stops.into_iter().map(|i| node[i]).collect();
    // Shortest ways from a node to every other.
    let ways = |from: usize| {
        let mut dist = vec![f32::MAX; spots.len()];
        let mut via: Vec<Option<usize>> = vec![None; spots.len()];
        let mut open = vec![from];
        dist[from] = 0.0;
        while let Some(k) = (0..open.len()).min_by(|&a, &b| dist[open[a]].total_cmp(&dist[open[b]]))
        {
            let u = open.swap_remove(k);
            for &e in &links[u] {
                let v = if edges[e].a == u {
                    edges[e].b
                } else {
                    edges[e].a
                };
                let d = dist[u] + edges[e].len;
                if d < dist[v] {
                    if dist[v] == f32::MAX {
                        open.push(v);
                    }
                    dist[v] = d;
                    via[v] = Some(e);
                }
            }
        }
        (dist, via)
    };
    let total: f32 = edges.iter().map(|e| e.len).sum();
    let count = (total / 220.0).round() as u32;
    let mut trips = Vec::new();
    if stops.len() < 2 {
        return trips;
    }
    let mut used = vec![0u8; spots.len()];
    for k in 0..count {
        let h = |n: u32| crate::shape::hash(k.wrapping_mul(2_654_435_761) ^ n.wrapping_mul(40_503));
        // A stop has a few spots, one car's each, so no two ever wait in the same one.
        let first = (h(1) * stops.len() as f32) as usize % stops.len();
        let Some(home) = (0..stops.len())
            .map(|j| stops[(first + j) % stops.len()])
            .find(|&s| used[s] < SLOTS)
        else {
            break;
        };
        let (dist, via) = ways(home);
        // Off to somewhere a good way off, so a rider sees a lot of the island.
        let mut far: Vec<usize> = stops
            .iter()
            .copied()
            .filter(|&d| d != home && used[d] < SLOTS && dist[d] < f32::MAX && dist[d] > 800.0)
            .collect();
        if far.is_empty() {
            far = stops
                .iter()
                .copied()
                .filter(|&d| d != home && used[d] < SLOTS && dist[d] < f32::MAX)
                .collect();
        }
        if far.is_empty() {
            used[home] = SLOTS;
            continue;
        }
        far.sort_by(|&a, &b| dist[b].total_cmp(&dist[a]));
        let pick = far[(h(2) * (far.len() as f32 * 0.6).ceil()) as usize % far.len()];
        let (home_slot, far_slot) = (used[home], used[pick]);
        used[home] += 1;
        used[pick] += 1;
        // Walk back from the far end to home, then lay the edges out in order.
        let mut route = Vec::new();
        let mut at = pick;
        while at != home {
            let Some(e) = via[at] else {
                break;
            };
            route.push(e);
            at = if edges[e].a == at {
                edges[e].b
            } else {
                edges[e].a
            };
        }
        route.reverse();
        let mut line: Vec<Vec3> = Vec::new();
        let mut lane: Vec<f32> = Vec::new();
        let mut node_at = home;
        let mut widest = 0.0f32;
        // Where the trip turns from one road onto the next.
        let mut joints: Vec<Vec3> = Vec::new();
        for &e in &route {
            let ed = &edges[e];
            let (from, to, next) = if ed.a == node_at {
                (ed.from, ed.to, ed.b)
            } else {
                (ed.to, ed.from, ed.a)
            };
            let piece = stretch(&pts[ed.road], from, to);
            if !line.is_empty() {
                joints.push(piece[0]);
            }
            for p in piece {
                if line.last().is_none_or(|q| q.distance(p) > 0.5) {
                    line.push(p);
                    lane.push(w[ed.road] * 0.22);
                }
            }
            widest = widest.max(w[ed.road]);
            node_at = next;
        }
        // Round each junction: the points close in to it go, so the car sweeps from one road
        // onto the next instead of zigzagging where one road's end pokes past the other.
        for j in &joints {
            let near: Vec<usize> = (0..line.len())
                .filter(|&i| line[i].distance(*j) < 9.0)
                .collect();
            if near.len() > 1 {
                let (first, last) = (near[0], near[near.len() - 1]);
                if last + 1 < line.len() && first > 0 {
                    line.drain(first..=last);
                    lane.drain(first..=last);
                }
            }
        }
        // Where one road runs into another at a slant, the way can run up to its end and
        // straight back down the other: cut across the hairpin.
        let mut i = 0;
        while i + 2 < line.len() {
            let mut along = 0.0;
            let mut cut = None;
            for j in i + 1..(i + 40).min(line.len()) {
                along += line[j].distance(line[j - 1]);
                if along > 30.0 && line[i].distance(line[j]) < 8.0 {
                    cut = Some(j);
                }
            }
            if let Some(j) = cut {
                line.drain(i + 1..j);
                lane.drain(i + 1..j);
            }
            i += 1;
        }
        // Where a joining road's end pokes past the middle of the road it joins, the line
        // doubles back on itself for a moment: drop those points.
        let mut i = 1;
        while i + 1 < line.len() {
            let (a, b) = (line[i] - line[i - 1], line[i + 1] - line[i]);
            let (a, b) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z));
            if a.length() > 0.01 && b.length() > 0.01 && a.angle_to(b).abs() > 1.75 {
                line.remove(i);
                lane.remove(i);
                i = i.saturating_sub(1).max(1);
            } else {
                i += 1;
            }
        }
        // Each car waits at its own spot, a little short of the stop for the later ones.
        trim_end(&mut line, &mut lane, far_slot as f32 * SLOT_GAP);
        line.reverse();
        lane.reverse();
        trim_end(&mut line, &mut lane, home_slot as f32 * SLOT_GAP);
        line.reverse();
        lane.reverse();
        if line.len() < 2 {
            continue;
        }
        let out = keep_right(&line, &lane);
        let back_line: Vec<Vec3> = line.iter().rev().copied().collect();
        let back_lane: Vec<f32> = lane.iter().rev().copied().collect();
        let back = keep_right(&back_line, &back_lane);
        let n = line.len();
        let dir_out = {
            let d = line[n - 1] - line[n - 2];
            Vec2::new(d.x, d.z).normalize_or(Vec2::Y)
        };
        let dir_home = {
            let d = line[0] - line[1];
            Vec2::new(d.x, d.z).normalize_or(Vec2::Y)
        };
        let length = |p: &[Vec3]| p.windows(2).map(|q| q[0].distance(q[1])).sum::<f32>();
        let mut pts = out;
        let (mut round, wait) = turn(line[n - 1], dir_out, lane[n - 1]);
        lay_on_road(&mut round, &lines, &w, line[n - 1]);
        let far_stop = length(&pts) + wait;
        pts.extend(round);
        pts.extend(back);
        let (mut round, wait) = turn(line[0], dir_home, lane[0]);
        lay_on_road(&mut round, &lines, &w, line[0]);
        let home_stop = length(&pts) + wait;
        pts.extend(round);
        // Round again to the start, for a closed loop.
        pts.push(pts[0]);
        trips.push(Trip {
            pts,
            stops: [far_stop, home_stop],
            widest,
        });
    }
    trips
}
