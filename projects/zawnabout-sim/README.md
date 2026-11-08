# zawnabout-sim

The deterministic game simulation: terrain, world, flight, traffic and landmarks. No rendering, devices or wall clock, so the same inputs replay the same run. Its build script compiles `zawnabout-world/` with `zawnabout-worldc`.

`cargo test --release -p zawnabout-sim` runs its tests. The tests behind the `probes` feature, most of them in `tests/breaches.rs`, are probes for checking the world.
