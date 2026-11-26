//! Cars broken down by the roadside, hazard lights blinking and the bonnet up, with the
//! driver standing beside them hoping for a lift, and sailboats becalmed off the coasts with
//! their sailor aboard. Talk to one and they ask to be flown somewhere near: the game treats
//! them as townsfolk who always ask for a ride.

use glam::{Vec2, Vec3};
use zawnabout_sim::World;

/// A car stopped on a road's shoulder and the driver standing by it.
#[derive(Clone, Copy, Debug)]
pub struct Stranded {
    pub car: Vec3,
    pub heading: f32,
    pub driver: Vec3,
    /// A sailboat adrift, not a car.
    pub boat: bool,
}

/// About one broken-down car to this much open road (m).
const EVERY: f32 = 1400.0;
/// Most of them anywhere.
const MOST: usize = 16;

fn hash(x: u32) -> u32 {
    let mut x = x.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 13)
}

/// Where cars have broken down: on the right-hand shoulder of open road, out of town and
/// off bridges and out of tunnels, on ground flat enough to stop on.
pub fn spots(world: &World) -> Vec<Stranded> {
    let t = &world.terrain;
    let mut out: Vec<Stranded> = Vec::new();
    for (i, r) in t.def.roads.iter().enumerate() {
        if r.rail || r.runway || r.subway {
            continue;
        }
        let prof = t.road_points(i);
        let mut along = 0.0;
        let mut next = EVERY * (0.3 + (hash(i as u32) % 1000) as f32 / 1000.0);
        for (k, (w, y)) in r.pts.windows(2).zip(prof.windows(2)).enumerate() {
            let (a, b) = (Vec2::new(w[0][0], w[0][1]), Vec2::new(w[1][0], w[1][1]));
            let seg = a.distance(b);
            along += seg;
            if along < next || seg < 1.0 {
                continue;
            }
            if y[0].bridge || y[0].tunnel || y[1].bridge || y[1].tunnel {
                continue;
            }
            let dir = (b - a) / seg;
            let right = Vec2::new(-dir.y, dir.x);
            let mid = a.lerp(b, 0.5);
            let car = mid + right * (r.w * 0.5 + 1.2);
            let driver = car + right * 2.4 + dir * 1.5;
            let road_y = (y[0].y + y[1].y) * 0.5;
            let flat = [car, driver]
                .iter()
                .all(|p| (t.height(p.x, p.y) - road_y).abs() < 1.2);
            let free = [car, driver].iter().all(|p| {
                let at = Vec3::new(p.x, road_y + 1.0, p.y);
                !world.in_building(at) && !world.afloat(p.x, p.y, road_y)
            });
            let lonely = out
                .iter()
                .all(|s| Vec2::new(s.car.x, s.car.z).distance(car) > 600.0);
            if !(flat && free && lonely) || hash(i as u32 * 131 + k as u32).is_multiple_of(3) {
                continue;
            }
            next = along + EVERY;
            let ground = |p: Vec2| world.support_still(p.x, p.y, road_y + 2.0);
            out.push(Stranded {
                car: Vec3::new(car.x, ground(car), car.y),
                heading: dir.x.atan2(dir.y),
                driver: Vec3::new(driver.x, ground(driver), driver.y),
                boat: false,
            });
            if out.len() >= MOST {
                return out;
            }
        }
    }
    out
}

/// Most sailboats adrift anywhere, and how far apart they keep (m).
const MOST_BOATS: usize = 6;
const BOATS_APART: f32 = 900.0;
/// How high a sailor stands over the sea, on the deck.
pub const DECK: f32 = 0.9;

/// Sailboats becalmed a little way off a coast: on open water a few meters deep or more,
/// with land in sight, so a dragon flying along the shore comes across them.
pub fn adrift(world: &World) -> Vec<Stranded> {
    let t = &world.terrain;
    let sea = t.def.sea_level;
    let half = world.half_size;
    let step = 170.0;
    let n = (half * 2.0 / step) as i32;
    let mut out: Vec<Stranded> = Vec::new();
    for k in 0..n * n {
        // Visit the grid in a scrambled order, so the boats spread over every coast.
        let k = (hash(k as u32 ^ 0x5bd1_e995) % (n * n) as u32) as i32;
        let p = Vec2::new(
            -half + (k % n) as f32 * step + step * 0.5,
            -half + (k / n) as f32 * step + step * 0.5,
        );
        let open = (0..12).all(|j| {
            let a = j as f32 / 12.0 * std::f32::consts::TAU;
            let q = p + Vec2::new(a.sin(), a.cos()) * 40.0;
            t.height(q.x, q.y) < sea - 3.0
        });
        let shore = (0..12).any(|j| {
            let a = j as f32 / 12.0 * std::f32::consts::TAU;
            let q = p + Vec2::new(a.sin(), a.cos()) * 220.0;
            t.height(q.x, q.y) > sea + 2.0
        });
        let lonely = out
            .iter()
            .all(|s| Vec2::new(s.car.x, s.car.z).distance(p) > BOATS_APART);
        let at = Vec3::new(p.x, sea, p.y);
        if !(open && shore && lonely) || world.solid_at(at + Vec3::Y) {
            continue;
        }
        let heading = (hash(k as u32) % 628) as f32 / 100.0;
        out.push(Stranded {
            car: at,
            heading,
            driver: at + Vec3::Y * DECK,
            boat: true,
        });
        if out.len() >= MOST_BOATS {
            break;
        }
    }
    out
}

/// Every broken-down car and becalmed boat.
pub fn all(world: &World) -> Vec<Stranded> {
    let mut out = spots(world);
    out.extend(adrift(world));
    out
}

#[cfg(test)]
mod tests {
    /// There are some broken-down cars about, each with its driver on dry ground beside
    /// the road, not on it.
    #[test]
    fn some_cars_broke_down() {
        let world = zawnabout_sim::World::generate(1);
        let spots = super::spots(&world);
        for s in &spots {
            println!(
                "car {:.0} {:.0} {:.0} driver {:.0} {:.0}",
                s.car.x, s.car.y, s.car.z, s.driver.x, s.driver.z
            );
            assert!(
                world
                    .terrain
                    .road_spot(s.driver.x, s.driver.z, 0.0)
                    .is_none()
            );
        }
        assert!(spots.len() >= 6, "{}", spots.len());
    }

    /// Some sailboats are adrift off the coasts, out on open water.
    #[test]
    fn some_boats_adrift() {
        let world = zawnabout_sim::World::generate(1);
        let boats = super::adrift(&world);
        for b in &boats {
            println!("boat {:.0} {:.0}", b.car.x, b.car.z);
            assert!(world.afloat(b.car.x, b.car.z, b.car.y));
        }
        assert!(boats.len() >= 3, "{}", boats.len());
    }
}
