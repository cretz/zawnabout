//! Treasure to find and hoard, one crude model a kind, about a meter across so it reads
//! from the air once you are close. Gold parts are polished metal that mirrors the sky.
//! The base sits on y = 0.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec3};

use crate::mesh::{METAL, MeshBuilder, Rgb};

const GOLD: Rgb = Rgb(0xF5B62E);
const GOLD_DEEP: Rgb = Rgb(0xD9922A);
const SILVER: Rgb = Rgb(0xD8DCE4);
const BRASS: Rgb = Rgb(0xD8A84A);
const WOOD: Rgb = Rgb(0x9C6B4E);
const WOOD_DARK: Rgb = Rgb(0x7A5038);
const RUBY: Rgb = Rgb(0xE0405A);
const SAPPHIRE: Rgb = Rgb(0x4A7FE0);
const EMERALD: Rgb = Rgb(0x3EC07A);
const GLASS: Rgb = Rgb(0xBFE8F4);
const IVORY: Rgb = Rgb(0xF6EEDC);
const INK: Rgb = Rgb(0x3A3040);

/// Every kind with a model. `coins` is the plain pile.
pub const KINDS: &[&str] = &[
    "crown",
    "bell",
    "trumpet",
    "goblet",
    "ring",
    "compass",
    "spyglass",
    "figurehead",
    "chest",
    "lens",
    "chess",
    "lantern",
    "anchor",
    "harp",
    "egg",
    "map",
    "teapot",
    "music-box",
    "kite",
    "basket",
    "hat",
    "shell",
    "painting",
    "ribbon",
    "trophy",
    "pot",
    "pearl",
    "vase",
    "key",
    "book",
    "medal",
    "fossil",
    "flute",
    "toy-boat",
    "gem",
    "clock",
    "candle",
    "coins",
];

/// A shape in `metal`, marked to mirror the sky.
fn shiny(m: &mut MeshBuilder, f: impl FnOnce(&mut MeshBuilder)) {
    let from = m.verts.len();
    f(m);
    m.flag_from(from, METAL);
}

fn part(m: &mut MeshBuilder, sides: u32, y0: f32, r0: f32, y1: f32, r1: f32, c: Rgb) {
    m.frustum(sides, y0, r0, y1, r1, c, c);
}

fn tube(m: &mut MeshBuilder, sides: u32, a: Vec3, ra: f32, b: Vec3, rb: f32, c: Rgb) {
    let mut t = MeshBuilder::new();
    t.frustum(sides, 0.0, ra, (b - a).length(), rb, c, c);
    let rot = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or(Vec3::Y));
    m.append(&t, Mat4::from_rotation_translation(rot, a));
}

fn gem(m: &mut MeshBuilder, at: Vec3, r: f32, c: Rgb) {
    m.blob(at, Vec3::splat(r), false, 0.0, 1, c);
}

/// A flat ring of `sides` boxes round the y axis, as for a crown band or a bell lip.
fn hoop(m: &mut MeshBuilder, sides: u32, y: f32, r: f32, thick: f32, tall: f32, c: Rgb) {
    for i in 0..sides {
        let a = i as f32 / sides as f32 * TAU;
        let mut b = MeshBuilder::new();
        b.cube(
            Vec3::ZERO,
            Vec3::new(thick, tall, r * PI / sides as f32 + 0.01),
            c,
        );
        m.append(
            &b,
            Mat4::from_rotation_translation(
                Quat::from_rotation_y(-a),
                Vec3::new(a.cos() * r, y, a.sin() * r),
            ),
        );
    }
}

pub fn treasure(kind: &str) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    match kind {
        "crown" => {
            shiny(&mut m, |m| {
                part(m, 8, 0.0, 0.5, 0.35, 0.55, GOLD);
                for i in 0..5 {
                    let a = i as f32 / 5.0 * TAU;
                    let base = Vec3::new(a.cos() * 0.5, 0.35, a.sin() * 0.5);
                    tube(
                        m,
                        3,
                        base,
                        0.16,
                        base + Vec3::new(a.cos() * 0.1, 0.4, a.sin() * 0.1),
                        0.0,
                        GOLD,
                    );
                }
            });
            for (i, c) in [RUBY, SAPPHIRE, EMERALD, RUBY, SAPPHIRE]
                .into_iter()
                .enumerate()
            {
                let a = (i as f32 + 0.5) / 5.0 * TAU;
                gem(
                    &mut m,
                    Vec3::new(a.cos() * 0.54, 0.18, a.sin() * 0.54),
                    0.08,
                    c,
                );
            }
        }
        "bell" => shiny(&mut m, |m| {
            part(m, 10, 0.0, 0.6, 0.2, 0.55, BRASS);
            part(m, 10, 0.2, 0.55, 0.85, 0.38, BRASS);
            part(m, 10, 0.85, 0.38, 1.0, 0.0, BRASS);
            tube(
                m,
                4,
                Vec3::new(0.0, 1.0, 0.0),
                0.08,
                Vec3::new(0.0, 1.2, 0.0),
                0.08,
                BRASS,
            );
        }),
        "trumpet" => {
            shiny(&mut m, |m| {
                tube(
                    m,
                    6,
                    Vec3::new(-0.7, 0.12, 0.0),
                    0.05,
                    Vec3::new(0.4, 0.12, 0.0),
                    0.07,
                    GOLD,
                );
                tube(
                    m,
                    8,
                    Vec3::new(0.4, 0.12, 0.0),
                    0.07,
                    Vec3::new(0.8, 0.25, 0.0),
                    0.3,
                    GOLD,
                );
                tube(
                    m,
                    6,
                    Vec3::new(-0.3, 0.12, 0.0),
                    0.04,
                    Vec3::new(-0.3, 0.32, 0.0),
                    0.04,
                    GOLD,
                );
            });
        }
        "goblet" => {
            shiny(&mut m, |m| {
                part(m, 8, 0.0, 0.3, 0.06, 0.3, GOLD);
                part(m, 6, 0.06, 0.06, 0.5, 0.06, GOLD);
                part(m, 8, 0.5, 0.08, 0.8, 0.32, GOLD);
                part(m, 8, 0.8, 0.32, 0.95, 0.35, GOLD_DEEP);
            });
            gem(&mut m, Vec3::new(0.0, 0.66, 0.24), 0.07, RUBY);
        }
        "ring" => {
            shiny(&mut m, |m| {
                let mut r = MeshBuilder::new();
                hoop(&mut r, 10, 0.0, 0.45, 0.07, 0.12, GOLD);
                m.append(
                    &r,
                    Mat4::from_rotation_translation(
                        Quat::from_rotation_x(PI / 2.0),
                        Vec3::new(0.0, 0.52, 0.0),
                    ),
                );
            });
            gem(&mut m, Vec3::new(0.0, 1.05, 0.0), 0.16, SAPPHIRE);
        }
        "compass" => {
            shiny(&mut m, |m| part(m, 10, 0.0, 0.55, 0.18, 0.55, BRASS));
            part(&mut m, 10, 0.18, 0.5, 0.19, 0.5, IVORY);
            m.tri(
                Vec3::new(0.0, 0.2, -0.4),
                Vec3::new(-0.08, 0.2, 0.0),
                Vec3::new(0.08, 0.2, 0.0),
                RUBY,
            );
            m.tri(
                Vec3::new(0.0, 0.2, 0.4),
                Vec3::new(0.08, 0.2, 0.0),
                Vec3::new(-0.08, 0.2, 0.0),
                INK,
            );
        }
        "spyglass" => {
            shiny(&mut m, |m| {
                tube(
                    m,
                    6,
                    Vec3::new(-0.8, 0.14, 0.0),
                    0.1,
                    Vec3::new(-0.2, 0.14, 0.0),
                    0.11,
                    BRASS,
                );
                tube(
                    m,
                    6,
                    Vec3::new(-0.2, 0.15, 0.0),
                    0.13,
                    Vec3::new(0.3, 0.15, 0.0),
                    0.14,
                    BRASS,
                );
                tube(
                    m,
                    6,
                    Vec3::new(0.3, 0.17, 0.0),
                    0.16,
                    Vec3::new(0.8, 0.17, 0.0),
                    0.18,
                    BRASS,
                );
            });
            tube(
                &mut m,
                6,
                Vec3::new(-0.15, 0.15, 0.0),
                0.135,
                Vec3::new(0.2, 0.15, 0.0),
                0.145,
                WOOD_DARK,
            );
        }
        "figurehead" => {
            // A carved lady of the sea, lying where the tide left her.
            tube(
                &mut m,
                6,
                Vec3::new(0.0, 0.25, -0.9),
                0.3,
                Vec3::new(0.0, 0.3, 0.5),
                0.25,
                WOOD,
            );
            m.blob(
                Vec3::new(0.0, 0.4, 0.85),
                Vec3::splat(0.28),
                false,
                0.05,
                4,
                Rgb(0xE8C8A0),
            );
            m.blob(
                Vec3::new(0.0, 0.5, 0.65),
                Vec3::new(0.3, 0.2, 0.3),
                false,
                0.1,
                5,
                Rgb(0xC0402A),
            );
            tube(
                &mut m,
                4,
                Vec3::new(0.0, 0.25, -0.9),
                0.3,
                Vec3::new(0.0, 0.2, -1.3),
                0.05,
                WOOD_DARK,
            );
            shiny(&mut m, |m| gem(m, Vec3::new(0.0, 0.62, 0.95), 0.1, GOLD));
        }
        "chest" => {
            m.cube(Vec3::new(0.0, 0.32, 0.0), Vec3::new(0.7, 0.32, 0.45), WOOD);
            let mut lid = MeshBuilder::new();
            lid.frustum(6, -0.7, 0.45, 0.7, 0.45, WOOD_DARK, WOOD_DARK);
            m.append(
                &lid,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_z(PI / 2.0),
                    Vec3::new(0.0, 0.64, 0.0),
                ),
            );
            shiny(&mut m, |m| {
                for x in [-0.5, 0.5] {
                    m.cube(Vec3::new(x, 0.55, 0.0), Vec3::new(0.06, 0.55, 0.47), GOLD);
                }
                m.cube(
                    Vec3::new(0.0, 0.62, 0.47),
                    Vec3::new(0.12, 0.12, 0.03),
                    GOLD,
                );
                // Gold spilling over the front.
                m.blob(
                    Vec3::new(0.2, 0.12, 0.6),
                    Vec3::new(0.3, 0.12, 0.2),
                    false,
                    0.2,
                    6,
                    GOLD,
                );
            });
        }
        "lens" => {
            shiny(&mut m, |m| hoop(m, 10, 0.0, 0.6, 0.08, 0.1, BRASS));
            let mut glass = MeshBuilder::new();
            glass.blob(
                Vec3::ZERO,
                Vec3::new(0.56, 0.12, 0.56),
                false,
                0.0,
                2,
                GLASS,
            );
            m.append(&glass, Mat4::from_translation(Vec3::new(0.0, 0.1, 0.0)));
            let mut out = MeshBuilder::new();
            out.append(
                &m,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(1.2),
                    Vec3::new(0.0, 0.55, 0.0),
                ),
            );
            return out;
        }
        "chess" => {
            // A king piece, big as a dog.
            part(&mut m, 8, 0.0, 0.4, 0.15, 0.36, IVORY);
            part(&mut m, 8, 0.15, 0.25, 0.8, 0.16, IVORY);
            part(&mut m, 8, 0.8, 0.28, 1.0, 0.24, IVORY);
            shiny(&mut m, |m| {
                m.cube(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.05, 0.2, 0.05), GOLD);
                m.cube(Vec3::new(0.0, 1.25, 0.0), Vec3::new(0.15, 0.05, 0.05), GOLD);
            });
        }
        "lantern" => {
            shiny(&mut m, |m| {
                part(m, 6, 0.0, 0.35, 0.1, 0.35, BRASS);
                part(m, 6, 0.8, 0.38, 1.1, 0.0, BRASS);
                for i in 0..6 {
                    let a = i as f32 / 6.0 * TAU;
                    let p = Vec3::new(a.cos() * 0.33, 0.0, a.sin() * 0.33);
                    tube(
                        m,
                        3,
                        p + Vec3::Y * 0.1,
                        0.03,
                        p + Vec3::Y * 0.8,
                        0.03,
                        BRASS,
                    );
                }
            });
            let from = m.verts.len();
            part(&mut m, 6, 0.1, 0.3, 0.8, 0.3, Rgb(0xFFD27A));
            m.glow_from(from);
        }
        "anchor" => shiny(&mut m, |m| {
            tube(
                m,
                5,
                Vec3::new(0.0, 0.1, -0.9),
                0.08,
                Vec3::new(0.0, 0.1, 0.7),
                0.08,
                SILVER,
            );
            tube(
                m,
                5,
                Vec3::new(-0.5, 0.1, 0.5),
                0.07,
                Vec3::new(0.5, 0.1, 0.5),
                0.07,
                SILVER,
            );
            tube(
                m,
                5,
                Vec3::new(0.0, 0.1, -0.9),
                0.08,
                Vec3::new(-0.6, 0.1, -0.5),
                0.06,
                SILVER,
            );
            tube(
                m,
                5,
                Vec3::new(0.0, 0.1, -0.9),
                0.08,
                Vec3::new(0.6, 0.1, -0.5),
                0.06,
                SILVER,
            );
        }),
        "harp" => {
            shiny(&mut m, |m| {
                tube(
                    m,
                    5,
                    Vec3::new(0.0, 0.0, -0.4),
                    0.08,
                    Vec3::new(0.0, 1.3, -0.4),
                    0.06,
                    GOLD,
                );
                tube(
                    m,
                    5,
                    Vec3::new(0.0, 1.3, -0.4),
                    0.06,
                    Vec3::new(0.0, 1.1, 0.5),
                    0.06,
                    GOLD,
                );
                tube(
                    m,
                    5,
                    Vec3::new(0.0, 1.1, 0.5),
                    0.08,
                    Vec3::new(0.0, 0.0, 0.2),
                    0.1,
                    GOLD,
                );
                part(m, 6, 0.0, 0.35, 0.08, 0.35, GOLD_DEEP);
            });
            for i in 0..6 {
                let z = -0.3 + i as f32 * 0.14;
                let top = 1.25 - i as f32 * 0.03;
                tube(
                    &mut m,
                    3,
                    Vec3::new(0.0, 0.1, z),
                    0.012,
                    Vec3::new(0.0, top, z),
                    0.012,
                    IVORY,
                );
            }
        }
        "egg" => {
            // A stone egg, cool to the touch and very old.
            m.blob(
                Vec3::new(0.0, 0.55, 0.0),
                Vec3::new(0.42, 0.55, 0.42),
                true,
                0.03,
                7,
                Rgb(0x8FB8A8),
            );
            shiny(&mut m, |m| hoop(m, 10, 0.5, 0.43, 0.04, 0.05, GOLD));
        }
        "map" => {
            // A rolled chart, tied with a ribbon.
            tube(
                &mut m,
                8,
                Vec3::new(-0.6, 0.2, 0.0),
                0.2,
                Vec3::new(0.6, 0.2, 0.0),
                0.2,
                Rgb(0xF2E2B8),
            );
            tube(
                &mut m,
                8,
                Vec3::new(-0.05, 0.2, 0.0),
                0.215,
                Vec3::new(0.05, 0.2, 0.0),
                0.215,
                RUBY,
            );
        }
        "teapot" => {
            shiny(&mut m, |m| {
                m.blob(
                    Vec3::new(0.0, 0.4, 0.0),
                    Vec3::new(0.5, 0.4, 0.5),
                    true,
                    0.0,
                    3,
                    SILVER,
                );
                tube(
                    m,
                    5,
                    Vec3::new(0.4, 0.4, 0.0),
                    0.09,
                    Vec3::new(0.8, 0.7, 0.0),
                    0.05,
                    SILVER,
                );
                tube(
                    m,
                    4,
                    Vec3::new(-0.42, 0.6, 0.0),
                    0.05,
                    Vec3::new(-0.62, 0.4, 0.0),
                    0.05,
                    SILVER,
                );
                tube(
                    m,
                    4,
                    Vec3::new(-0.62, 0.4, 0.0),
                    0.05,
                    Vec3::new(-0.42, 0.22, 0.0),
                    0.05,
                    SILVER,
                );
                part(m, 6, 0.78, 0.15, 0.92, 0.04, SILVER);
            });
        }
        "music-box" => {
            m.cube(
                Vec3::new(0.0, 0.3, 0.0),
                Vec3::new(0.5, 0.3, 0.35),
                Rgb(0x6A3A5A),
            );
            shiny(&mut m, |m| {
                m.cube(Vec3::new(0.0, 0.62, 0.0), Vec3::new(0.52, 0.03, 0.37), GOLD);
                tube(
                    m,
                    4,
                    Vec3::new(0.5, 0.3, 0.0),
                    0.03,
                    Vec3::new(0.7, 0.3, 0.0),
                    0.03,
                    GOLD,
                );
                m.cube(Vec3::new(0.7, 0.3, 0.08), Vec3::new(0.02, 0.1, 0.08), GOLD);
            });
            m.blob(
                Vec3::new(0.0, 0.8, 0.0),
                Vec3::new(0.08, 0.16, 0.08),
                false,
                0.0,
                2,
                Rgb(0xFF9EC8),
            );
        }
        "kite" => {
            // A diamond kite with a bowed tail, lying where it caught.
            let (n, e, s_, w) = (
                Vec3::new(0.0, 0.3, 0.9),
                Vec3::new(0.55, 0.3, 0.0),
                Vec3::new(0.0, 0.3, -0.5),
                Vec3::new(-0.55, 0.3, 0.0),
            );
            let c = Vec3::new(0.0, 0.42, 0.1);
            m.thin_tri(n, c, e, Rgb(0xE0564A));
            m.thin_tri(e, c, s_, Rgb(0xF2C14E));
            m.thin_tri(s_, c, w, Rgb(0xE0564A));
            m.thin_tri(w, c, n, Rgb(0xF2C14E));
            for k in 0..4 {
                let z = -0.6 - k as f32 * 0.35;
                let x = if k % 2 == 0 { 0.12 } else { -0.12 };
                m.cube(
                    Vec3::new(x, 0.2, z),
                    Vec3::new(0.1, 0.04, 0.06),
                    [Rgb(0x4A7FE0), Rgb(0x3EC07A)][k % 2],
                );
            }
        }
        "basket" => {
            part(&mut m, 8, 0.0, 0.42, 0.5, 0.55, Rgb(0xC89A5E));
            part(&mut m, 8, 0.5, 0.55, 0.56, 0.56, Rgb(0xA87A44));
            tube(
                &mut m,
                4,
                Vec3::new(-0.5, 0.55, 0.0),
                0.04,
                Vec3::new(0.0, 1.0, 0.0),
                0.04,
                Rgb(0xA87A44),
            );
            tube(
                &mut m,
                4,
                Vec3::new(0.0, 1.0, 0.0),
                0.04,
                Vec3::new(0.5, 0.55, 0.0),
                0.04,
                Rgb(0xA87A44),
            );
            m.blob(
                Vec3::new(0.0, 0.58, 0.0),
                Vec3::new(0.45, 0.1, 0.45),
                false,
                0.1,
                5,
                Rgb(0xE8E0D0),
            );
        }
        "hat" => {
            part(&mut m, 10, 0.0, 0.62, 0.06, 0.6, Rgb(0x2A2E4A));
            part(&mut m, 10, 0.06, 0.36, 0.42, 0.33, Rgb(0x2A2E4A));
            part(&mut m, 10, 0.42, 0.33, 0.48, 0.0, Rgb(0x2A2E4A));
            shiny(&mut m, |m| {
                m.cube(Vec3::new(0.0, 0.3, 0.35), Vec3::new(0.12, 0.08, 0.02), GOLD)
            });
            part(&mut m, 10, 0.06, 0.37, 0.14, 0.37, IVORY);
        }
        "shell" => {
            // A scallop: ribbed fan halves.
            for k in 0..7 {
                let a0 = -1.2 + k as f32 * 0.34;
                let a1 = a0 + 0.34;
                let tip = Vec3::new(0.0, 0.1, -0.35);
                let p = |a: f32, up: f32| Vec3::new(a.sin() * 0.6, 0.1 + up, a.cos() * 0.6 - 0.35);
                let c = if k % 2 == 0 {
                    Rgb(0xFFC9B0)
                } else {
                    Rgb(0xF4A98E)
                };
                m.tri(tip, p(a1, 0.18), p(a0, 0.18), c);
                m.tri(tip, p(a0, 0.0), p(a1, 0.0), c);
            }
        }
        "painting" => {
            // A framed picture, propped up.
            let mut f = MeshBuilder::new();
            shiny(&mut f, |f| {
                f.cube(Vec3::ZERO, Vec3::new(0.6, 0.45, 0.05), GOLD_DEEP)
            });
            f.cube(
                Vec3::new(0.0, 0.12, 0.05),
                Vec3::new(0.5, 0.25, 0.02),
                Rgb(0x9CCBF0),
            );
            f.cube(
                Vec3::new(0.0, -0.2, 0.05),
                Vec3::new(0.5, 0.15, 0.02),
                Rgb(0x8FD16A),
            );
            f.blob(
                Vec3::new(0.25, 0.22, 0.07),
                Vec3::splat(0.08),
                false,
                0.0,
                2,
                Rgb(0xFFE38A),
            );
            m.append(
                &f,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(-0.25),
                    Vec3::new(0.0, 0.48, 0.0),
                ),
            );
        }
        "ribbon" => {
            // A prize rosette, leaning back: frilled blue petals round a gold middle, and
            // two tails hanging down.
            let mut r = MeshBuilder::new();
            for k in 0..12 {
                let a = k as f32 / 12.0 * TAU;
                let c = if k % 2 == 0 { SAPPHIRE } else { Rgb(0x6A9AF0) };
                r.blob(
                    Vec3::new(a.cos() * 0.3, 0.0, a.sin() * 0.3),
                    Vec3::new(0.14, 0.04, 0.14),
                    false,
                    0.0,
                    1,
                    c,
                );
            }
            part(&mut r, 10, 0.0, 0.26, 0.06, 0.26, Rgb(0xF2F2F6));
            shiny(&mut r, |r| part(r, 10, 0.06, 0.17, 0.1, 0.15, GOLD));
            for side in [-1.0f32, 1.0] {
                r.cube(
                    Vec3::new(side * 0.12, 0.0, 0.45),
                    Vec3::new(0.08, 0.02, 0.25),
                    SAPPHIRE,
                );
            }
            m.append(
                &r,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(-1.1),
                    Vec3::new(0.0, 0.42, 0.0),
                ),
            );
        }
        "trophy" => {
            // A loving cup on a wooden plinth, a handle each side.
            part(&mut m, 4, 0.0, 0.32, 0.22, 0.28, WOOD_DARK);
            shiny(&mut m, |m| {
                part(m, 8, 0.22, 0.18, 0.3, 0.12, SILVER);
                part(m, 6, 0.3, 0.06, 0.55, 0.06, SILVER);
                part(m, 10, 0.55, 0.1, 0.95, 0.34, SILVER);
                part(m, 10, 0.95, 0.34, 1.0, 0.36, SILVER);
                for side in [-1.0f32, 1.0] {
                    tube(
                        m,
                        4,
                        Vec3::new(side * 0.3, 0.9, 0.0),
                        0.04,
                        Vec3::new(side * 0.48, 0.78, 0.0),
                        0.04,
                        SILVER,
                    );
                    tube(
                        m,
                        4,
                        Vec3::new(side * 0.48, 0.78, 0.0),
                        0.04,
                        Vec3::new(side * 0.2, 0.62, 0.0),
                        0.04,
                        SILVER,
                    );
                }
            });
            gem(&mut m, Vec3::new(0.0, 0.8, 0.27), 0.07, RUBY);
        }
        "pot" => {
            // The pot at the end of the rainbow: a round black pot heaped with gold.
            part(&mut m, 10, 0.0, 0.3, 0.12, 0.5, Rgb(0x2E2A2C));
            part(&mut m, 10, 0.12, 0.5, 0.55, 0.58, Rgb(0x3A3537));
            part(&mut m, 10, 0.55, 0.58, 0.68, 0.5, Rgb(0x2E2A2C));
            hoop(&mut m, 12, 0.66, 0.53, 0.05, 0.06, Rgb(0x4A4446));
            for s in [-1.0f32, 1.0] {
                m.cube(
                    Vec3::new(s * 0.6, 0.55, 0.0),
                    Vec3::new(0.06, 0.06, 0.1),
                    Rgb(0x4A4446),
                );
            }
            shiny(&mut m, |m| {
                m.blob(
                    Vec3::new(0.0, 0.7, 0.0),
                    Vec3::new(0.48, 0.2, 0.48),
                    false,
                    0.08,
                    3,
                    GOLD,
                );
                for i in 0..7 {
                    let a = i as f32 * 0.9;
                    let at =
                        Vec3::new(a.cos() * 0.25, 0.86 + (i % 3) as f32 * 0.04, a.sin() * 0.25);
                    let mut c = MeshBuilder::new();
                    c.frustum(8, 0.0, 0.11, 0.025, 0.11, GOLD, GOLD);
                    m.append(
                        &c,
                        Mat4::from_rotation_translation(Quat::from_rotation_x(0.5 + a), at),
                    );
                }
            });
        }
        "pearl" => {
            // A great pearl in its open shell.
            part(&mut m, 12, 0.0, 0.5, 0.12, 0.55, Rgb(0xC9A9B8));
            m.blob(
                Vec3::new(0.0, 0.42, 0.0),
                Vec3::splat(0.3),
                true,
                0.0,
                9,
                Rgb(0xF4EEF2),
            );
            let mut lid = MeshBuilder::new();
            lid.frustum(12, 0.0, 0.55, 0.1, 0.5, Rgb(0xD8B8C6), Rgb(0xE8D0DA));
            m.append(
                &lid,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(-1.1),
                    Vec3::new(0.0, 0.15, -0.5),
                ),
            );
        }
        "vase" => {
            // A painted vase, blue on white.
            part(&mut m, 10, 0.0, 0.22, 0.1, 0.25, IVORY);
            part(&mut m, 10, 0.1, 0.25, 0.55, 0.4, IVORY);
            part(&mut m, 10, 0.55, 0.4, 0.85, 0.18, IVORY);
            part(&mut m, 10, 0.85, 0.18, 1.05, 0.24, IVORY);
            hoop(&mut m, 12, 0.5, 0.41, 0.03, 0.08, SAPPHIRE);
            hoop(&mut m, 12, 0.3, 0.34, 0.03, 0.05, SAPPHIRE);
        }
        "key" => shiny(&mut m, |m| {
            // A big old key, lying down.
            hoop(m, 10, 0.06, 0.22, 0.06, 0.06, BRASS);
            m.cube(
                Vec3::new(0.0, 0.06, 0.55),
                Vec3::new(0.05, 0.05, 0.36),
                BRASS,
            );
            m.cube(
                Vec3::new(0.1, 0.06, 0.8),
                Vec3::new(0.08, 0.05, 0.04),
                BRASS,
            );
            m.cube(
                Vec3::new(0.1, 0.06, 0.68),
                Vec3::new(0.06, 0.05, 0.04),
                BRASS,
            );
        }),
        "book" => {
            // A thick old book, its cover red and its pages cream.
            m.cube(
                Vec3::new(0.0, 0.14, 0.0),
                Vec3::new(0.38, 0.14, 0.5),
                Rgb(0xB8443A),
            );
            m.cube(
                Vec3::new(0.03, 0.14, 0.0),
                Vec3::new(0.37, 0.11, 0.47),
                IVORY,
            );
            shiny(&mut m, |m| {
                m.cube(
                    Vec3::new(-0.38, 0.14, 0.0),
                    Vec3::new(0.02, 0.15, 0.5),
                    GOLD,
                )
            });
        }
        "medal" => {
            // A medal on a striped ribbon.
            m.cube(
                Vec3::new(0.0, 0.02, -0.25),
                Vec3::new(0.16, 0.02, 0.3),
                Rgb(0x4A7FE0),
            );
            m.cube(
                Vec3::new(0.0, 0.03, -0.25),
                Vec3::new(0.05, 0.02, 0.3),
                Rgb(0xE0564A),
            );
            shiny(&mut m, |m| {
                let mut c = MeshBuilder::new();
                c.frustum(12, 0.0, 0.3, 0.06, 0.3, GOLD, GOLD_DEEP);
                m.append(&c, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.2)));
            });
        }
        "fossil" => {
            // A curled shell fossil in a flat stone.
            m.blob(
                Vec3::new(0.0, 0.12, 0.0),
                Vec3::new(0.55, 0.14, 0.45),
                true,
                0.04,
                5,
                Rgb(0xB8AE9C),
            );
            for i in 0..9 {
                let a = i as f32 * 0.8;
                let r = 0.06 + i as f32 * 0.035;
                m.blob(
                    Vec3::new(a.cos() * r, 0.24, a.sin() * r),
                    Vec3::splat(0.05 + i as f32 * 0.008),
                    false,
                    0.0,
                    i as u32,
                    Rgb(0x8C8270),
                );
            }
        }
        "flute" => {
            // A wooden flute lying on its side.
            tube(
                &mut m,
                6,
                Vec3::new(-0.6, 0.06, 0.0),
                0.06,
                Vec3::new(0.6, 0.06, 0.0),
                0.05,
                WOOD,
            );
            for i in 0..5 {
                m.cube(
                    Vec3::new(-0.25 + i as f32 * 0.12, 0.12, 0.0),
                    Vec3::new(0.02, 0.01, 0.02),
                    INK,
                );
            }
        }
        "toy-boat" => {
            // A little toy sailing boat.
            m.cube(
                Vec3::new(0.0, 0.12, 0.0),
                Vec3::new(0.18, 0.12, 0.5),
                Rgb(0xE0564A),
            );
            m.cube(
                Vec3::new(0.0, 0.25, 0.0),
                Vec3::new(0.16, 0.02, 0.46),
                IVORY,
            );
            tube(
                &mut m,
                4,
                Vec3::new(0.0, 0.25, 0.0),
                0.025,
                Vec3::new(0.0, 1.0, 0.0),
                0.02,
                WOOD,
            );
            m.tri(
                Vec3::new(0.0, 0.95, 0.0),
                Vec3::new(0.0, 0.32, 0.0),
                Vec3::new(0.0, 0.32, 0.4),
                IVORY,
            );
            m.tri(
                Vec3::new(0.0, 0.95, 0.0),
                Vec3::new(0.0, 0.32, 0.4),
                Vec3::new(0.0, 0.32, 0.0),
                IVORY,
            );
        }
        "gem" => {
            // A big cut gem, green as the sea.
            m.blob(
                Vec3::new(0.0, 0.35, 0.0),
                Vec3::new(0.32, 0.35, 0.32),
                false,
                0.0,
                2,
                EMERALD,
            );
            m.blob(
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::new(0.18, 0.2, 0.18),
                false,
                0.0,
                4,
                Rgb(0x8FE8B8),
            );
        }
        "clock" => {
            // A brass carriage clock.
            shiny(&mut m, |m| {
                m.cube(Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.3, 0.3, 0.18), BRASS);
                m.cube(
                    Vec3::new(0.0, 0.64, 0.0),
                    Vec3::new(0.12, 0.04, 0.04),
                    BRASS,
                );
            });
            let mut face = MeshBuilder::new();
            face.frustum(12, 0.0, 0.2, 0.02, 0.2, IVORY, IVORY);
            m.append(
                &face,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(PI / 2.0),
                    Vec3::new(0.0, 0.32, 0.18),
                ),
            );
            m.cube(
                Vec3::new(0.0, 0.38, 0.21),
                Vec3::new(0.012, 0.08, 0.01),
                INK,
            );
            m.cube(
                Vec3::new(0.05, 0.32, 0.21),
                Vec3::new(0.06, 0.012, 0.01),
                INK,
            );
        }
        "candle" => {
            // A candle in a brass holder, still burning.
            shiny(&mut m, |m| {
                part(m, 10, 0.0, 0.35, 0.05, 0.35, BRASS);
                part(m, 8, 0.05, 0.08, 0.15, 0.08, BRASS);
            });
            part(&mut m, 8, 0.15, 0.1, 0.75, 0.1, IVORY);
            m.blob(
                Vec3::new(0.0, 0.85, 0.0),
                Vec3::new(0.05, 0.1, 0.05),
                false,
                0.0,
                1,
                Rgb(0xFFC94A),
            );
        }
        _ => coins(&mut m, 1.0),
    }
    m
}

/// A bunting flag, white to be tinted: a triangle hanging from its top edge along x,
/// a unit across.
pub fn flag() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (a, b, tip) = (
        Vec3::new(-0.5, 0.0, 0.0),
        Vec3::new(0.5, 0.0, 0.0),
        Vec3::new(0.0, -0.8, 0.0),
    );
    m.tri(a, b, tip, Rgb(0xFFFFFF));
    m.tri(b, a, tip, Rgb(0xFFFFFF));
    m
}

/// A round paper lantern, white to be tinted, hanging from y = 0.
pub fn paper_lantern() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    m.blob(
        Vec3::new(0.0, -0.45, 0.0),
        Vec3::new(0.32, 0.38, 0.32),
        false,
        0.0,
        2,
        Rgb(0xFFFFFF),
    );
    part(&mut m, 6, -0.1, 0.12, 0.0, 0.1, Rgb(0xFFFFFF));
    m
}

/// An old lamp for a story: a stone post with an iron bowl on top, where the fire
/// burns 2.2 m up.
pub fn brazier() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let stone = Rgb(0x9A938A);
    let iron = Rgb(0x3E3A3A);
    part(&mut m, 6, 0.0, 0.45, 0.25, 0.4, stone);
    part(&mut m, 6, 0.25, 0.22, 1.7, 0.18, stone);
    part(&mut m, 6, 1.7, 0.3, 1.85, 0.3, stone);
    part(&mut m, 8, 1.85, 0.15, 2.2, 0.45, iron);
    part(&mut m, 8, 2.2, 0.45, 2.25, 0.48, iron);
    m
}

/// A plain wooden pole a unit tall, to be stretched.
pub fn pole() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    part(&mut m, 6, 0.0, 0.1, 1.0, 0.08, WOOD_DARK);
    m
}

/// The journal on a wooden stand, open, for the lair.
pub fn lectern() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    tube(
        &mut m,
        5,
        Vec3::ZERO,
        0.12,
        Vec3::new(0.0, 1.0, 0.0),
        0.08,
        WOOD_DARK,
    );
    part(&mut m, 6, 0.0, 0.4, 0.06, 0.35, WOOD_DARK);
    let mut top = MeshBuilder::new();
    top.cube(Vec3::ZERO, Vec3::new(0.5, 0.04, 0.35), WOOD);
    top.cube(
        Vec3::new(-0.24, 0.07, 0.0),
        Vec3::new(0.22, 0.03, 0.3),
        IVORY,
    );
    top.cube(
        Vec3::new(0.24, 0.07, 0.0),
        Vec3::new(0.22, 0.03, 0.3),
        IVORY,
    );
    top.cube(
        Vec3::new(0.0, 0.06, 0.0),
        Vec3::new(0.02, 0.04, 0.31),
        Rgb(0xE0564A),
    );
    m.append(
        &top,
        Mat4::from_rotation_translation(Quat::from_rotation_x(-0.5), Vec3::new(0.0, 1.05, 0.0)),
    );
    m
}

/// A speech bubble over someone who can be talked to, a meter wide, facing +z, glowing
/// so it reads by night too. `ask` marks someone with a favor to ask or be thanked for.
/// A passenger's seat for the dragon's back: a woven basket on a striped blanket, with
/// a strap down each side. Its floor is at the origin, a unit across.
pub fn saddle() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let (wicker, rim, cloth, stripe, strap) = (
        Rgb(0xC89A5E),
        Rgb(0x9A6A38),
        Rgb(0xD9534A),
        Rgb(0xF2C14E),
        Rgb(0x5A3A24),
    );
    m.cube(
        Vec3::new(0.0, -0.04, 0.0),
        Vec3::new(0.62, 0.05, 0.78),
        cloth,
    );
    m.cube(
        Vec3::new(0.0, -0.035, 0.0),
        Vec3::new(0.64, 0.045, 0.12),
        stripe,
    );
    for x in [-1.0f32, 1.0] {
        m.cube(
            Vec3::new(x * 0.6, -0.3, 0.0),
            Vec3::new(0.04, 0.3, 0.07),
            strap,
        );
    }
    part(&mut m, 9, 0.0, 0.42, 0.42, 0.5, wicker);
    part(&mut m, 9, 0.42, 0.52, 0.48, 0.52, rim);
    m
}

/// An umbrella held up by a passenger floating down: a striped canopy on a stick, the
/// handle at the origin.
pub fn umbrella() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    tube(
        &mut m,
        4,
        Vec3::ZERO,
        0.03,
        Vec3::new(0.0, 1.5, 0.0),
        0.03,
        Rgb(0x5A3A24),
    );
    let sides = 8;
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        let c = if i % 2 == 0 {
            Rgb(0xE0564A)
        } else {
            Rgb(0xFFF6E6)
        };
        let rim = |a: f32| Vec3::new(a.cos() * 1.1, 1.25, a.sin() * 1.1);
        let top = Vec3::new(0.0, 1.6, 0.0);
        m.tri(top, rim(a1), rim(a0), c);
        m.tri(top, rim(a0), rim(a1), c);
    }
    m
}

pub fn bubble(ask: bool) -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let rim = if ask { Rgb(0xF2C14E) } else { Rgb(0x8A6A50) };
    let fill = Rgb(0xFFFDF6);
    let sides = 10;
    let at = |i: u32, r: f32| {
        let a = i as f32 / sides as f32 * TAU;
        Vec3::new(a.cos() * r * 1.3, a.sin() * r, 0.0)
    };
    for i in 0..sides {
        m.tri(
            Vec3::new(0.0, 0.0, -0.01),
            at(i + 1, 0.56),
            at(i, 0.56),
            rim,
        );
        m.tri(Vec3::ZERO, at(i, 0.48), at(i + 1, 0.48), fill);
    }
    // The tail, pointing down to the speaker.
    m.tri(
        Vec3::new(-0.1, -0.4, 0.0),
        Vec3::new(-0.25, -0.85, 0.0),
        Vec3::new(0.18, -0.42, 0.0),
        fill,
    );
    if ask {
        m.cube(
            Vec3::new(0.0, 0.08, 0.02),
            Vec3::new(0.06, 0.2, 0.01),
            Rgb(0xE0564A),
        );
        m.cube(
            Vec3::new(0.0, -0.25, 0.02),
            Vec3::new(0.06, 0.06, 0.01),
            Rgb(0xE0564A),
        );
    } else {
        for x in [-0.25, 0.0, 0.25] {
            m.cube(
                Vec3::new(x, 0.0, 0.02),
                Vec3::new(0.06, 0.06, 0.01),
                Rgb(0x8A6A50),
            );
        }
    }
    m.glow_from(0);
    m
}

/// A heap of coins, `size` meters to the top, with a few loose ones round it.
pub fn coins(m: &mut MeshBuilder, size: f32) {
    shiny(m, |m| {
        m.blob(
            Vec3::new(0.0, size * 0.25, 0.0),
            Vec3::new(size * 0.9, size * 0.5, size * 0.9),
            true,
            0.12,
            9,
            GOLD,
        );
        for i in 0..7 {
            let a = i as f32 * 2.39;
            let r = size * (0.95 + 0.25 * (i % 3) as f32);
            let mut c = MeshBuilder::new();
            c.frustum(6, 0.0, 0.14, 0.04, 0.14, GOLD_DEEP, GOLD);
            m.append(
                &c,
                Mat4::from_rotation_translation(
                    Quat::from_rotation_x(0.3 * (i % 2) as f32),
                    Vec3::new(a.cos() * r, 0.02, a.sin() * r),
                ),
            );
        }
    });
}

/// The glint over a find: a four point star, unit size, drawn glowing.
pub fn star() -> MeshBuilder {
    let mut m = MeshBuilder::new();
    let w = Rgb(0xFFFFFF);
    for i in 0..4 {
        let a = i as f32 / 4.0 * TAU;
        let (c, s) = (a.cos(), a.sin());
        let tip = Vec3::new(c, 0.0, s);
        let side = Vec3::new(-s, 0.0, c) * 0.16;
        m.tri(Vec3::ZERO, tip, side, w);
        m.tri(Vec3::ZERO, -side, tip, w);
    }
    for (i, v) in m.verts.iter_mut().enumerate() {
        v.color[3] = if i % 3 == 0 { 255 } else { 0 };
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind the gameplay file allows has a model of its own, small and cheap.
    #[test]
    fn every_kind_has_a_model() {
        for kind in KINDS {
            let m = treasure(kind);
            let tris = m.verts.len() / 3;
            assert!(tris > 4 && tris < 600, "{kind}: {tris}");
        }
    }
}
