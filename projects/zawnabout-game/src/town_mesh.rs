//! Town meshes, written by hand from boxes, gables and cones. Every building is baked into
//! the mesh of the tile it stands in, once with windows, doors and trim for up close and
//! once as plain shapes for far away. Roads and bridges are baked the same way.

use std::collections::HashMap;
use std::f32::consts::TAU;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use zawnabout_sim::bridge::Suspension;
use zawnabout_sim::shape::hash;
use zawnabout_sim::terrain::{CoverKind, Style, Tech, Terrain};
use zawnabout_sim::town::{Building, Kind};

use crate::mesh::{MeshBuilder, Rgb};

/// Town tiles are this big, for culling and detail.
pub const TILE: f32 = 512.0;

/// A tile's meshes: near and far, and whether it is under ground.
pub struct Tile {
    pub near: MeshBuilder,
    pub far: MeshBuilder,
    pub cave: bool,
}

/// Palette picks for one building.
struct Colors {
    wall: Rgb,
    roof: Rgb,
    trim: Rgb,
    window: Rgb,
    door: Rgb,
}

const DARK_GLASS: Rgb = Rgb(0x3D5873);
const WARM_GLASS: Rgb = Rgb(0xFFD27A);
const WOOD: Rgb = Rgb(0x9A6B47);
const DARK_WOOD: Rgb = Rgb(0x6E4A33);
const STONE: Rgb = Rgb(0xB8AE9C);
const WHITE: Rgb = Rgb(0xF6F1E7);
const RED: Rgb = Rgb(0xE0564A);
const STEEL: Rgb = Rgb(0x8E9AA6);
const LAMP: Rgb = Rgb(0xFFE59A);
const SAIL: Rgb = Rgb(0xFFFDF7);
const ASPHALT: Rgb = Rgb(0x6F6E73);
const PAVING: Rgb = Rgb(0xCDBFA5);

fn pick(list: &[Rgb], seed: u32, salt: u32) -> Rgb {
    list[(hash(seed.wrapping_mul(31).wrapping_add(salt)) * list.len() as f32) as usize % list.len()]
}

fn colors(b: &Building) -> Colors {
    let s = b.seed;
    let (walls, roofs): (&[Rgb], &[Rgb]) = match (b.style, b.tech) {
        (Style::Canal, _) => (
            &[
                Rgb(0xC9705A),
                Rgb(0xB85C4A),
                Rgb(0xE8D5B0),
                Rgb(0xF2E6CF),
                Rgb(0x8FA9B8),
                Rgb(0xD9A05B),
                Rgb(0x9C5545),
            ],
            &[Rgb(0x4E4A55), Rgb(0x5C4B45), Rgb(0x7A4A3E)],
        ),
        (Style::Hill, _) | (Style::Cliff, _) => (
            &[
                Rgb(0xFBF6EC),
                Rgb(0xF4E9D4),
                Rgb(0xFFE8C2),
                Rgb(0xF7D6C6),
                Rgb(0xD5ECF2),
                Rgb(0xFFF3D6),
            ],
            &[Rgb(0xD9774E), Rgb(0xC9653F), Rgb(0x5F9BC9), Rgb(0xE08B5A)],
        ),
        (Style::Stone, _) => (
            &[Rgb(0xB9AF9E), Rgb(0xA79F91), Rgb(0xC7BBA4), Rgb(0x9E978C)],
            &[Rgb(0x5B6470), Rgb(0x4F5763), Rgb(0x6B5D52)],
        ),
        (Style::Seafaring, _) => (
            &[
                Rgb(0x6FA8C9),
                Rgb(0xE9DFC9),
                Rgb(0xD96C5B),
                Rgb(0xF2C14E),
                Rgb(0x7CB37A),
                Rgb(0xF4EFE6),
            ],
            &[Rgb(0x5A4E4A), Rgb(0x3F5A6B), Rgb(0x7A4A3E)],
        ),
        (Style::Modern, _) => (
            &[
                Rgb(0x9FC3D9),
                Rgb(0xB8D6E3),
                Rgb(0xD8E2E8),
                Rgb(0x87B4C9),
                Rgb(0xE9EEF0),
                Rgb(0xA9C9C0),
            ],
            &[Rgb(0x9AA3AB)],
        ),
        (Style::Cavern, _) => (
            &[Rgb(0xE8C9A0), Rgb(0xD9A877), Rgb(0xF0D9B5), Rgb(0xC98F6A)],
            &[Rgb(0x8A5A44), Rgb(0x6E4A3A), Rgb(0x9C6B4E)],
        ),
        (Style::Stilts, _) | (_, Tech::Simple) => (
            &[Rgb(0xC9A475), Rgb(0xB8935F), Rgb(0xD6B887)],
            &[Rgb(0xD9BE7C), Rgb(0xC9A964), Rgb(0xE3C98E)],
        ),
        (_, Tech::Modern) => (
            &[
                Rgb(0xFFFFFF),
                Rgb(0xF5EBDD),
                Rgb(0xDDEFF2),
                Rgb(0xFFE9D9),
                Rgb(0xE6F2DA),
            ],
            &[Rgb(0x5FA7C9), Rgb(0xE07A5F), Rgb(0x81B29A)],
        ),
        _ => (
            &[
                Rgb(0xF3E6CC),
                Rgb(0xFFF4E0),
                Rgb(0xE9D3B0),
                Rgb(0xDCE8F0),
                Rgb(0xF6D9C9),
                Rgb(0xE3EFD3),
                Rgb(0xF2C9A0),
                Rgb(0xC9DDE8),
                Rgb(0xEFD98C),
                Rgb(0xD8C6E6),
            ],
            &[
                Rgb(0xC85C48),
                Rgb(0x3F6E8C),
                Rgb(0x5E8C4E),
                Rgb(0x8C5A44),
                Rgb(0xD98A3D),
                Rgb(0x6B5B8C),
                Rgb(0x4A4A52),
            ],
        ),
    };
    let cave = b.style == Style::Cavern;
    // A touch lighter or darker, so neighbors in one color still differ.
    let shade = hash(s.wrapping_mul(7).wrapping_add(11));
    let wall = pick(walls, s, 1);
    let wall = if shade < 0.5 {
        wall.mix(Rgb(0xFFFFFF), (0.5 - shade) * 0.3)
    } else {
        wall.mix(Rgb(0x2A2020), (shade - 0.5) * 0.25)
    };
    Colors {
        wall,
        roof: pick(roofs, s, 2),
        trim: if b.style == Style::Stone {
            Rgb(0x8C857A)
        } else {
            WHITE
        },
        window: if cave || b.tech == Tech::Simple {
            WARM_GLASS
        } else {
            DARK_GLASS
        },
        door: pick(
            &[DARK_WOOD, Rgb(0x3F6E8C), Rgb(0xB84A3E), Rgb(0x5E8C4E)],
            s,
            3,
        ),
    }
}

/// Builds one building's parts in its own frame (front +z, ridge along x, base at 0).
struct Parts {
    m: MeshBuilder,
    /// Indices of triangles that glow on their own.
    glow: Vec<usize>,
    /// Indices of triangles that light up at night.
    night: Vec<usize>,
}

impl Parts {
    fn new() -> Self {
        Self {
            m: MeshBuilder::new(),
            glow: Vec::new(),
            night: Vec::new(),
        }
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, col: Rgb) {
        self.m.quad(a, b, c, d, col);
    }

    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, col: Rgb) {
        self.m.tri(a, b, c, col);
    }

    /// Marks everything added since `from` (in vertices) as lighting up at night.
    fn night_from(&mut self, from: usize) {
        for t in (from / 3)..(self.m.verts.len() / 3) {
            self.night.push(t);
        }
    }

    /// Marks everything added since `from` (in vertices) as glowing.
    fn glow_from(&mut self, from: usize) {
        for t in (from / 3)..(self.m.verts.len() / 3) {
            self.glow.push(t);
        }
    }

    /// Four walls of a box from `y0` to `y1`, `w` wide on x and `d` deep on z.
    fn walls(&mut self, w: f32, d: f32, y0: f32, y1: f32, col: Rgb) {
        let (x, z) = (w * 0.5, d * 0.5);
        let c = |px: f32, py: f32, pz: f32| Vec3::new(px, py, pz);
        self.quad(c(-x, y0, z), c(x, y0, z), c(x, y1, z), c(-x, y1, z), col);
        self.quad(
            c(x, y0, -z),
            c(-x, y0, -z),
            c(-x, y1, -z),
            c(x, y1, -z),
            col,
        );
        self.quad(c(x, y0, z), c(x, y0, -z), c(x, y1, -z), c(x, y1, z), col);
        self.quad(
            c(-x, y0, -z),
            c(-x, y0, z),
            c(-x, y1, z),
            c(-x, y1, -z),
            col,
        );
    }

    fn lid(&mut self, w: f32, d: f32, y: f32, col: Rgb) {
        let (x, z) = (w * 0.5, d * 0.5);
        self.quad(
            Vec3::new(-x, y, z),
            Vec3::new(x, y, z),
            Vec3::new(x, y, -z),
            Vec3::new(-x, y, -z),
            col,
        );
    }

    /// A solid box.
    fn cube(&mut self, c: Vec3, half: Vec3, col: Rgb) {
        self.m.cube(c, half, col);
    }

    /// A gable roof over walls `w` by `d` at height `h`, ridge along x, `o` overhang.
    fn gable(&mut self, w: f32, d: f32, h: f32, rise: f32, o: f32, roof: Rgb, end: Rgb) {
        let (x, z) = (w * 0.5 + o, d * 0.5 + o);
        let top = h + rise;
        let eave = h - o * rise / (d * 0.5);
        let v = Vec3::new;
        self.quad(
            v(-x, eave, z),
            v(x, eave, z),
            v(x, top, 0.0),
            v(-x, top, 0.0),
            roof,
        );
        self.quad(
            v(x, eave, -z),
            v(-x, eave, -z),
            v(-x, top, 0.0),
            v(x, top, 0.0),
            roof,
        );
        let (wx, wz) = (w * 0.5, d * 0.5);
        self.tri(v(wx, h, wz), v(wx, h, -wz), v(wx, top, 0.0), end);
        self.tri(v(-wx, h, -wz), v(-wx, h, wz), v(-wx, top, 0.0), end);
    }

    /// A hipped roof: four slopes up to a short ridge along x.
    #[allow(clippy::too_many_arguments)]
    fn hip(&mut self, w: f32, d: f32, h: f32, rise: f32, o: f32, roof: Rgb) {
        let (x, z) = (w * 0.5 + o, d * 0.5 + o);
        let eave = h - o * rise / (d * 0.5);
        let r = (w * 0.5 - d * 0.5).max(0.3);
        let v = Vec3::new;
        let (r0, r1) = (v(-r, h + rise, 0.0), v(r, h + rise, 0.0));
        let c = [
            v(-x, eave, z),
            v(x, eave, z),
            v(x, eave, -z),
            v(-x, eave, -z),
        ];
        self.quad(c[0], c[1], r1, r0, roof);
        self.quad(c[2], c[3], r0, r1, roof);
        self.tri(c[1], c[2], r1, roof);
        self.tri(c[3], c[0], r0, roof);
    }

    /// A pyramid roof to a point.
    fn pyramid(&mut self, w: f32, d: f32, h: f32, rise: f32, o: f32, roof: Rgb) {
        let (x, z) = (w * 0.5 + o, d * 0.5 + o);
        let apex = Vec3::new(0.0, h + rise, 0.0);
        let c = [
            Vec3::new(-x, h - o * 0.4, z),
            Vec3::new(x, h - o * 0.4, z),
            Vec3::new(x, h - o * 0.4, -z),
            Vec3::new(-x, h - o * 0.4, -z),
        ];
        for i in 0..4 {
            self.tri(c[i], c[(i + 1) % 4], apex, roof);
        }
    }

    /// A ring of `n` quads from radius `r0` at `y0` to `r1` at `y1`. A zero `r1` makes a cone.
    fn ring(&mut self, n: u32, y0: f32, r0: f32, y1: f32, r1: f32, col: Rgb) {
        for i in 0..n {
            let a = |k: u32, r: f32, y: f32| {
                let t = k as f32 / n as f32 * TAU + 0.3;
                Vec3::new(t.cos() * r, y, t.sin() * r)
            };
            if r1 > 0.0 {
                self.quad(
                    a(i, r0, y0),
                    a(i + 1, r0, y0),
                    a(i + 1, r1, y1),
                    a(i, r1, y1),
                    col,
                );
            } else {
                self.tri(a(i, r0, y0), a(i + 1, r0, y0), Vec3::new(0.0, y1, 0.0), col);
            }
        }
    }

    /// A flat round cap at height `y`.
    fn disc(&mut self, n: u32, y: f32, r: f32, col: Rgb) {
        for i in 0..n {
            let a = |k: u32| {
                let t = k as f32 / n as f32 * TAU + 0.3;
                Vec3::new(t.cos() * r, y, t.sin() * r)
            };
            self.tri(Vec3::new(0.0, y, 0.0), a(i + 1), a(i), col);
        }
    }

    /// A window or door: a flat panel just off a wall. `face` picks the wall: 0 front,
    /// 1 back, 2 right (+x), 3 left.
    #[allow(clippy::too_many_arguments)]
    fn panel(&mut self, face: u8, w: f32, d: f32, u: f32, y: f32, pw: f32, ph: f32, col: Rgb) {
        let off = 0.07;
        let (x, z) = (w * 0.5 + off, d * 0.5 + off);
        let (hw, y1) = (pw * 0.5, y + ph);
        let v = Vec3::new;
        match face {
            0 => self.quad(
                v(u - hw, y, z),
                v(u + hw, y, z),
                v(u + hw, y1, z),
                v(u - hw, y1, z),
                col,
            ),
            1 => self.quad(
                v(u + hw, y, -z),
                v(u - hw, y, -z),
                v(u - hw, y1, -z),
                v(u + hw, y1, -z),
                col,
            ),
            2 => self.quad(
                v(x, y, u + hw),
                v(x, y, u - hw),
                v(x, y1, u - hw),
                v(x, y1, u + hw),
                col,
            ),
            _ => self.quad(
                v(-x, y, u - hw),
                v(-x, y, u + hw),
                v(-x, y1, u + hw),
                v(-x, y1, u - hw),
                col,
            ),
        }
    }

    /// Rows of windows on the given walls (0 front, 1 back, 2 right, 3 left), one row per
    /// floor.
    #[allow(clippy::too_many_arguments)]
    fn windows(
        &mut self,
        faces: &[u8],
        w: f32,
        d: f32,
        y0: f32,
        h: f32,
        floor: f32,
        col: Rgb,
        glow: bool,
    ) {
        let from = self.m.verts.len();
        let floors = ((h - y0) / floor).floor().clamp(1.0, 14.0) as u32;
        for f in 0..floors {
            let y = y0 + f as f32 * floor + floor * 0.35;
            if y + floor * 0.4 > h {
                break;
            }
            for &face in faces {
                let len = if face < 2 { w } else { d };
                let n = (len / 3.6).floor() as i32;
                for k in 0..n {
                    let u = (k as f32 - (n - 1) as f32 * 0.5) * (len / n as f32);
                    self.panel(face, w, d, u, y, 1.1, floor * 0.42, col);
                }
            }
        }
        if glow {
            self.glow_from(from);
        } else {
            self.night_from(from);
        }
    }

    /// Posts from the base down `depth` at the corners, each two crossed slats.
    fn stilts(&mut self, w: f32, d: f32, depth: f32, col: Rgb) {
        for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let c = Vec3::new(x * (w * 0.5 - 0.4), 0.2, z * (d * 0.5 - 0.4));
            self.post(c, depth, 0.25, col);
        }
    }

    /// A thin post down from `top`, as two crossed slats.
    fn post(&mut self, top: Vec3, len: f32, r: f32, col: Rgb) {
        let low = top - Vec3::Y * len;
        for o in [Vec3::X * r, Vec3::Z * r] {
            self.quad(low - o, low + o, top + o, top - o, col);
        }
    }

    /// A boat hull along x, `l` long and `b` wide, with its waterline at 0.
    fn hull(&mut self, l: f32, b: f32, h: f32, col: Rgb, deck: Rgb) {
        let (x, z) = (l * 0.5, b * 0.5);
        let v = Vec3::new;
        let bow = v(x, h, 0.0);
        let keel = v(x * 0.75, -h * 0.5, 0.0);
        let sides = [
            (
                v(-x, h, z),
                v(x * 0.55, h, z),
                v(x * 0.55, -h * 0.5, z * 0.6),
                v(-x * 0.95, -h * 0.5, z * 0.6),
            ),
            (
                v(x * 0.55, h, -z),
                v(-x, h, -z),
                v(-x * 0.95, -h * 0.5, -z * 0.6),
                v(x * 0.55, -h * 0.5, -z * 0.6),
            ),
        ];
        for (a, b2, c, d) in sides {
            self.quad(a, b2, c, d, col);
        }
        self.quad(
            v(x * 0.55, h, z),
            bow,
            keel,
            v(x * 0.55, -h * 0.5, z * 0.6),
            col,
        );
        self.quad(
            bow,
            v(x * 0.55, h, -z),
            v(x * 0.55, -h * 0.5, -z * 0.6),
            keel,
            col,
        );
        self.quad(
            v(-x, h, -z),
            v(-x, h, z),
            v(-x * 0.95, -h * 0.5, z * 0.6),
            v(-x * 0.95, -h * 0.5, -z * 0.6),
            col,
        );
        self.quad(
            v(-x, h, z),
            v(-x, h, -z),
            v(x * 0.55, h, -z),
            v(x * 0.55, h, z),
            deck,
        );
        self.tri(v(x * 0.55, h, z), v(x * 0.55, h, -z), bow, deck);
    }
}

/// Builds one building into `near` and `far`, in world space.
/// The top of a house's chimney in its own frame, if it has one.
pub fn chimney(b: &Building) -> Option<Vec3> {
    let v = b.seed >> 5;
    let hipped = v % 3 == 1 && b.style != Style::Stone;
    (b.kind == Kind::House && b.roof > 0.0 && !hipped && !b.seed.is_multiple_of(3))
        .then(|| Vec3::new(b.size.x * 0.28, b.size.y + b.roof * 1.25, -b.size.z * 0.18))
}

fn building(b: &Building, terrain: &Terrain, near: &mut MeshBuilder, far: &mut MeshBuilder) {
    let c = colors(b);
    let (w, h, d) = (b.size.x, b.size.y, b.size.z);
    let mut n = Parts::new();
    let mut f = Parts::new();
    let floor = 3.4;
    let glow_windows = b.style == Style::Cavern || b.tech == Tech::Simple;
    match b.kind {
        Kind::House
        | Kind::Row
        | Kind::Warehouse
        | Kind::Hall
        | Kind::Hut
        | Kind::Terminal
        | Kind::Hangar => {
            let o = match b.kind {
                Kind::Hut => 0.9,
                Kind::Hangar | Kind::Terminal => 0.3,
                _ => 0.5,
            };
            let wall = if b.kind == Kind::Hangar {
                STEEL
            } else {
                c.wall
            };
            let c = Colors {
                roof: if b.kind == Kind::Terminal {
                    Rgb(0xE4E6E8)
                } else {
                    c.roof
                },
                ..c
            };
            // Houses differ: hipped roofs, a wing out the back, trim, a plinth, a porch.
            let v = b.seed >> 5;
            let house = b.kind == Kind::House;
            let detached = house && !matches!(b.style, Style::Canal | Style::Seafaring);
            let hipped = house && v % 3 == 1 && b.style != Style::Stone;
            let wing = detached && v % 4 == 2;
            for p in [&mut n, &mut f] {
                p.walls(w, d, 0.0, h, wall);
                if b.roof > 0.0 {
                    if b.kind == Kind::Hut {
                        p.pyramid(w, d, h, b.roof, o, c.roof);
                    } else if hipped {
                        p.hip(w, d, h, b.roof, o, c.roof);
                    } else {
                        p.gable(w, d, h, b.roof, o, c.roof, wall);
                    }
                } else {
                    p.lid(w, d, h, c.roof);
                }
                if wing {
                    // A lower wing out the back, its ridge running away from the house.
                    let (ww, wd, wh) = (w * 0.42, d * 0.7, h * 0.72);
                    let mut part = Parts::new();
                    part.walls(wd, ww, 0.0, wh, wall);
                    part.gable(wd, ww, wh, b.roof * 0.7, o, c.roof, wall);
                    let at = Vec3::new(
                        w * (0.5 - 0.21) * if v.is_multiple_of(2) { 1.0 } else { -1.0 },
                        0.0,
                        -d * 0.5 - wd * 0.5 + 0.3,
                    );
                    p.m.append(
                        &part.m,
                        Mat4::from_translation(at)
                            * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
                    );
                }
            }
            if house && v.is_multiple_of(2) {
                // A band of trim under the eaves.
                n.walls(w + 0.12, d + 0.12, h - 0.35, h, c.trim);
            }
            if house && v % 5 < 2 {
                // A darker stone plinth.
                n.walls(w + 0.1, d + 0.1, 0.0, 0.9, wall.mix(Rgb(0x3A3030), 0.3));
            }
            if detached && v.is_multiple_of(3) {
                // A little porch roof over the door.
                n.cube(
                    Vec3::new(w * 0.18, 2.7, d * 0.5 + 0.9),
                    Vec3::new(1.3, 0.1, 0.9),
                    c.roof,
                );
                n.post(
                    Vec3::new(w * 0.18 - 1.1, 2.7, d * 0.5 + 1.6),
                    2.7,
                    0.1,
                    c.trim,
                );
                n.post(
                    Vec3::new(w * 0.18 + 1.1, 2.7, d * 0.5 + 1.6),
                    2.7,
                    0.1,
                    c.trim,
                );
            }
            if b.stilts > 0.0 {
                n.stilts(w, d, b.stilts, DARK_WOOD);
                n.cube(
                    Vec3::new(0.0, -0.15, 0.0),
                    Vec3::new(w * 0.5 + 1.0, 0.15, d * 0.5 + 1.2),
                    WOOD,
                );
            }
            match b.kind {
                Kind::Hangar => {
                    n.panel(0, w, d, 0.0, 0.0, w * 0.8, h * 0.8, Rgb(0x5D6873));
                }
                Kind::Terminal => {
                    n.panel(0, w, d, 0.0, 3.0, w * 0.92, h * 0.5, DARK_GLASS);
                    n.panel(1, w, d, 0.0, 3.0, w * 0.92, h * 0.5, DARK_GLASS);
                }
                Kind::Warehouse => {
                    for k in [-1.0, 1.0] {
                        n.panel(0, w, d, k * w * 0.22, 0.0, w * 0.22, h * 0.6, c.door);
                    }
                    n.windows(&[0], w, d, h * 0.65, h, h, c.window, glow_windows);
                }
                Kind::Row => {
                    // Gable to the street: a tall front of windows and a door.
                    n.windows(&[2], w, d, 0.6, h, floor, c.window, glow_windows);
                    n.panel(2, w, d, 0.0, 0.0, 1.4, 2.4, c.door);
                    n.cube(
                        Vec3::new(w * 0.5 + 0.1, h + b.roof * 0.35, 0.0),
                        Vec3::new(0.12, 0.6, d * 0.32),
                        c.trim,
                    );
                }
                _ => {
                    n.windows(&[0, 1], w, d, 0.6, h, floor, c.window, glow_windows);
                    n.panel(0, w, d, w * 0.18, 0.0, 1.3, 2.3, c.door);
                    if let Some(top) = chimney(b) {
                        n.cube(
                            top - Vec3::Y * b.roof * 0.5,
                            Vec3::new(0.5, b.roof * 0.5, 0.5),
                            Rgb(0x9C6B5A),
                        );
                    }
                }
            }
        }
        Kind::Flat => {
            for p in [&mut n, &mut f] {
                p.walls(w, d, 0.0, h, c.wall);
                p.lid(w, d, h, c.roof);
            }
            n.walls(w + 0.3, d + 0.3, h - 0.2, h + 0.5, c.trim);
            // Windows all round, so no side is a blank wall.
            n.windows(&[0, 1, 2, 3], w, d, 0.6, h, floor, c.window, glow_windows);
            n.panel(0, w, d, -w * 0.2, 0.0, 1.4, 2.3, c.door);
            if h > 15.0 {
                // Tall blocks: a trim band every few floors, and a stair hut and water
                // tank on the roof.
                let mut y = floor * 3.0;
                while y < h - 2.0 {
                    n.walls(w + 0.16, d + 0.16, y - 0.12, y + 0.12, c.trim);
                    y += floor * 3.0;
                }
                n.cube(
                    Vec3::new(-w * 0.2, h + 1.3, -d * 0.15),
                    Vec3::new(1.4, 1.3, 1.6),
                    c.wall,
                );
                n.ring(8, h, 1.2, h + 2.6, 1.2, Rgb(0x8B5E3C));
                n.ring(8, h + 2.6, 1.3, h + 3.4, 0.2, Rgb(0x6E4A33));
            }
            if b.seed.is_multiple_of(4) {
                // A little awning over the door.
                n.cube(
                    Vec3::new(-w * 0.2, 2.6, d * 0.5 + 0.8),
                    Vec3::new(1.4, 0.08, 0.8),
                    c.door,
                );
            }
        }
        Kind::Skyscraper => {
            let band = 10.5;
            let bands = (h / band).floor().max(1.0) as u32;
            let step = h / bands as f32;
            let glass = pick(
                &[Rgb(0x5E89A8), Rgb(0x4F7FA0), Rgb(0x6C9DB5), Rgb(0x3F6F8F)],
                b.seed,
                5,
            );
            for k in 0..bands {
                let y = k as f32 * step;
                n.walls(w, d, y, y + step * 0.3, c.wall);
                let from = n.m.verts.len();
                n.walls(w - 0.2, d - 0.2, y + step * 0.3, y + step, glass);
                n.night_from(from);
            }
            n.lid(w, d, h, c.roof);
            n.cube(
                Vec3::new(0.0, h + 1.5, 0.0),
                Vec3::new(w * 0.25, 1.5, d * 0.25),
                c.wall,
            );
            if b.seed.is_multiple_of(3) {
                n.cube(
                    Vec3::new(0.0, h + 9.0, 0.0),
                    Vec3::new(0.25, 6.0, 0.25),
                    STEEL,
                );
            }
            f.walls(w, d, 0.0, h, glass.mix(c.wall, 0.4));
            f.lid(w, d, h, c.roof);
        }
        Kind::Tower => {
            let r = w * 0.5;
            for p in [&mut n, &mut f] {
                p.ring(8, 0.0, r, h, r * 0.92, c.wall);
                p.ring(8, h - 0.2, r * 1.15, h + r * 1.6, 0.0, c.roof);
            }
            let from = n.m.verts.len();
            for k in 0..3 {
                let y = h * (0.3 + k as f32 * 0.22);
                n.panel(0, w * 0.9, w * 0.9, 0.0, y, 1.0, 1.6, c.window);
            }
            if glow_windows {
                n.glow_from(from);
            }
        }
        Kind::Dome => {
            let r = w * 0.5;
            for p in [&mut n, &mut f] {
                p.ring(10, 0.0, r, h, r, c.wall);
                p.ring(10, h, r * 1.04, h + r * 0.55, r * 0.75, c.roof);
                p.ring(10, h + r * 0.55, r * 0.75, h + r * 0.85, 0.0, c.roof);
            }
            n.panel(0, w * 0.95, w * 0.95, 0.0, 0.0, 1.4, 2.4, c.door);
            let from = n.m.verts.len();
            n.panel(2, w * 0.95, w * 0.95, 0.0, h * 0.45, 1.2, 1.3, c.window);
            n.panel(3, w * 0.95, w * 0.95, 0.0, h * 0.45, 1.2, 1.3, c.window);
            if glow_windows {
                n.glow_from(from);
            }
        }
        Kind::Church => {
            let tx = w * 0.5 + 3.0;
            for p in [&mut n, &mut f] {
                p.walls(w, d, 0.0, h, c.wall);
                p.gable(w, d, h, b.roof, 0.4, c.roof, c.wall);
                p.m.append(
                    &{
                        let mut t = Parts::new();
                        t.walls(6.8, 6.8, 0.0, h * 1.7, c.wall);
                        t.ring(4, h * 1.7, 4.4, h * 2.6, 0.0, c.roof);
                        t.m
                    },
                    Mat4::from_translation(Vec3::new(tx, 0.0, 0.0)),
                );
            }
            n.windows(&[0, 1], w, d, 1.5, h, 6.0, c.window, glow_windows);
            n.panel(2, w + 6.8, 6.8, 0.0, h * 1.25, 1.6, 1.6, WHITE);
        }
        Kind::Lighthouse => {
            let r = w * 0.5;
            let seg = h / 6.0;
            for k in 0..6 {
                let col = if k % 2 == 0 { WHITE } else { RED };
                let (r0, r1) = (
                    r * (1.0 - k as f32 * 0.06),
                    r * (1.0 - (k + 1) as f32 * 0.06),
                );
                n.ring(10, k as f32 * seg, r0, (k + 1) as f32 * seg, r1, col);
            }
            f.ring(8, 0.0, r, h, r * 0.64, WHITE);
            let top = r * 0.64;
            let room = zawnabout_sim::town::LAMP_ROOM;
            for p in [&mut n, &mut f] {
                p.disc(10, h, top + 1.2, Rgb(0x4A4A52));
                p.ring(10, h + room - 0.5, top + 0.6, h + room, top + 0.5, RED);
                p.disc(10, h + room, top + 0.5, RED);
            }
            let from = n.m.verts.len();
            n.ring(10, h, top * 0.8, h + room - 0.5, top * 0.8, LAMP);
            n.glow_from(from);
            f.ring(8, h, top * 0.8, h + room - 0.5, top * 0.8, LAMP);
        }
        Kind::Mast => {
            let r = w * 0.5;
            let seg = h / 8.0;
            for k in 0..8 {
                let col = if k % 2 == 0 { WHITE } else { RED };
                let (r0, r1) = (
                    r * (1.0 - k as f32 * 0.09),
                    r * (1.0 - (k + 1) as f32 * 0.09),
                );
                n.ring(8, k as f32 * seg, r0, (k + 1) as f32 * seg, r1, col);
            }
            f.ring(6, 0.0, r, h, r * 0.28, WHITE);
            let pod = h * 0.72;
            for p in [&mut n, &mut f] {
                p.ring(10, pod - 5.0, r * 0.5, pod, w * 1.4, STEEL);
                p.ring(10, pod, w * 1.4, pod + 6.0, w * 1.3, DARK_GLASS);
                p.ring(10, pod + 6.0, w * 1.3, pod + 9.0, r * 0.4, WHITE);
            }
            n.cube(Vec3::new(0.0, h + 8.0, 0.0), Vec3::new(0.3, 8.0, 0.3), RED);
        }
        Kind::ControlTower => {
            for p in [&mut n, &mut f] {
                p.ring(6, 0.0, w * 0.3, h, w * 0.25, WHITE);
                p.walls(w, w, h, h + 5.0, DARK_GLASS);
                p.lid(w + 1.0, w + 1.0, h + 5.0, WHITE);
            }
        }
        Kind::Crane => {
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, h * 0.5, 0.0),
                    Vec3::new(1.2, h * 0.5, 1.2),
                    Rgb(0xF2B134),
                );
                p.cube(
                    Vec3::new(0.0, h + 1.0, d * 0.35),
                    Vec3::new(0.8, 0.8, d * 0.6),
                    Rgb(0xF2B134),
                );
                p.cube(
                    Vec3::new(0.0, h + 0.5, -d * 0.2),
                    Vec3::new(1.6, 1.6, 2.0),
                    STEEL,
                );
            }
            n.cube(
                Vec3::new(0.0, h * 0.6, d * 0.85),
                Vec3::new(0.08, h * 0.4, 0.08),
                STEEL,
            );
        }
        Kind::Hull => {
            for p in [&mut n, &mut f] {
                p.hull(w, d, h, Rgb(0xC9533F), WOOD);
            }
            for k in -2..=2 {
                n.cube(
                    Vec3::new(k as f32 * w * 0.18, -1.0, 0.0),
                    Vec3::new(0.4, 1.6, d * 0.45),
                    DARK_WOOD,
                );
            }
        }
        Kind::Train if b.seed >= zawnabout_sim::traffic::METRO_CAB as u32 => {
            // A metro car on the rails at the origin, x along the track: brushed steel with
            // a blue band, wide windows lit inside, doors down both sides, and a sloped cab
            // at the leading or trailing end.
            use zawnabout_sim::traffic::{METRO_CAB, METRO_TAIL};
            let (steel, band, door) = (Rgb(0xD9DDE2), Rgb(0x3E7FC4), Rgb(0x8E959E));
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, h * 0.5, 0.0),
                    Vec3::new(w * 0.5, h * 0.36, d * 0.5),
                    steel,
                );
            }
            n.cube(
                Vec3::new(0.0, h * 0.09, 0.0),
                Vec3::new(w * 0.44, h * 0.08, d * 0.4),
                Rgb(0x3A3A40),
            );
            n.cube(
                Vec3::new(0.0, h * 0.25, 0.0),
                Vec3::new(w * 0.5 + 0.02, h * 0.035, d * 0.5 + 0.02),
                band,
            );
            n.cube(
                Vec3::new(0.0, h * 0.87, 0.0),
                Vec3::new(w * 0.46, h * 0.03, d * 0.4),
                Rgb(0xB8BEC6),
            );
            // Lit inside, always: they never leave the tunnels.
            let from = n.m.verts.len();
            for k in [-1.0f32, 0.0, 1.0] {
                n.cube(
                    Vec3::new(k * w * 0.3, h * 0.6, 0.0),
                    Vec3::new(w * 0.11, h * 0.13, d * 0.5 + 0.02),
                    Rgb(0xC9BC98),
                );
            }
            n.m.glow_from(from);
            for k in [-1.0f32, 1.0] {
                n.cube(
                    Vec3::new(k * w * 0.15, h * 0.48, 0.0),
                    Vec3::new(w * 0.035, h * 0.3, d * 0.5 + 0.03),
                    door,
                );
            }
            let cab = match b.seed as u8 {
                METRO_CAB => Some(1.0f32),
                METRO_TAIL => Some(-1.0),
                _ => None,
            };
            if let Some(end) = cab {
                let from = n.m.verts.len();
                n.cube(
                    Vec3::new(end * (w * 0.5 + 0.02), h * 0.62, 0.0),
                    Vec3::new(0.04, h * 0.14, d * 0.4),
                    DARK_GLASS,
                );
                n.night_from(from);
                n.cube(
                    Vec3::new(end * (w * 0.5 + 0.03), h * 0.3, 0.0),
                    Vec3::new(0.04, h * 0.05, d * 0.42),
                    band,
                );
                for z in [-1.0f32, 1.0] {
                    n.cube(
                        Vec3::new(end * (w * 0.5 + 0.03), h * 0.2, z * d * 0.32),
                        Vec3::new(0.04, 0.12, 0.2),
                        Rgb(0xFFF2C0),
                    );
                }
            }
        }
        Kind::Train => {
            // An engine (seed 0) or a carriage, on the rails at the origin, x along the track.
            let body = if b.seed == 0 {
                RED
            } else {
                [Rgb(0xF2E6C8), Rgb(0x4A90C8), Rgb(0x6BBF7A)][(b.seed % 3) as usize]
            };
            let dark = Rgb(0x3A3A40);
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, h * 0.55, 0.0),
                    Vec3::new(w * 0.5, h * 0.38, d * 0.5),
                    body,
                );
                p.cube(
                    Vec3::new(0.0, h * 0.95, 0.0),
                    Vec3::new(w * 0.48, h * 0.05, d * 0.42),
                    Rgb(0xC8C4BC),
                );
            }
            n.cube(
                Vec3::new(0.0, h * 0.12, 0.0),
                Vec3::new(w * 0.46, h * 0.1, d * 0.4),
                dark,
            );
            let from = n.m.verts.len();
            if b.seed == 0 {
                n.cube(
                    Vec3::new(w * 0.3, h * 0.7, 0.0),
                    Vec3::new(w * 0.12, h * 0.12, d * 0.51),
                    DARK_GLASS,
                );
                n.cube(
                    Vec3::new(w * 0.5 + 0.02, h * 0.7, 0.0),
                    Vec3::new(0.04, h * 0.14, d * 0.36),
                    DARK_GLASS,
                );
            } else {
                n.cube(
                    Vec3::new(0.0, h * 0.68, 0.0),
                    Vec3::new(w * 0.44, h * 0.13, d * 0.51),
                    DARK_GLASS,
                );
            }
            n.night_from(from);
            n.cube(
                Vec3::new(0.0, h * 0.45, 0.0),
                Vec3::new(w * 0.5 + 0.02, h * 0.04, d * 0.5 + 0.02),
                WHITE,
            );
        }
        Kind::LiftChair => {
            // Hangs from the cable at the origin: a bar down to a bench for two facing the
            // way it travels, with skiers on most.
            use zawnabout_sim::traffic::CHAIR_DROP as DROP;
            let paint = [RED, Rgb(0x4A90C8), Rgb(0xF2C14E), Rgb(0x6BBF7A)][(b.seed % 4) as usize];
            n.post(Vec3::new(-0.45, 0.0, 0.0), DROP - 0.4, 0.05, STEEL);
            n.cube(
                Vec3::new(-0.45, -0.1, 0.0),
                Vec3::new(0.12, 0.12, 0.3),
                STEEL,
            );
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, -DROP + 0.1, 0.0),
                    Vec3::new(0.45, 0.1, 0.95),
                    paint,
                );
                p.cube(
                    Vec3::new(-0.42, -DROP + 0.6, 0.0),
                    Vec3::new(0.06, 0.5, 0.95),
                    paint,
                );
            }
            n.cube(
                Vec3::new(0.42, -DROP + 0.8, 0.0),
                Vec3::new(0.03, 0.03, 0.95),
                STEEL,
            );
            let riders: &[f32] = match b.seed % 4 {
                0 | 2 => &[-0.45, 0.45],
                1 => &[0.45],
                _ => &[],
            };
            for (k, z) in riders.iter().enumerate() {
                let jacket = [
                    Rgb(0xE0564A),
                    Rgb(0x2F6FD0),
                    Rgb(0xF2C14E),
                    Rgb(0x8E5CC8),
                    Rgb(0x3BB08F),
                ][(b.seed as usize * 3 + k * 2) % 5];
                n.cube(
                    Vec3::new(-0.1, -DROP + 0.55, *z),
                    Vec3::new(0.2, 0.32, 0.2),
                    jacket,
                );
                n.cube(
                    Vec3::new(-0.08, -DROP + 1.05, *z),
                    Vec3::new(0.14, 0.15, 0.14),
                    Rgb(0xF2C8A8),
                );
                n.cube(
                    Vec3::new(-0.08, -DROP + 1.23, *z),
                    Vec3::new(0.15, 0.06, 0.15),
                    jacket.mix(WHITE, 0.5),
                );
                n.cube(
                    Vec3::new(0.3, -DROP + 0.18, *z),
                    Vec3::new(0.25, 0.09, 0.12),
                    Rgb(0x3A3A40),
                );
                n.cube(
                    Vec3::new(0.5, -DROP - 0.3, *z),
                    Vec3::new(0.85, 0.03, 0.07),
                    jacket.mix(Rgb(0x3A3A40), 0.3),
                );
            }
        }
        Kind::LiftCabin => {
            use zawnabout_sim::traffic::CABIN_DROP as DROP;
            let paint = [RED, Rgb(0x4A90C8), Rgb(0xF2C14E), WHITE][(b.seed % 4) as usize];
            n.post(Vec3::ZERO, DROP - 2.8, 0.07, STEEL);
            n.cube(Vec3::new(0.0, -0.1, 0.0), Vec3::new(0.5, 0.18, 0.25), STEEL);
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, -DROP + 0.7, 0.0),
                    Vec3::new(1.6, 0.7, 1.4),
                    paint,
                );
                p.cube(
                    Vec3::new(0.0, -DROP + 2.75, 0.0),
                    Vec3::new(1.6, 0.06, 1.4),
                    paint,
                );
            }
            let from = n.m.verts.len();
            n.cube(
                Vec3::new(0.0, -DROP + 2.05, 0.0),
                Vec3::new(1.55, 0.65, 1.35),
                DARK_GLASS,
            );
            n.night_from(from);
            n.cube(
                Vec3::new(0.0, -DROP + 2.95, 0.0),
                Vec3::new(1.1, 0.14, 0.9),
                WHITE,
            );
        }
        Kind::LiftTower => {
            let dark = Rgb(0x3A3A40);
            for p in [&mut n, &mut f] {
                p.ring(6, 0.0, 0.6, h, 0.42, STEEL);
                p.cube(
                    Vec3::new(0.0, h - 0.05, 0.0),
                    Vec3::new(0.3, 0.25, d * 0.5 + 0.6),
                    STEEL,
                );
            }
            for z in [-0.5f32, 0.5] {
                n.cube(
                    Vec3::new(0.0, h - 0.3, z * d),
                    Vec3::new(0.7, 0.14, 0.12),
                    dark,
                );
            }
            // A maintenance ladder up one side.
            n.cube(
                Vec3::new(-0.62, h * 0.5, 0.0),
                Vec3::new(0.04, h * 0.5, 0.25),
                dark,
            );
        }
        Kind::LiftStation => {
            use zawnabout_sim::traffic::STATION_CABLE;
            let (wheel, dark) = (d * 0.5 - 2.5, Rgb(0x3A3A40));
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, 0.25, 0.0),
                    Vec3::new(w * 0.5, 0.25, d * 0.5),
                    STONE,
                );
                p.cube(
                    Vec3::new(-w * 0.5 + 0.3, h * 0.5, 0.0),
                    Vec3::new(0.3, h * 0.5, d * 0.5),
                    c.wall,
                );
                p.gable(w, d, h, b.roof, 0.6, c.roof, c.wall);
            }
            for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0)] {
                n.post(
                    Vec3::new(x * (w * 0.5 - 0.3), h, z * (d * 0.5 - 0.3)),
                    h,
                    0.25,
                    c.wall,
                );
            }
            // The great wheel the cable turns round.
            n.disc(14, STATION_CABLE + 0.2, wheel, dark);
            n.ring(
                14,
                STATION_CABLE - 0.1,
                wheel,
                STATION_CABLE + 0.2,
                wheel,
                STEEL,
            );
            n.post(Vec3::new(0.0, h, 0.0), h - STATION_CABLE - 0.2, 0.3, dark);
        }
        Kind::Car if b.seed == u32::from(zawnabout_sim::traffic::BUS) => {
            // A town bus: cream and red, a long band of windows, lit inside at night.
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, h * 0.52, 0.0),
                    Vec3::new(w * 0.5, h * 0.42, d * 0.5),
                    Rgb(0xF2E6C8),
                );
                p.cube(
                    Vec3::new(0.0, h * 0.25, 0.0),
                    Vec3::new(w * 0.5 + 0.02, h * 0.12, d * 0.5 + 0.02),
                    RED,
                );
            }
            let from = n.m.verts.len();
            n.cube(
                Vec3::new(0.0, h * 0.66, 0.0),
                Vec3::new(w * 0.46, h * 0.14, d * 0.5 + 0.03),
                DARK_GLASS,
            );
            n.cube(
                Vec3::new(w * 0.5 + 0.02, h * 0.62, 0.0),
                Vec3::new(0.03, h * 0.2, d * 0.4),
                DARK_GLASS,
            );
            n.night_from(from);
            for (x, z) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                n.cube(
                    Vec3::new(x * w * 0.34, 0.4, z * d * 0.46),
                    Vec3::new(0.42, 0.42, 0.14),
                    Rgb(0x3A3A40),
                );
            }
        }
        Kind::Car if b.seed == u32::from(zawnabout_sim::traffic::TRUCK) => {
            // A delivery truck: a cab and a tall white box with a colored stripe.
            let paint = [RED, Rgb(0x4A90C8), Rgb(0x6BBF7A)][(b.seed % 3) as usize];
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(w * 0.36, h * 0.36, 0.0),
                    Vec3::new(w * 0.14, h * 0.32, d * 0.48),
                    paint,
                );
                p.cube(
                    Vec3::new(-w * 0.14, h * 0.56, 0.0),
                    Vec3::new(w * 0.36, h * 0.44, d * 0.5),
                    WHITE,
                );
            }
            n.cube(
                Vec3::new(-w * 0.14, h * 0.5, 0.0),
                Vec3::new(w * 0.36 + 0.02, h * 0.06, d * 0.5 + 0.02),
                paint,
            );
            n.cube(
                Vec3::new(w * 0.5, h * 0.48, 0.0),
                Vec3::new(0.03, h * 0.14, d * 0.4),
                DARK_GLASS,
            );
            for (x, z) in [(0.36, 1.0), (0.36, -1.0), (-0.3, 1.0), (-0.3, -1.0)] {
                n.cube(
                    Vec3::new(x * w, 0.38, z * d * 0.46),
                    Vec3::new(0.38, 0.38, 0.14),
                    Rgb(0x3A3A40),
                );
            }
        }
        Kind::Car => {
            let paint = [
                Rgb(0xE0564A),
                Rgb(0x4A90C8),
                Rgb(0xF2C14E),
                Rgb(0xF6F1E7),
                Rgb(0x6BBF7A),
                Rgb(0xE88AB0),
            ][(b.seed % 6) as usize];
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, h * 0.42, 0.0),
                    Vec3::new(w * 0.5, h * 0.22, d * 0.5),
                    paint,
                );
                p.cube(
                    Vec3::new(-w * 0.06, h * 0.8, 0.0),
                    Vec3::new(w * 0.27, h * 0.18, d * 0.44),
                    paint.mix(Rgb(0xFFFFFF), 0.25),
                );
            }
            for (x, z) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                n.cube(
                    Vec3::new(x * w * 0.32, h * 0.2, z * d * 0.45),
                    Vec3::new(h * 0.2, h * 0.2, 0.12),
                    Rgb(0x3A3A40),
                );
            }
            let from = n.m.verts.len();
            n.cube(
                Vec3::new(w * 0.5, h * 0.45, d * 0.32),
                Vec3::new(0.05, 0.12, 0.18),
                Rgb(0xFFF2C0),
            );
            n.cube(
                Vec3::new(w * 0.5, h * 0.45, -d * 0.32),
                Vec3::new(0.05, 0.12, 0.18),
                Rgb(0xFFF2C0),
            );
            n.night_from(from);
        }
        Kind::Wreck => {
            let mut wreck = Parts::new();
            wreck.hull(w, d, h, Rgb(0x6E5A4A), Rgb(0x5A4A3E));
            wreck.cube(
                Vec3::new(w * 0.1, h + 5.0, 0.0),
                Vec3::new(0.35, 5.0, 0.35),
                DARK_WOOD,
            );
            let tilt = Mat4::from_rotation_x(0.35) * Mat4::from_rotation_z(0.12);
            n.m.append(&wreck.m, tilt);
            f.m.append(&wreck.m, tilt);
        }
        Kind::Boat | Kind::Canoe => {
            let hull_col = pick(
                &[RED, Rgb(0x3F6E8C), WHITE, Rgb(0x5E8C4E), Rgb(0xF2C14E)],
                b.seed,
                7,
            );
            for p in [&mut n, &mut f] {
                p.hull(w, d, h * 0.45, hull_col, WOOD);
            }
            if b.kind == Kind::Boat {
                n.walls(w * 0.25, d * 0.6, h * 0.45, h * 1.3, WHITE);
                n.lid(w * 0.25, d * 0.6, h * 1.3, RED);
                n.cube(
                    Vec3::new(w * 0.15, h * 2.2, 0.0),
                    Vec3::new(0.15, h * 1.2, 0.15),
                    WOOD,
                );
            }
        }
        Kind::Sailboat => {
            for p in [&mut n, &mut f] {
                p.hull(w, d, 0.9, WHITE, WOOD);
            }
            let mast = w * 1.2;
            n.cube(
                Vec3::new(w * 0.1, mast * 0.5 + 0.9, 0.0),
                Vec3::new(0.12, mast * 0.5, 0.12),
                STEEL,
            );
            let v = Vec3::new;
            n.m.thin_tri(
                v(w * 0.12, 1.6, 0.0),
                v(w * 0.12, mast, 0.0),
                v(-w * 0.38, 1.7, 0.0),
                SAIL,
            );
            n.m.thin_tri(
                v(w * 0.1, mast * 0.9, 0.0),
                v(w * 0.48, 1.4, 0.0),
                v(w * 0.12, 1.6, 0.0),
                SAIL,
            );
            f.cube(
                Vec3::new(w * 0.1, mast * 0.5, 0.0),
                Vec3::new(0.2, mast * 0.5, 0.2),
                SAIL,
            );
        }
        Kind::Houseboat => {
            let col = pick(
                &[Rgb(0x5E8C4E), Rgb(0x3F6E8C), RED, Rgb(0xF2C14E)],
                b.seed,
                8,
            );
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, 0.2, 0.0),
                    Vec3::new(w * 0.5, 0.6, d * 0.5),
                    DARK_WOOD,
                );
                p.walls(w * 0.75, d * 0.8, 0.8, h + 0.4, col);
                p.gable(w * 0.75, d * 0.8, h + 0.4, 1.0, 0.3, Rgb(0x4E4A55), col);
            }
            n.windows(
                &[0, 1],
                w * 0.75,
                d * 0.8,
                1.0,
                h + 0.4,
                2.2,
                WARM_GLASS,
                false,
            );
        }
        Kind::Cruise => cruise(&mut n, &mut f),
        Kind::BridgeTower => {
            // Two tapering legs, x across the deck, joined by three portal beams, with red
            // lights on top for the planes.
            let paint = BRIDGE_PAINT;
            let leg = zawnabout_sim::town::TOWER_LEG;
            let spread = (w - leg.x) * 0.5;
            for p in [&mut n, &mut f] {
                for x in [-spread, spread] {
                    let mut l = Parts::new();
                    l.m.loft(
                        -leg.y * 0.5,
                        (leg.x * 0.5, h * 0.5, h * 0.5),
                        leg.y * 0.5,
                        (leg.x * 0.5, h * 0.5, h * 0.5),
                        [paint; 4],
                    );
                    p.m.append(&l.m, Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
                }
                for y in [h * 0.55, h * 0.78, h - 1.5] {
                    p.cube(
                        Vec3::new(0.0, y, 0.0),
                        Vec3::new(spread, 1.5, leg.y * 0.35),
                        paint,
                    );
                }
            }
            let from = n.m.verts.len();
            for x in [-spread, spread] {
                n.cube(Vec3::new(x, h + 0.4, 0.0), Vec3::splat(0.4), Rgb(0xFF5040));
            }
            n.m.glow_from(from);
        }
        Kind::Ferry => {
            for p in [&mut n, &mut f] {
                p.hull(w, d, 2.0, WHITE, Rgb(0x3F6E8C));
                p.walls(w * 0.55, d * 0.8, 2.0, 5.0, WHITE);
                p.lid(w * 0.55, d * 0.8, 5.0, Rgb(0x3F6E8C));
                p.walls(w * 0.3, d * 0.6, 5.0, 7.5, WHITE);
                p.lid(w * 0.3, d * 0.6, 7.5, WHITE);
            }
            n.windows(&[0, 1], w * 0.55, d * 0.8, 2.3, 5.0, 2.7, DARK_GLASS, false);
            n.ring(8, 7.5, 1.4, 11.0, 1.2, RED);
        }
        Kind::Balloon => {
            // Striped envelope over a wicker basket on ropes.
            let palettes = [
                [RED, Rgb(0xF2C14E)],
                [Rgb(0x4A90C8), WHITE],
                [Rgb(0x6BBF7A), Rgb(0xF6E27A)],
                [Rgb(0x9A7FD1), Rgb(0xF29E4C)],
            ];
            let stripes = palettes[(b.seed % 4) as usize];
            let shape = [
                (6.0, 1.2),
                (8.0, 4.6),
                (10.5, 7.0),
                (13.5, 8.0),
                (17.0, 7.3),
                (19.5, 5.2),
                (21.3, 2.4),
                (22.0, 0.0),
            ];
            for (k, pair) in shape.windows(2).enumerate() {
                let ((y0, r0), (y1, r1)) = (pair[0], pair[1]);
                n.ring(12, y0, r0, y1, r1, stripes[k % 2]);
                f.ring(6, y0, r0, y1, r1, stripes[k % 2]);
            }
            let wicker = Rgb(0x9C6B3C);
            for p in [&mut n, &mut f] {
                p.cube(Vec3::new(0.0, 0.7, 0.0), Vec3::new(1.3, 0.7, 1.3), wicker);
            }
            for (x, z) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                n.cube(
                    Vec3::new(x * 1.1, 3.6, z * 1.1),
                    Vec3::new(0.04, 2.3, 0.04),
                    Rgb(0x5A4A3A),
                );
            }
            // The burner, glowing.
            let from = n.m.verts.len();
            n.cube(
                Vec3::new(0.0, 4.6, 0.0),
                Vec3::new(0.35, 0.25, 0.35),
                Rgb(0xFFB45A),
            );
            n.m.glow_from(from);
        }
        Kind::Seaplane | Kind::Airliner => {
            let (l, span) = (w, d);
            let (body, wing) = if b.kind == Kind::Airliner {
                (WHITE, Rgb(0xD9DEE3))
            } else {
                (Rgb(0xF2C14E), WHITE)
            };
            let r = l * 0.065;
            let y = if b.kind == Kind::Airliner { h } else { h * 0.7 };
            for p in [&mut n, &mut f] {
                let mut fus = Parts::new();
                fus.ring(8, -l * 0.5, r * 0.6, -l * 0.35, r, body);
                fus.ring(8, -l * 0.35, r, l * 0.38, r, body);
                fus.ring(8, l * 0.38, r, l * 0.5, 0.0, body);
                p.m.append(
                    &fus.m,
                    Mat4::from_translation(Vec3::new(0.0, y, 0.0))
                        * Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                );
                p.cube(
                    Vec3::new(l * 0.02, y - r * 0.3, 0.0),
                    Vec3::new(l * 0.07, 0.25, span * 0.5),
                    wing,
                );
                p.cube(
                    Vec3::new(-l * 0.43, y + r * 0.2, 0.0),
                    Vec3::new(l * 0.04, 0.2, span * 0.17),
                    wing,
                );
                p.cube(
                    Vec3::new(-l * 0.44, y + r * 1.8, 0.0),
                    Vec3::new(l * 0.05, r * 1.4, 0.15),
                    Rgb(0x3F6E8C),
                );
            }
            if b.kind == Kind::Seaplane {
                for s in [-1.0, 1.0] {
                    n.cube(
                        Vec3::new(0.0, 0.4, s * span * 0.18),
                        Vec3::new(l * 0.3, 0.4, 0.45),
                        WHITE,
                    );
                    n.cube(
                        Vec3::new(0.0, y * 0.45, s * span * 0.18),
                        Vec3::new(0.1, y * 0.4, 0.1),
                        STEEL,
                    );
                }
            } else {
                for s in [-1.0, 1.0] {
                    n.ring(6, y - r * 1.4, r * 0.35, y - r * 0.4, r * 0.35, STEEL);
                    n.cube(
                        Vec3::new(l * 0.1, y - r * 1.0, s * span * 0.2),
                        Vec3::new(l * 0.06, r * 0.35, r * 0.35),
                        STEEL,
                    );
                }
            }
        }
        Kind::Pier => {
            let deck = h;
            for p in [&mut n, &mut f] {
                p.cube(
                    Vec3::new(0.0, deck - 0.3, 0.0),
                    Vec3::new(w * 0.5, 0.3, d * 0.5),
                    WOOD,
                );
            }
            let posts = (w / 8.0).floor() as i32;
            for k in 0..=posts {
                for s in [-1.0, 1.0] {
                    let x = -w * 0.5 + k as f32 * w / posts.max(1) as f32;
                    n.cube(
                        Vec3::new(x, deck * 0.5 - b.stilts * 0.5, s * (d * 0.5 - 0.3)),
                        Vec3::new(0.25, (deck + b.stilts) * 0.5, 0.25),
                        DARK_WOOD,
                    );
                }
            }
        }
        Kind::Footbridge => {
            let (x, top) = (w * 0.5, h);
            let rise = 1.4;
            let v = Vec3::new;
            let col = if b.style == Style::Modern {
                STEEL
            } else {
                STONE
            };
            for p in [&mut n, &mut f] {
                for s in [-1.0, 1.0] {
                    let z = s * d * 0.5;
                    p.quad(
                        v(-x, 0.0, z),
                        v(-x * 0.4, top + rise, z),
                        v(x * 0.4, top + rise, z),
                        v(x, 0.0, z),
                        col,
                    );
                }
                p.quad(
                    v(-x, top - 0.6, -d * 0.5),
                    v(-x, top - 0.6, d * 0.5),
                    v(-x * 0.4, top + rise, d * 0.5),
                    v(-x * 0.4, top + rise, -d * 0.5),
                    PAVING,
                );
                p.quad(
                    v(-x * 0.4, top + rise, -d * 0.5),
                    v(-x * 0.4, top + rise, d * 0.5),
                    v(x * 0.4, top + rise, d * 0.5),
                    v(x * 0.4, top + rise, -d * 0.5),
                    PAVING,
                );
                p.quad(
                    v(x * 0.4, top + rise, -d * 0.5),
                    v(x * 0.4, top + rise, d * 0.5),
                    v(x, top - 0.6, d * 0.5),
                    v(x, top - 0.6, -d * 0.5),
                    PAVING,
                );
            }
        }
        Kind::Stones
        | Kind::Turbine
        | Kind::Campsite
        | Kind::Picnic
        | Kind::PhoneBox
        | Kind::Umbrellas
        | Kind::Scarecrow
        | Kind::Telescope
        | Kind::DriveIn
        | Kind::Sign
        | Kind::Ferris
        | Kind::Swing
        | Kind::Maze
        | Kind::Sandcastle
        | Kind::Snowman
        | Kind::Bench
        | Kind::Gnomes
        | Kind::Beehives
        | Kind::Pumpkins
        | Kind::BusStop
        | Kind::WaterTower
        | Kind::HotSpring
        | Kind::Chess
        | Kind::Cairns
        | Kind::Treehouse
        | Kind::Well
        | Kind::Lemonade
        | Kind::Igloo
        | Kind::Laundry
        | Kind::Sundial
        | Kind::Circus
        | Kind::Fountain
        | Kind::Postbox
        | Kind::Bandstand
        | Kind::Stadium
        | Kind::Waterpark
        | Kind::ThemePark
        | Kind::Zoo
        | Kind::Statue
        | Kind::Mall
        | Kind::Grocery
        | Kind::Park
        | Kind::Station
        | Kind::Metro
        | Kind::MetroGate
        | Kind::SpanTower
        | Kind::Market
        | Kind::Observatory
        | Kind::Lookout
        | Kind::SurfShack
        | Kind::RopeBridge
        | Kind::Hospital
        | Kind::FireStation
        | Kind::School
        | Kind::TownHall
        | Kind::Farm
        | Kind::Port => {
            landmark(b, &mut n, &mut f);
        }
        Kind::Lantern => {
            n.post(Vec3::new(0.0, h, 0.0), h, 0.12, DARK_WOOD);
            let from = n.m.verts.len();
            n.ring(4, h - 0.2, 0.4, h + 0.7, 0.0, LAMP);
            n.ring(4, h - 0.2, 0.4, h - 0.7, 0.0, LAMP);
            n.glow_from(from);
            let from = f.m.verts.len();
            f.cube(Vec3::new(0.0, h + 0.3, 0.0), Vec3::new(0.5, 0.5, 0.5), LAMP);
            f.glow_from(from);
        }
        Kind::Windmill => {
            let r = w * 0.5;
            for p in [&mut n, &mut f] {
                p.ring(8, 0.0, r, h, r * 0.7, WHITE);
                p.ring(8, h, r * 0.8, h + r * 0.9, 0.0, c.roof);
            }
            let hub = Vec3::new(0.0, h * 0.85, r * 0.8);
            for k in 0..4 {
                let (s, co) = (k as f32 * std::f32::consts::FRAC_PI_2 + 0.4).sin_cos();
                let tip = hub + Vec3::new(co, s, 0.0) * h * 0.75;
                let side = Vec3::new(-s, co, 0.0) * 1.2;
                n.m.thin_tri(hub, tip, tip + side, SAIL);
                n.m.thin_tri(hub, tip + side, hub + side * 0.3, SAIL);
                f.m.thin_tri(hub, tip, tip + side, SAIL);
            }
        }
    }
    // Into the world: local x along the axis, local z the front.
    let a = b.axis;
    let m = Mat4::from_cols(
        Vec4::new(a.x, 0.0, a.y, 0.0),
        Vec4::Y,
        Vec4::new(-a.y, 0.0, a.x, 0.0),
        b.pos.extend(1.0),
    );
    let day = (terrain.daylight(b.pos + Vec3::Y * 3.0) * 255.0) as u8;
    for (parts, out) in [(n, near), (f, far)] {
        let start = out.verts.len();
        out.append(&parts.m, m);
        for v in &mut out.verts[start..] {
            v.color[3] = day;
        }
        for t in parts.glow {
            for v in &mut out.verts[start + t * 3..start + t * 3 + 3] {
                v.normal[3] = -127;
            }
        }
        for t in parts.night {
            for v in &mut out.verts[start + t * 3..start + t * 3 + 3] {
                v.normal[3] = -64;
            }
        }
    }
}

/// A vehicle's mesh at the origin, its length along local x, for drawing it as it moves.
pub fn vehicle(terrain: &Terrain, kind: Kind, size: Vec3, seed: u32) -> MeshBuilder {
    let b = Building {
        id: 0,
        kind,
        style: Style::Plain,
        tech: Tech::Modern,
        pos: Vec3::ZERO,
        axis: Vec2::X,
        size,
        roof: 0.0,
        stilts: 0.0,
        seed,
        site: None,
        text: "",
    };
    let (mut near, mut far) = (MeshBuilder::new(), MeshBuilder::new());
    building(&b, terrain, &mut near, &mut far);
    // Daylight comes per instance instead.
    for v in &mut near.verts {
        v.color[3] = 255;
    }
    near
}

/// The red-orange of suspension bridge towers and cables.
pub const BRIDGE_PAINT: Rgb = Rgb(0xC8553D);

/// A cruise ship, bow toward +x, the waterline at the origin: a navy hull with a red
/// boot top, white tiers of cabins with balconies stepping back, a glass-fronted bridge,
/// lifeboats along both sides, two red funnels, and a pool deck with loungers and a slide.
fn cruise(n: &mut Parts, f: &mut Parts) {
    use zawnabout_sim::traffic::{CRUISE_DECKS, CRUISE_FUNNELS};
    let navy = Rgb(0x23365C);
    let teak = Rgb(0xC9A27A);
    let pool = Rgb(0x5CC8E8);
    let funnel = Rgb(0xD8423A);
    let (l, hull_h, beam) = (
        CRUISE_DECKS[0].1 * 2.0,
        CRUISE_DECKS[0].3,
        CRUISE_DECKS[0].4 * 2.0,
    );
    for p in [&mut *n, &mut *f] {
        p.hull(l, beam, hull_h, navy, teak);
    }
    // The red below the waterline peeks out as a stripe along the sides.
    n.cube(
        Vec3::new(-l * 0.1, 0.4, 0.0),
        Vec3::new(l * 0.36, 0.5, beam * 0.43),
        RED,
    );
    for &(x, hx, y, h, hz) in &CRUISE_DECKS[1..] {
        let mut tier = Parts::new();
        tier.walls(hx * 2.0, hz * 2.0, 0.0, h, WHITE);
        tier.lid(hx * 2.0, hz * 2.0, h, teak);
        let mut far = Parts::new();
        far.walls(hx * 2.0, hz * 2.0, 0.0, h, WHITE);
        far.lid(hx * 2.0, hz * 2.0, h, teak);
        // A row of cabins every deck, most lit at night, and balcony rails along them.
        tier.windows(
            &[0, 1, 2, 3],
            hx * 2.0,
            hz * 2.0,
            0.4,
            h,
            3.0,
            DARK_GLASS,
            false,
        );
        let decks = (h / 3.0) as u32;
        for k in 1..decks {
            for z in [-1.0f32, 1.0] {
                tier.cube(
                    Vec3::new(0.0, k as f32 * 3.0, z * (hz + 0.4)),
                    Vec3::new(hx - 1.0, 0.08, 0.4),
                    WHITE,
                );
            }
        }
        let at = Mat4::from_translation(Vec3::new(x, y, 0.0));
        n.m.append(&tier.m, at);
        f.m.append(&far.m, at);
        for t in tier.night {
            n.night
                .push(t + n.m.verts.len() / 3 - tier.m.verts.len() / 3);
        }
    }
    // The bridge: a glass front across the full beam, wings out over the sides.
    let (bx, _, by, bh, _) = CRUISE_DECKS[2];
    let front = bx + CRUISE_DECKS[2].1;
    n.cube(
        Vec3::new(front - 3.0, by + bh * 0.6, 0.0),
        Vec3::new(3.0, bh * 0.4, beam * 0.52),
        WHITE,
    );
    n.cube(
        Vec3::new(front + 0.05, by + bh * 0.62, 0.0),
        Vec3::new(0.08, bh * 0.18, beam * 0.5),
        DARK_GLASS,
    );
    // Lifeboats hung along both sides of the first tier.
    let (lx, lhx, ly, _, lhz) = CRUISE_DECKS[1];
    for z in [-1.0f32, 1.0] {
        for k in 0..12 {
            let x = lx - lhx + 14.0 + k as f32 * ((lhx * 2.0 - 28.0) / 11.0);
            n.cube(
                Vec3::new(x, ly + 2.4, z * (lhz + 1.4)),
                Vec3::new(3.6, 1.0, 1.1),
                Rgb(0xF29E4C),
            );
            n.cube(
                Vec3::new(x, ly + 3.5, z * (lhz + 1.4)),
                Vec3::new(3.2, 0.2, 1.0),
                WHITE,
            );
        }
    }
    // The pool deck on top of the last cabin tier, aft of the sky deck.
    let (_, _, py, ph, phz) = CRUISE_DECKS[2];
    let top = py + ph;
    for (x, hx) in [(-10.0f32, 12.0f32), (12.0, 7.0)] {
        n.cube(
            Vec3::new(x, top + 0.05, 0.0),
            Vec3::new(hx, 0.08, phz * 0.42),
            pool,
        );
    }
    for k in 0..14 {
        let x = -30.0 + k as f32 * 3.6;
        for z in [-1.0f32, 1.0] {
            n.cube(
                Vec3::new(x, top + 0.3, z * phz * 0.75),
                Vec3::new(0.9, 0.2, 0.35),
                [WHITE, Rgb(0x4A90C8), Rgb(0xF2C14E)][k % 3],
            );
        }
    }
    // A slide spiralling down beside the funnels into the pool.
    let mut prev = Vec3::new(-62.0, top + 12.0, phz * 0.5);
    for k in 1..=24 {
        let u = k as f32 / 24.0;
        let a = u * TAU * 1.25;
        let at = Vec3::new(
            -62.0 + u * 40.0 + a.cos() * 4.0,
            top + 12.0 * (1.0 - u) + 0.6,
            phz * 0.5 + a.sin() * 4.0,
        );
        let mut seg = Parts::new();
        let len = at.distance(prev);
        seg.ring(6, 0.0, 0.8, len, 0.8, Rgb(0x6BBF7A));
        let rot = Quat::from_rotation_arc(Vec3::Y, (at - prev).normalize_or(Vec3::Y));
        n.m.append(&seg.m, Mat4::from_rotation_translation(rot, prev));
        prev = at;
    }
    n.post(Vec3::new(-62.0, top + 12.0, phz * 0.5), 12.0, 0.4, WHITE);
    // The funnels, red with black tops, and a mast forward with a light.
    for x in CRUISE_FUNNELS {
        for p in [&mut *n, &mut *f] {
            let mut fun = Parts::new();
            fun.ring(10, 0.0, 5.0, 12.0, 4.4, funnel);
            fun.ring(10, 12.0, 4.4, 14.0, 4.3, Rgb(0x2A2A2E));
            p.m.append(&fun.m, Mat4::from_translation(Vec3::new(x, top, 0.0)));
        }
    }
    let (sx, shx, sy, sh, _) = CRUISE_DECKS[3];
    n.post(
        Vec3::new(sx + shx * 0.5, sy + sh + 12.0, 0.0),
        12.0,
        0.3,
        WHITE,
    );
    let from = n.m.verts.len();
    n.cube(
        Vec3::new(sx + shx * 0.5, sy + sh + 12.3, 0.0),
        Vec3::splat(0.4),
        Rgb(0xFFF2C0),
    );
    n.m.glow_from(from);
}

/// A landmark's pieces, in its own frame. Small loose pieces are left off the far mesh.
fn landmark(b: &Building, n: &mut Parts, f: &mut Parts) {
    use zawnabout_sim::landmark::{Form, Mat as M, pieces};
    let canvas = [
        Rgb(0xE0564A),
        Rgb(0x4A90C8),
        Rgb(0xF2C14E),
        Rgb(0x6BBF7A),
        Rgb(0xE88AB0),
        Rgb(0xF29E4C),
    ];
    for piece in pieces(b.kind, b.seed, b.text) {
        let col = match piece.mat {
            M::Stone => Rgb(0xA8A196),
            M::Wood => Rgb(0x8B5E3C),
            M::Canvas(i) => canvas[i as usize % canvas.len()],
            M::Metal => Rgb(0x8C949C),
            M::White => Rgb(0xF2F2EE),
            M::Red => Rgb(0xD8322A),
            M::Glass => Rgb(0xBFE3F0),
            M::Hay => Rgb(0xE3C567),
            M::Dark => Rgb(0x3A3A40),
            M::Screen => Rgb(0xF4F4F0),
            M::Hedge => Rgb(0x3F7A3A),
            M::Sand => Rgb(0xE8D49A),
            M::Snow => Rgb(0xF6F8FA),
            M::Leaf => Rgb(0x5FA046),
            M::Water => Rgb(0x7FD6E0),
            M::Skin => Rgb(0xF2C8A8),
            M::Lamp => Rgb(0xFFD27A),
            M::Grass => Rgb(0x6BBF5A),
            M::Copper => Rgb(0x6FB59B),
            M::Concrete => Rgb(0xC8C4BC),
            M::Asphalt => Rgb(0x6F6E73),
        };
        let lit = matches!(piece.mat, M::Glass | M::Screen | M::Water | M::Lamp);
        let mut one = Parts::new();
        let place = match piece.form {
            Form::Box {
                at,
                half,
                yaw,
                roof,
            } => {
                if roof > 0.0 {
                    one.walls(half.x * 2.0, half.z * 2.0, 0.0, half.y, col);
                    one.gable(half.x * 2.0, half.z * 2.0, half.y, roof, 0.1, col, col);
                } else {
                    one.cube(
                        Vec3::new(0.0, half.y * 0.5, 0.0),
                        Vec3::new(half.x, half.y * 0.5, half.z),
                        col,
                    );
                }
                Mat4::from_translation(at) * Mat4::from_rotation_y(yaw)
            }
            Form::Round { at, h, r0, r1 } => {
                one.ring(10, 0.0, r0, h, r1, col);
                if r1 > 0.05 {
                    one.disc(10, h, r1, col);
                }
                if r0 > 0.05 {
                    one.disc(10, 0.0, r0, col);
                }
                Mat4::from_translation(at)
            }
            Form::Ball { at, r } => {
                one.m
                    .blob(Vec3::ZERO, Vec3::splat(r), r > 1.5, 0.06, 7, col);
                Mat4::from_translation(at)
            }
        };
        let add = |p: &mut Parts| {
            let from = p.m.verts.len();
            p.m.append(&one.m, place);
            // Lamps underground are lit all day.
            if b.kind == Kind::Metro && matches!(piece.mat, M::Lamp) {
                p.m.glow_from(from);
            } else if lit {
                p.night_from(from);
            }
        };
        add(n);
        if piece.solid {
            add(f);
        }
    }
}

/// Road surfaces as strips laid on the ground, decks and piers where they bridge water,
/// and runway markings.
fn roads(terrain: &Terrain, hung: &[Suspension], tiles: &mut HashMap<(i32, i32), Tile>) {
    let def = terrain.def;
    for (ri, r) in def.roads.iter().enumerate() {
        // Stretches held up by a suspension bridge's cables need no piers.
        let spans: Vec<[f32; 2]> = hung
            .iter()
            .flat_map(|b| b.spans.iter())
            .filter(|(road, _)| *road == ri)
            .map(|(_, span)| *span)
            .collect();
        let prof = terrain.road_points(ri);
        let n = r.pts.len();
        let half = r.w * 0.5;
        let at = |i: usize| Vec2::new(r.pts[i][0], r.pts[i][1]);
        let side = |i: usize| {
            let (a, b) = (at(i.saturating_sub(1)), at((i + 1).min(n - 1)));
            let d = (b - a).normalize_or_zero();
            Vec2::new(-d.y, d.x)
        };
        // Runways are long, so split them into short pieces that follow the ground.
        let mut pts: Vec<(Vec2, f32, bool, Vec2)> = Vec::new();
        for i in 0..n - 1 {
            let (a, b) = (at(i), at(i + 1));
            let steps = ((b - a).length() / 10.0).ceil().max(1.0) as usize;
            for k in 0..steps {
                let t = k as f32 / steps as f32;
                let y = prof[i].y + (prof[i + 1].y - prof[i].y) * t;
                let bridge = prof[i].bridge && prof[i + 1].bridge;
                pts.push((
                    a.lerp(b, t),
                    y,
                    bridge,
                    side(i).lerp(side(i + 1), t).normalize_or_zero(),
                ));
            }
        }
        pts.push((at(n - 1), prof[n - 1].y, prof[n - 1].bridge, side(n - 1)));
        let surface = if r.runway {
            ASPHALT
        } else if r.rail {
            Rgb(0x9A8F80)
        } else {
            Rgb(0x8A8790)
        };
        let mut along = 0.0;
        for w in pts.windows(2) {
            let ((a, ya, ba, sa), (b, yb, bb, sb)) = (w[0], w[1]);
            let lift = 0.25;
            // On a bridge or in a tunnel, the road's own height, else the ground under it.
            let y = |p: Vec2, road_y: f32, bridge: bool| {
                let ground = terrain.height(p.x, p.y);
                let tunnel =
                    ground > road_y + 6.0 && terrain.sdf(Vec3::new(p.x, road_y + 2.0, p.y)) > 0.0;
                if bridge || tunnel {
                    road_y + if tunnel { lift } else { 0.0 }
                } else {
                    ground + lift
                }
            };
            let (la, ra) = (a + sa * half, a - sa * half);
            let (lb, rb) = (b + sb * half, b - sb * half);
            let v3 = |p: Vec2, yy: f32| Vec3::new(p.x, yy, p.y);
            let corners = [
                v3(la, y(la, ya, ba)),
                v3(ra, y(ra, ya, ba)),
                v3(rb, y(rb, yb, bb)),
                v3(lb, y(lb, yb, bb)),
            ];
            let key = tile_key(a);
            let tile = tiles.entry(key).or_insert_with(|| Tile {
                near: MeshBuilder::new(),
                far: MeshBuilder::new(),
                cave: false,
            });
            for out in [&mut tile.near, &mut tile.far] {
                out.quad(corners[0], corners[1], corners[2], corners[3], surface);
            }
            let seg = b.distance(a);
            let highway = !r.runway && !r.rail && r.w >= 14.0;
            if r.rail {
                // Sleepers across the ballast, and two steel rails along them.
                let flat = |p: Vec2, s: Vec2, off: f32, rise: f32, yy: f32, bb: bool| {
                    let q = p + s * off;
                    v3(q, y(q, yy, bb) + rise)
                };
                let steps = (b.distance(a) / 3.0).ceil().max(1.0) as usize;
                for k in 0..steps {
                    let t = k as f32 / steps as f32;
                    let (p, s) = (a.lerp(b, t), sa.lerp(sb, t).normalize_or_zero());
                    let along = Vec2::new(s.y, -s.x) * 0.3;
                    let yy = ya + (yb - ya) * t;
                    tile.near.quad(
                        flat(p - along, s, 1.3, 0.08, yy, ba),
                        flat(p - along, s, -1.3, 0.08, yy, ba),
                        flat(p + along, s, -1.3, 0.08, yy, ba),
                        flat(p + along, s, 1.3, 0.08, yy, ba),
                        Rgb(0x6E4A33),
                    );
                }
                for off in [0.75f32, -0.75] {
                    {
                        let (lo, hi) = (0.12f32, 0.24f32);
                        let (pa, pb) = (flat(a, sa, off, hi, ya, ba), flat(b, sb, off, hi, yb, bb));
                        let w3 = |s: Vec2| Vec3::new(s.x, 0.0, s.y) * 0.06;
                        tile.near
                            .quad(pa + w3(sa), pa - w3(sa), pb - w3(sb), pb + w3(sb), STEEL);
                        tile.near.quad(
                            pa + w3(sa),
                            pb + w3(sb),
                            pb + w3(sb) - Vec3::Y * (hi - lo),
                            pa + w3(sa) - Vec3::Y * (hi - lo),
                            STEEL,
                        );
                    }
                }
            }
            if highway {
                // Solid edge lines, a double yellow line down the middle, and low concrete
                // barriers along both sides.
                let line = |out: &mut MeshBuilder, off: f32, width: f32, col: Rgb| {
                    let (pa, pb) = (a + sa * off, b + sb * off);
                    let (oa, ob) = (
                        Vec3::new(sa.x, 0.0, sa.y) * width,
                        Vec3::new(sb.x, 0.0, sb.y) * width,
                    );
                    let (ca, cb) = (v3(pa, y(pa, ya, ba) + 0.03), v3(pb, y(pb, yb, bb) + 0.03));
                    out.quad(ca + oa, ca - oa, cb - ob, cb + ob, col);
                };
                for off in [half - 0.9, -(half - 0.9)] {
                    line(&mut tile.near, off, 0.15, WHITE);
                }
                for off in [0.25, -0.25] {
                    line(&mut tile.near, off, 0.1, Rgb(0xF2C14E));
                }
                if !(ba && bb) {
                    for (p, q) in [(corners[0], corners[3]), (corners[1], corners[2])] {
                        tile.near
                            .quad(p, q, q + Vec3::Y * 0.7, p + Vec3::Y * 0.7, Rgb(0xC8C4BC));
                    }
                }
            }
            // Dashes down the middle.
            if !highway && !r.rail && (along / 12.0) as i32 % 2 == 0 {
                let cw = if r.runway { 0.8 } else { 0.2 };
                let (ca, cb) = (
                    v3(a, (corners[0].y + corners[1].y) * 0.5 + 0.03),
                    v3(b, (corners[2].y + corners[3].y) * 0.5 + 0.03),
                );
                let (oa, ob) = (
                    Vec3::new(sa.x, 0.0, sa.y) * cw,
                    Vec3::new(sb.x, 0.0, sb.y) * cw,
                );
                tile.near.quad(ca + oa, ca - oa, cb - ob, cb + ob, WHITE);
            }
            if r.runway {
                for s in [-1.0, 1.0] {
                    let (ea, eb) = (a + sa * s * (half - 2.0), b + sb * s * (half - 2.0));
                    let (pa, pb) = (
                        v3(ea, y(ea, ya, false) + 0.03),
                        v3(eb, y(eb, yb, false) + 0.03),
                    );
                    let (oa, ob) = (
                        Vec3::new(sa.x, 0.0, sa.y) * 0.5,
                        Vec3::new(sb.x, 0.0, sb.y) * 0.5,
                    );
                    tile.near.quad(pa + oa, pa - oa, pb - ob, pb + ob, WHITE);
                }
            }
            if ba && bb {
                // Deck sides, rails and a pier at every other step.
                let (dl, dr) = (corners[0] - Vec3::Y * 1.6, corners[1] - Vec3::Y * 1.6);
                let (el, er) = (corners[3] - Vec3::Y * 1.6, corners[2] - Vec3::Y * 1.6);
                let deck = if r.w > 12.0 { Rgb(0xC9C2B8) } else { STONE };
                for out in [&mut tile.near, &mut tile.far] {
                    out.quad(dl, corners[0], corners[3], el, deck);
                    out.quad(corners[1], dr, er, corners[2], deck);
                    out.quad(dl, el, er, dr, deck.mix(Rgb(0x000000), 0.2));
                }
                for (p, q) in [(corners[0], corners[3]), (corners[1], corners[2])] {
                    tile.near.quad(
                        p,
                        q,
                        q + Vec3::Y * 1.1,
                        p + Vec3::Y * 1.1,
                        RED.mix(WHITE, 0.3),
                    );
                }
                let held = spans
                    .iter()
                    .any(|s| along > s[0] + 5.0 && along < s[1] - 5.0);
                if (along / 10.0) as i32 % 4 == 0 && !held {
                    let mid = (a + b) * 0.5;
                    let top = (ya + yb) * 0.5 - 1.6;
                    let ground = terrain.height(mid.x, mid.y).min(top);
                    let floor = ground.min(-4.0).max(ground);
                    let c = Vec3::new(mid.x, (top + floor) * 0.5, mid.y);
                    let half_h = (top - floor) * 0.5;
                    if half_h > 0.5 {
                        for out in [&mut tile.near, &mut tile.far] {
                            out.cube(c, Vec3::new(1.6, half_h, 1.6), deck);
                        }
                    }
                }
            }
            along += seg;
        }
        // A road that ends in a town ends in a paved town square.
        if !r.runway {
            for end in [0, n - 1] {
                let p = at(end);
                let other = at(n - 1 - end);
                let joins = other.distance(p) < r.w
                    || def.roads.iter().enumerate().any(|(oi, o)| {
                        oi != ri
                            && o.pts
                                .iter()
                                .any(|q| Vec2::new(q[0], q[1]).distance(p) < o.w + half)
                    });
                let town = terrain
                    .cover(p.x, p.y)
                    .is_some_and(|c| c.kind == CoverKind::Town);
                if joins || !town || prof[end].bridge {
                    continue;
                }
                let key = tile_key(p);
                let tile = tiles.entry(key).or_insert_with(|| Tile {
                    near: MeshBuilder::new(),
                    far: MeshBuilder::new(),
                    cave: false,
                });
                let s = side(end);
                let f = Vec2::new(s.y, -s.x);
                let size = half * 3.0;
                let corner = |u: f32, v: f32| {
                    let q = p + s * u * size + f * v * size;
                    Vec3::new(q.x, terrain.height(q.x, q.y) + 0.27, q.y)
                };
                for (u, v) in [(-1.0f32, -1.0f32), (0.0, -1.0), (-1.0, 0.0), (0.0, 0.0)] {
                    for out in [&mut tile.near, &mut tile.far] {
                        out.quad(
                            corner(u, v),
                            corner(u + 1.0, v),
                            corner(u + 1.0, v + 1.0),
                            corner(u, v + 1.0),
                            PAVING,
                        );
                    }
                }
            }
        }
        // Stone abutments where each bridge meets the bank, so no deck ends in the air.
        let mut unit = MeshBuilder::new();
        unit.cube(Vec3::Y, Vec3::ONE, STONE);
        for j in 0..pts.len() {
            let (p, y, bridge, s) = pts[j];
            let first = j == 0 || !pts[j - 1].2;
            let last = j + 1 == pts.len() || !pts[j + 1].2;
            if !bridge || !(first || last) {
                continue;
            }
            let top = y - 0.15;
            let bottom = terrain.height(p.x, p.y).min(top - 1.6) - 3.0;
            let along = Vec2::new(s.y, -s.x);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(4.0, (top - bottom) * 0.5, half + 0.9),
                Quat::from_rotation_y((-along.y).atan2(along.x)),
                Vec3::new(p.x, bottom, p.y),
            );
            let tile = tiles.entry(tile_key(p)).or_insert_with(|| Tile {
                near: MeshBuilder::new(),
                far: MeshBuilder::new(),
                cave: false,
            });
            for out in [&mut tile.near, &mut tile.far] {
                out.append(&unit, m);
            }
        }
    }
}

pub fn tile_key(p: Vec2) -> (i32, i32) {
    ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32)
}

/// Every town and road mesh, by tile.
pub fn build(
    terrain: &Terrain,
    buildings: &[Building],
    bridges: &[Suspension],
) -> Vec<((i32, i32), Tile)> {
    let mut tiles: HashMap<(i32, i32), Tile> = HashMap::new();
    let mut caves: HashMap<(i32, i32), Tile> = HashMap::new();
    for b in buildings {
        let key = tile_key(Vec2::new(b.pos.x, b.pos.z));
        let under = b.style == Style::Cavern;
        let map = if under { &mut caves } else { &mut tiles };
        let tile = map.entry(key).or_insert_with(|| Tile {
            near: MeshBuilder::new(),
            far: MeshBuilder::new(),
            cave: under,
        });
        let (near_from, far_from) = (tile.near.verts.len(), tile.far.verts.len());
        building(b, terrain, &mut tile.near, &mut tile.far);
        // Boats moored afloat rock on the swell, all but their lit windows.
        if matches!(
            b.kind,
            Kind::Boat | Kind::Sailboat | Kind::Houseboat | Kind::Canoe
        ) {
            for v in tile.near.verts[near_from..]
                .iter_mut()
                .chain(tile.far.verts[far_from..].iter_mut())
            {
                if v.normal[3] == 0 {
                    v.normal[3] = crate::mesh::BOB;
                }
            }
        }
    }
    roads(terrain, bridges, &mut tiles);
    let mut out: Vec<((i32, i32), Tile)> = tiles.into_iter().chain(caves).collect();
    out.sort_by_key(|(k, t)| (t.cave, *k));
    out
}
