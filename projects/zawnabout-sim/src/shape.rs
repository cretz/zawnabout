//! Solid shapes of props. Rendering builds meshes from the same data, so what you see is
//! what you bump into and land on.

use std::f32::consts::TAU;

use glam::{Quat, Vec2, Vec3};

/// Cheap integer hash to `[0, 1)`.
pub fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// A vertical cone or cylinder section. Collision treats it as round.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    /// Center of the bottom face.
    pub base: Vec3,
    pub height: f32,
    pub r0: f32,
    pub r1: f32,
    /// Spin of the faceted mesh around its axis.
    pub spin: f32,
}

impl Frustum {
    fn radius_at(&self, y: f32) -> f32 {
        let t = ((y - self.base.y) / self.height).clamp(0.0, 1.0);
        self.r0 + (self.r1 - self.r0) * t
    }

    pub fn top(&self) -> f32 {
        self.base.y + self.height
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Frustum(Frustum),
    Sphere { center: Vec3, radius: f32 },
}

impl Shape {
    /// Center and radius of a bounding circle on the ground plane.
    pub fn footprint(&self) -> (Vec2, f32) {
        match self {
            Shape::Frustum(f) => (Vec2::new(f.base.x, f.base.z), f.r0.max(f.r1)),
            Shape::Sphere { center, radius } => (Vec2::new(center.x, center.z), *radius),
        }
    }

    /// Highest standable point of this shape above `(x, z)`, if any.
    pub fn support(&self, x: f32, z: f32) -> Option<f32> {
        match self {
            Shape::Frustum(f) => {
                let d = Vec2::new(x - f.base.x, z - f.base.z).length();
                (d <= f.r1 && f.r1 > 0.5).then(|| f.top())
            }
            Shape::Sphere { center, radius } => {
                let d = Vec2::new(x - center.x, z - center.z).length();
                // Only the gently sloped cap is standable.
                (d <= radius * 0.7).then(|| center.y + (radius * radius - d * d).sqrt())
            }
        }
    }

    /// Pushes a sphere at `p` with radius `r` out of this shape. Returns the outward
    /// normal and depth.
    pub fn push_out(&self, p: Vec3, r: f32) -> Option<(Vec3, f32)> {
        match self {
            Shape::Frustum(f) => {
                if p.y < f.base.y - r || p.y > f.top() + r {
                    return None;
                }
                let flat = Vec2::new(p.x - f.base.x, p.z - f.base.z);
                let d = flat.length();
                let edge = f.radius_at(p.y);
                if p.y > f.top() {
                    // Over the top face: only the rim pushes, the top is for landing.
                    let over = p.y - f.top();
                    let rim = (d - f.r1).max(0.0);
                    let dist = (rim * rim + over * over).sqrt();
                    if d > f.r1 && dist < r {
                        let n =
                            Vec3::new(flat.x / d.max(1e-4) * rim, over, flat.y / d.max(1e-4) * rim)
                                / dist.max(1e-4);
                        return Some((n, r - dist));
                    }
                    return None;
                }
                let depth = edge + r - d;
                if depth <= 0.0 {
                    return None;
                }
                let n = if d > 1e-4 { flat / d } else { Vec2::X };
                Some((Vec3::new(n.x, 0.0, n.y), depth))
            }
            Shape::Sphere { center, radius } => {
                let to = p - *center;
                let d = to.length();
                let depth = radius + r - d;
                (depth > 0.0).then(|| (if d > 1e-4 { to / d } else { Vec3::Y }, depth))
            }
        }
    }
}

/// Spire tiers in prop-local units: unscaled, unrotated, base at the origin.
pub fn spire_tiers(variant: u32) -> Vec<Frustum> {
    let v = variant;
    let tiers = 6 + v as usize;
    let mut out = Vec::with_capacity(tiers + 1);
    let mut y = 0.0;
    let mut r = 8.0 + v as f32;
    let mut off = Vec3::ZERO;
    for i in 0..tiers as u32 {
        let h = 5.0 + hash(v * 31 + i) * 4.0;
        let r1 = r * (0.86 + hash(v * 17 + i) * 0.08);
        out.push(Frustum {
            base: off + Vec3::Y * y,
            height: h,
            r0: r,
            r1,
            spin: hash(v * 5 + i) * TAU,
        });
        y += h;
        r = r1;
        off += Vec3::new(hash(i * 3 + v) - 0.5, 0.0, hash(i * 7 + v) - 0.5) * 1.2;
    }
    // A grassy cap that slightly overhangs the last tier.
    out.push(Frustum {
        base: off + Vec3::Y * (y - 0.2),
        height: 0.7,
        r0: r * 1.04,
        r1: r * 0.9,
        spin: 0.0,
    });
    out
}

/// Places a prop-local frustum in the world.
pub fn place(f: &Frustum, pos: Vec3, yaw: f32, scale: f32) -> Frustum {
    Frustum {
        base: pos + Quat::from_rotation_y(yaw) * (f.base * scale),
        height: f.height * scale,
        r0: f.r0 * scale,
        r1: f.r1 * scale,
        spin: f.spin + yaw,
    }
}
