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

/// What fills a town tile: triangles by building kind, near and far, and props.
/// `TILE="x z"` (meters, any point in the tile), same command as above with `tile_cost`.
#[test]
#[ignore]
fn tile_cost() {
    use std::collections::HashMap;
    let sim = Sim::new(1);
    let at: Vec<f32> = std::env::var("TILE")
        .unwrap_or("-250 800".into())
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let key = zawnabout_game::town_mesh::tile_key(glam::Vec2::new(at[0], at[1]));
    let mut kinds: HashMap<String, (usize, usize, usize)> = HashMap::new();
    for b in &sim.world.buildings {
        if zawnabout_game::town_mesh::tile_key(glam::Vec2::new(b.pos.x, b.pos.z)) != key {
            continue;
        }
        let (n, f) = zawnabout_game::town_mesh::building_cost(b, &sim.world.terrain);
        let e = kinds.entry(format!("{:?}", b.kind)).or_default();
        *e = (e.0 + 1, e.1 + n, e.2 + f);
    }
    for p in &sim.world.props {
        if zawnabout_game::town_mesh::tile_key(glam::Vec2::new(p.pos.x, p.pos.z)) != key {
            continue;
        }
        let vs = models::prop_variants(p.kind);
        let v = p.variant as usize % vs.len();
        let far = models::prop_far(p.kind, v as u32).map_or(0, |m| m.verts.len() / 3);
        let e = kinds.entry(format!("prop {:?}", p.kind)).or_default();
        *e = (e.0 + 1, e.1 + vs[v].verts.len() / 3, e.2 + far);
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.1));
    let total: usize = rows.iter().map(|r| r.1.1).sum();
    println!("tile {key:?} near {total}");
    for (k, (n, near, far)) in rows.iter().take(25) {
        println!("  {k:24} x{n:4} near {near:6} far {far:6}");
    }
}

/// The heaviest buildings in the world, near and far triangles.
#[test]
#[ignore]
fn heavy_buildings() {
    let sim = Sim::new(1);
    let mut rows: Vec<_> = sim
        .world
        .buildings
        .iter()
        .map(|b| {
            let (n, f) = zawnabout_game::town_mesh::building_cost(b, &sim.world.terrain);
            (n, f, format!("{:?}", b.kind), b.pos)
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.0));
    for (n, f, k, p) in rows.iter().take(40) {
        println!("{k:16} near {n:6} far {f:6} at {:.0} {:.0}", p.x, p.z);
    }
    let (near, far): (usize, usize) = rows.iter().fold((0, 0), |a, r| (a.0 + r.0, a.1 + r.1));
    println!("all buildings: near {near} far {far}");
    let over = |t: usize| rows.iter().filter(|r| r.0 > t).count();
    println!("over 1500: {}  over 3000: {}", over(1500), over(3000));
}
