//! Every model in the game, written by hand from primitives.

use std::f32::consts::{FRAC_PI_2, TAU};

use glam::{Mat4, Quat, Vec3};
use zawnabout_sim::{PropKind, World};

use crate::mesh::{MeshBuilder, Rgb, hash};

pub mod palette {
    use crate::mesh::Rgb;

    pub const SCALE: Rgb = Rgb(0xF2845C);
    pub const BELLY: Rgb = Rgb(0xFFE2B0);
    pub const MEMBRANE: Rgb = Rgb(0xFFC77D);
    pub const HORN: Rgb = Rgb(0xFFF3DC);
    pub const SPIKE: Rgb = Rgb(0xFFCF6E);
    pub const EYE: Rgb = Rgb(0x2B2240);
    pub const SHINE: Rgb = Rgb(0xFFFFFF);
    pub const CHEEK: Rgb = Rgb(0xFF9EAA);
    pub const GRASS: [Rgb; 4] = [Rgb(0x8FD16A), Rgb(0x82C862), Rgb(0x9DD872), Rgb(0x93CC66)];
    pub const MEADOW: Rgb = Rgb(0xB9E07A);
    pub const LEAF: [Rgb; 3] = [Rgb(0x5DB65A), Rgb(0x4FAA52), Rgb(0x79C45E)];
    pub const PINE: [Rgb; 3] = [Rgb(0x3E9A5B), Rgb(0x4BAA66), Rgb(0x358C55)];
    pub const TRUNK: Rgb = Rgb(0x9C6B4E);
    pub const ROCK: [Rgb; 3] = [Rgb(0xBDB5AC), Rgb(0xA9A49E), Rgb(0xCAC0B2)];
    pub const SAND: [Rgb; 3] = [Rgb(0xE8BE8F), Rgb(0xDBA879), Rgb(0xF0CFA6)];
    pub const PETAL: [Rgb; 4] = [Rgb(0xFF8FB1), Rgb(0xFFD166), Rgb(0xFFFFFF), Rgb(0xB79BFF)];
    pub const STEM: Rgb = Rgb(0x5DAA4E);
    pub const POLLEN: Rgb = Rgb(0xFFE08A);
    pub const CLOUD: Rgb = Rgb(0xFFFFFF);
    pub const CLOUD_BELLY: Rgb = Rgb(0xE9EEFA);
}

use palette::*;

/// Number of tail segments.
pub const TAIL_SEGMENTS: usize = 6;
/// Length of each neck segment.
pub const NECK_LEN: f32 = 0.6;
/// Length of each tail segment.
pub const TAIL_LEN: f32 = 0.7;
/// Where each wing attaches on the body (left side, mirror X for right).
pub const SHOULDER: Vec3 = Vec3::new(0.55, 0.45, 0.5);
/// Elbow in the arm frame (left side).
pub const ELBOW: Vec3 = Vec3::new(2.2, 0.0, 0.05);
/// Hips (left side, mirror X for right): front then back.
pub const HIPS: [Vec3; 2] = [Vec3::new(0.48, -0.3, 0.72), Vec3::new(0.5, -0.3, -0.78)];
pub const NECK_BASE: Vec3 = Vec3::new(0.0, 0.3, 1.0);
pub const TAIL_BASE: Vec3 = Vec3::new(0.0, 0.1, -1.25);
pub const EYES: Vec3 = Vec3::new(0.41, 0.36, 0.6);

/// Dragon parts, one mesh each.
pub struct DragonMeshes {
    pub body: MeshBuilder,
    pub neck: MeshBuilder,
    pub head: MeshBuilder,
    pub eye: MeshBuilder,
    pub tail: Vec<MeshBuilder>,
    pub leg: MeshBuilder,
    pub arm: [MeshBuilder; 2],
    pub hand: [MeshBuilder; 2],
}

fn spike(m: &mut MeshBuilder, y: f32, z: f32, size: f32) {
    m.tri(
        Vec3::new(0.0, y, z - size * 0.5),
        Vec3::new(0.0, y + size, z - size * 0.35),
        Vec3::new(0.0, y, z + size * 0.5),
        SPIKE,
    );
}

pub fn dragon() -> DragonMeshes {
    let skin = [SCALE, SCALE, BELLY, SCALE];

    let mut body = MeshBuilder::new();
    body.loft(-1.3, (0.42, 0.42, 0.1), -0.1, (0.68, 0.6, 0.0), skin);
    body.loft(-0.1, (0.68, 0.6, 0.0), 1.15, (0.5, 0.5, 0.18), skin);
    for (z, s) in [(-1.0f32, 0.3), (-0.45, 0.38), (0.15, 0.4), (0.75, 0.34)] {
        spike(&mut body, 0.6 - z.abs() * 0.08, z, s);
    }

    let mut neck = MeshBuilder::new();
    neck.loft(
        -0.1,
        (0.36, 0.36, 0.0),
        NECK_LEN + 0.1,
        (0.31, 0.31, 0.0),
        skin,
    );
    spike(&mut neck, 0.3, NECK_LEN * 0.5, 0.26);

    let mut head = MeshBuilder::new();
    head.loft(-0.15, (0.4, 0.36, 0.25), 0.8, (0.42, 0.37, 0.25), skin);
    head.loft(0.8, (0.32, 0.25, 0.12), 1.4, (0.26, 0.19, 0.08), skin);
    for x in [-1.0f32, 1.0] {
        let horn = {
            let mut h = MeshBuilder::new();
            h.frustum(4, 0.0, 0.11, 0.62, 0.0, HORN, HORN);
            h
        };
        let m = Mat4::from_translation(Vec3::new(0.24 * x, 0.55, 0.15))
            * Mat4::from_quat(Quat::from_rotation_x(-1.05) * Quat::from_rotation_z(-0.35 * x));
        head.append(&horn, m);
        let cx = 0.425 * x;
        head.quad(
            Vec3::new(cx, 0.1, 0.48),
            Vec3::new(cx, 0.1, 0.72),
            Vec3::new(cx, 0.25, 0.72),
            Vec3::new(cx, 0.25, 0.48),
            CHEEK,
        );
        head.cube(
            Vec3::new(0.11 * x, 0.28, 1.36),
            Vec3::new(0.035, 0.02, 0.035),
            EYE,
        );
    }

    let mut eye = MeshBuilder::new();
    eye.cube(Vec3::ZERO, Vec3::new(0.05, 0.11, 0.11), EYE);
    eye.cube(
        Vec3::new(0.035, 0.05, 0.04),
        Vec3::new(0.025, 0.035, 0.035),
        SHINE,
    );

    let sizes = [0.4, 0.34, 0.28, 0.22, 0.16, 0.11, 0.07];
    let tail = (0..TAIL_SEGMENTS)
        .map(|i| {
            let mut t = MeshBuilder::new();
            let (a, b) = (sizes[i], sizes[i + 1]);
            t.loft(-TAIL_LEN - 0.05, (b, b, 0.0), 0.05, (a, a, 0.0), skin);
            spike(&mut t, b * 0.9, -TAIL_LEN * 0.5, a * 0.8);
            if i == TAIL_SEGMENTS - 1 {
                let z = -TAIL_LEN;
                let (l, r) = (
                    Vec3::new(0.32, 0.0, z - 0.25),
                    Vec3::new(-0.32, 0.0, z - 0.25),
                );
                let (front, tip) = (Vec3::new(0.0, 0.0, z + 0.05), Vec3::new(0.0, 0.0, z - 0.65));
                t.tri(front, l, tip, SPIKE);
                t.tri(front, tip, r, SPIKE);
            }
            t
        })
        .collect();

    let mut leg = MeshBuilder::new();
    {
        let mut l = MeshBuilder::new();
        l.loft(-0.1, (0.21, 0.21, 0.0), 1.05, (0.17, 0.17, 0.0), [SCALE; 4]);
        leg.append(&l, Mat4::from_rotation_x(FRAC_PI_2));
        leg.cube(
            Vec3::new(0.0, -1.12, 0.08),
            Vec3::new(0.2, 0.09, 0.26),
            SCALE,
        );
        for x in [-0.12, 0.0, 0.12] {
            leg.cube(Vec3::new(x, -1.16, 0.36), Vec3::new(0.04, 0.05, 0.05), HORN);
        }
    }

    let mut arm = MeshBuilder::new();
    {
        let mut bone = MeshBuilder::new();
        bone.loft(
            -0.1,
            (0.11, 0.11, 0.0),
            ELBOW.x + 0.05,
            (0.08, 0.08, 0.0),
            [SCALE; 4],
        );
        arm.append(&bone, Mat4::from_rotation_y(FRAC_PI_2));
        let pts = [
            Vec3::new(0.0, 0.0, 0.12),
            ELBOW,
            Vec3::new(2.0, 0.0, -1.25),
            Vec3::new(1.05, 0.0, -1.4),
            Vec3::new(0.0, 0.0, -1.55),
        ];
        for w in pts[1..].windows(2) {
            arm.thin_tri(pts[0], w[0], w[1], MEMBRANE);
        }
    }

    let mut hand = MeshBuilder::new();
    {
        let mut bone = MeshBuilder::new();
        bone.loft(
            -0.05,
            (0.08, 0.08, 0.0),
            2.45,
            (0.03, 0.03, 0.0),
            [SCALE; 4],
        );
        hand.append(&bone, Mat4::from_rotation_y(FRAC_PI_2 + 0.08));
        let tip = Vec3::new(2.45, 0.0, -0.2);
        let pts = [
            tip,
            Vec3::new(1.9, 0.0, -0.75),
            Vec3::new(1.55, 0.0, -0.7),
            Vec3::new(1.1, 0.0, -1.25),
            Vec3::new(0.7, 0.0, -1.0),
            Vec3::new(-0.2, 0.0, -1.25),
        ];
        let root = Vec3::new(0.0, 0.0, 0.05);
        for w in pts.windows(2) {
            hand.thin_tri(root, w[0], w[1], MEMBRANE);
        }
    }

    DragonMeshes {
        body,
        neck,
        head,
        eye,
        tail,
        leg,
        arm: [arm.clone(), arm.mirrored_x()],
        hand: [hand.clone(), hand.mirrored_x()],
    }
}

/// Mesh variants per prop kind. Each prop instance picks `variant % variants.len()`.
pub fn prop_variants(kind: PropKind) -> Vec<MeshBuilder> {
    match kind {
        PropKind::RoundTree => (0..3).map(round_tree).collect(),
        PropKind::PineTree => (0..3).map(pine_tree).collect(),
        PropKind::Bush => (0..3)
            .map(|v| {
                let mut m = MeshBuilder::new();
                let c = LEAF[v as usize];
                m.blob(
                    Vec3::new(0.0, 0.6, 0.0),
                    Vec3::new(1.3, 1.0, 1.2),
                    false,
                    0.2,
                    v * 7 + 1,
                    c,
                );
                m.blob(
                    Vec3::new(0.9, 0.45, 0.4),
                    Vec3::new(0.8, 0.7, 0.8),
                    false,
                    0.2,
                    v * 7 + 2,
                    c,
                );
                m
            })
            .collect(),
        PropKind::Rock => (0..3)
            .map(|v| {
                let mut m = MeshBuilder::new();
                m.blob(
                    Vec3::new(0.0, 0.5, 0.0),
                    Vec3::new(1.4, 1.0, 1.1),
                    false,
                    0.3,
                    v * 13 + 5,
                    ROCK[v as usize],
                );
                m
            })
            .collect(),
        PropKind::Spire => (0..3).map(spire).collect(),
        PropKind::Flower => (0..4).map(flower).collect(),
        PropKind::Cloud => (0..3).map(cloud).collect(),
    }
}

fn round_tree(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.frustum(5, 0.0, 0.4, 3.2, 0.28, TRUNK, TRUNK);
    let leaf = LEAF[v as usize];
    m.blob(
        Vec3::new(0.0, 4.6, 0.0),
        Vec3::new(2.5, 2.2, 2.5),
        false,
        0.16,
        v * 3 + 11,
        leaf,
    );
    m.blob(
        Vec3::new(1.2, 3.8, 0.6),
        Vec3::new(1.4, 1.3, 1.4),
        false,
        0.16,
        v * 3 + 12,
        leaf.mix(Rgb(0xFFFFFF), 0.08),
    );
    m
}

fn pine_tree(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.frustum(5, 0.0, 0.35, 2.0, 0.25, TRUNK, TRUNK);
    let c = PINE[v as usize];
    let tiers = 3 + v;
    for i in 0..tiers {
        let t = i as f32 / tiers as f32;
        let y = 1.6 + i as f32 * 1.7;
        m.frustum(
            7,
            y,
            2.4 * (1.0 - t * 0.6),
            y + 2.6,
            0.0,
            c.mix(Rgb(0xFFFFFF), t * 0.1),
            c,
        );
    }
    m
}

fn spire(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let tiers = 6 + v as usize;
    let mut y = 0.0;
    let mut r = 8.0 + v as f32;
    let mut off = Vec3::ZERO;
    for i in 0..tiers {
        let h = 5.0 + hash(v * 31 + i as u32) * 4.0;
        let r1 = r * (0.86 + hash(v * 17 + i as u32) * 0.08);
        let mut tier = MeshBuilder::new();
        let c = SAND[(i + v as usize) % 3];
        tier.frustum(6, 0.0, r, h, r1, c, c);
        let spin = hash(v * 5 + i as u32) * TAU;
        m.append(
            &tier,
            Mat4::from_translation(off + Vec3::Y * y) * Mat4::from_rotation_y(spin),
        );
        y += h;
        r = r1;
        off += Vec3::new(
            hash(i as u32 * 3 + v) - 0.5,
            0.0,
            hash(i as u32 * 7 + v) - 0.5,
        ) * 1.2;
    }
    let top = off + Vec3::Y * y;
    let mut cap = MeshBuilder::new();
    cap.frustum(6, -0.2, r * 1.04, 0.5, r * 0.9, GRASS[0], GRASS[2]);
    m.append(&cap, Mat4::from_translation(top));
    m.append(
        &round_tree(v % 3),
        Mat4::from_translation(top + Vec3::new(r * 0.3, 0.3, 0.0)),
    );
    m
}

fn flower(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.frustum(3, 0.0, 0.06, 0.9, 0.04, STEM, STEM);
    let petal = PETAL[v as usize];
    let c = Vec3::new(0.0, 0.95, 0.0);
    for i in 0..5 {
        let a = i as f32 / 5.0 * TAU;
        let dir = Vec3::new(a.cos(), 0.15, a.sin());
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.16;
        m.thin_tri(c, c + dir * 0.4 + side, c + dir * 0.4 - side, petal);
    }
    m.cube(c + Vec3::Y * 0.03, Vec3::splat(0.08), POLLEN);
    m
}

fn cloud(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let puffs = 4 + v;
    for i in 0..puffs {
        let t = i as f32 / (puffs - 1) as f32;
        let x = (t - 0.5) * 22.0;
        let r = 7.0 + (1.0 - (t - 0.5).abs() * 2.0) * 5.0 + hash(v * 9 + i) * 2.0;
        let c = Vec3::new(x, r * 0.2, (hash(v * 4 + i) - 0.5) * 6.0);
        m.blob(
            c,
            Vec3::new(r, r * 0.72, r * 0.85),
            true,
            0.08,
            v * 19 + i,
            CLOUD,
        );
    }
    // A flat, slightly shaded belly.
    let n = m.verts.len();
    for t in m.verts[..n].as_chunks_mut::<3>().0 {
        if t.iter().all(|p| p.pos[1] < 0.5) {
            for p in t {
                p.pos[1] = p.pos[1].max(-0.5);
                p.color = CLOUD_BELLY.bytes();
            }
        }
    }
    m
}

/// The ground as one flat-shaded grid that follows the world height, plus a low skirt
/// out to the fog.
pub fn ground(world: &World) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let extent = world.half_size + 300.0;
    let cells = 240;
    let step = extent * 2.0 / cells as f32;
    let at = |i: u32, j: u32| {
        let (x, z) = (-extent + i as f32 * step, -extent + j as f32 * step);
        Vec3::new(x, world.ground_height(x, z), z)
    };
    for j in 0..cells {
        for i in 0..cells {
            let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            let h = hash(i * 7919 + j * 104_729);
            let base = GRASS[0].mix(GRASS[(h * 4.0) as usize % 4], 0.55);
            let center = (a + c) * 0.5;
            let meadow = ((center.x * 0.004).sin() * (center.z * 0.005).cos()).max(0.0);
            let col = base.mix(MEADOW, meadow * 0.6);
            let h2 = hash(i * 31 + j * 977 + 5);
            m.tri(a, d, c, col);
            m.tri(a, c, b, col.mix(GRASS[(h2 * 4.0) as usize % 4], 0.3));
        }
    }
    let (inner, outer, y) = (extent, extent * 4.0, -6.0);
    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        let p = |c: (f32, f32), r: f32| Vec3::new(c.0 * r, y, c.1 * r);
        m.quad(p(a, inner), p(a, outer), p(b, outer), p(b, inner), GRASS[1]);
    }
    m
}

/// A unit disc in the XZ plane for blob shadows. Red holds the fade (1 center, 0 rim).
pub fn shadow_disc() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let sides = 20;
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        m.tri(
            Vec3::ZERO,
            Vec3::new(a1.cos(), 0.0, a1.sin()),
            Vec3::new(a0.cos(), 0.0, a0.sin()),
            Rgb(0x000000),
        );
    }
    for (i, v) in m.verts.iter_mut().enumerate() {
        v.color[0] = if i % 3 == 0 { 255 } else { 0 };
    }
    m
}
