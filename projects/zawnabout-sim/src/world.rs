//! The static world: ground shape and placed props. Generated from a seed so every
//! peer builds the identical world.

use glam::Vec3;

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

fn hash2(seed: u64, x: i32, z: i32) -> f32 {
    let mut r = Rng::new(seed ^ ((x as u64) << 32) ^ (z as u32 as u64));
    r.f32()
}

/// Smooth value noise in `[0, 1]`.
fn value_noise(seed: u64, x: f32, z: f32) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (fx, fz) = (x - xi, z - zi);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let (xi, zi) = (xi as i32, zi as i32);
    let a = hash2(seed, xi, zi);
    let b = hash2(seed, xi + 1, zi);
    let c = hash2(seed, xi, zi + 1);
    let d = hash2(seed, xi + 1, zi + 1);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sz
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

#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    /// The playable square spans `-half_size..half_size` on X and Z.
    pub half_size: f32,
    pub props: Vec<Prop>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        let mut world = World {
            seed,
            half_size: 1500.0,
            props: Vec::new(),
        };
        world.place_props();
        world
    }

    /// Gentle rolling ground. Mostly flat on purpose for now.
    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        let broad = value_noise(self.seed, x / 260.0, z / 260.0) - 0.5;
        let fine = value_noise(self.seed ^ 0xA5A5, x / 70.0, z / 70.0) - 0.5;
        broad * 9.0 + fine * 1.6
    }

    pub fn ground_normal(&self, x: f32, z: f32) -> Vec3 {
        let e = 1.0;
        let dx = self.ground_height(x + e, z) - self.ground_height(x - e, z);
        let dz = self.ground_height(x, z + e) - self.ground_height(x, z - e);
        Vec3::new(-dx, 2.0 * e, -dz).normalize()
    }

    fn place_props(&mut self) {
        let mut rng = Rng::new(self.seed ^ 0x5EED);
        let h = self.half_size;
        let push = |world: &mut World, kind, x: f32, z: f32, y_off: f32, scale, variant| {
            let y = world.ground_height(x, z) + y_off;
            let id = world.props.len() as u32;
            let yaw = (id as f32 * 2.399_963).rem_euclid(std::f32::consts::TAU);
            world.props.push(Prop {
                id,
                kind,
                pos: Vec3::new(x, y, z),
                yaw,
                scale,
                variant,
            });
        };

        // Groves: trees cluster where a low-frequency noise is high, leaving open meadows.
        for _ in 0..9000 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let grove = value_noise(self.seed ^ 0x6407E, x / 180.0, z / 180.0);
            if grove < 0.58 || rng.f32() > (grove - 0.58) * 3.5 {
                continue;
            }
            let kind = if value_noise(self.seed ^ 0x914E, x / 400.0, z / 400.0) > 0.5 {
                PropKind::PineTree
            } else {
                PropKind::RoundTree
            };
            let v = (rng.next_u64() % 3) as u8;
            push(self, kind, x, z, -0.3, rng.range(0.8, 1.5), v);
        }
        for _ in 0..1400 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let v = (rng.next_u64() % 3) as u8;
            push(self, PropKind::Bush, x, z, -0.2, rng.range(0.7, 1.4), v);
        }
        for _ in 0..700 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let v = (rng.next_u64() % 3) as u8;
            push(self, PropKind::Rock, x, z, -0.4, rng.range(0.6, 2.6), v);
        }
        // Tall spires to weave between and judge speed against.
        for _ in 0..60 {
            let (x, z) = (rng.range(-h * 0.9, h * 0.9), rng.range(-h * 0.9, h * 0.9));
            let v = (rng.next_u64() % 3) as u8;
            push(self, PropKind::Spire, x, z, -1.0, rng.range(0.8, 1.8), v);
        }
        // Flower patches.
        for _ in 0..260 {
            let (cx, cz) = (rng.range(-h, h), rng.range(-h, h));
            let v = (rng.next_u64() % 4) as u8;
            for _ in 0..24 {
                let (x, z) = (cx + rng.range(-14.0, 14.0), cz + rng.range(-14.0, 14.0));
                push(self, PropKind::Flower, x, z, 0.0, rng.range(1.8, 2.8), v);
            }
        }
        for _ in 0..170 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let v = (rng.next_u64() % 3) as u8;
            let id = self.props.len() as u32;
            self.props.push(Prop {
                id,
                kind: PropKind::Cloud,
                pos: Vec3::new(x, rng.range(140.0, 300.0), z),
                yaw: rng.range(0.0, std::f32::consts::TAU),
                scale: rng.range(1.0, 2.6),
                variant: v,
            });
        }
    }
}
