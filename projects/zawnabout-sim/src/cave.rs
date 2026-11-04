//! Things in caves: crystals, glowing mushrooms, stalagmites and stalactites. Placed from
//! each cavern's shape and decor, the same on every peer.

use glam::{Vec2, Vec3};

use crate::terrain::{Decor, Terrain, Void, VoidShape, void_sdf};
use crate::world::{PropKind, Rng};

/// Crystal colors, by variant: ice, pink, violet, teal, amber, green.
pub const CRYSTAL_KINDS: u8 = 6;
const AMBER: u8 = 4;
/// Mushroom colors, by variant.
pub const SHROOM_KINDS: u8 = 4;

/// One thing to place: kind, position, scale and variant.
pub type Placed = (PropKind, Vec3, f32, u8);
/// The same, with which way it faces (yaw).
pub type Turned = (PropKind, Vec3, f32, u8, f32);

/// Sample spacing over cavern floors (m).
const SPACING: f32 = 30.0;
/// Sample spacing for the giants that fill big rooms: huge stalactites, towering
/// mushrooms and crystals, curtains of glowworms (m).
const GIANT_SPACING: f32 = 85.0;

/// Where a giant mushroom's cap sits on its stalk, at scale 1, before turning. Shared by
/// the mesh and the collider.
pub fn shroom_top(variant: u8) -> Vec3 {
    let h = |k: u32| crate::shape::hash(variant as u32 * 7 + k);
    Vec3::new(0.5 + h(1) * 0.6, 6.6 + h(2) * 1.6, h(3) * 0.4 - 0.2)
}

/// FNV-1a, for seeding each cavern from its id.
fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01B3)
    })
}

/// Chances per sample of a crystal, mushroom patch, stalagmite and stalactite.
fn weights(decor: Decor) -> [f32; 4] {
    match decor {
        Decor::Bare => [0.0; 4],
        Decor::Mixed => [0.12, 0.14, 0.07, 0.12],
        Decor::Crystal => [0.32, 0.05, 0.05, 0.1],
        Decor::Shroom => [0.05, 0.36, 0.05, 0.08],
        Decor::Ember => [0.18, 0.0, 0.12, 0.1],
        Decor::Ruins => [0.05, 0.14, 0.03, 0.06],
    }
}

/// Walks from `from` along `dir` (up or down) through the void's own space to its wall.
fn reach(v: &Void, from: Vec3, dir: f32, limit: f32) -> Option<f32> {
    let mut y = from.y;
    for _ in 0..40 {
        let d = void_sdf(v, Vec3::new(from.x, y, from.z));
        if d > -0.4 {
            return Some(y);
        }
        y += dir * (-d * 0.8).max(0.5);
        if (y - from.y).abs() > limit {
            return None;
        }
    }
    None
}

/// Whether `p` is near any cave town, where houses stand instead.
fn in_town(t: &Terrain, p: Vec3) -> bool {
    t.def.towns.iter().filter(|town| town.under).any(|town| {
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for q in town.pts {
            lo = lo.min(Vec2::from(*q));
            hi = hi.max(Vec2::from(*q));
        }
        let pad = Vec2::splat(40.0);
        let at = Vec2::new(p.x, p.z);
        at.cmpge(lo - pad).all() && at.cmple(hi + pad).all()
    })
}

/// Everything growing or hanging in every decorated cavern.
pub fn decorate(t: &Terrain, seed: u64) -> Vec<Placed> {
    let mut out = Vec::new();
    for v in t.def.voids {
        if matches!(v.shape, VoidShape::Tube | VoidShape::Basin) || v.decor == Decor::Bare {
            continue;
        }
        decorate_one(t, v, seed, &mut out);
        giants(t, v, seed, &mut out);
    }
    out
}

/// The big things that give a huge room landmarks and fill its height: stalactites tens
/// of meters long with stalagmites rising to meet them, mushrooms as tall as trees with caps
/// to land on, crystals like towers, and curtains of glowworms hanging in the dark.
fn giants(t: &Terrain, v: &Void, seed: u64, out: &mut Vec<Placed>) {
    let he = Vec3::from(v.half);
    // Small rooms are full enough already.
    if he.x.min(he.z) < 120.0 || he.y < 50.0 {
        return;
    }
    let h = hash_str(v.id);
    let mut rng = Rng::new(seed ^ h ^ 0x6A17);
    let shroom = ((h >> 16) % SHROOM_KINDS as u64) as u8;
    let crystal = if v.decor == Decor::Ember {
        AMBER
    } else {
        let c = (h % CRYSTAL_KINDS as u64) as u8;
        if c == AMBER { 0 } else { c }
    };
    // Chances per sample of a giant mushroom and a giant crystal.
    let (shrooms, crystals) = match v.decor {
        Decor::Shroom => (0.4, 0.05),
        Decor::Crystal => (0.04, 0.3),
        Decor::Ember => (0.0, 0.22),
        Decor::Mixed => (0.15, 0.12),
        _ => (0.1, 0.04),
    };
    let c = Vec3::from(v.center);
    let mut u = -he.x;
    while u < he.x {
        let mut s = -he.z;
        while s < he.z {
            let (qu, qs) = (
                u + rng.range(0.0, GIANT_SPACING),
                s + rng.range(0.0, GIANT_SPACING),
            );
            let (roof_roll, floor_roll, size, pick) = (rng.f32(), rng.f32(), rng.f32(), rng.f32());
            s += GIANT_SPACING;
            let x = c.x + qu * v.cos - qs * v.sin;
            let z = c.z + qu * v.sin + qs * v.cos;
            let mid = Vec3::new(x, c.y, z);
            if void_sdf(v, mid) > -20.0 || in_town(t, mid) {
                continue;
            }
            let roof = reach(v, mid, 1.0, he.y * 2.0);
            let floor = reach(v, mid, -1.0, he.y * 2.0).map(|f| t.support(x, z, f + 3.0));
            let (Some(roof), Some(floor)) = (roof, floor) else {
                continue;
            };
            let room = roof - floor;
            if room < 40.0 {
                continue;
            }
            let top = Vec3::new(x, roof + 2.0, z);
            let roof_ok = t.sdf(top - Vec3::Y * 6.0) > 0.0 && t.sdf(top + Vec3::Y * 2.0) < 0.5;
            let wet = t.cave_water(x, z, floor).is_some();
            // From the roof: a great stalactite, often with a stalagmite growing to meet it,
            // or a curtain of glowworms.
            if roof_ok && roof_roll < 0.12 {
                let len = (room * (0.25 + size * 0.3)).min(160.0);
                let scale = len / 5.6;
                out.push((PropKind::Stalactite, top, scale, 1));
                if !wet && pick < 0.45 {
                    let rise = (room - len - 25.0).max(0.0) * (0.5 + pick * 0.5);
                    if rise > 20.0 {
                        out.push((
                            PropKind::Stalagmite,
                            Vec3::new(x + 1.0, floor - 1.0, z),
                            rise / 5.0,
                            3 + (pick * 3.0) as u8 % 3,
                        ));
                    }
                }
                continue;
            }
            if roof_ok && roof_roll < 0.4 {
                out.push((
                    PropKind::Glowworms,
                    top - Vec3::Y * 1.5,
                    2.0 + size * 3.0,
                    (pick * 3.0) as u8 % 3,
                ));
            }
            if wet || t.sdf(Vec3::new(x, floor + 6.0, z)) < 2.0 {
                continue;
            }
            let at = Vec3::new(x, floor - 0.5, z);
            if floor_roll < shrooms {
                // Mushrooms grow in rings and clumps: one big and a few smaller ones.
                let n = 1 + (size * 2.6) as u32;
                for k in 0..n {
                    let a = k as f32 * 2.4 + pick * 6.0;
                    let d = if k == 0 { 0.0 } else { 12.0 + k as f32 * 5.0 };
                    let p = at + Vec3::new(a.cos() * d, 0.0, a.sin() * d);
                    let f = t.support(p.x, p.z, p.y + 8.0);
                    if (f - floor).abs() > 6.0 || t.cave_water(p.x, p.z, f).is_some() {
                        continue;
                    }
                    let scale = if k == 0 {
                        2.2 + size * 2.4
                    } else {
                        1.0 + crate::shape::hash(k + (h as u32)) * 1.4
                    };
                    let color = if k % 3 == 2 {
                        (shroom + 1) % SHROOM_KINDS
                    } else {
                        shroom
                    };
                    out.push((
                        PropKind::GiantShroom,
                        Vec3::new(p.x, f - 0.4, p.z),
                        scale,
                        color,
                    ));
                }
            } else if floor_roll < shrooms + crystals {
                out.push((PropKind::Crystal, at, 9.0 + size * 9.0, crystal));
            }
        }
        u += GIANT_SPACING;
    }
}

fn decorate_one(t: &Terrain, v: &Void, seed: u64, out: &mut Vec<Placed>) {
    let h = hash_str(v.id);
    let mut rng = Rng::new(seed ^ h);
    // Two crystal colors and one mushroom color per cavern, so each has its own look.
    let (c0, c1) = if v.decor == Decor::Ember {
        (AMBER, AMBER)
    } else {
        let a = (h % CRYSTAL_KINDS as u64) as u8;
        let b = ((h >> 8) % CRYSTAL_KINDS as u64) as u8;
        // Amber stays with the ember rooms.
        let fix = |c: u8| if c == AMBER { 0 } else { c };
        (fix(a), fix(b))
    };
    let shroom = ((h >> 16) % SHROOM_KINDS as u64) as u8;
    let w = weights(v.decor);
    let c = Vec3::from(v.center);
    let he = Vec3::from(v.half);
    let mut u = -he.x;
    while u < he.x {
        let mut s = -he.z;
        while s < he.z {
            let (qu, qs) = (u + rng.range(0.0, SPACING), s + rng.range(0.0, SPACING));
            let roll = rng.f32();
            let size = rng.f32();
            let pick = rng.f32();
            s += SPACING;
            // Rooms are turned about y: this undoes the turn in `void_sdf`.
            let x = c.x + qu * v.cos - qs * v.sin;
            let z = c.z + qu * v.sin + qs * v.cos;
            let mid = Vec3::new(x, c.y, z);
            if void_sdf(v, mid) > -4.0 {
                continue;
            }
            let [crystal, shroom_w, mite, tite] = w;
            let total = crystal + shroom_w + mite + tite;
            if roll > total {
                continue;
            }
            // Stalactites hang from the roof, everything else stands on the floor.
            if roll > crystal + shroom_w + mite {
                let Some(roof) = reach(v, mid, 1.0, he.y * 2.0) else {
                    continue;
                };
                let at = Vec3::new(x, roof + 1.0, z);
                if t.sdf(at - Vec3::Y * 3.0) > 0.0 && t.sdf(at + Vec3::Y * 2.0) < 0.5 {
                    out.push((PropKind::Stalactite, at, 1.5 + size * 3.5, 0));
                }
                continue;
            }
            let Some(floor) = reach(v, mid, -1.0, he.y * 2.0) else {
                continue;
            };
            // The room's own shape ignores lake basins and other caves, so settle on the
            // real floor.
            let floor = t.support(x, z, floor + 3.0);
            let at = Vec3::new(x, floor - 0.6, z);
            // Open just above (not in a pillar or another wall). Only crystals grow under
            // cave water, glowing up through it.
            let wet = t.cave_water(x, z, floor).is_some();
            if t.sdf(at + Vec3::Y * 2.5) < 0.5 || (wet && roll >= crystal) || in_town(t, at) {
                continue;
            }
            let placed = if roll < crystal {
                let color = if pick < 0.6 { c0 } else { c1 };
                // Now and then a giant, so crystal rooms have landmarks.
                let giant = v.decor == Decor::Crystal && size > 0.9;
                let scale = if giant {
                    6.0 + size * 4.0
                } else {
                    1.2 + size * 2.4
                };
                (PropKind::Crystal, at, scale, color)
            } else if roll < crystal + shroom_w {
                (PropKind::Shroom, at, 1.0 + size * 1.6, shroom)
            } else {
                (PropKind::Stalagmite, at, 1.5 + size * 3.0, 0)
            };
            out.push(placed);
        }
        u += SPACING;
    }
}

/// Dressing round every cave opening, so a mouth reads as a place rather than a hole
/// punched in the hill: boulders heaped round its rim and vines hanging over its lip, or
/// down into the pit of a sinkhole.
pub fn mouths(t: &Terrain, seed: u64) -> Vec<Turned> {
    let mut out = Vec::new();
    let sea = t.def.sea_level;
    for (k, v) in t.def.voids.iter().enumerate() {
        if v.shape != VoidShape::Tube {
            continue;
        }
        let (a, b) = (Vec3::from(v.a), Vec3::from(v.b));
        let above = |p: Vec3| p.y - t.height(p.x, p.z);
        // Only the stretch of tube that breaks through the surface.
        let (inside, outside) = match (above(a) < 0.0, above(b) < 0.0) {
            (true, false) => (a, b),
            (false, true) => (b, a),
            _ => continue,
        };
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..20 {
            let m = (lo + hi) * 0.5;
            if above(inside.lerp(outside, m)) < 0.0 {
                lo = m;
            } else {
                hi = m;
            }
        }
        let exit = inside.lerp(outside, lo);
        if exit.y < sea + 1.0 {
            continue;
        }
        let dir = (outside - inside).normalize_or(Vec3::Y);
        let pit = dir.y.abs() > 0.7;
        let side = dir.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = side.cross(dir).normalize_or(Vec3::Y);
        let r = v.r;
        let mut rng = Rng::new(seed ^ (k as u64).wrapping_mul(0x9E37_79B9) ^ 0x50C7);
        let n = (10.0 + r * 0.3).min(34.0) as u32;
        for i in 0..n {
            let turn = (i as f32 + rng.range(-0.3, 0.3)) / n as f32 * std::f32::consts::TAU;
            let (size, roll, vroll) = (rng.f32(), rng.f32(), rng.f32());
            let radial = side * turn.cos() + up * turn.sin();
            // The rim is where this line along the tube wall breaks out of the ground.
            let wall = |u: f32| inside.lerp(outside, u) + radial * r;
            let steps = 48;
            let mut rim = None;
            let mut prev = above(wall(0.0));
            for k in 1..=steps {
                let u = k as f32 / steps as f32 * 1.6;
                let now = above(wall(u));
                if prev < 0.0 && now >= 0.0 {
                    let (mut lo, mut hi) = (u - 1.6 / steps as f32, u);
                    for _ in 0..12 {
                        let m = (lo + hi) * 0.5;
                        if above(wall(m)) < 0.0 {
                            lo = m;
                        } else {
                            hi = m;
                        }
                    }
                    rim = Some(wall(lo));
                    break;
                }
                prev = now;
            }
            let Some(p) = rim else {
                continue;
            };
            if p.y < sea + 1.0 {
                continue;
            }
            let lip = if pit { true } else { radial.y > 0.25 };
            // Crags heaped round the rim, in uneven clumps with gaps, some bulging out and a
            // heavy brow over the top, so the edge of the hole disappears under rock.
            if roll < 0.2 {
                continue;
            }
            let craggy = 3 + (vroll * 3.0) as u8 % 3;
            let scale = (r * (0.06 + size * size * 0.22)).clamp(2.0, 16.0);
            let out_by = scale * rng.range(-0.4, 0.5);
            out.push((
                PropKind::Rock,
                p + radial * out_by - Vec3::Y * scale * 0.35,
                scale,
                craggy,
                turn * 3.1,
            ));
            if roll > 0.55 {
                let s2 = scale * (0.4 + roll * 0.5);
                let q = p + radial * (scale + s2) * 0.7 + dir * rng.range(-3.0, 4.0);
                out.push((
                    PropKind::Rock,
                    q - Vec3::Y * s2 * 0.4,
                    s2,
                    3 + i as u8 % 3,
                    turn * 5.3,
                ));
            }
            if lip && !pit && size > 0.6 {
                let s3 = scale * 1.4;
                out.push((
                    PropKind::Rock,
                    p + up * s3 * 0.6 + dir * 2.0,
                    s3,
                    craggy,
                    turn * 2.3,
                ));
            }
            // Vines over the top of a cliff mouth, and all round the edge of a pit.
            if lip && vroll < 0.8 {
                let scale = (r * (0.07 + size * 0.08)).clamp(2.5, 9.0);
                // The curtain faces out of the hole, or for a pit, in across it.
                let face = if pit { -radial } else { dir };
                out.push((
                    PropKind::Vines,
                    p - radial * scale * 0.3 + dir * 2.0,
                    scale,
                    (roll * 3.0) as u8 % 3,
                    face.x.atan2(face.z),
                ));
            }
        }
    }
    out
}

/// Shelf fungi growing in tiers up cavern walls, glowing underneath, facing out into the
/// room. Each is a place, kind, size, color and the way it faces.
pub fn brackets(t: &Terrain, seed: u64) -> Vec<Turned> {
    let mut out = Vec::new();
    for v in t.def.voids {
        let he = Vec3::from(v.half);
        if matches!(v.shape, VoidShape::Tube | VoidShape::Basin)
            || v.decor == Decor::Bare
            || he.x.min(he.z) < 60.0
        {
            continue;
        }
        let h = hash_str(v.id);
        let mut rng = Rng::new(seed ^ h ^ 0xB4AC);
        let color = ((h >> 16) % SHROOM_KINDS as u64) as u8;
        let c = Vec3::from(v.center);
        // About one cluster per 70m of wall all round, at every height.
        let tries = ((he.x + he.z) * 2.0 * he.y / 9000.0).clamp(6.0, 160.0) as u32;
        for _ in 0..tries {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let dir = Vec3::new(a.cos(), 0.0, a.sin());
            let y = c.y + rng.range(-0.6, 0.8) * he.y;
            let (size, pick) = (rng.f32(), rng.f32());
            let start = Vec3::new(c.x, y, c.z) + dir * rng.range(0.0, 0.3) * he.x.min(he.z);
            if void_sdf(v, start) > -10.0 {
                continue;
            }
            // Out to the wall.
            let mut p = start;
            let mut hit = None;
            for _ in 0..80 {
                let d = t.sdf(p);
                if d < 0.5 {
                    hit = Some(p);
                    break;
                }
                p += dir * d.clamp(1.0, 40.0);
            }
            let Some(wall) = hit else {
                continue;
            };
            // Only on steep rock, not floors or other things in the way.
            if t.sdf(wall + Vec3::Y * 4.0) < 0.5 && t.sdf(wall - Vec3::Y * 4.0) > 0.5 {
                continue;
            }
            if t.cave_water(wall.x, wall.z, wall.y - 3.0).is_some() || in_town(t, wall) {
                continue;
            }
            let face = -dir;
            let scale = 1.5 + size * size * 4.5;
            let color = if pick < 0.75 {
                color
            } else {
                (color + 2) % SHROOM_KINDS
            };
            out.push((
                PropKind::Brackets,
                wall + face * 0.3,
                scale,
                color,
                face.x.atan2(face.z),
            ));
        }
    }
    out
}

/// One solid piece of a ruin, in the ruin's own frame (scale 1, facing +z).
#[derive(Clone, Copy, Debug)]
pub enum Part {
    /// A box standing on `base` (bottom center) with half width, height and half depth.
    Box { base: Vec3, half: Vec3 },
    /// A round column standing on `base`.
    Column { base: Vec3, h: f32, r: f32 },
}

/// Ruin kinds, by variant.
pub const RUIN_KINDS: [&str; 5] = ["arch", "columns", "wall", "stair", "statue"];

/// The pieces of a ruin. Meshes and colliders both come from these.
pub fn ruin_parts(variant: u8) -> Vec<Part> {
    let b = |x: f32, y: f32, z: f32, hx: f32, hy: f32, hz: f32| Part::Box {
        base: Vec3::new(x, y, z),
        half: Vec3::new(hx, hy, hz),
    };
    let col = |x: f32, z: f32, h: f32| Part::Column {
        base: Vec3::new(x, 0.0, z),
        h,
        r: 1.0,
    };
    match variant {
        0 => vec![
            b(-5.0, 0.0, 0.0, 1.3, 9.0, 1.3),
            b(5.0, 0.0, 0.0, 1.3, 9.0, 1.3),
            b(0.0, 9.0, 0.0, 6.6, 1.8, 1.5),
        ],
        1 => vec![
            b(0.0, 0.0, 0.0, 9.0, 0.6, 3.0),
            col(-6.0, 0.0, 10.0),
            col(-2.0, 0.0, 6.5),
            col(2.0, 0.0, 10.0),
            col(6.0, 0.0, 3.0),
        ],
        2 => vec![
            b(0.0, 0.0, 0.0, 8.0, 5.0, 1.1),
            b(-6.5, 5.0, 0.0, 1.5, 1.2, 1.1),
            b(-1.5, 5.0, 0.0, 1.5, 1.2, 1.1),
            b(3.5, 5.0, 0.0, 1.5, 1.2, 1.1),
        ],
        3 => (0..5)
            .map(|i| b(0.0, 0.0, i as f32 * 2.2, 4.0, (i + 1) as f32 * 1.1, 1.1))
            .collect(),
        _ => vec![
            b(0.0, 0.0, 0.0, 2.2, 2.0, 2.2),
            Part::Column {
                base: Vec3::new(0.0, 2.0, 0.0),
                h: 5.0,
                r: 1.3,
            },
            b(0.0, 7.0, 0.0, 1.0, 1.8, 1.0),
        ],
    }
}

/// Beacon size at scale 1: a stone post with a wide fire bowl on top, landable.
pub const BEACON_POST: f32 = 1.6;
pub const BEACON_TOP: f32 = 2.4;
pub const BEACON_RIM: f32 = 1.5;
