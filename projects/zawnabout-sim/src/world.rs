//! The static world: terrain from the world files plus placed props. Built the same way
//! on every peer, so every peer has the identical world.

use glam::{Quat, Vec2, Vec3};

use crate::cave::{self, BEACON_RIM, BEACON_TOP, Part};
use crate::shape::{Block, Frustum, Shape, place, spire_tiers};
use crate::terrain::{CoverKind, Density, Mark, Style, Terrain, Trees};
use crate::town::{self, Building, Kind};
use crate::traffic::Traffic;

/// Small deterministic PRNG (SplitMix64). Not for anything but placement.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    RoundTree,
    PineTree,
    Bush,
    Rock,
    Spire,
    Flower,
    Cloud,
    /// A glowing crystal cluster in a cave. Variant is the color.
    Crystal,
    /// A patch of glowing mushrooms in a cave. Variant is the color.
    Shroom,
    Stalagmite,
    /// Hangs down from its position.
    Stalactite,
    /// A fire bowl to light with fire breath.
    Beacon,
    /// Old stonework. Variant is the kind, see [`cave::RUIN_KINDS`].
    Ruin,
    /// A leaning palm with a crown of fronds, for warm shores.
    Palm,
    /// A mushroom as big as a tree on a cave floor, cap broad enough to land on. Variant
    /// is the color.
    GiantShroom,
    /// A curtain of glowing threads hanging from a cave roof.
    Glowworms,
    /// Leafy vines hanging over the lip of a cave mouth.
    Vines,
    /// Shelf fungi in tiers on a cave wall, glowing underneath. Variant is the color.
    Brackets,
}

impl PropKind {
    pub const ALL: [PropKind; 18] = [
        PropKind::RoundTree,
        PropKind::PineTree,
        PropKind::Bush,
        PropKind::Rock,
        PropKind::Spire,
        PropKind::Flower,
        PropKind::Cloud,
        PropKind::Crystal,
        PropKind::Shroom,
        PropKind::Stalagmite,
        PropKind::Stalactite,
        PropKind::Beacon,
        PropKind::Ruin,
        PropKind::Palm,
        PropKind::GiantShroom,
        PropKind::Glowworms,
        PropKind::Vines,
        PropKind::Brackets,
    ];
}

/// A fire bowl that fire breath lights for good. Its place in [`World::beacons`] is its
/// place in the sim's lit list.
#[derive(Clone, Copy, Debug)]
pub struct Beacon {
    /// Stable id from the world files, such as `home/summit-beacon`.
    pub id: &'static str,
    /// Where the flame sits.
    pub flame: Vec3,
    pub prop: u32,
}

/// A placed static thing. `id` is stable for a given seed, for saves and networking.
#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub id: u32,
    pub kind: PropKind,
    pub pos: Vec3,
    pub yaw: f32,
    pub scale: f32,
    /// Picks a mesh or color variant.
    pub variant: u8,
}

/// A solid part of a prop, or of a building when `prop` has [`Collider::BUILDING`] set.
#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub prop: u32,
    pub shape: Shape,
}

impl Collider {
    pub const BUILDING: u32 = 1 << 31;
    /// A bridge deck, with the road index below.
    pub const BRIDGE: u32 = 1 << 30;
}

/// A good spot to land, marked for tap-to-land.
#[derive(Clone, Copy, Debug)]
pub struct Perch {
    /// Stable id from the world files, such as `home/summit-perch`.
    pub id: &'static str,
    pub pos: Vec3,
    pub radius: f32,
}

/// Size of a collision grid cell (m).
const CELL: f32 = 40.0;

#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub terrain: Terrain,
    /// The playable square spans `-half_size..half_size` on X and Z.
    pub half_size: f32,
    /// Width of the soft band inside the edge where a breeze turns you back.
    pub edge: f32,
    /// Height where the air starts pushing back down.
    pub sky: f32,
    pub props: Vec<Prop>,
    /// Every building, landmark and boat in every town.
    pub buildings: Vec<Building>,
    pub colliders: Vec<Collider>,
    pub perches: Vec<Perch>,
    pub beacons: Vec<Beacon>,
    /// Ferries, boats, planes and cars, moved by the sim every tick.
    pub traffic: Traffic,
    /// Suspension bridges' cables, for looks. Their towers are buildings.
    pub bridges: Vec<crate::bridge::Suspension>,
    grid: Vec<Vec<u32>>,
    grid_n: usize,
}

/// Where the tree on top of a spire stands, relative to the spire's cap center.
pub fn spire_tree_offset(cap_radius: f32) -> Vec3 {
    Vec3::new(cap_radius * 0.55, 0.3, 0.0)
}

impl World {
    pub fn generate(seed: u64) -> Self {
        let terrain = Terrain::default();
        let def = terrain.def;
        let mut world = World {
            seed,
            half_size: terrain.half_size(),
            edge: def.edge,
            sky: def.sky,
            terrain,
            props: Vec::new(),
            buildings: Vec::new(),
            colliders: Vec::new(),
            perches: Vec::new(),
            beacons: Vec::new(),
            traffic: Traffic::default(),
            bridges: Vec::new(),
            grid: Vec::new(),
            grid_n: 0,
        };
        world.place_props();
        world.buildings = town::place(&world.terrain, seed);
        world.landmarks();
        world.stations();
        world.bridges();
        world.clear_landmarks();
        world.yard_trees();
        world.wild_props();
        world.place_caves();
        world.lifts();
        world.build_colliders();
        world.traffic = Traffic::new(&world.terrain);
        world.traffic.update(0.0);
        for m in def.marks.iter().filter(|m| m.kind == "perch") {
            let (x, z) = (m.pos[0], m.pos[2]);
            // Perches on or near the ground sit exactly on its rough surface.
            let ground = world.terrain.height(x, z);
            let y = if m.ground || (ground - m.pos[1]).abs() < 10.0 {
                ground
            } else {
                m.pos[1]
            };
            world.perches.push(Perch {
                id: m.id,
                pos: Vec3::new(x, y, z),
                radius: 4.0,
            });
        }
        // The tops of lighthouses and towers make fine perches.
        for b in &world.buildings {
            let top = match b.kind {
                Kind::Lighthouse => b.size.y + town::LAMP_ROOM,
                Kind::Mast => b.size.y * 0.72 + b.size.x * 1.4,
                Kind::ControlTower => b.size.y + 5.0,
                _ => continue,
            };
            world.perches.push(Perch {
                id: b.site.unwrap_or("town/tower-top"),
                pos: b.pos + Vec3::Y * top,
                radius: 3.0,
            });
        }
        world
    }

    /// Trees in back gardens, so streets feel lived in.
    fn yard_trees(&mut self) {
        let mut rng = Rng::new(self.seed ^ 0x7A2D);
        let mut placed = Vec::new();
        for b in &self.buildings {
            let leafy = matches!(
                b.style,
                Style::Canal | Style::Seafaring | Style::Stone | Style::Hill | Style::Plain
            );
            if !leafy || !matches!(b.kind, Kind::House | Kind::Row) || rng.f32() > 0.55 {
                continue;
            }
            // Behind the house: away from the street it faces.
            let (back, depth) = if b.kind == Kind::Row {
                (-b.axis, b.size.x)
            } else {
                (-b.front(), b.size.z)
            };
            let at = Vec2::new(b.pos.x, b.pos.z) + back * (depth * 0.5 + rng.range(2.5, 4.5));
            let side = Vec2::new(-back.y, back.x) * rng.range(-2.0, 2.0);
            let at = at + side;
            let s = rng.range(1.1, 1.7);
            let v = (rng.next_u64() % 3) as u8;
            if let Some(h) = self.dry_spot(at.x, at.y, 0.75) {
                placed.push((Vec3::new(at.x, h - 0.3, at.y), s, v));
            }
        }
        for (pos, s, v) in placed {
            self.push(PropKind::RoundTree, pos, s, v);
        }
    }

    /// Landmarks from the world files, as buildings so they are solid and drawn with the
    /// towns. A drive-in gets cars parked in front of its screen.
    fn landmarks(&mut self) {
        let def = self.terrain.def;
        for m in def.marks {
            let Some(rest) = m.kind.strip_prefix("landmark:") else {
                continue;
            };
            let mut parts = rest.splitn(3, ':');
            let name = parts.next().unwrap_or_default();
            let rot: f32 = parts.next().and_then(|r| r.parse().ok()).unwrap_or(0.0);
            let text = parts.next().unwrap_or_default();
            let Some(&(_, kind)) = crate::landmark::KINDS.iter().find(|(n, _)| *n == name) else {
                continue;
            };
            let (x, z) = (m.pos[0], m.pos[2]);
            let y = if m.ground {
                self.terrain.height(x, z)
            } else {
                m.pos[1]
            };
            let r = rot.to_radians();
            let axis = Vec2::new(r.cos(), -r.sin());
            let seed =
                m.id.bytes()
                    .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
            let pieces = crate::landmark::pieces(kind, seed, text);
            let b = Building {
                id: self.buildings.len() as u32,
                kind,
                style: Style::Plain,
                tech: crate::terrain::Tech::Modern,
                pos: Vec3::new(x, y, z),
                axis,
                size: crate::landmark::extent(&pieces),
                roof: 0.0,
                stilts: 0.0,
                seed,
                site: Some(m.id),
                text,
            };
            if kind == Kind::Sign {
                // One block per letter, each standing on its own patch of hillside.
                let cell = crate::landmark::SIGN_CELL;
                let width = text.len() as f32 * 4.0 * cell - cell;
                for (i, (at, c)) in text.char_indices().enumerate() {
                    let x = (i as f32 * 4.0 + 1.5) * cell - width * 0.5;
                    let p = b.at(x, 0.0, 0.0);
                    let ground = [-1.0f32, 0.0, 1.0]
                        .map(|k| {
                            let q = b.at(x + k * cell, 0.0, 0.0);
                            self.terrain.height(q.x, q.z)
                        })
                        .into_iter()
                        .fold(f32::MAX, f32::min);
                    self.buildings.push(Building {
                        id: 0,
                        pos: Vec3::new(p.x, ground - 0.5, p.z),
                        size: Vec3::new(cell * 3.0, cell * 5.0 + 1.0, 1.0),
                        text: &text[at..at + c.len_utf8()],
                        ..b
                    });
                }
                continue;
            }
            if kind == Kind::DriveIn {
                for row in 0..2 {
                    for k in 0..5 {
                        let local = Vec2::new(k as f32 * 6.5 - 13.0, 16.0 + row as f32 * 9.0);
                        let at = b.at(local.x, 0.0, local.y);
                        self.buildings.push(Building {
                            id: self.buildings.len() as u32 + 1,
                            kind: Kind::Car,
                            style: Style::Plain,
                            tech: crate::terrain::Tech::Modern,
                            pos: Vec3::new(at.x, self.terrain.height(at.x, at.z), at.z),
                            axis: -b.front(),
                            size: Vec3::new(4.2, 1.5, 1.9),
                            roof: 0.0,
                            stilts: 0.0,
                            seed: seed.wrapping_add(row * 5 + k),
                            site: None,
                            text: "",
                        });
                    }
                }
            }
            self.buildings.push(b);
        }
        for (i, b) in self.buildings.iter_mut().enumerate() {
            b.id = i as u32;
        }
    }

    /// Everything that grows or lies about outside towns, so no land is bare: bushes in
    /// clumps, lone trees, pines up high, rocks on slopes and boulders on beaches, and
    /// undergrowth in the woods. Keeps clear of buildings and roads.
    /// A station beside the track at every railway `station` mark, its platform on the
    /// mark's side.
    fn stations(&mut self) {
        let def = self.terrain.def;
        for m in def.marks.iter().filter(|m| m.kind == "station") {
            let at = Vec2::new(m.pos[0], m.pos[2]);
            let Some((i, k)) = nearest_rail(def, at) else {
                continue;
            };
            let (a, b) = (def.roads[i].pts[k], def.roads[i].pts[k + 1]);
            let track = Vec2::new(a[0], a[1]);
            let mut axis = (Vec2::new(b[0], b[1]) - track).normalize_or(Vec2::X);
            if (at - track).dot(Vec2::new(-axis.y, axis.x)) < 0.0 {
                axis = -axis;
            }
            let y = self.terrain.road_points(i)[k].y;
            let (kind, name) = if def.roads[i].subway {
                // Named on its signs after the last word of its id.
                (
                    Kind::Metro,
                    m.id.rsplit(['-', '/']).next().unwrap_or_default(),
                )
            } else {
                (Kind::Station, "")
            };
            let pieces = crate::landmark::pieces(kind, 0, name);
            self.buildings.push(Building {
                id: self.buildings.len() as u32,
                kind,
                style: Style::Plain,
                tech: crate::terrain::Tech::Modern,
                pos: Vec3::new(track.x, y, track.y),
                axis,
                size: crate::landmark::extent(&pieces),
                roof: 0.0,
                stilts: 0.0,
                seed: 0,
                site: Some(m.id),
                text: name,
            });
        }
    }

    /// Towers for every suspension bridge.
    fn bridges(&mut self) {
        self.bridges = crate::bridge::suspensions(&self.terrain);
        for b in &self.bridges {
            for t in b.towers {
                self.buildings.push(Building {
                    id: self.buildings.len() as u32,
                    kind: Kind::BridgeTower,
                    style: Style::Plain,
                    tech: crate::terrain::Tech::Modern,
                    pos: Vec3::new(t.at.x, t.base, t.at.y),
                    axis: t.across,
                    size: Vec3::new(
                        t.spread * 2.0 + crate::town::TOWER_LEG.x,
                        t.top - t.base,
                        crate::town::TOWER_LEG.y,
                    ),
                    roof: 0.0,
                    stilts: 0.0,
                    seed: 0,
                    site: None,
                    text: "",
                });
            }
        }
    }

    /// Pylons along every lift and a station at each end.
    fn lifts(&mut self) {
        let def = self.terrain.def;
        for r in def.routes {
            let Some(spec) = crate::traffic::lift_spec(r.kind) else {
                continue;
            };
            let line = crate::traffic::lift_line(&self.terrain, r.pts, &spec);
            let (a, b) = (line[0], line[line.len() - 1]);
            let dir = Vec2::new(b.x - a.x, b.z - a.z).normalize_or(Vec2::X);
            let seed =
                r.id.bytes()
                    .fold(5u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32));
            let mut add = |kind: Kind, at: Vec3, axis: Vec2, size: Vec3, roof: f32| {
                self.buildings.push(Building {
                    id: self.buildings.len() as u32,
                    kind,
                    style: Style::Plain,
                    tech: crate::terrain::Tech::Modern,
                    pos: at,
                    axis,
                    size,
                    roof,
                    stilts: 0.0,
                    seed,
                    site: None,
                    text: "",
                });
            };
            let wide = spec.gauge * 2.0;
            for (end, axis) in [(a, dir), (b, -dir)] {
                // Sunk to the lowest corner, so it never floats on a slope.
                let reach = spec.gauge + 3.0;
                let ground = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
                    .iter()
                    .map(|(x, z)| self.terrain.height(end.x + x * reach, end.z + z * reach))
                    .fold(self.terrain.height(end.x, end.z), f32::min);
                let at = Vec3::new(end.x, ground, end.z);
                add(
                    Kind::LiftStation,
                    at,
                    axis,
                    Vec3::new(
                        spec.gauge * 2.0 + 6.0,
                        crate::traffic::STATION_CABLE + 1.4,
                        wide + 5.0,
                    ),
                    1.6,
                );
            }
            // A pylon about every 60m, clear of the stations.
            let mut along = 0.0;
            let mut next = 40.0;
            for w in line.windows(2) {
                along += Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length();
                let left = (Vec2::new(b.x, b.z) - Vec2::new(w[1].x, w[1].z)).length();
                if along >= next && left > 30.0 {
                    let ground = self.terrain.height(w[1].x, w[1].z);
                    add(
                        Kind::LiftTower,
                        Vec3::new(w[1].x, ground, w[1].z),
                        dir,
                        Vec3::new(1.2, w[1].y + 0.4 - ground, wide),
                        0.0,
                    );
                    next = along + 60.0;
                }
            }
        }
    }

    /// Clears trees, bushes and rocks off the ground landmarks stand on.
    fn clear_landmarks(&mut self) {
        let spots: Vec<(Vec2, f32)> = self
            .buildings
            .iter()
            .filter(|b| crate::landmark::KINDS.iter().any(|(_, k)| *k == b.kind))
            .map(|b| {
                (
                    Vec2::new(b.pos.x, b.pos.z),
                    b.size.x.max(b.size.z) * 0.6 + 4.0,
                )
            })
            .collect();
        self.props.retain(|p| {
            let wild = matches!(
                p.kind,
                PropKind::RoundTree
                    | PropKind::PineTree
                    | PropKind::Palm
                    | PropKind::Bush
                    | PropKind::Rock
                    | PropKind::Flower
            );
            let at = Vec2::new(p.pos.x, p.pos.z);
            !wild || spots.iter().all(|(c, r)| c.distance(at) > *r)
        });
        for (i, p) in self.props.iter_mut().enumerate() {
            p.id = i as u32;
        }
        // Town buildings give way to the big places dropped into towns.
        let big: Vec<(Vec2, f32)> = self
            .buildings
            .iter()
            .filter(|b| crate::landmark::KINDS.iter().any(|(_, k)| *k == b.kind))
            .filter(|b| b.size.x.max(b.size.z) > 20.0)
            .map(|b| {
                (
                    Vec2::new(b.pos.x, b.pos.z),
                    b.size.x.max(b.size.z) * 0.55 + 4.0,
                )
            })
            .collect();
        self.buildings.retain(|b| {
            // Landmarks and the sites the world files name (towers, lighthouses) stay.
            let mark = crate::landmark::KINDS.iter().any(|(_, k)| *k == b.kind) || b.site.is_some();
            let at = Vec2::new(b.pos.x, b.pos.z);
            mark || big
                .iter()
                .all(|(c, r)| c.distance(at) > *r + b.size.x.max(b.size.z) * 0.5)
        });
        for (i, b) in self.buildings.iter_mut().enumerate() {
            b.id = i as u32;
        }
    }

    fn wild_props(&mut self) {
        const CELL: f32 = 20.0;
        let mut rng = Rng::new(self.seed ^ 0x3117_D0C5);
        // Ground near buildings is gardens and streets, not wild.
        let mut built = std::collections::HashSet::new();
        for b in &self.buildings {
            let r = b.size.x.max(b.size.z) * 0.6 + 8.0;
            let (x0, z0) = (
                ((b.pos.x - r) / CELL).floor() as i32,
                ((b.pos.z - r) / CELL).floor() as i32,
            );
            let (x1, z1) = (
                ((b.pos.x + r) / CELL).floor() as i32,
                ((b.pos.z + r) / CELL).floor() as i32,
            );
            for z in z0..=z1 {
                for x in x0..=x1 {
                    built.insert((x, z));
                }
            }
        }
        let terrain = &self.terrain;
        let n = (self.half_size / CELL).ceil() as i32;
        let mut placed = Vec::new();
        for cz in -n..n {
            for cx in -n..n {
                if built.contains(&(cx, cz)) {
                    continue;
                }
                let x = (cx as f32 + rng.range(0.0, 1.0)) * CELL;
                let z = (cz as f32 + rng.range(0.0, 1.0)) * CELL;
                let roll = rng.f32();
                let (s, v) = (rng.f32(), (rng.next_u64() % 3) as u8);
                let h = terrain.height(x, z);
                if h < 1.2 || terrain.water(x, z, h).is_some() {
                    continue;
                }
                if terrain.road_spot(x, z, 5.0).is_some() {
                    continue;
                }
                let up = terrain.normal(x, z).y;
                let cover = terrain.cover(x, z).map(|c| c.kind);
                if matches!(cover, Some(CoverKind::Town | CoverKind::Snow)) {
                    continue;
                }
                // Clumps: bushes and trees gather where this is high.
                let clump = crate::terrain::noise(71, x / 90.0, z / 90.0);
                let at = Vec3::new(x, h, z);
                let pick = match cover {
                    Some(CoverKind::Forest) if up > 0.7 => {
                        (roll < 0.22).then(|| (PropKind::Bush, at - Vec3::Y * 0.2, 1.0 + s * 0.8))
                    }
                    Some(CoverKind::Meadow) if up > 0.8 => {
                        if roll < 0.05 {
                            Some((PropKind::Bush, at - Vec3::Y * 0.2, 0.9 + s * 0.6))
                        } else if roll < 0.08 {
                            Some((PropKind::Rock, at - Vec3::Y * 0.3, 0.6 + s * 0.8))
                        } else {
                            None
                        }
                    }
                    _ if h < 5.0 => {
                        // Beaches: the odd boulder, smoothed by the sea.
                        (roll < 0.025).then(|| (PropKind::Rock, at - Vec3::Y * 0.6, 1.5 + s * 2.5))
                    }
                    _ if up < 0.55 => None,
                    _ if up < 0.85 => {
                        // Slopes: scree and the odd bush in a crack.
                        if roll < 0.16 {
                            Some((PropKind::Rock, at - Vec3::Y * 0.4, 0.8 + s * 2.4))
                        } else if roll < 0.2 {
                            Some((PropKind::Bush, at - Vec3::Y * 0.2, 0.8 + s * 0.5))
                        } else {
                            None
                        }
                    }
                    _ if h > 320.0 => {
                        (roll < 0.07).then(|| (PropKind::Rock, at - Vec3::Y * 0.4, 0.8 + s * 3.0))
                    }
                    _ => {
                        let bushy = 0.04 + clump.max(0.0) * 0.35;
                        if roll < bushy {
                            Some((PropKind::Bush, at - Vec3::Y * 0.2, 0.9 + s * 0.9))
                        } else if roll < bushy + 0.012 + clump.max(0.0) * 0.05 {
                            let kind = if h > 170.0 {
                                PropKind::PineTree
                            } else {
                                PropKind::RoundTree
                            };
                            Some((kind, at - Vec3::Y * 0.3, 1.6 + s * 1.8))
                        } else if roll < bushy + 0.07 {
                            Some((PropKind::Rock, at - Vec3::Y * 0.3, 0.5 + s * 1.2))
                        } else {
                            None
                        }
                    }
                };
                if let Some((kind, pos, scale)) = pick
                    && terrain.rim(x, z, 4.0 + scale * 1.5) < 1.5
                {
                    placed.push((kind, pos, scale, v));
                }
            }
        }
        for (kind, pos, s, v) in placed {
            self.push(kind, pos, s, v);
        }
    }

    /// Cave decor, then beacons and ruins from the world files.
    fn place_caves(&mut self) {
        for (kind, pos, s, v) in cave::decorate(&self.terrain, self.seed) {
            self.push(kind, pos, s, v);
        }
        let turned = cave::mouths(&self.terrain, self.seed)
            .into_iter()
            .chain(cave::brackets(&self.terrain, self.seed));
        for (kind, pos, s, v, yaw) in turned {
            self.push(kind, pos, s, v);
            if let Some(p) = self.props.last_mut() {
                p.yaw = yaw;
            }
        }
        let def = self.terrain.def;
        // Marks with a height sit on whatever is under them: a cave floor, a ledge.
        let floor = |m: &Mark| {
            let (x, z) = (m.pos[0], m.pos[2]);
            let top = if m.ground {
                self.terrain.height(x, z) + 1.0
            } else {
                m.pos[1] + 12.0
            };
            Vec3::new(x, self.terrain.support(x, z, top), z)
        };
        let mut beacons = Vec::new();
        let mut ruins = Vec::new();
        for m in def.marks {
            if m.kind == "beacon" {
                beacons.push((m.id, floor(m)));
            } else if let Some(kind) = m.kind.strip_prefix("ruin:") {
                let v = cave::RUIN_KINDS
                    .iter()
                    .position(|k| *k == kind)
                    .unwrap_or(0);
                ruins.push((floor(m), v as u8));
            }
        }
        for (id, at) in beacons {
            let scale = 1.3;
            let prop = self.props.len() as u32;
            self.push(PropKind::Beacon, at, scale, 0);
            self.beacons.push(Beacon {
                id,
                flame: at + Vec3::Y * (BEACON_TOP * scale + 0.3),
                prop,
            });
        }
        for (at, v) in ruins {
            // Old stonework sinks a little into the floor.
            self.push(PropKind::Ruin, at - Vec3::Y * 0.4, 1.0, v);
        }
    }

    fn build_colliders(&mut self) {
        for p in &self.props {
            let s = p.scale;
            let mut add = |shape| self.colliders.push(Collider { prop: p.id, shape });
            let trunk = |h: f32| {
                Shape::Frustum(Frustum {
                    base: p.pos,
                    height: h * s,
                    r0: 0.4 * s,
                    r1: 0.28 * s,
                    spin: 0.0,
                })
            };
            match p.kind {
                PropKind::Spire => {
                    let tiers = spire_tiers(p.variant as u32 % 3);
                    let mut cap = tiers[0];
                    for t in &tiers {
                        cap = place(t, p.pos, p.yaw, s);
                        add(Shape::Frustum(cap));
                    }
                    let top = Vec3::new(cap.base.x, cap.top(), cap.base.z);
                    let tree =
                        top + Quat::from_rotation_y(p.yaw) * (spire_tree_offset(cap.r1 / s) * s);
                    add(Shape::Sphere {
                        center: tree + Vec3::Y * 4.6 * s,
                        radius: 2.3 * s,
                    });
                }
                PropKind::RoundTree => {
                    add(trunk(3.2));
                    add(Shape::Sphere {
                        center: p.pos + Vec3::Y * 4.6 * s,
                        radius: 2.3 * s,
                    });
                }
                PropKind::PineTree => {
                    add(trunk(2.0));
                    let tiers = 3 + (p.variant as u32 % 3);
                    add(Shape::Frustum(Frustum {
                        base: p.pos + Vec3::Y * 1.6 * s,
                        height: ((tiers - 1) as f32 * 1.7 + 2.6) * s,
                        r0: 2.2 * s,
                        r1: 0.0,
                        spin: 0.0,
                    }));
                }
                PropKind::Palm => {
                    add(Shape::Frustum(Frustum {
                        base: p.pos,
                        height: 6.5 * s,
                        r0: 0.35 * s,
                        r1: 0.25 * s,
                        spin: 0.0,
                    }));
                }
                PropKind::Rock => {
                    add(Shape::Sphere {
                        center: p.pos + Vec3::Y * 0.5 * s,
                        radius: 1.1 * s,
                    });
                }
                PropKind::Crystal => {
                    add(Shape::Frustum(Frustum {
                        base: p.pos,
                        height: 2.6 * s,
                        r0: 1.0 * s,
                        r1: 0.6 * s,
                        spin: 0.0,
                    }));
                }
                PropKind::Stalagmite => {
                    add(Shape::Frustum(Frustum {
                        base: p.pos,
                        height: 5.0 * s,
                        r0: 1.6 * s,
                        r1: 0.25 * s,
                        spin: 0.0,
                    }));
                }
                PropKind::Stalactite => {
                    add(Shape::Frustum(Frustum {
                        base: p.pos - Vec3::Y * 5.0 * s,
                        height: 5.6 * s,
                        r0: 0.15 * s,
                        r1: 1.3 * s,
                        spin: 0.0,
                    }));
                }
                PropKind::GiantShroom => {
                    let top =
                        p.pos + Quat::from_rotation_y(p.yaw) * cave::shroom_top(p.variant) * s;
                    add(Shape::Frustum(Frustum {
                        base: p.pos,
                        height: top.y - p.pos.y,
                        r0: 0.8 * s,
                        r1: 0.45 * s,
                        spin: 0.0,
                    }));
                    add(Shape::Frustum(Frustum {
                        base: top,
                        height: 1.6 * s,
                        r0: 3.4 * s,
                        r1: 1.7 * s,
                        spin: 0.0,
                    }));
                }
                PropKind::Beacon => {
                    add(Shape::Frustum(Frustum {
                        base: p.pos,
                        height: BEACON_TOP * s,
                        r0: 1.1 * s,
                        r1: BEACON_RIM * s,
                        spin: 0.0,
                    }));
                }
                PropKind::Ruin => {
                    let turn = Quat::from_rotation_y(p.yaw);
                    let x = turn * Vec3::X;
                    for part in cave::ruin_parts(p.variant) {
                        add(match part {
                            Part::Box { base, half } => Shape::Block(Block {
                                base: p.pos + turn * base * s,
                                half: half * s,
                                axis: Vec2::new(x.x, x.z),
                                roof: 0.0,
                            }),
                            Part::Column { base, h, r } => Shape::Frustum(Frustum {
                                base: p.pos + turn * base * s,
                                height: h * s,
                                r0: r * s,
                                r1: r * 0.85 * s,
                                spin: 0.0,
                            }),
                        });
                    }
                }
                PropKind::Bush
                | PropKind::Flower
                | PropKind::Cloud
                | PropKind::Shroom
                | PropKind::Glowworms
                | PropKind::Vines
                | PropKind::Brackets => {}
            }
        }
        // Bridge decks hold you up and stop you, one box per stretch of deck.
        for (ri, road) in self.terrain.def.roads.iter().enumerate() {
            let prof = self.terrain.road_points(ri);
            for i in 0..road.pts.len().saturating_sub(1) {
                if !(prof[i].bridge && prof[i + 1].bridge) {
                    continue;
                }
                let a = Vec2::new(road.pts[i][0], road.pts[i][1]);
                let b = Vec2::new(road.pts[i + 1][0], road.pts[i + 1][1]);
                let mid = (a + b) * 0.5;
                let top = (prof[i].y + prof[i + 1].y) * 0.5;
                self.colliders.push(Collider {
                    prop: Collider::BRIDGE | ri as u32,
                    shape: Shape::Block(Block {
                        base: Vec3::new(mid.x, top - 1.6, mid.y),
                        half: Vec3::new(a.distance(b) * 0.5 + 0.5, 1.6, road.w * 0.5),
                        axis: (b - a).normalize_or(Vec2::X),
                        roof: 0.0,
                    }),
                });
            }
        }
        for b in &self.buildings {
            for shape in b.shapes() {
                self.colliders.push(Collider {
                    prop: Collider::BUILDING | b.id,
                    shape,
                });
            }
        }
        self.grid_n = ((self.half_size * 2.0 + 800.0) / CELL).ceil() as usize;
        self.grid = vec![Vec::new(); self.grid_n * self.grid_n];
        for (i, c) in self.colliders.iter().enumerate() {
            let (center, r) = c.shape.footprint();
            let (x0, z0) = self.cell(center.x - r, center.y - r);
            let (x1, z1) = self.cell(center.x + r, center.y + r);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    self.grid[z * self.grid_n + x].push(i as u32);
                }
            }
        }
    }

    fn cell(&self, x: f32, z: f32) -> (usize, usize) {
        let off = self.half_size + 400.0;
        let f = |v: f32| (((v + off) / CELL).floor().max(0.0) as usize).min(self.grid_n - 1);
        (f(x), f(z))
    }

    /// Colliders that might touch `(x, z)`.
    pub fn near(&self, x: f32, z: f32) -> impl Iterator<Item = &Collider> {
        let (cx, cz) = self.cell(x, z);
        self.grid[cz * self.grid_n + cx]
            .iter()
            .map(|i| &self.colliders[*i as usize])
    }

    /// Whether `p` is inside a building, prop or the ground.
    pub fn solid_at(&self, p: Vec3) -> bool {
        self.near(p.x, p.z)
            .any(|c| c.shape.push_out(p, 0.05).is_some())
            || self.terrain.sdf(p) < 0.0
    }

    /// Where to land for a spot: up on the ridge when it is on a sloping roof, where a
    /// bird would perch, else the spot itself.
    pub fn perch_spot(&self, p: Vec3) -> Vec3 {
        self.near(p.x, p.z)
            .find_map(|c| match &c.shape {
                Shape::Block(b) => b.ridge_beside(p),
                _ => None,
            })
            .unwrap_or(p)
    }

    /// Height of the highest standable surface at `(x, z)` that is not above `below`.
    /// Water counts as a surface for now, so a dragon can rest on it.
    pub fn support(&self, x: f32, z: f32, below: f32) -> f32 {
        let ground = self.terrain.support(x, z, below);
        let mut best = ground;
        // Under rock, only cave water counts: the sea or a river overhead is no floor.
        let water = if ground < self.terrain.height(x, z) - 1.0 {
            self.terrain.cave_water(x, z, ground)
        } else {
            self.terrain.water(x, z, ground)
        };
        if let Some(w) = water
            && w <= below
        {
            best = w;
        }
        for shape in self.near(x, z).map(|c| c.shape).chain(self.moving(x, z)) {
            if let Some(h) = shape.support(x, z)
                && h <= below
                && h > best
            {
                best = h;
            }
        }
        best
    }

    /// Solid vehicles that might touch `(x, z)`.
    fn moving(&self, x: f32, z: f32) -> impl Iterator<Item = Shape> + '_ {
        (0..self.traffic.vehicles.len())
            .filter(move |&i| {
                let p = self.traffic.poses[i].pos;
                let reach = self.traffic.vehicles[i].size.x;
                (p.x - x).abs() < reach && (p.z - z).abs() < reach
            })
            .flat_map(|i| self.traffic.shapes(i))
    }

    /// The vehicle someone standing at `feet` over `(x, z)` rides on, if any.
    /// The top of vehicle `i` over `(x, z)`, if it reaches there.
    pub fn deck_of(&self, i: usize, x: f32, z: f32) -> Option<f32> {
        self.traffic
            .shapes(i)
            .iter()
            .filter_map(|s| s.support(x, z))
            .reduce(f32::max)
    }

    pub fn carrier(&self, x: f32, z: f32, feet: f32) -> Option<usize> {
        (0..self.traffic.vehicles.len()).find(|&i| {
            let p = self.traffic.poses[i].pos;
            let reach = self.traffic.vehicles[i].size.x;
            (p.x - x).abs() < reach
                && (p.z - z).abs() < reach
                && self
                    .traffic
                    .shapes(i)
                    .iter()
                    .filter_map(|s| s.support(x, z))
                    .any(|h| (h - feet).abs() < 0.8)
        })
    }

    /// Which way the steep face at `p` looks, level, when `p` is on the side of a building,
    /// rock or cliff that rises at least a little above it. None on open ground, roofs and
    /// things too thin to cling to.
    pub fn wall_normal(&self, p: Vec3) -> Option<Vec3> {
        // A spot on top of something is not on its side.
        if self.support(p.x, p.z, p.y + 300.0) < p.y + 0.3 {
            return None;
        }
        let mut out = Vec3::ZERO;
        let mut solid = 0;
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            let d = Vec3::new(a.cos(), 0.0, a.sin());
            let q = p + d * 1.5;
            let top = self.support(q.x, q.z, p.y + 300.0);
            if top > p.y + 1.0 {
                solid += 1;
            } else if top < p.y - 1.0 {
                out += d;
            }
        }
        (solid >= 2 && out.length() > 0.5).then(|| out.normalize())
    }

    /// A car that has just driven into someone standing at `p` with feet at `feet`,
    /// `r` round: where its middle is and how far it moved this step. Someone already
    /// riding on top was inside its footprint before, so never counts.
    pub fn struck_by_car(&self, p: Vec3, feet: f32, r: f32) -> Option<(Vec3, Vec3)> {
        (0..self.traffic.vehicles.len()).find_map(|i| {
            let v = &self.traffic.vehicles[i];
            let (was, now) = (&self.traffic.prev[i], &self.traffic.poses[i]);
            if v.kind != Kind::Car || now.pos.distance(p) > v.size.x + r + 2.0 {
                return None;
            }
            let inside = |pose: &crate::traffic::Pose| {
                let l = pose.to_local(p);
                l.x.abs() < v.size.x * 0.5 + r + 0.2 && l.z.abs() < v.size.z * 0.5 + r + 0.2
            };
            (inside(now) && !inside(was) && feet < now.pos.y + v.size.y)
                .then_some((now.pos, now.pos - was.pos))
        })
    }

    /// Whether standing at height `feet` over `(x, z)` means standing on a prop or
    /// building rather than the ground.
    pub fn on_structure(&self, x: f32, z: f32, feet: f32) -> bool {
        self.terrain.support(x, z, feet + 0.6) < feet - 0.5
    }

    /// Pushes a sphere out of every prop it overlaps, ignoring the ground.
    pub fn push_out_props(&self, mut p: Vec3, r: f32) -> (Vec3, Vec3) {
        let mut normal = Vec3::ZERO;
        let shapes: Vec<Shape> = self
            .near(p.x, p.z)
            .map(|c| c.shape)
            .chain(self.moving(p.x, p.z))
            .collect();
        for shape in shapes {
            if let Some((n, depth)) = shape.push_out(p, r) {
                p += n * depth;
                normal += n;
            }
        }
        (p, normal.normalize_or_zero())
    }

    /// Pushes a sphere out of the ground and every prop it overlaps. Returns the
    /// corrected center and the summed outward normal (zero when nothing was hit).
    pub fn push_out(&self, mut p: Vec3, r: f32) -> (Vec3, Vec3) {
        let mut normal = Vec3::ZERO;
        if let Some((n, depth)) = self.terrain.push_out(p, r) {
            p += n * depth;
            normal += n;
        }
        let shapes: Vec<Shape> = self
            .near(p.x, p.z)
            .map(|c| c.shape)
            .chain(self.moving(p.x, p.z))
            .collect();
        for shape in shapes {
            if let Some((n, depth)) = shape.push_out(p, r) {
                p += n * depth;
                normal += n;
            }
        }
        (p, normal.normalize_or_zero())
    }

    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        self.terrain.height(x, z)
    }

    pub fn ground_normal(&self, x: f32, z: f32) -> Vec3 {
        self.terrain.normal(x, z)
    }

    fn push(&mut self, kind: PropKind, pos: Vec3, scale: f32, variant: u8) {
        let id = self.props.len() as u32;
        let yaw = (id as f32 * 2.399_963).rem_euclid(std::f32::consts::TAU);
        self.props.push(Prop {
            id,
            kind,
            pos,
            yaw,
            scale,
            variant,
        });
    }

    /// The ground height at a dry, gentle enough spot off the roads, or none.
    fn dry_spot(&self, x: f32, z: f32, min_up: f32) -> Option<f32> {
        let h = self.terrain.height(x, z);
        if h < 1.8
            || self.terrain.water(x, z, h).is_some()
            || self.terrain.road_spot(x, z, 3.0).is_some()
        {
            return None;
        }
        (self.terrain.normal(x, z).y >= min_up && self.terrain.rim(x, z, 6.0) < 2.0).then_some(h)
    }

    /// Calls `f` at one jittered point per `spacing` square over the cover's bounds.
    fn scatter(
        rng: &mut Rng,
        pts: &[[f32; 2]],
        spacing: f32,
        mut f: impl FnMut(&mut Rng, f32, f32),
    ) {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for p in pts {
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        }
        let mut z = lo[1];
        while z < hi[1] {
            let mut x = lo[0];
            while x < hi[0] {
                let (px, pz) = (x + rng.range(0.0, spacing), z + rng.range(0.0, spacing));
                f(rng, px, pz);
                x += spacing;
            }
            z += spacing;
        }
    }

    fn place_props(&mut self) {
        let mut rng = Rng::new(self.seed ^ 0x5EED);
        let def = self.terrain.def;
        let mut placed = Vec::new();
        for cover in def.covers {
            let mine = |x: f32, z: f32| {
                self.terrain
                    .cover(x, z)
                    .is_some_and(|c| std::ptr::eq(c, cover))
            };
            match cover.kind {
                CoverKind::Forest => {
                    let (spacing, scale, min_up) = match (cover.density, cover.trees) {
                        (Density::Open, Trees::Palm) => (34.0, (1.0, 1.5), 0.8),
                        (Density::Open, _) => (48.0, (2.6, 4.0), 0.8),
                        (Density::Rain, _) => (20.0, (1.8, 2.8), 0.7),
                        (Density::Dense, _) => (15.0, (1.3, 2.2), 0.7),
                    };
                    let kind = match cover.trees {
                        Trees::Pine => PropKind::PineTree,
                        Trees::Palm => PropKind::Palm,
                        _ => PropKind::RoundTree,
                    };
                    Self::scatter(&mut rng, cover.pts, spacing, |rng, x, z| {
                        let s = rng.range(scale.0, scale.1);
                        let v = (rng.next_u64() % 3) as u8;
                        if mine(x, z)
                            && let Some(h) = self.dry_spot(x, z, min_up)
                        {
                            placed.push((kind, Vec3::new(x, h - 0.3, z), s, v));
                        }
                    });
                }
                CoverKind::Meadow => {
                    Self::scatter(&mut rng, cover.pts, 40.0, |rng, cx, cz| {
                        let v = (rng.next_u64() % 4) as u8;
                        if rng.f32() > 0.55 || !mine(cx, cz) {
                            return;
                        }
                        for _ in 0..10 {
                            let x = cx + rng.range(-12.0, 12.0);
                            let z = cz + rng.range(-12.0, 12.0);
                            let s = rng.range(1.8, 2.8);
                            if let Some(h) = self.dry_spot(x, z, 0.8) {
                                placed.push((PropKind::Flower, Vec3::new(x, h, z), s, v));
                            }
                        }
                    });
                }
                // Ski runs stay open, with a few pines in clumps.
                CoverKind::Snow => {
                    Self::scatter(&mut rng, cover.pts, 70.0, |rng, cx, cz| {
                        if rng.f32() > 0.3 || !mine(cx, cz) {
                            return;
                        }
                        for _ in 0..4 {
                            let x = cx + rng.range(-10.0, 10.0);
                            let z = cz + rng.range(-10.0, 10.0);
                            let s = rng.range(1.6, 2.6);
                            let v = (rng.next_u64() % 3) as u8;
                            if let Some(h) = self.dry_spot(x, z, 0.7) {
                                placed.push((PropKind::PineTree, Vec3::new(x, h - 0.3, z), s, v));
                            }
                        }
                    });
                }
                CoverKind::Town => {}
            }
        }
        for (kind, pos, s, v) in placed {
            self.push(kind, pos, s, v);
        }
        let h = self.half_size;
        // A light scatter of fair weather clouds until real weather arrives.
        for _ in 0..450 {
            let (x, z) = (rng.range(-h, h), rng.range(-h, h));
            let pos = Vec3::new(x, rng.range(300.0, 1100.0), z);
            let s = rng.range(2.0, 5.0);
            let v = (rng.next_u64() % 3) as u8;
            let id = self.props.len() as u32;
            self.props.push(Prop {
                id,
                kind: PropKind::Cloud,
                pos,
                yaw: rng.range(0.0, std::f32::consts::TAU),
                scale: s,
                variant: v,
            });
        }
    }
}

/// The railway point nearest `at`: which railway and which of its points. Stations stand
/// beside it, and trains halt there.
pub fn nearest_rail(def: &crate::terrain::WorldDef, at: Vec2) -> Option<(usize, usize)> {
    def.roads
        .iter()
        .enumerate()
        .filter(|(_, r)| r.rail)
        .flat_map(|(i, r)| (0..r.pts.len() - 1).map(move |k| (i, k)))
        .min_by(|a, b| {
            let d = |(i, k): (usize, usize)| {
                let p = def.roads[i].pts[k];
                Vec2::new(p[0], p[1]).distance(at)
            };
            d(*a).total_cmp(&d(*b))
        })
}
