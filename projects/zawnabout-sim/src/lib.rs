//! Deterministic game simulation. No rendering, devices or wall clock. Every entity is
//! stepped from its own input at a fixed rate, so the same inputs replay the same run.

pub mod dragon;
pub mod params;
pub mod shape;
pub mod swoop;
pub mod terrain;
pub mod world;

pub use dragon::{Dragon, DragonInput, EntityId, FlightParams, Mode};
pub use glam;
pub use params::{ParamDef, Params};
pub use world::{Collider, Perch, Prop, PropKind, World};

use glam::Vec3;

/// Ticks per second.
pub const TICK_HZ: u32 = 120;
/// Seconds per tick.
pub const DT: f32 = 1.0 / TICK_HZ as f32;

#[derive(Clone, Debug)]
pub struct Sim {
    pub flight: FlightParams,
    pub world: World,
    pub dragons: Vec<Dragon>,
    pub tick: u64,
    next_id: u32,
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        Self {
            flight: FlightParams::default(),
            world: World::generate(seed),
            dragons: Vec::new(),
            tick: 0,
            next_id: 1,
        }
    }

    /// Spawns a dragon standing on the ground at `(x, z)`.
    pub fn spawn_dragon(&mut self, x: f32, z: f32, heading: f32) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        let y = self.world.support(x, z, 1.0e4) + self.flight.stand_height;
        self.dragons
            .push(Dragon::new(id, Vec3::new(x, y, z), heading));
        id
    }

    pub fn dragon(&self, id: EntityId) -> Option<&Dragon> {
        self.dragons.iter().find(|d| d.id == id)
    }

    pub fn dragon_mut(&mut self, id: EntityId) -> Option<&mut Dragon> {
        self.dragons.iter_mut().find(|d| d.id == id)
    }

    /// Advances one tick. Dragons without an input entry get a neutral one.
    pub fn step(&mut self, inputs: &[(EntityId, DragonInput)]) {
        for dragon in &mut self.dragons {
            let input = inputs
                .iter()
                .find(|(id, _)| *id == dragon.id)
                .map(|(_, i)| *i)
                .unwrap_or_default();
            dragon.step(&self.flight, &self.world, input, DT);
        }
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Open sea well inside the edge, so tests fly over flat water with nothing to hit.
    const TEST_X: f32 = 4800.0;
    const TEST_Z: f32 = -4800.0;

    fn run(sim: &mut Sim, id: EntityId, secs: f32, input: DragonInput) {
        let mut first = true;
        for _ in 0..(secs * TICK_HZ as f32) as u32 {
            let mut i = input;
            if !first {
                i.flap_taps = 0;
            }
            first = false;
            sim.step(&[(id, i)]);
        }
    }

    #[test]
    fn hands_off_glide_settles() {
        let mut sim = Sim::new(1);
        let id = sim.spawn_dragon(TEST_X, TEST_Z, 0.0);
        let d = sim.dragon_mut(id).unwrap();
        d.mode = Mode::Air;
        d.pos.y = 300.0;
        d.vel = Vec3::new(0.0, 0.0, 25.0);
        run(&mut sim, id, 20.0, DragonInput::default());
        let d = sim.dragon(id).unwrap();
        assert_eq!(d.mode, Mode::Air);
        let speed = d.speed();
        assert!((24.0..44.0).contains(&speed), "glide speed {speed}");
        let sink = -d.vel.y;
        assert!((0.5..4.0).contains(&sink), "sink {sink}");
    }

    #[test]
    fn takeoff_with_held_flap_climbs() {
        let mut sim = Sim::new(1);
        let id = sim.spawn_dragon(TEST_X, TEST_Z, 0.0);
        run(&mut sim, id, 0.3, DragonInput::default());
        let start = sim.dragon(id).unwrap().pos.y;
        let input = DragonInput {
            flap: true,
            flap_taps: 1,
            pitch: 0.3,
            ..Default::default()
        };
        run(&mut sim, id, 6.0, input);
        let d = sim.dragon(id).unwrap();
        assert_eq!(d.mode, Mode::Air);
        assert!(d.pos.y - start > 15.0, "climbed {}", d.pos.y - start);
    }

    #[test]
    fn tucked_dive_is_fast_and_lands_with_a_tumble() {
        let mut sim = Sim::new(1);
        let id = sim.spawn_dragon(TEST_X, TEST_Z, 0.0);
        let d = sim.dragon_mut(id).unwrap();
        d.mode = Mode::Air;
        d.pos.y = 380.0;
        d.vel = Vec3::new(0.0, -5.0, 20.0);
        let dive = DragonInput {
            pitch: -1.0,
            tuck: true,
            ..Default::default()
        };
        let mut top = 0.0f32;
        for _ in 0..(12 * TICK_HZ) {
            sim.step(&[(id, dive)]);
            let d = sim.dragon(id).unwrap();
            top = top.max(d.speed());
            if d.mode == Mode::Ground {
                assert!(d.tumble > 0.0);
                break;
            }
        }
        assert!(top > 45.0, "top speed {top}");
    }

    #[test]
    fn brake_and_flap_hovers() {
        let mut sim = Sim::new(1);
        let id = sim.spawn_dragon(TEST_X, TEST_Z, 0.0);
        let d = sim.dragon_mut(id).unwrap();
        d.mode = Mode::Air;
        d.pos.y = 100.0;
        d.vel = Vec3::new(0.0, 0.0, 20.0);
        let brake = DragonInput {
            brake: true,
            ..Default::default()
        };
        run(&mut sim, id, 3.0, brake);
        let start = sim.dragon(id).unwrap().pos;
        run(
            &mut sim,
            id,
            5.0,
            DragonInput {
                flap: true,
                ..brake
            },
        );
        let d = sim.dragon(id).unwrap();
        assert!(d.speed() < 3.0, "hover speed {}", d.speed());
        assert!(
            (d.pos - start).length() < 12.0,
            "drifted {}",
            (d.pos - start).length()
        );
    }

    #[test]
    fn same_inputs_same_result() {
        let make = || {
            let mut sim = Sim::new(7);
            let id = sim.spawn_dragon(TEST_X + 10.0, TEST_Z + 20.0, 1.0);
            let input = DragonInput {
                flap: true,
                flap_taps: 1,
                turn: 0.4,
                pitch: 0.2,
                ..Default::default()
            };
            run(&mut sim, id, 5.0, input);
            sim.dragon(id).unwrap().pos
        };
        assert_eq!(make(), make());
    }
}
