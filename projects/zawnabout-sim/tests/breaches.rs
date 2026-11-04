//! Places where a cave opens through the ground. A hole under the sea is always a mistake:
//! a passage that strayed under the seabed.

use glam::Vec3;
use std::collections::BTreeMap;
use zawnabout_sim::terrain::{Terrain, WORLD, void_sdf};

/// Openings by nearest void: cells, cells under the sea, and the last spot under the sea.
fn breaches(t: &Terrain) -> BTreeMap<&'static str, (u32, u32, [f32; 3])> {
    let d = t.def;
    let step = 6.0;
    let mut hits: BTreeMap<&str, (u32, u32, [f32; 3])> = BTreeMap::new();
    let mut x = d.bounds[0];
    while x < d.bounds[2] {
        let mut z = d.bounds[1];
        while z < d.bounds[3] {
            let h = t.height(x, z);
            let p = Vec3::new(x, h - 1.5, z);
            if t.sdf(p) > 0.0 {
                let near = d
                    .voids
                    .iter()
                    .min_by(|a, b| void_sdf(a, p).total_cmp(&void_sdf(b, p)))
                    .map_or("channel", |v| v.id);
                let e = hits.entry(near).or_insert((0, 0, [x, h, z]));
                e.0 += 1;
                if h < d.sea_level {
                    e.1 += 1;
                    e.2 = [x, h, z];
                }
            }
            z += step;
        }
        x += step;
    }
    hits
}

#[test]
fn no_sea_holes() {
    let t = Terrain::new(&WORLD);
    let bad: Vec<_> = breaches(&t)
        .into_iter()
        .filter(|(_, (_, sea, _))| *sea > 0)
        .collect();
    assert!(bad.is_empty(), "caves open under the sea: {bad:?}");
}

/// Run with `--ignored --nocapture` to list every opening.
#[test]
#[ignore]
fn list_breaches() {
    let t = Terrain::new(&WORLD);
    for (id, (n, sea, at)) in breaches(&t) {
        println!(
            "{id}: {n} cells, {sea} under sea, at {:.0} {:.0} {:.0}",
            at[0], at[1], at[2]
        );
    }
}

/// Prints ground heights on a line: `PROBE="x0 z0 x1 z1 n"` in world space.
#[test]
#[ignore]
fn probe() {
    let t = Terrain::new(&WORLD);
    let v: Vec<f32> = std::env::var("PROBE")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let [x0, z0, x1, z1, n] = v[..] else { return };
    for i in 0..=n as usize {
        let f = i as f32 / n;
        let (x, z) = (x0 + (x1 - x0) * f, z0 + (z1 - z0) * f);
        println!("{x:.0} {z:.0}: {:.0}", t.height(x, z));
    }
}

/// Prints rock distance up a column: `COLUMN="x z y0 y1 n"` in world space.
#[test]
#[ignore]
fn column() {
    let t = Terrain::new(&WORLD);
    let v: Vec<f32> = std::env::var("COLUMN")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let [x, z, y0, y1, n] = v[..] else { return };
    for i in 0..=n as usize {
        let y = y0 + (y1 - y0) * i as f32 / n;
        let p = zawnabout_sim::glam::Vec3::new(x, y, z);
        println!(
            "{y:.1}: {:.2} water {:?}",
            t.sdf(p),
            t.cave_water(x, z, y - 5.0)
        );
    }
}

/// Run with `--ignored --nocapture` to see how long the world takes to build.
#[test]
#[ignore]
fn world_time() {
    let t0 = std::time::Instant::now();
    let w = zawnabout_sim::World::generate(1);
    let mut counts = std::collections::BTreeMap::new();
    for p in &w.props {
        *counts.entry(format!("{:?}", p.kind)).or_insert(0) += 1;
    }
    println!(
        "generate {:?} {counts:?} beacons {}",
        t0.elapsed(),
        w.beacons.len()
    );
}

#[test]
#[ignore]
fn decor_time() {
    let t = Terrain::new(&WORLD);
    let t0 = std::time::Instant::now();
    let n = zawnabout_sim::cave::decorate(&t, 1).len();
    println!("decorate {:?} {n}", t0.elapsed());
}

/// Boats stay on water and planes clear the ground.
#[test]
fn boats_stay_afloat() {
    let w = zawnabout_sim::World::generate(1);
    let t = &w.terrain;
    let mut bad = Vec::new();
    for v in &w.traffic.vehicles {
        use zawnabout_sim::town::Kind;
        let boat = matches!(v.kind, Kind::Ferry | Kind::Sailboat);
        if !boat && !matches!(v.kind, Kind::Seaplane | Kind::Airliner | Kind::Balloon) {
            continue;
        }
        let path = &w.traffic.paths[v.path];
        let mut s = 0.0;
        while s < path.len() {
            let (p, _) = path.at(s);
            let h = t.height(p.x, p.z);
            // Boats need water under them, planes room under them once in the air.
            // Planes take off and land low, but once up they keep well clear.
            let low = if boat {
                h > -2.0
            } else {
                p.y > 60.0 && h > p.y - 20.0
            };
            if low {
                bad.push((v.id, p.x as i32, p.z as i32, h as i32));
            }
            s += 20.0;
        }
    }
    let mut last = (i32::MIN, 0);
    for b in &bad {
        if (b.1 - last.0).abs() + (b.2 - last.1).abs() > 60 {
            println!("vehicle {} over land from {} {}", b.0, b.1, b.2);
        }
        last = (b.1, b.2);
    }
    assert!(bad.is_empty(), "{} spots over land", bad.len());
}

#[test]
#[ignore]
fn height_time() {
    let t = Terrain::new(&WORLD);
    let h = t.half_size();
    let t0 = std::time::Instant::now();
    let mut land = 0;
    let n = 200;
    for i in 0..n {
        for j in 0..n {
            let x = -h + (i as f32 + 0.5) / n as f32 * 2.0 * h;
            let z = -h + (j as f32 + 0.5) / n as f32 * 2.0 * h;
            if t.height(x, z) > 2.0 {
                land += 1;
            }
        }
    }
    println!("{} heights {:?}, land {land}", n * n, t0.elapsed());
}

/// Height, slope, cover, water and roads at spots, for placing things by hand. Run with
/// `SPOTS="x z;x z" cargo test --release --test breaches spots -- --ignored --nocapture`.
#[test]
#[ignore]
fn spots() {
    let t = Terrain::new(&WORLD);
    let list = std::env::var("SPOTS").unwrap_or_default();
    for s in list.split(';').filter(|s| !s.trim().is_empty()) {
        let v: Vec<f32> = s
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (x, z) = (v[0], v[1]);
        let h = t.height(x, z);
        let n = t.normal(x, z).y;
        let cover = t
            .cover(x, z)
            .map(|c| format!("{:?}", c.kind))
            .unwrap_or_default();
        let road = t.road_spot(x, z, 30.0).map(|r| r.d as i32);
        let water = t.water(x, z, h).is_some();
        println!("{x} {z}: h {h:.0} up {n:.2} {cover} road {road:?} water {water}");
    }
}

/// Buildings near a spot and their solid shapes:
/// `NEAR="x z" cargo test --release --test breaches near_buildings -- --ignored --nocapture`.
#[test]
#[ignore]
fn near_buildings() {
    let w = zawnabout_sim::World::generate(1);
    let v: Vec<f32> = std::env::var("NEAR")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let p = glam::Vec2::new(v[0], v[1]);
    for b in &w.buildings {
        if glam::Vec2::new(b.pos.x, b.pos.z).distance(p) < 40.0 {
            println!(
                "{:?} {:?} pos {} size {} roof {}",
                b.kind, b.style, b.pos, b.size, b.roof
            );
            println!("  support {}", w.support(p.x, p.y, 1e4));
        }
    }
}

/// Every road end that meets no other road, and what it is in:
/// `cargo test --release --test breaches road_ends -- --ignored --nocapture`.
#[test]
#[ignore]
fn road_ends() {
    let t = Terrain::new(&WORLD);
    let roads = t.def.roads;
    for (ri, r) in roads.iter().enumerate() {
        if r.runway {
            continue;
        }
        let n = r.pts.len();
        for end in [0, n - 1] {
            let p = glam::Vec2::new(r.pts[end][0], r.pts[end][1]);
            let other = glam::Vec2::new(r.pts[n - 1 - end][0], r.pts[n - 1 - end][1]);
            let joins = other.distance(p) < r.w
                || roads.iter().enumerate().any(|(oi, o)| {
                    oi != ri
                        && o.pts
                            .iter()
                            .any(|q| glam::Vec2::new(q[0], q[1]).distance(p) < o.w + r.w * 0.5)
                });
            if !joins {
                let what = t.cover(p.x, p.y).map(|c| c.id).unwrap_or("open");
                println!("{} end {end} at {p}: {what}", r.id);
            }
        }
    }
}

/// Flat dry spots for something `r` across, searched over a square:
/// `FLAT="cx cz half step r" cargo test --release --test breaches flat_spots -- --ignored --nocapture`.
#[test]
#[ignore]
fn flat_spots() {
    let t = Terrain::new(&WORLD);
    let v: Vec<f32> = std::env::var("FLAT")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let (cx, cz, half, step, r) = (v[0], v[1], v[2], v[3], v[4]);
    let mut found = Vec::new();
    let mut z = cz - half;
    while z <= cz + half {
        let mut x = cx - half;
        while x <= cx + half {
            let mut lo = f32::MAX;
            let mut hi = f32::MIN;
            let mut wet = false;
            for k in 0..17 {
                let (dx, dz) = if k == 16 {
                    (0.0, 0.0)
                } else {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                    let rr = if k % 2 == 0 { r } else { r * 0.5 };
                    (a.cos() * rr, a.sin() * rr)
                };
                let h = t.height(x + dx, z + dz);
                lo = lo.min(h);
                hi = hi.max(h);
                wet |= h < 2.0 || t.water(x + dx, z + dz, h).is_some();
            }
            if !wet {
                let cover = t.cover(x, z).map(|c| c.id).unwrap_or("open");
                found.push((hi - lo, x, z, lo, cover));
            }
            x += step;
        }
        z += step;
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (d, x, z, h, c) in found.iter().take(8) {
        println!("{x} {z}: spread {d:.1} h {h:.0} {c}");
    }
}

/// The index of the first vehicle of a kind:
/// `KIND=Train cargo test --release --test breaches first_vehicle -- --ignored --nocapture`.
#[test]
#[ignore]
fn first_vehicle() {
    let w = zawnabout_sim::World::generate(1);
    let want = std::env::var("KIND").unwrap_or_default();
    if let Some(i) = w
        .traffic
        .vehicles
        .iter()
        .position(|v| format!("{:?}", v.kind) == want)
    {
        println!("first {want} is {i}");
    }
}
