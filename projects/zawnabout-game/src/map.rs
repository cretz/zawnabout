//! A top-down shaded map of the terrain, for checking the world files and later the
//! minimap.

use glam::Vec3;
use zawnabout_sim::terrain::{CoverKind, Terrain};

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The whole world as `size` by `size` RGBA8 pixels, north up. `marks` dots every
/// named place, for debugging only, since it gives away hidden things.
pub fn render(terrain: &Terrain, size: u32, marks: bool) -> Vec<u8> {
    let b = terrain.def.bounds;
    render_area(terrain, b[0], b[1], b[2] - b[0], size, marks)
}

/// A square of the world, `span` meters across from its corner `(x0, z0)`, as `size` by
/// `size` RGBA8 pixels, for a close look on the big map.
pub fn render_area(
    terrain: &Terrain,
    x0: f32,
    z0: f32,
    span: f32,
    size: u32,
    marks: bool,
) -> Vec<u8> {
    let step = span / size as f32;
    let at = |i: u32, j: u32| (x0 + (i as f32 + 0.5) * step, z0 + (j as f32 + 0.5) * step);
    let mut heights = vec![0.0f32; (size * size) as usize];
    for j in 0..size {
        for i in 0..size {
            let (x, z) = at(i, j);
            heights[(j * size + i) as usize] = terrain.height(x, z);
        }
    }
    let h = |i: u32, j: u32| heights[(j.min(size - 1) * size + i.min(size - 1)) as usize];
    let light = Vec3::new(-0.6, 0.7, -0.4).normalize();
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for j in 0..size {
        for i in 0..size {
            let (x, z) = at(i, j);
            let y = h(i, j);
            let dx = h(i + 1, j) - h(i.saturating_sub(1), j);
            let dz = h(i, j + 1) - h(i, j.saturating_sub(1));
            let n = Vec3::new(-dx, 4.0 * step, -dz).normalize();
            let shade = 0.55 + 0.6 * n.dot(light).max(0.0);
            let c = match terrain.water(x, z, y) {
                Some(level) => {
                    let depth = level - y;
                    let lake = level > 0.5;
                    let deep = if lake {
                        [0.25, 0.55, 0.7]
                    } else {
                        [0.16, 0.38, 0.62]
                    };
                    mix([0.55, 0.82, 0.9], deep, depth / 40.0)
                }
                None => {
                    let base = match terrain.cover(x, z).map(|c| c.kind) {
                        Some(CoverKind::Town) => [0.86, 0.66, 0.5],
                        Some(CoverKind::Forest) => [0.3, 0.55, 0.28],
                        Some(CoverKind::Meadow) => [0.72, 0.85, 0.45],
                        Some(CoverKind::Snow) => [0.94, 0.96, 0.98],
                        None => [0.52, 0.74, 0.38],
                    };
                    let land = if y < 3.5 {
                        [0.9, 0.85, 0.62]
                    } else {
                        mix(base, [0.62, 0.57, 0.52], (y - 350.0) / 400.0)
                    };
                    let land = mix(land, [0.95, 0.95, 0.97], (y - 850.0) / 80.0);
                    land.map(|v| v * shade)
                }
            };
            out.extend(c.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8));
            out.push(255);
        }
    }
    roads(terrain, x0, z0, step, size, &mut out);
    // Named places as dots: beacons orange, perches white, sites red, spawn black.
    for m in terrain.def.marks.iter().filter(|_| marks) {
        let color = match m.kind {
            "beacon" => [255, 160, 30],
            "perch" => [255, 255, 255],
            "spawn" => [0, 0, 0],
            k if k.starts_with("site") => [220, 40, 40],
            _ => continue,
        };
        let ci = ((m.pos[0] - x0) / step) as i32;
        let cj = ((m.pos[2] - z0) / step) as i32;
        for dj in -2..=2 {
            for di in -2..=2 {
                let (i, j) = (ci + di, cj + dj);
                if i >= 0 && j >= 0 && (i as u32) < size && (j as u32) < size {
                    let k = ((j as u32 * size + i as u32) * 4) as usize;
                    out[k..k + 3].copy_from_slice(&color);
                }
            }
        }
    }
    out
}

/// Places worth finding on the big map, one per line as `kind id x z under`, where
/// `kind` is `town`, `village`, `station`, `site:<kind>` or a landmark kind, and `under`
/// is 1 for places down in the caves. Hidden things (caverns, dugouts, beacons) are left
/// out.
pub fn places(terrain: &Terrain) -> String {
    let def = terrain.def;
    let mut out = String::new();
    for t in def.towns {
        let n = t.pts.len().max(1) as f32;
        let (x, z) = t
            .pts
            .iter()
            .fold((0.0, 0.0), |(x, z), p| (x + p[0] / n, z + p[1] / n));
        let kind = if t.village { "village" } else { "town" };
        out += &format!("{kind} {} {x:.0} {z:.0} {}\n", t.id, t.under as u8);
    }
    for m in def.marks {
        let kind = if let Some(rest) = m.kind.strip_prefix("landmark:") {
            rest.split(':').next().unwrap_or(rest)
        } else if m.kind == "station" || m.kind.starts_with("site:") {
            m.kind
        } else {
            continue;
        };
        let [x, y, z] = m.pos;
        let under = !m.ground && y < terrain.height(x, z) - 15.0;
        out += &format!("{kind} {} {x:.0} {z:.0} {}\n", m.id, under as u8);
    }
    out
}

/// Roads, railways and runways drawn over the map: highways gold, roads pale, railways
/// dark, runways grey. Tunnels, the subway and mine lines stay hidden underground.
fn roads(terrain: &Terrain, x0: f32, z0: f32, step: f32, size: u32, out: &mut [u8]) {
    for (ri, r) in terrain.def.roads.iter().enumerate() {
        if r.subway || r.mine {
            continue;
        }
        let color = if r.runway {
            [150, 150, 160]
        } else if r.rail {
            [110, 90, 80]
        } else if r.w >= 14.0 {
            [255, 214, 110]
        } else {
            [246, 240, 228]
        };
        let radius = (r.w * 0.5 / step).max(0.7);
        let prof = terrain.road_points(ri);
        for (k, w) in r.pts.windows(2).enumerate() {
            if prof.get(k).is_some_and(|p| p.tunnel) && prof.get(k + 1).is_some_and(|p| p.tunnel) {
                continue;
            }
            let (a, b) = ((w[0][0], w[0][1]), (w[1][0], w[1][1]));
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let n = (len / (step * 0.5)).ceil().max(1.0) as u32;
            for t in 0..=n {
                let u = t as f32 / n as f32;
                let ci = (a.0 + (b.0 - a.0) * u - x0) / step;
                let cj = (a.1 + (b.1 - a.1) * u - z0) / step;
                let reach = radius.ceil() as i32;
                for dj in -reach..=reach {
                    for di in -reach..=reach {
                        let (i, j) = (ci as i32 + di, cj as i32 + dj);
                        if i < 0 || j < 0 || i as u32 >= size || j as u32 >= size {
                            continue;
                        }
                        let (fx, fz) = (i as f32 + 0.5 - ci, j as f32 + 0.5 - cj);
                        if fx * fx + fz * fz > radius * radius + 0.25 {
                            continue;
                        }
                        let k = ((j as u32 * size + i as u32) * 4) as usize;
                        out[k..k + 3].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}
