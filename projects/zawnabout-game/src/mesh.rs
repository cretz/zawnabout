//! Hand-built low-poly meshes. Every triangle is flat shaded with its own face normal,
//! so the crude facets are the look.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    /// Snorm8 xyz, then 127 for thin surfaces lit from both sides (wings, petals).
    pub normal: [i8; 4],
    /// sRGB rgba.
    pub color: [u8; 4],
}

/// Vertex flag for scales that catch the light: a gloss with twinkling glints.
pub const SHEEN: i8 = 90;
/// Vertex flag for polished gold, which mirrors the sky.
pub const METAL: i8 = 70;
/// Vertex flag for boats afloat, which rock on the swell. Small, as it also counts as
/// thinness when lit from behind.
pub const BOB: i8 = 20;

/// An sRGB color from a hex literal such as `0xF08A5D`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u32);

impl Rgb {
    pub fn bytes(self) -> [u8; 4] {
        [(self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8, 255]
    }

    /// Mixes toward `other` by `t`.
    pub fn mix(self, other: Rgb, t: f32) -> Rgb {
        let (a, b) = (self.bytes(), other.bytes());
        let c = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u32;
        Rgb((c(0) << 16) | (c(1) << 8) | c(2))
    }
}

#[derive(Clone, Debug, Default)]
pub struct MeshBuilder {
    pub verts: Vec<Vertex>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Rgb) {
        self.tri_flagged(a, b, c, color, 0);
    }

    /// A thin triangle that light shines through, such as a wing membrane.
    pub fn thin_tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Rgb) {
        self.tri_flagged(a, b, c, color, 127);
    }

    fn tri_flagged(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Rgb, flag: i8) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let normal = [
            (n.x * 127.0) as i8,
            (n.y * 127.0) as i8,
            (n.z * 127.0) as i8,
            flag,
        ];
        let color = color.bytes();
        for p in [a, b, c] {
            self.verts.push(Vertex {
                pos: p.to_array(),
                normal,
                color,
            });
        }
    }

    /// A triangle with its own color at each corner, shading smoothly between them.
    pub fn tri_colors(&mut self, a: Vec3, b: Vec3, c: Vec3, colors: [Rgb; 3], flag: i8) {
        let from = self.verts.len();
        self.tri_flagged(a, b, c, colors[0], flag);
        for (v, c) in self.verts[from..].iter_mut().zip(colors) {
            v.color = c.bytes();
        }
    }

    /// Sets the flag of every vertex from `from` on, such as [`SHEEN`].
    pub fn flag_from(&mut self, from: usize, flag: i8) {
        for v in &mut self.verts[from..] {
            v.normal[3] = flag;
        }
    }

    /// Marks every vertex from `from` on as glowing on its own, like a lamp.
    pub fn glow_from(&mut self, from: usize) {
        for v in &mut self.verts[from..] {
            v.normal[3] = -127;
        }
    }

    /// A planar quad `a b c d` in winding order.
    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Rgb) {
        self.tri(a, b, c, color);
        self.tri(a, c, d, color);
    }

    /// Appends another mesh transformed by `m`.
    pub fn append(&mut self, other: &MeshBuilder, m: Mat4) {
        for t in other.verts.as_chunks::<3>().0 {
            let p = |v: &Vertex| m.transform_point3(Vec3::from_array(v.pos));
            let (a, b, c) = (p(&t[0]), p(&t[1]), p(&t[2]));
            let n = (b - a).cross(c - a).normalize_or_zero();
            let flag = t[0].normal[3];
            let normal = [
                (n.x * 127.0) as i8,
                (n.y * 127.0) as i8,
                (n.z * 127.0) as i8,
                flag,
            ];
            for (v, pos) in t.iter().zip([a, b, c]) {
                self.verts.push(Vertex {
                    pos: pos.to_array(),
                    normal,
                    color: v.color,
                });
            }
        }
    }

    /// Appends another mesh moved by `m`, a uniform scale, turn and shift, keeping its
    /// normals as they are (smooth stays smooth).
    pub fn place(&mut self, other: &MeshBuilder, m: Mat4) {
        for v in &other.verts {
            let p = m.transform_point3(Vec3::from_array(v.pos));
            let n = Vec3::new(v.normal[0] as f32, v.normal[1] as f32, v.normal[2] as f32);
            let n = m.transform_vector3(n).normalize_or_zero() * 127.0;
            self.verts.push(Vertex {
                pos: p.to_array(),
                normal: [n.x as i8, n.y as i8, n.z as i8, v.normal[3]],
                color: v.color,
            });
        }
    }

    /// A box between two rectangles along Z: `back` at `z0` and `front` at `z1`, each given
    /// as (half width, half height, center y). Colors are top, sides, bottom, ends.
    pub fn loft(
        &mut self,
        z0: f32,
        back: (f32, f32, f32),
        z1: f32,
        front: (f32, f32, f32),
        colors: [Rgb; 4],
    ) {
        let ring = |z: f32, (hw, hh, cy): (f32, f32, f32)| {
            [
                Vec3::new(hw, cy + hh, z),
                Vec3::new(-hw, cy + hh, z),
                Vec3::new(-hw, cy - hh, z),
                Vec3::new(hw, cy - hh, z),
            ]
        };
        let (b, f) = (ring(z0, back), ring(z1, front));
        let [top, side, bottom, end] = colors;
        self.quad(b[0], b[1], f[1], f[0], top);
        self.quad(b[1], b[2], f[2], f[1], side);
        self.quad(b[2], b[3], f[3], f[2], bottom);
        self.quad(b[3], b[0], f[0], f[3], side);
        self.quad(f[0], f[1], f[2], f[3], end);
        self.quad(b[3], b[2], b[1], b[0], end);
    }

    /// An axis-aligned box.
    pub fn cube(&mut self, center: Vec3, half: Vec3, color: Rgb) {
        self.loft(
            center.z - half.z,
            (half.x, half.y, center.y),
            center.z + half.z,
            (half.x, half.y, center.y),
            [color; 4],
        );
        if center.x != 0.0 {
            let n = self.verts.len();
            for v in &mut self.verts[n - 36..] {
                v.pos[0] += center.x;
            }
        }
    }

    /// A cone or cylinder along +Y from `y0` to `y1` with `sides` faces.
    pub fn frustum(
        &mut self,
        sides: u32,
        y0: f32,
        r0: f32,
        y1: f32,
        r1: f32,
        color: Rgb,
        cap_color: Rgb,
    ) {
        let at = |i: u32, y: f32, r: f32| {
            let a = i as f32 / sides as f32 * std::f32::consts::TAU;
            Vec3::new(a.cos() * r, y, a.sin() * r)
        };
        for i in 0..sides {
            let (a0, a1) = (at(i, y0, r0), at(i + 1, y0, r0));
            let (b0, b1) = (at(i, y1, r1), at(i + 1, y1, r1));
            if r1 > 0.0 {
                self.quad(a0, b0, b1, a1, color);
                self.tri(Vec3::new(0.0, y1, 0.0), b1, b0, cap_color);
            } else {
                self.tri(a0, b0, a1, color);
            }
            self.tri(Vec3::new(0.0, y0, 0.0), a0, a1, cap_color);
        }
    }

    /// A lumpy ball: an icosahedron, optionally subdivided once, with each vertex pushed
    /// in or out by `jitter` from a seed.
    pub fn blob(
        &mut self,
        center: Vec3,
        radius: Vec3,
        subdivide: bool,
        jitter: f32,
        seed: u32,
        color: Rgb,
    ) {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let mut pts: Vec<Vec3> = [
            (-1.0, t, 0.0),
            (1.0, t, 0.0),
            (-1.0, -t, 0.0),
            (1.0, -t, 0.0),
            (0.0, -1.0, t),
            (0.0, 1.0, t),
            (0.0, -1.0, -t),
            (0.0, 1.0, -t),
            (t, 0.0, -1.0),
            (t, 0.0, 1.0),
            (-t, 0.0, -1.0),
            (-t, 0.0, 1.0),
        ]
        .iter()
        .map(|&(x, y, z)| Vec3::new(x, y, z).normalize())
        .collect();
        let mut faces: Vec<[usize; 3]> = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        if subdivide {
            let mut mids = std::collections::HashMap::new();
            let mut mid = |a: usize, b: usize, pts: &mut Vec<Vec3>| {
                *mids.entry((a.min(b), a.max(b))).or_insert_with(|| {
                    pts.push(((pts[a] + pts[b]) * 0.5).normalize());
                    pts.len() - 1
                })
            };
            faces = faces
                .iter()
                .flat_map(|&[a, b, c]| {
                    let (ab, bc, ca) = (
                        mid(a, b, &mut pts),
                        mid(b, c, &mut pts),
                        mid(c, a, &mut pts),
                    );
                    [[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
                })
                .collect();
        }
        let pts: Vec<Vec3> = pts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let h = hash(seed.wrapping_mul(977).wrapping_add(i as u32));
                center + *p * radius * (1.0 + (h - 0.5) * 2.0 * jitter)
            })
            .collect();
        for [a, b, c] in faces {
            self.tri(pts[a], pts[b], pts[c], color);
        }
    }

    /// Mirrors across the YZ plane (left to right).
    pub fn mirrored_x(&self) -> MeshBuilder {
        let mut out = MeshBuilder::new();
        out.append(self, Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
        out
    }

    /// Recolors every vertex of one color to another.
    pub fn recolor(&mut self, from: Rgb, to: Rgb) {
        let (f, t) = (from.bytes(), to.bytes());
        for v in &mut self.verts {
            if v.color == f {
                v.color = t;
            }
        }
    }
}

pub use zawnabout_sim::shape::hash;
