//! Prop counts and mesh sizes, for budgeting.

use zawnabout_game::models;
use zawnabout_sim::{PropKind, Sim};

/// `cargo test --release -p zawnabout-game --test props -- --ignored --nocapture`
#[test]
#[ignore]
fn prop_table() {
    let sim = Sim::new(1);
    for kind in PropKind::ALL {
        let n = sim.world.props.iter().filter(|p| p.kind == kind).count();
        let near: Vec<usize> = models::prop_variants(kind)
            .iter()
            .map(|m| m.verts.len())
            .collect();
        let far: Vec<usize> = (0..near.len() as u32)
            .map(|v| models::prop_far(kind, v).map_or(0, |m| m.verts.len()))
            .collect();
        println!("{kind:?}: {n} props, near verts {near:?}, far verts {far:?}");
    }
}
