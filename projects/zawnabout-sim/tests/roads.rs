//! Every road checked every 2 m for faults: ground poking through, things standing on
//! it, rock overhead, water over it, grades no car could climb, and cuttings and banks
//! deeper than a real road would dig.

use glam::{Vec2, Vec3, Vec3Swizzles};
use zawnabout_sim::World;
use zawnabout_sim::landmark::{Form, Piece};
use zawnabout_sim::town::{Building, Kind};
use zawnabout_sim::world::Collider;

/// One run of the same fault along a road.
struct Fault {
    road: &'static str,
    /// `poke` ground above the surface as drawn, `wall` something solid on the road, `lot`
    /// a landmark's piece (a car park, a plaza) laid over it, `rock` ground overhead
    /// outside a tunnel, `wet` water over the road, `steep` a grade over 14%, `deep` a
    /// cutting into the natural ground, `bank` an embankment over it, `cut` ground beside
    /// the shoulder far above or below the road.
    kind: &'static str,
    from: Vec2,
    to: Vec2,
    /// Meters along the road where it starts.
    at: f32,
    /// The worst of it: meters, percent, or the building index for `wall` and `lot`.
    worst: f32,
    /// Near the mouth of a tunnel, where a cutting is expected.
    portal: bool,
}

/// Every piece of every landmark, car parks and plazas included. Stations stand beside
/// their track by design, and the span tower over the road.
fn lots(w: &World) -> Vec<(usize, &Building, Vec<Piece>)> {
    w.buildings
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            zawnabout_sim::landmark::KINDS
                .iter()
                .any(|(_, k)| *k == b.kind)
                && !matches!(
                    b.kind,
                    Kind::SpanTower | Kind::Sign | Kind::Metro | Kind::MetroGate | Kind::Station
                )
        })
        .map(|(i, b)| {
            (
                i,
                b,
                zawnabout_sim::landmark::pieces(b.kind, b.seed, b.text),
            )
        })
        .collect()
}

/// Whether a spot on the road at height `rise` over the landmark's base is on one of
/// its pieces.
fn on_piece(pieces: &[Piece], local: Vec2, rise: f32) -> bool {
    pieces.iter().any(|pc| match pc.form {
        Form::Box { at, half, yaw, .. } => {
            let l = Vec2::from_angle(-yaw).rotate(local - Vec2::new(at.x, at.z));
            l.x.abs() < half.x
                && l.y.abs() < half.z
                && at.y < rise + 3.0
                && at.y + half.y > rise - 0.5
        }
        Form::Round { at, h, r0, r1 } => {
            local.distance(Vec2::new(at.x, at.z)) < r0.max(r1)
                && at.y < rise + 3.0
                && at.y + h > rise - 0.5
        }
        Form::Ball { at, r } => {
            local.distance(Vec2::new(at.x, at.z)) < r && (at.y - rise).abs() < r + 2.0
        }
        Form::Letter { .. } => false,
    })
}

fn audit(w: &World, only: &str) -> (Vec<Fault>, f32) {
    let t = &w.terrain;
    let lots = lots(w);
    let mut out = Vec::new();
    let mut length = 0.0f32;
    for (ri, r) in t.def.roads.iter().enumerate() {
        if !only.is_empty() && !r.id.contains(only) {
            continue;
        }
        let prof = t.road_points(ri);
        let half = r.w * 0.5;
        // Meters along each point, and how far each is from the nearest tunnel point.
        let mut along = vec![0.0f32; r.pts.len()];
        for i in 1..r.pts.len() {
            let (a, b) = (r.pts[i - 1], r.pts[i]);
            along[i] = along[i - 1] + Vec2::new(b[0] - a[0], b[1] - a[1]).length();
        }
        let tunnels: Vec<f32> = (0..r.pts.len())
            .filter(|&i| prof[i].tunnel)
            .map(|i| along[i])
            .collect();
        let near_portal = |s: f32| tunnels.iter().any(|t| (t - s).abs() < 60.0);
        let mut runs: Vec<Fault> = Vec::new();
        let mut note = |kind: &'static str, p: Vec2, at: f32, v: f32| {
            if let Some(last) = runs.iter_mut().rev().find(|f| f.kind == kind)
                && last.to.distance(p) < 12.0
            {
                last.to = p;
                if v.abs() > last.worst.abs() {
                    last.worst = v;
                }
                last.portal &= near_portal(at);
                return;
            }
            runs.push(Fault {
                road: r.id,
                kind,
                from: p,
                to: p,
                at,
                worst: v,
                portal: near_portal(at),
            });
        };
        for i in 0..r.pts.len() - 1 {
            let (a, b) = (
                Vec2::new(r.pts[i][0], r.pts[i][1]),
                Vec2::new(r.pts[i + 1][0], r.pts[i + 1][1]),
            );
            let len = a.distance(b);
            let dir = (b - a).normalize_or(Vec2::X);
            let side = Vec2::new(-dir.y, dir.x);
            let bridge = prof[i].bridge && prof[i + 1].bridge;
            let tunnel = prof[i].tunnel || prof[i + 1].tunnel;
            let grade = (prof[i + 1].y - prof[i].y).abs() / len.max(0.5);
            let steps = (len / 2.0).ceil().max(1.0) as usize;
            for k in 0..steps {
                let u = k as f32 / steps as f32;
                let p = a.lerp(b, u);
                let y = prof[i].y + (prof[i + 1].y - prof[i].y) * u;
                let at = along[i] + len * u;
                let natural = t.natural_height(p.x, p.y);
                if !bridge && !tunnel && natural - y > 8.0 {
                    note("deep", p, at, natural - y);
                }
                if !bridge && !tunnel && y - natural > 8.0 {
                    note("bank", p, at, y - natural);
                }
                if !r.rail && grade > 0.14 && !bridge {
                    note("steep", p, at, grade * 100.0);
                }
                for o in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
                    let q = p + side * o * half;
                    let g = t.height(q.x, q.y);
                    // As drawn: at the road's own height, just over the graded ground.
                    let drawn = y + 0.25;
                    // Ground over the surface, and solid there: at a tunnel's mouth the
                    // bore opens the hill above the height map.
                    if !tunnel
                        && (!bridge || g > y - 0.5)
                        && g > drawn + 0.02
                        && t.sdf(Vec3::new(q.x, drawn + 0.05, q.y)) < 0.0
                    {
                        note("poke", p, at, g - drawn);
                    }
                    for (bi, b, pieces) in &lots {
                        let d = q - Vec2::new(b.pos.x, b.pos.z);
                        if d.length() > b.size.x.max(b.size.z) {
                            continue;
                        }
                        let local = Vec2::new(d.dot(b.axis), b.axis.perp_dot(d));
                        if on_piece(pieces, local, y - b.pos.y) {
                            note("lot", p, at, *bi as f32);
                        }
                    }
                    let up = Vec3::new(q.x, y + 1.2, q.y);
                    if o.abs() < 0.9 {
                        let hit = w.near(up.x, up.z).find(|c| {
                            c.prop & Collider::BRIDGE == 0 && c.shape.push_out(up, 0.05).is_some()
                        });
                        if let Some(c) = hit {
                            let what = if c.prop & Collider::BUILDING != 0 {
                                let id = c.prop & !Collider::BUILDING;
                                w.buildings
                                    .iter()
                                    .position(|b| b.id == id)
                                    .map_or(-1.0, |b| b as f32)
                            } else {
                                // A prop: its kind, as a negative number.
                                w.props
                                    .iter()
                                    .find(|pr| pr.id == c.prop)
                                    .map_or(-1.0, |pr| -(pr.kind as u32 as f32) - 1.0)
                            };
                            note("wall", p, at, what);
                        }
                    }
                    if !tunnel && t.sdf(Vec3::new(q.x, y + 3.0, q.y)) < 0.0 && g > y + 3.0 {
                        note("rock", p, at, g - y);
                    }
                    // Underground, the space a vehicle passes through must be open.
                    if tunnel && t.sdf(Vec3::new(q.x, y + 2.5, q.y)) < -0.3 {
                        note("rock", p, at, -t.sdf(Vec3::new(q.x, y + 2.5, q.y)));
                    }
                    if !bridge
                        && !tunnel
                        && let Some(l) = t.water(q.x, q.y, g)
                        && l > y + 0.1
                    {
                        note("wet", p, at, l - y);
                    }
                }
                if !bridge && !tunnel {
                    for o in [-1.0f32, 1.0] {
                        let q = p + side * o * (half + 6.0);
                        let d = t.height(q.x, q.y) - y;
                        if d.abs() > 10.0 {
                            note("cut", q, at, d);
                        }
                    }
                }
            }
        }
        length += along[along.len() - 1];
        out.extend(runs);
    }
    (out, length)
}

/// What a fault names: the building in the way, if any.
fn what(w: &World, f: &Fault) -> String {
    if (f.kind == "wall" || f.kind == "lot") && f.worst >= 0.0 {
        let b = &w.buildings[f.worst as usize];
        format!(" {:?} {}", b.kind, b.site.unwrap_or(""))
    } else if f.kind == "wall" {
        let k = (-f.worst - 1.0) as u32;
        let prop = w.props.iter().find(|p| p.kind as u32 == k);
        format!(" prop {:?}", prop.map(|p| p.kind))
    } else {
        String::new()
    }
}

/// Faults no road may have: anything in the way, ground through it, water on it, a grade
/// no car could climb, or a cutting or bank far deeper than real roads dig (save at a
/// tunnel's mouth, where the cutting leads into the hill).
#[test]
fn roads_hold_up() {
    let w = World::generate(1);
    let (faults, _) = audit(&w, "");
    let bad: Vec<String> = faults
        .iter()
        .filter(|f| match f.kind {
            "wall" | "lot" | "rock" | "wet" | "steep" => true,
            "poke" => f.worst > 0.5,
            "deep" => f.worst > 25.0 && !f.portal,
            "bank" => f.worst > 12.0,
            _ => false,
        })
        .map(|f| {
            format!(
                "{} {} at {:.0}m ({:.0},{:.0}) {:.1}{}",
                f.road,
                f.kind,
                f.at,
                f.from.x,
                f.from.y,
                f.worst,
                what(&w, f)
            )
        })
        .collect();
    assert!(bad.is_empty(), "road faults:\n{}", bad.join("\n"));
}

/// Every fault on every road, for review:
/// `ROAD=fang cargo test --release --test roads road_audit -- --ignored --nocapture`.
#[test]
#[ignore]
fn road_audit() {
    let w = World::generate(1);
    let only = std::env::var("ROAD").unwrap_or_default();
    let (faults, length) = audit(&w, &only);
    for f in &faults {
        println!(
            "{:<22} {:<5} at {:>6.0}m ({:.0},{:.0})..({:.0},{:.0}) {:.2}{}{}",
            f.road,
            f.kind,
            f.at,
            f.from.x,
            f.from.y,
            f.to.x,
            f.to.y,
            f.worst,
            what(&w, f),
            if f.portal { " (portal)" } else { "" }
        );
    }
    println!("{} faults over {length:.0} m of road", faults.len());
}

/// Car trips: how many, how long, and the shortest:
/// `cargo test --release --test roads trips -- --ignored --nocapture`.
#[test]
#[ignore]
fn trips() {
    let w = World::generate(1);
    let places: Vec<Vec2> = w
        .buildings
        .iter()
        .map(|b| Vec2::new(b.pos.x, b.pos.z))
        .collect();
    let trips = zawnabout_sim::trips::car_trips(&w.terrain, &places);
    if let Ok(at) = std::env::var("AT") {
        let v: Vec<f32> = at
            .split_whitespace()
            .filter_map(|x| x.parse().ok())
            .collect();
        for r in w.terrain.def.roads {
            for (i, q) in r.pts.iter().enumerate() {
                if Vec2::new(q[0] - v[0], q[1] - v[1]).length() < 25.0 {
                    println!(
                        "  road {} pt {i}/{} {:.1} {:.1} w {}",
                        r.id,
                        r.pts.len(),
                        q[0],
                        q[1],
                        r.w
                    );
                }
            }
        }
    }
    let mut lens: Vec<f32> = trips.iter().map(|t| t.stops[0]).collect();
    lens.sort_by(f32::total_cmp);
    println!("{} trips", trips.len());
    let mut ends: Vec<(i32, i32, i32, i32)> = trips
        .iter()
        .map(|t| {
            let (a, b) = (t.pts[0], t.pts[t.pts.len() / 2]);
            (
                a.x as i32 / 50,
                a.z as i32 / 50,
                b.x as i32 / 50,
                b.z as i32 / 50,
            )
        })
        .collect();
    ends.sort();
    ends.dedup();
    println!("{} different trips", ends.len());
    if !lens.is_empty() {
        println!(
            "one way: min {:.0} median {:.0} max {:.0}",
            lens[0],
            lens[lens.len() / 2],
            lens[lens.len() - 1]
        );
    }
    for (k, trip) in trips.iter().enumerate() {
        let p = &trip.pts;
        let mut gap = 0.0f32;
        let mut kinks = Vec::new();
        for i in 1..p.len() - 1 {
            gap = gap.max(p[i].distance(p[i - 1]));
            let (a, b) = (p[i] - p[i - 1], p[i + 1] - p[i]);
            let (a, b) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z));
            if a.length() > 0.3 && b.length() > 0.3 {
                let ang = a.angle_to(b).abs().to_degrees();
                if ang > 45.0 && (p[i + 1].distance(p[i]) < 12.0 || p[i].distance(p[i - 1]) < 12.0)
                {
                    kinks.push(format!("{:.0}deg at {:.0} {:.0}", ang, p[i].x, p[i].z));
                }
            }
        }
        if k == 6 {
            for q in p
                .iter()
                .filter(|q| q.distance(Vec3::new(-1748.0, q.y, 1560.0)) < 30.0)
            {
                println!("  pt {:.1} {:.1} {:.1}", q.x, q.y, q.z);
            }
        }
        if gap > 40.0 || kinks.len() > 2 {
            println!("trip {k}: gap {gap:.0} kinks {}", kinks.join(", "));
        }
    }
}

/// One car's trip over time: `CAR=5 cargo test --release --test roads car_trace -- --ignored --nocapture`.
#[test]
#[ignore]
fn car_trace() {
    let w = World::generate(1);
    let k: usize = std::env::var("CAR")
        .ok()
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let i = w
        .traffic
        .vehicles
        .iter()
        .enumerate()
        .filter(|(_, v)| v.kind == Kind::Car)
        .nth(k)
        .unwrap()
        .0;
    let v = &w.traffic.vehicles[i];
    let path = &w.traffic.paths[v.path];
    println!(
        "stops {:?} len {:.0} lap {:.0}",
        path.stops,
        path.len(),
        path.lap_time(v.speed)
    );
    for t in 0..240 {
        let p = w.traffic.pose(v, t as f32);
        println!(
            "t {t} pos {:.0} {:.0} speed {:.1} heading {:.2}",
            p.pos.x,
            p.pos.z,
            p.vel.length(),
            p.heading
        );
    }
}

/// When boats pass a spot, `AT="x z"`: `cargo test --release --test roads boats_by -- --ignored --nocapture`.
#[test]
#[ignore]
fn boats_by() {
    let w = World::generate(1);
    let v: Vec<f32> = std::env::var("AT")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    let at = Vec2::new(v[0], v[1]);
    for t in 0..600 {
        for (i, veh) in w.traffic.vehicles.iter().enumerate() {
            if !matches!(
                veh.kind,
                Kind::Sailboat | Kind::Trawler | Kind::Ferry | Kind::Cruise
            ) {
                continue;
            }
            let p = w.traffic.pose(veh, t as f32).pos;
            let d = Vec2::new(p.x, p.z).distance(at);
            if d < 60.0 {
                println!("t {t} vehicle {i} {:?} d {d:.0}", veh.kind);
            }
        }
    }
}

/// Helicopters: where they stop, and how close they pass over the ground:
/// `cargo test --release --test roads helicopters -- --ignored --nocapture`.
#[test]
#[ignore]
fn helicopters() {
    let w = World::generate(1);
    for (i, v) in w.traffic.vehicles.iter().enumerate() {
        if v.kind != Kind::Helicopter {
            continue;
        }
        let path = &w.traffic.paths[v.path];
        let lap = path.lap_time(v.speed);
        println!("heli {i}: lap {lap:.0}s, stops:");
        for s in &path.stops {
            let (p, _) = path.at(*s);
            println!("  {:.0} {:.1} {:.0}", p.x, p.y, p.z);
        }
        let mut low = (f32::MAX, Vec3::ZERO);
        let mut t = 0.0;
        while t < lap {
            let p = w.traffic.pose(v, t).pos;
            let clear = p.y - w.terrain.height(p.x, p.z).max(0.0);
            let away = path.stops.iter().all(|s| path.at(*s).0.distance(p) > 80.0);
            if clear < low.0 && away {
                low = (clear, p);
            }
            t += 0.5;
        }
        println!(
            "  lowest clearance {:.1} at {:.0} {:.0} {:.0}",
            low.0, low.1.x, low.1.y, low.1.z
        );
    }
}

/// Where cars dip into the road or float over it, near `AT="x z"` (200 m round): each
/// one's ends against the ground, worst first. `cargo test --release --test roads sinking -- --ignored --nocapture`.
#[test]
#[ignore]
fn sinking() {
    use zawnabout_sim::traffic::{BUS, TRUCK};
    let w = World::generate(1);
    let v: Vec<f32> = std::env::var("AT")
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let spot = Vec2::new(v[0], v[1]);
    let mut worst = Vec::new();
    for v in &w.traffic.vehicles {
        if v.kind != Kind::Car {
            continue;
        }
        let mut bad = (0.0f32, Vec3::ZERO, 0.0f32);
        for t in 0..1200 {
            let p = w.traffic.pose(v, t as f32 * 0.5);
            let fwd = Vec3::new(p.heading.sin(), 0.0, p.heading.cos());
            for e in [-0.5f32, 0.0, 0.5] {
                let along = e * v.size.x;
                let at = p.pos + fwd * along * p.pitch.cos() + Vec3::Y * along * p.pitch.sin();
                let g = w.ground_height(at.x, at.z);
                let d = at.y - g;
                if at.xz().distance(spot) < 200.0 && d.abs() > bad.0.abs() {
                    bad = (d, at, t as f32 * 0.5);
                }
            }
        }
        worst.push((v.variant, bad));
    }
    worst.sort_by(|a, b| b.1.0.abs().total_cmp(&a.1.0.abs()));
    for (var, (d, at, t)) in worst.iter().take(20) {
        println!(
            "{} {d:+.2} at {:.0} {:.0} {:.0} t {t}",
            if *var == BUS {
                "bus".to_string()
            } else if *var == TRUCK {
                "truck".to_string()
            } else {
                format!("car{var}")
            },
            at.x,
            at.y,
            at.z
        );
    }
}

/// Every vehicle near `AT="x z"` at time `T`: `cargo test --release --test roads near -- --ignored --nocapture`.
#[test]
#[ignore]
fn near() {
    let w = World::generate(1);
    let v: Vec<f32> = std::env::var("AT")
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let t: f32 = std::env::var("T")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(15.0);
    for (i, veh) in w.traffic.vehicles.iter().enumerate() {
        let p = w.traffic.pose(veh, t);
        let d = p.pos.xz().distance(Vec2::new(v[0], v[1]));
        if d < 150.0 {
            println!(
                "{i} {:?} var {} size {:?} pos {:.1} {:.1} {:.1} pitch {:.2} ground {:.1}",
                veh.kind,
                veh.variant,
                veh.size,
                p.pos.x,
                p.pos.y,
                p.pos.z,
                p.pitch,
                w.ground_height(p.pos.x, p.pos.z)
            );
        }
    }
}

/// How far each drawbridge is lifted over time: `cargo test --release --test roads draw_lifts -- --ignored --nocapture`.
#[test]
#[ignore]
fn draw_lifts() {
    let mut w = World::generate(1);
    for t in 0..120 {
        w.traffic.update(t as f32);
        let lifts: Vec<String> = w
            .drawbridges
            .iter()
            .map(|&i| {
                let b = &w.buildings[i];
                format!("{:.2}", w.traffic.draw_lift(Vec2::new(b.pos.x, b.pos.z)))
            })
            .collect();
        println!("t {t} {}", lifts.join(" "));
    }
}

/// How far the ground pokes through or sags under each landmark's footprint, `KIND=waterpark`:
/// `cargo test --release --test roads ground_under -- --ignored --nocapture`.
#[test]
#[ignore]
fn ground_under() {
    let w = World::generate(1);
    let kind = std::env::var("KIND").unwrap_or("waterpark".into());
    for b in &w.buildings {
        if !format!("{:?}", b.kind).eq_ignore_ascii_case(&kind) {
            continue;
        }
        let (mut hi, mut lo) = (f32::MIN, f32::MAX);
        let (mut at_hi, mut at_lo) = (Vec2::ZERO, Vec2::ZERO);
        for i in -14..=14 {
            for j in -10..=10 {
                let (lx, lz) = (i as f32 * 5.0, j as f32 * 5.0);
                let p = Vec2::new(b.pos.x, b.pos.z) + b.axis * lx + b.front() * lz;
                let d = w.ground_height(p.x, p.y) - b.pos.y;
                if d > hi {
                    (hi, at_hi) = (d, Vec2::new(lx, lz));
                }
                if d < lo {
                    (lo, at_lo) = (d, Vec2::new(lx, lz));
                }
            }
        }
        println!(
            "{:?} at {:.0} {:.0} {:.0}: ground {hi:+.2} at {at_hi} .. {lo:+.2} at {at_lo}",
            b.kind, b.pos.x, b.pos.y, b.pos.z
        );
    }
}

/// Boats keep to open water: every boat checked every 2 s for ten minutes against the
/// ground under it and a little round it.
#[test]
fn boats_afloat() {
    let w = World::generate(1);
    let mut bad = Vec::new();
    for v in &w.traffic.vehicles {
        if !matches!(
            v.kind,
            Kind::Speedboat
                | Kind::MotorYacht
                | Kind::Sailboat
                | Kind::Trawler
                | Kind::Cruise
                | Kind::Supply
        ) {
            continue;
        }
        for t in 0..300 {
            let p = w.traffic.pose(v, t as f32 * 2.0);
            let r = v.size.z * 0.5 + 2.0;
            let aground = [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]
                .iter()
                .any(|(dx, dz)| w.ground_height(p.pos.x + dx, p.pos.z + dz) > p.pos.y - 1.5);
            if aground {
                bad.push(format!("{:?} at {:.0} {:.0}", v.kind, p.pos.x, p.pos.z));
                break;
            }
        }
    }
    for b in &bad {
        println!("{b}");
    }
    assert!(bad.is_empty(), "{} boats aground", bad.len());
}

/// Balloons on routes through a balloon field come down onto its grass, sit there a
/// while, and go back up high.
#[test]
fn balloons_land_on_the_field() {
    let w = World::generate(1);
    let landing: Vec<_> = w
        .traffic
        .vehicles
        .iter()
        .filter(|v| v.kind == Kind::Balloon && !w.traffic.paths[v.path].stops.is_empty())
        .collect();
    assert!(landing.len() >= 2, "no balloons land on a field");
    for v in landing {
        let (mut sat, mut top) = (0, f32::MIN);
        for t in 0..1200 {
            let p = w.traffic.pose(v, t as f32);
            let ground = w.terrain.height(p.pos.x, p.pos.z);
            if (p.pos.y - ground).abs() < 0.6 {
                sat += 1;
            }
            assert!(
                p.pos.y > ground - 0.6,
                "balloon in the ground at {:?}",
                p.pos
            );
            top = top.max(p.pos.y - ground);
        }
        assert!(sat >= 30, "balloon sat down only {sat} s");
        assert!(top > 150.0, "balloon only rose {top:.0} m");
    }
}

/// Buses pull in at the bus stops along their way, not only at the ends of their trips.
#[test]
fn buses_call_at_stops() {
    let w = World::generate(1);
    let stops: Vec<Vec2> = w
        .buildings
        .iter()
        .filter(|b| b.kind == Kind::BusStop)
        .map(|b| b.pos.xz())
        .collect();
    let buses: Vec<_> = w
        .traffic
        .vehicles
        .iter()
        .filter(|v| v.kind == Kind::Car && v.variant == zawnabout_sim::traffic::BUS)
        .collect();
    let calling = buses
        .iter()
        .filter(|v| {
            let p = &w.traffic.paths[v.path];
            p.stops.iter().any(|&s| {
                let at = p.at(s).0.xz();
                stops.iter().any(|b| b.distance(at) < 10.0)
            })
        })
        .count();
    println!("{calling} of {} buses call at a bus stop", buses.len());
    assert!(
        calling * 2 >= buses.len(),
        "{calling} of {} buses call at stops",
        buses.len()
    );
}

/// Probe: parcel vans halted at a stop, a couple of seconds in, after `T` seconds.
#[test]
#[ignore]
fn vans() {
    let w = World::generate(1);
    let from: f32 = std::env::var("T")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20.0);
    for v in &w.traffic.vehicles {
        if v.kind != Kind::Car || v.variant != zawnabout_sim::traffic::VAN {
            continue;
        }
        for k in 0..400 {
            let t = from + k as f32 * 0.5;
            if w.traffic
                .halted_for(v, t)
                .is_some_and(|h| (2.0..2.5).contains(&h))
            {
                let p = w.traffic.pose(v, t);
                println!(
                    "t {t} at {:.1} {:.1} {:.1} heading {:.2}",
                    p.pos.x, p.pos.y, p.pos.z, p.heading
                );
                break;
            }
        }
    }
}
