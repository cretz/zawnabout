//! Little pictures of each treasure for the journal, drawn once on the CPU from the
//! same models the world shows, so the hoard can show what it holds and not just names.

use glam::{Mat3, Vec2, Vec3};

use crate::mesh::MeshBuilder;

/// A thumbnail's size in pixels, square.
pub const THUMB: u32 = 64;

/// Draws `mesh` turned a little toward the viewer and lit from the upper left, filling
/// a `THUMB` square, as RGBA with a clear background.
pub fn draw(mesh: &MeshBuilder) -> Vec<u8> {
    const SS: usize = 3;
    let n = THUMB as usize * SS;
    let turn = Mat3::from_rotation_x(0.42) * Mat3::from_rotation_y(-0.6);
    let pts: Vec<Vec3> = mesh
        .verts
        .iter()
        .map(|v| turn * Vec3::from(v.pos))
        .collect();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &pts {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let span = (hi - lo).truncate().max_element().max(1e-3);
    let mid = (lo + hi) * 0.5;
    let scale = n as f32 * 0.86 / span;
    let to_px = |p: Vec3| {
        Vec2::new(
            (p.x - mid.x) * scale + n as f32 * 0.5,
            (mid.y - p.y) * scale + n as f32 * 0.5,
        )
    };
    let light = Vec3::new(-0.45, 0.75, 0.5).normalize();
    let mut depth = vec![f32::MIN; n * n];
    let mut color = vec![[0f32; 4]; n * n];
    for (k, tri) in pts.chunks(3).enumerate() {
        let [a, b, c] = [tri[0], tri[1], tri[2]];
        let normal = (b - a).cross(c - a).normalize_or_zero();
        // Thin parts show from both sides.
        let normal = if normal.z < 0.0 { -normal } else { normal };
        let src = mesh.verts[k * 3].color;
        let lin = |c: u8| (c as f32 / 255.0).powf(2.2);
        let shade = 0.45 + 0.65 * normal.dot(light).max(0.0);
        let rgb = [
            lin(src[0]) * shade,
            lin(src[1]) * shade,
            lin(src[2]) * shade,
        ];
        let (pa, pb, pc) = (to_px(a), to_px(b), to_px(c));
        let area = (pb - pa).perp_dot(pc - pa);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = pa.x.min(pb.x).min(pc.x).floor().max(0.0) as usize;
        let x1 = (pa.x.max(pb.x).max(pc.x).ceil() as usize).min(n - 1);
        let y0 = pa.y.min(pb.y).min(pc.y).floor().max(0.0) as usize;
        let y1 = (pa.y.max(pb.y).max(pc.y).ceil() as usize).min(n - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let wa = (pb - p).perp_dot(pc - p) / area;
                let wb = (pc - p).perp_dot(pa - p) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let z = a.z * wa + b.z * wb + c.z * wc;
                let i = y * n + x;
                if z > depth[i] {
                    depth[i] = z;
                    color[i] = [rgb[0], rgb[1], rgb[2], 1.0];
                }
            }
        }
    }
    // Average each block of samples into a pixel, back to sRGB.
    let t = THUMB as usize;
    let mut out = vec![0u8; t * t * 4];
    for y in 0..t {
        for x in 0..t {
            let mut sum = [0f32; 4];
            for sy in 0..SS {
                for sx in 0..SS {
                    let c = color[(y * SS + sy) * n + x * SS + sx];
                    for (s, v) in sum.iter_mut().zip(c) {
                        *s += v;
                    }
                }
            }
            let cover = sum[3] / (SS * SS) as f32;
            let k = (y * t + x) * 4;
            for j in 0..3 {
                let v = if sum[3] > 0.0 { sum[j] / sum[3] } else { 0.0 };
                out[k + j] = (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8;
            }
            out[k + 3] = (cover * 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    /// Every treasure draws something, roughly filling its square.
    #[test]
    fn every_treasure_shows() {
        for kind in crate::treasure::KINDS {
            let px = super::draw(&crate::treasure::treasure(kind));
            let solid = px.chunks(4).filter(|p| p[3] > 128).count();
            assert!(solid > 300, "{kind} covers only {solid} pixels");
        }
    }
}

#[cfg(test)]
mod sheet {
    /// Writes every treasure's picture side by side as raw RGBA, for a look:
    /// `OUT=path cargo test --release -p zawnabout-game thumb_sheet -- --ignored`.
    #[test]
    #[ignore]
    fn thumb_sheet() {
        let t = super::THUMB as usize;
        let kinds = crate::treasure::KINDS;
        let mut out = vec![0u8; t * t * 4 * kinds.len()];
        let w = t * kinds.len();
        for (i, kind) in kinds.iter().enumerate() {
            let px = super::draw(&crate::treasure::treasure(kind));
            for y in 0..t {
                let k = (y * w + i * t) * 4;
                out[k..k + t * 4].copy_from_slice(&px[y * t * 4..(y + 1) * t * 4]);
            }
        }
        std::fs::write(std::env::var("OUT").unwrap(), out).unwrap();
    }
}
