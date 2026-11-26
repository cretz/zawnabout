//! Walking everywhere: the dragon set down on all sorts of ground, running up, down and
//! across slopes, must never end up inside rock or drop through the ground into a hollow
//! under it.

use zawnabout_sim::glam::Vec3;
use zawnabout_sim::{DT, DragonInput, Mode, Sim, TICK_HZ};

/// Where the dragon went wrong on a walk from `x z` heading `heading`, if it did. The
/// sim is reused walk after walk: copying a whole world for each is slow.
fn walk(sim: &mut Sim, x: f32, z: f32, heading: f32) -> Option<String> {
    sim.dragons.clear();
    let id = sim.spawn_dragon(x, z, heading);
    sim.dragon_mut(id)?.mode = Mode::Ground;
    let stand = sim.flight.stand_height;
    let mut was = sim.dragon(id)?.pos;
    let start = was - Vec3::Y * stand;
    for i in 0..(5 * TICK_HZ) {
        let t = i as f32 * DT;
        let turn = if (t as u32).is_multiple_of(2) {
            0.3
        } else {
            -0.3
        };
        sim.step(&[(
            id,
            DragonInput {
                pitch: 1.0,
                turn,
                ..Default::default()
            },
        )]);
        let d = sim.dragon(id)?;
        let feet = d.pos - Vec3::Y * stand;
        let w = &sim.world;
        let body = w.terrain.sdf(d.pos);
        if d.mode == Mode::Ground && body < -0.8 {
            return Some(format!(
                "in rock at {:.0} {:.0} {:.0} (sdf {body:.1})",
                d.pos.x, d.pos.y, d.pos.z
            ));
        }
        // Well under the open ground with rock overhead: fell into a hollow. Roads and
        // tracks run into tunnels on purpose, and a walk that starts under an overhang
        // is under it all along.
        let top = w.terrain.height(d.pos.x, d.pos.z);
        let road = w.terrain.road_spot(d.pos.x, d.pos.z, 4.0).is_some();
        let covered = |p: Vec3| {
            let top = w.terrain.height(p.x, p.z);
            p.y < top - 6.0 && w.terrain.sdf(p + Vec3::Y * ((top - p.y) * 0.5)) < 0.0
        };
        // Inside a cave on purpose: walked in through its mouth.
        let cave = || {
            w.terrain
                .def
                .voids
                .iter()
                .any(|v| zawnabout_sim::terrain::void_sdf(v, d.pos) < 0.0)
        };
        if !road && !covered(start) && covered(feet) && was.y - d.pos.y > 4.0 && !cave() {
            return Some(format!(
                "under the ground at {:.0} {:.0} {:.0}, surface {top:.0}, last on the ground at {:.0} {:.0} {:.0}",
                d.pos.x, d.pos.y, d.pos.z, was.x, was.y, was.z
            ));
        }
        if d.mode == Mode::Ground {
            was = d.pos;
        }
    }
    None
}

/// Walks over a grid of spots on every island, three ways each, and along every road
/// both ways.
#[test]
fn walk_everywhere() {
    let base = Sim::new(1);
    let def = base.world.terrain.def;
    // Every walk to take: where, which way, and what to call it.
    let mut todo: Vec<(f32, f32, f32, String)> = Vec::new();
    let step = 160.0;
    let mut x = def.bounds[0];
    while x < def.bounds[2] {
        let mut z = def.bounds[1];
        while z < def.bounds[3] {
            let h = base.world.terrain.height(x, z);
            if h > def.sea_level + 2.0 {
                for heading in [0.0f32, 2.1, 4.2] {
                    todo.push((x, z, heading, String::new()));
                }
            }
            z += step;
        }
        x += step;
    }
    // Along every road, both ways, from every tenth point: tunnel mouths and cuttings.
    for r in def.roads.iter().filter(|r| !r.runway) {
        for (i, w) in r.pts.windows(2).enumerate().step_by(10) {
            let heading = (w[1][0] - w[0][0]).atan2(w[1][1] - w[0][1]);
            for back in [0.0, std::f32::consts::PI] {
                todo.push((
                    w[0][0],
                    w[0][1],
                    heading + back,
                    format!("{} pt {i}: ", r.id),
                ));
            }
        }
    }
    let walks = todo.len();
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(16);
    let chunk = walks.div_ceil(threads);
    let bad: Vec<String> = std::thread::scope(|s| {
        let jobs: Vec<_> = todo
            .chunks(chunk)
            .map(|part| {
                let mut sim = base.clone();
                s.spawn(move || {
                    part.iter()
                        .filter_map(|(x, z, h, name)| {
                            walk(&mut sim, *x, *z, *h).map(|why| format!("{name}{why}"))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        jobs.into_iter().flat_map(|j| j.join().unwrap()).collect()
    });
    assert!(
        bad.is_empty(),
        "{} of {walks} walks went wrong:
{}",
        bad.len(),
        bad.join(
            "
"
        )
    );
}

/// One walk with every tick printed, and what the walking checks see: `WALK="x z heading"`.
#[test]
#[cfg(feature = "probes")]
fn walk_trace() {
    let Ok(v) = std::env::var("WALK") else {
        return;
    };
    let v: Vec<f32> = v
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let mut sim = Sim::new(1);
    let id = sim.spawn_dragon(v[0], v[1], v[2]);
    // A fourth number sets it down at that height, in a cave under the surface.
    if let Some(&y) = v.get(3) {
        let floor = sim.world.support(v[0], v[1], y + 3.0);
        let stand = sim.flight.stand_height;
        sim.dragon_mut(id).unwrap().pos.y = floor + stand;
    }
    sim.dragon_mut(id).unwrap().mode = Mode::Ground;
    let r = sim.flight.body_radius;
    for i in 0..(4 * TICK_HZ) {
        sim.step(&[(
            id,
            DragonInput {
                pitch: 1.0,
                ..Default::default()
            },
        )]);
        if i % 10 == 0 {
            let d = sim.dragon(id).unwrap();
            let w = &sim.world;
            let rock = w.terrain.push_out(d.pos + Vec3::Y, r).map(|(n, _)| n);
            let props = w.push_out_props(d.pos + Vec3::Y * 0.4, r).1;
            if props != Vec3::ZERO {
                for c in w.near(d.pos.x, d.pos.z) {
                    if c.shape.push_out(d.pos + Vec3::Y * 0.4, r).is_some() {
                        println!("  hits prop {:x} {:?}", c.prop, c.shape);
                    }
                }
            }
            println!(
                "{:.0} {:.1} {:.0} {:?} v{:.1} rock {:?} props {:?} ground {:.1} normal {:.2}",
                d.pos.x,
                d.pos.y,
                d.pos.z,
                d.mode,
                d.vel.length(),
                rock,
                props,
                w.ground_height(d.pos.x, d.pos.z),
                w.ground_normal(d.pos.x, d.pos.z).y
            );
        }
    }
}
