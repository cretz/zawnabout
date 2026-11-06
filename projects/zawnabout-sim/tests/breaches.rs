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
        let nat = t.natural_height(x, z);
        println!(
            "{x} {z}: h {h:.0} natural {nat:.0} up {n:.2} {cover} road {road:?} water {water}"
        );
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
                "{:?} {:?} pos {} size {} roof {} axis {}",
                b.kind, b.style, b.pos, b.size, b.roof, b.axis
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

/// Flat dry spots off the roads for something `r` across, searched over a square:
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
                wet |= h < 2.0
                    || t.water(x + dx, z + dz, h).is_some()
                    || t.road_spot(x + dx, z + dz, 2.0).is_some();
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
    let min_h: f32 = std::env::var("MIN_H")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(f32::MIN);
    for (d, x, z, h, c) in found.iter().filter(|f| f.3 >= min_h).take(8) {
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
    // Optionally the first of that kind with at least this variant (`VARIANT=4`).
    let variant: u8 = std::env::var("VARIANT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if let Some(i) = w
        .traffic
        .vehicles
        .iter()
        .position(|v| format!("{:?}", v.kind) == want && v.variant >= variant)
    {
        println!("first {want} is {i}");
    }
}

/// Every run of bridge points on every road: where it starts and ends, how long, and its
/// deck heights: `cargo test --release --test breaches bridge_spans -- --ignored --nocapture`.
#[test]
#[ignore]
fn bridge_spans() {
    let t = Terrain::new(&WORLD);
    for (i, r) in t.def.roads.iter().enumerate() {
        let prof = t.road_points(i);
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
            let (a, b) = (r.pts[s], r.pts[k - 1]);
            let len: f32 = (s..k - 1)
                .map(|j| {
                    glam::Vec2::new(r.pts[j][0], r.pts[j][1])
                        .distance(glam::Vec2::new(r.pts[j + 1][0], r.pts[j + 1][1]))
                })
                .sum();
            let top = prof[s..k].iter().map(|p| p.y).fold(f32::MIN, f32::max);
            println!(
                "{} [{s}..{k}] ({:.0},{:.0})->({:.0},{:.0}) len {len:.0} ends {:.1}/{:.1} top {top:.1}",
                r.id,
                a[0],
                a[1],
                b[0],
                b[1],
                prof[s].y,
                prof[k - 1].y
            );
        }
    }
}

/// The suspension bridges and their towers:
/// `cargo test --release --test breaches suspensions -- --ignored --nocapture`.
#[test]
#[ignore]
fn suspensions() {
    let w = zawnabout_sim::World::generate(1);
    for b in &w.bridges {
        for t in b.towers {
            println!("tower at {} base {} top {}", t.at, t.base, t.top);
        }
        println!(
            "  cable points {} hangers {}",
            b.cables[0].len(),
            b.hangers.len()
        );
    }
    let towers = w
        .buildings
        .iter()
        .filter(|b| b.kind == zawnabout_sim::town::Kind::BridgeTower)
        .count();
    println!("{} bridges, {towers} tower buildings", w.bridges.len());
}

/// Each road's highest point and steepest stretch, to find where a tunnel belongs:
/// `cargo test --release --test breaches road_climbs -- --ignored --nocapture`.
#[test]
#[ignore]
fn road_climbs() {
    let t = Terrain::new(&WORLD);
    for (i, r) in t.def.roads.iter().enumerate() {
        let prof = t.road_points(i);
        let (mut top, mut at) = (f32::MIN, 0);
        let mut steep = 0.0f32;
        for k in 0..prof.len() {
            if prof[k].y > top {
                top = prof[k].y;
                at = k;
            }
            if k + 1 < prof.len() {
                let d = glam::Vec2::new(r.pts[k][0], r.pts[k][1])
                    .distance(glam::Vec2::new(r.pts[k + 1][0], r.pts[k + 1][1]));
                steep = steep.max((prof[k + 1].y - prof[k].y).abs() / d.max(1.0));
            }
        }
        println!(
            "{} top {top:.0} at ({:.0},{:.0}) steepest {:.0}%",
            r.id,
            r.pts[at][0],
            r.pts[at][1],
            steep * 100.0
        );
    }
}

/// The first few buildings of a kind, where they stand and which way:
/// `KIND=Pier cargo test --release --test breaches buildings_of -- --ignored --nocapture`.
#[test]
#[ignore]
fn buildings_of() {
    let w = zawnabout_sim::World::generate(1);
    let want = std::env::var("KIND").unwrap_or_default();
    for b in w
        .buildings
        .iter()
        .filter(|b| format!("{:?}", b.kind) == want)
        .take(6)
    {
        println!("{:?} pos {} axis {} size {}", b.kind, b.pos, b.axis, b.size);
    }
}

/// One road's profile point by point: `ROAD=fang-road cargo test --release --test
/// breaches road_profile -- --ignored --nocapture`.
#[test]
#[ignore]
fn road_profile() {
    let t = Terrain::new(&WORLD);
    let want = std::env::var("ROAD").unwrap_or_default();
    for (i, r) in t.def.roads.iter().enumerate() {
        if !r.id.contains(&want) {
            continue;
        }
        for (q, p) in r.pts.iter().zip(t.road_points(i)) {
            println!(
                "{} ({:.0},{:.0}) held {:.1} y {:.1} ground {:.1} bridge {} tunnel {}",
                r.id,
                q[0],
                q[1],
                q[2],
                p.y,
                t.height(q[0], q[1]),
                p.bridge,
                p.tunnel
            );
        }
    }
}

/// Heights over a grid and every road's profile, for drawing road maps:
/// `GRID="x0 z0 x1 z1 step" OUT=file cargo test --release --test breaches height_grid --
/// --ignored --nocapture`. Lines are `h x z y` for ground, `r id x z y bridge tunnel`.
#[test]
#[ignore]
fn height_grid() {
    use std::fmt::Write;
    let t = Terrain::new(&WORLD);
    let g: Vec<f32> = std::env::var("GRID")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    let mut out = String::new();
    let (mut z, step) = (g[1], g[4]);
    while z <= g[3] {
        let mut x = g[0];
        while x <= g[2] {
            // With NATURAL set, the ground before roads and rivers shape it.
            let h = if std::env::var("NATURAL").is_ok() {
                t.natural_height(x, z)
            } else {
                t.height(x, z)
            };
            let wet = t.water(x, z, h).is_some() as u8;
            writeln!(out, "h {x} {z} {h:.1} {wet}").unwrap();
            x += step;
        }
        z += step;
    }
    for (i, r) in t.def.roads.iter().enumerate() {
        for (q, p) in r.pts.iter().zip(t.road_points(i)) {
            writeln!(
                out,
                "r {} {:.1} {:.1} {:.1} {} {}",
                r.id, q[0], q[1], p.y, p.bridge as u8, p.tunnel as u8
            )
            .unwrap();
        }
    }
    // Landmarks and big buildings as boxes: `b kind x z axis_x axis_z width depth`.
    let w = zawnabout_sim::World::generate(1);
    for b in w.buildings.iter().filter(|b| b.size.x * b.size.z > 300.0) {
        writeln!(
            out,
            "b {:?} {:.1} {:.1} {:.3} {:.3} {:.1} {:.1}",
            b.kind, b.pos.x, b.pos.z, b.axis.x, b.axis.y, b.size.x, b.size.z
        )
        .unwrap();
    }
    std::fs::write(std::env::var("OUT").unwrap(), out).unwrap();
}

/// Cave floor heights over a grid, for laying paths through caverns:
/// `FLOOR="x0 z0 x1 z1 step ytop" OUT=file cargo test --release --test breaches
/// cave_floor -- --ignored`. Lines are `h x z y wet` as for `height_grid`, with `y` the
/// first floor below `ytop` (or -9999 where `ytop` is in rock).
#[test]
#[ignore]
fn cave_floor() {
    use std::fmt::Write;
    let t = Terrain::new(&WORLD);
    let g: Vec<f32> = std::env::var("FLOOR")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    let mut out = String::new();
    let (mut z, step, top) = (g[1], g[4], g[5]);
    while z <= g[3] {
        let mut x = g[0];
        while x <= g[2] {
            let p = glam::Vec3::new(x, top, z);
            let (y, wet) = if t.sdf(p) > 0.0 {
                let y = t.support(x, z, top);
                (y, t.cave_water(x, z, y).is_some() as u8)
            } else {
                (-9999.0, 0)
            };
            writeln!(out, "h {x} {z} {y:.1} {wet}").unwrap();
            x += step;
        }
        z += step;
    }
    std::fs::write(std::env::var("OUT").unwrap(), out).unwrap();
}

/// The floor and any water under spots in caves, for laying boat routes:
/// `WATER="x z ytop;x z ytop" cargo test --release --test breaches cave_water -- --ignored
/// --nocapture`.
#[test]
#[ignore]
fn cave_water() {
    let t = Terrain::new(&WORLD);
    for s in std::env::var("WATER").unwrap_or_default().split(';') {
        let v: Vec<f32> = s
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        if v.len() < 3 {
            continue;
        }
        let floor = t.support(v[0], v[1], v[2]);
        let water = t.cave_water(v[0], v[1], floor);
        println!("{} {}: floor {floor:.1} water {water:?}", v[0], v[1]);
    }
}

/// The solid shapes of the landmark nearest `NEAR="x z"`, with the support over each:
/// `cargo test --release --test breaches landmark_shapes -- --ignored --nocapture`.
#[test]
#[ignore]
fn landmark_shapes() {
    let w = zawnabout_sim::World::generate(1);
    let v: Vec<f32> = std::env::var("NEAR")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let p = glam::Vec2::new(v[0], v[1]);
    let b = w
        .buildings
        .iter()
        .min_by(|a, b| {
            let d =
                |x: &zawnabout_sim::town::Building| glam::Vec2::new(x.pos.x, x.pos.z).distance(p);
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    println!("{:?} pos {} axis {}", b.kind, b.pos, b.axis);
    for s in b.shapes() {
        println!("  {s:?}");
    }
}

/// Where every landmark of a kind stands: `KIND=Racetrack cargo test --release --test
/// breaches landmarks_of -- --ignored --nocapture`.
#[test]
#[ignore]
fn landmarks_of() {
    let w = zawnabout_sim::World::generate(1);
    let want = std::env::var("KIND").unwrap_or_default();
    for b in &w.buildings {
        if format!("{:?}", b.kind) == want {
            println!("{:?} {:?} pos {} size {}", b.kind, b.site, b.pos, b.size);
        }
    }
}

/// Landmarks inside a box, `BOX="x0 z0 x1 z1"`, with their footprints:
/// `cargo test --release --test breaches landmarks_in -- --ignored --nocapture`.
#[test]
#[ignore]
fn landmarks_in() {
    let w = zawnabout_sim::World::generate(1);
    let v: Vec<f32> = std::env::var("BOX")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    for b in &w.buildings {
        let named = b.site.is_some()
            && zawnabout_sim::landmark::KINDS
                .iter()
                .any(|(_, k)| *k == b.kind);
        if named && b.pos.x > v[0] && b.pos.x < v[2] && b.pos.z > v[1] && b.pos.z < v[3] {
            println!(
                "{:?} {:?} {:.0} {:.0} y {:.0} size {:.0} {:.0}",
                b.kind,
                b.site.unwrap_or(""),
                b.pos.x,
                b.pos.z,
                b.pos.y,
                b.size.x,
                b.size.z
            );
        }
    }
}

/// Probe: open spots for a new landmark in `AREA="x0 z0 x1 z1"`: no road within `R`
/// meters (default 50), no building, flat to `FLAT` meters, dry and under no rock.
#[test]
#[ignore]
fn open_spots() {
    let w = zawnabout_sim::World::generate(1);
    let t = &w.terrain;
    let env = |k: &str| std::env::var(k).ok();
    let area: Vec<f32> = env("AREA")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let r: f32 = env("R").and_then(|s| s.parse().ok()).unwrap_or(50.0);
    let flat: f32 = env("FLAT").and_then(|s| s.parse().ok()).unwrap_or(4.0);
    let step = r * 0.5;
    let mut z = area[1];
    while z < area[3] {
        let mut x = area[0];
        while x < area[2] {
            let hs: Vec<f32> = [
                (0.0, 0.0),
                (r, 0.0),
                (-r, 0.0),
                (0.0, r),
                (0.0, -r),
                (r, r),
                (-r, -r),
                (r, -r),
                (-r, r),
            ]
            .iter()
            .map(|(dx, dz)| t.height(x + dx * 0.8, z + dz * 0.8))
            .collect();
            let (lo, hi) = hs
                .iter()
                .fold((f32::MAX, f32::MIN), |a, h| (a.0.min(*h), a.1.max(*h)));
            let clear = t.road_spot(x, z, r).is_none()
                && w.buildings.iter().all(|b| {
                    glam::Vec2::new(b.pos.x - x, b.pos.z - z).length()
                        > r + b.size.x.max(b.size.z) * 0.5
                });
            if clear && hi - lo < flat && lo > 3.0 {
                println!("{x:.0} {z:.0}: h {lo:.0}..{hi:.0}");
            }
            x += step;
        }
        z += step;
    }
}
