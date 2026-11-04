//! Towns: where every building, landmark, pier and moored boat stands. Laid out once from
//! the world files with a fixed seed, so every peer builds the same streets.
//!
//! Each town fills in three passes: landmarks at its sites, rows of buildings facing its
//! canals, rivers, roads, ledges and shore, then a loose grid over what is left. Styles
//! and tech levels pick the sizes and kinds. Only add, multiply, divide and sqrt are used,
//! so the layout matches on every machine.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::shape::{Block, Frustum, Shape};
use crate::terrain::def::Mark;
use crate::terrain::{CarveKind, Style, Tech, Terrain, Town};
use crate::world::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Gable roofed house, ridge along its width.
    House,
    /// Narrow tall town house with its gable to the street.
    Row,
    /// Small hut with a steep thatched roof.
    Hut,
    /// Flat roofed house or block.
    Flat,
    /// Round tower with a cone roof.
    Tower,
    Skyscraper,
    Warehouse,
    /// Hall with a spire at one end.
    Church,
    /// A big hall: market, keep or town hall.
    Hall,
    /// Round house with a dome.
    Dome,
    Lighthouse,
    /// Radio and TV tower.
    Mast,
    Terminal,
    ControlTower,
    Hangar,
    Crane,
    /// A ship being built on a slipway.
    Hull,
    Boat,
    Sailboat,
    Houseboat,
    Ferry,
    /// A huge cruise ship on its long route round the islands: decks to land on, funnels,
    /// a pool and rows of lit cabins.
    Cruise,
    Seaplane,
    Airliner,
    Canoe,
    /// Plank deck on posts over water.
    Pier,
    /// Little arched bridge over a canal.
    Footbridge,
    Lantern,
    Windmill,
    Wreck,
    /// Only on the roads, never placed in towns.
    Car,
    /// A hot air balloon drifting on its route. Land on top of it!
    Balloon,
    /// A chairlift's chair and a gondola's cabin, hanging from the cable.
    LiftChair,
    LiftCabin,
    /// A lift's pylon, `size.z` the width between its two cables, and the station at
    /// each end where the cable turns round.
    LiftTower,
    LiftStation,
    /// A railway engine (variant 0) or carriage, on its track.
    Train,
    // Landmarks from the world files, see [`crate::landmark`].
    Stones,
    Turbine,
    Campsite,
    Picnic,
    PhoneBox,
    Umbrellas,
    Scarecrow,
    Telescope,
    DriveIn,
    Sign,
    Ferris,
    Swing,
    Maze,
    Sandcastle,
    Snowman,
    Bench,
    Gnomes,
    Beehives,
    Pumpkins,
    BusStop,
    WaterTower,
    HotSpring,
    Chess,
    Cairns,
    Treehouse,
    Well,
    Lemonade,
    Igloo,
    Laundry,
    Sundial,
    Circus,
    Fountain,
    Postbox,
    Bandstand,
    Stadium,
    Waterpark,
    ThemePark,
    Zoo,
    Statue,
    Mall,
    Grocery,
    Park,
    Station,
    /// A subway station in its hall underground, beside the track.
    Metro,
    /// A subway entrance in the street: a railed opening over the stair shaft.
    MetroGate,
    /// A market square: stalls under striped awnings, piled with produce.
    Market,
    Observatory,
    /// A fire lookout tower in the woods.
    Lookout,
    SurfShack,
    /// A rope bridge between two heights: its length and rise are in its text.
    RopeBridge,
    Hospital,
    FireStation,
    School,
    TownHall,
    Farm,
    /// A container port on a quay, with gantry cranes.
    Port,
    /// An office block straddling a road: two glass towers either side, joined high up,
    /// the road running through the opening under it.
    SpanTower,
    /// One tower of a suspension bridge: two legs either side of the deck, x across it,
    /// joined by beams.
    BridgeTower,
}

/// A bridge tower leg's width across the deck and depth along it.
pub const TOWER_LEG: Vec2 = Vec2::new(3.2, 4.4);

/// Height of a lighthouse's lamp room over its tower. Its flat cap is a perch.
pub const LAMP_ROOM: f32 = 3.4;

/// One placed building or thing. Its shape is a turned box: `axis` is the world direction
/// of its local x, its front faces local +z.
#[derive(Clone, Copy, Debug)]
pub struct Building {
    pub id: u32,
    pub kind: Kind,
    pub style: Style,
    pub tech: Tech,
    /// Center of the footprint at the base.
    pub pos: Vec3,
    pub axis: Vec2,
    /// Width (local x), wall height, depth (local z).
    pub size: Vec3,
    /// Ridge height over the walls, 0 for flat.
    pub roof: f32,
    /// Posts reach this far below the base.
    pub stilts: f32,
    pub seed: u32,
    /// The site in the world files this landmark stands for, such as `home/kettle-light`.
    pub site: Option<&'static str>,
    /// Words on it, for signs.
    pub text: &'static str,
}

impl Building {
    /// World direction of local z, where the front faces.
    pub fn front(&self) -> Vec2 {
        Vec2::new(-self.axis.y, self.axis.x)
    }

    fn block(&self, base: Vec3, w: f32, h: f32, d: f32, roof: f32) -> Shape {
        Shape::Block(Block {
            base,
            half: Vec3::new(w * 0.5, h, d * 0.5),
            axis: self.axis,
            roof,
        })
    }

    /// A point in this building's frame: x along its axis, y up, z toward its front.
    pub fn at(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let f = self.front();
        let p = Vec2::new(self.pos.x, self.pos.z) + self.axis * x + f * z;
        Vec3::new(p.x, self.pos.y + y, p.y)
    }

    /// The world direction of a piece's x when turned `yaw` within this building.
    pub fn turned(&self, yaw: f32) -> Vec2 {
        self.axis * yaw.cos() - self.front() * yaw.sin()
    }

    /// The solid parts of this building, which the mesh follows.
    pub fn shapes(&self) -> Vec<Shape> {
        let s = self.size;
        let round = |base: Vec3, h: f32, r0: f32, r1: f32| {
            Shape::Frustum(Frustum {
                base,
                height: h,
                r0,
                r1,
                spin: 0.0,
            })
        };
        match self.kind {
            Kind::Tower | Kind::Windmill => {
                vec![round(self.pos, s.y, s.x * 0.5, s.x * 0.42)]
            }
            Kind::Lighthouse => {
                let top = s.x * 0.5 * 0.64;
                vec![
                    round(self.pos, s.y, s.x * 0.5, top),
                    round(
                        self.at(0.0, s.y, 0.0),
                        LAMP_ROOM,
                        top * 0.8 + 0.4,
                        top * 0.8 + 0.4,
                    ),
                ]
            }
            Kind::Mast => vec![
                round(self.pos, s.y, s.x * 0.5, s.x * 0.2),
                Shape::Sphere {
                    center: self.at(0.0, s.y * 0.72, 0.0),
                    radius: s.x * 1.4,
                },
            ],
            Kind::ControlTower => vec![
                round(self.pos, s.y, s.x * 0.3, s.x * 0.25),
                self.block(self.at(0.0, s.y, 0.0), s.x, 5.0, s.x, 0.0),
            ],
            Kind::Church => vec![
                self.block(self.pos, s.x, s.y, s.z, self.roof),
                round(self.at(s.x * 0.5 + 3.0, 0.0, 0.0), s.y * 2.6, 3.4, 0.3),
            ],
            Kind::Lantern | Kind::Canoe => Vec::new(),
            k if crate::landmark::KINDS.iter().any(|(_, l)| *l == k) => {
                crate::landmark::pieces(k, self.seed, self.text)
                    .into_iter()
                    .filter(|p| p.solid)
                    .map(|p| match p.form {
                        crate::landmark::Form::Box {
                            at,
                            half,
                            yaw,
                            roof,
                        } => Shape::Block(Block {
                            base: self.at(at.x, at.y, at.z),
                            half,
                            axis: self.turned(yaw),
                            roof,
                        }),
                        crate::landmark::Form::Round { at, h, r0, r1 } => {
                            round(self.at(at.x, at.y, at.z), h, r0, r1)
                        }
                        crate::landmark::Form::Ball { at, r } => Shape::Sphere {
                            center: self.at(at.x, at.y, at.z),
                            radius: r,
                        },
                    })
                    .collect()
            }
            Kind::Crane => vec![round(self.pos, s.y, 1.2, 1.2)],
            Kind::BridgeTower => {
                let leg = TOWER_LEG;
                let spread = (s.x - leg.x) * 0.5;
                vec![
                    self.block(self.at(-spread, 0.0, 0.0), leg.x, s.y, leg.y, 0.0),
                    self.block(self.at(spread, 0.0, 0.0), leg.x, s.y, leg.y, 0.0),
                    self.block(self.at(0.0, s.y - 3.0, 0.0), s.x, 3.0, 3.0, 0.0),
                ]
            }
            // Only the roof is solid, so chairs and their riders pass through underneath.
            Kind::LiftStation => {
                vec![self.block(self.at(0.0, s.y - 0.4, 0.0), s.x, 0.4, s.z, self.roof)]
            }
            Kind::LiftTower => vec![
                round(self.pos, s.y, 0.6, 0.45),
                self.block(self.at(0.0, s.y - 0.3, 0.0), 0.6, 0.5, s.z + 1.2, 0.0),
            ],
            Kind::Pier | Kind::Footbridge => {
                vec![self.block(self.at(0.0, s.y - 0.6, 0.0), s.x, 0.6, s.z, 0.0)]
            }
            _ => vec![self.block(self.pos, s.x, s.y, s.z, self.roof)],
        }
    }
}

/// Spacing and sizes for one style.
struct Plan {
    /// Grid spacing for the fill pass.
    fill: f32,
    /// Spacing along fronts.
    front: f32,
    /// Gap between a front line's edge and the buildings.
    setback: f32,
    /// How much ground may slope under one building (m).
    slope: f32,
    max: usize,
}

fn plan(t: &Town) -> Plan {
    let (fill, front, setback, slope) = match t.style {
        Style::Canal => (24.0, 9.0, 4.0, 6.0),
        Style::Hill => (24.0, 12.0, 4.0, 3.5),
        Style::Cliff => (30.0, 12.0, 2.0, 3.0),
        Style::Stone => (24.0, 12.0, 4.0, 6.0),
        Style::Seafaring => (26.0, 12.0, 5.0, 6.0),
        Style::Modern => (40.0, 34.0, 8.0, 8.0),
        Style::Cavern => (40.0, 16.0, 4.0, 6.0),
        Style::Stilts => (26.0, 14.0, 2.0, 6.0),
        Style::Plain => (30.0, 18.0, 5.0, 5.0),
    };
    let village = t.village;
    Plan {
        fill: if village { fill * 1.3 } else { fill },
        front,
        setback,
        slope,
        max: if village { 180 } else { 1500 },
    }
}

/// Whether `p` is inside an outline.
fn inside(pts: &[[f32; 2]], p: Vec2) -> bool {
    let n = pts.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a[1] > p.y) != (b[1] > p.y) {
            let x = a[0] + (p.y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p.x < x {
                c = !c;
            }
        }
    }
    c
}

/// A turned rectangle on the ground.
#[derive(Clone, Copy, Debug)]
struct Foot {
    c: Vec2,
    axis: Vec2,
    half: Vec2,
}

impl Foot {
    /// Separating axis test between two turned rectangles.
    fn overlaps(&self, o: &Foot) -> bool {
        let d = o.c - self.c;
        let axes = [
            self.axis,
            Vec2::new(-self.axis.y, self.axis.x),
            o.axis,
            Vec2::new(-o.axis.y, o.axis.x),
        ];
        let reach = |f: &Foot, n: Vec2| {
            f.half.x * f.axis.dot(n).abs() + f.half.y * Vec2::new(-f.axis.y, f.axis.x).dot(n).abs()
        };
        axes.iter()
            .all(|n| d.dot(*n).abs() < reach(self, *n) + reach(o, *n))
    }
}

/// A line buildings front onto: points, its half width, and which sides take buildings.
struct Line {
    pts: Vec<Vec2>,
    half: f32,
    sides: [bool; 2],
    /// Tall buildings go here in modern towns.
    water: bool,
    /// An edge between terraces: only build on its high side, looking out over it.
    uphill: bool,
}

struct Planner<'a> {
    terrain: &'a Terrain,
    town: &'a Town,
    rng: Rng,
    plan: Plan,
    taken: HashMap<(i32, i32), Vec<Foot>>,
    out: Vec<Building>,
    center: Vec2,
    /// Widens streets and plots, for towns that would otherwise overflow.
    spread: f32,
}

const CELL: f32 = 24.0;

impl<'a> Planner<'a> {
    fn new(terrain: &'a Terrain, town: &'a Town, seed: u64) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ seed;
        for b in town.id.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let mut c = Vec2::ZERO;
        for p in town.pts {
            c += Vec2::new(p[0], p[1]);
        }
        Self {
            terrain,
            town,
            rng: Rng::new(h),
            plan: plan(town),
            taken: HashMap::new(),
            out: Vec::new(),
            center: c / town.pts.len().max(1) as f32,
            spread: 1.0,
        }
    }

    fn ground(&self, p: Vec2) -> f32 {
        if self.town.under {
            self.terrain.support(p.x, p.y, self.town.y + 40.0)
        } else {
            self.terrain.height(p.x, p.y)
        }
    }

    /// Whether a footprint, grown by `gap` all round, is clear of everything placed.
    fn free(&self, f: &Foot, gap: f32) -> bool {
        let grown = Foot {
            half: f.half + Vec2::splat(gap),
            ..*f
        };
        let r = grown.half.length();
        let (cx, cz) = ((f.c.x / CELL).floor() as i32, (f.c.y / CELL).floor() as i32);
        let reach = (r / CELL).ceil() as i32 + 2;
        for z in cz - reach..=cz + reach {
            for x in cx - reach..=cx + reach {
                if let Some(list) = self.taken.get(&(x, z))
                    && list.iter().any(|o| o.overlaps(&grown))
                {
                    return false;
                }
            }
        }
        true
    }

    fn take(&mut self, f: Foot) {
        let key = ((f.c.x / CELL).floor() as i32, (f.c.y / CELL).floor() as i32);
        self.taken.entry(key).or_default().push(f);
    }

    /// Places a building on dry, gentle ground if it fits. `axis` is its local x.
    #[allow(clippy::too_many_arguments)]
    fn try_place(
        &mut self,
        kind: Kind,
        c: Vec2,
        axis: Vec2,
        size: Vec3,
        roof: f32,
        gap: f32,
    ) -> bool {
        if self.out.len() >= self.plan.max {
            return false;
        }
        let f = Vec2::new(-axis.y, axis.x);
        let (hx, hz) = (size.x * 0.5, size.z * 0.5);
        let corners = [
            c + axis * hx + f * hz,
            c + axis * hx - f * hz,
            c - axis * hx + f * hz,
            c - axis * hx - f * hz,
        ];
        let foot = Foot {
            c,
            axis,
            half: Vec2::new(hx, hz),
        };
        if !corners.iter().all(|p| inside(self.town.pts, *p)) || !self.free(&foot, gap) {
            return false;
        }
        let stilts = self.town.style == Style::Stilts;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut water = f32::MIN;
        let under = self.town.under;
        // Water far overhead (the sea over a cavern) does not count.
        let wet_at = |p: Vec2, g: f32| {
            self.terrain
                .water(p.x, p.y, g)
                .filter(|w| !under || *w - g < 15.0)
        };
        for p in corners.iter().chain([&c]) {
            let g = self.ground(*p);
            lo = lo.min(g);
            hi = hi.max(g);
            if let Some(w) = wet_at(*p, g) {
                water = water.max(w);
            }
            if !under && self.terrain.road_spot(p.x, p.y, 1.5).is_some() {
                return false;
            }
        }
        // Big footprints can straddle a road between their corners: check across them.
        if !under {
            let steps = (size.x.max(size.z) / 6.0).ceil().clamp(1.0, 8.0) as i32;
            for i in -steps..=steps {
                for j in -steps..=steps {
                    let p = c
                        + axis * hx * (i as f32 / steps as f32)
                        + f * hz * (j as f32 / steps as f32);
                    if self.terrain.road_spot(p.x, p.y, 0.3).is_some() {
                        return false;
                    }
                }
            }
        }
        // Keep clear of canal and river edges too.
        for p in corners {
            let out = p + (p - c).normalize_or_zero() * 3.0;
            let g = self.ground(out);
            if !stilts && wet_at(out, g).is_some() {
                return false;
            }
        }
        let wet = water > f32::MIN;
        let mut base = lo - 0.4;
        let mut posts = 0.0;
        if stilts {
            if wet && water - lo > 4.5 {
                return false;
            }
            let top = if wet { water + 2.2 } else { hi + 1.6 };
            posts = top - lo + 0.6;
            base = top;
        } else if wet || hi - lo > self.plan.slope {
            return false;
        }
        if self.town.under {
            let room = self
                .terrain
                .sdf(Vec3::new(c.x, base + size.y + roof + 3.0, c.y));
            if room < 2.0 || (lo - self.town.y).abs() > 60.0 {
                return false;
            }
        }
        let walls = size.y + (hi - base).max(0.0) * if stilts { 0.0 } else { 1.0 };
        self.take(foot);
        let seed = self.rng.next_u64() as u32;
        self.out.push(Building {
            id: 0,
            kind,
            style: self.town.style,
            tech: self.town.tech,
            pos: Vec3::new(c.x, base, c.y),
            axis,
            size: Vec3::new(size.x, walls, size.z),
            roof,
            stilts: posts,
            seed,
            site: None,
            text: "",
        });
        true
    }

    /// What to build at a spot: kind, footprint and walls, roof. `front` buildings line a
    /// street or water, `near_water` for the tallest modern towers.
    fn pick(&mut self, front: bool, near_water: bool) -> (Kind, Vec3, f32) {
        let r = &mut self.rng;
        let tech = self.town.tech;
        let roll = r.f32();
        match self.town.style {
            Style::Canal => {
                if front {
                    let (w, d, h) = (r.range(6.5, 8.5), r.range(11.0, 14.0), r.range(9.0, 15.0));
                    (Kind::Row, Vec3::new(w, h, d), r.range(4.0, 6.0))
                } else if roll < 0.25 {
                    (
                        Kind::Warehouse,
                        Vec3::new(r.range(16.0, 22.0), r.range(8.0, 11.0), r.range(10.0, 13.0)),
                        r.range(3.0, 4.0),
                    )
                } else {
                    (
                        Kind::House,
                        Vec3::new(r.range(10.0, 14.0), r.range(7.0, 11.0), r.range(8.0, 10.0)),
                        r.range(3.5, 5.0),
                    )
                }
            }
            Style::Hill => {
                let (w, d) = (r.range(8.0, 13.0), r.range(8.0, 11.0));
                if roll < 0.35 {
                    (Kind::Flat, Vec3::new(w, r.range(5.0, 9.0), d), 0.0)
                } else if roll < 0.4 {
                    {
                        let w = r.range(6.0, 8.0);
                        (Kind::Tower, Vec3::new(w, r.range(14.0, 20.0), w), 0.0)
                    }
                } else {
                    (
                        Kind::House,
                        Vec3::new(w, r.range(5.0, 9.0), d),
                        r.range(2.0, 3.0),
                    )
                }
            }
            Style::Cliff => {
                let (w, d) = (r.range(8.0, 12.0), r.range(7.5, 10.0));
                if roll < 0.08 {
                    // Tall houses built up against the cliff, reaching the ledge above.
                    (Kind::Flat, Vec3::new(w, r.range(38.0, 48.0), d), 0.0)
                } else if roll < 0.2 {
                    (Kind::Dome, Vec3::new(d, r.range(5.0, 7.0), d), 0.0)
                } else if roll < 0.7 {
                    (Kind::Flat, Vec3::new(w, r.range(5.0, 9.0), d), 0.0)
                } else {
                    (
                        Kind::House,
                        Vec3::new(w, r.range(5.0, 8.0), d),
                        r.range(2.0, 3.0),
                    )
                }
            }
            Style::Stone => {
                if roll < 0.1 {
                    {
                        let w = r.range(7.0, 10.0);
                        (Kind::Tower, Vec3::new(w, r.range(16.0, 26.0), w), 0.0)
                    }
                } else {
                    let (w, d) = (r.range(9.0, 13.0), r.range(8.0, 11.0));
                    (
                        Kind::House,
                        Vec3::new(w, r.range(6.0, 10.0), d),
                        r.range(4.0, 5.5),
                    )
                }
            }
            Style::Seafaring => {
                if front && roll < 0.5 {
                    (
                        Kind::Warehouse,
                        Vec3::new(r.range(16.0, 24.0), r.range(8.0, 11.0), r.range(11.0, 14.0)),
                        r.range(2.5, 3.5),
                    )
                } else if front {
                    (
                        Kind::Row,
                        Vec3::new(r.range(7.0, 9.0), r.range(8.0, 12.0), r.range(10.0, 12.0)),
                        r.range(3.5, 5.0),
                    )
                } else {
                    (
                        Kind::House,
                        Vec3::new(r.range(9.0, 12.0), r.range(6.0, 9.0), r.range(8.0, 10.0)),
                        r.range(3.0, 4.0),
                    )
                }
            }
            Style::Modern => {
                if near_water {
                    let w = r.range(20.0, 28.0);
                    (
                        Kind::Skyscraper,
                        Vec3::new(w, r.range(50.0, 150.0), r.range(18.0, 26.0)),
                        0.0,
                    )
                } else if roll < 0.35 {
                    let w = r.range(18.0, 26.0);
                    (
                        Kind::Skyscraper,
                        Vec3::new(w, r.range(26.0, 60.0), r.range(16.0, 24.0)),
                        0.0,
                    )
                } else {
                    (
                        Kind::Flat,
                        Vec3::new(
                            r.range(16.0, 24.0),
                            r.range(10.0, 22.0),
                            r.range(14.0, 20.0),
                        ),
                        0.0,
                    )
                }
            }
            Style::Cavern => {
                if roll < 0.4 {
                    {
                        let w = r.range(8.0, 11.0);
                        (Kind::Dome, Vec3::new(w, r.range(4.0, 6.0), w), 0.0)
                    }
                } else {
                    (
                        Kind::House,
                        Vec3::new(r.range(8.0, 11.0), r.range(4.5, 7.0), r.range(7.0, 9.0)),
                        r.range(3.0, 4.0),
                    )
                }
            }
            Style::Stilts => (
                Kind::Hut,
                Vec3::new(r.range(6.0, 8.0), r.range(3.0, 3.8), r.range(6.0, 7.5)),
                r.range(3.0, 4.0),
            ),
            Style::Plain => match tech {
                Tech::Modern => {
                    let (w, d) = (r.range(10.0, 14.0), r.range(9.0, 11.0));
                    if roll < 0.6 {
                        (Kind::Flat, Vec3::new(w, r.range(4.0, 7.0), d), 0.0)
                    } else {
                        (
                            Kind::House,
                            Vec3::new(w, r.range(4.0, 6.0), d),
                            r.range(2.5, 3.5),
                        )
                    }
                }
                Tech::Simple => (
                    Kind::Hut,
                    Vec3::new(r.range(6.0, 8.0), r.range(3.0, 3.8), r.range(6.0, 7.5)),
                    r.range(3.0, 4.0),
                ),
                _ => {
                    let (w, d) = (r.range(9.0, 12.0), r.range(7.0, 9.0));
                    (
                        Kind::House,
                        Vec3::new(w, r.range(4.0, 6.5), d),
                        r.range(3.0, 4.2),
                    )
                }
            },
        }
    }

    /// Places one building along a front, facing `face` (unit, from the building toward
    /// the line).
    fn front_one(&mut self, at: Vec2, face: Vec2, near_water: bool) -> bool {
        let (kind, size, roof) = self.pick(true, near_water);
        // Village houses sit a little out of line.
        let loose = matches!(
            self.town.style,
            Style::Stilts | Style::Plain | Style::Cavern
        );
        let (at, face) = if loose {
            let t = self.rng.range(-0.12, 0.12);
            let turn = Vec2::new(1.0 - t * t, 2.0 * t) / (1.0 + t * t);
            let face = Vec2::new(
                face.x * turn.x - face.y * turn.y,
                face.x * turn.y + face.y * turn.x,
            );
            (at - face * self.rng.range(0.0, 4.0), face)
        } else {
            (at, face)
        };
        // Row houses show their gable to the street, so their ridge runs toward it.
        let (axis, size) = if kind == Kind::Row {
            (face, Vec3::new(size.z, size.y, size.x))
        } else {
            (Vec2::new(face.y, -face.x), size)
        };
        let back = if kind == Kind::Row { size.x } else { size.z };
        let c = at - face * back * 0.5;
        self.try_place(kind, c, axis, size, roof, 0.8)
    }

    /// Rows of buildings along every line, on each allowed side.
    fn fronts(&mut self, lines: &[Line]) {
        for line in lines {
            for (side, &on) in [-1.0f32, 1.0].iter().zip(&line.sides) {
                if !on {
                    continue;
                }
                let mut carry = self.rng.range(0.0, self.plan.front);
                for w in line.pts.windows(2) {
                    let seg = w[1] - w[0];
                    let len = seg.length();
                    if len < 0.01 {
                        continue;
                    }
                    let dir = seg / len;
                    let n = Vec2::new(-dir.y, dir.x) * *side;
                    let mut t = carry;
                    while t < len {
                        let p = w[0] + dir * t;
                        if line.uphill {
                            // Stand back from the edge on the high side, looking out.
                            if self.ground(p + n * 8.0) > self.ground(p - n * 8.0) + 2.0 {
                                let at = p + n * (self.plan.setback + 1.0);
                                self.front_one(at, -n, false);
                            }
                            t += self.plan.front * self.rng.range(0.85, 1.25);
                            continue;
                        }
                        let edge = p + n * (line.half + self.plan.setback);
                        self.front_one(edge, -n, line.water);
                        t += self.plan.front * self.rng.range(0.85, 1.25);
                    }
                    carry = t - len;
                }
            }
        }
    }

    /// The main direction of the town, from its outline's spread.
    fn grain(&self) -> Vec2 {
        let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
        for p in self.town.pts {
            let d = Vec2::new(p[0], p[1]) - self.center;
            a += d.x * d.x;
            b += d.x * d.y;
            c += d.y * d.y;
        }
        let half = (a - c) * 0.5;
        let big = (a + c) * 0.5 + (half * half + b * b).sqrt();
        Vec2::new(b, big - a).try_normalize().unwrap_or(Vec2::X)
    }

    /// The town's spread in its own frame: main direction, cross direction and the
    /// outline's extent along each, measured from the center.
    fn frame(&self) -> (Vec2, Vec2, Vec2, Vec2) {
        let u = self.grain();
        let v = Vec2::new(-u.y, u.x);
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for p in self.town.pts {
            let d = Vec2::new(p[0], p[1]) - self.center;
            let q = Vec2::new(d.dot(u), d.dot(v));
            lo = lo.min(q);
            hi = hi.max(q);
        }
        (u, v, lo, hi)
    }

    /// Fills the rest of the town: terraced streets of joined houses in old and busy
    /// towns, loose plots elsewhere. Cliff towns only build on their ledges.
    fn fill(&mut self) {
        match self.town.style {
            Style::Cliff => {}
            Style::Canal | Style::Seafaring | Style::Stone | Style::Hill => self.streets(),
            _ => self.plots(),
        }
    }

    /// Blocks of houses wall to wall, in back to back rows between streets, with gardens
    /// behind. Streets wander gently, so rows bend like real neighborhoods.
    fn streets(&mut self) {
        let (u, v, lo, hi) = self.frame();
        let band = 14.0;
        let mut b = lo.y + self.rng.range(0.0, 11.0);
        while b < hi.y {
            b += self.rng.range(9.0, 14.0) * self.spread;
            self.row(u, v, lo.x, hi.x, b, band, -v);
            // Back gardens between the two rows.
            b += band + self.rng.range(5.0, 10.0) + (self.spread - 1.0) * 8.0;
            self.row(u, v, lo.x, hi.x, b, band, v);
            b += band;
        }
    }

    /// How far a street strays across from its straight line at `a` along it.
    fn wander(a: f32, b: f32) -> f32 {
        crate::terrain::noise(33, a / 220.0, b / 160.0) * 42.0
            + crate::terrain::noise(35, a / 90.0, b / 90.0) * 9.0
            + crate::terrain::noise(34, a / 60.0, b / 60.0) * 3.0
    }

    /// One row of joined buildings along `u`, in the band from `b` to `b + band` across,
    /// fronts lined up on the street side `face`, following the street's bends.
    #[allow(clippy::too_many_arguments)]
    fn row(&mut self, u: Vec2, v: Vec2, a0: f32, a1: f32, b: f32, band: f32, face: Vec2) {
        let mut a = a0 + self.rng.range(0.0, 30.0);
        while a < a1 {
            let end = a + self.rng.range(35.0, 85.0);
            // Now and then a block is left open: a square, a garden, a yard.
            if self.rng.f32() < 0.14 * self.spread {
                a = end + 12.0;
                continue;
            }
            while a < end && a < a1 {
                let (kind, size, roof) = self.pick(self.town.style == Style::Canal, false);
                let along = size.x;
                let mid = a + along * 0.5;
                // The street's direction here, from how fast it wanders.
                let slope = (Self::wander(mid + 2.0, b) - Self::wander(mid - 2.0, b)) / 4.0;
                let dir = (u + v * slope).normalize();
                let side = Vec2::new(-dir.y, dir.x);
                let f = if side.dot(face) > 0.0 { side } else { -side };
                // A little turn and setback of its own.
                let t = self.rng.range(-0.07, 0.07);
                let turn = Vec2::new(1.0 - t * t, 2.0 * t) / (1.0 + t * t);
                let f = Vec2::new(f.x * turn.x - f.y * turn.y, f.x * turn.y + f.y * turn.x);
                // Rows turn their gables to the street.
                let (axis, size) = if kind == Kind::Row {
                    (f, Vec3::new(size.z, size.y, size.x))
                } else {
                    (Vec2::new(f.y, -f.x), size)
                };
                let (along, deep) = if kind == Kind::Row {
                    (size.z, size.x)
                } else {
                    (size.x, size.z)
                };
                let set = self.rng.range(0.0, 2.6);
                let across = if face.dot(v) < 0.0 {
                    b + deep * 0.5 + set
                } else {
                    b + band - deep * 0.5 - set
                };
                let c = self.center + u * (a + along * 0.5) + v * (across + Self::wander(mid, b));
                if self.try_place(kind, c, axis, size, roof, 0.15) {
                    a += along + self.rng.range(0.3, 1.2);
                } else {
                    a += 5.0;
                }
            }
            // A cross street between blocks.
            a = end + self.rng.range(10.0, 16.0) * self.spread;
        }
    }

    /// Loose plots on a jittered grid, each house a little turned.
    fn plots(&mut self) {
        let (u, v, lo, hi) = self.frame();
        let s = self.plan.fill * self.spread;
        let mut b = lo.y;
        let mut row = 0;
        while b < hi.y {
            let mut a = lo.x;
            while a < hi.x {
                let jitter = Vec2::new(self.rng.range(-0.2, 0.2), self.rng.range(-0.2, 0.2)) * s;
                let c = self.center + u * a + v * b + jitter;
                let (kind, size, roof) = self.pick(false, false);
                let face = if row % 2 == 0 { v } else { -v };
                // A slight turn, by a rational rotation so no trig is needed.
                let t = if self.town.style == Style::Modern {
                    0.0
                } else {
                    self.rng.range(-0.14, 0.14)
                };
                let turn = Vec2::new(1.0 - t * t, 2.0 * t) / (1.0 + t * t);
                let face = Vec2::new(
                    face.x * turn.x - face.y * turn.y,
                    face.x * turn.y + face.y * turn.x,
                );
                let axis = Vec2::new(face.y, -face.x);
                self.try_place(kind, c, axis, size, roof, 2.0);
                a += s;
            }
            b += s;
            row += 1;
        }
    }

    /// Lines to build along: canals and rivers through town, roads, ledges and the shore.
    fn lines(&self) -> Vec<Line> {
        let def = self.terrain.def;
        let town = self.town;
        let near = |pts: &[Vec2]| pts.iter().any(|p| inside(town.pts, *p));
        let mut out = Vec::new();
        for c in def.carves.iter().filter(|c| c.kind == CarveKind::Canal) {
            let pts: Vec<Vec2> = c.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            if near(&pts) {
                out.push(Line {
                    pts,
                    half: c.w * 0.5,
                    sides: [true, true],
                    water: true,
                    uphill: false,
                });
            }
        }
        for r in def.rivers.iter().filter(|r| !r.under) {
            let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            if near(&pts) {
                out.push(Line {
                    pts,
                    half: r.w * 0.5 + 6.0,
                    sides: [true, true],
                    water: true,
                    uphill: false,
                });
            }
        }
        for r in def.roads.iter().filter(|r| !r.runway) {
            let pts: Vec<Vec2> = r.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            if near(&pts) {
                out.push(Line {
                    pts,
                    half: r.w * 0.5,
                    sides: [true, true],
                    water: false,
                    uphill: false,
                });
            }
        }
        if town.style == Style::Cliff {
            for s in def.shelves {
                let pts: Vec<Vec2> = s.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
                if near(&pts) {
                    // Buildings sit on the ledge, backed against the cliff.
                    out.push(Line {
                        pts,
                        half: -s.w * 0.5 + 1.0,
                        sides: [true, true],
                        water: false,
                        uphill: false,
                    });
                }
            }
        }
        // The edges of terraces, found by walking out from the middle along many rays.
        if !town.tiers.is_empty() {
            let mut dirs = vec![Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y];
            for _ in 0..4 {
                let mut more = Vec::new();
                for i in 0..dirs.len() {
                    let (a, b) = (dirs[i], dirs[(i + 1) % dirs.len()]);
                    more.push(a);
                    more.push((a + b).normalize());
                }
                dirs = more;
            }
            for &level in town.tiers {
                let mut run: Vec<Vec2> = Vec::new();
                for d in dirs.iter().chain(dirs.first()) {
                    let mut hit = None;
                    let mut was_up = false;
                    let mut r = 0.0;
                    while r < 1500.0 {
                        let p = self.center + *d * r;
                        let h = self.terrain.height(p.x, p.y);
                        if h > level - 1.0 {
                            was_up = true;
                        } else if was_up {
                            hit = Some(self.center + *d * (r - 5.0));
                            break;
                        }
                        r += 5.0;
                    }
                    match hit {
                        Some(p) if inside(town.pts, p) => run.push(p),
                        _ => {
                            if run.len() > 1 {
                                out.push(Line {
                                    pts: std::mem::take(&mut run),
                                    half: 0.0,
                                    sides: [true, true],
                                    water: false,
                                    uphill: true,
                                });
                            }
                            run.clear();
                        }
                    }
                }
                if run.len() > 1 {
                    out.push(Line {
                        pts: run,
                        half: 0.0,
                        sides: [true, true],
                        water: false,
                        uphill: true,
                    });
                }
            }
        }
        // The shore, walked along every coast inside the town, land side only.
        for land in def.lands {
            let n = land.pts.len();
            let mut run: Vec<Vec2> = Vec::new();
            for i in 0..=n {
                let p = Vec2::new(land.pts[i % n][0], land.pts[i % n][1]);
                if i < n && inside(town.pts, p) {
                    run.push(p);
                } else if run.len() > 1 {
                    out.push(Line {
                        pts: std::mem::take(&mut run),
                        half: 4.0,
                        sides: [true, true],
                        water: true,
                        uphill: false,
                    });
                } else {
                    run.clear();
                }
            }
        }
        out
    }

    /// Footbridges every so often over canals, and houseboats along their edges.
    fn canal_life(&mut self) {
        let def = self.terrain.def;
        for c in def.carves.iter().filter(|c| c.kind == CarveKind::Canal) {
            let pts: Vec<Vec2> = c.pts.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            if !pts.iter().any(|p| inside(self.town.pts, *p)) {
                continue;
            }
            let mut along = self.rng.range(30.0, 80.0);
            let mut boat = self.rng.range(10.0, 30.0);
            for w in pts.windows(2) {
                let seg = w[1] - w[0];
                let len = seg.length();
                let dir = seg / len.max(0.01);
                let mut t = 0.0;
                while t < len {
                    let p = w[0] + dir * t;
                    if along <= 0.0 {
                        along = self.rng.range(110.0, 170.0);
                        self.push(
                            Kind::Footbridge,
                            Vec3::new(p.x, c.level, p.y),
                            Vec2::new(-dir.y, dir.x),
                            Vec3::new(c.w + 8.0, 3.2, 5.0),
                            0.0,
                        );
                    }
                    if boat <= 0.0 && self.town.style != Style::Modern {
                        boat = self.rng.range(25.0, 60.0);
                        let side = if self.rng.f32() < 0.5 { 1.0 } else { -1.0 };
                        let at = p + Vec2::new(-dir.y, dir.x) * side * (c.w * 0.5 - 2.6);
                        self.push(
                            Kind::Houseboat,
                            Vec3::new(at.x, c.level, at.y),
                            dir,
                            Vec3::new(14.0, 2.8, 4.2),
                            0.0,
                        );
                    }
                    t += 10.0;
                    along -= 10.0;
                    boat -= 10.0;
                }
            }
        }
    }

    /// A building placed as is, with no ground checks. For landmarks and boats.
    fn push(&mut self, kind: Kind, pos: Vec3, axis: Vec2, size: Vec3, roof: f32) {
        self.take(Foot {
            c: Vec2::new(pos.x, pos.z),
            axis,
            half: Vec2::new(size.x, size.z) * 0.5,
        });
        let seed = self.rng.next_u64() as u32;
        self.out.push(Building {
            id: 0,
            kind,
            style: self.town.style,
            tech: self.town.tech,
            pos,
            axis,
            size,
            roof,
            stilts: 0.0,
            seed,
            site: None,
            text: "",
        });
    }

    /// One standout building near the middle of each town.
    fn centerpiece(&mut self) {
        let (kind, size, roof) = match (self.town.style, self.town.tech) {
            (Style::Canal, _) | (Style::Plain, Tech::Mid) | (Style::Hill, _) => {
                (Kind::Church, Vec3::new(22.0, 12.0, 12.0), 6.0)
            }
            (Style::Stone, _) => (Kind::Hall, Vec3::new(34.0, 16.0, 24.0), 9.0),
            (Style::Seafaring, _) => (Kind::Hall, Vec3::new(30.0, 12.0, 18.0), 5.0),
            (Style::Cavern, _) => (Kind::Hall, Vec3::new(26.0, 10.0, 18.0), 6.0),
            _ => return,
        };
        let u = self.grain();
        for k in 0..30 {
            let off =
                Vec2::new(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * (k as f32 * 8.0);
            if self.try_place(kind, self.center + off, u, size, roof, 6.0) {
                break;
            }
        }
        if self.town.id == "home/home-village" {
            for k in 0..30 {
                let off = Vec2::new(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0))
                    * (60.0 + k as f32 * 6.0);
                if self.try_place(
                    Kind::Windmill,
                    self.center + off,
                    u,
                    Vec3::new(8.0, 14.0, 8.0),
                    0.0,
                    6.0,
                ) {
                    break;
                }
            }
        }
    }

    /// Lamps along the lanes: lanterns in cave towns and simple villages, street lamps
    /// everywhere else.
    fn lanterns(&mut self) {
        let lantern = matches!(self.town.style, Style::Cavern | Style::Stilts);
        let (every, lamp) = if lantern {
            (2, Vec3::new(0.6, 4.0, 0.6))
        } else if self.town.tech >= Tech::Mid {
            (3, Vec3::new(0.5, 5.5, 0.5))
        } else {
            (3, Vec3::new(0.6, 4.2, 0.6))
        };
        let homes: Vec<(Vec2, Vec2, f32)> = self
            .out
            .iter()
            .filter(|b| matches!(b.kind, Kind::House | Kind::Dome | Kind::Hut))
            .map(|b| {
                (
                    Vec2::new(b.pos.x, b.pos.z),
                    b.front(),
                    b.size.z.max(b.size.x),
                )
            })
            .collect();
        for (i, (c, f, d)) in homes.into_iter().enumerate() {
            if i % every != 0 {
                continue;
            }
            let at = c + f * (d * 0.5 + 3.0) + Vec2::new(f.y, -f.x) * 3.0;
            let g = self.ground(at);
            if self
                .terrain
                .water(at.x, at.y, g)
                .is_none_or(|w| w - g > 15.0)
            {
                self.push(Kind::Lantern, Vec3::new(at.x, g, at.y), f, lamp, 0.0);
            }
        }
    }
}

/// Where the water is from a spot on the shore: the direction with the lowest ground
/// nearby, and the distance to the waterline that way.
fn seaward(terrain: &Terrain, at: Vec2) -> (Vec2, f32) {
    let mut best = (Vec2::X, f32::MAX);
    // Sixteen directions by halving angles, without trig.
    let mut dirs = vec![Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y];
    for _ in 0..2 {
        let mut more = Vec::new();
        for i in 0..dirs.len() {
            let (a, b) = (dirs[i], dirs[(i + 1) % dirs.len()]);
            more.push(a);
            more.push((a + b).normalize());
        }
        dirs = more;
    }
    for d in dirs {
        let h = terrain.height(at.x + d.x * 60.0, at.y + d.y * 60.0)
            + terrain.height(at.x + d.x * 120.0, at.y + d.y * 120.0);
        if h < best.1 {
            best = (d, h);
        }
    }
    let dir = best.0;
    let mut t = 0.0;
    while t < 300.0
        && terrain.height(at.x + dir.x * t, at.y + dir.y * t) > terrain.def.sea_level - 1.0
    {
        t += 4.0;
    }
    (dir, t)
}

/// Landmarks at the sites in the world files.
fn sites(terrain: &Terrain, rng: &mut Rng, out: &mut Vec<Building>) {
    let def = terrain.def;
    let style_at = |p: Vec2| {
        def.towns
            .iter()
            .find(|t| inside(t.pts, p))
            .map_or((Style::Plain, Tech::Mid), |t| (t.style, t.tech))
    };
    let mut push = |out: &mut Vec<Building>,
                    kind: Kind,
                    pos: Vec3,
                    axis: Vec2,
                    size: Vec3,
                    roof: f32,
                    stilts: f32,
                    rng: &mut Rng| {
        let (style, tech) = style_at(Vec2::new(pos.x, pos.z));
        out.push(Building {
            id: 0,
            kind,
            style,
            tech,
            pos,
            axis,
            size,
            roof,
            stilts,
            seed: rng.next_u64() as u32,
            site: None,
            text: "",
        });
    };
    let sea = def.sea_level;
    for m in def.marks.iter().filter(|m| m.kind.starts_with("site:")) {
        let at = Vec2::new(m.pos[0], m.pos[2]);
        let g = terrain.height(at.x, at.y);
        let kind = &m.kind[5..];
        let (dir, shore) = seaward(terrain, at);
        let side = Vec2::new(-dir.y, dir.x);
        let on_shore = at + dir * (shore - 6.0).max(0.0);
        let water_at = |d: f32| at + dir * (shore + d);
        match kind {
            "lighthouse" => {
                push(
                    out,
                    Kind::Lighthouse,
                    Vec3::new(at.x, g - 0.5, at.y),
                    Vec2::X,
                    Vec3::new(9.0, 30.0, 9.0),
                    0.0,
                    0.0,
                    rng,
                );
                out.last_mut().expect("just pushed").site = Some(m.id);
            }
            "tower" => {
                push(
                    out,
                    Kind::Mast,
                    Vec3::new(at.x, g - 0.5, at.y),
                    Vec2::X,
                    Vec3::new(10.0, 190.0, 10.0),
                    0.0,
                    0.0,
                    rng,
                );
                out.last_mut().expect("just pushed").site = Some(m.id);
            }
            "docks" | "ferry" | "marina" | "seaplane" | "shipyard" => {
                let len = match kind {
                    "marina" => 70.0,
                    "docks" => 80.0,
                    _ => 50.0,
                };
                // A pier straight out from the shore, with boats along it.
                let mid = water_at(len * 0.5 - 8.0);
                let pier = Vec3::new(mid.x, sea, mid.y);
                if kind != "shipyard" {
                    push(
                        out,
                        Kind::Pier,
                        pier,
                        dir,
                        Vec3::new(len, 1.8, 7.0),
                        0.0,
                        6.0,
                        rng,
                    );
                }
                let berths: &[(Kind, f32, Vec3)] = match kind {
                    "marina" => &[
                        (Kind::Sailboat, 12.0, Vec3::new(11.0, 2.0, 4.0)),
                        (Kind::Sailboat, 26.0, Vec3::new(12.0, 2.0, 4.2)),
                        (Kind::Sailboat, 40.0, Vec3::new(10.0, 2.0, 3.8)),
                        (Kind::Sailboat, 54.0, Vec3::new(13.0, 2.0, 4.4)),
                    ],
                    "docks" => &[
                        (Kind::Boat, 20.0, Vec3::new(18.0, 3.0, 6.0)),
                        (Kind::Boat, 48.0, Vec3::new(22.0, 3.5, 7.0)),
                    ],
                    "ferry" => &[(Kind::Ferry, 30.0, Vec3::new(42.0, 6.0, 12.0))],
                    "seaplane" => &[(Kind::Seaplane, 24.0, Vec3::new(12.0, 3.5, 14.0))],
                    _ => &[],
                };
                for (k, (bk, d, size)) in berths.iter().enumerate() {
                    let s = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let p = water_at(*d - 8.0) + side * s * (3.5 + size.z * 0.5 + 1.0);
                    push(
                        out,
                        *bk,
                        Vec3::new(p.x, sea, p.y),
                        dir,
                        *size,
                        0.0,
                        0.0,
                        rng,
                    );
                }
                match kind {
                    "docks" | "ferry" => {
                        let back = on_shore - dir * 18.0;
                        let gb = terrain.height(back.x, back.y);
                        let k = if kind == "ferry" {
                            Kind::Terminal
                        } else {
                            Kind::Warehouse
                        };
                        push(
                            out,
                            k,
                            Vec3::new(back.x, gb - 0.5, back.y),
                            side,
                            Vec3::new(26.0, 9.0, 14.0),
                            2.5,
                            0.0,
                            rng,
                        );
                    }
                    "shipyard" => {
                        let slip = on_shore + dir * 6.0;
                        let gs = terrain.height(slip.x, slip.y).max(sea);
                        push(
                            out,
                            Kind::Hull,
                            Vec3::new(slip.x, gs + 1.0, slip.y),
                            dir,
                            Vec3::new(40.0, 9.0, 11.0),
                            0.0,
                            0.0,
                            rng,
                        );
                        for s in [-1.0, 1.0] {
                            let c = slip + side * s * 16.0 - dir * 8.0;
                            let gc = terrain.height(c.x, c.y).max(sea);
                            push(
                                out,
                                Kind::Crane,
                                Vec3::new(c.x, gc, c.y),
                                -side * s,
                                Vec3::new(3.0, 34.0, 24.0),
                                0.0,
                                0.0,
                                rng,
                            );
                        }
                    }
                    _ => {}
                }
            }
            "airport" => airport(terrain, m, &mut push, out, rng),
            "wreck" => {
                push(
                    out,
                    Kind::Wreck,
                    Vec3::new(at.x, terrain.height(at.x, at.y).max(sea - 3.0), at.y),
                    Vec2::new(0.8, 0.6),
                    Vec3::new(30.0, 6.0, 9.0),
                    0.0,
                    0.0,
                    rng,
                );
            }
            _ => {}
        }
    }
}

type Push<'a> = dyn FnMut(&mut Vec<Building>, Kind, Vec3, Vec2, Vec3, f32, f32, &mut Rng) + 'a;

/// Terminal, tower, hangars and a parked airliner beside the runway nearest the site.
fn airport(terrain: &Terrain, m: &Mark, push: &mut Push, out: &mut Vec<Building>, rng: &mut Rng) {
    let at = Vec2::new(m.pos[0], m.pos[2]);
    let Some((ri, r)) = terrain
        .def
        .roads
        .iter()
        .enumerate()
        .filter(|(_, r)| r.runway)
        .min_by(|a, b| {
            let d = |r: &crate::terrain::Road| Vec2::new(r.pts[0][0], r.pts[0][1]).distance(at);
            d(a.1).total_cmp(&d(b.1))
        })
    else {
        return;
    };
    let a = Vec2::new(r.pts[0][0], r.pts[0][1]);
    let b = Vec2::new(r.pts[r.pts.len() - 1][0], r.pts[r.pts.len() - 1][1]);
    let dir = (b - a).normalize();
    let y = terrain.road_points(ri)[0].y;
    // The apron is on the side of the runway toward the site.
    let mid = (a + b) * 0.5;
    let mut side = Vec2::new(-dir.y, dir.x);
    if (at - mid).dot(side) < 0.0 {
        side = -side;
    }
    let off = r.w * 0.5;
    let place = |along: f32, out_by: f32| mid + dir * along + side * (off + out_by);
    let g = |p: Vec2| terrain.height(p.x, p.y).min(y + 2.0) - 0.4;
    let t = place(-60.0, 70.0);
    push(
        out,
        Kind::Terminal,
        Vec3::new(t.x, g(t), t.y),
        dir,
        Vec3::new(110.0, 12.0, 30.0),
        0.0,
        0.0,
        rng,
    );
    let c = place(90.0, 60.0);
    push(
        out,
        Kind::ControlTower,
        Vec3::new(c.x, g(c), c.y),
        dir,
        Vec3::new(9.0, 34.0, 9.0),
        0.0,
        0.0,
        rng,
    );
    for k in 0..2 {
        let h = place(170.0 + k as f32 * 55.0, 60.0);
        push(
            out,
            Kind::Hangar,
            Vec3::new(h.x, g(h), h.y),
            dir,
            Vec3::new(48.0, 16.0, 40.0),
            7.0,
            0.0,
            rng,
        );
    }
    let p = place(-60.0, 28.0);
    push(
        out,
        Kind::Airliner,
        Vec3::new(p.x, y, p.y),
        -side,
        Vec3::new(34.0, 4.0, 32.0),
        0.0,
        0.0,
        rng,
    );
}

/// Every building in the world, with ids in a fixed order.
pub fn place(terrain: &Terrain, seed: u64) -> Vec<Building> {
    let mut out = Vec::new();
    let mut rng = Rng::new(seed ^ 0x7031);
    sites(terrain, &mut rng, &mut out);
    for town in terrain.def.towns {
        // A town that runs out of room before it is filled is planned again with wider
        // streets, so it thins out evenly instead of stopping halfway.
        let mut spread = 1.0;
        let planned = loop {
            let mut p = Planner::new(terrain, town, seed);
            p.spread = spread;
            for b in &out {
                p.take(Foot {
                    c: Vec2::new(b.pos.x, b.pos.z),
                    axis: b.axis,
                    half: Vec2::new(b.size.x, b.size.z) * 0.5 + Vec2::splat(4.0),
                });
            }
            p.centerpiece();
            p.canal_life();
            let lines = p.lines();
            p.fronts(&lines);
            p.fill();
            p.lanterns();
            if p.out.len() < p.plan.max || spread > 2.5 {
                break p.out;
            }
            spread += 0.35;
        };
        out.extend(planned);
    }
    for (i, b) in out.iter_mut().enumerate() {
        b.id = i as u32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn towns_have_buildings() {
        let t = Terrain::default();
        let start = std::time::Instant::now();
        let all = place(&t, 1);
        eprintln!("{} buildings in {:?}", all.len(), start.elapsed());
        for town in t.def.towns {
            let n = all
                .iter()
                .filter(|b| inside(town.pts, Vec2::new(b.pos.x, b.pos.z)))
                .count();
            eprintln!("{} {n}", town.id);
            assert!(n >= 8, "{} has only {n}", town.id);
        }
        let again = place(&t, 1);
        assert!(
            all.iter()
                .zip(&again)
                .all(|(a, b)| a.pos == b.pos && a.size == b.size)
        );
    }
}
