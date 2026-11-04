//! Every model in the game, written by hand from primitives.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec3};
use zawnabout_sim::PropKind;
use zawnabout_sim::shape::spire_tiers;
use zawnabout_sim::world::spire_tree_offset;

use crate::mesh::{METAL, MeshBuilder, Rgb, SHEEN, hash};

pub mod palette {
    use crate::mesh::Rgb;

    pub const SCALE: Rgb = Rgb(0xF2845C);
    pub const BACK: Rgb = Rgb(0xD9503A);
    pub const BELLY: Rgb = Rgb(0xFFE2B0);
    pub const BELLY2: Rgb = Rgb(0xF8CF94);
    pub const MEMBRANE: Rgb = Rgb(0xFFD49A);
    pub const MEMBRANE_DEEP: Rgb = Rgb(0xEE7B4C);
    pub const FRILL: Rgb = Rgb(0xFFC27A);
    pub const HORN: Rgb = Rgb(0xFFF3DC);
    pub const GOLD: Rgb = Rgb(0xF5B62E);
    pub const GOLD_LIGHT: Rgb = Rgb(0xFFE6A0);
    pub const FEATHER: Rgb = Rgb(0xF59A3A);
    pub const FEATHER_TIP: Rgb = Rgb(0xFFF0C8);
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
pub const TAIL_SEGMENTS: usize = 8;
/// Thickness of the tail where it leaves the body, and how much each segment tapers.
pub const TAIL_ROOT: f32 = 0.4;
pub const TAIL_TAPER: f32 = 0.84;
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
pub const EYES: Vec3 = Vec3::new(0.38, 0.3, 0.62);
/// Where the whiskers grow from on the snout (left side), and each whisker piece's length.
pub const SNOUT: Vec3 = Vec3::new(0.16, 0.08, 1.3);
pub const WHISKER_LEN: f32 = 0.34;
pub const WHISKER_SEGMENTS: usize = 3;

/// Dragon parts, one mesh each. The tail, whisker and mane pieces are each one mesh drawn
/// several times.
pub struct DragonMeshes {
    pub body: MeshBuilder,
    pub neck: MeshBuilder,
    pub head: MeshBuilder,
    pub eye: MeshBuilder,
    pub tail: MeshBuilder,
    pub plume: MeshBuilder,
    pub whisker: MeshBuilder,
    pub mane: MeshBuilder,
    pub leg: MeshBuilder,
    pub arm: [MeshBuilder; 2],
    pub hand: [MeshBuilder; 2],
    pub star: MeshBuilder,
}

/// A rounded body along z through `rings` of (z, half width, half height, center y),
/// `sides` faces round. Each face is colored by how far up it faces and which ring it
/// starts from. Both ends are capped.
fn lathe(
    m: &mut MeshBuilder,
    rings: &[(f32, f32, f32, f32)],
    sides: u32,
    color: impl Fn(f32, usize) -> Rgb,
) {
    // Corners sit half a face round, so a face lies flat along the back and the belly.
    let at = |r: (f32, f32, f32, f32), k: u32| {
        let a = (k as f32 + 0.5) / sides as f32 * TAU;
        Vec3::new(a.cos() * r.1, r.3 + a.sin() * r.2, r.0)
    };
    for (i, w) in rings.windows(2).enumerate() {
        for k in 0..sides {
            let up = ((k + 1) as f32 / sides as f32 * TAU).sin();
            m.quad(
                at(w[0], k),
                at(w[0], k + 1),
                at(w[1], k + 1),
                at(w[1], k),
                color(up, i),
            );
        }
    }
    for r in [rings[0], rings[rings.len() - 1]] {
        let c = Vec3::new(0.0, r.3, r.0);
        for k in 0..sides {
            m.tri(c, at(r, k), at(r, k + 1), color(0.0, 0));
        }
    }
}

/// Scale colors: a deep coral back, warm sides and cream belly plates in bands.
fn skin(up: f32, ring: usize) -> Rgb {
    if up > 0.9 {
        BACK
    } else if up > 0.5 {
        BACK.mix(SCALE, 0.55)
    } else if up > -0.5 {
        SCALE
    } else if ring.is_multiple_of(2) {
        BELLY
    } else {
        BELLY2
    }
}

/// A gold blade standing on `at`: `len` long at its foot, `h` tall, leaning back, `w`
/// thick, its tip catching the light.
fn blade(m: &mut MeshBuilder, at: Vec3, len: f32, h: f32, w: f32) {
    let (f, b) = (at + Vec3::Z * len * 0.5, at - Vec3::Z * len * 0.5);
    let (l, r) = (at + Vec3::X * w, at - Vec3::X * w);
    let tip = at + Vec3::new(0.0, h, -len * 0.7);
    let from = m.verts.len();
    for (p, q) in [(f, l), (l, b), (b, r), (r, f)] {
        m.tri_colors(p, q, tip, [GOLD, GOLD, GOLD_LIGHT], 0);
    }
    m.flag_from(from, METAL);
}

/// A feather from `root` along `dir`, `len` long and `w` wide across `side`, shading from
/// its root color to a pale tip. Thin, so the sun glows through it.
fn feather(m: &mut MeshBuilder, root: Vec3, dir: Vec3, side: Vec3, len: f32, w: f32) {
    let mid = root + dir * len * 0.4;
    let tip = root + dir * len;
    let (l, r) = (mid + side * w, mid - side * w);
    let half = FEATHER.mix(FEATHER_TIP, 0.45);
    m.tri_colors(root, l, tip, [FEATHER, half, FEATHER_TIP], 127);
    m.tri_colors(root, tip, r, [FEATHER, FEATHER_TIP, half], 127);
}

/// A curved claw from `at` reaching along `dir`, hooking down at the end.
fn talon(m: &mut MeshBuilder, at: Vec3, dir: Vec3, len: f32, r: f32) {
    let from = m.verts.len();
    let mid = at + dir * len * 0.55;
    tube(m, 4, at, r, mid, r * 0.75, HORN);
    tube(
        m,
        4,
        mid,
        r * 0.75,
        mid + dir * len * 0.3 - Vec3::Y * len * 0.45,
        0.0,
        GOLD,
    );
    m.flag_from(from, METAL);
}

/// A wing membrane fanned from `root` round `edge`, deep near the root and paler toward
/// its trailing edge, with the edge pulled in between finger tips like a bat's.
fn membrane(m: &mut MeshBuilder, root: Vec3, tips: &[Vec3]) {
    let mut edge = vec![tips[0]];
    for w in tips.windows(2) {
        let mid = (w[0] + w[1]) * 0.5;
        edge.push(mid + (root - mid) * 0.16);
        edge.push(w[1]);
    }
    for w in edge.windows(2) {
        m.tri_colors(root, w[0], w[1], [MEMBRANE_DEEP, MEMBRANE, MEMBRANE], 127);
    }
}

pub fn dragon() -> DragonMeshes {
    let mut body = MeshBuilder::new();
    let rings = [
        (-1.35, 0.4, 0.38, 0.1),
        (-0.8, 0.58, 0.52, 0.04),
        (-0.15, 0.7, 0.62, 0.0),
        (0.5, 0.68, 0.62, 0.06),
        (1.0, 0.54, 0.52, 0.2),
        (1.3, 0.38, 0.38, 0.3),
    ];
    lathe(&mut body, &rings, 8, skin);
    body.flag_from(0, SHEEN);
    // A crest of gold blades down the spine, tallest over the shoulders.
    let top = |z: f32| {
        let i = rings.iter().rposition(|r| r.0 <= z).unwrap_or(0).min(4);
        let (a, b) = (rings[i], rings[i + 1]);
        let t = ((z - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
        a.3 + (b.3 - a.3) * t + (a.2 + (b.2 - a.2) * t) * 0.9
    };
    for (z, h) in [
        (-1.1f32, 0.26),
        (-0.65, 0.34),
        (-0.15, 0.42),
        (0.35, 0.44),
        (0.85, 0.34),
    ] {
        blade(&mut body, Vec3::new(0.0, top(z) - 0.04, z), 0.42, h, 0.07);
    }

    let mut neck = MeshBuilder::new();
    lathe(
        &mut neck,
        &[(-0.1, 0.38, 0.38, 0.0), (NECK_LEN + 0.1, 0.32, 0.32, 0.0)],
        8,
        skin,
    );
    neck.flag_from(0, SHEEN);
    blade(
        &mut neck,
        Vec3::new(0.0, 0.3, NECK_LEN * 0.5),
        0.34,
        0.3,
        0.06,
    );

    let mut head = MeshBuilder::new();
    lathe(
        &mut head,
        &[
            (-0.18, 0.34, 0.3, 0.27),
            (0.12, 0.42, 0.38, 0.27),
            (0.55, 0.43, 0.38, 0.26),
            (0.85, 0.34, 0.28, 0.16),
            (1.2, 0.27, 0.21, 0.11),
            (1.45, 0.2, 0.15, 0.09),
        ],
        8,
        |up, ring| if up < -0.5 { BELLY } else { skin(up, ring) },
    );
    head.flag_from(0, SHEEN);
    for x in [-1.0f32, 1.0] {
        // Horns sweep back in two curves, ivory into gold.
        let from = head.verts.len();
        let base = Vec3::new(0.22 * x, 0.58, 0.12);
        let mid = base + Vec3::new(0.08 * x, 0.3, -0.32);
        let tip = mid + Vec3::new(0.05 * x, 0.1, -0.46);
        tube(&mut head, 5, base, 0.1, mid, 0.065, HORN);
        tube(&mut head, 5, mid, 0.065, tip, 0.0, GOLD);
        head.flag_from(from, METAL);
        // Frilled ears fanning back behind the eyes.
        let root = Vec3::new(0.36 * x, 0.42, 0.08);
        let fan = [
            root + Vec3::new(0.3 * x, 0.38, -0.42),
            root + Vec3::new(0.5 * x, 0.12, -0.55),
            root + Vec3::new(0.38 * x, -0.16, -0.42),
        ];
        for w in fan.windows(2) {
            head.tri_colors(root, w[0], w[1], [MEMBRANE_DEEP, FRILL, FRILL], 127);
        }
        head.cube(
            Vec3::new(0.37 * x, 0.12, 0.8),
            Vec3::new(0.03, 0.05, 0.09),
            CHEEK,
        );
        head.cube(
            Vec3::new(0.1 * x, 0.2, 1.45),
            Vec3::new(0.035, 0.025, 0.03),
            EYE,
        );
        // Two little fangs, more cute than fierce.
        tube(
            &mut head,
            3,
            Vec3::new(0.13 * x, 0.0, 1.3),
            0.035,
            Vec3::new(0.13 * x, -0.1, 1.31),
            0.0,
            SHINE,
        );
    }
    for (z, h) in [(0.32f32, 0.16), (0.02, 0.2)] {
        blade(&mut head, Vec3::new(0.0, 0.58, z), 0.22, h, 0.05);
    }

    let mut eye = MeshBuilder::new();
    eye.cube(Vec3::ZERO, Vec3::new(0.05, 0.12, 0.12), EYE);
    eye.cube(
        Vec3::new(0.035, 0.055, 0.045),
        Vec3::new(0.025, 0.04, 0.04),
        SHINE,
    );
    eye.cube(
        Vec3::new(0.035, -0.05, -0.04),
        Vec3::new(0.02, 0.018, 0.018),
        SHINE,
    );

    // One tail piece, drawn at a smaller size down the tail.
    let mut tail = MeshBuilder::new();
    lathe(
        &mut tail,
        &[
            (0.05, 1.0, 1.0, 0.0),
            (-TAIL_LEN - 0.05, TAIL_TAPER, TAIL_TAPER, 0.0),
        ],
        8,
        skin,
    );
    tail.flag_from(0, SHEEN);
    blade(
        &mut tail,
        Vec3::new(0.0, 0.85, -TAIL_LEN * 0.45),
        0.9,
        0.75,
        0.14,
    );

    // A fan of long feathers at the tail's tip, with a gold blade at its heart.
    let mut plume = MeshBuilder::new();
    for (a, len) in [
        (0.0f32, 1.5),
        (0.35, 1.3),
        (-0.35, 1.3),
        (0.7, 1.0),
        (-0.7, 1.0),
    ] {
        let dir = Vec3::new(a.sin(), 0.06, -a.cos()).normalize();
        let side = Vec3::Y.cross(dir).normalize();
        feather(&mut plume, Vec3::ZERO, dir, side, len, 0.2);
    }
    for (p, len) in [(0.45f32, 1.0), (-0.3, 0.8)] {
        let dir = Vec3::new(0.0, p.sin(), -p.cos());
        feather(&mut plume, Vec3::ZERO, dir, Vec3::X, len, 0.16);
    }
    blade(&mut plume, Vec3::new(0.0, -0.02, -0.2), 0.5, 0.18, 0.06);

    let mut whisker = MeshBuilder::new();
    tube(
        &mut whisker,
        4,
        Vec3::ZERO,
        0.034,
        Vec3::Z * (WHISKER_LEN + 0.02),
        0.024,
        GOLD_LIGHT,
    );
    whisker.flag_from(0, METAL);

    // A ruff of feathers riding the back of the neck.
    let mut mane = MeshBuilder::new();
    for (x, len) in [(0.0f32, 0.75), (0.16, 0.6), (-0.16, 0.6)] {
        let dir = Vec3::new(x * 0.8, 0.55, -0.83).normalize();
        feather(&mut mane, Vec3::new(x, 0.0, 0.0), dir, Vec3::X, len, 0.13);
    }

    let mut leg = MeshBuilder::new();
    {
        let knee = Vec3::new(0.0, -0.55, 0.08);
        let ankle = Vec3::new(0.0, -1.02, 0.0);
        tube(
            &mut leg,
            6,
            Vec3::new(0.0, 0.05, 0.0),
            0.3,
            knee,
            0.23,
            SCALE,
        );
        tube(&mut leg, 6, knee, 0.22, ankle, 0.17, SCALE);
        leg.cube(Vec3::new(0.0, -1.1, 0.1), Vec3::new(0.2, 0.08, 0.24), SCALE);
        leg.flag_from(0, SHEEN);
        for x in [-0.13f32, 0.0, 0.13] {
            talon(
                &mut leg,
                Vec3::new(x, -1.11, 0.3),
                Vec3::new(x * 0.6, 0.0, 1.0).normalize(),
                0.26,
                0.055,
            );
        }
        talon(
            &mut leg,
            Vec3::new(0.0, -1.08, -0.12),
            Vec3::NEG_Z,
            0.16,
            0.04,
        );
    }

    let mut arm = MeshBuilder::new();
    {
        tube(
            &mut arm,
            6,
            Vec3::new(-0.1, 0.0, 0.0),
            0.12,
            ELBOW + Vec3::X * 0.05,
            0.08,
            SCALE,
        );
        arm.flag_from(0, SHEEN);
        // A thumb claw at the wrist, like a bat's.
        talon(
            &mut arm,
            ELBOW + Vec3::Y * 0.04,
            Vec3::new(0.1, 0.2, 1.0).normalize(),
            0.3,
            0.05,
        );
        membrane(
            &mut arm,
            Vec3::new(0.0, 0.0, 0.12),
            &[
                ELBOW,
                Vec3::new(2.0, 0.0, -1.45),
                Vec3::new(0.95, 0.0, -1.62),
                Vec3::new(0.0, 0.0, -1.7),
            ],
        );
    }

    let mut hand = MeshBuilder::new();
    {
        let root = Vec3::new(0.0, 0.0, 0.05);
        let fingers = [
            Vec3::new(2.6, 0.0, -0.22),
            Vec3::new(2.1, 0.0, -1.15),
            Vec3::new(1.15, 0.0, -1.6),
        ];
        for (i, f) in fingers.iter().enumerate() {
            let r = if i == 0 { 0.08 } else { 0.045 };
            tube(&mut hand, 5, root, r, *f, 0.018, SCALE);
        }
        hand.flag_from(0, SHEEN);
        membrane(
            &mut hand,
            root,
            &[
                fingers[0],
                fingers[1],
                fingers[2],
                Vec3::new(-0.2, 0.0, -1.45),
            ],
        );
    }

    let mut star = MeshBuilder::new();
    for i in 0..5 {
        let a = |k: f32| (i as f32 + k) / 5.0 * TAU;
        let point = |k: f32, r: f32| Vec3::new(a(k).sin() * r, a(k).cos() * r, 0.0);
        star.thin_tri(Vec3::ZERO, point(0.0, 1.0), point(0.5, 0.45), POLLEN);
        star.thin_tri(Vec3::ZERO, point(-0.5, 0.45), point(0.0, 1.0), POLLEN);
    }

    DragonMeshes {
        body,
        neck,
        head,
        eye,
        tail,
        plume,
        whisker,
        mane,
        leg,
        arm: [arm.clone(), arm.mirrored_x()],
        hand: [hand.clone(), hand.mirrored_x()],
        star,
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
        // Plain boulders, then (from 3) craggy, mossy ones for cave mouths.
        PropKind::Rock => (0..6)
            .map(|v| {
                let mut m = MeshBuilder::new();
                if v < 3 {
                    m.blob(
                        Vec3::new(0.0, 0.5, 0.0),
                        Vec3::new(1.4, 1.0, 1.1),
                        false,
                        0.3,
                        v * 13 + 5,
                        ROCK[v as usize],
                    );
                } else {
                    let c = ROCK[v as usize - 3].mix(Rgb(0x5E554E), 0.45);
                    m.blob(
                        Vec3::new(0.0, 0.5, 0.0),
                        Vec3::new(1.3, 1.05, 1.0),
                        false,
                        0.45,
                        v * 29 + 3,
                        c,
                    );
                    // Moss settles on the tops.
                    let moss = Rgb(0x5E8C3E).mix(c, 0.25);
                    for t in m.verts.chunks_mut(3) {
                        let up = (t[0].normal[1] as f32) / 127.0;
                        if up > 0.55 {
                            for v in t {
                                v.color = moss.bytes();
                            }
                        }
                    }
                }
                m
            })
            .collect(),
        PropKind::Spire => (0..3).map(spire).collect(),
        PropKind::Flower => (0..4).map(flower).collect(),
        PropKind::Cloud => (0..3).map(cloud).collect(),
        PropKind::Crystal => (0..CRYSTAL.len() as u32).map(crystal).collect(),
        PropKind::Shroom => (0..SHROOM.len() as u32).map(shroom).collect(),
        // Small ones plain, giants (variants from 3) built up in drips.
        PropKind::Stalagmite => (0..3)
            .map(small_stalagmite)
            .chain((0..3).map(stalagmite))
            .collect(),
        PropKind::Stalactite => vec![small_stalactite(), stalactite()],
        PropKind::Beacon => vec![beacon()],
        PropKind::Ruin => (0..zawnabout_sim::cave::RUIN_KINDS.len() as u8)
            .map(ruin)
            .collect(),
        PropKind::Palm => (0..3).map(palm).collect(),
        PropKind::GiantShroom => (0..SHROOM.len() as u32).map(giant_shroom).collect(),
        PropKind::Glowworms => (0..3).map(glowworms).collect(),
        PropKind::Vines => (0..3).map(vines).collect(),
        PropKind::Brackets => (0..SHROOM.len() as u32).map(brackets).collect(),
    }
}

/// A round piece from `a` to `b`, radius `ra` at `a` and `rb` at `b`.
fn tube(m: &mut MeshBuilder, sides: u32, a: Vec3, ra: f32, b: Vec3, rb: f32, c: Rgb) {
    let mut t = MeshBuilder::new();
    t.frustum(sides, 0.0, ra, (b - a).length(), rb, c, c);
    let rot = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or(Vec3::Y));
    m.append(&t, Mat4::from_rotation_translation(rot, a));
}

/// A giant cave mushroom: a leaning stalk, a broad domed cap to land on, glowing gills
/// underneath and glowing spots on top. About 8 m tall at scale 1.
fn giant_shroom(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let cap = SHROOM[v as usize % SHROOM.len()].mix(Rgb(0x6A4A8A), 0.35);
    let gill = SHROOM[v as usize % SHROOM.len()];
    let stalk = Rgb(0xEDE3D0);
    let top = zawnabout_sim::cave::shroom_top(v as u8);
    let bend = |t: f32| Vec3::new(top.x * t * t, top.y * t, top.z * t * t);
    let radii = [0.8, 0.6, 0.5, 0.45];
    for i in 0..3 {
        let (t0, t1) = (i as f32 / 3.0, (i + 1) as f32 / 3.0);
        tube(&mut m, 5, bend(t0), radii[i], bend(t1), radii[i + 1], stalk);
    }
    let mut c = MeshBuilder::new();
    c.frustum(9, 0.0, 3.4, 0.9, 2.9, cap, cap);
    c.frustum(9, 0.9, 2.9, 1.6, 1.7, cap, cap);
    c.frustum(9, 1.6, 1.7, 1.9, 0.0, cap, cap);
    let from = c.verts.len();
    c.frustum(9, -0.5, 0.5, 0.0, 3.35, gill, gill);
    for i in 0..5u32 {
        let a = i as f32 * 2.3 + hash(v * 17 + i);
        let (r, y) = if i == 0 {
            (0.0, 1.9)
        } else {
            (1.5 + hash(v + i) * 1.0, 1.25)
        };
        let mut spot = MeshBuilder::new();
        spot.frustum(5, 0.0, 0.34, 0.14, 0.0, Rgb(0xFFF6E6), Rgb(0xFFF6E6));
        c.append(
            &spot,
            Mat4::from_translation(Vec3::new(a.cos() * r, y, a.sin() * r)),
        );
    }
    c.glow_from(from);
    m.append(&c, Mat4::from_translation(top));
    m
}

/// Shelf fungi stepping up a wall: half discs sticking out toward +z from the wall at
/// z = 0, pale on top and glowing underneath. About 3 m tall at scale 1.
fn brackets(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let glow = SHROOM[v as usize % SHROOM.len()];
    let top = glow.mix(Rgb(0xF2E6D2), 0.65);
    let mut under = MeshBuilder::new();
    for i in 0..5u32 {
        let x = (hash(v * 7 + i) - 0.5) * 2.2;
        let y = i as f32 * 0.65 + hash(v * 11 + i) * 0.3;
        let r = 0.45 + hash(v * 13 + i) * 0.55;
        let sides = 6;
        for k in 0..sides {
            let a0 = k as f32 / sides as f32 * PI;
            let a1 = (k + 1) as f32 / sides as f32 * PI;
            let rim = |a: f32, y: f32| Vec3::new(x + a.cos() * r, y, a.sin() * r);
            let c = Vec3::new(x, y + 0.12, -0.05);
            // A gentle dome on top, a flat glowing gill underneath.
            m.tri(c, rim(a1, y), rim(a0, y), top);
            under.tri(
                Vec3::new(x, y - 0.02, -0.05),
                rim(a0, y - 0.02),
                rim(a1, y - 0.02),
                glow,
            );
        }
    }
    let from = m.verts.len();
    m.append(&under, Mat4::IDENTITY);
    m.glow_from(from);
    m
}

/// Vines hanging over a cave mouth: a curtain of strands with leaves all down them, some
/// long, some short, the odd one in flower. Hangs down from the origin, up to about 7 m
/// at scale 1.
fn vines(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let greens = [Rgb(0x4E8C3A), Rgb(0x5FA046), Rgb(0x3F7A34)];
    let stem = Rgb(0x5A6B32);
    let flower = [Rgb(0xF2A6C8), Rgb(0xFFF1A8), Rgb(0xC9A6F2)][v as usize % 3];
    for i in 0..10u32 {
        let x = (i as f32 / 9.0 - 0.5) * 4.0 + (hash(v * 29 + i) - 0.5) * 0.6;
        let z = (hash(v * 31 + i) - 0.5) * 0.8;
        let top = Vec3::new(x, 0.3, z);
        let len = 1.5 + hash(v * 41 + i) * 5.5;
        let sway = Vec3::new((hash(v * 43 + i) - 0.5) * 0.8, 0.0, 0.3);
        let bot = top - Vec3::Y * len + sway;
        m.thin_tri(top - Vec3::X * 0.04, top + Vec3::X * 0.04, bot, stem);
        let n = 2 + (len * 1.2) as u32;
        for k in 1..=n {
            let p = top.lerp(bot, k as f32 / n as f32);
            let a = hash(v * 47 + i * 13 + k) * std::f32::consts::TAU;
            let out = Vec3::new(a.cos(), -0.3, a.sin()) * 0.45;
            let wide = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.18;
            let c = if hash(v * 53 + i * 7 + k) < 0.08 {
                flower
            } else {
                greens[(i + k) as usize % 3]
            };
            m.thin_tri(p, p + out + wide, p + out * 1.3 - wide, c);
        }
    }
    m
}

/// Glowworm threads hanging from a cave roof: a curtain of fine strands beaded with
/// glowing drops. Hangs down from the origin, up to about 7 m at scale 1.
fn glowworms(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let thread = Rgb(0xA8C8D0);
    let bead = [Rgb(0x8CF5FF), Rgb(0xB4FFD8), Rgb(0xC8D8FF)][v as usize % 3];
    let mut beads = MeshBuilder::new();
    for i in 0..7u32 {
        let a = hash(v * 31 + i) * TAU;
        let d = hash(v * 37 + i).sqrt() * 2.8;
        let top = Vec3::new(a.cos() * d, 0.3, a.sin() * d);
        let len = 2.0 + hash(v * 41 + i) * 5.0;
        let bot = top - Vec3::Y * len;
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.03;
        m.thin_tri(top - side, top + side, bot, thread);
        let n = 2 + (len * 0.6) as u32;
        for k in 1..=n {
            let p = top.lerp(bot, k as f32 / n as f32);
            let s = 0.08 + 0.05 * (k == n) as u32 as f32;
            let t = hash(v * 53 + i * 7 + k) * TAU;
            let x = Vec3::new(t.cos(), 0.0, t.sin()) * s;
            let y = Vec3::Y * s * 1.4;
            beads.tri(p + y, p + x, p - y, bead);
            beads.tri(p + y, p - y, p - x, bead);
        }
    }
    let from = m.verts.len();
    m.append(&beads, Mat4::IDENTITY);
    m.glow_from(from);
    m
}

/// A palm: a ringed trunk curving up and over, a crown of drooping fronds and coconuts.
fn palm(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let lean = 0.9 + v as f32 * 0.5;
    let segments = 6;
    let at = |t: f32| Vec3::new(lean * t * t, t * 6.6, 0.0);
    for i in 0..segments {
        let (t0, t1) = (i as f32 / segments as f32, (i + 1) as f32 / segments as f32);
        let (a, b) = (at(t0), at(t1));
        let r = |t: f32| 0.32 - t * 0.12;
        let c = if i % 2 == 0 {
            Rgb(0xA8875E)
        } else {
            Rgb(0x8F7150)
        };
        let mut seg = MeshBuilder::new();
        seg.frustum(5, 0.0, r(t0), (b - a).length(), r(t1), c, c);
        let tilt = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize());
        m.append(&seg, Mat4::from_rotation_translation(tilt, a));
    }
    let top = at(1.0);
    let fronds = 7 + v;
    for k in 0..fronds {
        let a = k as f32 / fronds as f32 * TAU + v as f32;
        let out = Vec3::new(a.cos(), 0.0, a.sin());
        let side = Vec3::new(-out.z, 0.0, out.x);
        let len = 2.6 + (k % 3) as f32 * 0.4;
        let mid = top + out * len * 0.5 + Vec3::Y * 0.7;
        let tip = top + out * len - Vec3::Y * (0.9 + (k % 2) as f32 * 0.5);
        let green = [Rgb(0x4FA046), Rgb(0x5DB04C), Rgb(0x46923F)][(k % 3) as usize];
        m.thin_tri(top, mid + side * 0.55, mid, green);
        m.thin_tri(mid, mid + side * 0.55, tip, green.mix(Rgb(0x000000), 0.1));
        m.thin_tri(top, mid, mid - side * 0.55, green);
        m.thin_tri(mid, tip, mid - side * 0.55, green.mix(Rgb(0x000000), 0.1));
    }
    for k in 0..3 {
        let a = k as f32 * 2.1;
        m.blob(
            top + Vec3::new(a.cos() * 0.3, -0.35, a.sin() * 0.3),
            Vec3::splat(0.2),
            false,
            0.0,
            k,
            Rgb(0x6B4A2E),
        );
    }
    m
}

/// Crystal colors by variant, matching `zawnabout_sim::cave`: ice, pink, violet, teal,
/// amber, green.
pub const CRYSTAL: [Rgb; 6] = [
    Rgb(0x8FE3FF),
    Rgb(0xFF9AD5),
    Rgb(0xB69CFF),
    Rgb(0x6FF2D2),
    Rgb(0xFFB54D),
    Rgb(0xA8F07A),
];
/// Mushroom cap colors by variant.
pub const SHROOM: [Rgb; 4] = [Rgb(0x7FF5FF), Rgb(0xFF8CEB), Rgb(0xC8FF6E), Rgb(0xFFC46B)];
// The same rock as cave walls, so they read as part of the cave.
const CAVE_ROCK: [Rgb; 3] = [Rgb(0x8C8078), Rgb(0x7E746E), Rgb(0x988A7C)];
const OLD_STONE: [Rgb; 2] = [Rgb(0xD9CDB8), Rgb(0xC7BBA6)];
const MOSS: Rgb = Rgb(0x8CBF73);

/// One pointed shard from `base` along `dir`, open at the bottom where it meets rock.
fn shard(m: &mut MeshBuilder, sides: u32, base: Vec3, dir: Vec3, len: f32, r: f32, c: Rgb) {
    let rot = Quat::from_rotation_arc(Vec3::Y, dir.normalize());
    let at = |i: u32, y: f32| {
        let a = i as f32 / sides as f32 * TAU;
        base + rot * Vec3::new(a.cos() * r, y, a.sin() * r)
    };
    let tip = base + rot * Vec3::Y * len;
    let body = len * 0.72;
    let light = c.mix(Rgb(0xFFFFFF), 0.35);
    for i in 0..sides {
        let shade = if i % 2 == 0 {
            c
        } else {
            c.mix(Rgb(0x000000), 0.1)
        };
        m.quad(
            at(i, -0.3),
            at(i, body),
            at(i + 1, body),
            at(i + 1, -0.3),
            shade,
        );
        m.tri(at(i, body), tip, at(i + 1, body), light);
    }
}

/// A cone from a ring at `base` to `tip`, with no bottom face.
fn cone(m: &mut MeshBuilder, sides: u32, base: Vec3, r: f32, tip: Vec3, c: Rgb) {
    let up = (tip - base).normalize_or(Vec3::Y);
    let rot = Quat::from_rotation_arc(Vec3::Y, up);
    let at = |i: u32| {
        let a = i as f32 / sides as f32 * TAU;
        base + rot * Vec3::new(a.cos() * r, 0.0, a.sin() * r)
    };
    for i in 0..sides {
        let shade = if i % 2 == 0 {
            c
        } else {
            c.mix(Rgb(0x000000), 0.08)
        };
        // Wound to face out whichever way the cone points.
        if up.y >= 0.0 {
            m.tri(at(i + 1), tip, at(i), shade);
        } else {
            m.tri(at(i), tip, at(i + 1), shade);
        }
    }
}

fn crystal(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = CRYSTAL[v as usize];
    // A tall center shard and a few shorter ones leaning out.
    shard(&mut m, 6, Vec3::ZERO, Vec3::Y, 3.2, 0.42, c);
    for i in 0..3 {
        let a = i as f32 / 3.0 * TAU + hash(v * 5 + i) * 0.8;
        let lean = 0.45 + hash(v * 7 + i) * 0.4;
        let dir = Vec3::new(a.cos() * lean, 1.0, a.sin() * lean);
        let len = 1.3 + hash(v * 11 + i) * 1.3;
        let off = Vec3::new(a.cos(), 0.0, a.sin()) * 0.35;
        let tone = c.mix(Rgb(0xFFFFFF), 0.12 * (i % 2) as f32);
        shard(&mut m, 4, off, dir, len, 0.3, tone);
    }
    m.glow_from(0);
    m
}

fn shroom(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let cap = SHROOM[v as usize];
    let stalk = Rgb(0xF2E8D8);
    for i in 0..4 {
        let a = i as f32 * 2.4 + v as f32;
        let d = if i == 0 {
            0.0
        } else {
            0.9 + hash(v * 3 + i) * 0.8
        };
        let at = Vec3::new(a.cos() * d, 0.0, a.sin() * d);
        let h = if i == 0 {
            1.6
        } else {
            0.6 + hash(v * 13 + i) * 0.8
        };
        let r = h * 0.5;
        cone(&mut m, 3, at, 0.13 * h, at + Vec3::Y * h, stalk);
        let from = m.verts.len();
        // The cap: an umbrella with a lighter underside.
        let rim = at + Vec3::Y * h * 0.9;
        cone(&mut m, 6, rim, r, at + Vec3::Y * h * 1.3, cap);
        cone(
            &mut m,
            6,
            rim,
            r,
            at + Vec3::Y * h * 0.8,
            cap.mix(Rgb(0xFFFFFF), 0.4),
        );
        m.glow_from(from);
    }
    m
}

fn small_stalagmite(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = CAVE_ROCK[v as usize];
    cone(&mut m, 5, Vec3::Y * -0.4, 1.6, Vec3::Y * 5.0, c);
    cone(
        &mut m,
        4,
        Vec3::new(1.3, -0.4, 0.4),
        0.7,
        Vec3::new(1.4, 2.2 + v as f32 * 0.4, 0.5),
        c,
    );
    m
}

fn small_stalactite() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = CAVE_ROCK[2];
    cone(&mut m, 5, Vec3::Y * 0.6, 1.3, Vec3::Y * -5.0, c);
    cone(
        &mut m,
        4,
        Vec3::new(1.0, 0.6, -0.5),
        0.6,
        Vec3::new(1.0, -2.6, -0.5),
        c,
    );
    m
}

fn stalagmite(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = CAVE_ROCK[v as usize];
    // Built up in drips: a wide foot, bulging tiers, a leaning tip, and a smaller sister.
    let h = |k: u32| hash(v * 13 + k);
    let lean = Vec3::new(h(1) - 0.5, 0.0, h(2) - 0.5) * 0.5;
    let pts = [
        Vec3::Y * -0.4,
        Vec3::Y * 1.2 + lean * 0.1,
        Vec3::Y * 2.6 + lean * 0.4,
        Vec3::Y * 3.9 + lean * 0.7,
        Vec3::Y * 5.0 + lean,
    ];
    let radii = [1.7, 1.15, 0.95, 0.55, 0.0];
    for i in 0..3 {
        tube(
            &mut m,
            6,
            pts[i],
            radii[i],
            pts[i + 1],
            radii[i + 1] * 1.15,
            c,
        );
        // A drip ring where each tier meets the next.
        tube(
            &mut m,
            6,
            pts[i + 1] - Vec3::Y * 0.15,
            radii[i + 1] * 1.15,
            pts[i + 1] + Vec3::Y * 0.1,
            radii[i + 1],
            c.mix(Rgb(0xFFFFFF), 0.06),
        );
    }
    cone(&mut m, 6, pts[3], radii[3], pts[4], c);
    cone(
        &mut m,
        5,
        Vec3::new(1.3, -0.4, 0.4),
        0.7,
        Vec3::new(1.4, 2.2 + v as f32 * 0.4, 0.5),
        c,
    );
    m
}

fn stalactite() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = CAVE_ROCK[2];
    // Hangs down from the roof at the origin, in drips like a stalagmite upside down.
    let pts = [
        Vec3::Y * 0.6,
        Vec3::new(0.05, -1.2, 0.0),
        Vec3::new(0.1, -2.7, 0.05),
        Vec3::new(0.15, -3.9, 0.1),
        Vec3::new(0.2, -5.0, 0.1),
    ];
    let radii = [1.3, 0.95, 0.75, 0.45, 0.0];
    for i in 0..3 {
        tube(
            &mut m,
            6,
            pts[i],
            radii[i],
            pts[i + 1],
            radii[i + 1] * 1.12,
            c,
        );
    }
    cone(&mut m, 6, pts[3], radii[3], pts[4], c);
    cone(
        &mut m,
        4,
        Vec3::new(1.0, 0.6, -0.5),
        0.6,
        Vec3::new(1.0, -2.6, -0.5),
        c,
    );
    m
}

fn beacon() -> MeshBuilder {
    use zawnabout_sim::cave::{BEACON_POST, BEACON_RIM, BEACON_TOP};
    let mut m = MeshBuilder::new();
    let stone = OLD_STONE[0];
    m.frustum(
        6,
        0.0,
        1.25,
        0.35,
        1.2,
        stone.mix(Rgb(0x000000), 0.1),
        stone,
    );
    m.frustum(6, 0.35, 0.95, BEACON_POST, 0.75, stone, stone);
    // The bowl, dark inside with old coals.
    m.frustum(
        8,
        BEACON_POST,
        0.75,
        BEACON_TOP,
        BEACON_RIM,
        Rgb(0x8C6E5A),
        Rgb(0x4A3A36),
    );
    m
}

fn ruin(v: u8) -> MeshBuilder {
    use zawnabout_sim::cave::Part;
    let mut m = MeshBuilder::new();
    for (i, part) in zawnabout_sim::cave::ruin_parts(v).into_iter().enumerate() {
        let c = OLD_STONE[i % 2];
        match part {
            Part::Box { base, half } => {
                m.cube(
                    base + Vec3::Y * half.y * 0.5,
                    Vec3::new(half.x, half.y * 0.5, half.z),
                    c,
                );
                // A little moss on top.
                m.cube(
                    base + Vec3::Y * (half.y + 0.05),
                    Vec3::new(half.x * 0.6, 0.08, half.z * 0.7),
                    MOSS,
                );
            }
            Part::Column { base, h, r } => {
                let mut col = MeshBuilder::new();
                col.frustum(8, 0.0, r, h, r * 0.85, c, MOSS);
                col.frustum(
                    8,
                    0.0,
                    r * 1.25,
                    0.5,
                    r * 1.15,
                    c.mix(Rgb(0x000000), 0.08),
                    c,
                );
                m.append(&col, Mat4::from_translation(base));
            }
        }
    }
    if v == 4 {
        // Pointy ears on the statue's head: an animal folk elder.
        for s in [-1.0f32, 1.0] {
            m.tri(
                Vec3::new(0.55 * s, 8.8, -0.3),
                Vec3::new(0.75 * s, 10.0, 0.0),
                Vec3::new(0.25 * s, 8.8, 0.3),
                OLD_STONE[1],
            );
        }
    }
    m
}

/// A unit flame: a bright core in an orange shell, alpha fading up. Drawn glowing.
pub fn flame() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let shells = [
        (1.0, 2.6, Rgb(0xFF7A2E), 150u8),
        (0.6, 1.9, Rgb(0xFFD36B), 230u8),
    ];
    for (r, h, c, a) in shells {
        let from = m.verts.len();
        m.frustum(6, 0.0, r, h, 0.0, c, c);
        for v in &mut m.verts[from..] {
            let up = (v.pos[1] / h).clamp(0.0, 1.0);
            v.color[3] = (a as f32 * (1.0 - up * 0.7)) as u8;
        }
    }
    m
}

/// A unit disc in the XZ plane, bright in the middle and fading to nothing at the rim.
/// Drawn glowing: pools of lamplight and halos.
pub fn pool() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let sides = 16;
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        m.tri(
            Vec3::ZERO,
            Vec3::new(a1.cos(), 0.0, a1.sin()),
            Vec3::new(a0.cos(), 0.0, a0.sin()),
            Rgb(0xFFFFFF),
        );
    }
    for (i, v) in m.verts.iter_mut().enumerate() {
        v.color[3] = if i % 3 == 0 { 255 } else { 0 };
    }
    m
}

/// A puff of fire or ember for breath, unit size.
pub fn ember() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(Vec3::ZERO, Vec3::ONE, false, 0.15, 3, Rgb(0xFFFFFF));
    m
}

/// A soft column of light from `a` down to `b`, `r` wide: three crossed planes, bright in
/// the middle and fading to the edges and ends. Drawn glowing.
pub fn light_beam(m: &mut MeshBuilder, a: Vec3, b: Vec3, r: f32, fade_a: f32, fade_b: f32) {
    let axis = (b - a).normalize_or(Vec3::NEG_Y);
    let side0 = axis.any_orthonormal_vector();
    let color = Rgb(0xFFF4D6);
    for k in 0..3 {
        let side = Quat::from_axis_angle(axis, k as f32 * TAU / 6.0) * side0 * r;
        for s in [-1.0f32, 1.0] {
            let (ea, eb) = (a + side * s, b + side * s);
            let n = m.verts.len();
            m.quad(a, ea, eb, b, color);
            // Quad corners per triangle: a ea eb, then a eb b.
            let alpha = [fade_a, 0.0, 0.0, fade_a, 0.0, fade_b];
            for (v, al) in m.verts[n..].iter_mut().zip(alpha) {
                v.color[3] = (al * 255.0) as u8;
            }
        }
    }
}

/// A pair of skis and poles for a skier standing at the origin facing +z.
pub fn skis() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (board, pole) = (Rgb(0xE0564A), Rgb(0x8E9AA6));
    for x in [-0.14f32, 0.14] {
        m.cube(
            Vec3::new(x, 0.03, 0.15),
            Vec3::new(0.05, 0.025, 0.85),
            board,
        );
        m.cube(Vec3::new(x, 0.09, 0.98), Vec3::new(0.05, 0.03, 0.06), board);
        tube(
            &mut m,
            3,
            Vec3::new(x * 2.6, 0.95, 0.15),
            0.025,
            Vec3::new(x * 3.0, 0.05, -0.35),
            0.02,
            pole,
        );
    }
    m
}

/// A tuft of grass blades, about knee high to a person, dark at the root and pale at the
/// tips. Its color comes from the instance tint, and the blades sway in the wind (their
/// normal's w marks them, see the shader).
pub fn tuft() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    for i in 0..5 {
        let a = i as f32 / 5.0 * TAU + 0.4;
        let out = Vec3::new(a.cos(), 0.0, a.sin());
        let side = Vec3::new(-out.z, 0.0, out.x) * 0.07;
        let root = out * 0.08;
        let tip = out * (0.28 + 0.06 * (i % 2) as f32) + Vec3::Y * (0.62 + 0.12 * (i % 3) as f32);
        m.thin_tri(root - side, root + side, tip, Rgb(0xA4B09C));
        let n = m.verts.len();
        m.verts[n - 1].color = Rgb(0xFFFFF0).bytes();
    }
    for v in &mut m.verts {
        v.normal[3] = SWAY;
    }
    m
}

/// A gull, wings out along x, flying toward +z. Scaling y by -1 to 1 flaps it.
pub fn gull() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (front, back) = (Vec3::new(0.0, 0.0, 0.3), Vec3::new(0.0, 0.0, -0.35));
    for x in [-1.0f32, 1.0] {
        let elbow = Vec3::new(x * 0.55, 0.18, 0.02);
        let tip = Vec3::new(x * 1.25, 0.32, -0.15);
        m.thin_tri(front, back, elbow, Rgb(0xF4F4F0));
        m.thin_tri(elbow, back, tip, Rgb(0xE6E8EA));
        let n = m.verts.len();
        m.verts[n - 1].color = Rgb(0x4A4E55).bytes();
    }
    m.tri(front, Vec3::new(0.0, -0.08, -0.1), back, Rgb(0xF4F4F0));
    m.tri(
        front,
        Vec3::new(0.0, 0.0, 0.42),
        Vec3::new(0.0, -0.05, 0.25),
        Rgb(0xF2B33D),
    );
    m
}

/// A bat, nose toward +z: a dark body and scalloped membrane wings. Flap by scaling y.
pub fn bat() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (front, back) = (Vec3::new(0.0, 0.0, 0.22), Vec3::new(0.0, 0.0, -0.2));
    let wing = Rgb(0x4A3A44);
    for x in [-1.0f32, 1.0] {
        let elbow = Vec3::new(x * 0.4, 0.14, 0.12);
        let finger = Vec3::new(x * 0.95, 0.26, 0.05);
        let trail = Vec3::new(x * 0.6, 0.12, -0.22);
        m.thin_tri(front, back, elbow, wing);
        m.thin_tri(elbow, back, trail, wing);
        m.thin_tri(elbow, trail, finger, wing.mix(Rgb(0x000000), 0.2));
    }
    m.blob(
        Vec3::new(0.0, 0.0, 0.02),
        Vec3::new(0.09, 0.08, 0.2),
        false,
        0.1,
        9,
        Rgb(0x2E2428),
    );
    for x in [-0.05f32, 0.05] {
        m.tri(
            Vec3::new(x, 0.04, 0.16),
            Vec3::new(x * 1.6, 0.16, 0.13),
            Vec3::new(x * 0.4, 0.06, 0.12),
            Rgb(0x2E2428),
        );
    }
    m
}

/// A woolly sheep, nose toward +z, feet at the origin.
pub fn sheep() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::new(0.0, 0.78, 0.0),
        Vec3::new(0.48, 0.42, 0.68),
        false,
        0.12,
        5,
        Rgb(0xF3F0E6),
    );
    m.cube(
        Vec3::new(0.0, 0.88, 0.72),
        Vec3::new(0.15, 0.17, 0.2),
        Rgb(0x3A3230),
    );
    for (x, z) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        m.cube(
            Vec3::new(x * 0.24, 0.24, z * 0.36),
            Vec3::new(0.06, 0.24, 0.06),
            Rgb(0x3A3230),
        );
    }
    m
}

/// Four legs under a body, `h` tall, `x` and `z` apart.
fn legs(m: &mut MeshBuilder, x: f32, z: f32, h: f32, r: f32, c: Rgb) {
    for (sx, sz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        m.cube(
            Vec3::new(sx * x, h * 0.5, sz * z),
            Vec3::new(r, h * 0.5, r),
            c,
        );
    }
}

/// A black and white cow, facing +z, feet at the origin.
pub fn cow() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (white, black, pink) = (Rgb(0xF4F1EA), Rgb(0x2E2A2A), Rgb(0xF0B8B0));
    m.cube(Vec3::new(0.0, 1.1, 0.0), Vec3::new(0.42, 0.36, 0.85), white);
    m.cube(
        Vec3::new(0.2, 1.25, 0.25),
        Vec3::new(0.24, 0.24, 0.3),
        black,
    );
    m.cube(
        Vec3::new(-0.22, 1.05, -0.4),
        Vec3::new(0.22, 0.26, 0.28),
        black,
    );
    m.cube(
        Vec3::new(0.0, 1.2, 1.05),
        Vec3::new(0.22, 0.24, 0.28),
        black,
    );
    m.cube(
        Vec3::new(0.0, 1.05, 1.33),
        Vec3::new(0.17, 0.12, 0.08),
        pink,
    );
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.22, 1.47, 1.0),
            Vec3::new(0.06, 0.06, 0.04),
            white,
        );
        m.cube(
            Vec3::new(x * 0.3, 1.32, 0.98),
            Vec3::new(0.1, 0.04, 0.05),
            black,
        );
    }
    m.cube(Vec3::new(0.0, 0.72, 0.3), Vec3::new(0.14, 0.08, 0.14), pink);
    m.cube(
        Vec3::new(0.0, 0.95, -0.9),
        Vec3::new(0.04, 0.3, 0.04),
        black,
    );
    legs(&mut m, 0.28, 0.6, 0.75, 0.08, white);
    m
}

/// A chestnut horse with a dark mane, facing +z, feet at the origin.
pub fn horse() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (coat, dark) = (Rgb(0x9A5B34), Rgb(0x3B2618));
    m.cube(Vec3::new(0.0, 1.35, 0.0), Vec3::new(0.3, 0.32, 0.8), coat);
    let neck = Mat4::from_translation(Vec3::new(0.0, 1.7, 0.85)) * Mat4::from_rotation_x(-0.7);
    let mut n = MeshBuilder::new();
    n.cube(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.16, 0.18, 0.45), coat);
    n.cube(
        Vec3::new(0.0, 0.2, -0.05),
        Vec3::new(0.05, 0.08, 0.45),
        dark,
    );
    m.append(&n, neck);
    m.cube(Vec3::new(0.0, 2.05, 1.3), Vec3::new(0.13, 0.15, 0.32), coat);
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.08, 2.25, 1.12),
            Vec3::new(0.03, 0.08, 0.03),
            coat,
        );
    }
    let tail = Mat4::from_translation(Vec3::new(0.0, 1.4, -0.9)) * Mat4::from_rotation_x(0.5);
    let mut t = MeshBuilder::new();
    t.cube(Vec3::new(0.0, -0.3, 0.0), Vec3::new(0.06, 0.35, 0.06), dark);
    m.append(&t, tail);
    legs(&mut m, 0.2, 0.6, 1.05, 0.07, coat);
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        m.cube(
            Vec3::new(x * 0.2, 0.06, z * 0.6),
            Vec3::new(0.08, 0.06, 0.08),
            dark,
        );
    }
    m
}

/// A deer with a white tail and small antlers, facing +z, feet at the origin.
pub fn deer() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (coat, pale, horn) = (Rgb(0xB57A48), Rgb(0xF2E6D2), Rgb(0x7A5A3C));
    m.cube(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.2, 0.22, 0.55), coat);
    m.cube(Vec3::new(0.0, 0.85, 0.0), Vec3::new(0.17, 0.08, 0.45), pale);
    let neck = Mat4::from_translation(Vec3::new(0.0, 1.3, 0.55)) * Mat4::from_rotation_x(-0.8);
    let mut n = MeshBuilder::new();
    n.cube(Vec3::ZERO, Vec3::new(0.09, 0.1, 0.3), coat);
    m.append(&n, neck);
    m.cube(Vec3::new(0.0, 1.55, 0.78), Vec3::new(0.09, 0.1, 0.2), coat);
    m.cube(
        Vec3::new(0.0, 1.5, 0.98),
        Vec3::new(0.04, 0.04, 0.04),
        Rgb(0x2E2A2A),
    );
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.13, 1.68, 0.7),
            Vec3::new(0.06, 0.04, 0.02),
            coat,
        );
        tube(
            &mut m,
            3,
            Vec3::new(x * 0.05, 1.65, 0.72),
            0.025,
            Vec3::new(x * 0.18, 1.98, 0.6),
            0.01,
            horn,
        );
    }
    m.cube(
        Vec3::new(0.0, 1.12, -0.58),
        Vec3::new(0.06, 0.08, 0.05),
        pale,
    );
    legs(&mut m, 0.13, 0.42, 0.8, 0.04, coat);
    m
}

/// A small brown dog, facing +z, feet at the origin.
pub fn dog() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (coat, dark) = (Rgb(0xC08A50), Rgb(0x5A3A22));
    m.cube(Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.14, 0.13, 0.3), coat);
    m.cube(
        Vec3::new(0.0, 0.58, 0.34),
        Vec3::new(0.12, 0.12, 0.13),
        coat,
    );
    m.cube(Vec3::new(0.0, 0.53, 0.5), Vec3::new(0.06, 0.05, 0.06), dark);
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.11, 0.66, 0.3),
            Vec3::new(0.03, 0.08, 0.04),
            dark,
        );
    }
    m.cube(
        Vec3::new(0.0, 0.58, -0.33),
        Vec3::new(0.03, 0.1, 0.03),
        coat,
    );
    legs(&mut m, 0.09, 0.2, 0.3, 0.035, coat);
    m
}

/// An eagle with its wings spread, dark brown with a white head, facing +z.
pub fn eagle() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (brown, white, beak) = (Rgb(0x5A3E2B), Rgb(0xF4F1EA), Rgb(0xF2C14E));
    m.cube(Vec3::ZERO, Vec3::new(0.12, 0.1, 0.38), brown);
    m.cube(
        Vec3::new(0.0, 0.04, 0.42),
        Vec3::new(0.09, 0.09, 0.1),
        white,
    );
    m.cube(Vec3::new(0.0, 0.0, 0.54), Vec3::new(0.03, 0.03, 0.04), beak);
    m.cube(
        Vec3::new(0.0, 0.0, -0.45),
        Vec3::new(0.12, 0.02, 0.1),
        white,
    );
    for x in [-1.0f32, 1.0] {
        let root = Vec3::new(x * 0.1, 0.02, 0.1);
        let tip = Vec3::new(x * 1.1, 0.12, -0.05);
        m.thin_tri(root, Vec3::new(x * 0.1, 0.02, -0.2), tip, brown);
        m.thin_tri(root, tip, Vec3::new(x * 0.7, 0.08, 0.12), brown);
        // Splayed fingers at the tips.
        for k in 0..3 {
            let a = Vec3::new(x * (0.9 + k as f32 * 0.07), 0.1, -0.02 - k as f32 * 0.06);
            m.thin_tri(
                a,
                a + Vec3::new(x * 0.28, 0.02, -0.04),
                a + Vec3::new(0.0, 0.0, -0.06),
                Rgb(0x3E2A1E),
            );
        }
    }
    m
}

/// A little songbird, wings out, about 20cm across, facing +z.
pub fn songbird() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (body, breast) = (Rgb(0x7A5A3C), Rgb(0xE8A060));
    m.cube(Vec3::ZERO, Vec3::new(0.03, 0.03, 0.06), body);
    m.cube(
        Vec3::new(0.0, -0.015, 0.03),
        Vec3::new(0.025, 0.02, 0.03),
        breast,
    );
    for x in [-1.0f32, 1.0] {
        m.thin_tri(
            Vec3::new(x * 0.02, 0.01, 0.03),
            Vec3::new(x * 0.02, 0.01, -0.04),
            Vec3::new(x * 0.12, 0.02, -0.02),
            body,
        );
    }
    m
}

/// A butterfly, white so the instance tint colors its wings, facing +z. The wings open
/// along x.
pub fn butterfly() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let wing = Rgb(0xFFFFFF);
    m.cube(Vec3::ZERO, Vec3::new(0.012, 0.012, 0.05), Rgb(0x3A3A40));
    for x in [-1.0f32, 1.0] {
        // Held up in a V, so they show from the side too.
        m.thin_tri(
            Vec3::ZERO,
            Vec3::new(x * 0.1, 0.06, 0.09),
            Vec3::new(x * 0.11, 0.07, -0.01),
            wing,
        );
        m.thin_tri(
            Vec3::ZERO,
            Vec3::new(x * 0.08, 0.05, -0.02),
            Vec3::new(x * 0.05, 0.03, -0.09),
            wing,
        );
    }
    m
}

/// A wooden rowboat with someone at the oars, waterline at the origin, bow +z.
pub fn rowboat() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (wood, inside) = (Rgb(0x9A6B47), Rgb(0xC69A6A));
    m.cube(Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.6, 0.2, 1.4), wood);
    m.cube(
        Vec3::new(0.0, 0.15, 1.45),
        Vec3::new(0.35, 0.15, 0.15),
        wood,
    );
    m.cube(Vec3::new(0.0, 0.31, 0.0), Vec3::new(0.5, 0.02, 1.3), inside);
    m.cube(Vec3::new(0.0, 0.4, -0.2), Vec3::new(0.5, 0.04, 0.12), wood);
    m.cube(
        Vec3::new(0.0, 0.7, -0.2),
        Vec3::new(0.18, 0.28, 0.14),
        Rgb(0x4A90C8),
    );
    m.cube(
        Vec3::new(0.0, 1.1, -0.2),
        Vec3::new(0.12, 0.13, 0.12),
        Rgb(0xF2C8A8),
    );
    m.cube(
        Vec3::new(0.0, 1.25, -0.2),
        Vec3::new(0.15, 0.05, 0.15),
        Rgb(0xF2C14E),
    );
    for x in [-1.0f32, 1.0] {
        tube(
            &mut m,
            3,
            Vec3::new(x * 0.25, 0.75, 0.05),
            0.03,
            Vec3::new(x * 1.7, 0.0, -0.3),
            0.03,
            wood,
        );
    }
    m
}

/// A kayak with its paddler, waterline at the origin, bow +z.
pub fn kayak() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let hull = Rgb(0xF29E4C);
    m.cube(Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.3, 0.14, 1.5), hull);
    m.cube(Vec3::new(0.0, 0.1, 1.7), Vec3::new(0.12, 0.08, 0.25), hull);
    m.cube(Vec3::new(0.0, 0.1, -1.7), Vec3::new(0.12, 0.08, 0.25), hull);
    m.cube(
        Vec3::new(0.0, 0.45, 0.0),
        Vec3::new(0.17, 0.25, 0.13),
        Rgb(0xE0564A),
    );
    m.cube(
        Vec3::new(0.0, 0.83, 0.0),
        Vec3::new(0.12, 0.13, 0.12),
        Rgb(0xF2C8A8),
    );
    m.cube(
        Vec3::new(0.0, 0.98, 0.0),
        Vec3::new(0.14, 0.06, 0.14),
        Rgb(0xF6F1E7),
    );
    tube(
        &mut m,
        3,
        Vec3::new(-1.1, 0.25, 0.2),
        0.025,
        Vec3::new(1.1, 0.75, 0.25),
        0.025,
        Rgb(0x3A3A40),
    );
    m
}

/// One third of a flag, white for the tint: a strip 0.75m along x hanging down from the
/// origin.
pub fn flag() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = Rgb(0xFFFFFF);
    m.thin_tri(
        Vec3::ZERO,
        Vec3::new(0.78, 0.0, 0.0),
        Vec3::new(0.78, -1.3, 0.0),
        c,
    );
    m.thin_tri(
        Vec3::ZERO,
        Vec3::new(0.78, -1.3, 0.0),
        Vec3::new(0.0, -1.3, 0.0),
        c,
    );
    m
}

/// A flagpole 5m tall with its top at the origin.
pub fn flagpole() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    tube(
        &mut m,
        4,
        Vec3::new(0.0, -5.0, 0.0),
        0.07,
        Vec3::ZERO,
        0.05,
        Rgb(0xD8D8DC),
    );
    m.cube(
        Vec3::new(0.0, 0.08, 0.0),
        Vec3::new(0.09, 0.09, 0.09),
        Rgb(0xF2C14E),
    );
    m
}

/// A person's body, white so the instance tint colors the clothes, feet at the origin.
pub fn townsfolk_body() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.frustum(5, 0.0, 0.2, 0.8, 0.22, Rgb(0x9A9AA0), Rgb(0x9A9AA0));
    m.frustum(5, 0.8, 0.24, 1.42, 0.18, Rgb(0xFFFFFF), Rgb(0xFFFFFF));
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.27, 1.12, 0.0),
            Vec3::new(0.06, 0.28, 0.07),
            Rgb(0xF4F4F4),
        );
    }
    m
}

/// A head for a person: 0 human, 1 rabbit folk, 2 bear folk.
pub fn townsfolk_head(variant: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let head = Vec3::new(0.0, 1.62, 0.0);
    let skin = [Rgb(0xE8B88E), Rgb(0xD9C2A4), Rgb(0x9A6A44)][variant as usize % 3];
    m.blob(head, Vec3::splat(0.18), false, 0.05, 11 + variant, skin);
    match variant % 3 {
        0 => {
            let hair = [Rgb(0x4A3020), Rgb(0xE0B060), Rgb(0x222222)][(variant / 3) as usize % 3];
            m.frustum(6, 1.66, 0.19, 1.83, 0.08, hair, hair);
        }
        1 => {
            for x in [-1.0f32, 1.0] {
                m.cube(
                    head + Vec3::new(x * 0.08, 0.3, 0.0),
                    Vec3::new(0.04, 0.17, 0.025),
                    skin,
                );
            }
        }
        _ => {
            for x in [-1.0f32, 1.0] {
                m.blob(
                    head + Vec3::new(x * 0.14, 0.15, 0.0),
                    Vec3::splat(0.06),
                    false,
                    0.0,
                    3,
                    skin,
                );
            }
        }
    }
    m
}

/// Three turbine blades turning about x, unit long, for spinning in place.
pub fn rotor() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU;
        let (s, c) = a.sin_cos();
        let out = Vec3::new(0.0, c, s);
        let side = Vec3::new(0.0, -s, c);
        m.thin_tri(side * 0.09, out, -side * 0.06, Rgb(0xF2F2EE));
        m.thin_tri(-side * 0.04, out, side * 0.02 + out * 0.2, Rgb(0xE6E6E2));
    }
    m.blob(Vec3::ZERO, Vec3::splat(0.05), false, 0.0, 2, Rgb(0xF2F2EE));
    m
}

/// A dolphin, nose toward +z, for leaps out of the sea.
pub fn dolphin() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::ZERO,
        Vec3::new(0.38, 0.4, 1.3),
        false,
        0.05,
        9,
        Rgb(0x6C8796),
    );
    m.thin_tri(
        Vec3::new(0.0, 0.3, 0.2),
        Vec3::new(0.0, 0.3, -0.35),
        Vec3::new(0.0, 0.75, -0.4),
        Rgb(0x5A7484),
    );
    m.thin_tri(
        Vec3::new(0.0, 0.0, -1.1),
        Vec3::new(0.5, 0.05, -1.65),
        Vec3::new(-0.5, 0.05, -1.65),
        Rgb(0x5A7484),
    );
    m
}

/// A fairground wheel, unit radius in the XY plane, turning about z: a double rim, spokes
/// and a hub, with bulbs round the rim that light up at night.
pub fn ferris_wheel() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let steel = Rgb(0xF2F2EE);
    let n = 20;
    for z in [-0.12f32, 0.12] {
        for i in 0..n {
            let a0 = i as f32 / n as f32 * TAU;
            let a1 = (i + 1) as f32 / n as f32 * TAU;
            let p = |a: f32, r: f32| Vec3::new(a.cos() * r, a.sin() * r, z);
            m.quad(p(a0, 1.0), p(a1, 1.0), p(a1, 0.96), p(a0, 0.96), steel);
            m.quad(p(a0, 0.96), p(a1, 0.96), p(a1, 1.0), p(a0, 1.0), steel);
        }
        for i in 0..10 {
            let a = i as f32 / 10.0 * TAU;
            let side = Vec3::new(-a.sin(), a.cos(), 0.0) * 0.012;
            let tip = Vec3::new(a.cos(), a.sin(), z);
            m.thin_tri(
                Vec3::new(0.0, 0.0, z) - side,
                Vec3::new(0.0, 0.0, z) + side,
                tip,
                steel,
            );
        }
    }
    m.frustum(8, -0.2, 0.06, 0.2, 0.06, Rgb(0xD8423A), Rgb(0xD8423A));
    let bulbs = [Rgb(0xFFE07A), Rgb(0xFF9AC8), Rgb(0x9AE0FF)];
    let from = m.verts.len();
    for i in 0..n {
        let a = (i as f32 + 0.5) / n as f32 * TAU;
        let c = Vec3::new(a.cos(), a.sin(), 0.14) * Vec3::new(1.0, 1.0, 1.0);
        m.blob(c, Vec3::splat(0.02), false, 0.0, i, bulbs[i as usize % 3]);
    }
    for v in &mut m.verts[from..] {
        v.normal[3] = -64;
    }
    m
}

/// A fairground gondola, hanging from its pin at the origin: a little open car with a
/// striped roof. White, for tinting.
pub fn gondola() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let w = Rgb(0xFFFFFF);
    m.cube(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.9, 0.5, 0.7), w);
    m.frustum(6, -1.0, 1.1, -0.6, 0.0, w, w);
    m.cube(
        Vec3::new(0.0, -0.8, 0.0),
        Vec3::new(0.05, 0.4, 0.05),
        Rgb(0x8C949C),
    );
    m
}

/// A rope swing hanging from the origin: two ropes and a plank seat, unit long.
pub fn swing_seat() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let rope = Rgb(0xC9B38A);
    for x in [-0.08f32, 0.08] {
        m.thin_tri(
            Vec3::new(x - 0.004, 0.0, 0.0),
            Vec3::new(x + 0.004, 0.0, 0.0),
            Vec3::new(x, -1.0, 0.0),
            rope,
        );
    }
    m.cube(
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.11, 0.012, 0.04),
        Rgb(0x8B5E3C),
    );
    m
}

/// A little silver fish, nose toward +z, about 0.6 m long.
pub fn fish() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::ZERO,
        Vec3::new(0.08, 0.12, 0.3),
        false,
        0.05,
        4,
        Rgb(0xB9CCD6),
    );
    m.thin_tri(
        Vec3::new(0.0, 0.0, -0.25),
        Vec3::new(0.0, 0.14, -0.42),
        Vec3::new(0.0, -0.14, -0.42),
        Rgb(0x8FA8B6),
    );
    m.thin_tri(
        Vec3::new(0.0, 0.1, 0.05),
        Vec3::new(0.0, 0.1, -0.12),
        Vec3::new(0.0, 0.2, -0.1),
        Rgb(0x8FA8B6),
    );
    m
}

/// A whale's back, nose toward +z, about 16 m long: a long rounded body with a pale
/// belly and broad tail flukes.
pub fn whale() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let back = Rgb(0x4E6170);
    m.blob(Vec3::ZERO, Vec3::new(2.0, 1.8, 6.5), true, 0.04, 21, back);
    m.blob(
        Vec3::new(0.0, -0.6, 1.0),
        Vec3::new(1.7, 1.1, 5.0),
        false,
        0.04,
        22,
        Rgb(0xC9D2D6),
    );
    m.blob(
        Vec3::new(0.0, 0.0, -6.5),
        Vec3::new(0.9, 0.8, 2.5),
        false,
        0.05,
        23,
        back,
    );
    for x in [-1.0f32, 1.0] {
        m.thin_tri(
            Vec3::new(0.0, 0.0, -8.3),
            Vec3::new(x * 3.4, 0.1, -9.8),
            Vec3::new(x * 1.0, 0.0, -9.0),
            back,
        );
        m.thin_tri(
            Vec3::new(x * 1.6, -0.5, 2.0),
            Vec3::new(x * 3.8, -1.2, -0.6),
            Vec3::new(x * 1.6, -0.6, 0.2),
            back,
        );
    }
    m
}

/// A sea turtle, nose toward +z, about 1.2 m long: a domed shell and four flippers.
pub fn turtle() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::ZERO,
        Vec3::new(0.5, 0.2, 0.6),
        false,
        0.08,
        31,
        Rgb(0x6E7F3E),
    );
    m.blob(
        Vec3::new(0.0, 0.02, 0.62),
        Vec3::new(0.14, 0.12, 0.18),
        false,
        0.05,
        32,
        Rgb(0x9DAA6E),
    );
    for x in [-1.0f32, 1.0] {
        for (z, len) in [(0.3, 0.6), (-0.4, 0.3)] {
            m.thin_tri(
                Vec3::new(x * 0.4, 0.0, z + 0.1),
                Vec3::new(x * (0.4 + len), -0.02, z - 0.15),
                Vec3::new(x * 0.4, 0.0, z - 0.1),
                Rgb(0x8C9A5A),
            );
        }
    }
    m
}

/// A seal lying on its belly, nose toward +z, about 1.8 m long.
pub fn seal() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = Rgb(0x8D8C88);
    m.blob(
        Vec3::new(0.0, 0.32, 0.0),
        Vec3::new(0.38, 0.3, 0.8),
        false,
        0.06,
        41,
        c,
    );
    m.blob(
        Vec3::new(0.0, 0.5, 0.75),
        Vec3::new(0.2, 0.2, 0.24),
        false,
        0.04,
        42,
        c,
    );
    m.blob(
        Vec3::new(0.0, 0.47, 0.98),
        Vec3::new(0.08, 0.07, 0.06),
        false,
        0.0,
        43,
        Rgb(0x3A3836),
    );
    for x in [-1.0f32, 1.0] {
        m.thin_tri(
            Vec3::new(x * 0.1, 0.1, -0.7),
            Vec3::new(x * 0.4, 0.05, -1.05),
            Vec3::new(x * 0.05, 0.1, -0.95),
            c.mix(Rgb(0x000000), 0.2),
        );
    }
    m
}

/// A duck, nose toward +z, sitting on the water at the origin. Scaled down, a duckling.
pub fn duck() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::new(0.0, 0.1, 0.0),
        Vec3::new(0.16, 0.12, 0.26),
        false,
        0.05,
        51,
        Rgb(0x8A6B4E),
    );
    m.blob(
        Vec3::new(0.0, 0.3, 0.2),
        Vec3::new(0.09, 0.09, 0.1),
        false,
        0.03,
        52,
        Rgb(0x2F6B45),
    );
    m.tri(
        Vec3::new(-0.04, 0.28, 0.28),
        Vec3::new(0.04, 0.28, 0.28),
        Vec3::new(0.0, 0.27, 0.38),
        Rgb(0xF2B33D),
    );
    m.thin_tri(
        Vec3::new(0.0, 0.14, -0.2),
        Vec3::new(0.0, 0.24, -0.32),
        Vec3::new(0.0, 0.12, -0.3),
        Rgb(0x6E5640),
    );
    m
}

/// A lily pad, unit radius, with a notch, flat on the water. Variant 1 has a flower.
pub fn lily(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let green = [Rgb(0x5B9A44), Rgb(0x6AAA4C)][v as usize % 2];
    let sides = 9;
    for i in 0..sides {
        let a0 = 0.35 + i as f32 / sides as f32 * (TAU - 0.5);
        let a1 = 0.35 + (i + 1) as f32 / sides as f32 * (TAU - 0.5);
        m.tri(
            Vec3::new(0.0, 0.03, 0.0),
            Vec3::new(a1.cos(), 0.03, a1.sin()),
            Vec3::new(a0.cos(), 0.03, a0.sin()),
            green,
        );
    }
    if v == 1 {
        for k in 0..5 {
            let a = k as f32 / 5.0 * TAU;
            let tip = Vec3::new(a.cos() * 0.4, 0.35, a.sin() * 0.4);
            let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.12;
            m.thin_tri(
                Vec3::new(0.0, 0.08, 0.0) - side,
                tip,
                Vec3::new(0.0, 0.08, 0.0) + side,
                Rgb(0xF5A9C9),
            );
        }
        m.blob(
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::splat(0.08),
            false,
            0.0,
            61,
            Rgb(0xFFE07A),
        );
    }
    m
}

/// A harbor buoy, about 2 m tall above the water: a fat float and a cage with a lamp.
pub fn buoy(v: u32) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let c = [Rgb(0xD8423A), Rgb(0x3E9A5A), Rgb(0xF2C14E)][v as usize % 3];
    m.frustum(8, -0.4, 0.7, 0.5, 0.6, c, c);
    m.frustum(4, 0.5, 0.35, 1.8, 0.1, c, c);
    let from = m.verts.len();
    m.blob(
        Vec3::new(0.0, 1.9, 0.0),
        Vec3::splat(0.12),
        false,
        0.0,
        71,
        Rgb(0xFFF2C8),
    );
    m.glow_from(from);
    m
}

/// A lighthouse beam: a fan of light along +x from the lamp, unit long and wide, fading
/// out to its end. Drawn glowing.
pub fn lighthouse_beam() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let end = Vec3::X;
    for k in 0..3 {
        let a = k as f32 / 3.0 * std::f32::consts::PI;
        let side = Vec3::new(0.0, a.cos(), a.sin());
        let n = m.verts.len();
        m.tri(Vec3::ZERO, end + side, end - side, Rgb(0xFFF2C8));
        m.verts[n].color[3] = 230;
        m.verts[n + 1].color[3] = 0;
        m.verts[n + 2].color[3] = 0;
    }
    m
}

/// Normal w that makes a vertex sway with the wind by its height above the mesh origin.
pub const SWAY: i8 = 100;

/// The cheapest stand-in for a far mesh, for whole groups of tiles seen from far off: a
/// three sided spire the size and top color of the original.
pub fn spire_of(far: &MeshBuilder) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &far.verts {
        lo = lo.min(Vec3::from_array(v.pos));
        hi = hi.max(Vec3::from_array(v.pos));
    }
    if far.verts.is_empty() {
        return m;
    }
    let mid = (lo.y + hi.y) * 0.5;
    let (mut sum, mut n) = ([0u32; 3], 0);
    for v in far.verts.iter().filter(|v| v.pos[1] >= mid) {
        for (s, c) in sum.iter_mut().zip(v.color) {
            *s += c as u32;
        }
        n += 1;
    }
    let c = sum.map(|s| s / n.max(1));
    let color = Rgb((c[0] << 16) | (c[1] << 8) | c[2]);
    let r = (hi.x - lo.x).max(hi.z - lo.z) * 0.55;
    let base = lo.y + (hi.y - lo.y) * 0.2;
    let top = Vec3::new(0.0, hi.y, 0.0);
    let ring: Vec<Vec3> = (0..3)
        .map(|i| {
            let a = i as f32 / 3.0 * TAU;
            Vec3::new(a.cos() * r, base, a.sin() * r)
        })
        .collect();
    for i in 0..3 {
        m.tri(ring[(i + 1) % 3], ring[i], top, color);
    }
    m
}

/// A far, cheap stand-in for a prop with the same size and color, or none to always use
/// the full mesh.
pub fn prop_far(kind: PropKind, v: u32) -> Option<MeshBuilder> {
    let mut m = MeshBuilder::new();
    match kind {
        PropKind::RoundTree => {
            // Far away the trunk is hidden by the canopy anyway.
            octahedron(
                &mut m,
                Vec3::new(0.3, 4.3, 0.15),
                Vec3::new(2.8, 2.6, 2.8),
                LEAF[v as usize],
            );
        }
        PropKind::Cloud => {
            let puffs = 3 + v;
            for i in 0..puffs {
                let t = i as f32 / (puffs - 1) as f32;
                let r = 8.0 + (1.0 - (t - 0.5).abs() * 2.0) * 5.0;
                let c = Vec3::new((t - 0.5) * 22.0, r * 0.3, 0.0);
                octahedron(&mut m, c, Vec3::new(r, r * 0.7, r * 0.85), CLOUD);
            }
        }
        PropKind::PineTree => {
            let c = PINE[v as usize];
            let top = 1.6 + (2 + v) as f32 * 1.7 + 2.6;
            m.frustum(5, 1.0, 2.3, top, 0.0, c, c);
        }
        _ => return None,
    }
    Some(m)
}

/// Eight faces around a center, stretched by `r`.
fn octahedron(m: &mut MeshBuilder, c: Vec3, r: Vec3, color: Rgb) {
    let (x, y, z) = (Vec3::X * r.x, Vec3::Y * r.y, Vec3::Z * r.z);
    for (a, b) in [(x, z), (z, -x), (-x, -z), (-z, x)] {
        m.tri(c + y, c + b, c + a, color);
        m.tri(c - y, c + a, c + b, color.mix(Rgb(0x000000), 0.08));
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
    let tiers = spire_tiers(v);
    let (cap, body) = tiers.split_last().expect("spire has tiers");
    for (i, t) in body.iter().enumerate() {
        let c = SAND[(i + v as usize) % 3];
        let mut tier = MeshBuilder::new();
        tier.frustum(6, 0.0, t.r0, t.height, t.r1, c, c);
        m.append(
            &tier,
            Mat4::from_translation(t.base) * Mat4::from_rotation_y(t.spin),
        );
    }
    let mut top = MeshBuilder::new();
    top.frustum(6, 0.0, cap.r0, cap.height, cap.r1, GRASS[0], GRASS[2]);
    m.append(&top, Mat4::from_translation(cap.base));
    let center = Vec3::new(cap.base.x, cap.top(), cap.base.z);
    m.append(
        &round_tree(v % 3),
        Mat4::from_translation(center + spire_tree_offset(cap.r1)),
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

/// A lump of spray, for the foot of falls.
pub fn spray() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(Vec3::ZERO, Vec3::ONE, false, 0.18, 7, CLOUD);
    m
}

/// A unit ring in the XZ plane for landing markers. Alpha fades to both edges.
pub fn ring() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let sides = 40;
    let radii = [0.78, 1.0, 1.22];
    let at = |i: u32, r: f32| {
        let a = i as f32 / sides as f32 * TAU;
        Vec3::new(a.cos() * r, 0.0, a.sin() * r)
    };
    for i in 0..sides {
        for band in 0..2 {
            let (r0, r1) = (radii[band], radii[band + 1]);
            m.quad(
                at(i, r0),
                at(i, r1),
                at(i + 1, r1),
                at(i + 1, r0),
                Rgb(0xFFFFFF),
            );
            let n = m.verts.len();
            // Quad corners in order: r0, r1, r1, then r0, r1, r0 per triangle pair.
            for (v, on_mid) in m.verts[n - 6..].iter_mut().zip([
                band == 1,
                band == 0,
                band == 0,
                band == 1,
                band == 0,
                band == 1,
            ]) {
                v.color[3] = if on_mid { 255 } else { 0 };
            }
        }
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

/// An elephant, facing +z, feet at the origin, about 3 m at the shoulder: a big round
/// body, wide ears, tusks and a trunk hanging to the ground.
pub fn elephant() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (hide, ear, ivory, dark) = (Rgb(0x9C9AA0), Rgb(0x8E8792), Rgb(0xF4EEDC), Rgb(0x2E2A2A));
    m.blob(
        Vec3::new(0.0, 2.05, 0.0),
        Vec3::new(0.95, 0.9, 1.45),
        false,
        0.06,
        31,
        hide,
    );
    legs(&mut m, 0.52, 0.85, 1.45, 0.25, hide);
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        m.cube(
            Vec3::new(x * 0.52, 0.05, z * 0.85 + 0.08),
            Vec3::new(0.27, 0.05, 0.22),
            ear,
        );
    }
    m.blob(
        Vec3::new(0.0, 2.55, 1.45),
        Vec3::new(0.62, 0.66, 0.58),
        false,
        0.04,
        32,
        hide,
    );
    for x in [-1.0f32, 1.0] {
        // Ears flat against the head, flaring out and back, seen from both sides.
        let root = Vec3::new(x * 0.45, 2.75, 1.2);
        let ear_pts = [
            root,
            root + Vec3::new(x * 0.55, 0.3, -0.45),
            root + Vec3::new(x * 0.6, -0.75, -0.5),
            root + Vec3::new(x * 0.1, -0.85, -0.05),
        ];
        m.quad(ear_pts[0], ear_pts[1], ear_pts[2], ear_pts[3], ear);
        m.quad(ear_pts[3], ear_pts[2], ear_pts[1], ear_pts[0], ear);
        m.cube(
            Vec3::new(x * 0.36, 2.72, 1.88),
            Vec3::new(0.05, 0.05, 0.04),
            dark,
        );
        tube(
            &mut m,
            4,
            Vec3::new(x * 0.22, 2.05, 1.85),
            0.08,
            Vec3::new(x * 0.26, 1.7, 2.45),
            0.02,
            ivory,
        );
    }
    // The trunk, in three tapering pieces, curling forward at the tip.
    let trunk = [
        (Vec3::new(0.0, 2.35, 1.95), 0.22),
        (Vec3::new(0.0, 1.5, 2.2), 0.15),
        (Vec3::new(0.0, 0.7, 2.25), 0.11),
        (Vec3::new(0.0, 0.35, 2.5), 0.08),
    ];
    for w in trunk.windows(2) {
        tube(&mut m, 5, w[0].0, w[0].1, w[1].0, w[1].1, hide);
    }
    tube(
        &mut m,
        3,
        Vec3::new(0.0, 2.2, -1.35),
        0.05,
        Vec3::new(0.0, 1.3, -1.5),
        0.03,
        hide,
    );
    m
}

/// A giraffe, facing +z, feet at the origin, about 5 m to the horns: patches on a
/// golden coat, a long neck and little horns.
pub fn giraffe() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (coat, patch, dark) = (Rgb(0xF0C070), Rgb(0x9A5A2A), Rgb(0x3B2618));
    m.cube(Vec3::new(0.0, 2.3, 0.05), Vec3::new(0.38, 0.42, 0.85), coat);
    for (k, (x, y, z)) in [
        (1.0f32, 2.45f32, 0.5f32),
        (1.0, 2.1, -0.3),
        (1.0, 2.55, -0.55),
        (-1.0, 2.4, 0.2),
        (-1.0, 2.1, -0.5),
        (-1.0, 2.55, 0.65),
    ]
    .into_iter()
    .enumerate()
    {
        let s = 0.13 + (k % 3) as f32 * 0.02;
        m.cube(
            Vec3::new(x * 0.39, y, z),
            Vec3::new(0.01, s, s * 1.2),
            patch,
        );
        m.cube(
            Vec3::new(x * 0.15, 2.73, z * 0.8),
            Vec3::new(s, 0.01, s),
            patch,
        );
    }
    // Front legs a little longer, so the back slopes down to the tail.
    for (x, z, h) in [
        (1.0f32, 0.6f32, 2.0f32),
        (-1.0, 0.6, 2.0),
        (1.0, -0.55, 1.9),
        (-1.0, -0.55, 1.9),
    ] {
        m.cube(
            Vec3::new(x * 0.24, h * 0.5, z),
            Vec3::new(0.07, h * 0.5, 0.07),
            coat,
        );
        m.cube(
            Vec3::new(x * 0.24, 0.06, z),
            Vec3::new(0.08, 0.06, 0.08),
            dark,
        );
    }
    let (base, top) = (Vec3::new(0.0, 2.55, 0.75), Vec3::new(0.0, 4.55, 1.35));
    tube(&mut m, 6, base, 0.26, top, 0.13, coat);
    for k in 0..5 {
        let p = base.lerp(top, 0.15 + k as f32 * 0.17);
        let x = if k % 2 == 0 { 1.0 } else { -1.0 };
        let r = 0.24 - k as f32 * 0.022;
        m.cube(
            p + Vec3::new(x * r, 0.0, 0.02),
            Vec3::new(0.01, 0.1, 0.1),
            patch,
        );
    }
    // A short mane down the back of the neck.
    tube(
        &mut m,
        3,
        base + Vec3::new(0.0, 0.2, -0.2),
        0.06,
        top + Vec3::new(0.0, 0.05, -0.12),
        0.04,
        patch,
    );
    m.cube(Vec3::new(0.0, 4.68, 1.6), Vec3::new(0.13, 0.15, 0.34), coat);
    m.cube(Vec3::new(0.0, 4.6, 1.92), Vec3::new(0.1, 0.1, 0.06), patch);
    for x in [-1.0f32, 1.0] {
        tube(
            &mut m,
            3,
            Vec3::new(x * 0.07, 4.8, 1.45),
            0.035,
            Vec3::new(x * 0.08, 5.08, 1.4),
            0.03,
            patch,
        );
        m.cube(
            Vec3::new(x * 0.08, 5.1, 1.4),
            Vec3::new(0.04, 0.04, 0.04),
            dark,
        );
        m.cube(
            Vec3::new(x * 0.2, 4.82, 1.42),
            Vec3::new(0.09, 0.03, 0.04),
            coat,
        );
        m.cube(
            Vec3::new(x * 0.135, 4.75, 1.66),
            Vec3::new(0.01, 0.03, 0.03),
            dark,
        );
    }
    tube(
        &mut m,
        3,
        Vec3::new(0.0, 2.6, -0.8),
        0.04,
        Vec3::new(0.0, 1.8, -0.95),
        0.02,
        coat,
    );
    m.cube(
        Vec3::new(0.0, 1.75, -0.96),
        Vec3::new(0.05, 0.12, 0.05),
        dark,
    );
    m
}

/// A zebra, facing +z, feet at the origin: a stocky horse in black and white stripes.
pub fn zebra() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (white, black) = (Rgb(0xF6F3EC), Rgb(0x2A2828));
    m.cube(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.28, 0.3, 0.7), white);
    // Bands round the body, slightly proud of it.
    for k in 0..7 {
        let z = -0.6 + k as f32 * 0.2;
        m.cube(
            Vec3::new(0.0, 1.2, z),
            Vec3::new(0.285, 0.305, 0.035),
            black,
        );
    }
    let neck = Mat4::from_translation(Vec3::new(0.0, 1.55, 0.75)) * Mat4::from_rotation_x(-0.75);
    let mut n = MeshBuilder::new();
    n.cube(Vec3::ZERO, Vec3::new(0.14, 0.16, 0.38), white);
    for k in 0..3 {
        n.cube(
            Vec3::new(0.0, 0.0, -0.22 + k as f32 * 0.2),
            Vec3::new(0.145, 0.165, 0.03),
            black,
        );
    }
    n.cube(
        Vec3::new(0.0, 0.2, -0.02),
        Vec3::new(0.04, 0.08, 0.4),
        black,
    );
    m.append(&n, neck);
    m.cube(
        Vec3::new(0.0, 1.8, 1.12),
        Vec3::new(0.12, 0.13, 0.28),
        white,
    );
    m.cube(Vec3::new(0.0, 1.74, 1.38), Vec3::new(0.1, 0.1, 0.06), black);
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.08, 1.98, 0.95),
            Vec3::new(0.03, 0.08, 0.03),
            white,
        );
        m.cube(
            Vec3::new(x * 0.125, 1.85, 1.15),
            Vec3::new(0.01, 0.025, 0.025),
            black,
        );
    }
    let tail = Mat4::from_translation(Vec3::new(0.0, 1.3, -0.72)) * Mat4::from_rotation_x(0.3);
    let mut t = MeshBuilder::new();
    t.cube(
        Vec3::new(0.0, -0.25, 0.0),
        Vec3::new(0.03, 0.28, 0.03),
        white,
    );
    t.cube(
        Vec3::new(0.0, -0.55, 0.0),
        Vec3::new(0.05, 0.08, 0.05),
        black,
    );
    m.append(&t, tail);
    legs(&mut m, 0.18, 0.52, 0.9, 0.06, white);
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        for y in [0.35f32, 0.65] {
            m.cube(
                Vec3::new(x * 0.18, y, z * 0.52),
                Vec3::new(0.065, 0.03, 0.065),
                black,
            );
        }
        m.cube(
            Vec3::new(x * 0.18, 0.05, z * 0.52),
            Vec3::new(0.07, 0.05, 0.07),
            black,
        );
    }
    m
}

/// A lion, facing +z, feet at the origin: tawny, with a shaggy mane when `mane`, a little
/// smaller without.
pub fn lion(mane: bool) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (coat, pale, dark, nose) = (Rgb(0xD9A458), Rgb(0xF0D9A8), Rgb(0x8A4A22), Rgb(0x5A3A30));
    m.blob(
        Vec3::new(0.0, 0.88, 0.0),
        Vec3::new(0.32, 0.3, 0.78),
        false,
        0.04,
        41,
        coat,
    );
    legs(&mut m, 0.2, 0.5, 0.7, 0.075, coat);
    let head = Vec3::new(0.0, 1.12, 0.82);
    if mane {
        m.blob(
            head - Vec3::Z * 0.08,
            Vec3::new(0.42, 0.44, 0.3),
            false,
            0.1,
            42,
            dark,
        );
    }
    m.blob(head, Vec3::new(0.21, 0.22, 0.24), false, 0.03, 43, coat);
    m.cube(
        head + Vec3::new(0.0, -0.06, 0.2),
        Vec3::new(0.11, 0.08, 0.08),
        pale,
    );
    m.cube(
        head + Vec3::new(0.0, -0.01, 0.28),
        Vec3::new(0.05, 0.03, 0.02),
        nose,
    );
    for x in [-1.0f32, 1.0] {
        m.cube(
            head + Vec3::new(x * 0.09, 0.07, 0.2),
            Vec3::new(0.025, 0.02, 0.01),
            nose,
        );
        m.cube(
            head + Vec3::new(x * 0.15, 0.2, -0.02),
            Vec3::new(0.05, 0.05, 0.02),
            coat,
        );
    }
    tube(
        &mut m,
        3,
        Vec3::new(0.0, 0.95, -0.7),
        0.035,
        Vec3::new(0.0, 0.35, -1.15),
        0.025,
        coat,
    );
    m.blob(
        Vec3::new(0.0, 0.33, -1.17),
        Vec3::splat(0.07),
        false,
        0.02,
        44,
        dark,
    );
    if !mane {
        m.recolor(coat, Rgb(0xE0B06A));
    }
    m
}

/// A penguin, facing +z, feet at the origin, about 0.8 m tall.
pub fn penguin() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (black, white, orange) = (Rgb(0x2A2C34), Rgb(0xF8F6F0), Rgb(0xF2A03A));
    m.blob(
        Vec3::new(0.0, 0.38, 0.0),
        Vec3::new(0.19, 0.33, 0.18),
        false,
        0.02,
        61,
        black,
    );
    m.blob(
        Vec3::new(0.0, 0.35, 0.07),
        Vec3::new(0.15, 0.27, 0.13),
        false,
        0.01,
        62,
        white,
    );
    m.blob(
        Vec3::new(0.0, 0.74, 0.02),
        Vec3::splat(0.12),
        false,
        0.01,
        63,
        black,
    );
    m.tri(
        Vec3::new(-0.035, 0.75, 0.12),
        Vec3::new(0.035, 0.75, 0.12),
        Vec3::new(0.0, 0.72, 0.24),
        orange,
    );
    m.tri(
        Vec3::new(0.035, 0.7, 0.12),
        Vec3::new(-0.035, 0.7, 0.12),
        Vec3::new(0.0, 0.72, 0.24),
        orange,
    );
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.07, 0.02, 0.08),
            Vec3::new(0.05, 0.02, 0.08),
            orange,
        );
        m.cube(
            Vec3::new(x * 0.06, 0.79, 0.11),
            Vec3::new(0.018, 0.018, 0.01),
            white,
        );
        m.thin_tri(
            Vec3::new(x * 0.19, 0.55, 0.0),
            Vec3::new(x * 0.19, 0.55, -0.08),
            Vec3::new(x * 0.27, 0.2, -0.04),
            black,
        );
    }
    m
}

/// A roller coaster car, nose toward +z, its wheels on the rail at the origin: a rounded
/// tub with two rows of seats and lap bars. White, for tinting.
pub fn coaster_car() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (w, seat, steel) = (Rgb(0xFFFFFF), Rgb(0x2E3440), Rgb(0x8C949C));
    m.loft(
        -1.1,
        (0.75, 0.35, 0.65),
        1.0,
        (0.72, 0.35, 0.65),
        [w, w, steel, w],
    );
    m.loft(
        1.0,
        (0.72, 0.35, 0.65),
        1.35,
        (0.5, 0.2, 0.55),
        [w, w, steel, w],
    );
    for z in [-0.55f32, 0.35] {
        m.cube(Vec3::new(0.0, 1.0, z), Vec3::new(0.62, 0.05, 0.3), seat);
        m.cube(
            Vec3::new(0.0, 1.35, z - 0.3),
            Vec3::new(0.62, 0.35, 0.05),
            seat,
        );
        m.cube(
            Vec3::new(0.0, 1.38, z + 0.35),
            Vec3::new(0.55, 0.03, 0.03),
            steel,
        );
    }
    for (x, z) in [(1.0f32, 0.8f32), (-1.0, 0.8), (1.0, -0.8), (-1.0, -0.8)] {
        m.cube(
            Vec3::new(x * 0.55, 0.2, z),
            Vec3::new(0.08, 0.15, 0.15),
            steel,
        );
    }
    m
}

/// A carousel horse on its brass pole, leaping toward +z, its belly at the origin. White
/// with a gold pole and saddle, for pastel tinting.
pub fn carousel_horse() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (w, gold, mane) = (Rgb(0xFFFFFF), Rgb(0xF5C64E), Rgb(0xF2D7A0));
    m.cube(Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.2, 0.22, 0.55), w);
    let neck = Mat4::from_translation(Vec3::new(0.0, 0.55, 0.55)) * Mat4::from_rotation_x(-0.9);
    let mut n = MeshBuilder::new();
    n.cube(Vec3::ZERO, Vec3::new(0.12, 0.13, 0.3), w);
    n.cube(
        Vec3::new(0.0, 0.15, -0.04),
        Vec3::new(0.04, 0.06, 0.3),
        mane,
    );
    m.append(&n, neck);
    m.cube(Vec3::new(0.0, 0.82, 0.82), Vec3::new(0.1, 0.11, 0.22), w);
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.06, 0.98, 0.7),
            Vec3::new(0.025, 0.06, 0.025),
            w,
        );
        // Front legs tucked up mid leap, back legs reaching.
        tube(
            &mut m,
            3,
            Vec3::new(x * 0.13, 0.1, 0.45),
            0.05,
            Vec3::new(x * 0.13, -0.05, 0.85),
            0.04,
            w,
        );
        tube(
            &mut m,
            3,
            Vec3::new(x * 0.13, 0.1, -0.45),
            0.05,
            Vec3::new(x * 0.13, -0.4, -0.75),
            0.04,
            w,
        );
    }
    m.cube(
        Vec3::new(0.0, 0.5, -0.05),
        Vec3::new(0.22, 0.04, 0.22),
        gold,
    );
    tube(
        &mut m,
        3,
        Vec3::new(0.0, 0.35, -0.55),
        0.05,
        Vec3::new(0.0, -0.1, -0.85),
        0.03,
        mane,
    );
    m.frustum(5, -1.6, 0.05, 5.6, 0.05, gold, gold);
    m
}

/// A drop tower's ride ring round its tower at the origin: a band of outward-facing seats
/// with lights that glow at night.
pub fn drop_ring() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (band, seat) = (Rgb(0xE0564A), Rgb(0x2E3440));
    m.frustum(12, -0.3, 3.4, 0.6, 3.4, band, Rgb(0xF6F1E7));
    m.frustum(12, -0.6, 2.4, -0.3, 3.4, band, band);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let at = Vec3::new(a.cos(), 0.0, a.sin()) * 3.75;
        let q = Quat::from_rotation_y(-a + PI * 0.5);
        let mut s = MeshBuilder::new();
        s.cube(Vec3::new(0.0, -0.3, 0.0), Vec3::new(0.3, 0.05, 0.3), seat);
        s.cube(Vec3::new(0.0, 0.2, -0.3), Vec3::new(0.3, 0.5, 0.05), seat);
        s.cube(
            Vec3::new(0.0, 0.75, -0.1),
            Vec3::new(0.32, 0.06, 0.25),
            band,
        );
        m.append(&s, Mat4::from_rotation_translation(q, at));
    }
    let from = m.verts.len();
    for k in 0..16 {
        let a = (k as f32 + 0.5) / 16.0 * TAU;
        m.blob(
            Vec3::new(a.cos() * 3.42, 0.15, a.sin() * 3.42),
            Vec3::splat(0.08),
            false,
            0.0,
            k,
            [Rgb(0xFFE07A), Rgb(0x9AE0FF)][k as usize % 2],
        );
    }
    for v in &mut m.verts[from..] {
        v.normal[3] = -64;
    }
    m
}

/// A rubber ring for floating on, round the origin on the water. White, for tinting.
pub fn swim_ring() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let w = Rgb(0xFFFFFF);
    let (n, k) = (10, 4);
    let at = |i: u32, j: u32| {
        let a = i as f32 / n as f32 * TAU;
        let b = j as f32 / k as f32 * TAU;
        let r = 0.55 + b.cos() * 0.2;
        Vec3::new(a.cos() * r, b.sin() * 0.2, a.sin() * r)
    };
    for i in 0..n {
        for j in 0..k {
            m.quad(at(i, j), at(i, j + 1), at(i + 1, j + 1), at(i + 1, j), w);
        }
    }
    m
}

/// A water park's slides, in the park's frame: smooth open troughs spiralling down from
/// the tower, a little water running in each.
pub fn water_slides() -> MeshBuilder {
    use zawnabout_sim::landmark::{SLIDES, slide};
    let mut m = MeshBuilder::new();
    let paint = [Rgb(0xE0564A), Rgb(0x4A90C8), Rgb(0xE88AB0)];
    let water = Rgb(0xA8E6F6);
    let (n, sides, w) = (90, 6, 0.85);
    for s in 0..SLIDES {
        let c = paint[s as usize % paint.len()];
        let inner = c.mix(Rgb(0xFFFFFF), 0.25);
        let ring = |u: f32| {
            let at = slide(s, u);
            let ahead = (slide(s, (u + 0.005).min(1.0)) - slide(s, (u - 0.005).max(0.0)))
                .normalize_or(Vec3::Z);
            let right = ahead.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(ahead);
            (0..=sides)
                .map(|k| {
                    let a = (k as f32 / sides as f32 - 0.5) * 3.6;
                    at + right * a.sin() * w + up * (1.0 - a.cos()) * w
                })
                .collect::<Vec<_>>()
        };
        let mut prev = ring(0.0);
        for i in 1..=n {
            let next = ring(i as f32 / n as f32);
            for k in 0..sides {
                let (a, b, cc, d) = (prev[k], prev[k + 1], next[k + 1], next[k]);
                m.quad(a, b, cc, d, inner);
                m.quad(d, cc, b, a, c);
            }
            // A thin sheet of water down the middle.
            let lift = Vec3::Y * 0.03;
            let (a, b) = (
                prev[sides / 2 - 1].lerp(prev[sides / 2], 0.5),
                prev[sides / 2 + 1].lerp(prev[sides / 2], 0.5),
            );
            let (cc, d) = (
                next[sides / 2 + 1].lerp(next[sides / 2], 0.5),
                next[sides / 2 - 1].lerp(next[sides / 2], 0.5),
            );
            m.quad(a + lift, b + lift, cc + lift, d + lift, water);
            prev = next;
        }
    }
    m
}

/// A little farm tractor, nose toward +z, wheels on the ground: a green body and bonnet,
/// a cab with a roof, big rear wheels and small front ones, and a driver.
pub fn tractor() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (green, yellow, tyre, glass) = (Rgb(0x3E9A4E), Rgb(0xF2C14E), Rgb(0x2A2A2E), Rgb(0x9FC7DE));
    m.cube(Vec3::new(0.0, 1.1, 0.4), Vec3::new(0.55, 0.45, 1.3), green);
    m.cube(Vec3::new(0.0, 1.65, 1.05), Vec3::new(0.4, 0.15, 0.6), green);
    m.cube(Vec3::new(0.0, 2.0, -0.45), Vec3::new(0.6, 0.6, 0.55), glass);
    m.cube(
        Vec3::new(0.0, 2.65, -0.45),
        Vec3::new(0.7, 0.06, 0.65),
        green,
    );
    m.cube(
        Vec3::new(0.0, 1.35, -0.45),
        Vec3::new(0.25, 0.4, 0.2),
        Rgb(0xE88A5A),
    );
    tube(
        &mut m,
        4,
        Vec3::new(0.3, 1.7, 1.3),
        0.06,
        Vec3::new(0.3, 2.4, 1.3),
        0.06,
        tyre,
    );
    for x in [-1.0f32, 1.0] {
        let mut w = MeshBuilder::new();
        w.frustum(8, -0.3, 0.75, 0.3, 0.75, tyre, yellow);
        m.append(
            &w,
            Mat4::from_rotation_translation(
                Quat::from_rotation_z(PI * 0.5),
                Vec3::new(x * 0.85, 0.75, -0.6),
            ),
        );
        let mut f = MeshBuilder::new();
        f.frustum(8, -0.18, 0.42, 0.18, 0.42, tyre, yellow);
        m.append(
            &f,
            Mat4::from_rotation_translation(
                Quat::from_rotation_z(PI * 0.5),
                Vec3::new(x * 0.62, 0.42, 1.3),
            ),
        );
    }
    m
}

/// A crane trolley riding a boom.
pub fn trolley() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.cube(Vec3::ZERO, Vec3::new(2.4, 0.7, 2.0), Rgb(0x2E6FA8));
    m
}

/// A pair of crane cables hanging from the origin, unit long, to stretch to any drop.
pub fn cables() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.8, -0.5, 0.0),
            Vec3::new(0.05, 0.5, 0.05),
            Rgb(0x3A3A40),
        );
    }
    m
}

/// A shipping container, 12 m long along z, its base at the origin. White, for tinting.
pub fn container() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let w = Rgb(0xFFFFFF);
    m.cube(Vec3::new(0.0, 1.3, 0.0), Vec3::new(1.2, 1.3, 6.0), w);
    for z in [-4.0f32, -2.0, 0.0, 2.0, 4.0] {
        m.cube(
            Vec3::new(0.0, 1.3, z),
            Vec3::new(1.25, 1.25, 0.08),
            Rgb(0xE6E6E6),
        );
    }
    m
}

/// A bicycle, rolling toward +z, wheels on the ground: two thin wheels, a frame, handlebars
/// and a saddle. White, for tinting the frame.
pub fn bicycle() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (frame, tyre) = (Rgb(0xFFFFFF), Rgb(0x2A2A2E));
    for z in [-0.55f32, 0.55] {
        let n = 10;
        for k in 0..n {
            let (a, b) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
            let p = |a: f32| Vec3::new(0.0, 0.34 + a.sin() * 0.34, z + a.cos() * 0.34);
            m.quad(
                p(a) - Vec3::X * 0.03,
                p(b) - Vec3::X * 0.03,
                p(b) + Vec3::X * 0.03,
                p(a) + Vec3::X * 0.03,
                tyre,
            );
            m.quad(
                p(a) + Vec3::X * 0.03,
                p(b) + Vec3::X * 0.03,
                p(b) - Vec3::X * 0.03,
                p(a) - Vec3::X * 0.03,
                tyre,
            );
        }
    }
    for (a, b) in [
        (Vec3::new(0.0, 0.34, -0.55), Vec3::new(0.0, 0.85, -0.15)),
        (Vec3::new(0.0, 0.34, -0.55), Vec3::new(0.0, 0.4, 0.0)),
        (Vec3::new(0.0, 0.4, 0.0), Vec3::new(0.0, 0.85, -0.15)),
        (Vec3::new(0.0, 0.4, 0.0), Vec3::new(0.0, 0.9, 0.42)),
        (Vec3::new(0.0, 0.85, -0.15), Vec3::new(0.0, 0.9, 0.42)),
        (Vec3::new(0.0, 0.34, 0.55), Vec3::new(0.0, 1.0, 0.45)),
    ] {
        tube(&mut m, 3, a, 0.03, b, 0.03, frame);
    }
    m.cube(
        Vec3::new(0.0, 1.0, 0.45),
        Vec3::new(0.25, 0.025, 0.025),
        tyre,
    );
    m.cube(
        Vec3::new(0.0, 0.9, -0.17),
        Vec3::new(0.08, 0.03, 0.13),
        tyre,
    );
    m
}

/// A fishing rod held at the origin, reaching out toward +z and up, with its line
/// dropping 4.2 m to a red float.
pub fn fishing_rod() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (rod, line) = (Rgb(0x6B4A2E), Rgb(0xEDEDED));
    let tip = Vec3::new(0.0, 1.6, 2.6);
    tube(&mut m, 3, Vec3::ZERO, 0.03, tip, 0.012, rod);
    m.thin_tri(
        tip - Vec3::X * 0.01,
        tip + Vec3::X * 0.01,
        tip - Vec3::Y * 4.2,
        line,
    );
    m.blob(
        tip - Vec3::Y * 4.2,
        Vec3::splat(0.06),
        false,
        0.0,
        5,
        Rgb(0xE0564A),
    );
    m
}
