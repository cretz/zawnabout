//! Probes for placing gameplay content. Run one with
//! `cargo test --release -p zawnabout-play --test probe NAME -- --ignored --nocapture`.

use zawnabout_sim::terrain::def::WORLD;
use zawnabout_sim::world::World;

/// Bridge decks over water or gaps: road, middle point, deck height and what is below.
#[test]
#[ignore]
fn gaps() {
    let w = World::generate(1);
    for (i, r) in WORLD.roads.iter().enumerate() {
        let prof = w.terrain.road_points(i);
        let mut k = 0;
        while k < prof.len() {
            if !prof[k].bridge {
                k += 1;
                continue;
            }
            let s = k;
            while k < prof.len() && prof[k].bridge {
                k += 1;
            }
            let m = (s + k - 1) / 2;
            let (a, b) = (r.pts[s.saturating_sub(1)], r.pts[k.min(r.pts.len() - 1)]);
            let low = prof[s..k].iter().map(|q| q.y).fold(f32::MAX, f32::min);
            println!(
                "  ends ({:.0}, {:.0}) ({:.0}, {:.0}) low deck {:.1}",
                a[0], a[1], b[0], b[1], low
            );
            let p = r.pts[m.min(r.pts.len() - 1)];
            let deck = prof[m].y;
            let below = w.support_still(p[0], p[1], deck - 4.0);
            println!(
                "bridge {} rail={} pts {}..{} mid ({:.0}, {:.0}) deck {:.1} below {:.1} w {}",
                r.id,
                r.rail,
                s,
                k - 1,
                p[0],
                p[1],
                deck,
                below,
                r.w
            );
        }
    }
    for m in WORLD.marks {
        if !m.kind.starts_with("perch") && !m.kind.starts_with("beacon") {
            println!(
                "mark {} {} ({:.0}, {:.0}, {:.0})",
                m.kind, m.id, m.pos[0], m.pos[1], m.pos[2]
            );
        }
    }
    for s in WORLD.spires {
        println!(
            "spire {} ({:.0}, {:.0}) h {} r {}",
            s.id, s.x, s.z, s.h, s.r
        );
    }
    for r in WORLD.routes {
        if ["ferry", "seaplane", "balloon", "cruise"].contains(&r.kind) {
            let pts: Vec<String> = r
                .pts
                .iter()
                .map(|p| format!("({:.0},{:.0},{:.0})", p[0], p[1], p[2]))
                .collect();
            println!("route {} {} {}", r.kind, r.id, pts.join(" "));
        }
    }
    use zawnabout_sim::town::Kind;
    for b in &w.buildings {
        if matches!(
            b.kind,
            Kind::Ferris | Kind::Lighthouse | Kind::Drawbridge | Kind::RopeBridge | Kind::SpanTower
        ) {
            println!("axis {:?} ({:.3}, {:.3})", b.kind, b.axis.x, b.axis.y);
            println!(
                "building {:?} ({:.0}, {:.0}, {:.0}) size {:.0} {:.0} {:.0}",
                b.kind, b.pos.x, b.pos.y, b.pos.z, b.size.x, b.size.y, b.size.z
            );
        }
    }
}

/// Ground and water across a gate's middle, along its crossing: GATE="x1 z1 x2 z2".
#[test]
#[ignore]
fn across() {
    let w = World::generate(1);
    let v: Vec<f32> = std::env::var("GATE")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let (a, b) = (glam::Vec2::new(v[0], v[1]), glam::Vec2::new(v[2], v[3]));
    let n = (b - a).perp().normalize();
    for t in [0.2f32, 0.35, 0.5, 0.65, 0.8] {
        let m = a.lerp(b, t);
        let row: Vec<String> = (-6..=6)
            .map(|k| {
                let p = m + n * (k as f32 * 5.0);
                let g = w.terrain.support(p.x, p.y, 200.0);
                let water = w.terrain.water(p.x, p.y, g).unwrap_or(-99.0);
                format!("{g:.0}/{water:.0}")
            })
            .collect();
        println!("t {t}: {}", row.join(" "));
    }
}

/// The nearest dry, open, flat spot to each of `DRY="x z [y];..."`, out of the water and
/// not on or in a building, for moving a find or a townsperson that landed badly.
#[test]
#[ignore]
fn dry() {
    let w = World::generate(1);
    for s in std::env::var("DRY").unwrap_or_default().split(';') {
        let v: Vec<f32> = s
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if v.len() < 2 {
            continue;
        }
        let below = v.get(2).map_or(5000.0, |y| y + 2.0);
        let mut found = None;
        'out: for r in 0..40 {
            let r = r as f32 * 2.5;
            let n = (r * 0.8).max(1.0) as usize;
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let (x, z) = (v[0] + a.cos() * r, v[1] + a.sin() * r);
                let y = w.support_still(x, z, below);
                let p = glam::Vec3::new(x, y, z);
                let flat = w.terrain.normal(x, z).y > 0.85 || v.len() > 2;
                // Clear for a couple of meters all round, so rounding never lands it on
                // a roof edge or a quay's lip.
                let clear = [(0.0, 0.0), (2.0, 0.0), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)]
                    .iter()
                    .all(|(dx, dz)| {
                        let (x, z) = (x + dx, z + dz);
                        let y2 = w.support_still(x, z, below);
                        let q = glam::Vec3::new(x, y2, z);
                        (y2 - y).abs() < 1.0
                            && !w.afloat(x, z, y2)
                            && !w.in_building(q - glam::Vec3::Y * 0.5)
                            && !w.in_building(q + glam::Vec3::Y)
                    });
                if clear && flat && (y > 0.8 || v.len() > 2) {
                    found = Some(p);
                    break 'out;
                }
            }
        }
        match found {
            Some(p) => println!("{s} -> x={:.0} z={:.0} (y {:.1})", p.x, p.z, p.y),
            None => println!("{s} -> nothing dry nearby"),
        }
    }
}
