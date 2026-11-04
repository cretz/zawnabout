//! Client-only effects that live a moment: fire breath puffs and sparks from beacons as
//! they catch. Never part of the sim, so they can be as pretty as they like.

use glam::Vec3;
use zawnabout_sim::Sim;
use zawnabout_sim::shape::hash;

/// One glowing puff of fire.
#[derive(Clone, Copy, Debug)]
pub struct Puff {
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    pub life: f32,
    /// Picks size and color wobble.
    pub seed: u32,
    /// Sparks are small and bright, breath puffs grow big.
    pub spark: bool,
}

impl Puff {
    /// 0 at birth to 1 at death.
    pub fn t(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Fx {
    pub puffs: Vec<Puff>,
    /// Puffs owed to the breath, carried between ticks.
    owed: f32,
    count: u32,
    /// Beacons lit as of the last tick, to spot the moment one catches.
    was_lit: Vec<bool>,
}

/// Breath puffs per second at full blast.
const BREATH_RATE: f32 = 70.0;
/// Speed of breath fire leaving the snout (m/s).
const BREATH_SPEED: f32 = 85.0;

impl Fx {
    pub fn tick(&mut self, sim: &Sim, dt: f32) {
        for p in &mut self.puffs {
            p.age += dt;
            p.pos += p.vel * dt;
            // Fire slows in the air and rises as it cools.
            p.vel *= 1.0 - 1.5 * dt;
            p.vel.y += if p.spark { 6.0 } else { 9.0 } * dt;
        }
        self.puffs.retain(|p| p.age < p.life);

        for d in &sim.dragons {
            if d.breath < 0.05 {
                continue;
            }
            let (mouth, dir) = d.breath_ray();
            self.owed += BREATH_RATE * d.breath * dt;
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                self.count = self.count.wrapping_add(1);
                let s = self.count;
                let jitter = Vec3::new(
                    hash(s * 3) - 0.5,
                    hash(s * 3 + 1) - 0.5,
                    hash(s * 3 + 2) - 0.5,
                );
                let spread = (dir + jitter * 0.22).normalize();
                self.puffs.push(Puff {
                    pos: mouth + jitter * 0.3,
                    vel: d.vel + spread * BREATH_SPEED * (0.75 + hash(s * 7) * 0.5),
                    age: 0.0,
                    life: 0.8 + hash(s * 11) * 0.45,
                    seed: s,
                    spark: false,
                });
            }
        }

        // A burst of sparks the moment a beacon catches.
        self.was_lit.resize(sim.lit.len(), true);
        for (i, (now, was)) in sim.lit.iter().zip(&mut self.was_lit).enumerate() {
            if *now && !*was {
                let at = sim.world.beacons[i].flame;
                for k in 0..40u32 {
                    let s = i as u32 * 1000 + k;
                    let a = hash(s) * std::f32::consts::TAU;
                    let out = hash(s + 7) * 6.0;
                    self.puffs.push(Puff {
                        pos: at,
                        vel: Vec3::new(a.cos() * out, 6.0 + hash(s + 3) * 12.0, a.sin() * out),
                        age: 0.0,
                        life: 0.8 + hash(s + 5) * 0.9,
                        seed: s,
                        spark: true,
                    });
                }
            }
            *was = *now;
        }
    }
}
