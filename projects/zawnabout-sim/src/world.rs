//! The static world: terrain from the world files plus placed props. Built the same way
//! on every peer, so every peer has the identical world.

use glam::{Quat, Vec3};

use crate::shape::{Frustum, Shape, place, spire_tiers};
use crate::terrain::{CoverKind, Density, Terrain, Trees};

/// Small deterministic PRNG (SplitMix64). Not for anything but placement.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    RoundTree,
    PineTree,
    Bush,
    Rock,
    Spire,
    Flower,
    Cloud,
}

impl PropKind {
    pub const ALL: [PropKind; 7] = [
        PropKind::RoundTree,
        PropKind::PineTree,
        PropKind::Bush,
        PropKind::Rock,
        PropKind::Spire,
        PropKind::Flower,
        PropKind::Cloud,
    ];
}

/// A placed static thing. `id` is stable for a given seed, for saves and networking.
#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub id: u32,
    pub kind: PropKind,
    pub pos: Vec3,
    pub yaw: f32,
    pub scale: f32,
    /// Picks a mesh or color variant.
    pub variant: u8,
}

/// A solid part of a prop.
#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub prop: u32,
    pub shape: Shape,
}

/// A good spot to land, marked for tap-to-land.
#[derive(Clone, Copy, Debug)]
pub struct Perch {
    /// Stable id from the world files, such as `home/summit-perch`.
    pub id: &'static str,
    pub pos: Vec3,
    pub radius: f32,
}

/// Size of a collision grid cell (m).
const CELL: f32 = 40.0;

#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub terrain: Terrain,
    /// The playable square spans `-half_size..half_size` on X and Z.
    pub half_size: f32,
    /// Width of the soft band inside the edge where a breeze turns you back.
    pub edge: f32,
    /// Height where the air starts pushing back down.
    pub sky: f32,
    pub props: Vec<Prop>,
    pub colliders: Vec<Collider>,
    pub perches: Vec<Perch>,
    grid: Vec<Vec<u32>>,
    grid_n: usize,
}

/// Where the tree on top of a spire stands, relative to the spire's cap center.
pub fn spire_tree_offset(cap_radius: f32) -> Vec3 {
    Vec3::new(cap_radius * 0.55, 0.3, 0.0)
}

impl World {
    pub fn generate(seed: u64) -> Self {
        let terrain = Terrain::default();
        let def = terrain.def;
        let mut world = World {
            seed,
            half_size: terrain.half_size(),
            edge: def.edge,
            sky: def.sky,
            terrain,
            props: Vec::new(),
            colliders: Vec::new(),
            perches: Vec::new(),
            grid: Vec::new(),
            grid_n: 0,
        };
        world.place_props();
        world.build_colliders();
        for m in def.marks.iter().filter(|m| m.kind == "perch") {
            let (x, z) = (m.pos[0], m.pos[2]);
            // Perches on or near the ground sit exactly on its rough surface.
            let ground = world.terrain.height(x, z);
            let y = if m.ground || (ground - m.pos[1]).abs() < 10.0 {
                ground
            } else {
                m.pos[1]
            };
            world.perches.push(Perch {
                id: m.id,
                pos: Vec3::new(x, y, z),
                radius: 4.0,
            });
        }
        world
    }

    fn build_colliders(&mut self) {
        for p in &self.props {
            let s = p.scale;
            let mut add = |shape| self.colliders.push(Collider { prop: p.id, shape });
            let trunk = |h: f32| {
                Shape::Frustum(Frustum {
                    base: p.pos,
                    height: h * s,
                    r0: 0.4 * s,
                    r1: 0.28 * s,
                    spin: 0.0,
                })
            };
            match p.kind {
                PropKind::Spire => {
                    let tiers = spire_tiers(p.variant as u32 % 3);
                    let mut cap = tiers[0];
                    for t in &tiers {
                        cap = place(t, p.pos, p.yaw, s);
                        add(Shape::Frustum(cap));
                    }
                    let top = Vec3::new(cap.base.x, cap.top(), cap.base.z);
                    let tree =
                        top + Quat::from_rotation_y(p.yaw) * (spire_tree_offset(cap.r1 / s) * s);
                    add(Shape::Sphere {
                        center: tree + Vec3::Y * 4.6 * s,
                        radius: 2.3 * s,
                    });
                }
                PropKind::RoundTree => {
                    add(trunk(3.2));
                    add(Shape::Sphere {
                        center: p.pos + Vec3::Y * 4.6 * s,
                        radius: 2.3 * s,
                    });
                }
                PropKind::PineTree => {
                    add(trunk(2.0));
                    let tiers = 3 + (p.variant as u32 % 3);
                    add(Shape::Frustum(Frustum {
                        base: p.pos + Vec3::Y * 1.6 * s,
                        height: ((tiers - 1) as f32 * 1.7 + 2.6) * s,
                        r0: 2.2 * s,
                        r1: 0.0,
                        spin: 0.0,
                    }));
                }
                PropKind::Rock => {
                    add(Shape::Sphere {
                        center: p.pos + Vec3::Y * 0.5 * s,
                        radius: 1.1 * s,
                    });
                }
                PropKind::Bush | PropKind::Flower | PropKind::Cloud => {}
            }
        }
        self.grid_n = ((self.half_size * 2.0 + 800.0) / CELL).ceil() as usize;
        self.grid = vec![Vec::new(); self.grid_n * self.grid_n];
        for (i, c) in self.colliders.iter().enumerate() {
            let (center, r) = c.shape.footprint();
            let (x0, z0) = self.cell(center.x - r, center.y - r);
            let (x1, z1) = self.cell(center.x + r, center.y + r);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    self.grid[z * self.grid_n + x].push(i as u32);
                }
            }
        }
    }

    fn cell(&self, x: f32, z: f32) -> (usize, usize) {
        let off = self.half_size + 400.0;
        let f = |v: f32| (((v + off) / CELL).floor().max(0.0) as usize).min(self.grid_n - 1);
        (f(x), f(z))
    }

    /// Colliders that might touch `(x, z)`.
    pub fn near(&self, x: f32, z: f32) -> impl Iterator<Item = &Collider> {
        let (cx, cz) = self.cell(x, z);
        self.grid[cz * self.grid_n + cx]
            .iter()
            .map(|i| &self.colliders[*i as usize])
    }

    /// Height of the highest standable surface at `(x, z)` that is not above `below`.
    /// Water counts as a surface for now, so a dragon can rest on it.
    pub fn support(&self, x: f32, z: f32, below: f32) -> f32 {
        let ground = self.terrain.support(x, z, below);
        let mut best = ground;
        if let Some(w) = self.terrain.water(x, z, ground)
            && w <= below
        {
            best = w;
        }
        for c in self.near(x, z) {
            if let Some(h) = c.shape.support(x, z)
                && h <= below
                && h > best
            {
                best = h;
            }
        }
        best
    }

    /// Pushes a sphere out of every prop it overlaps, ignoring the ground.
    pub fn push_out_props(&self, mut p: Vec3, r: f32) -> (Vec3, Vec3) {
        let mut normal = Vec3::ZERO;
        for c in self.near(p.x, p.z) {
            if let Some((n, depth)) = c.shape.push_out(p, r) {
                p += n * depth;
                normal += n;
            }
        }
        (p, normal.normalize_or_zero())
    }

    /// Pushes a sphere out of the ground and every prop it overlaps. Returns the
    /// corrected center and the summed outward normal (zero when nothing was hit).
    pub fn push_out(&self, mut p: Vec3, r: f32) -> (Vec3, Vec3) {
        let mut normal = Vec3::ZERO;
        if let Some((n, depth)) = self.terrain.push_out(p, r) {
            p += n * depth;
            normal += n;
        }
        for c in self.near(p.x, p.z) {
            if let Some((n, depth)) = c.shape.push_out(p, r) {
                p += n * depth;
                normal += n;
            }
        }
        (p, normal.normalize_or_zero())
    }

    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        self.terrain.height(x, z)
    }

    pub fn ground_normal(&self, x: f32, z: f32) -> Vec3 {
        self.terrain.normal(x, z)
    }

    fn push(&mut self, kind: PropKind, pos: Vec3, scale: f32, variant: u8) {
        let id = self.props.len() as u32;
        let yaw = (id as f32 * 2.399_963).rem_euclid(std::f32::consts::TAU);
        self.props.push(Prop {
            id,
            kind,
            pos,
            yaw,
            scale,
            variant,
        });
    }

    /// The ground height at a dry, gentle enough spot, or none.
    fn dry_spot(&self, x: f32, z: f32, min_up: f32) -> Option<f32> {
        let h = self.terrain.height(x, z);
        if h < 1.8 || self.terrain.water(x, z, h).is_some() {
            return None;
        }
        (self.terrain.normal(x, z).y >= min_up).then_some(h)
    }

    /// Calls `f` at one jittered point per `spacing` square over the cover's bounds.
    fn scatter(
        rng: &mut Rng,
        pts: &[[f32; 2]],
        spacing: f32,
        mut f: impl FnMut(&mut Rng, f32, f32),
    ) {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for p in pts {
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        }
        let mut z = lo[1];
        while z < hi[1] {
            let mut x = lo[0];
            while x < hi[0] {
                let (px, pz) = (x + rng.range(0.0, spacing), z + rng.range(0.0, spacing));
                f(rng, px, pz);
                x += spacing;
            }
            z += spacing;
        }
    }

    fn place_props(&mut self) {
        let mut rng = Rng::new(self.seed ^ 0x5EED);
        let def = self.terrain.def;
        let mut placed = Vec::new();
        for cover in def.covers {
            let mine = |x: f32, z: f32| {
                self.terrain
                    .cover(x, z)
                    .is_some_and(|c| std::ptr::eq(c, cover))
            };
            match cover.kind {
                CoverKind::Forest => {
                    let (spacing, scale, min_up) = match (cover.density, cover.trees) {
                        (Density::Open, Trees::Palm) => (34.0, (1.0, 1.5), 0.8),
                        (Density::Open, _) => (48.0, (2.6, 4.0), 0.8),
                        (Density::Rain, _) => (20.0, (1.8, 2.8), 0.7),
                        (Density::Dense, _) => (15.0, (1.3, 2.2), 0.7),
                    };
                    let kind = match cover.trees {
                        Trees::Pine => PropKind::PineTree,
                        _ => PropKind::RoundTree,
                    };
                    Self::scatter(&mut rng, cover.pts, spacing, |rng, x, z| {
                        let s = rng.range(scale.0, scale.1);
                        let v = (rng.next_u64() % 3) as u8;
                        if mine(x, z)
                            && let Some(h) = self.dry_spot(x, z, min_up)
                        {
                            placed.push((kind, Vec3::new(x, h - 0.3, z), s, v));
                        }
                    });
                }
                CoverKind::Meadow => {
                    Self::scatter(&mut rng, cover.pts, 40.0, |rng, cx, cz| {
                        let v = (rng.next_u64() % 4) as u8;
                        if rng.f32() > 0.55 || !mine(cx, cz) {
                            return;
                        }
                        for _ in 0..10 {
                            let x = cx + rng.range(-12.0, 12.0);
                            let z = cz + rng.range(-12.0, 12.0);
                            let s = rng.range(1.8, 2.8);
                            if let Some(h) = self.dry_spot(x, z, 0.8) {
                                placed.push((PropKind::Flower, Vec3::new(x, h, z), s, v));
                            }
                        }
                    });
                }
                CoverKind::Town => {}
            }
        }
        for (kind, pos, s, v) in placed {
            self.push(kind, pos, s, v);
        }
        let h = self.half_size;
        // A light scatter of fair weather clouds until real weather arrives.
        for _ in 0..450 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let pos = Vec3::new(x, rng.range(300.0, 1100.0), z);
            let s = rng.range(2.0, 5.0);
            let v = (rng.next_u64() % 3) as u8;
            let id = self.props.len() as u32;
            self.props.push(Prop {
                id,
                kind: PropKind::Cloud,
                pos,
                yaw: rng.range(0.0, std::f32::consts::TAU),
                scale: s,
                variant: v,
            });
        }
    }
}
