//! How turns feel: turn rate, bank and speed over a held turn.

#![cfg(feature = "probes")]

use zawnabout_sim::glam::Vec3;
use zawnabout_sim::{DT, DragonInput, Mode, Sim, TICK_HZ};

/// Prints turn rate over time for a few speeds and stick amounts. Run with
/// `cargo test --release --features probes --test turning -- --nocapture`.
#[test]
#[cfg(feature = "probes")]
fn turn_table() {
    let base = Sim::new(1);
    for speed in [30.0, 60.0, 110.0] {
        for amount in [0.3, 0.6, 1.0] {
            let mut sim = base.clone();
            let id = sim.spawn_dragon(4800.0, -4800.0, 0.0);
            let d = sim.dragon_mut(id).unwrap();
            d.mode = Mode::Air;
            d.pos.y = 300.0;
            d.vel = Vec3::new(0.0, 0.0, speed);
            let mut line = format!("v{speed:>4} in{amount:.1}:");
            let mut last = 0.0;
            // Keys ramp in at 4/s, as the client does.
            for i in 0..(3 * TICK_HZ) {
                let t = i as f32 * DT;
                let turn = (t * 4.0).min(1.0) * amount;
                sim.step(&[(
                    id,
                    DragonInput {
                        turn,
                        ..Default::default()
                    },
                )]);
                if (i + 1) % (TICK_HZ / 4) == 0 {
                    let d = sim.dragon(id).unwrap();
                    let rate = (d.heading - last) * 4.0;
                    last = d.heading;
                    line += &format!(" {:>5.0}", rate.to_degrees());
                }
            }
            let d = sim.dragon(id).unwrap();
            line += &format!(" | v {:.0} y {:.0}", d.speed(), d.pos.y);
            println!("{line}");
        }
    }
}
