//! Deterministic game simulation. No rendering, devices or wall clock. Every entity is
//! stepped from its own input at a fixed rate, so the same inputs replay the same run.

pub mod bridge;
pub mod cave;
pub mod dragon;
pub mod landmark;
pub mod params;
pub mod shape;
pub mod swoop;
pub mod terrain;
pub mod town;
pub mod traffic;
pub mod trips;
pub mod world;

pub use dragon::{Dragon, DragonInput, EntityId, FlightParams, Mode};
pub use glam;
pub use params::{ParamDef, Params};
pub use world::{Beacon, Collider, Perch, Prop, PropKind, World};

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
    /// Which beacons burn, in the order of `world.beacons`. Once lit, always lit.
    pub lit: Vec<bool>,
    pub tick: u64,
    next_id: u32,
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        let world = World::generate(seed);
        Self {
            flight: FlightParams::default(),
            lit: vec![false; world.beacons.len()],
            world,
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
        // Vehicles move first, so riders are carried to where they are now.
        let near: Vec<Vec3> = self.dragons.iter().map(|d| d.pos).collect();
        self.world
            .traffic
            .update_near((self.tick + 1) as f32 * DT, &near, self.tick, 8);
        for dragon in &mut self.dragons {
            let input = inputs
                .iter()
                .find(|(id, _)| *id == dragon.id)
                .map(|(_, i)| *i)
                .unwrap_or_default();
            dragon.step(&self.flight, &self.world, input, DT);
        }
        self.breathe();
        self.tick += 1;
    }

    /// Fire breath lights any beacon in a cone in front of the snout. It bends toward the
    /// nearest unlit beacon near where it aims, so things on the ground are easy to hit.
    fn breathe(&mut self) {
        let p = &self.flight;
        let range = p.breath_range;
        for d in &mut self.dragons {
            d.breath_aim = Vec3::ZERO;
            if d.breath < 0.05 {
                continue;
            }
            let (mouth, ahead) = d.breath_ahead(p.breath_down);
            let cos = p.breath_assist.cos();
            let target = self
                .world
                .beacons
                .iter()
                .zip(&self.lit)
                .filter(|(_, lit)| !**lit)
                .map(|(b, _)| b.flame - mouth)
                .filter(|to| to.length() < range && to.normalize_or_zero().dot(ahead) > cos)
                .min_by(|a, b| a.length().total_cmp(&b.length()));
            d.breath_aim = target.map_or(ahead, |to| to.normalize());
        }
        for d in self.dragons.iter().filter(|d| d.breath > 0.5) {
            let (mouth, dir) = d.breath_ray();
            for (b, lit) in self.world.beacons.iter().zip(&mut self.lit) {
                let to = b.flame - mouth;
                let dist = to.length();
                if dist < 5.0 || (dist < range && to.dot(dir) > dist * 0.8) {
                    *lit = true;
                }
            }
        }
    }

    /// The beacon with this id, and whether it burns.
    pub fn beacon(&self, id: &str) -> Option<(usize, bool)> {
        let i = self.world.beacons.iter().position(|b| b.id == id)?;
        Some((i, self.lit[i]))
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
    fn breath_lights_a_beacon_and_it_stays_lit() {
        let mut sim = Sim::new(1);
        let b = sim.world.beacons[0];
        let back = b.flame - Vec3::Z * 15.0;
        let id = sim.spawn_dragon(back.x, back.z, 0.0);
        let d = sim.dragon_mut(id).unwrap();
        d.mode = Mode::Air;
        d.pos = back;
        d.vel = Vec3::ZERO;
        let fire = DragonInput {
            breath: true,
            brake: true,
            flap: true,
            ..Default::default()
        };
        run(&mut sim, id, 0.5, fire);
        assert!(sim.lit[0], "beacon {} not lit", b.id);
        run(&mut sim, id, 1.0, DragonInput::default());
        assert!(sim.lit[0]);
        assert!(sim.lit.iter().filter(|l| **l).count() == 1);
    }

    #[test]
    fn braking_tightens_turns() {
        let turned = |brake: bool| {
            let mut sim = Sim::new(1);
            let id = sim.spawn_dragon(TEST_X, TEST_Z, 0.0);
            let d = sim.dragon_mut(id).unwrap();
            d.mode = Mode::Air;
            d.pos.y = 300.0;
            d.vel = Vec3::new(0.0, 0.0, 40.0);
            let input = DragonInput {
                turn: 1.0,
                brake,
                ..Default::default()
            };
            run(&mut sim, id, 0.8, input);
            -sim.dragon(id).unwrap().heading
        };
        let (open, tight) = (turned(false), turned(true));
        assert!(open > 0.4, "turned {open}");
        assert!(tight > open * 1.4, "braking turned {tight}, open {open}");
    }

    #[test]
    fn running_is_faster_than_walking() {
        let mut sim = Sim::new(1);
        let spawn = sim.world.terrain.spawn();
        let id = sim.spawn_dragon(spawn.x, spawn.y, 0.0);
        let input = DragonInput {
            pitch: 1.0,
            ..Default::default()
        };
        run(&mut sim, id, 0.4, input);
        let walk = sim.dragon(id).unwrap().speed();
        run(&mut sim, id, 2.0, input);
        let d = sim.dragon(id).unwrap();
        assert!(d.mode == Mode::Ground);
        assert!(
            d.speed() > sim.flight.walk_speed * 1.8,
            "ran {} walked {walk}",
            d.speed()
        );
    }

    #[test]
    fn cars_bowl_you_over() {
        use crate::town::Kind;
        // Stand in the road where a car will be in two seconds. Some spots are at a road's
        // end where cars turn round, so try a few cars.
        let cars: Vec<usize> = Sim::new(1)
            .world
            .traffic
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| v.kind == Kind::Car)
            .map(|(i, _)| i)
            .collect();
        let bowled = cars.iter().step_by(7).take(9).any(|&i| {
            let mut sim = Sim::new(1);
            let t = &sim.world.traffic;
            let ahead = t.pose(&t.vehicles[i], 2.0).pos;
            let id = sim.spawn_dragon(ahead.x, ahead.z, 0.0);
            let mut hit = false;
            for _ in 0..(4.0 / DT) as usize {
                sim.step(&[(id, DragonInput::default())]);
                let d = sim.dragon(id).unwrap();
                hit |= d.bonk < 0.1 && d.mode == Mode::Air;
            }
            hit
        });
        assert!(bowled, "no car ever bumped the dragon");
    }

    #[test]
    fn clings_to_walls_and_climbs_onto_the_roof() {
        use crate::town::Kind;
        let mut sim = Sim::new(1);
        // A tall plain building, and a spot halfway up its front wall.
        let (wall, roof) = sim
            .world
            .buildings
            .iter()
            .filter(|b| {
                matches!(b.kind, Kind::Flat | Kind::Warehouse | Kind::Skyscraper) && b.size.y > 12.0
            })
            .find_map(|b| {
                let spot = b.at(0.0, b.size.y * 0.5, b.size.z * 0.5 - 0.1);
                sim.world
                    .wall_normal(spot)
                    .map(|_| (spot, b.pos.y + b.size.y))
            })
            .expect("a wall to cling to");
        let n = sim.world.wall_normal(wall).unwrap();
        let start = wall + n * 60.0;
        let id = sim.spawn_dragon(start.x, start.z, 0.0);
        let go = DragonInput {
            land_at: Some(wall),
            ..Default::default()
        };
        sim.step(&[(id, go)]);
        run(&mut sim, id, 12.0, DragonInput::default());
        let d = sim.dragon(id).unwrap();
        assert!(d.cling.is_some(), "never reached the wall, at {}", d.pos);
        assert!(
            (d.pos.y - wall.y).abs() < 3.0,
            "clung at {} not {}",
            d.pos,
            wall
        );
        // Climb up and over the top.
        let up = DragonInput {
            pitch: 1.0,
            ..Default::default()
        };
        for _ in 0..(20.0 / DT) as usize {
            sim.step(&[(id, up)]);
            if sim.dragon(id).unwrap().cling.is_none() {
                break;
            }
        }
        run(&mut sim, id, 0.5, DragonInput::default());
        let d = sim.dragon(id).unwrap();
        // On top, or on a ledge partway up where the wall steps back.
        assert!(d.cling.is_none(), "still on the wall at {}", d.pos);
        let feet = d.pos.y - sim.flight.stand_height;
        let under = sim.world.support(d.pos.x, d.pos.z, feet + 0.5);
        assert!(d.mode == Mode::Ground && (feet - under).abs() < 0.5);
        assert!(
            feet > wall.y + 2.0 && feet < roof + 2.0,
            "stood at {} below the wall spot {wall} or over the roof at {roof}",
            d.pos
        );
        // A flap leaps off into the air.
        let flap = DragonInput {
            flap_taps: 1,
            flap: true,
            ..Default::default()
        };
        run(&mut sim, id, 0.5, flap);
        assert!(sim.dragon(id).unwrap().mode == Mode::Air);
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
