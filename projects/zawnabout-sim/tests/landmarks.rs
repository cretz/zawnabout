//! Landmark meshes must not flicker: no two boxes of different colors may share a face
//! plane where they overlap, or the GPU picks between them pixel by pixel.

use zawnabout_sim::landmark::{Form, KINDS, Mat, pieces};

/// A box's faces as (axis and which way it faces, plane, the other two axes' ranges), for
/// boxes not turned. Faces back to back are culled, so only faces facing the same way can
/// flicker.
/// A box face: its axis, its plane, and its span on the other two axes.
type Face = (usize, f32, [(f32, f32); 2]);

fn faces(at: glam::Vec3, half: glam::Vec3) -> Vec<Face> {
    let (x0, x1) = (at.x - half.x, at.x + half.x);
    let (y0, y1) = (at.y, at.y + half.y);
    let (z0, z1) = (at.z - half.z, at.z + half.z);
    vec![
        (1, y1, [(x0, x1), (z0, z1)]),
        (3, x0, [(y0, y1), (z0, z1)]),
        (0, x1, [(y0, y1), (z0, z1)]),
        (5, z0, [(x0, x1), (y0, y1)]),
        (2, z1, [(x0, x1), (y0, y1)]),
    ]
}

fn overlap(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.1.min(b.1) - a.0.max(b.0)).max(0.0)
}

#[test]
fn no_flicker() {
    let mut bad = Vec::new();
    for (name, kind) in KINDS {
        let ps = pieces(kind, 7, "TEST");
        let boxes: Vec<(glam::Vec3, glam::Vec3, Mat)> = ps
            .iter()
            .filter_map(|p| match p.form {
                Form::Box {
                    at,
                    half,
                    yaw,
                    roof,
                } if yaw == 0.0 && roof == 0.0 => Some((at, half, p.mat)),
                _ => None,
            })
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                if a.2 == b.2 {
                    continue;
                }
                for fa in faces(a.0, a.1) {
                    for fb in faces(b.0, b.1) {
                        if fa.0 != fb.0 || (fa.1 - fb.1).abs() > 0.01 {
                            continue;
                        }
                        let (u, v) = (overlap(fa.2[0], fb.2[0]), overlap(fa.2[1], fb.2[1]));
                        let area = u * v;
                        // Slivers too thin to see flicker are let be.
                        if area > 0.5 && u.min(v) > 0.15 {
                            bad.push(format!("{name}: {:?} {} and {:?} {} share a face at axis {} = {:.2} ({area:.1} m2)", a.2, a.0, b.2, b.0, fa.0, fa.1));
                        }
                    }
                }
            }
        }
    }
    for b in &bad {
        println!("{b}");
    }
    assert!(bad.is_empty(), "{} flickering faces", bad.len());
}

/// No tree, bush or palm stands in a pool, nor so close it hangs over the water.
#[test]
fn no_trees_in_pools() {
    use zawnabout_sim::world::PropKind;
    let w = zawnabout_sim::World::generate(1);
    let mut bad = Vec::new();
    for p in &w.props {
        if !matches!(
            p.kind,
            PropKind::RoundTree | PropKind::PineTree | PropKind::Palm | PropKind::Bush
        ) {
            continue;
        }
        let reach = 1.5 * p.scale;
        let wet = w.pools.iter().any(|s| {
            let (c, r) = s.footprint();
            c.distance(glam::Vec2::new(p.pos.x, p.pos.z)) < r + reach
                && [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
                    .iter()
                    .any(|(dx, dz)| {
                        s.support(p.pos.x + dx * reach, p.pos.z + dz * reach)
                            .is_some_and(|h| (h - p.pos.y).abs() < 6.0)
                    })
        });
        if wet {
            bad.push(format!(
                "{:?} at {:.0} {:.0} {:.0}",
                p.kind, p.pos.x, p.pos.y, p.pos.z
            ));
        }
    }
    for b in &bad {
        println!("{b}");
    }
    assert!(bad.is_empty(), "{} trees in pools", bad.len());
}

/// No landmark's trees or hedges stand in its own water.
#[test]
fn no_leaves_in_water() {
    let mut bad = Vec::new();
    for (name, kind) in KINDS {
        let ps = pieces(kind, 7, "TEST");
        let water: Vec<_> = ps.iter().filter(|p| p.mat == Mat::Water).collect();
        for p in ps
            .iter()
            .filter(|p| matches!(p.mat, Mat::Leaf | Mat::Hedge))
        {
            let (at, r) = match p.form {
                Form::Box { at, half, .. } => (at, half.x.max(half.z)),
                Form::Round { at, r0, r1, .. } => (at, r0.max(r1)),
                Form::Ball { at, r } => (at, r),
                Form::Letter { .. } => continue,
            };
            // Reeds may stand at the very edge, a tree's crown may not reach far over it.
            let reach = r * 0.5 - 1.0;
            for w in &water {
                let d = glam::Vec2::new(at.x, at.z);
                let inside = match w.form {
                    Form::Box { at: wat, half, .. } => {
                        let q = (d - glam::Vec2::new(wat.x, wat.z)).abs();
                        q.x < half.x + reach && q.y < half.z + reach
                    }
                    Form::Round {
                        at: wat, r0, r1, ..
                    } => d.distance(glam::Vec2::new(wat.x, wat.z)) < r0.max(r1) + reach,
                    Form::Ball { .. } | Form::Letter { .. } => false,
                };
                let wat = match w.form {
                    Form::Box { at, .. }
                    | Form::Round { at, .. }
                    | Form::Ball { at, .. }
                    | Form::Letter { at, .. } => at,
                };
                if inside && at.y > wat.y - 3.0 {
                    bad.push(format!("{name}: leaves at {at} in water at {wat}"));
                }
            }
        }
    }
    for b in &bad {
        println!("{b}");
    }
    assert!(bad.is_empty(), "{} leaves in water", bad.len());
}

/// Probe: where every building of kind `KIND` (as in the world file, like `billboard`)
/// stands, and which way it faces.
#[test]
#[ignore]
fn where_kind() {
    let w = zawnabout_sim::World::generate(1);
    let name = std::env::var("KIND").unwrap_or_default();
    let Some(&(_, kind)) = zawnabout_sim::landmark::KINDS
        .iter()
        .find(|(n, _)| *n == name)
    else {
        panic!("no kind {name}");
    };
    for b in w.buildings.iter().filter(|b| b.kind == kind) {
        let f = b.front();
        println!(
            "{:.0} {:.1} {:.0} front {:.2} {:.2}",
            b.pos.x, b.pos.y, b.pos.z, f.x, f.y
        );
    }
}
