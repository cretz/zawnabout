//! Client-only effects that live a moment: fire breath puffs, sparks from beacons as
//! they catch, and splashes where a dragon hits the water. Never part of the sim, so they can be as pretty as they like.

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
    /// A drop of splashed water, which falls back down.
    pub water: bool,
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
    /// Each dragon's swimming and speed as of the last tick, to spot the moment it hits
    /// the water and how hard.
    was_swimming: Vec<(bool, f32)>,
    /// Wake drops owed, carried between ticks.
    wake: f32,
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
            if p.water {
                // Spray falls back down.
                p.vel *= 1.0 - 0.6 * dt;
                p.vel.y -= 9.8 * dt;
                continue;
            }
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
                    water: false,
                });
            }
        }

        // A splash the moment a dragon lands in the water, bigger the harder it hits, and a
        // little spray off its chest while it paddles along.
        self.was_swimming.resize(sim.dragons.len(), (false, 0.0));
        for (i, d) in sim.dragons.iter().enumerate() {
            let (was, speed) = self.was_swimming[i];
            let water = d.pos - Vec3::Y * 0.4;
            if d.swimming && !was {
                let n = (speed * 4.0).clamp(30.0, 90.0) as u32;
                for k in 0..n {
                    self.count = self.count.wrapping_add(1);
                    let s = self.count * 13 + k;
                    let a = hash(s) * std::f32::consts::TAU;
                    let out = 1.5 + hash(s + 1) * (2.0 + speed * 0.12);
                    let up = 3.0 + hash(s + 2) * (3.0 + speed * 0.25);
                    self.puffs.push(Puff {
                        pos: water + Vec3::new(a.cos(), 0.0, a.sin()) * 0.8,
                        vel: Vec3::new(a.cos() * out, up, a.sin() * out) + d.vel * 0.15,
                        age: 0.0,
                        life: 0.7 + hash(s + 3) * 0.7,
                        seed: s,
                        spark: false,
                        water: true,
                    });
                }
            }
            let moving = Vec3::new(d.vel.x, 0.0, d.vel.z).length();
            if d.swimming && moving > 0.5 {
                self.wake += moving * 2.0 * dt;
                while self.wake >= 1.0 {
                    self.wake -= 1.0;
                    self.count = self.count.wrapping_add(1);
                    let s = self.count * 17;
                    let side = if hash(s) < 0.5 { -1.0 } else { 1.0 };
                    let fwd = Vec3::new(d.heading.sin(), 0.0, d.heading.cos());
                    let right = Vec3::new(fwd.z, 0.0, -fwd.x) * side;
                    self.puffs.push(Puff {
                        pos: water + fwd * 0.9 + right * 0.4,
                        vel: right * (1.0 + hash(s + 1)) + Vec3::Y * (1.2 + hash(s + 2) * 1.5),
                        age: 0.0,
                        life: 0.5 + hash(s + 3) * 0.3,
                        seed: s,
                        spark: false,
                        water: true,
                    });
                }
            }
            self.was_swimming[i] = (d.swimming, d.vel.length());
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
                        water: false,
                    });
                }
            }
            *was = *now;
        }
    }
}
