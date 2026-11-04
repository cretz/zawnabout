//! Water meshes: the sea, lakes, cave pools and river ribbons with their falls. Drawn
//! see-through after everything solid by the water shader, which animates the facets.
//!
//! Vertex data means something else here: alpha is how deep the water is (0 at the shore,
//! 1 when deep), the normal's x and z are the flow, its y how much daylight reaches, and
//! its w marks falling water.

use glam::{Vec2, Vec3};
use zawnabout_sim::terrain::Terrain;

use crate::mesh::{MeshBuilder, Rgb, Vertex};

/// Water this deep or more is fully opaque, so the seabed below it is never drawn.
pub const OPAQUE_DEPTH: f32 = 12.0;
/// Sea tiles are this big, for culling.
const SEA_TILE: f32 = 2000.0;
const SEA: Rgb = Rgb(0x2F8FC8);
const LAKE: Rgb = Rgb(0x3FA4C8);
const RIVER: Rgb = Rgb(0x4AB0CF);
const CAVE: Rgb = Rgb(0x3AB8C0);

/// One water mesh and where it is.
pub struct WaterPart {
    pub mesh: MeshBuilder,
    /// Under ground, so only drawn with the cave insides.
    pub cave: bool,
}

/// A water vertex. `depth` in meters, `flow` in m/s on x and z.
fn vert(p: Vec3, color: Rgb, depth: f32, flow: Vec2, day: f32, falls: bool) -> Vertex {
    let c = color.bytes();
    let s = |v: f32| (v.clamp(-1.0, 1.0) * 127.0) as i8;
    Vertex {
        pos: p.to_array(),
        normal: [
            s(flow.x / 20.0),
            s(day),
            s(flow.y / 20.0),
            if falls { 127 } else { 0 },
        ],
        color: [
            c[0],
            c[1],
            c[2],
            ((depth / OPAQUE_DEPTH).clamp(0.0, 1.0) * 255.0) as u8,
        ],
    }
}

fn tri(m: &mut MeshBuilder, a: Vertex, b: Vertex, c: Vertex) {
    m.verts.extend_from_slice(&[a, b, c]);
}

/// A flat quad with per corner depths, wound to face up.
fn flat_quad(
    m: &mut MeshBuilder,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    y: f32,
    d: [f32; 4],
    color: Rgb,
    day: f32,
) {
    let v =
        |x: f32, z: f32, depth: f32| vert(Vec3::new(x, y, z), color, depth, Vec2::ZERO, day, false);
    let (a, b, c, e) = (
        v(x0, z0, d[0]),
        v(x0, z1, d[1]),
        v(x1, z1, d[2]),
        v(x1, z0, d[3]),
    );
    tri(m, a, b, c);
    tri(m, a, c, e);
}

/// The sea: one mesh of long deep strips plus a frame out past the horizon, and tiles of
/// shallow water near land. Only cells on the shoreline split finer, so the shore and
/// canals get real depths.
pub fn sea(terrain: &Terrain) -> Vec<WaterPart> {
    let level = terrain.def.sea_level;
    let b = terrain.def.bounds;
    let (x0, z0, x1, z1) = (b[0] - 1000.0, b[1] - 1000.0, b[2] + 1000.0, b[3] + 1000.0);
    let cell = 40.0;
    let (nx, nz) = (((x1 - x0) / cell) as usize, ((z1 - z0) / cell) as usize);
    let per = (SEA_TILE / cell) as usize;
    let mut deep = MeshBuilder::new();
    let mut tiles: Vec<MeshBuilder> = Vec::new();
    let tiles_x = nx.div_ceil(per);
    tiles.resize_with(tiles_x * nz.div_ceil(per), MeshBuilder::new);
    let mut row = vec![0.0f32; nx + 1];
    let mut next = vec![0.0f32; nx + 1];
    for (i, h) in row.iter_mut().enumerate() {
        *h = terrain.height(x0 + i as f32 * cell, z0);
    }
    for j in 0..nz {
        let (za, zb) = (z0 + j as f32 * cell, z0 + (j + 1) as f32 * cell);
        for (i, h) in next.iter_mut().enumerate() {
            *h = terrain.height(x0 + i as f32 * cell, zb);
        }
        let mut run: Option<f32> = None;
        for i in 0..=nx {
            let xa = x0 + i as f32 * cell;
            let mut is_deep = false;
            if i < nx {
                let hs = [row[i], next[i], next[i + 1], row[i + 1]];
                let lo = hs.iter().fold(f32::MAX, |a, b| a.min(*b));
                let hi = hs.iter().fold(f32::MIN, |a, b| a.max(*b));
                if hi < level - OPAQUE_DEPTH {
                    is_deep = true;
                } else if lo < level + 1.0 {
                    let t = &mut tiles[(j / per) * tiles_x + i / per];
                    if hi > level - 3.0 {
                        shallow(t, terrain, xa, za, cell, 2, level);
                    } else {
                        let d = hs.map(|y| level - y);
                        flat_quad(t, xa, za, xa + cell, zb, level, d, SEA, 1.0);
                    }
                }
            }
            match (is_deep, run) {
                (true, None) => run = Some(xa),
                (false, Some(start)) => {
                    let d = [OPAQUE_DEPTH; 4];
                    flat_quad(&mut deep, start, za, xa, zb, level, d, SEA, 1.0);
                    run = None;
                }
                _ => {}
            }
        }
        std::mem::swap(&mut row, &mut next);
    }
    let r = 40_000.0;
    let d = [OPAQUE_DEPTH; 4];
    for (a, b, c, e) in [
        (-r, -r, r, z0),
        (-r, z1, r, r),
        (-r, z0, x0, z1),
        (x1, z0, r, z1),
    ] {
        flat_quad(&mut deep, a, b, c, e, level, d, SEA, 1.0);
    }
    let mut parts = vec![WaterPart {
        mesh: deep,
        cave: false,
    }];
    parts.extend(
        tiles
            .into_iter()
            .map(|mesh| WaterPart { mesh, cave: false }),
    );
    parts
}

/// One sea cell near land, split into `fine` by `fine` quads with real depths.
fn shallow(
    m: &mut MeshBuilder,
    terrain: &Terrain,
    x: f32,
    z: f32,
    cell: f32,
    fine: usize,
    level: f32,
) {
    let step = cell / fine as f32;
    let n = fine + 1;
    let mut h = vec![0.0f32; n * n];
    for j in 0..n {
        for i in 0..n {
            h[j * n + i] = terrain.height(x + i as f32 * step, z + j as f32 * step);
        }
    }
    for j in 0..fine {
        for i in 0..fine {
            let c = [
                h[j * n + i],
                h[(j + 1) * n + i],
                h[(j + 1) * n + i + 1],
                h[j * n + i + 1],
            ];
            if c.iter().all(|y| *y > level + 0.5) {
                continue;
            }
            let (xa, za) = (x + i as f32 * step, z + j as f32 * step);
            let d = c.map(|y| level - y);
            flat_quad(m, xa, za, xa + step, za + step, level, d, SEA, 1.0);
        }
    }
}

/// Still water over an outline at one level, in cells that reach past the shore. Cells
/// only count where `wet` says the water is really there.
fn still(
    m: &mut MeshBuilder,
    pts: &[[f32; 2]],
    level: f32,
    color: Rgb,
    day: f32,
    floor: impl Fn(f32, f32) -> Option<f32>,
) {
    let cell = 6.0;
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in pts {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    let y = level - 0.05;
    let mut z = lo[1] - 20.0;
    while z < hi[1] + 20.0 {
        let mut x = lo[0] - 20.0;
        while x < hi[0] + 20.0 {
            let corners = [(x, z), (x, z + cell), (x + cell, z + cell), (x + cell, z)];
            let floors = corners.map(|(cx, cz)| floor(cx, cz));
            if floors.iter().any(|f| f.is_some()) {
                let d = floors.map(|f| f.map_or(0.0, |g| level - g));
                flat_quad(m, x, z, x + cell, z + cell, y, d, color, day);
            }
            x += cell;
        }
        z += cell;
    }
}

/// Surface lakes, one part each.
pub fn lakes(terrain: &Terrain) -> Vec<WaterPart> {
    terrain
        .def
        .lakes
        .iter()
        .map(|lake| {
            let mut m = MeshBuilder::new();
            still(&mut m, lake.pts, lake.level, LAKE, 1.0, |x, z| {
                let g = terrain.height(x, z);
                (terrain.water(x, z, g) == Some(lake.level)).then_some(g)
            });
            WaterPart {
                mesh: m,
                cave: false,
            }
        })
        .collect()
}

/// Cave lakes, wherever the cave floor dips under their level.
pub fn pools(terrain: &Terrain) -> Vec<WaterPart> {
    terrain
        .def
        .pools
        .iter()
        .map(|pool| {
            let mut m = MeshBuilder::new();
            let level = pool.level;
            still(&mut m, pool.pts, level, CAVE, 0.0, |x, z| {
                let g = terrain.support(x, z, level + 4.0);
                (g < level && g > level - 60.0).then_some(g)
            });
            WaterPart {
                mesh: m,
                cave: true,
            }
        })
        .collect()
}

/// Rivers as ribbons along their paths. Steep stretches become falls.
pub fn rivers(terrain: &Terrain) -> Vec<WaterPart> {
    let def = terrain.def;
    let mut parts = Vec::new();
    for (ri, r) in def.rivers.iter().enumerate() {
        let lv = terrain.river_levels(ri);
        let n = r.pts.len();
        let at = |i: usize| Vec3::new(r.pts[i][0], lv[i], r.pts[i][1]);
        let flat = |i: usize| Vec2::new(r.pts[i][0], r.pts[i][1]);
        let side = |i: usize| {
            let (a, b) = (flat(i.saturating_sub(1)), flat((i + 1).min(n - 1)));
            let d = (b - a).normalize_or_zero();
            Vec3::new(-d.y, 0.0, d.x)
        };
        // Cave rivers fill their channel. Surface ribbons tuck under the banks.
        let half = r.w * 0.5 + if r.under { 2.0 } else { 3.0 };
        let (color, day) = if r.under { (CAVE, 0.0) } else { (RIVER, 1.0) };
        let mut m = MeshBuilder::new();
        let depth_at = |p: Vec3| {
            let g = if r.under {
                terrain.support(p.x, p.z, p.y + 2.0)
            } else {
                terrain.height(p.x, p.z)
            };
            (p.y - g).max(0.0)
        };
        // Where a river runs into a lake or the sea, that water takes over.
        let swallowed = |p: Vec3| {
            let g = terrain.height(p.x, p.z);
            !r.under
                && terrain
                    .still_water(p.x, p.z, g)
                    .is_some_and(|l| l >= p.y - 0.3)
        };
        // A branch stops at its parent's edge, where the parent's water takes over.
        let parent = (r.parent >= 0).then(|| &def.rivers[r.parent as usize]);
        let joined = |p: Vec3| {
            parent.is_some_and(|pr| {
                pr.pts.windows(2).any(|w| {
                    let (a, b) = (Vec2::new(w[0][0], w[0][1]), Vec2::new(w[1][0], w[1][1]));
                    let ab = b - a;
                    let t = ((Vec2::new(p.x, p.z) - a).dot(ab) / ab.length_squared().max(1e-6))
                        .clamp(0.0, 1.0);
                    (a + ab * t).distance(Vec2::new(p.x, p.z)) < pr.w * 0.5
                })
            })
        };
        for i in 0..n - 1 {
            let (a, b) = (at(i), at(i + 1));
            if (swallowed(a) && swallowed(b)) || (joined(a) && joined(b)) {
                continue;
            }
            let run = (flat(i + 1) - flat(i)).length().max(0.1);
            let drop = a.y - b.y;
            let falls = drop / run > 0.6;
            let dir = (flat(i + 1) - flat(i)) / run;
            // Speed from the slope: lazy on the flat, quick on rapids.
            let speed = 1.5 + (drop / run * 60.0).min(8.0);
            let flow = dir * speed;
            let (sa, sb) = (side(i) * half, side(i + 1) * half);
            let corners = [a - sa, a + sa, b + sb, b - sb];
            let v = |p: Vec3, edge: bool| {
                let depth = if falls {
                    OPAQUE_DEPTH
                } else if edge {
                    0.0
                } else {
                    depth_at(p)
                };
                vert(p, color, depth, flow, day, falls)
            };
            if falls && r.under {
                // Cave falls pour off the lip and hang straight down into the dark.
                let top = Vec3::new(b.x, a.y, b.z);
                let w = |p: Vec3, f: bool| vert(p, color, OPAQUE_DEPTH * 0.6, flow, day, f);
                let (l0, r0, l1, r1) = (a - sa, a + sa, top - sb, top + sb);
                tri(&mut m, w(l0, false), w(l1, false), w(r1, false));
                tri(&mut m, w(l0, false), w(r1, false), w(r0, false));
                let (bl, br) = (b - sb, b + sb);
                tri(&mut m, w(l1, true), w(bl, true), w(br, true));
                tri(&mut m, w(l1, true), w(br, true), w(r1, true));
                continue;
            }
            let mid_a = v(a, false);
            let mid_b = v(b, false);
            let [la, ra, rb, lb] = corners.map(|p| v(p, true));
            // Two strips, so the middle is deep and the edges fade into the banks.
            tri(&mut m, la, lb, mid_b);
            tri(&mut m, la, mid_b, mid_a);
            tri(&mut m, mid_a, mid_b, rb);
            tri(&mut m, mid_a, rb, ra);
        }
        parts.push(WaterPart {
            mesh: m,
            cave: r.under,
        });
    }
    parts
}

/// The foot of every falls, with its width, for spray.
pub fn falls_feet(terrain: &Terrain) -> Vec<(Vec3, f32)> {
    let mut out = Vec::new();
    for (ri, r) in terrain.def.rivers.iter().enumerate() {
        let lv = terrain.river_levels(ri);
        let mut was = false;
        for i in 0..r.pts.len() - 1 {
            let run = Vec2::new(r.pts[i + 1][0] - r.pts[i][0], r.pts[i + 1][1] - r.pts[i][1]);
            let falls = (lv[i] - lv[i + 1]) / run.length().max(0.1) > 0.6;
            let end = !falls && was;
            if end {
                out.push((Vec3::new(r.pts[i][0], lv[i], r.pts[i][1]), r.w));
            }
            was = falls;
        }
    }
    out
}

/// Every water part in the world.
pub fn all(terrain: &Terrain) -> Vec<WaterPart> {
    let mut out = sea(terrain);
    out.extend(lakes(terrain));
    out.extend(pools(terrain));
    out.extend(rivers(terrain));
    out.retain(|p| !p.mesh.verts.is_empty());
    out
}
