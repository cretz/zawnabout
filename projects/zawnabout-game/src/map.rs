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
    let step = (b[2] - b[0]) / size as f32;
    let at = |i: u32, j: u32| {
        (
            b[0] + (i as f32 + 0.5) * step,
            b[1] + (j as f32 + 0.5) * step,
        )
    };
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
    // Named places as dots: beacons orange, perches white, sites red, spawn black.
    for m in terrain.def.marks.iter().filter(|_| marks) {
        let color = match m.kind {
            "beacon" => [255, 160, 30],
            "perch" => [255, 255, 255],
            "spawn" => [0, 0, 0],
            k if k.starts_with("site") => [220, 40, 40],
            _ => continue,
        };
        let ci = ((m.pos[0] - b[0]) / step) as i32;
        let cj = ((m.pos[2] - b[1]) / step) as i32;
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
