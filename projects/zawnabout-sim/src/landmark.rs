//! Landmarks placed one by one in the world files: the odd, homely things real places
//! have. Standing stones, wind turbines, campsites, picnic spots, phone boxes, beach
//! umbrellas, scarecrows, a radio telescope, a drive-in screen and big hillside letters,
//! and more: a fairground wheel, a rope swing, a hedge maze, a sandcastle, a snowman,
//! garden gnomes, a treehouse, a wishing well, a circus, a bandstand, and the big places
//! towns have: a stadium, a waterpark, a theme park, a zoo, a harbor statue, a mall,
//! grocery stores, parks with playgrounds and railway stations.
//! Each is a list of simple pieces, so the mesh and the colliders always agree.

use glam::{Vec2, Vec3};

use crate::shape::hash;
use crate::town::Kind;

/// What a piece is made of, for its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mat {
    Stone,
    Wood,
    /// A bright fabric, picked by the number.
    Canvas(u8),
    Metal,
    White,
    Red,
    /// Panes that glow at night.
    Glass,
    Hay,
    Dark,
    /// A movie screen, softly lit at night.
    Screen,
    Hedge,
    Sand,
    Snow,
    Leaf,
    /// Warm spring water, softly glowing.
    Water,
    Skin,
    /// A warm light: flames, lamps, a lit doorway. Glows at night.
    Lamp,
    /// Mown turf.
    Grass,
    /// Weathered green copper, for statues.
    Copper,
    Concrete,
    Asphalt,
    /// Bare cliff rock, for figures carved out of it.
    Rock,
    /// Pale sea-washed stone, for the Sea Doors.
    Pale,
    /// Dark volcanic stone. Round pieces of it are drawn as six-sided columns, the way
    /// basalt sets.
    Basalt,
    /// Not drawn: a solid stand-in for something the client draws itself, smoothly.
    Hidden,
}

#[derive(Clone, Copy, Debug)]
pub enum Form {
    /// A box standing on `at`: half width along x, full height, half depth along z, turned
    /// by `yaw` about its own center, with a ridge `roof` high on top.
    Box {
        at: Vec3,
        half: Vec3,
        yaw: f32,
        roof: f32,
    },
    /// An upright round piece from `at`, `h` tall, radius `r0` at the bottom and `r1` at
    /// the top.
    Round { at: Vec3, h: f32, r0: f32, r1: f32 },
    /// A ball centered on `at`.
    Ball { at: Vec3, r: f32 },
    /// One letter of a sign, painted flat: its middle at the bottom on `at`, 3 by 5 cells
    /// of `cell` (negative to read from the -z side), on the face `depth` toward the
    /// reader. Never solid.
    Letter {
        at: Vec3,
        cell: f32,
        depth: f32,
        ch: u8,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub form: Form,
    pub mat: Mat,
    /// Holds you up and stops you.
    pub solid: bool,
}

/// The kinds a world file can name, and what they become.
pub const KINDS: [(&str, Kind); 95] = [
    ("faces", Kind::Faces),
    ("warden", Kind::Warden),
    ("stones", Kind::Stones),
    ("turbine", Kind::Turbine),
    ("campsite", Kind::Campsite),
    ("picnic", Kind::Picnic),
    ("phonebox", Kind::PhoneBox),
    ("umbrellas", Kind::Umbrellas),
    ("scarecrow", Kind::Scarecrow),
    ("telescope", Kind::Telescope),
    ("drivein", Kind::DriveIn),
    ("sign", Kind::Sign),
    ("ferris", Kind::Ferris),
    ("swing", Kind::Swing),
    ("maze", Kind::Maze),
    ("sandcastle", Kind::Sandcastle),
    ("snowman", Kind::Snowman),
    ("bench", Kind::Bench),
    ("gnomes", Kind::Gnomes),
    ("beehives", Kind::Beehives),
    ("pumpkins", Kind::Pumpkins),
    ("busstop", Kind::BusStop),
    ("watertower", Kind::WaterTower),
    ("hotspring", Kind::HotSpring),
    ("chess", Kind::Chess),
    ("cairns", Kind::Cairns),
    ("treehouse", Kind::Treehouse),
    ("well", Kind::Well),
    ("lemonade", Kind::Lemonade),
    ("igloo", Kind::Igloo),
    ("laundry", Kind::Laundry),
    ("sundial", Kind::Sundial),
    ("circus", Kind::Circus),
    ("fountain", Kind::Fountain),
    ("postbox", Kind::Postbox),
    ("bandstand", Kind::Bandstand),
    ("stadium", Kind::Stadium),
    ("waterpark", Kind::Waterpark),
    ("themepark", Kind::ThemePark),
    ("zoo", Kind::Zoo),
    ("statue", Kind::Statue),
    ("mall", Kind::Mall),
    ("grocery", Kind::Grocery),
    ("park", Kind::Park),
    ("station", Kind::Station),
    ("metro", Kind::Metro),
    ("metro-gate", Kind::MetroGate),
    ("metro-stairs", Kind::MetroStairs),
    ("restaurant", Kind::Restaurant),
    ("drawbridge", Kind::Drawbridge),
    ("gallery", Kind::Gallery),
    ("skatepark", Kind::SkatePark),
    ("helipad", Kind::Helipad),
    ("clocktower", Kind::ClockTower),
    ("span-tower", Kind::SpanTower),
    ("market", Kind::Market),
    ("shroomfarm", Kind::ShroomFarm),
    ("mine", Kind::Mine),
    ("cafe", Kind::Cafe),
    ("library", Kind::Library),
    ("museum", Kind::Museum),
    ("cinema", Kind::Cinema),
    ("weir", Kind::Weir),
    ("penstock", Kind::Penstock),
    ("hydro", Kind::Hydro),
    ("racetrack", Kind::Racetrack),
    ("aquarium", Kind::Aquarium),
    ("university", Kind::University),
    ("police", Kind::Police),
    ("gasstation", Kind::GasStation),
    ("amphitheater", Kind::Amphitheater),
    ("solarfarm", Kind::SolarFarm),
    ("golf", Kind::Golf),
    ("observatory", Kind::Observatory),
    ("lookout", Kind::Lookout),
    ("surfshack", Kind::SurfShack),
    ("castaway", Kind::Castaway),
    ("balloonfield", Kind::BalloonField),
    ("seals", Kind::Seals),
    ("carnival", Kind::Carnival),
    ("cemetery", Kind::Cemetery),
    ("factory", Kind::Factory),
    ("powerstation", Kind::PowerStation),
    ("oilrig", Kind::OilRig),
    ("billboard", Kind::Billboard),
    ("dam", Kind::Dam),
    ("ropebridge", Kind::RopeBridge),
    ("hospital", Kind::Hospital),
    ("firestation", Kind::FireStation),
    ("school", Kind::School),
    ("townhall", Kind::TownHall),
    ("farm", Kind::Farm),
    ("port", Kind::Port),
    ("seadoors", Kind::SeaDoors),
    ("basaltgate", Kind::BasaltGate),
];

/// Where a fairground wheel turns, in the landmark's frame, and its radius.
pub const FERRIS_HUB: Vec3 = Vec3::new(0.0, 21.0, 0.0);
pub const FERRIS_R: f32 = 17.0;
/// How fast a fairground wheel turns (rad/s), and how many cars hang from it. Wheel `k`
/// (in world file order) is at angle `FERRIS_SPIN * time + k`.
pub const FERRIS_SPIN: f32 = 0.18;
pub const FERRIS_CARS: u32 = 10;
/// A ferris car's size (width along the wheel's axle, height, depth), and how far its
/// floor hangs below the pin.
pub const FERRIS_CAR: Vec3 = Vec3::new(2.4, 2.3, 2.0);
pub const FERRIS_DROP: f32 = 3.0;
/// Where a rope swing hangs from its branch, and how long its ropes are.
pub const SWING_PIVOT: Vec3 = Vec3::new(9.0, 8.0, 0.0);
pub const SWING_ROPE: f32 = 6.6;
/// A town park's benches, in its frame.
pub const PARK_BENCHES: [Vec3; 2] = [Vec3::new(-8.0, 0.0, 0.9), Vec3::new(8.0, 0.0, 5.8)];

/// Where a theme park's carousel turns.
pub const CAROUSEL: Vec3 = Vec3::new(40.0, 0.0, 25.0);

/// The roller coaster track round a theme park, `t` from 0 to 1 once round: a long oval
/// with hills and dips, in the park's frame.
pub fn coaster(t: f32) -> Vec3 {
    let a = t * std::f32::consts::TAU;
    let hill = (0.5 + 0.5 * (3.0 * a).sin()).powi(2);
    Vec3::new(
        a.cos() * 68.0,
        4.0 + hill * 20.0 + (a.sin() * 0.5 + 0.5) * 6.0,
        a.sin() * 42.0 + (2.0 * a).sin() * 10.0,
    )
}

/// The coaster track sampled by distance along it, with the time the train reaches each
/// sample, so the train climbs the lift hill slowly and races down the drops.
#[derive(Clone, Debug)]
pub struct Coaster {
    /// Track points in the park's frame, evenly spaced along the track.
    pub pts: Vec<Vec3>,
    /// Spacing between points (m).
    pub step: f32,
    /// When the front of the train passes each point, from 0 to `lap`.
    pub at: Vec<f32>,
    pub lap: f32,
}

/// Points along the coaster track.
pub const RIDE_PTS: usize = 600;
/// Cars in a coaster train, and how far apart they ride (m).
pub const COASTER_CARS: usize = 5;
pub const CAR_GAP: f32 = 2.7;
/// Speed up the lift hill, and how long the brake run before it is (m/s, m).
pub const LIFT_SPEED: f32 = 3.5;
pub const BRAKE_RUN: f32 = 30.0;
const G: f32 = 9.8;

impl Default for Coaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Coaster {
    pub fn new() -> Self {
        // Fine samples first, to measure the track.
        let fine = 4000;
        let raw: Vec<Vec3> = (0..=fine)
            .map(|k| coaster(k as f32 / fine as f32))
            .collect();
        let mut len = vec![0.0f32];
        for w in raw.windows(2) {
            len.push(len.last().unwrap() + w[0].distance(w[1]));
        }
        let total = *len.last().unwrap();
        let step = total / RIDE_PTS as f32;
        let mut pts = Vec::with_capacity(RIDE_PTS);
        let mut j = 0;
        for k in 0..RIDE_PTS {
            let s = k as f32 * step;
            while len[j + 1] < s {
                j += 1;
            }
            let u = (s - len[j]) / (len[j + 1] - len[j]).max(1e-6);
            pts.push(raw[j].lerp(raw[j + 1], u));
        }
        // The lift hill climbs from the lowest point before the top to the top.
        let top = (0..RIDE_PTS)
            .max_by(|&a, &b| pts[a].y.total_cmp(&pts[b].y))
            .unwrap();
        let mut foot = top;
        for k in 1..RIDE_PTS {
            let i = (top + RIDE_PTS - k) % RIDE_PTS;
            if pts[i].y <= pts[(i + RIDE_PTS - 1) % RIDE_PTS].y {
                foot = i;
                break;
            }
        }
        let peak = pts[top].y;
        let speed = |i: usize| {
            let on_lift = (i + RIDE_PTS - foot) % RIDE_PTS <= (top + RIDE_PTS - foot) % RIDE_PTS;
            if on_lift {
                return LIFT_SPEED;
            }
            // Free running from the top, losing a little to friction along the way, then
            // easing down on the brakes just before the lift.
            let along = ((i + RIDE_PTS - top) % RIDE_PTS) as f32 * step;
            let free = (LIFT_SPEED * LIFT_SPEED + 2.0 * G * (peak - pts[i].y) - along * 0.02)
                .max(1.0)
                .sqrt();
            let before = ((foot + RIDE_PTS - i) % RIDE_PTS) as f32 * step;
            if before < BRAKE_RUN {
                LIFT_SPEED + (free - LIFT_SPEED) * (before / BRAKE_RUN)
            } else {
                free
            }
        };
        let mut at = Vec::with_capacity(RIDE_PTS);
        let mut t = 0.0;
        for i in 0..RIDE_PTS {
            at.push(t);
            t += step * 2.0 / (speed(i) + speed((i + 1) % RIDE_PTS));
        }
        Self {
            pts,
            step,
            at,
            lap: t,
        }
    }

    /// Distance along the track of the train's front at `time`.
    pub fn front(&self, time: f32) -> f32 {
        let t = time.rem_euclid(self.lap);
        let i = self.at.partition_point(|&a| a <= t).max(1) - 1;
        let next = if i + 1 < RIDE_PTS {
            self.at[i + 1]
        } else {
            self.lap
        };
        (i as f32 + (t - self.at[i]) / (next - self.at[i]).max(1e-6)) * self.step
    }

    /// The track point `s` meters along.
    pub fn point(&self, s: f32) -> Vec3 {
        let f = (s / self.step).rem_euclid(RIDE_PTS as f32);
        let i = f as usize % RIDE_PTS;
        self.pts[i].lerp(self.pts[(i + 1) % RIDE_PTS], f.fract())
    }
}

/// A theme park's drop tower, and how tall it stands.
pub const DROP: Vec3 = Vec3::new(-55.0, 0.0, 35.0);
pub const DROP_TALL: f32 = 60.0;
/// How long a drop tower's ride takes, start to start (s).
pub const DROP_CYCLE: f32 = 42.0;

/// How high a drop tower's ring is `t` seconds into its ride: loading at the bottom, a
/// slow ride up, a pause at the top, a free fall and the brakes.
pub fn drop_height(t: f32) -> f32 {
    const G: f32 = 9.8;
    let t = t.rem_euclid(DROP_CYCLE);
    let (bottom, top) = (1.3, 54.0);
    let fall = 2.6;
    let fallen = 0.5 * G * fall * fall;
    let v = G * fall;
    let brake = v * v / (2.0 * (top - fallen - bottom));
    let brake_time = v / brake;
    if t < 6.0 {
        bottom
    } else if t < 24.0 {
        let u = (t - 6.0) / 18.0;
        bottom + (top - bottom) * (u * u * (3.0 - 2.0 * u))
    } else if t < 28.0 {
        top
    } else if t < 28.0 + fall {
        let u = t - 28.0;
        top - 0.5 * G * u * u
    } else if t < 28.0 + fall + brake_time {
        let u = t - 28.0 - fall;
        top - fallen - (v * u - 0.5 * brake * u * u)
    } else {
        bottom
    }
}

/// A water park's lazy river: an oval round the big pool, its middle and radii.
pub const LAZY_RIVER: (Vec2, Vec2) = (Vec2::new(-25.0, -10.0), Vec2::new(30.0, 24.0));
/// The top of a water park's slide tower.
pub const SLIDE_TOP: Vec3 = Vec3::new(48.0, 17.0, -15.0);
/// How many slides spiral down from the tower.
pub const SLIDES: u32 = 3;

/// Where water slide `s` runs, `u` from 0 at the top of the tower to 1 at the pool. Each
/// spirals out round the tower and ends over the wave pool beside it.
pub fn slide(s: u32, u: f32) -> Vec3 {
    let turns = 1.2 + s as f32 * 0.4;
    let a =
        std::f32::consts::PI + (s as f32 - 1.0) * 0.5 - (1.0 - u) * turns * std::f32::consts::TAU;
    let r = 5.0 + u * (6.0 + s as f32 * 2.0);
    Vec3::new(
        SLIDE_TOP.x + a.cos() * r,
        SLIDE_TOP.y * (1.0 - u),
        SLIDE_TOP.z + a.sin() * r,
    )
}

/// A zoo's pens in its frame: middle, radius, and which animal lives there (0 elephants,
/// 1 giraffes, 2 zebras, 3 lions, 4 penguins).
pub const ZOO_PENS: [(Vec2, f32, u8); 5] = [
    (Vec2::new(-50.0, -30.0), 22.0, 0),
    (Vec2::new(0.0, -42.0), 18.0, 1),
    (Vec2::new(52.0, -12.0), 20.0, 2),
    (Vec2::new(-48.0, 30.0), 18.0, 3),
    (Vec2::new(40.0, 36.0), 15.0, 4),
];

/// The zoo's walk round the middle: an oval that bends in wherever a pen comes close,
/// so it always keeps outside the fences.
pub fn zoo_path() -> Vec<Vec2> {
    (0..36)
        .map(|k| {
            let a = k as f32 / 36.0 * std::f32::consts::TAU;
            let oval = Vec2::new(a.cos() * 38.0, a.sin() * 28.0);
            let mut s = 1.0f32;
            while s > 0.2
                && ZOO_PENS
                    .iter()
                    .any(|(c, r, _)| (oval * s).distance(*c) < r + 4.5)
            {
                s -= 0.01;
            }
            oval * s
        })
        .collect()
}

/// A water park's splash playground: the pivot of the big bucket that fills and tips
/// over the deck, the deck's height, and how long the bucket takes to fill (s).
pub const SPLASH: Vec3 = Vec3::new(10.0, 8.6, 30.0);
pub const SPLASH_DECK: f32 = 3.2;
pub const SPLASH_FILL: f32 = 22.0;

/// How far a splash playground's bucket is tipped at `time` (radians), and whether it is
/// pouring: it fills, wobbling a little as it gets heavy, tips right over, empties and
/// swings back up.
pub fn splash_tip(time: f32) -> (f32, bool) {
    let t = time.rem_euclid(SPLASH_FILL + 4.0);
    let ease = |u: f32| u * u * (3.0 - 2.0 * u);
    if t < SPLASH_FILL {
        let full = t / SPLASH_FILL;
        ((t * 2.3).sin() * 0.06 * full * full, false)
    } else if t < SPLASH_FILL + 0.8 {
        (ease((t - SPLASH_FILL) / 0.8) * 2.3, false)
    } else if t < SPLASH_FILL + 2.6 {
        (2.3, true)
    } else {
        ((1.0 - ease((t - SPLASH_FILL - 2.6) / 1.4)) * 2.3, false)
    }
}

/// A seal colony's rocks, standing up out of the sea: middle, radius of the flat top,
/// and the top's height above the water.
pub const SEAL_ROCKS: [(Vec2, f32, f32); 4] = [
    (Vec2::new(0.0, 0.0), 4.5, 1.2),
    (Vec2::new(9.0, 4.0), 3.0, 0.8),
    (Vec2::new(-7.0, 6.0), 2.6, 1.6),
    (Vec2::new(3.0, -9.0), 2.8, 0.6),
];

/// A carnival's chair swing (the foot of its tower), the top of the tower where the
/// spinning crown turns, and how far out its chairs hang (m).
pub const CHAIR_SWING: Vec3 = Vec3::new(-22.0, 0.0, 4.0);
pub const SWING_TOP: f32 = 9.3;
pub const SWING_REACH: f32 = 4.6;
/// A carnival's bumper car floor: middle and half size, in its frame.
pub const BUMPER: (Vec2, Vec2) = (Vec2::new(18.0, -6.0), Vec2::new(10.0, 7.0));
/// Where a carnival's helter skelter stands.
pub const HELTER: Vec3 = Vec3::new(10.0, 0.0, 18.0);

/// How far round a chair swing has turned at `time` (radians) and how fast it turns
/// (radians a second): it winds up, flies round, slows and stops to let riders off.
pub fn chair_swing(time: f32) -> (f32, f32) {
    const LAP: f32 = 50.0;
    const TOP: f32 = 1.4;
    let speed = |t: f32| {
        if t < 8.0 {
            TOP * t / 8.0
        } else if t < 38.0 {
            TOP
        } else if t < 46.0 {
            TOP * (46.0 - t) / 8.0
        } else {
            0.0
        }
    };
    // Each full ride turns the same way round: its whole sweep, then this one so far.
    let sweep = TOP * (4.0 + 30.0 + 4.0);
    let t = time.rem_euclid(LAP);
    let so_far = if t < 8.0 {
        TOP * t * t / 16.0
    } else if t < 38.0 {
        TOP * 4.0 + TOP * (t - 8.0)
    } else if t < 46.0 {
        let left = 46.0 - t;
        sweep - TOP * left * left / 16.0
    } else {
        sweep
    };
    ((time / LAP).floor() * sweep + so_far, speed(t))
}

/// The top of a factory's chimney, where its steam comes out.
pub const FACTORY_STACK: Vec3 = Vec3::new(22.0, 34.0, -12.0);
/// The tops of a power station's cooling towers.
pub const POWER_STACKS: [Vec3; 2] = [Vec3::new(-24.0, 34.0, -14.0), Vec3::new(12.0, 34.0, -16.0)];
/// An oil rig's deck height over the sea, and the middle of its helipad's top.
pub const RIG_DECK: f32 = 20.0;
pub const RIG_PAD: Vec3 = Vec3::new(16.0, 21.7, 16.0);

/// A dam's height from its foot to its crest, and where its spillway water leaves the
/// crest and where it lands, in its frame (downstream is +z).
pub const DAM_TALL: f32 = 112.0;
pub const DAM_BACK: f32 = 20.0;
pub const DAM_SPILL: (Vec3, Vec3) = (
    Vec3::new(0.0, 111.0, 2.5 - DAM_BACK),
    Vec3::new(0.0, 14.0, 34.0 - DAM_BACK),
);

/// Water a landmark pours down: where it starts and where it lands, in its frame.
pub fn spills(kind: Kind) -> Vec<(Vec3, Vec3)> {
    match kind {
        Kind::Dam => vec![DAM_SPILL],
        _ => Vec::new(),
    }
}

/// Where a landmark lets off steam, in its frame, and how wide the plume starts (m).
pub fn stacks(kind: Kind) -> Vec<(Vec3, f32)> {
    match kind {
        Kind::Factory => vec![(FACTORY_STACK, 1.4)],
        Kind::PowerStation => POWER_STACKS.iter().map(|t| (*t, 9.0)).collect(),
        _ => Vec::new(),
    }
}

/// Where a theme park's big wheel stands in its frame (its own landmark, beside the park).
pub const THEMEPARK_FERRIS: Vec2 = Vec2::new(60.0, -52.0);

/// Where balloons come down on a balloon field, in its frame.
pub const BALLOON_SPOTS: [Vec2; 2] = [Vec2::new(-20.0, -10.0), Vec2::new(22.0, 2.0)];

/// Where a hot spring's water steams.
pub const SPRING: Vec3 = Vec3::new(0.0, 0.2, 0.0);
/// Where a turbine's rotor turns, in the landmark's frame, and its blade length.
pub const ROTOR: Vec3 = Vec3::new(2.4, 41.0, 0.0);
pub const BLADE: f32 = 17.0;

/// Size of one block of a sign's letters (m).
pub const SIGN_CELL: f32 = 2.4;

/// Where a campsite's fire burns, in the landmark's frame.
pub const CAMPFIRE: Vec3 = Vec3::ZERO;

fn bx(at: Vec3, half: Vec3, mat: Mat) -> Piece {
    Piece {
        form: Form::Box {
            at,
            half,
            yaw: 0.0,
            roof: 0.0,
        },
        mat,
        solid: true,
    }
}

fn round(at: Vec3, h: f32, r0: f32, r1: f32, mat: Mat) -> Piece {
    Piece {
        form: Form::Round { at, h, r0, r1 },
        mat,
        solid: true,
    }
}

fn ball(at: Vec3, r: f32, mat: Mat) -> Piece {
    Piece {
        form: Form::Ball { at, r },
        mat,
        solid: true,
    }
}

fn turned(at: Vec3, half: Vec3, yaw: f32, mat: Mat) -> Piece {
    Piece {
        form: Form::Box {
            at,
            half,
            yaw,
            roof: 0.0,
        },
        mat,
        solid: true,
    }
}

fn soft(mut p: Piece) -> Piece {
    p.solid = false;
    p
}

/// Four faces carved side by side out of a sea cliff, looking out to sea (+z) over the
/// water at its foot: a dragon, an owl, a bear and a bearded elder, each about 40 m tall
/// (grown a third, see `pieces`). The cliff's face runs along z = -10, so the heads sink
/// back into the living rock. The frame's origin is at sea level by the cliff's foot.
fn faces(out: &mut Vec<Piece>) {
    let (y0, c) = (30.0f32, -7.5f32);
    for (k, x) in [-57.0f32, -19.0, 19.0, 57.0].into_iter().enumerate() {
        let at = |dx: f32, y: f32, dz: f32| Vec3::new(x + dx, y0 + y, c + dz);
        if k == 0 || k == 3 {
            out.push(bx(
                at(0.0, -16.0, -4.0),
                Vec3::new(10.0, 18.0, 8.0),
                Mat::Rock,
            ));
        }
        match k {
            0 => {
                // The dragon: a long snout, a heavy brow, horns swept back.
                out.push(bx(
                    at(0.0, 0.0, -2.0),
                    Vec3::new(11.0, 30.0, 10.0),
                    Mat::Rock,
                ));
                out.push(bx(at(0.0, 4.0, 10.0), Vec3::new(7.0, 12.0, 8.0), Mat::Rock));
                for s in [-1.0f32, 1.0] {
                    out.push(soft(ball(at(s * 3.0, 13.0, 18.2), 1.2, Mat::Dark)));
                    out.push(soft(ball(at(s * 7.0, 20.0, 8.2), 2.0, Mat::Dark)));
                    out.push(bx(
                        at(s * 7.0, 22.0, 8.0),
                        Vec3::new(3.5, 2.0, 3.0),
                        Mat::Rock,
                    ));
                    out.push(turned(
                        at(s * 6.0, 28.0, -6.0),
                        Vec3::new(2.4, 14.0, 2.4),
                        s * 0.4,
                        Mat::Rock,
                    ));
                }
            }
            1 => {
                // The owl: a round face, great eyes and tufted ears.
                out.push(ball(at(0.0, 18.0, 0.0), 15.0, Mat::Rock));
                for s in [-1.0f32, 1.0] {
                    out.push(ball(at(s * 6.5, 20.0, 10.0), 5.0, Mat::Rock));
                    out.push(soft(ball(at(s * 6.5, 20.0, 14.2), 2.2, Mat::Dark)));
                    out.push(round(at(s * 9.0, 29.0, 0.0), 10.0, 4.0, 0.5, Mat::Rock));
                }
                out.push(turned(
                    at(0.0, 8.0, 13.0),
                    Vec3::new(2.0, 8.0, 3.0),
                    0.0,
                    Mat::Rock,
                ));
            }
            2 => {
                // The bear: round ears, a broad snout and a dark nose.
                out.push(ball(at(0.0, 17.0, 0.0), 15.0, Mat::Rock));
                out.push(ball(at(0.0, 11.0, 12.0), 7.0, Mat::Rock));
                out.push(soft(ball(at(0.0, 13.0, 18.6), 2.2, Mat::Dark)));
                for s in [-1.0f32, 1.0] {
                    out.push(ball(at(s * 11.0, 31.0, -2.0), 5.0, Mat::Rock));
                    out.push(soft(ball(at(s * 6.0, 22.0, 12.6), 1.6, Mat::Dark)));
                }
            }
            _ => {
                // The elder: a heavy brow, a long nose and a great beard.
                out.push(bx(
                    at(0.0, 4.0, 0.0),
                    Vec3::new(11.0, 30.0, 10.0),
                    Mat::Rock,
                ));
                out.push(bx(
                    at(0.0, 30.0, -1.0),
                    Vec3::new(12.0, 6.0, 11.0),
                    Mat::Rock,
                ));
                out.push(bx(
                    at(0.0, 22.0, 10.0),
                    Vec3::new(10.0, 2.5, 1.5),
                    Mat::Rock,
                ));
                out.push(bx(at(0.0, 12.0, 11.0), Vec3::new(2.5, 9.0, 3.0), Mat::Rock));
                out.push(bx(at(0.0, 9.0, 12.4), Vec3::new(6.0, 1.6, 1.5), Mat::Rock));
                out.push(bx(
                    at(0.0, -12.0, 3.0),
                    Vec3::new(10.0, 18.0, 8.0),
                    Mat::Rock,
                ));
                for s in [-1.0f32, 1.0] {
                    out.push(soft(ball(at(s * 5.0, 20.0, 10.2), 1.4, Mat::Dark)));
                }
            }
        }
    }
}

/// A giant robed guardian carved out of a sea stack's face, facing out (+z): a mariner
/// with a great oar upright beside them, or a lamplighter whose staff carries a lantern
/// that burns at night. The frame's origin is at sea level on the face, so the back of
/// the figure sinks into the stack (grown, see `pieces`).
fn warden(out: &mut Vec<Piece>, lamp: bool) {
    out.push(round(Vec3::Y * -12.0, 92.0, 13.0, 7.0, Mat::Rock));
    // Deep folds down the front of the robe, and a belt.
    for x in [-6.5f32, 0.0, 6.5] {
        out.push(bx(
            Vec3::new(x, -12.0, 8.5 - x.abs() * 0.3),
            Vec3::new(1.6, 70.0 - x.abs() * 2.0, 4.0),
            Mat::Rock,
        ));
    }
    out.push(round(Vec3::Y * 44.0, 3.0, 10.4, 10.2, Mat::Rock));
    out.push(bx(
        Vec3::new(0.0, 74.0, 0.0),
        Vec3::new(10.0, 8.0, 6.0),
        Mat::Rock,
    ));
    out.push(round(Vec3::new(0.0, 82.0, -0.8), 14.0, 7.5, 2.0, Mat::Rock));
    out.push(ball(Vec3::new(0.0, 88.0, 1.4), 5.0, Mat::Rock));
    // The near arm down to the staff, the other hand at the chest.
    out.push(turned(
        Vec3::new(10.0, 56.0, 2.0),
        Vec3::new(2.6, 20.0, 2.6),
        0.0,
        Mat::Rock,
    ));
    out.push(bx(
        Vec3::new(-4.0, 64.0, 6.4),
        Vec3::new(3.2, 5.0, 2.6),
        Mat::Rock,
    ));
    let x = 13.5;
    if lamp {
        out.push(round(Vec3::new(x, 0.0, 3.0), 108.0, 1.5, 1.3, Mat::Rock));
        out.push(bx(
            Vec3::new(x, 108.0, 3.0),
            Vec3::new(3.6, 0.8, 3.6),
            Mat::Rock,
        ));
        out.push(soft(bx(
            Vec3::new(x, 108.8, 3.0),
            Vec3::new(2.6, 6.0, 2.6),
            Mat::Lamp,
        )));
        out.push(round(Vec3::new(x, 114.8, 3.0), 4.0, 3.8, 0.3, Mat::Rock));
    } else {
        out.push(round(Vec3::new(x, 0.0, 3.0), 112.0, 1.4, 1.2, Mat::Rock));
        out.push(bx(
            Vec3::new(x, 4.0, 3.0),
            Vec3::new(4.0, 30.0, 1.0),
            Mat::Rock,
        ));
    }
}

/// A great square spiral cut in steps out of a cliff, facing out (+z), with three round
/// bosses in a row beneath: no one remembers what it means. The cliff's face is at z = 0,
/// so each step runs back into the rock. Its middle is `SPIRAL_MID` up from the origin.
fn spiral(out: &mut Vec<Piece>) {
    const ROWS: [&str; 11] = [
        "###########",
        "..........#",
        "#########.#",
        "#.......#.#",
        "#.#####.#.#",
        "#.#...#.#.#",
        "#.#.###.#.#",
        "#.#.....#.#",
        "#.#######.#",
        "#.........#",
        "###########",
    ];
    let cell = 6.0f32;
    let mid = SPIRAL_MID;
    for (r, row) in ROWS.iter().enumerate() {
        let y = mid + (5.0 - r as f32) * cell - cell * 0.5;
        // Each run of cells in a row is one block.
        let cells: Vec<bool> = row.chars().map(|c| c == '#').collect();
        let mut i = 0;
        while i < cells.len() {
            if !cells[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < cells.len() && cells[i] {
                i += 1;
            }
            let x = ((start + i) as f32 * 0.5 - 5.5) * cell;
            out.push(bx(
                Vec3::new(x, y, -3.0),
                Vec3::new((i - start) as f32 * cell * 0.5, cell, 9.0),
                Mat::Stone,
            ));
        }
    }
    for x in [-16.0f32, 0.0, 16.0] {
        out.push(ball(Vec3::new(x * 1.3, mid - 45.0, 1.0), 5.5, Mat::Stone));
    }
}

/// The islands' old serpent god, carved into a niche cut in a sea cliff, facing out
/// (+z): a great horned head with a pearl in its jaws, its coils curling round it. The
/// cliff's face is at z = 0 and the niche goes back some 20 m, so all of it stays inside
/// the face, carved in rather than stuck on. Its eyes are `SERPENT_MID` up from the
/// origin (grown a third, see `pieces`).
fn serpent(out: &mut Vec<Piece>) {
    let h = SERPENT_MID;
    let k = 1.35;
    let at = |x: f32, y: f32, z: f32| Vec3::new(x * k, h + y * k, z * k);
    // The head: a broad skull, a long snout and a heavy jaw.
    out.push(ball(at(0.0, 0.0, -16.0), 10.0 * k, Mat::Rock));
    out.push(bx(
        at(0.0, -7.0, -9.0),
        Vec3::new(5.5, 7.0, 7.0) * k,
        Mat::Rock,
    ));
    out.push(bx(
        at(0.0, -12.0, -10.0),
        Vec3::new(4.8, 3.5, 6.0) * k,
        Mat::Rock,
    ));
    // Brows over deep eyes, nostrils at the snout's end.
    for s in [-1.0f32, 1.0] {
        out.push(bx(
            at(s * 5.5, 2.5, -10.0),
            Vec3::new(3.2, 1.6, 2.6) * k,
            Mat::Rock,
        ));
        out.push(soft(ball(at(s * 5.2, 1.6, -7.6), 1.7 * k, Mat::Dark)));
        out.push(soft(ball(at(s * 2.4, -3.0, -2.2), 0.9 * k, Mat::Dark)));
        // Horns sweeping up, and fangs down from the upper jaw.
        out.push(round(
            at(s * 6.5, 6.0, -19.0),
            13.0 * k,
            2.2 * k,
            0.3,
            Mat::Rock,
        ));
        out.push(round(
            at(s * 3.0, -9.5, -3.4),
            2.6 * k,
            0.15,
            0.8 * k,
            Mat::Rock,
        ));
        // Whiskers trailing off the snout along the rock.
        out.push(turned(
            at(s * 7.5, -8.0, -6.0),
            Vec3::new(4.5, 0.6, 0.6) * k,
            s * 0.5,
            Mat::Rock,
        ));
        // A frill of fins down each side of the head.
        for f in 0..3 {
            out.push(turned(
                at(s * (10.0 + f as f32 * 1.5), -2.0 - f as f32 * 5.0, -17.0),
                Vec3::new(1.0, 5.0, 3.5) * k,
                s * 0.5,
                Mat::Rock,
            ));
        }
        // Its coils, curling out round the head and down into the ledge.
        for c in 0..20 {
            let t = c as f32 / 19.0;
            let a = 1.4 + t * 3.6;
            let r = 13.0 + t * 7.0;
            out.push(ball(
                at(s * a.sin() * r, a.cos() * r - 8.0, -19.0 + t * 2.0),
                (4.4 - t * 1.6) * k,
                Mat::Rock,
            ));
        }
    }
    // The pearl in its jaws, glowing at night for the ships.
    out.push(soft(ball(at(0.0, -9.5, -1.8), 1.8 * k, Mat::Lamp)));
}

/// How high the serpent's eyes are over its origin, in its own frame.
const SERPENT_MID: f32 = 73.0;

/// A small effigy of the serpent god in a town square: the god coiled on a round stone
/// plinth, head raised, a pearl in its jaws.
fn effigy(out: &mut Vec<Piece>) {
    out.push(round(Vec3::ZERO, 1.6, 3.2, 3.0, Mat::Stone));
    let mut last = Vec3::ZERO;
    for k in 0..18 {
        let t = k as f32 / 17.0;
        let a = t * 1.8 * std::f32::consts::TAU;
        let r = 1.8 - t * 0.9;
        let at = Vec3::new(a.cos() * r, 1.9 + t * 3.2, a.sin() * r);
        out.push(ball(at, 0.75 - t * 0.3, Mat::Copper));
        last = at;
    }
    let head = last + Vec3::new(0.4, 1.4, 0.9);
    out.push(ball(head, 0.75, Mat::Copper));
    out.push(bx(
        head + Vec3::new(0.0, -0.4, 0.7),
        Vec3::new(0.38, 0.55, 0.6),
        Mat::Copper,
    ));
    for s in [-1.0f32, 1.0] {
        out.push(round(
            head + Vec3::new(s * 0.4, 0.4, -0.3),
            1.0,
            0.18,
            0.02,
            Mat::Copper,
        ));
    }
    out.push(soft(ball(
        head + Vec3::new(0.0, -0.3, 1.35),
        0.3,
        Mat::Lamp,
    )));
}

/// How high a spiral carving's middle is over its origin, in its own frame.
const SPIRAL_MID: f32 = 112.0;

/// Big blocky letters centered on `at` along x, standing up from it, each `cell` square
/// and `depth` deep either side. A negative `cell` runs the text toward -x, to read from
/// the -z side. Solid letters are blocks, each column of cells merged into one; the rest
/// are painted on, one flat letter each.
fn spell(out: &mut Vec<Piece>, text: &str, at: Vec3, step: f32, depth: f32, mat: Mat, solid: bool) {
    let cell = step.abs();
    let letters: Vec<char> = text.chars().map(|c| c.to_ascii_uppercase()).collect();
    let width = letters.len() as f32 * 4.0 * step - step;
    if !solid {
        for (i, c) in letters.iter().enumerate() {
            if *c == ' ' || !c.is_ascii() {
                continue;
            }
            let x = (i as f32 * 4.0 + 1.5) * step - width * 0.5;
            out.push(Piece {
                form: Form::Letter {
                    at: at + Vec3::X * x,
                    cell: step,
                    depth,
                    ch: *c as u8,
                },
                mat,
                solid: false,
            });
        }
        return;
    }
    for (i, c) in letters.iter().enumerate() {
        let rows = glyph(*c);
        for col in 0..3u32 {
            let x = (i as f32 * 4.0 + col as f32) * step - width * 0.5 + step * 0.5;
            let mut row = 0;
            while row < 5 {
                if rows[row] & (0b100 >> col) == 0 {
                    row += 1;
                    continue;
                }
                let top = row;
                while row < 5 && rows[row] & (0b100 >> col) != 0 {
                    row += 1;
                }
                let (y0, y1) = ((5 - row) as f32 * cell, (5 - top) as f32 * cell);
                let mut p = bx(
                    at + Vec3::new(x, y0, 0.0),
                    Vec3::new(cell * 0.5, y1 - y0, depth),
                    mat,
                );
                p.solid = solid;
                out.push(p);
            }
        }
    }
}

/// Half the length of a subway platform, and the height of its mezzanine over the rails.
pub const METRO_PLATFORM: f32 = 52.0;
const MEZZANINE: f32 = 7.0;

/// A subway station in its vaulted hall, the track along x at the origin: a long tiled
/// platform with a yellow edge, lamps hung in rows, columns, benches and the station's
/// name on boards facing the track. Stairs at each end climb to a mezzanine along the back
/// with ticket gates, under the daylit shaft up to the street.
fn metro(out: &mut Vec<Piece>, name: &str) {
    let l = METRO_PLATFORM;
    // The platform, from the hall floor up to a step above the rails.
    out.push(bx(
        Vec3::new(0.0, -1.5, 8.5),
        Vec3::new(l, 2.5, 5.5),
        Mat::White,
    ));
    out.push(soft(bx(
        Vec3::new(0.0, 1.0, 3.3),
        Vec3::new(l, 0.02, 0.3),
        Mat::Canvas(2),
    )));
    // A low wall across the far side of the track, and the walkway behind the platform.
    out.push(soft(bx(
        Vec3::new(0.0, -1.5, -4.2),
        Vec3::new(l + 8.0, 2.0, 0.4),
        Mat::White,
    )));
    // Columns down the middle of the platform, lamp rows hung from the vault on both
    // sides of them.
    let n = 9;
    for k in 0..n {
        let x = (k as f32 / (n - 1) as f32 - 0.5) * (l * 2.0 - 12.0);
        out.push(round(
            Vec3::new(x, 1.0, 9.0),
            23.0,
            0.45,
            0.45,
            Mat::Canvas(1),
        ));
        if k % 2 == 0 && k != n / 2 {
            // The name on boards hung facing the track.
            out.push(soft(bx(
                Vec3::new(x, 3.6, 8.45),
                Vec3::new(4.2, 1.5, 0.08),
                Mat::Canvas(1),
            )));
            spell(
                out,
                name,
                Vec3::new(x, 3.95, 8.3),
                -0.16,
                0.05,
                Mat::White,
                false,
            );
        } else {
            // Benches back to back round the column.
            for z in [7.9f32, 10.1] {
                out.push(soft(bx(
                    Vec3::new(x, 1.0, z),
                    Vec3::new(1.6, 0.45, 0.35),
                    Mat::Wood,
                )));
            }
        }
    }
    for z in [5.0f32, 12.5] {
        for k in 0..6 {
            let x = (k as f32 / 5.0 - 0.5) * (l * 2.0 - 16.0);
            out.push(soft(bx(
                Vec3::new(x, 8.2, z),
                Vec3::new(5.5, 0.18, 0.35),
                Mat::Lamp,
            )));
            out.push(soft(round(
                Vec3::new(x, 8.4, z),
                16.0,
                0.05,
                0.05,
                Mat::Metal,
            )));
        }
    }
    // The mezzanine along the back, its railing, and the ticket gates across it.
    let m = MEZZANINE;
    out.push(bx(
        Vec3::new(0.0, m - 0.4, 18.5),
        Vec3::new(l * 0.75, 0.4, 3.5),
        Mat::Concrete,
    ));
    out.push(soft(bx(
        Vec3::new(0.0, m + 1.0, 15.05),
        Vec3::new(l * 0.75, 0.06, 0.06),
        Mat::Metal,
    )));
    for k in 0..12 {
        let x = (k as f32 / 11.0 - 0.5) * l * 1.5;
        out.push(soft(round(
            Vec3::new(x, m, 15.05),
            1.0,
            0.05,
            0.05,
            Mat::Metal,
        )));
    }
    for k in 0..7 {
        let x = (k as f32 / 6.0 - 0.5) * l * 1.5;
        out.push(round(
            Vec3::new(x, -1.5, 15.4),
            m - 1.1,
            0.35,
            0.35,
            Mat::White,
        ));
    }
    for k in 0..6 {
        out.push(soft(bx(
            Vec3::new(-5.0 + k as f32 * 2.0, m, 19.0),
            Vec3::new(0.25, 1.0, 0.7),
            Mat::Metal,
        )));
    }
    // Stairs up from the platform at each end of the mezzanine.
    let steps = 14;
    for side in [-1.0f32, 1.0] {
        for k in 0..steps {
            let u = k as f32 / steps as f32;
            let x = side * (l * 0.75 + 2.0 + (1.0 - u) * 10.0);
            out.push(bx(
                Vec3::new(x, 1.0, 12.0),
                Vec3::new(0.4, (m - 1.0) * (u + 1.0 / steps as f32), 2.0),
                Mat::Concrete,
            ));
        }
        out.push(bx(
            Vec3::new(side * (l * 0.75 + 1.0), m - 0.4, 16.0),
            Vec3::new(1.0, 0.4, 4.0),
            Mat::Concrete,
        ));
    }
}

/// A subway entrance in the street, round the stair shaft at the origin: paving round the
/// opening, a railing to lean on and look down, and tall signs with an M at two corners.
/// How steep subway stairs climb: rise over run.
pub const STAIR_SLOPE: f32 = 0.625;

/// A subway entrance at street level, in its frame with the stairs going down toward -z
/// from the origin: a glass canopy over the top of the stairs on four posts, railings
/// along both sides of the open cutting and across its far end, a paved apron in front
/// and two lit "M" totems either side of it.
fn metro_gate(out: &mut Vec<Piece>) {
    let cut = 42.0;
    out.push(soft(bx(
        Vec3::new(0.0, -0.4, 6.0),
        Vec3::new(13.0, 0.55, 6.0),
        Mat::Concrete,
    )));
    for (x, z) in [(-10.8f32, -7.0f32), (10.8, -7.0), (-10.8, 4.0), (10.8, 4.0)] {
        out.push(round(Vec3::new(x, 0.0, z), 5.0, 0.22, 0.2, Mat::Metal));
    }
    out.push(bx(
        Vec3::new(0.0, 5.0, -1.5),
        Vec3::new(11.6, 0.25, 6.8),
        Mat::Glass,
    ));
    out.push(soft(bx(
        Vec3::new(0.0, 4.7, -1.5),
        Vec3::new(11.8, 0.3, 7.0),
        Mat::Canvas(0),
    )));
    // Concrete retaining walls down both sides of the cutting, stepping down with the
    // stairs, and a paved edge over the rim behind them.
    for x in [-1.0f32, 1.0] {
        let mut z = 0.0f32;
        while z > -cut {
            let mid = z - 1.5;
            let low = mid * STAIR_SLOPE - 2.0;
            out.push(bx(
                Vec3::new(x * 8.5, low, mid),
                Vec3::new(0.5, 0.3 - low, 1.55),
                Mat::Concrete,
            ));
            z -= 3.0;
        }
        out.push(bx(
            Vec3::new(x * 10.2, -0.6, -cut * 0.5),
            Vec3::new(1.3, 0.85, cut * 0.5),
            Mat::Concrete,
        ));
    }
    // A headwall over the tunnel mouth at the foot of the cutting, framing it square so
    // the round bore behind never shows its rim.
    let foot = -cut * STAIR_SLOPE - 2.0;
    out.push(bx(
        Vec3::new(0.0, foot + 9.0, -cut - 0.4),
        Vec3::new(9.8, 0.3 - foot - 9.0, 0.6),
        Mat::Concrete,
    ));
    // Railings: posts and a top rail along each side of the cutting, and across its end.
    let rail = |out: &mut Vec<Piece>, a: Vec2, b: Vec2| {
        let n = (a.distance(b) / 2.5).ceil().max(1.0) as u32;
        for k in 0..=n {
            let p = a.lerp(b, k as f32 / n as f32);
            out.push(soft(round(
                Vec3::new(p.x, 0.0, p.y),
                1.1,
                0.06,
                0.06,
                Mat::Canvas(3),
            )));
        }
        let d = b - a;
        let mid = (a + b) * 0.5;
        out.push(soft(turned(
            Vec3::new(mid.x, 1.1, mid.y),
            Vec3::new(d.length() * 0.5, 0.08, 0.06),
            (-d.y).atan2(d.x),
            Mat::Canvas(3),
        )));
    };
    for x in [-10.4f32, 10.4] {
        rail(out, Vec2::new(x, 0.0), Vec2::new(x, -cut));
    }
    rail(out, Vec2::new(-10.4, -cut), Vec2::new(10.4, -cut));
    for x in [-1.0f32, 1.0] {
        let at = Vec3::new(x * 13.5, 0.2, 9.0);
        out.push(round(at, 5.5, 0.14, 0.12, Mat::Metal));
        out.push(soft(ball(at + Vec3::Y * 6.3, 0.9, Mat::Lamp)));
        for face in [-1.0f32, 1.0] {
            out.push(soft(bx(
                at + Vec3::new(0.0, 4.2, face * 0.2),
                Vec3::new(1.0, 1.6, 0.12),
                Mat::Canvas(0),
            )));
            spell(
                out,
                "M",
                at + Vec3::new(0.0, 4.4, face * 0.34),
                0.36,
                0.04,
                Mat::White,
                false,
            );
        }
    }
}

/// A wide flight of subway stairs in its frame: from the hall floor at the origin up
/// toward +z, `text="LEN DROP"`. Stone steps the full width with pale nosings, brass rail
/// posts down the middle and along both sides, lamps along the walls, and a landing at
/// the foot.
fn metro_stairs(out: &mut Vec<Piece>, text: &str) {
    let mut nums = text
        .split_whitespace()
        .filter_map(|n| n.parse::<f32>().ok());
    let (len, drop) = (nums.next().unwrap_or(90.0), nums.next().unwrap_or(56.0));
    let n = (drop / 0.5).ceil().max(1.0) as u32;
    let (rise, run) = (drop / n as f32, len / n as f32);
    let half = 8.0;
    for k in 0..n {
        let top = (k + 1) as f32 * rise;
        let mid = (k as f32 + 0.5) * run;
        out.push(bx(
            Vec3::new(0.0, top - 3.0, mid),
            Vec3::new(half, 3.0, run * 0.5 + 0.02),
            Mat::Stone,
        ));
        // A pale nosing on every step edge, so the flight reads as stairs from afar. It
        // stands proud of the step's top and front, never flush, so it cannot flicker.
        out.push(soft(bx(
            Vec3::new(0.0, top - 0.1, k as f32 * run + 0.06),
            Vec3::new(half - 0.05, 0.14, 0.12),
            Mat::White,
        )));
        if k % 3 == 0 {
            for x in [-half + 0.4, 0.0, half - 0.4] {
                out.push(soft(round(
                    Vec3::new(x, top, mid),
                    1.0,
                    0.05,
                    0.05,
                    Mat::Copper,
                )));
            }
        }
        if k % 12 == 6 {
            for x in [-1.0f32, 1.0] {
                out.push(soft(bx(
                    Vec3::new(x * 8.7, top + 4.0, mid),
                    Vec3::new(0.12, 0.5, 0.9),
                    Mat::Lamp,
                )));
            }
        }
    }
    out.push(bx(
        Vec3::new(0.0, -3.0, -3.0),
        Vec3::new(half, 3.0, 3.0),
        Mat::Stone,
    ));
}

/// A fence of posts and two rails round the corners given, in order and closed.
fn fence(out: &mut Vec<Piece>, corners: &[Vec2], post_gap: f32, mat: Mat) {
    for (k, &a) in corners.iter().enumerate() {
        let b = corners[(k + 1) % corners.len()];
        let len = a.distance(b);
        let n = (len / post_gap).ceil().max(1.0) as u32;
        for i in 0..n {
            let p = a.lerp(b, i as f32 / n as f32);
            out.push(soft(round(Vec3::new(p.x, 0.0, p.y), 1.3, 0.1, 0.08, mat)));
        }
        let mid = (a + b) * 0.5;
        let d = b - a;
        for y in [0.5f32, 1.1] {
            out.push(soft(turned(
                Vec3::new(mid.x, y, mid.y),
                Vec3::new(len * 0.5, 0.08, 0.05),
                (-d.y).atan2(d.x),
                mat,
            )));
        }
    }
}

/// An observatory on a mountain top: a round stone podium dug into the slope, a white
/// drum with a dome and its open slit, and a little workroom beside it.
fn observatory(out: &mut Vec<Piece>) {
    out.push(round(Vec3::Y * -14.0, 15.0, 9.0, 8.5, Mat::Stone));
    out.push(round(Vec3::Y, 6.0, 6.0, 6.0, Mat::White));
    out.push(ball(Vec3::Y * 7.0, 6.2, Mat::White));
    // The slit, open to the sky, and the telescope peeking out.
    out.push(soft(bx(
        Vec3::new(0.0, 7.5, 3.6),
        Vec3::new(1.0, 5.5, 2.4),
        Mat::Dark,
    )));
    out.push(soft(round(
        Vec3::new(0.0, 9.0, 3.0),
        3.0,
        0.5,
        0.4,
        Mat::Metal,
    )));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-9.0, -3.0, -2.0),
            half: Vec3::new(3.5, 7.0, 3.0),
            yaw: 0.0,
            roof: 1.2,
        },
        mat: Mat::Stone,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(-9.0, 1.0, 1.05),
        Vec3::new(1.2, 1.2, 0.1),
        Mat::Lamp,
    )));
}

/// A fire lookout: a wooden cabin with windows all round on four tall braced legs, a
/// stair zigzagging up between them, and a walkway round the cabin.
fn lookout(out: &mut Vec<Piece>) {
    let tall = 18.0;
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(bx(
            Vec3::new(x * 3.2, -8.0, z * 3.2),
            Vec3::new(0.3, tall + 8.0, 0.3),
            Mat::Wood,
        ));
    }
    for k in 0..4 {
        let y = 2.0 + k as f32 * 4.0;
        for (x, z, hx, hz) in [
            (0.0f32, 3.2f32, 3.2f32, 0.1f32),
            (0.0, -3.2, 3.2, 0.1),
            (3.2, 0.0, 0.1, 3.2),
            (-3.2, 0.0, 0.1, 3.2),
        ] {
            out.push(soft(bx(
                Vec3::new(x, y, z),
                Vec3::new(hx, 0.15, hz),
                Mat::Wood,
            )));
        }
        // A flight of the stair, turning at every level.
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        out.push(soft(Piece {
            form: Form::Box {
                at: Vec3::new(0.0, y + 1.8, side * 1.5),
                half: Vec3::new(2.6, 0.12, 0.6),
                yaw: side * 0.6,
                roof: 0.0,
            },
            mat: Mat::Wood,
            solid: false,
        }));
    }
    out.push(bx(
        Vec3::new(0.0, tall, 0.0),
        Vec3::new(5.0, 0.4, 5.0),
        Mat::Wood,
    ));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, tall + 0.4, 0.0),
            half: Vec3::new(3.6, 3.2, 3.6),
            yaw: 0.0,
            roof: 1.8,
        },
        mat: Mat::Wood,
        solid: true,
    });
    for (x, z, hx, hz) in [
        (0.0f32, 3.62f32, 3.0f32, 0.05f32),
        (0.0, -3.62, 3.0, 0.05),
        (3.62, 0.0, 0.05, 3.0),
        (-3.62, 0.0, 0.05, 3.0),
    ] {
        out.push(soft(bx(
            Vec3::new(x, tall + 1.6, z),
            Vec3::new(hx, 1.3, hz),
            Mat::Glass,
        )));
        out.push(soft(bx(
            Vec3::new(x * 1.37, tall + 0.4, z * 1.37),
            Vec3::new(hx.max(0.05) * 1.6, 1.0, hz.max(0.05) * 1.6),
            Mat::Wood,
        )));
    }
}

/// A water park's splash playground: a blue splash pad, a climbing deck on red posts
/// with stairs up and a slide down, little fountains, and two tall posts carrying the
/// big bucket (drawn by the client) that tips over the deck.
fn splash_playground(out: &mut Vec<Piece>) {
    let c = Vec3::new(SPLASH.x, 0.0, SPLASH.z);
    out.push(soft(round(c, 0.26, 10.0, 10.0, Mat::Canvas(1))));
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(round(
            c + Vec3::new(x * 3.2, 0.0, z * 3.2),
            SPLASH_DECK + 2.4,
            0.2,
            0.2,
            Mat::Canvas(0),
        ));
    }
    out.push(bx(
        c + Vec3::Y * (SPLASH_DECK - 0.25),
        Vec3::new(3.5, 0.25, 3.5),
        Mat::Canvas(2),
    ));
    // Rails round the deck, open on the stair and slide sides.
    for (x, z, hx, hz) in [(0.0f32, -3.4f32, 3.4f32, 0.06f32), (-3.4, 0.0, 0.06, 3.4)] {
        out.push(soft(bx(
            c + Vec3::new(x, SPLASH_DECK + 0.9, z),
            Vec3::new(hx, 0.08, hz),
            Mat::White,
        )));
    }
    // Stairs up the side.
    for k in 0..5 {
        let y = (k + 1) as f32 * SPLASH_DECK / 6.0;
        out.push(bx(
            c + Vec3::new(4.3 + (4 - k) as f32 * 0.7, 0.0, 1.5),
            Vec3::new(0.36, y, 1.0),
            Mat::Canvas(3),
        ));
    }
    // A short slide down off the front, in steps a child slides over.
    for k in 0..6 {
        let y = SPLASH_DECK * (1.0 - (k as f32 + 0.5) / 6.0);
        out.push(soft(bx(
            c + Vec3::new(-1.2, y - 0.2, 4.0 + k as f32 * 0.8),
            Vec3::new(0.6, 0.2, 0.45),
            Mat::Canvas(5),
        )));
    }
    // The bucket's posts, tall above the deck, and the cap on each.
    for x in [-1.0f32, 1.0] {
        let at = c + Vec3::new(x * 2.3, 0.0, 0.0);
        out.push(round(at, SPLASH.y + 0.4, 0.22, 0.18, Mat::Canvas(4)));
        out.push(soft(ball(
            at + Vec3::Y * (SPLASH.y + 0.5),
            0.35,
            Mat::Canvas(2),
        )));
    }
    // Little fountains round the pad.
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.3;
        out.push(soft(round(
            c + Vec3::new(a.cos() * 7.5, 0.0, a.sin() * 7.5),
            0.9,
            0.25,
            0.15,
            Mat::Canvas(k as u8 % 3 * 2),
        )));
    }
}

/// A theme park's grounds: mown lawns, paved walks from the gate to a ring round the
/// castle and on to every ride, bright tile stars set in the paving, flower beds edged
/// with hedges, and a pond with a fountain.
fn themepark_grounds(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(85.0, 0.15, 60.0),
        Mat::Grass,
    )));
    let walk = |out: &mut Vec<Piece>, a: Vec2, b: Vec2, w: f32| {
        let d = b - a;
        out.push(soft(turned(
            Vec3::new((a.x + b.x) * 0.5, 0.0, (a.y + b.y) * 0.5),
            Vec3::new(d.length() * 0.5 + w * 0.5, 0.25, w * 0.5),
            (-d.y).atan2(d.x),
            Mat::Concrete,
        )));
    };
    // The plaza inside the gate, along the food stalls.
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 51.0),
        Vec3::new(62.0, 0.25, 8.0),
        Mat::Concrete,
    )));
    // The ring round the castle, and walks out to the rides.
    let ring: Vec<Vec2> = (0..20)
        .map(|k| {
            let a = k as f32 / 20.0 * std::f32::consts::TAU;
            Vec2::new(a.cos() * 22.0, a.sin() * 18.0)
        })
        .collect();
    for k in 0..ring.len() {
        walk(out, ring[k], ring[(k + 1) % ring.len()], 4.0);
    }
    for (a, b) in [
        (Vec2::new(0.0, 44.0), Vec2::new(0.0, 18.0)),
        (
            Vec2::new(20.0, 8.0),
            Vec2::new(CAROUSEL.x - 8.0, CAROUSEL.z - 4.0),
        ),
        (
            Vec2::new(-30.0, 44.0),
            Vec2::new(DROP.x + 5.0, DROP.z + 3.0),
        ),
        (
            Vec2::new(18.0, -10.0),
            Vec2::new(THEMEPARK_FERRIS.x, THEMEPARK_FERRIS.y + 8.0),
        ),
        (Vec2::new(-20.0, -8.0), Vec2::new(-38.0, -18.0)),
    ] {
        walk(out, a, b, 4.0);
    }
    // Stars of bright tiles set in the paving: one in the plaza, one before the castle.
    for (at, r) in [(Vec2::new(0.0, 50.0), 5.5f32), (Vec2::new(0.0, 14.0), 3.5)] {
        for k in 0..4u32 {
            out.push(soft(turned(
                Vec3::new(at.x, 0.0, at.y),
                Vec3::new(r, 0.27 + k as f32 * 0.01, r * 0.22),
                k as f32 * std::f32::consts::FRAC_PI_4,
                Mat::Canvas((k % 3 * 2) as u8),
            )));
        }
        out.push(soft(round(
            Vec3::new(at.x, 0.0, at.y),
            0.32,
            r * 0.3,
            r * 0.3,
            Mat::Canvas(2),
        )));
    }
    // Flower beds edged with low hedges, either side of the walk in.
    for (k, z) in [36.0f32, 26.0].into_iter().enumerate() {
        for x in [-1.0f32, 1.0] {
            let at = Vec3::new(x * 6.5, 0.0, z);
            out.push(soft(bx(at, Vec3::new(3.2, 0.5, 4.2), Mat::Hedge)));
            out.push(soft(bx(
                at,
                Vec3::new(2.6, 0.6, 3.6),
                Mat::Canvas(if x < 0.0 { 4 } else { 2 } + k as u8),
            )));
        }
    }
    // Round beds of flowers out on the lawns, in all the colors.
    for (k, (x, z)) in [
        (-30.0f32, 20.0f32),
        (30.0, -30.0),
        (-10.0, -35.0),
        (55.0, 5.0),
        (-60.0, 0.0),
        (25.0, 40.0),
    ]
    .into_iter()
    .enumerate()
    {
        let at = Vec3::new(x, 0.0, z);
        out.push(soft(round(at, 0.5, 3.4, 3.4, Mat::Hedge)));
        out.push(soft(round(at, 0.6, 2.8, 2.8, Mat::Canvas(k as u8))));
        out.push(soft(ball(
            at + Vec3::Y * 0.6,
            1.0,
            Mat::Canvas(k as u8 + 2),
        )));
    }
    // The pond, with a stone rim and a fountain in the middle.
    let pond = Vec3::new(-45.0, 0.0, -22.0);
    out.push(soft(round(pond, 0.5, 9.0, 9.0, Mat::Stone)));
    out.push(soft(round(pond, 0.55, 8.2, 8.2, Mat::Water)));
    out.push(round(pond, 1.6, 1.2, 0.9, Mat::Stone));
    out.push(soft(round(
        pond + Vec3::Y * 1.6,
        1.4,
        0.5,
        0.05,
        Mat::White,
    )));
}

/// A balloon field: a mown launch field where balloons come down, wait and lift off
/// again, with balloons laid out on the grass being filled, baskets and burners, a crew
/// tent, a ticket hut and a windsock.
fn balloon_field(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(70.0, 0.12, 55.0),
        Mat::Grass,
    )));
    // A ring of white marker stones round each landing spot.
    for spot in BALLOON_SPOTS {
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            out.push(soft(bx(
                Vec3::new(spot.x + a.cos() * 11.0, 0.0, spot.y + a.sin() * 11.0),
                Vec3::new(0.5, 0.3, 0.5),
                Mat::White,
            )));
        }
    }
    // Balloons getting ready: tethered upright over their baskets, one already full and
    // two still filling, rounder at the top and narrowing to the burner, ropes pegged out.
    for (k, (x, z, full)) in [
        (-40.0f32, -30.0f32, 1.0f32),
        (-15.0, 35.0, 0.75),
        (45.0, 30.0, 0.6),
    ]
    .into_iter()
    .enumerate()
    {
        let at = Vec3::new(x, 0.0, z);
        let r = 7.5 * full;
        let neck = 2.4 + 2.0 * full;
        let top = Vec3::Y * (neck + 2.4 + r * 0.9);
        out.push(ball(at + top, r, Mat::Canvas([0u8, 1, 4][k])));
        out.push(soft(round(
            at + Vec3::Y * 2.4,
            neck + r * 0.45,
            1.0,
            r * 0.92,
            Mat::Canvas([3u8, 5, 2][k]),
        )));
        out.push(bx(at, Vec3::new(0.8, 1.1, 0.8), Mat::Hay));
        out.push(soft(bx(
            at + Vec3::Y * 1.7,
            Vec3::new(0.5, 0.3, 0.5),
            Mat::Metal,
        )));
        for (dx, dz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            out.push(soft(bx(
                at + Vec3::new(dx * 6.0, 0.0, dz * 6.0),
                Vec3::new(0.15, 0.5, 0.15),
                Mat::Wood,
            )));
        }
    }
    // Baskets standing ready, a burner frame over each.
    for (x, z) in [(55.0f32, -35.0f32), (59.0, -31.0), (52.0, -28.0)] {
        let at = Vec3::new(x, 0.0, z);
        out.push(bx(at, Vec3::new(0.7, 1.1, 0.7), Mat::Hay));
        out.push(soft(bx(
            at + Vec3::Y * 2.2,
            Vec3::new(0.5, 0.25, 0.5),
            Mat::Metal,
        )));
    }
    // The crew tent and the ticket hut by the way in.
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-50.0, 0.0, 40.0),
            half: Vec3::new(6.0, 2.4, 4.0),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::Canvas(1),
        solid: true,
    });
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 0.0, 52.0),
            half: Vec3::new(3.0, 2.6, 2.0),
            yaw: 0.0,
            roof: 1.0,
        },
        mat: Mat::Wood,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 1.0, 54.05),
        Vec3::new(1.4, 0.9, 0.05),
        Mat::Lamp,
    )));
    // The windsock on its pole.
    let pole = Vec3::new(-62.0, 0.0, -45.0);
    out.push(soft(round(pole, 8.0, 0.12, 0.08, Mat::Metal)));
    out.push(soft(turned(
        pole + Vec3::new(1.8, 7.4, 0.0),
        Vec3::new(1.8, 0.45, 0.4),
        0.0,
        Mat::Canvas(5),
    )));
}

/// A seal colony: low flat-topped rocks just off the shore, splashed white by gulls. The seals are drawn by the client.
fn seal_rocks(out: &mut Vec<Piece>) {
    for (k, (c, r, top)) in SEAL_ROCKS.into_iter().enumerate() {
        let at = Vec3::new(c.x, -16.0, c.y);
        out.push(round(at, 16.0 + top, r * 1.8, r, Mat::Stone));
        // A boulder or two on the side of the top, leaving room to lie.
        let a = k as f32 * 2.3;
        out.push(ball(
            Vec3::new(c.x + a.cos() * r * 0.8, top, c.y + a.sin() * r * 0.8),
            r * 0.35,
            Mat::Stone,
        ));
        out.push(soft(bx(
            Vec3::new(c.x - a.cos() * r * 0.5, top, c.y - a.sin() * r * 0.4),
            Vec3::new(0.4, 0.02, 0.3),
            Mat::White,
        )));
    }
}

/// A travelling carnival on a field: a chair swing ride (its spinning top and chairs drawn
/// by the client), a bumper car pavilion (the cars too), a striped helter skelter with a
/// slide spiralling round it, game stalls, strings of bulbs on poles and a lit arch.
fn carnival(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(45.0, 0.12, 35.0),
        Mat::Grass,
    )));
    // The chair swing: a striped tower on a round deck.
    let c = CHAIR_SWING;
    out.push(round(c, 0.5, 5.0, 5.0, Mat::Wood));
    for k in 0..4 {
        out.push(round(
            c + Vec3::Y * (0.5 + k as f32 * 2.2),
            2.2,
            0.9 - k as f32 * 0.08,
            0.82 - k as f32 * 0.08,
            if k % 2 == 0 {
                Mat::Canvas(0)
            } else {
                Mat::White
            },
        ));
    }
    // The bumper cars' pavilion: a dark floor, a rail round it, posts and a striped roof.
    let (b, half) = BUMPER;
    let bc = Vec3::new(b.x, 0.0, b.y);
    out.push(bx(
        bc,
        Vec3::new(half.x + 1.0, 0.3, half.y + 1.0),
        Mat::Dark,
    ));
    for (x, z, hx, hz) in [
        (0.0f32, half.y + 0.9, half.x + 0.9, 0.08f32),
        (0.0, -half.y - 0.9, half.x + 0.9, 0.08),
        (half.x + 0.9, 0.0, 0.08, half.y + 0.9),
        (-half.x - 0.9, 0.0, 0.08, half.y + 0.9),
    ] {
        out.push(soft(bx(
            bc + Vec3::new(x, 0.3, z),
            Vec3::new(hx, 0.6, hz),
            Mat::Canvas(2),
        )));
    }
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(round(
            bc + Vec3::new(x * (half.x + 0.8), 0.3, z * (half.y + 0.8)),
            4.7,
            0.2,
            0.2,
            Mat::Metal,
        ));
    }
    for k in 0..4 {
        out.push(soft(bx(
            bc + Vec3::new(
                0.0,
                5.0 + k as f32 * 0.01,
                (k as f32 - 1.5) * (half.y + 1.4) * 0.5,
            ),
            Vec3::new(half.x + 1.4, 0.2, (half.y + 1.4) * 0.25),
            if k % 2 == 0 {
                Mat::Canvas(4)
            } else {
                Mat::White
            },
        )));
    }
    out.push(Piece {
        form: Form::Box {
            at: bc + Vec3::Y * 5.2,
            half: Vec3::new(half.x + 1.4, 0.1, half.y + 1.4),
            yaw: 0.0,
            roof: 1.6,
        },
        mat: Mat::Canvas(4),
        solid: true,
    });
    // The helter skelter: a tower in red and white bands, a pointed cap, and the slide
    // winding down round it.
    let h = HELTER;
    for k in 0..6 {
        out.push(round(
            h + Vec3::Y * (k as f32 * 2.0),
            2.0,
            3.0 - k as f32 * 0.12,
            2.88 - k as f32 * 0.12,
            if k % 2 == 0 { Mat::Red } else { Mat::White },
        ));
    }
    out.push(round(h + Vec3::Y * 12.0, 3.5, 2.6, 0.0, Mat::Canvas(1)));
    out.push(soft(round(h + Vec3::Y * 15.5, 1.6, 0.06, 0.04, Mat::Metal)));
    out.push(soft(bx(
        h + Vec3::new(0.6, 16.4, 0.0),
        Vec3::new(0.6, 0.35, 0.03),
        Mat::Canvas(2),
    )));
    let turns = 40;
    for k in 0..turns {
        let a = k as f32 / turns as f32 * std::f32::consts::TAU * 2.5;
        let y = 11.0 * (1.0 - k as f32 / turns as f32);
        out.push(soft(turned(
            h + Vec3::new(a.cos() * 4.1, y, a.sin() * 4.1),
            Vec3::new(1.2, 0.15, 0.6),
            (-a.cos()).atan2(-a.sin()),
            Mat::Canvas(2),
        )));
    }
    out.push(soft(bx(
        h + Vec3::new(4.1, 0.0, 0.0),
        Vec3::new(1.4, 0.2, 1.4),
        Mat::Canvas(4),
    )));
    // Game stalls along the back, striped roofs and prizes hung up.
    for k in 0..5u32 {
        let at = Vec3::new(-24.0 + k as f32 * 12.0, 0.0, -28.0);
        out.push(bx(at, Vec3::new(3.6, 2.4, 2.2), Mat::Wood));
        out.push(soft(bx(
            at + Vec3::new(0.0, 1.0, 2.25),
            Vec3::new(2.8, 0.9, 0.05),
            Mat::Lamp,
        )));
        out.push(Piece {
            form: Form::Box {
                at: at + Vec3::Y * 2.4,
                half: Vec3::new(4.0, 0.1, 2.8),
                yaw: 0.0,
                roof: 1.2,
            },
            mat: Mat::Canvas((k % 6) as u8),
            solid: true,
        });
        for j in 0..3 {
            out.push(soft(ball(
                at + Vec3::new(-1.8 + j as f32 * 1.8, 2.0, 2.4),
                0.35,
                Mat::Canvas(((k + j) % 6) as u8),
            )));
        }
    }
    // Strings of bulbs from pole to pole round the field, lit at night.
    let poles = [
        Vec2::new(-40.0, -32.0),
        Vec2::new(-40.0, 30.0),
        Vec2::new(40.0, 30.0),
        Vec2::new(40.0, -32.0),
        Vec2::new(0.0, 0.0),
    ];
    for p in poles {
        out.push(soft(round(
            Vec3::new(p.x, 0.0, p.y),
            7.0,
            0.14,
            0.1,
            Mat::Wood,
        )));
    }
    for k in 0..4 {
        let (a, b) = (poles[k], poles[4]);
        for j in 1..12 {
            let u = j as f32 / 12.0;
            let p = a.lerp(b, u);
            let sag = (u * std::f32::consts::PI).sin() * 1.6;
            out.push(soft(ball(Vec3::new(p.x, 6.8 - sag, p.y), 0.18, Mat::Lamp)));
        }
    }
    // The arch over the way in.
    for x in [-5.0f32, 5.0] {
        out.push(round(
            Vec3::new(x, 0.0, 34.0),
            6.0,
            0.4,
            0.35,
            Mat::Canvas(4),
        ));
    }
    out.push(bx(
        Vec3::new(0.0, 6.0, 34.0),
        Vec3::new(5.6, 1.3, 0.4),
        Mat::Canvas(1),
    ));
    spell(
        out,
        "FUN FAIR",
        Vec3::new(0.0, 6.2, 34.45),
        0.32,
        0.08,
        Mat::Lamp,
        false,
    );
}

/// A quiet churchyard: a low stone wall with a lych gate, gravel paths in a cross, rows of
/// headstones with flowers left at some, dark yews, benches, and a little chapel.
fn cemetery(out: &mut Vec<Piece>) {
    let (hx, hz) = (34.0f32, 26.0f32);
    out.push(soft(bx(Vec3::ZERO, Vec3::new(hx, 0.1, hz), Mat::Grass)));
    for (x, z, sx, sz) in [
        (0.0f32, -hz, hx, 0.35f32),
        (-hx, 0.0, 0.35, hz),
        (hx, 0.0, 0.35, hz),
        (-hx * 0.5 - 2.0, hz, hx * 0.5 - 2.0, 0.35),
        (hx * 0.5 + 2.0, hz, hx * 0.5 - 2.0, 0.35),
    ] {
        out.push(bx(Vec3::new(x, 0.0, z), Vec3::new(sx, 1.1, sz), Mat::Stone));
    }
    // The lych gate: a little roof on four posts over the way in.
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(round(
            Vec3::new(x * 2.2, 0.0, hz + z * 1.2),
            2.6,
            0.15,
            0.15,
            Mat::Wood,
        ));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 2.6, hz),
            half: Vec3::new(2.8, 0.15, 1.8),
            yaw: 0.0,
            roof: 1.4,
        },
        mat: Mat::Hay,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.5, 0.16, hz),
        Mat::Concrete,
    )));
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, -2.0),
        Vec3::new(hx, 0.15, 1.5),
        Mat::Concrete,
    )));
    // Rows of headstones either side of the paths.
    for qx in [-1.0f32, 1.0] {
        for qz in [-1.0f32, 1.0] {
            for row in 0..4 {
                for k in 0..7 {
                    let x = qx * (5.0 + k as f32 * 3.8);
                    let z = -2.0 + qz * (5.0 + row as f32 * 4.6);
                    if z.abs() > hz - 2.0 || (qz < 0.0 && x.abs() < 14.0 && z < -12.0) {
                        continue;
                    }
                    let s = (k * 7 + row * 3) as u32 + if qx > 0.0 { 50 } else { 0 };
                    let tall = 0.8 + (s % 3) as f32 * 0.2;
                    out.push(bx(
                        Vec3::new(x, 0.0, z),
                        Vec3::new(0.45, tall, 0.12),
                        Mat::Stone,
                    ));
                    out.push(soft(ball(Vec3::new(x, tall, z), 0.45, Mat::Stone)));
                    if s.is_multiple_of(4) {
                        out.push(soft(ball(
                            Vec3::new(x + 0.2, 0.15, z + 0.6),
                            0.22,
                            Mat::Canvas((s % 5) as u8),
                        )));
                    }
                }
            }
        }
    }
    // The chapel at the back, its spire over the trees.
    let ch = Vec3::new(0.0, 0.0, -18.0);
    out.push(Piece {
        form: Form::Box {
            at: ch,
            half: Vec3::new(5.0, 5.0, 4.0),
            yaw: 0.0,
            roof: 3.5,
        },
        mat: Mat::White,
        solid: true,
    });
    out.push(bx(
        ch + Vec3::new(0.0, 0.0, 4.5),
        Vec3::new(1.6, 9.0, 1.6),
        Mat::White,
    ));
    out.push(round(
        ch + Vec3::new(0.0, 9.0, 4.5),
        6.0,
        1.8,
        0.0,
        Mat::Dark,
    ));
    out.push(soft(bx(
        ch + Vec3::new(0.0, 0.0, 6.12),
        Vec3::new(0.8, 2.2, 0.05),
        Mat::Wood,
    )));
    for x in [-2.8f32, 2.8] {
        out.push(soft(bx(
            ch + Vec3::new(x, 1.8, 4.02),
            Vec3::new(0.6, 1.4, 0.05),
            Mat::Lamp,
        )));
    }
    // Yews, dark and round, and benches under them.
    for (x, z) in [
        (-28.0f32, 20.0f32),
        (28.0, 20.0),
        (-28.0, -20.0),
        (28.0, -20.0),
        (-12.0, -20.0),
    ] {
        out.push(round(Vec3::new(x, 0.0, z), 1.6, 0.4, 0.3, Mat::Wood));
        out.push(ball(Vec3::new(x, 4.2, z), 3.4, Mat::Hedge));
        out.push(ball(Vec3::new(x, 6.8, z), 2.2, Mat::Hedge));
    }
    for x in [-6.0f32, 6.0] {
        out.push(bx(
            Vec3::new(x, 0.0, 20.0),
            Vec3::new(1.0, 0.45, 0.3),
            Mat::Wood,
        ));
    }
}

/// A factory: three long brick halls under gabled roofs, loading doors along the front
/// with crates and a forklift, a tall banded chimney letting off steam, and an office.
fn factory(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(50.0, 0.15, 36.0),
        Mat::Asphalt,
    )));
    for k in 0..3 {
        let at = Vec3::new(-24.0 + k as f32 * 13.0, 0.0, -6.0);
        out.push(bx(at, Vec3::new(6.4, 8.0, 18.0), Mat::Red));
        out.push(Piece {
            form: Form::Box {
                at: at + Vec3::Y * 8.0,
                half: Vec3::new(6.6, 0.2, 18.2),
                yaw: 0.0,
                roof: 3.0,
            },
            mat: Mat::Metal,
            solid: true,
        });
        // A big door in the front, and windows down the side.
        out.push(soft(bx(
            at + Vec3::new(0.0, 0.0, 18.05),
            Vec3::new(3.0, 5.0, 0.05),
            Mat::Dark,
        )));
        for j in 0..5 {
            out.push(soft(bx(
                at + Vec3::new(6.45, 4.5, -14.0 + j as f32 * 7.0),
                Vec3::new(0.05, 1.6, 2.2),
                Mat::Lamp,
            )));
        }
    }
    // The chimney, with red and white bands up top.
    let c = FACTORY_STACK;
    let foot = Vec3::new(c.x, 0.0, c.z);
    out.push(round(foot, c.y - 4.0, 2.2, 1.5, Mat::Stone));
    out.push(round(
        foot + Vec3::Y * (c.y - 4.0),
        2.0,
        1.5,
        1.45,
        Mat::Red,
    ));
    out.push(round(
        foot + Vec3::Y * (c.y - 2.0),
        2.0,
        1.45,
        1.4,
        Mat::White,
    ));
    // The office in front, and a sign on its roof.
    let o = Vec3::new(26.0, 0.0, 14.0);
    out.push(bx(o, Vec3::new(9.0, 7.0, 5.0), Mat::White));
    for y in [1.5f32, 4.5] {
        out.push(soft(bx(
            o + Vec3::new(0.0, y, 5.05),
            Vec3::new(8.0, 1.0, 0.05),
            Mat::Glass,
        )));
    }
    out.push(soft(bx(
        o + Vec3::Y * 7.0,
        Vec3::new(7.0, 1.6, 0.2),
        Mat::Canvas(1),
    )));
    spell(
        out,
        "TOYS",
        o + Vec3::new(0.0, 7.3, 0.25),
        0.4,
        0.1,
        Mat::Lamp,
        false,
    );
    // Crates and pallets at the doors, and a forklift.
    for (k, x) in [-27.0f32, -20.0, -11.0, 0.0, 4.0].into_iter().enumerate() {
        out.push(bx(
            Vec3::new(x, 0.0, 15.0 + (k % 2) as f32 * 2.0),
            Vec3::new(1.0, 1.0 + (k % 3) as f32 * 0.6, 1.0),
            Mat::Wood,
        ));
    }
    out.push(bx(
        Vec3::new(-6.0, 0.0, 20.0),
        Vec3::new(0.8, 1.1, 1.3),
        Mat::Canvas(2),
    ));
    out.push(soft(bx(
        Vec3::new(-6.0, 0.0, 21.6),
        Vec3::new(0.6, 2.4, 0.08),
        Mat::Metal,
    )));
    // Lines in the yard for the trucks.
    for k in 0..6 {
        out.push(soft(bx(
            Vec3::new(-40.0 + k as f32 * 5.0, 0.0, 30.0),
            Vec3::new(0.12, 0.17, 4.0),
            Mat::White,
        )));
    }
}

/// A power station: two great cooling towers breathing steam, the turbine hall between
/// them, a slim chimney, and a switchyard of steel frames where the lines leave.
fn power_station(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(55.0, 0.15, 40.0),
        Mat::Concrete,
    )));
    for top in POWER_STACKS {
        let foot = Vec3::new(top.x, 0.0, top.z);
        // Waisted like the real ones: narrowing to a throat, flaring at the lip.
        out.push(round(foot, top.y * 0.65, 15.0, 10.5, Mat::Concrete));
        out.push(round(
            foot + Vec3::Y * top.y * 0.65,
            top.y * 0.35,
            10.5,
            11.5,
            Mat::Concrete,
        ));
        out.push(soft(round(
            foot + Vec3::Y * (top.y - 3.0),
            2.0,
            11.2,
            11.4,
            Mat::Canvas(0),
        )));
    }
    let hall = Vec3::new(0.0, 0.0, 18.0);
    out.push(bx(hall, Vec3::new(16.0, 12.0, 9.0), Mat::Canvas(1)));
    out.push(soft(bx(
        hall + Vec3::new(0.0, 9.0, 9.05),
        Vec3::new(14.0, 1.4, 0.05),
        Mat::Lamp,
    )));
    out.push(round(
        Vec3::new(-14.0, 0.0, 30.0),
        40.0,
        1.6,
        1.0,
        Mat::White,
    ));
    out.push(soft(round(
        Vec3::new(-14.0, 37.0, 30.0),
        3.0,
        1.12,
        1.02,
        Mat::Red,
    )));
    // The switchyard: frames of steel in rows.
    for i in 0..3 {
        for j in 0..2 {
            let at = Vec3::new(28.0 + i as f32 * 8.0, 0.0, 14.0 + j as f32 * 10.0);
            for x in [-2.5f32, 2.5] {
                out.push(soft(round(at + Vec3::X * x, 7.0, 0.2, 0.2, Mat::Metal)));
            }
            out.push(soft(bx(
                at + Vec3::Y * 7.0,
                Vec3::new(2.7, 0.2, 0.2),
                Mat::Metal,
            )));
            out.push(soft(bx(at, Vec3::new(1.2, 2.2, 1.0), Mat::Metal)));
        }
    }
}

/// An oil rig far out at sea: four yellow legs down to the seabed, a deck of white and
/// orange modules, the tall derrick, a crane, and a helipad hung off one corner. Seen
/// from afar its lights pick it out at night.
fn oil_rig(out: &mut Vec<Piece>) {
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(round(
            Vec3::new(x * 14.0, -100.0, z * 14.0),
            100.0 + RIG_DECK,
            2.4,
            2.0,
            Mat::Canvas(2),
        ));
        for y in [-8.0f32, 4.0] {
            out.push(soft(bx(
                Vec3::new(x * 14.0, y, 0.0),
                Vec3::new(0.5, 0.5, 14.0),
                Mat::Canvas(2),
            )));
        }
    }
    out.push(bx(
        Vec3::Y * (RIG_DECK - 1.6),
        Vec3::new(19.0, 1.6, 19.0),
        Mat::Metal,
    ));
    // Modules on the deck.
    for (x, z, sx, h, sz, m) in [
        (-9.0f32, -9.0f32, 8.0f32, 9.0f32, 8.0f32, Mat::White),
        (9.0, -10.0, 7.0, 6.0, 6.0, Mat::Canvas(5)),
        (-11.0, 8.0, 6.0, 5.0, 8.0, Mat::Canvas(5)),
        (2.0, 2.0, 5.0, 4.0, 5.0, Mat::White),
    ] {
        out.push(bx(Vec3::new(x, RIG_DECK, z), Vec3::new(sx, h, sz), m));
    }
    for k in 0..3 {
        out.push(soft(bx(
            Vec3::new(-9.0, RIG_DECK + 2.0 + k as f32 * 2.6, -0.95),
            Vec3::new(7.0, 0.5, 0.05),
            Mat::Lamp,
        )));
    }
    // The derrick: a tall tapering steel tower with a light at its top.
    let d = Vec3::new(9.0, RIG_DECK + 6.0, -10.0);
    out.push(round(d, 32.0, 3.2, 0.7, Mat::Metal));
    out.push(soft(ball(d + Vec3::Y * 32.5, 0.6, Mat::Lamp)));
    // Bands up the derrick, and lifeboats slung under the deck edge, bright orange.
    for k in 1..5 {
        let y = k as f32 * 6.0;
        let r = 3.2 - (3.2 - 0.7) * y / 32.0;
        out.push(soft(round(
            d + Vec3::Y * y,
            0.6,
            r + 0.06,
            r + 0.04,
            Mat::White,
        )));
    }
    for x in [-8.0f32, 0.0] {
        out.push(soft(bx(
            Vec3::new(x, RIG_DECK - 4.0, -19.6),
            Vec3::new(2.6, 1.6, 1.2),
            Mat::Canvas(5),
        )));
    }
    // The helipad, out past the corner on struts, marked with its H.
    let pad = RIG_PAD;
    out.push(round(
        Vec3::new(pad.x, pad.y - 0.5, pad.z),
        0.5,
        9.0,
        9.0,
        Mat::Dark,
    ));
    for (x, z, sx, sz) in [
        (-1.6f32, 0.0f32, 0.35f32, 2.4f32),
        (1.6, 0.0, 0.35, 2.4),
        (0.0, 0.0, 1.6, 0.35),
    ] {
        out.push(soft(bx(
            Vec3::new(pad.x + x, pad.y, pad.z + z),
            Vec3::new(sx, 0.02, sz),
            Mat::White,
        )));
    }
    for (x, z) in [(1.0f32, 0.3f32), (0.3, 1.0)] {
        out.push(soft(round(
            Vec3::new(pad.x - x * 6.0, RIG_DECK - 10.0, pad.z - z * 6.0),
            10.0,
            0.3,
            0.3,
            Mat::Metal,
        )));
    }
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        out.push(soft(ball(
            Vec3::new(pad.x + a.cos() * 8.6, pad.y + 0.1, pad.z + a.sin() * 8.6),
            0.18,
            Mat::Lamp,
        )));
    }
}

/// A roadside billboard on two steel legs: a bright board with a message in big letters,
/// a catwalk under it and lamps that light it at night.
fn billboard(out: &mut Vec<Piece>, seed: u32, text: &str) {
    let ads = [
        "SUNNY COLA",
        "VISIT KETTLE",
        "FLY SAFE",
        "MOON PIZZA",
        "EAT FRUIT",
        "SEE THE ZOO",
        "BUY TOYS",
        "SMILE",
    ];
    let words = if text.is_empty() {
        ads[seed as usize % ads.len()]
    } else {
        text
    };
    let back = Mat::Canvas((seed % 6) as u8);
    let ink = if seed % 6 == 3 {
        Mat::Canvas(1)
    } else {
        Mat::White
    };
    for x in [-4.0f32, 4.0] {
        out.push(round(Vec3::new(x, 0.0, -0.6), 6.0, 0.3, 0.25, Mat::Metal));
    }
    out.push(bx(
        Vec3::new(0.0, 6.0, -0.6),
        Vec3::new(7.0, 4.4, 0.25),
        back,
    ));
    let step = (12.0 / (words.len() as f32 * 4.0)).min(0.42);
    spell(
        out,
        words,
        Vec3::new(0.0, 8.2 - step * 2.5, -0.33),
        step,
        0.06,
        ink,
        false,
    );
    out.push(soft(bx(
        Vec3::new(0.0, 5.7, 0.2),
        Vec3::new(7.0, 0.08, 0.6),
        Mat::Metal,
    )));
    for x in [-4.5f32, 0.0, 4.5] {
        out.push(soft(bx(
            Vec3::new(x, 10.5, 0.4),
            Vec3::new(0.3, 0.15, 0.3),
            Mat::Lamp,
        )));
    }
}

/// A concrete gravity dam across a gorge: a tall wall with a road along its crest between
/// parapets and lamps, its downstream face stepping out toward the foot, a spillway down
/// the middle (its water drawn by the client), a valve tower standing in the reservoir
/// with a footbridge out to it, and a control house at one end.
fn dam(out: &mut Vec<Piece>) {
    let from = out.len();
    let top = DAM_TALL;
    out.push(bx(
        Vec3::new(0.0, 0.0, -4.0),
        Vec3::new(66.0, top, 6.0),
        Mat::Concrete,
    ));
    for (k, (z, half, h)) in [
        (5.0f32, 64.0f32, 0.72f32),
        (12.0, 60.0, 0.4),
        (19.0, 56.0, 0.18),
    ]
    .into_iter()
    .enumerate()
    {
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(half, top * h, 4.0),
            Mat::Concrete,
        ));
        // Foam down the spillway over each step, between its walls.
        out.push(soft(bx(
            Vec3::new(0.0, top * h, z - 0.2 + k as f32 * 0.01),
            Vec3::new(4.2, 0.3, 4.05),
            Mat::White,
        )));
    }
    // The face: buttress ribs down each step, lift lines where each pour of concrete
    // met the next, and at the foot two outlets letting water out in white plumes.
    let faces = [
        (2.0f32, top * 0.72, top),
        (9.0, top * 0.4, top * 0.72),
        (16.0, top * 0.18, top * 0.4),
        (23.0, 0.0, top * 0.18),
    ];
    for (k, (z, lo, hi)) in faces.into_iter().enumerate() {
        let wide = 64.0 - k.saturating_sub(1) as f32 * 4.0;
        for x in [-0.8f32, -0.55, -0.3, 0.3, 0.55, 0.8] {
            out.push(bx(
                Vec3::new(x * wide, lo, z + 0.65),
                Vec3::new(1.4, hi - lo - 0.2, 0.6),
                Mat::Stone,
            ));
        }
        let mut y = lo + 9.0;
        while y < hi - 3.0 {
            out.push(bx(
                Vec3::new(0.0, y, z + 0.2),
                Vec3::new(wide - 1.0, 0.4, 0.25),
                Mat::Stone,
            ));
            y += 9.0;
        }
    }
    for x in [-26.0f32, 26.0] {
        out.push(bx(
            Vec3::new(x, 1.0, 23.1),
            Vec3::new(3.2, 6.0, 0.3),
            Mat::Dark,
        ));
        out.push(soft(bx(
            Vec3::new(x, 0.0, 26.0),
            Vec3::new(2.6, 3.5, 3.0),
            Mat::White,
        )));
    }
    // The spillway's walls, down the face.
    for x in [-5.0f32, 5.0] {
        out.push(bx(
            Vec3::new(x, 0.0, 9.0),
            Vec3::new(0.6, top + 1.0, 13.0),
            Mat::Stone,
        ));
    }
    // The crest road, its parapets and lamps.
    out.push(soft(bx(
        Vec3::new(0.0, top, -4.0),
        Vec3::new(66.0, 0.15, 5.6),
        Mat::Asphalt,
    )));
    for z in [-9.6f32, 1.6] {
        out.push(bx(
            Vec3::new(0.0, top, z),
            Vec3::new(66.0, 1.1, 0.3),
            Mat::Concrete,
        ));
    }
    for k in 0..11 {
        let x = -60.0 + k as f32 * 12.0;
        out.push(soft(round(
            Vec3::new(x, top + 1.1, 1.6),
            4.5,
            0.12,
            0.1,
            Mat::Metal,
        )));
        out.push(soft(ball(Vec3::new(x, top + 5.7, 1.6), 0.35, Mat::Lamp)));
    }
    // The valve tower out in the water, and its footbridge.
    let t = Vec3::new(28.0, 0.0, -26.0);
    out.push(round(t, top + 2.0, 4.5, 4.0, Mat::Concrete));
    out.push(Piece {
        form: Form::Box {
            at: t + Vec3::Y * (top + 2.0),
            half: Vec3::new(4.6, 3.0, 4.6),
            yaw: 0.0,
            roof: 2.2,
        },
        mat: Mat::White,
        solid: true,
    });
    out.push(soft(bx(
        t + Vec3::new(0.0, top + 3.0, 4.65),
        Vec3::new(1.0, 1.0, 0.05),
        Mat::Lamp,
    )));
    out.push(bx(
        Vec3::new(t.x, top, (t.z - 9.5) * 0.5 - 2.0),
        Vec3::new(1.4, 0.4, (t.z.abs() - 9.5) * 0.5 - 2.0),
        Mat::Concrete,
    ));
    // The control house at the end of the crest.
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-56.0, top, -4.0),
            half: Vec3::new(6.0, 4.0, 4.5),
            yaw: 0.0,
            roof: 1.5,
        },
        mat: Mat::White,
        solid: true,
    });
    for x in [-59.0f32, -53.0] {
        out.push(soft(bx(
            Vec3::new(x, top + 1.5, 0.55),
            Vec3::new(1.2, 1.0, 0.05),
            Mat::Lamp,
        )));
    }
    // The dam's frame sits at its foot downstream, where the ground is near its height:
    // the wall stands back up the gorge from there.
    for p in &mut out[from..] {
        match &mut p.form {
            Form::Box { at, .. }
            | Form::Round { at, .. }
            | Form::Ball { at, .. }
            | Form::Letter { at, .. } => {
                at.z -= DAM_BACK;
            }
        }
    }
}

/// A happy castaway's camp on a tiny island: a thatched lean-to, a beach umbrella over
/// a deck chair, a ring of stones round a fire, a flag on a driftwood pole, a treasure
/// chest half dug in, a bottle with a message, and SOS laid out in white pebbles.
fn castaway(out: &mut Vec<Piece>) {
    // The lean-to: four poles, a thatch roof, and a bed of leaves under it.
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let h = if z > 0.0 { 2.4 } else { 1.6 };
        out.push(round(
            Vec3::new(x * 1.8, 0.0, z * 1.4 - 4.0),
            h,
            0.12,
            0.1,
            Mat::Wood,
        ));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 2.0, -4.0),
            half: Vec3::new(2.4, 0.15, 2.0),
            yaw: 0.0,
            roof: 0.6,
        },
        mat: Mat::Hay,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, -4.0),
        Vec3::new(1.4, 0.2, 1.0),
        Mat::Leaf,
    )));
    // The umbrella and its deck chair.
    let shade = Vec3::new(5.0, 0.0, 1.0);
    out.push(soft(round(shade, 2.4, 0.05, 0.05, Mat::White)));
    out.push(soft(round(
        shade + Vec3::Y * 2.0,
        0.6,
        1.8,
        0.05,
        Mat::Canvas(0),
    )));
    out.push(soft(turned(
        shade + Vec3::new(0.6, 0.25, 0.0),
        Vec3::new(0.9, 0.06, 0.35),
        0.3,
        Mat::Canvas(2),
    )));
    // The fire, ringed with stones, a log to sit on beside it.
    let fire = Vec3::new(0.0, 0.0, 2.5);
    for k in 0..7 {
        let a = k as f32 / 7.0 * std::f32::consts::TAU;
        out.push(soft(ball(
            fire + Vec3::new(a.cos() * 0.8, 0.12, a.sin() * 0.8),
            0.2,
            Mat::Stone,
        )));
    }
    out.push(soft(round(fire, 0.4, 0.3, 0.05, Mat::Lamp)));
    out.push(soft(turned(
        fire + Vec3::new(-2.0, 0.0, 0.3),
        Vec3::new(1.0, 0.35, 0.3),
        0.4,
        Mat::Wood,
    )));
    // A flag on a driftwood pole, for passing ships.
    let pole = Vec3::new(-4.5, 0.0, 0.5);
    out.push(round(pole, 5.0, 0.1, 0.08, Mat::Wood));
    out.push(soft(bx(
        pole + Vec3::new(0.7, 4.0, 0.0),
        Vec3::new(0.7, 0.45, 0.03),
        Mat::Red,
    )));
    // The chest, its lid open a crack, and the bottle washed up by the water.
    let chest = Vec3::new(2.5, 0.0, -1.0);
    out.push(bx(chest, Vec3::new(0.6, 0.5, 0.4), Mat::Wood));
    out.push(soft(bx(
        chest + Vec3::Y * 0.5,
        Vec3::new(0.62, 0.06, 0.42),
        Mat::Copper,
    )));
    out.push(soft(round(
        Vec3::new(7.5, 0.0, 4.0),
        0.4,
        0.1,
        0.05,
        Mat::Glass,
    )));
    // SOS in pebbles on the sand, big enough to read from the air.
    let cell = 0.9;
    for (i, c) in "SOS".chars().enumerate() {
        let rows = glyph(c);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..3u32 {
                if bits & (0b100 >> col) != 0 {
                    out.push(soft(ball(
                        Vec3::new(
                            (i as f32 * 4.0 + col as f32 - 5.0) * cell,
                            0.08,
                            8.0 + row as f32 * cell,
                        ),
                        0.3,
                        Mat::White,
                    )));
                }
            }
        }
    }
}

/// A beach surf shack on short stilts: a thatched hut with a counter and stools, surfboards
/// standing in the sand, and striped umbrellas over loungers.
fn surf_shack(out: &mut Vec<Piece>) {
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(soft(round(
            Vec3::new(x * 3.6, -1.0, z * 2.6),
            2.0,
            0.2,
            0.2,
            Mat::Wood,
        )));
    }
    out.push(bx(Vec3::Y, Vec3::new(4.0, 0.3, 3.0), Mat::Wood));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 1.3, -0.8),
            half: Vec3::new(3.6, 2.6, 2.0),
            yaw: 0.0,
            roof: 0.0,
        },
        mat: Mat::Wood,
        solid: true,
    });
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 3.9, 0.0),
            half: Vec3::new(4.6, 0.2, 3.8),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::Hay,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 1.3, 1.3),
        Vec3::new(3.0, 1.1, 0.25),
        Mat::Wood,
    )));
    for k in 0..4 {
        out.push(soft(round(
            Vec3::new(-2.4 + k as f32 * 1.6, 1.3, 2.4),
            0.75,
            0.25,
            0.25,
            Mat::Canvas(k as u8),
        )));
    }
    for k in 0..5 {
        out.push(soft(Piece {
            form: Form::Box {
                at: Vec3::new(6.0 + k as f32 * 1.1, -0.5, -2.0 + (k % 2) as f32 * 0.4),
                half: Vec3::new(0.3, 2.6, 0.08),
                yaw: 0.2 * (k as f32 - 2.0),
                roof: 0.0,
            },
            mat: Mat::Canvas((k + 1) as u8),
            solid: false,
        }));
    }
    for k in 0..3 {
        let at = Vec3::new(-8.0 - k as f32 * 4.5, 0.0, 4.0);
        out.push(soft(round(at, 2.6, 0.05, 0.05, Mat::White)));
        out.push(soft(round(
            at + Vec3::Y * 2.6,
            0.8,
            2.0,
            0.1,
            Mat::Canvas((k * 2) as u8),
        )));
        out.push(soft(bx(
            at + Vec3::new(0.0, 0.0, 1.6),
            Vec3::new(0.6, 0.3, 1.0),
            Mat::White,
        )));
    }
}

/// A rope bridge from the origin along x, `text` giving its length and how far its far
/// end rises (or falls): planks hung in a sag, rope rails either side, and stout posts at
/// both ends. The planks are solid, so it can be walked.
fn rope_bridge(out: &mut Vec<Piece>, text: &str) {
    let mut nums = text
        .split_whitespace()
        .filter_map(|n| n.parse::<f32>().ok());
    let (len, rise) = (nums.next().unwrap_or(60.0), nums.next().unwrap_or(0.0));
    let sag = len * 0.05;
    let y = |x: f32| {
        let u = (x / len).clamp(0.0, 1.0);
        rise * u - sag * 4.0 * u * (1.0 - u)
    };
    let n = (len / 1.4) as u32;
    for k in 0..=n {
        let x = k as f32 / n as f32 * len;
        out.push(bx(
            Vec3::new(x, y(x) - 0.15, 0.0),
            Vec3::new(0.55, 0.15, 1.1),
            Mat::Wood,
        ));
    }
    let segs = (len / 6.0).ceil() as u32;
    for k in 0..segs {
        let (a, b) = (
            k as f32 / segs as f32 * len,
            (k + 1) as f32 / segs as f32 * len,
        );
        for z in [-1.2f32, 1.2] {
            for lift in [0.1f32, 1.1] {
                let (pa, pb) = (y(a) + lift, y(b) + lift);
                // Rails as short level pieces stepping with the sag.
                out.push(soft(bx(
                    Vec3::new((a + b) * 0.5, (pa + pb) * 0.5 - 0.03, z),
                    Vec3::new((b - a) * 0.5, 0.06, 0.04),
                    Mat::Hay,
                )));
            }
        }
    }
    for (x, top) in [(-0.6f32, 0.0f32), (len + 0.6, rise)] {
        for z in [-1.3f32, 1.3] {
            out.push(round(
                Vec3::new(x, top - 1.0, z),
                2.6,
                0.22,
                0.18,
                Mat::Wood,
            ));
        }
    }
}

/// A market's stalls in its frame: where each stands and which way its counter faces
/// (+1 toward +z, -1 toward -z). Shoppers walk the lane between the rows.
pub const STALLS: [(Vec2, f32); 8] = [
    (Vec2::new(-15.0, -7.0), 1.0),
    (Vec2::new(-5.0, -7.0), 1.0),
    (Vec2::new(5.0, -7.0), 1.0),
    (Vec2::new(15.0, -7.0), 1.0),
    (Vec2::new(-15.0, 7.0), -1.0),
    (Vec2::new(-5.0, 7.0), -1.0),
    (Vec2::new(5.0, 7.0), -1.0),
    (Vec2::new(15.0, 7.0), -1.0),
];

/// A weir across a lake's outlet, its top at the origin along x: a concrete wall down to
/// the bed, a walkway with rails along the top, two sluice gates in their frames, and a
/// gatehouse at one end.
fn weir(out: &mut Vec<Piece>) {
    let half = 24.0;
    out.push(bx(
        Vec3::new(0.0, -14.0, 0.0),
        Vec3::new(half, 14.6, 1.8),
        Mat::Concrete,
    ));
    for z in [-1.7f32, 1.7] {
        out.push(soft(bx(
            Vec3::new(0.0, 0.6, z),
            Vec3::new(half, 1.0, 0.05),
            Mat::Metal,
        )));
    }
    for x in [-6.0f32, 6.0] {
        for s in [-1.0f32, 1.0] {
            out.push(bx(
                Vec3::new(x + s * 2.6, 0.6, 0.0),
                Vec3::new(0.4, 4.0, 1.2),
                Mat::Concrete,
            ));
        }
        out.push(bx(
            Vec3::new(x, 3.8, 0.0),
            Vec3::new(3.0, 0.8, 1.2),
            Mat::Concrete,
        ));
        out.push(soft(bx(
            Vec3::new(x, -1.0, 1.9),
            Vec3::new(2.2, 3.0, 0.1),
            Mat::Metal,
        )));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(half + 3.0, 0.0, 0.0),
            half: Vec3::new(3.0, 3.2, 3.0),
            yaw: 0.0,
            roof: 1.4,
        },
        mat: Mat::Stone,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(half + 0.05, 1.0, 0.0),
        Vec3::new(0.06, 1.8, 0.8),
        Mat::Lamp,
    )));
}

/// Two steel pipes from the origin down a slope along x, `text` giving how far along and
/// how far down they run: upright lengths stepping down and out, steep enough to read as
/// pipes, with concrete anchor blocks holding them every so often.
fn penstock(out: &mut Vec<Piece>, text: &str) {
    let mut nums = text
        .split_whitespace()
        .filter_map(|n| n.parse::<f32>().ok());
    let (len, drop) = (nums.next().unwrap_or(60.0), nums.next().unwrap_or(150.0));
    let n = (drop / 3.0).ceil().max(1.0) as u32;
    for z in [-1.6f32, 1.6] {
        for k in 0..n {
            let u = (k as f32 + 0.5) / n as f32;
            let at = Vec3::new(len * u, -drop * (k + 1) as f32 / n as f32, z);
            out.push(soft(round(at, drop / n as f32 + 0.4, 1.1, 1.1, Mat::Metal)));
            if k % 8 == 4 {
                out.push(bx(
                    at - Vec3::Y * 0.5,
                    Vec3::new(1.8, 2.6, 2.2),
                    Mat::Concrete,
                ));
            }
        }
    }
    out.push(bx(
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::new(3.0, 3.0, 4.0),
        Mat::Concrete,
    ));
}

/// Where a hydro house's water leaves it, in its frame: the tailrace runs out along +z.
pub const TAILRACE: Vec3 = Vec3::new(0.0, 0.0, 12.0);

/// A hydro power house at the foot of its penstocks: a long hall with tall windows and a
/// pitched roof, the pipes' end blocks behind it, the tailrace running out under an arch
/// in front, and a fenced yard of transformers with tall insulators.
fn hydro(out: &mut Vec<Piece>) {
    out.push(Piece {
        form: Form::Box {
            at: Vec3::ZERO,
            half: Vec3::new(14.0, 11.0, 8.0),
            yaw: 0.0,
            roof: 3.5,
        },
        mat: Mat::Stone,
        solid: true,
    });
    for k in 0..5 {
        let x = -10.0 + k as f32 * 5.0;
        out.push(soft(bx(
            Vec3::new(x, 3.0, 8.05),
            Vec3::new(1.4, 5.0, 0.08),
            Mat::Glass,
        )));
    }
    out.push(bx(
        Vec3::new(0.0, 0.0, -10.0),
        Vec3::new(6.0, 4.0, 2.4),
        Mat::Concrete,
    ));
    out.push(soft(bx(TAILRACE, Vec3::new(4.0, 0.2, 4.0), Mat::Water)));
    out.push(bx(
        TAILRACE + Vec3::new(0.0, 2.6, -3.5),
        Vec3::new(5.0, 0.8, 0.6),
        Mat::Concrete,
    ));
    // The transformer yard, beside the hall.
    let yard = Vec3::new(24.0, 0.0, 0.0);
    out.push(soft(bx(yard, Vec3::new(8.0, 0.08, 7.0), Mat::Sand)));
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(soft(round(
            yard + Vec3::new(x * 8.0, 0.0, z * 7.0),
            2.2,
            0.08,
            0.08,
            Mat::Metal,
        )));
    }
    for s in [-1.0f32, 1.0] {
        out.push(soft(bx(
            yard + Vec3::new(0.0, 0.0, s * 7.0),
            Vec3::new(8.0, 2.0, 0.04),
            Mat::Metal,
        )));
        out.push(soft(bx(
            yard + Vec3::new(s * 8.0, 0.0, 0.0),
            Vec3::new(0.04, 2.0, 7.0),
            Mat::Metal,
        )));
    }
    for x in [-4.0f32, 0.0, 4.0] {
        out.push(bx(
            yard + Vec3::new(x, 0.0, -2.0),
            Vec3::new(1.4, 2.4, 1.0),
            Mat::Dark,
        ));
        for k in 0..3 {
            out.push(soft(round(
                yard + Vec3::new(x - 0.8 + k as f32 * 0.8, 2.4, -2.0),
                1.6,
                0.12,
                0.08,
                Mat::White,
            )));
        }
        out.push(soft(round(
            yard + Vec3::new(x, 0.0, 3.0),
            7.0,
            0.15,
            0.12,
            Mat::Metal,
        )));
    }
    out.push(soft(bx(
        Vec3::new(14.05, 2.0, 0.0),
        Vec3::new(0.06, 2.2, 1.6),
        Mat::Lamp,
    )));
}

/// A racetrack's figure eight in its frame: how far each loop reaches out along x, how
/// far out along z, and half the track's width.
pub const RACE_OVAL: (f32, f32, f32) = (150.0, 75.0, 9.0);
/// How high the track climbs to cross itself on the overpass in the middle (m).
pub const RACE_BRIDGE: f32 = 8.5;

/// Where `u` of the way round the figure eight is, along the curve: the start line sits
/// on the left loop's near side, in front of the grandstand.
fn race_t(u: f32) -> f32 {
    u.rem_euclid(1.0) * std::f32::consts::TAU - 0.41
}

/// A point `u` of the way round a racetrack's figure eight, `out` meters to the left of
/// its middle line, with the way it runs there. Two loops, a tight bend at the top and
/// bottom of each and a fast sweep round each end, crossing in the middle.
pub fn race_line(u: f32, out: f32) -> (Vec2, Vec2) {
    let (l, r, _) = RACE_OVAL;
    let t = race_t(u);
    let p = Vec2::new(l * t.sin(), r * (2.0 * t).sin());
    let d = Vec2::new(l * t.cos(), 2.0 * r * (2.0 * t).cos()).normalize_or(Vec2::X);
    (p + d.perp() * out, d)
}

/// How high the track runs at `u`: on the ground, but for the overpass where the run from
/// the right loop to the left one climbs over the other.
pub fn race_height(u: f32) -> f32 {
    let t = race_t(u) - std::f32::consts::PI;
    let t = (t + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let x = (t.abs() / 0.5).min(1.0);
    RACE_BRIDGE * (1.0 - x * x * (3.0 - 2.0 * x))
}

/// A motor racing circuit: a figure eight of tarmac with red and white kerbs and low walls
/// both sides, crossing itself on an overpass in the middle, mown infield, a chequered
/// start line, a long grandstand down the near side under a roof, and the pit garages
/// along the far one.
fn racetrack(out: &mut Vec<Piece>) {
    let (l, r, hw) = RACE_OVAL;
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(l + 10.0, 0.08, r + 6.0),
        Mat::Grass,
    )));
    // Round the circuit in short straight pieces, each turned to the line.
    let n = 160;
    for k in 0..n {
        let (u0, u1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
        let (a, _) = race_line(u0, 0.0);
        let (b, _) = race_line(u1, 0.0);
        let y = (race_height(u0) + race_height(u1)) * 0.5;
        let mid = (a + b) * 0.5;
        let d = b - a;
        let yaw = (-d.y).atan2(d.x);
        let len = d.length() * 0.5 + 0.6;
        let up = y > 0.5;
        let turned = |at: Vec3, half: Vec3, mat: Mat, solid: bool| Piece {
            form: Form::Box {
                at,
                half,
                yaw,
                roof: 0.0,
            },
            mat,
            solid,
        };
        // The tarmac is drawn by the client, smooth over the overpass. Up there a hidden
        // deck holds you up.
        if up {
            out.push(turned(
                Vec3::new(mid.x, y - 0.6, mid.y),
                Vec3::new(len, 0.72, hw + 0.4),
                Mat::Hidden,
                true,
            ));
        }
        let side = Vec2::new(-d.y, d.x).normalize_or(Vec2::Y);
        // Kerbs on the bends, both sides, alternating red and white a hair apart in height
        // so where they overlap neither flickers.
        let bent = {
            let (c, _) = race_line(u1 + 1.0 / n as f32, 0.0);
            (c - b).normalize_or(Vec2::X).dot(d.normalize_or(Vec2::X)) < 0.995
        };
        if bent && !up {
            for s in [-1.0f32, 1.0] {
                let at = mid + side * s * (hw - 0.6);
                out.push(turned(
                    Vec3::new(at.x, 0.0, at.y),
                    Vec3::new(len, if k % 2 == 0 { 0.16 } else { 0.18 }, 0.6),
                    if k % 2 == 0 { Mat::Red } else { Mat::White },
                    false,
                ));
            }
        }
        // Walls both sides, but not where the low run passes under the overpass.
        let under = !up && mid.length() < 40.0;
        if !under {
            for s in [-1.0f32, 1.0] {
                let at = mid + side * s * (hw + 0.6);
                out.push(turned(
                    Vec3::new(at.x, y, at.y),
                    Vec3::new(len, 1.0, 0.3),
                    if up { Mat::Hidden } else { Mat::Concrete },
                    true,
                ));
            }
        }
        // Piers right up under the deck.
        if up && k % 4 == 0 && y > 3.0 {
            out.push(round(
                Vec3::new(mid.x, 0.0, mid.y),
                y - 0.8,
                1.4,
                1.2,
                Mat::Concrete,
            ));
        }
    }
    // The start line, chequered across the track.
    let (s0, d0) = race_line(0.0, 0.0);
    let yaw = (-d0.y).atan2(d0.x);
    for k in 0..9 {
        for j in 0..2 {
            let at = s0 + d0 * (-0.5 + j as f32) + d0.perp() * (-hw + 1.0 + k as f32 * 2.0);
            out.push(soft(Piece {
                form: Form::Box {
                    at: Vec3::new(at.x, 0.13, at.y),
                    half: Vec3::new(0.5, 0.02, 1.0),
                    yaw,
                    roof: 0.0,
                },
                mat: if (k + j) % 2 == 0 {
                    Mat::White
                } else {
                    Mat::Dark
                },
                solid: false,
            }));
        }
    }
    // The grandstand down the start straight, tiers stepping up and back under a roof.
    let back = -r - hw - 4.0;
    for t in 0..5 {
        let z = back - t as f32 * 2.4;
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(70.0, 1.2 + t as f32 * 1.4, 1.2),
            Mat::Canvas((t % 2) as u8 * 2 + 1),
        ));
    }
    out.push(bx(
        Vec3::new(0.0, 0.0, back - 13.0),
        Vec3::new(70.0, 13.0, 0.6),
        Mat::Concrete,
    ));
    out.push(bx(
        Vec3::new(0.0, 13.0, back - 6.5),
        Vec3::new(71.0, 0.5, 7.5),
        Mat::White,
    ));
    for x in [-60.0f32, -30.0, 0.0, 30.0, 60.0] {
        out.push(soft(round(
            Vec3::new(x, 0.0, back - 12.0),
            13.0,
            0.3,
            0.3,
            Mat::Metal,
        )));
    }
    // The pits and their garages along the far straight, a tower over the middle.
    let pits = r + hw + 8.0;
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, r + hw + 3.0),
        Vec3::new(90.0, 0.1, 3.0),
        Mat::Asphalt,
    )));
    out.push(bx(
        Vec3::new(0.0, 0.0, pits + 4.0),
        Vec3::new(80.0, 5.0, 4.0),
        Mat::White,
    ));
    for k in 0..12 {
        let x = -74.0 + k as f32 * (148.0 / 11.0);
        out.push(soft(bx(
            Vec3::new(x, 0.0, pits - 0.05),
            Vec3::new(4.5, 3.8, 0.1),
            Mat::Canvas((k % 6) as u8),
        )));
    }
    out.push(bx(
        Vec3::new(0.0, 5.0, pits + 4.0),
        Vec3::new(5.0, 8.0, 4.0),
        Mat::Glass,
    ));
    out.push(bx(
        Vec3::new(0.0, 13.0, pits + 4.0),
        Vec3::new(5.6, 0.6, 4.6),
        Mat::Red,
    ));
}

/// A police station: a brick two storey block with a flat roof, a blue lamp over the
/// door, its name above it, and a row of parking bays with patrol cars in them.
fn police(out: &mut Vec<Piece>) {
    out.push(bx(Vec3::ZERO, Vec3::new(14.0, 8.0, 8.0), Mat::Red));
    out.push(bx(Vec3::Y * 8.0, Vec3::new(14.3, 0.5, 8.3), Mat::Concrete));
    for row in [2.5f32, 5.8] {
        for k in 0..6 {
            out.push(soft(bx(
                Vec3::new(-10.5 + k as f32 * 4.2, row - 1.0, 8.05),
                Vec3::new(1.2, 1.0, 0.08),
                Mat::Glass,
            )));
        }
    }
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 8.05),
        Vec3::new(1.4, 2.6, 0.1),
        Mat::Glass,
    )));
    out.push(soft(bx(
        Vec3::new(0.0, 3.0, 8.4),
        Vec3::new(0.5, 0.5, 0.4),
        Mat::Canvas(1),
    )));
    spell(
        out,
        "POLICE",
        Vec3::new(0.0, 6.6, 8.2),
        0.22,
        0.05,
        Mat::White,
        false,
    );
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 15.0),
        Vec3::new(14.0, 0.08, 6.0),
        Mat::Asphalt,
    )));
    for k in 0..4 {
        let x = -9.0 + k as f32 * 6.0;
        out.push(soft(bx(
            Vec3::new(x + 3.0, 0.1, 15.0),
            Vec3::new(0.08, 0.02, 2.6),
            Mat::White,
        )));
        if k != 2 {
            out.push(bx(
                Vec3::new(x, 0.0, 15.0),
                Vec3::new(1.0, 1.4, 2.2),
                Mat::White,
            ));
            out.push(soft(bx(
                Vec3::new(x, 1.0, 15.0),
                Vec3::new(1.02, 0.15, 2.22),
                Mat::Canvas(1),
            )));
            out.push(soft(bx(
                Vec3::new(x, 1.4, 14.6),
                Vec3::new(0.4, 0.12, 0.2),
                Mat::Lamp,
            )));
        }
    }
}

/// A gas station: a flat canopy on posts over two islands of pumps, a little shop with
/// a glass front, and a tall price sign by the road.
fn gas_station(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 4.0),
        Vec3::new(14.0, 0.1, 10.0),
        Mat::Concrete,
    )));
    out.push(bx(
        Vec3::new(0.0, 5.0, 6.0),
        Vec3::new(10.0, 0.8, 6.0),
        Mat::White,
    ));
    out.push(soft(bx(
        Vec3::new(0.0, 5.0, 6.0),
        Vec3::new(10.1, 0.25, 6.1),
        Mat::Red,
    )));
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(soft(round(
            Vec3::new(x * 8.0, 0.0, 6.0 + z * 4.0),
            5.0,
            0.3,
            0.3,
            Mat::White,
        )));
    }
    for x in [-4.0f32, 4.0] {
        out.push(bx(
            Vec3::new(x, 0.0, 6.0),
            Vec3::new(0.6, 0.25, 3.0),
            Mat::Concrete,
        ));
        for z in [4.5f32, 7.5] {
            out.push(bx(
                Vec3::new(x, 0.25, z),
                Vec3::new(0.4, 1.5, 0.3),
                Mat::Red,
            ));
            out.push(soft(bx(
                Vec3::new(x, 1.1, z + 0.31),
                Vec3::new(0.25, 0.3, 0.02),
                Mat::Lamp,
            )));
        }
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 0.0, -6.0),
            half: Vec3::new(7.0, 3.6, 3.5),
            yaw: 0.0,
            roof: 0.0,
        },
        mat: Mat::White,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.3, -2.45),
        Vec3::new(5.0, 2.6, 0.08),
        Mat::Glass,
    )));
    out.push(soft(round(
        Vec3::new(12.0, 0.0, 13.0),
        8.0,
        0.2,
        0.2,
        Mat::Metal,
    )));
    out.push(bx(
        Vec3::new(12.0, 8.0, 13.0),
        Vec3::new(1.6, 3.0, 0.3),
        Mat::Red,
    ));
    out.push(soft(bx(
        Vec3::new(12.0, 8.6, 13.32),
        Vec3::new(1.2, 1.8, 0.02),
        Mat::Lamp,
    )));
}

/// An amphitheater's stage in its frame, and its seats: tiers round a half bowl facing
/// it, from `inner` to `outer` radius, each `rise` higher.
pub const STAGE: Vec3 = Vec3::new(0.0, 1.0, -6.0);
pub const AMPHI_SEATS: (f32, f32, f32) = (12.0, 30.0, 0.9);

/// An open air theater: tiers of stone seats in a half bowl facing a raised stage, a
/// white shell arching over the back of the stage, and lamps along the aisles.
fn amphitheater(out: &mut Vec<Piece>) {
    let (inner, outer, rise) = AMPHI_SEATS;
    let rows = ((outer - inner) / 1.8) as u32;
    for k in 0..rows {
        let rad = inner + k as f32 * 1.8;
        let n = 14;
        for j in 0..n {
            let a = std::f32::consts::PI * (j as f32 + 0.5) / n as f32;
            let at = Vec3::new(a.cos() * rad, 0.0, a.sin() * rad);
            let len = std::f32::consts::PI * rad / n as f32 * 0.5 + 0.2;
            out.push(Piece {
                form: Form::Box {
                    at,
                    half: Vec3::new(0.9, rise * (k + 1) as f32, len),
                    yaw: -a,
                    roof: 0.0,
                },
                mat: Mat::Stone,
                solid: true,
            });
        }
    }
    out.push(bx(STAGE - Vec3::Y, Vec3::new(10.0, 1.0, 5.0), Mat::Wood));
    for k in 0..9 {
        let a = std::f32::consts::PI * k as f32 / 8.0;
        out.push(Piece {
            form: Form::Box {
                at: STAGE + Vec3::new(a.cos() * 10.0, 0.0, -5.0 - a.sin() * 2.0),
                half: Vec3::new(2.0, 6.0 + a.sin() * 4.0, 0.3),
                yaw: -a * 0.3,
                roof: 0.0,
            },
            mat: Mat::White,
            solid: true,
        });
    }
    for s in [-1.0f32, 1.0] {
        out.push(soft(round(
            STAGE + Vec3::new(s * 9.5, 0.0, 4.5),
            5.0,
            0.12,
            0.12,
            Mat::Metal,
        )));
        out.push(soft(bx(
            STAGE + Vec3::new(s * 9.5, 5.0, 4.5),
            Vec3::splat(0.4),
            Mat::Lamp,
        )));
    }
}

/// A solar farm: rows of panels tilted to the sun on posts, a fence round it all and an
/// inverter hut by the gate.
fn solar_farm(out: &mut Vec<Piece>) {
    for row in 0..8 {
        let z = -28.0 + row as f32 * 8.0;
        out.push(Piece {
            form: Form::Box {
                at: Vec3::new(0.0, 1.2, z),
                half: Vec3::new(40.0, 0.08, 1.8),
                yaw: 0.0,
                roof: 0.9,
            },
            mat: Mat::Dark,
            solid: false,
        });
        for k in 0..9 {
            out.push(soft(round(
                Vec3::new(-36.0 + k as f32 * 9.0, 0.0, z),
                1.2,
                0.08,
                0.08,
                Mat::Metal,
            )));
        }
    }
    for (at, half) in [
        (Vec3::new(0.0, 0.0, 34.0), Vec3::new(44.0, 1.8, 0.04)),
        (Vec3::new(0.0, 0.0, -34.0), Vec3::new(44.0, 1.8, 0.04)),
        (Vec3::new(44.0, 0.0, 0.0), Vec3::new(0.04, 1.8, 34.0)),
        (Vec3::new(-44.0, 0.0, 0.0), Vec3::new(0.04, 1.8, 34.0)),
    ] {
        out.push(soft(bx(at, half, Mat::Metal)));
    }
    out.push(bx(
        Vec3::new(40.0, 0.0, 30.0),
        Vec3::new(2.5, 2.6, 2.0),
        Mat::Concrete,
    ));
}

/// A golf course's holes in its frame: tee, green and their bunkers, played in order.
pub const GOLF_HOLES: [(Vec2, Vec2); 4] = [
    (Vec2::new(-150.0, -60.0), Vec2::new(-20.0, -90.0)),
    (Vec2::new(10.0, -70.0), Vec2::new(150.0, -40.0)),
    (Vec2::new(140.0, 20.0), Vec2::new(20.0, 70.0)),
    (Vec2::new(-10.0, 60.0), Vec2::new(-140.0, 50.0)),
];

/// A golf course, landscaped: four holes, each a fairway that doglegs from a raised tee to
/// a raised round green with a red flag on its pin, kidney bunkers round the green and on
/// the corner, grassy mounds and lines of trees down both sides of the rough, a pond in
/// the middle with reeds, golf carts by the clubhouse and a putting green beside it.
fn golf(out: &mut Vec<Piece>, seed: u32) {
    let r = |k: u32| crate::shape::hash(seed.wrapping_add(k.wrapping_mul(2_654_435_761)));
    // The pond in the middle, which mounds, trees and bunkers keep out of.
    let pond = Vec2::new(70.0, 5.0);
    let dry = |at: Vec2, reach: f32| at.distance(pond) > 24.0 + reach;
    let lawn = |out: &mut Vec<Piece>, a: Vec2, b: Vec2, half: f32| {
        let d = b - a;
        let mid = (a + b) * 0.5;
        out.push(Piece {
            form: Form::Box {
                at: Vec3::new(mid.x, 0.0, mid.y),
                half: Vec3::new(d.length() * 0.5, 0.06, half),
                yaw: (-d.y).atan2(d.x),
                roof: 0.0,
            },
            mat: Mat::Grass,
            solid: false,
        });
    };
    let bunker = |out: &mut Vec<Piece>, at: Vec2, a: f32| {
        let off = Vec2::new(a.cos(), a.sin()) * 3.0;
        if !dry(at, 8.0) {
            return;
        }
        for (p, rad) in [(at - off, 5.0f32), (at + off, 4.0)] {
            out.push(soft(round(
                Vec3::new(p.x, 0.0, p.y),
                0.1,
                rad,
                rad - 0.4,
                Mat::Sand,
            )));
        }
    };
    let tree = |out: &mut Vec<Piece>, at: Vec2, k: u32| {
        if !dry(at, 4.0) {
            return;
        }
        let tall = 5.0 + r(k) * 4.0;
        out.push(round(Vec3::new(at.x, 0.0, at.y), tall, 0.4, 0.3, Mat::Wood));
        out.push(ball(
            Vec3::new(at.x, tall + 1.5, at.y),
            3.0 + r(k + 1) * 1.5,
            Mat::Leaf,
        ));
    };
    for (k, (tee, green)) in GOLF_HOLES.into_iter().enumerate() {
        let k = k as u32;
        let d = (green - tee).normalize_or(Vec2::X);
        let side = d.perp();
        let bend = (r(k * 11) - 0.5) * 50.0;
        let corner = tee.lerp(green, 0.55) + side * bend;
        lawn(out, tee, corner, 13.0);
        lawn(out, corner, green, 11.0);
        out.push(soft(round(
            Vec3::new(corner.x, 0.0, corner.y),
            0.06,
            13.0,
            13.0,
            Mat::Grass,
        )));
        out.push(round(
            Vec3::new(tee.x, 0.0, tee.y),
            0.6,
            6.0,
            5.0,
            Mat::Grass,
        ));
        out.push(round(
            Vec3::new(green.x, 0.0, green.y),
            0.5,
            16.0,
            14.0,
            Mat::Hedge,
        ));
        out.push(soft(round(
            Vec3::new(green.x, 0.5, green.y),
            3.2,
            0.05,
            0.05,
            Mat::White,
        )));
        out.push(soft(bx(
            Vec3::new(green.x + 0.6, 3.1, green.y),
            Vec3::new(0.6, 0.4, 0.03),
            Mat::Red,
        )));
        for j in 0..2u32 {
            let a = r(k * 5 + j) * std::f32::consts::TAU;
            bunker(out, green + Vec2::new(a.cos(), a.sin()) * 21.0, a);
        }
        // A bunker on the outside of the dogleg, waiting for the long drive.
        let outside = if bend > 0.0 { -side } else { side };
        bunker(out, corner + outside * 17.0, d.y.atan2(d.x));
        // Mounds and trees down both sides.
        let len = tee.distance(green);
        for j in 0..6u32 {
            let u = (j as f32 + 0.5) / 6.0;
            let along = if u < 0.55 {
                tee.lerp(corner, u / 0.55)
            } else {
                corner.lerp(green, (u - 0.55) / 0.45)
            };
            let s = if j % 2 == 0 { 1.0 } else { -1.0 };
            let mound = along + side * s * (26.0 + r(k * 31 + j) * 8.0);
            let size = 8.0 + r(k * 37 + j) * 6.0;
            if dry(mound, size) {
                out.push(round(
                    Vec3::new(mound.x, 0.0, mound.y),
                    2.0 + r(k * 41 + j) * 2.0,
                    size,
                    size * 0.25,
                    Mat::Hedge,
                ));
            }
            if len > 60.0 {
                tree(
                    out,
                    along - side * s * (36.0 + r(k * 43 + j) * 8.0),
                    k * 97 + j,
                );
                tree(
                    out,
                    along + side * s * (40.0 + r(k * 47 + j) * 6.0),
                    k * 89 + j,
                );
            }
        }
    }
    // The pond in the middle, reeds round its edge.
    out.push(soft(round(
        Vec3::new(pond.x, 0.0, pond.y),
        0.1,
        24.0,
        22.0,
        Mat::Water,
    )));
    for j in 0..10u32 {
        let a = j as f32 / 10.0 * std::f32::consts::TAU + r(j + 300) * 0.4;
        let at = pond + Vec2::new(a.cos(), a.sin()) * 23.6;
        out.push(soft(round(
            Vec3::new(at.x, 0.0, at.y),
            1.4,
            0.6,
            0.1,
            Mat::Hedge,
        )));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-170.0, 0.0, 0.0),
            half: Vec3::new(10.0, 4.0, 7.0),
            yaw: 0.0,
            roof: 3.0,
        },
        mat: Mat::White,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(-170.0, 3.0, 9.0),
        Vec3::new(10.0, 0.2, 2.5),
        Mat::Wood,
    )));
    // A putting green by the clubhouse, and carts parked in a row.
    out.push(round(
        Vec3::new(-165.0, 0.0, 26.0),
        0.3,
        9.0,
        8.5,
        Mat::Hedge,
    ));
    for j in 0..4 {
        let at = Vec3::new(-180.0 + j as f32 * 3.0, 0.0, -11.0);
        out.push(bx(at, Vec3::new(0.7, 1.0, 1.2), Mat::White));
        out.push(soft(bx(
            at + Vec3::Y * 1.9,
            Vec3::new(0.8, 0.08, 1.3),
            Mat::Canvas(3),
        )));
    }
}

/// A power pylon's height, and where its conductors hang: height up the pole and how
/// far out to each side (local z, across the line).
pub const PYLON_TALL: f32 = 26.0;
pub const PYLON_ARMS: [(f32, f32); 2] = [(25.0, 5.0), (20.5, 4.0)];

/// A seaside aquarium: a long curved roof of glass over a hall, big lit tanks behind its
/// glass front with fish in them, a bronze whale's tail rising from a pool out front,
/// and a paved forecourt.
fn aquarium(out: &mut Vec<Piece>) {
    out.push(bx(Vec3::ZERO, Vec3::new(24.0, 9.0, 12.0), Mat::White));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::Y * 9.0,
            half: Vec3::new(24.5, 0.4, 12.5),
            yaw: 0.0,
            roof: 5.0,
        },
        mat: Mat::Glass,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 12.05),
        Vec3::new(20.0, 7.0, 0.1),
        Mat::Glass,
    )));
    // Tanks of water inside the glass, lit, with a few fish in each.
    for k in 0..4 {
        let x = -15.0 + k as f32 * 10.0;
        out.push(soft(bx(
            Vec3::new(x, 0.6, 10.5),
            Vec3::new(4.0, 5.0, 1.0),
            Mat::Water,
        )));
        for j in 0..3 {
            out.push(soft(ball(
                Vec3::new(x - 2.0 + j as f32 * 2.0, 2.0 + (j % 2) as f32 * 1.6, 11.6),
                0.35,
                Mat::Canvas((k + j) as u8 % 6),
            )));
        }
    }
    spell(
        out,
        "AQUARIUM",
        Vec3::new(0.0, 7.2, 12.25),
        0.28,
        0.05,
        Mat::Canvas(1),
        false,
    );
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 22.0),
        Vec3::new(18.0, 0.1, 9.0),
        Mat::Stone,
    )));
    out.push(soft(round(
        Vec3::new(0.0, 0.0, 24.0),
        0.6,
        5.0,
        5.0,
        Mat::Water,
    )));
    // The tail: a tall fluke, tips curling out each side.
    out.push(round(Vec3::new(0.0, 0.0, 24.0), 4.0, 0.7, 0.5, Mat::Copper));
    for s in [-1.0f32, 1.0] {
        out.push(Piece {
            form: Form::Box {
                at: Vec3::new(s * 1.6, 4.0, 24.0),
                half: Vec3::new(1.8, 0.35, 0.5),
                yaw: 0.0,
                roof: 0.0,
            },
            mat: Mat::Copper,
            solid: true,
        });
        out.push(ball(Vec3::new(s * 3.2, 4.5, 24.0), 0.6, Mat::Copper));
    }
}

/// A university's quad in its frame, a lawn students cross and sit out on.
pub const QUAD: (Vec2, Vec2) = (Vec2::new(0.0, 22.0), Vec2::new(20.0, 14.0));

/// A university: a long main hall with a clock tower over its door, two wings down the
/// sides of a lawned quad crossed by paths, and benches round it.
fn university(out: &mut Vec<Piece>) {
    out.push(Piece {
        form: Form::Box {
            at: Vec3::ZERO,
            half: Vec3::new(28.0, 11.0, 7.0),
            yaw: 0.0,
            roof: 4.0,
        },
        mat: Mat::Red,
        solid: true,
    });
    for s in [-1.0f32, 1.0] {
        out.push(Piece {
            form: Form::Box {
                at: Vec3::new(s * 25.0, 0.0, 24.0),
                half: Vec3::new(6.0, 8.0, 16.0),
                yaw: 0.0,
                roof: 3.0,
            },
            mat: Mat::Red,
            solid: true,
        });
        for k in 0..6 {
            for y in [2.0f32, 6.0] {
                out.push(soft(bx(
                    Vec3::new(s * 19.0 - s * 0.05, y - 1.0, 12.0 + k as f32 * 4.5),
                    Vec3::new(0.08, 1.2, 1.0),
                    Mat::Glass,
                )));
            }
        }
    }
    for k in 0..10 {
        for y in [2.5f32, 7.0] {
            out.push(soft(bx(
                Vec3::new(-24.0 + k as f32 * 5.3, y - 1.2, 7.05),
                Vec3::new(1.0, 1.4, 0.08),
                Mat::Glass,
            )));
        }
    }
    let t = Vec3::new(0.0, 0.0, 6.0);
    out.push(bx(t, Vec3::new(3.6, 24.0, 3.6), Mat::Red));
    out.push(round(t + Vec3::Y * 24.0, 6.0, 3.8, 0.0, Mat::Copper));
    out.push(soft(round(
        t + Vec3::new(0.0, 19.0, 3.65),
        0.1,
        2.0,
        2.0,
        Mat::White,
    )));
    out.push(soft(bx(
        t + Vec3::new(0.0, 0.0, 3.65),
        Vec3::new(1.4, 3.0, 0.08),
        Mat::Lamp,
    )));
    let (c, half) = QUAD;
    out.push(soft(bx(
        Vec3::new(c.x, 0.0, c.y),
        Vec3::new(half.x, 0.08, half.y),
        Mat::Grass,
    )));
    for (at, h) in [
        (Vec3::new(c.x, 0.0, c.y), Vec3::new(1.2, 0.1, half.y)),
        (Vec3::new(c.x, 0.0, c.y), Vec3::new(half.x, 0.1, 1.2)),
    ] {
        out.push(soft(bx(at, h, Mat::Stone)));
    }
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(soft(bx(
            Vec3::new(c.x + x * (half.x - 2.0), 0.0, c.y + z * 4.0),
            Vec3::new(1.4, 0.5, 0.35),
            Mat::Wood,
        )));
    }
}

/// A library's reading benches out front, in its frame, facing +z.
pub const LIBRARY_BENCHES: [Vec3; 4] = [
    Vec3::new(-11.0, 0.0, 17.0),
    Vec3::new(-5.0, 0.0, 18.0),
    Vec3::new(5.0, 0.0, 18.0),
    Vec3::new(11.0, 0.0, 17.0),
];

/// A town library: a stone hall with tall windows down its sides, a porch of columns
/// under a pediment, a dome over the reading room, steps, and benches on the lawn out
/// front with its name over the door.
fn library(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (15.0, 10.0, 10.0);
    out.push(Piece {
        form: Form::Box {
            at: Vec3::ZERO,
            half: Vec3::new(hx, tall, hz),
            yaw: 0.0,
            roof: 2.5,
        },
        mat: Mat::Stone,
        solid: true,
    });
    for s in [-1.0f32, 1.0] {
        for k in 0..5 {
            out.push(soft(bx(
                Vec3::new(s * (hx + 0.05), 2.5, -7.0 + k as f32 * 3.5),
                Vec3::new(0.08, 4.0, 1.0),
                Mat::Glass,
            )));
        }
    }
    out.push(round(Vec3::Y * (tall + 1.5), 2.0, 6.0, 6.0, Mat::Stone));
    out.push(ball(Vec3::Y * (tall + 3.5), 6.0, Mat::Copper));
    for k in 0..6 {
        let x = -8.75 + k as f32 * 3.5;
        out.push(round(
            Vec3::new(x, 1.2, hz + 3.0),
            7.6,
            0.6,
            0.5,
            Mat::White,
        ));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 8.8, hz + 3.0),
            half: Vec3::new(10.5, 1.0, 3.2),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::White,
        solid: true,
    });
    for (k, z) in [hz + 5.2, hz + 4.0, hz + 2.8].into_iter().enumerate() {
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(10.5 + k as f32, 0.4 * (k + 1) as f32, 1.4),
            Mat::White,
        ));
    }
    out.push(soft(bx(
        Vec3::new(0.0, 1.2, hz + 0.05),
        Vec3::new(1.8, 3.4, 0.1),
        Mat::Lamp,
    )));
    spell(
        out,
        "LIBRARY",
        Vec3::new(0.0, 8.95, hz + 6.25),
        0.32,
        0.06,
        Mat::Copper,
        false,
    );
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 10.0),
        Vec3::new(14.0, 0.08, 5.0),
        Mat::Grass,
    )));
    for at in LIBRARY_BENCHES {
        out.push(soft(bx(at, Vec3::new(1.4, 0.5, 0.35), Mat::Wood)));
    }
}

/// A museum's front steps, in its frame: half their width, the middle of the top step
/// and how far each next one comes out. The top step is 2.4 high, each lower by 0.6.
pub const MUSEUM_STEPS: (f32, f32, f32) = (18.0, 15.0, 1.6);

/// A grand museum: a long stone hall with a pediment front on eight tall columns up a wide
/// stair, banners hung between the columns, its name across the frieze, and a bronze
/// sculpture on a plinth in the square before it.
fn museum(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (26.0, 14.0, 14.0);
    out.push(bx(Vec3::ZERO, Vec3::new(hx, tall, hz), Mat::Stone));
    out.push(bx(
        Vec3::Y * tall,
        Vec3::new(hx + 0.6, 1.2, hz + 0.6),
        Mat::White,
    ));
    for k in 0..8 {
        let x = -17.5 + k as f32 * 5.0;
        out.push(round(
            Vec3::new(x, 2.4, hz + 4.0),
            11.6,
            0.9,
            0.8,
            Mat::White,
        ));
        // Long banners between them, in alternating colors.
        if k < 7 {
            out.push(soft(bx(
                Vec3::new(x + 2.5, 6.0, hz + 4.0),
                Vec3::new(0.9, 6.0, 0.05),
                Mat::Canvas(if k % 2 == 0 { 0 } else { 3 }),
            )));
        }
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 14.0, hz + 4.0),
            half: Vec3::new(20.0, 1.6, 4.4),
            yaw: 0.0,
            roof: 4.0,
        },
        mat: Mat::White,
        solid: true,
    });
    spell(
        out,
        "MUSEUM",
        Vec3::new(0.0, 14.2, hz + 8.45),
        0.42,
        0.06,
        Mat::Dark,
        false,
    );
    let (_, front, _) = MUSEUM_STEPS;
    for k in 0..4u32 {
        let z = hz + 1.0 + k as f32 * 1.6;
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(21.0 + k as f32, 2.4 - k as f32 * 0.6, 0.8),
            Mat::White,
        ));
    }
    out.push(soft(bx(
        Vec3::new(0.0, 2.4, hz + 0.05),
        Vec3::new(2.2, 4.2, 0.1),
        Mat::Lamp,
    )));
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, front + 12.0),
        Vec3::new(18.0, 0.1, 10.0),
        Mat::Stone,
    )));
    let plinth = Vec3::new(0.0, 0.0, front + 12.0);
    out.push(bx(plinth, Vec3::new(1.6, 2.2, 1.6), Mat::Stone));
    out.push(ball(plinth + Vec3::Y * 3.4, 1.4, Mat::Copper));
    out.push(round(plinth + Vec3::Y * 2.2, 2.2, 0.5, 0.3, Mat::Copper));
}

/// Where the cinema queue forms, in its frame: from the doors out along +x.
pub const CINEMA_QUEUE: (Vec2, Vec2) = (Vec2::new(-4.0, 12.5), Vec2::new(1.0, 0.0));

/// A picture house: a tall block with a lit sign standing up its front reading CINEMA, a
/// marquee over the doors edged with bulbs, posters in lit frames either side, and a
/// ticket booth.
fn cinema(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (14.0, 13.0, 12.0);
    out.push(bx(Vec3::ZERO, Vec3::new(hx, tall, hz), Mat::Concrete));
    out.push(bx(
        Vec3::Y * tall,
        Vec3::new(hx + 0.4, 0.8, hz + 0.4),
        Mat::Red,
    ));
    // The tall sign out from the front, its letters lit.
    out.push(bx(
        Vec3::new(0.0, 6.0, hz + 1.2),
        Vec3::new(1.3, 12.0, 0.6),
        Mat::Red,
    ));
    for (k, c) in "CINEMA".chars().enumerate() {
        let y = 16.0 - k as f32 * 1.75;
        spell(
            out,
            &c.to_string(),
            Vec3::new(0.0, y, hz + 1.85),
            0.28,
            0.05,
            Mat::Lamp,
            false,
        );
    }
    // The marquee: a deep canopy over the doors, a row of bulbs round its edge.
    out.push(bx(
        Vec3::new(0.0, 4.2, hz + 2.6),
        Vec3::new(8.0, 0.8, 2.6),
        Mat::White,
    ));
    for k in 0..14 {
        let x = -7.5 + k as f32 * (15.0 / 13.0);
        out.push(soft(bx(
            Vec3::new(x, 4.1, hz + 5.25),
            Vec3::splat(0.14),
            Mat::Lamp,
        )));
    }
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 0.05),
        Vec3::new(5.0, 3.6, 0.1),
        Mat::Glass,
    )));
    for x in [-10.5f32, 10.5] {
        out.push(soft(bx(
            Vec3::new(x, 1.2, hz + 0.1),
            Vec3::new(1.6, 2.2, 0.1),
            Mat::Lamp,
        )));
        out.push(soft(bx(
            Vec3::new(x, 1.4, hz + 0.18),
            Vec3::new(1.3, 1.9, 0.05),
            Mat::Canvas((x > 0.0) as u8 * 3 + 1),
        )));
    }
    out.push(bx(
        Vec3::new(-8.0, 0.0, hz + 6.0),
        Vec3::new(1.2, 2.4, 1.0),
        Mat::Red,
    ));
    out.push(soft(bx(
        Vec3::new(-8.0, 1.2, hz + 7.05),
        Vec3::new(0.8, 0.6, 0.05),
        Mat::Lamp,
    )));
}

/// A mushroom farm's beds in its frame: rows along x, each centered at `z`, `len` long.
pub const SHROOM_BEDS: [f32; 6] = [-9.0, -5.4, -1.8, 1.8, 5.4, 9.0];
pub const SHROOM_BED_LEN: f32 = 24.0;

/// A mushroom farm on a cavern floor: a yard of packed earth, raised wooden beds of dark
/// soil in rows with pale and brown mushrooms up through them, lanterns on posts at the
/// ends, baskets, and a little hut.
fn shroom_farm(out: &mut Vec<Piece>, seed: u32) {
    let r = |k: u32| crate::shape::hash(seed.wrapping_add(k.wrapping_mul(2_654_435_761)));
    out.push(soft(bx(Vec3::ZERO, Vec3::new(18.0, 0.1, 13.0), Mat::Sand)));
    let half = SHROOM_BED_LEN * 0.5;
    for (k, z) in SHROOM_BEDS.into_iter().enumerate() {
        let k = k as u32;
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(half, 0.5, 0.9),
            Mat::Wood,
        ));
        out.push(soft(bx(
            Vec3::new(0.0, 0.5, z),
            Vec3::new(half - 0.1, 0.05, 0.8),
            Mat::Dark,
        )));
        // Low domed caps on short stems, kept cheap: they are many.
        let n = (SHROOM_BED_LEN / 2.2) as u32;
        for j in 0..n {
            let x = -half + 1.1 + j as f32 * 2.2 + (r(k * 31 + j) - 0.5) * 0.6;
            let zz = z + (r(k * 37 + j) - 0.5) * 0.9;
            let tall = 0.18 + r(k * 41 + j) * 0.2;
            let cap = if r(k * 43 + j) < 0.6 {
                Mat::White
            } else {
                Mat::Hay
            };
            let w = 0.16 + r(k * 47 + j) * 0.12;
            out.push(soft(round(
                Vec3::new(x, 0.55, zz),
                tall,
                0.06,
                0.05,
                Mat::White,
            )));
            out.push(soft(round(
                Vec3::new(x, 0.5 + tall, zz),
                w * 0.7,
                w,
                w * 0.25,
                cap,
            )));
        }
    }
    for x in [-half - 1.5, half + 1.5] {
        for z in [-10.8f32, 0.0, 10.8] {
            out.push(soft(round(
                Vec3::new(x, 0.0, z),
                2.6,
                0.08,
                0.08,
                Mat::Wood,
            )));
            out.push(soft(ball(Vec3::new(x, 2.7, z), 0.25, Mat::Lamp)));
        }
    }
    for k in 0..4u32 {
        let at = Vec3::new(-half + 2.0 + k as f32 * 6.5, 0.0, 11.9);
        out.push(soft(round(at, 0.5, 0.35, 0.42, Mat::Hay)));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-14.5, 0.0, -11.5),
            half: Vec3::new(2.5, 2.4, 2.0),
            yaw: 0.0,
            roof: 1.2,
        },
        mat: Mat::Wood,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(-14.5, 0.0, -9.45),
        Vec3::new(0.6, 1.8, 0.06),
        Mat::Lamp,
    )));
}

/// A crystal mine's track in its frame, from the adit mouth out to the tip, along +z.
pub const MINE_TRACK: (f32, f32) = (-5.5, 20.0);

/// A crystal mine cut into a rock outcrop on a cavern floor: a dark adit in a timber
/// frame with a lamp, a narrow track out to a tip of rubble glinting with crystals, and
/// a tool shed.
fn mine(out: &mut Vec<Piece>) {
    for (at, r) in [
        (Vec3::new(0.0, 2.0, -15.0), 11.0),
        (Vec3::new(-9.0, 0.0, -12.0), 8.0),
        (Vec3::new(8.5, 0.5, -12.5), 8.5),
        (Vec3::new(0.0, 9.0, -13.0), 7.5),
    ] {
        out.push(ball(at, r, Mat::Stone));
    }
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, -6.4),
        Vec3::new(2.2, 4.4, 1.6),
        Mat::Dark,
    )));
    for x in [-2.6f32, 2.6] {
        out.push(bx(
            Vec3::new(x, 0.0, -5.2),
            Vec3::new(0.3, 4.8, 0.3),
            Mat::Wood,
        ));
    }
    out.push(bx(
        Vec3::new(0.0, 4.8, -5.2),
        Vec3::new(3.1, 0.45, 0.4),
        Mat::Wood,
    ));
    out.push(soft(ball(Vec3::new(1.8, 4.2, -4.7), 0.3, Mat::Lamp)));
    let (from, to) = MINE_TRACK;
    let n = ((to - from) / 1.4) as u32;
    for k in 0..=n {
        let z = from + k as f32 * 1.4;
        out.push(soft(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(0.8, 0.08, 0.18),
            Mat::Wood,
        )));
    }
    for x in [-0.45f32, 0.45] {
        out.push(soft(bx(
            Vec3::new(x, 0.08, (from + to) * 0.5),
            Vec3::new(0.05, 0.07, (to - from) * 0.5),
            Mat::Metal,
        )));
    }
    // The tip at the end of the track, rubble with crystals in it.
    out.push(soft(round(
        Vec3::new(0.0, 0.0, to + 4.0),
        2.2,
        5.0,
        1.5,
        Mat::Stone,
    )));
    for k in 0..5u32 {
        let a = k as f32 * 1.3;
        out.push(soft(round(
            Vec3::new(a.cos() * 3.0, 0.8, to + 4.0 + a.sin() * 3.0),
            1.4,
            0.3,
            0.05,
            Mat::Glass,
        )));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(7.0, 0.0, 6.0),
            half: Vec3::new(2.0, 2.2, 2.5),
            yaw: 0.2,
            roof: 1.0,
        },
        mat: Mat::Wood,
        solid: true,
    });
}

/// A cafe's tables in its frame. Two to four people sit at each.
pub const CAFE_TABLES: [Vec2; 6] = [
    Vec2::new(-6.0, 3.0),
    Vec2::new(0.0, 3.5),
    Vec2::new(6.0, 3.0),
    Vec2::new(-6.0, 9.0),
    Vec2::new(0.0, 9.5),
    Vec2::new(6.0, 9.0),
];

/// A mountain restaurant's tables out on its deck, in its frame.
pub const RESTAURANT_TABLES: [Vec2; 6] = [
    Vec2::new(-8.0, 9.0),
    Vec2::new(0.0, 9.5),
    Vec2::new(8.0, 9.0),
    Vec2::new(-8.0, 15.5),
    Vec2::new(0.0, 16.0),
    Vec2::new(8.0, 15.5),
];

/// A restaurant up a mountain beside a lift's top station: a stone plinth set into the
/// slope, a timber chalet with a steep red roof and a glass front under its name, and a
/// broad deck jutting out over the drop on steel legs, railed all round, with tables
/// under umbrellas and a coin telescope at the corner for the view.
fn restaurant(out: &mut Vec<Piece>) {
    out.push(bx(
        Vec3::new(0.0, -16.0, 0.0),
        Vec3::new(12.0, 15.95, 7.0),
        Mat::Stone,
    ));
    out.push(bx(
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(10.0, 6.0, 5.5),
        Mat::Wood,
    ));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 6.0, -1.0),
            half: Vec3::new(10.8, 0.3, 6.4),
            yaw: 0.0,
            roof: 4.0,
        },
        mat: Mat::Red,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.6, 4.55),
        Vec3::new(8.0, 4.4, 0.1),
        Mat::Glass,
    )));
    spell(
        out,
        "RESTAURANT",
        Vec3::new(0.0, 5.3, 4.7),
        0.2,
        0.05,
        Mat::Lamp,
        false,
    );
    // The deck, out over the drop on long legs.
    out.push(bx(
        Vec3::new(0.0, -0.6, 12.5),
        Vec3::new(13.0, 0.6, 8.0),
        Mat::Wood,
    ));
    for x in [-12.0f32, -4.0, 4.0, 12.0] {
        for z in [13.0f32, 20.0] {
            out.push(round(Vec3::new(x, -24.0, z), 23.4, 0.35, 0.3, Mat::Metal));
        }
    }
    let rail = |out: &mut Vec<Piece>, a: Vec2, b: Vec2| {
        let n = (a.distance(b) / 2.0).ceil().max(1.0) as u32;
        for k in 0..=n {
            let p = a.lerp(b, k as f32 / n as f32);
            out.push(soft(round(
                Vec3::new(p.x, 0.0, p.y),
                1.1,
                0.05,
                0.05,
                Mat::Metal,
            )));
        }
        let d = b - a;
        let mid = (a + b) * 0.5;
        out.push(soft(turned(
            Vec3::new(mid.x, 1.1, mid.y),
            Vec3::new(d.length() * 0.5, 0.06, 0.06),
            (-d.y).atan2(d.x),
            Mat::Wood,
        )));
    };
    let (x0, x1, z1) = (-12.8, 12.8, 20.3);
    rail(out, Vec2::new(x0, 4.8), Vec2::new(x0, z1));
    rail(out, Vec2::new(x0, z1), Vec2::new(x1, z1));
    rail(out, Vec2::new(x1, z1), Vec2::new(x1, 4.8));
    for (k, at) in RESTAURANT_TABLES.into_iter().enumerate() {
        let t = Vec3::new(at.x, 0.0, at.y);
        out.push(soft(round(t, 0.8, 0.12, 0.12, Mat::Metal)));
        out.push(soft(round(t + Vec3::Y * 0.8, 0.06, 0.9, 0.9, Mat::White)));
        out.push(soft(round(t, 2.4, 0.05, 0.05, Mat::Wood)));
        out.push(soft(round(
            t + Vec3::Y * 2.3,
            0.7,
            2.1,
            0.1,
            Mat::Canvas(k as u8 % 6),
        )));
    }
    let scope = Vec3::new(11.5, 0.0, 19.0);
    out.push(soft(round(scope, 1.1, 0.1, 0.08, Mat::Metal)));
    out.push(soft(bx(
        scope + Vec3::new(0.0, 1.1, 0.0),
        Vec3::new(0.15, 0.2, 0.35),
        Mat::Copper,
    )));
}

/// A drawbridge's lifting leaves: where each hinges (x either side of the middle), how
/// long it is and the deck's height over the water, in its frame.
pub const DRAW_HINGE: f32 = 8.3;
pub const DRAW_DECK: f32 = 5.0;

/// A footbridge over a strait with a lifting span in the middle, in its frame along x,
/// `text="LEN"` long, the water at y = 0: plank decks on stone piers out from each shore,
/// railed both sides, to a brick tower either side of the gap where the two leaves (drawn
/// by the client, as they move) lift for boats.
fn drawbridge(out: &mut Vec<Piece>, text: &str) {
    let half = text
        .split_whitespace()
        .next()
        .and_then(|n| n.parse::<f32>().ok())
        .unwrap_or(200.0)
        * 0.5;
    let w = 2.5;
    for s in [-1.0f32, 1.0] {
        let (a, b) = (DRAW_HINGE + 0.2, half);
        let mid = (a + b) * 0.5 * s;
        out.push(bx(
            Vec3::new(mid, DRAW_DECK - 0.5, 0.0),
            Vec3::new((b - a) * 0.5, 0.5, w),
            Mat::Wood,
        ));
        let mut x = 15.0;
        while x < half - 4.0 {
            out.push(round(
                Vec3::new(x * s, -16.0, 0.0),
                20.5,
                0.9,
                0.8,
                Mat::Stone,
            ));
            x += 15.0;
        }
        // Railings, posts every 2.5 m and a top rail, along both sides.
        for z in [-w + 0.1, w - 0.1] {
            let mut x = a;
            while x <= b {
                out.push(soft(round(
                    Vec3::new(x * s, DRAW_DECK, z),
                    1.1,
                    0.06,
                    0.06,
                    Mat::White,
                )));
                x += 2.5;
            }
            out.push(soft(bx(
                Vec3::new(mid, DRAW_DECK + 1.05, z),
                Vec3::new((b - a) * 0.5, 0.08, 0.06),
                Mat::White,
            )));
        }
        // The tower beside the gap, on its own stone footing, with lit windows.
        let t = Vec3::new(s * (DRAW_HINGE + 3.5), 0.0, -w - 2.4);
        out.push(bx(
            t - Vec3::Y * 14.0,
            Vec3::new(2.8, 14.0, 2.6),
            Mat::Stone,
        ));
        out.push(Piece {
            form: Form::Box {
                at: t,
                half: Vec3::new(2.3, 10.0, 2.1),
                yaw: 0.0,
                roof: 2.2,
            },
            mat: Mat::Red,
            solid: true,
        });
        for y in [6.5f32, 8.5] {
            out.push(soft(bx(
                t + Vec3::new(0.0, y - 0.6, 2.15),
                Vec3::new(0.6, 0.7, 0.06),
                Mat::Glass,
            )));
        }
        out.push(soft(ball(t + Vec3::new(0.0, 12.6, 0.0), 0.35, Mat::Lamp)));
        // A stone footing under each hinge, its top just under the planks so there is no
        // step to trip on while the leaves are up.
        out.push(bx(
            Vec3::new(s * (DRAW_HINGE + 1.0), -14.0, 0.0),
            Vec3::new(1.4, 14.0 + DRAW_DECK - 0.3, w + 0.4),
            Mat::Stone,
        ));
    }
}

/// A gallery's walkway height over the floor (m).
pub const GALLERY_UP: f32 = 24.0;

/// A wooden gallery round the inside walls of a carved hall, in its frame, `text="W D"`
/// the hall's half length and half width inside its walls: a railed walkway on posts all
/// the way round, lanterns along the rail, and a broad flight of stairs up to it from the
/// floor in the middle of each long side.
fn gallery(out: &mut Vec<Piece>, text: &str) {
    let mut nums = text
        .split_whitespace()
        .filter_map(|n| n.parse::<f32>().ok());
    let (hw, hd) = (nums.next().unwrap_or(240.0), nums.next().unwrap_or(140.0));
    let (up, wide) = (GALLERY_UP, 4.0);
    let (x, z) = (hw - wide * 0.5, hd - wide * 0.5);
    // The four runs of walkway, each as its line (start, end) round the hall.
    let runs = [
        (Vec2::new(-x, z), Vec2::new(x, z)),
        (Vec2::new(x, z), Vec2::new(x, -z)),
        (Vec2::new(x, -z), Vec2::new(-x, -z)),
        (Vec2::new(-x, -z), Vec2::new(-x, z)),
    ];
    for (a, b) in runs {
        let d = b - a;
        let mid = (a + b) * 0.5;
        let yaw = (-d.y).atan2(d.x);
        out.push(turned(
            Vec3::new(mid.x, up - 0.6, mid.y),
            Vec3::new(d.length() * 0.5 + wide * 0.5, 0.6, wide * 0.5),
            yaw,
            Mat::Wood,
        ));
        // The rail on the inner edge, posts down to the floor, and lanterns.
        let inward = if d.x.abs() > d.y.abs() {
            Vec2::new(0.0, -mid.y.signum())
        } else {
            Vec2::new(-mid.x.signum(), 0.0)
        };
        let edge = inward * (wide * 0.5 - 0.15);
        let n = (d.length() / 4.0).ceil() as u32;
        for k in 0..=n {
            let p = a.lerp(b, k as f32 / n as f32) + edge;
            out.push(soft(round(
                Vec3::new(p.x, up, p.y),
                1.1,
                0.06,
                0.06,
                Mat::Wood,
            )));
            if k % 5 == 0 {
                out.push(round(
                    Vec3::new(p.x, 0.0, p.y),
                    up - 1.2,
                    0.35,
                    0.3,
                    Mat::Wood,
                ));
            }
            if k % 6 == 3 {
                out.push(soft(ball(Vec3::new(p.x, up + 1.5, p.y), 0.35, Mat::Lamp)));
            }
        }
        let r = (a + b) * 0.5 + edge;
        out.push(soft(turned(
            Vec3::new(r.x, up + 1.05, r.y),
            Vec3::new(d.length() * 0.5, 0.07, 0.07),
            yaw,
            Mat::Wood,
        )));
    }
    // Stairs up from the floor to the middle of each long side.
    for s in [-1.0f32, 1.0] {
        let run = up / 0.6;
        let steps = (up / 0.5).ceil() as u32;
        let (rise, tread) = (up / steps as f32, run / steps as f32);
        let foot = s * (hd - wide - run);
        for k in 0..steps {
            let top = (k + 1) as f32 * rise;
            let zc = foot + s * (k as f32 + 0.5) * tread;
            out.push(bx(
                Vec3::new(0.0, top - 2.0, zc),
                Vec3::new(4.0, 2.0, tread * 0.5 + 0.02),
                Mat::Stone,
            ));
            if k % 4 == 0 {
                for xr in [-3.8f32, 3.8] {
                    out.push(soft(round(
                        Vec3::new(xr, top, zc),
                        1.0,
                        0.05,
                        0.05,
                        Mat::Wood,
                    )));
                }
            }
        }
    }
}

/// A skate park's pad in its frame, half its size, and the lines its skaters ride
/// back and forth along, over the boxes.
pub const SKATE_PAD: Vec2 = Vec2::new(16.0, 12.0);
pub const SKATE_LINES: [(Vec2, Vec2); 4] = [
    (Vec2::new(-13.0, -6.0), Vec2::new(13.0, -6.0)),
    (Vec2::new(-13.0, 5.0), Vec2::new(13.0, 5.0)),
    (Vec2::new(-10.0, -9.0), Vec2::new(10.0, 9.0)),
    (Vec2::new(-12.0, 0.0), Vec2::new(12.0, 0.0)),
];
/// Where the skaters jump the fun boxes, along x, and how high.
pub const SKATE_BOXES: [(Vec2, f32); 2] =
    [(Vec2::new(-4.0, -6.0), 1.2), (Vec2::new(5.0, 5.0), 1.0)];

/// A skate park: a concrete pad with two fun boxes, grind rails and a ledge, a low
/// fence round it, and benches for the friends watching.
fn skate_park(out: &mut Vec<Piece>) {
    let pad = SKATE_PAD;
    out.push(bx(Vec3::ZERO, Vec3::new(pad.x, 0.2, pad.y), Mat::Concrete));
    for (at, h) in SKATE_BOXES {
        out.push(Piece {
            form: Form::Box {
                at: Vec3::new(at.x, 0.2, at.y),
                half: Vec3::new(3.0, 0.05, 2.0),
                yaw: std::f32::consts::FRAC_PI_2,
                roof: h,
            },
            mat: Mat::Concrete,
            solid: true,
        });
    }
    for (z, x) in [(-1.5f32, 0.0f32), (9.0, -6.0), (-10.0, 6.0)] {
        out.push(soft(bx(
            Vec3::new(x, 0.75, z),
            Vec3::new(4.0, 0.05, 0.05),
            Mat::Metal,
        )));
        for s in [-1.0f32, 1.0] {
            out.push(soft(bx(
                Vec3::new(x + s * 3.6, 0.2, z),
                Vec3::new(0.05, 0.55, 0.05),
                Mat::Metal,
            )));
        }
    }
    out.push(bx(
        Vec3::new(-11.0, 0.2, 9.5),
        Vec3::new(3.0, 0.5, 0.8),
        Mat::Stone,
    ));
    out.push(soft(bx(
        Vec3::new(10.0, 0.2, -9.0),
        Vec3::new(2.0, 0.06, 2.0),
        Mat::Canvas(4),
    )));
    let corners = [
        Vec2::new(-pad.x, -pad.y),
        Vec2::new(pad.x, -pad.y),
        Vec2::new(pad.x, pad.y),
        Vec2::new(-pad.x, pad.y),
    ];
    fence(out, &corners, 2.5, Mat::Metal);
    for x in [-8.0f32, 8.0] {
        out.push(soft(bx(
            Vec3::new(x, 0.0, pad.y + 2.0),
            Vec3::new(1.4, 0.5, 0.35),
            Mat::Wood,
        )));
    }
}

/// Where a helicopter sets down: on a helipad, and on a hospital's roof (m up).
pub const HELIPAD_TOP: f32 = 0.2;
pub const HOSPITAL_PAD: f32 = 28.12;

/// A helipad: a concrete pad with a yellow ring and a big white H, lights round its edge
/// and a windsock on a pole beside it.
fn helipad(out: &mut Vec<Piece>) {
    out.push(round(Vec3::ZERO, HELIPAD_TOP, 11.0, 11.0, Mat::Concrete));
    out.push(soft(round(
        Vec3::Y * HELIPAD_TOP,
        0.04,
        9.0,
        9.0,
        Mat::Canvas(2),
    )));
    out.push(soft(round(
        Vec3::Y * (HELIPAD_TOP + 0.04),
        0.04,
        8.0,
        8.0,
        Mat::Concrete,
    )));
    for x in [-2.4f32, 2.4] {
        out.push(soft(bx(
            Vec3::new(x, HELIPAD_TOP + 0.08, 0.0),
            Vec3::new(0.6, 0.03, 4.0),
            Mat::White,
        )));
    }
    out.push(soft(bx(
        Vec3::new(0.0, HELIPAD_TOP + 0.08, 0.0),
        Vec3::new(1.8, 0.03, 0.6),
        Mat::White,
    )));
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        out.push(soft(ball(
            Vec3::new(a.cos() * 10.6, 0.35, a.sin() * 10.6),
            0.18,
            Mat::Lamp,
        )));
    }
    let pole = Vec3::new(14.0, 0.0, 8.0);
    out.push(round(pole, 6.0, 0.08, 0.06, Mat::Metal));
    out.push(soft(round(
        pole + Vec3::new(1.2, 5.6, 0.0),
        0.5,
        0.35,
        0.18,
        Mat::Canvas(5),
    )));
}

/// Every clock face on a landmark kind, in its frame: the middle of the face, which way
/// it looks, and how long its minute hand is. Their hands show the time of day.
pub fn clocks(kind: Kind) -> Vec<(Vec3, Vec3, f32)> {
    let sides = [Vec3::Z, -Vec3::Z, Vec3::X, -Vec3::X];
    match kind {
        Kind::TownHall => {
            let t = Vec3::new(0.0, TOWN_HALL_TALL + 12.4, -1.0);
            sides.iter().map(|n| (t + *n * 4.1, *n, 1.8)).collect()
        }
        Kind::ClockTower => {
            let t = Vec3::new(0.0, CLOCK_TOWER_FACE, 0.0);
            sides.iter().map(|n| (t + *n * 3.05, *n, 1.6)).collect()
        }
        Kind::University => vec![(Vec3::new(0.0, 19.0, 9.7), Vec3::Z, 1.6)],
        Kind::BasaltGate => vec![
            (Vec3::new(0.0, 106.0, BASALT_FRONT + 1.5), Vec3::Z, 6.5),
            (Vec3::new(0.0, 106.0, BASALT_BACK - 1.5), -Vec3::Z, 6.5),
        ],
        _ => Vec::new(),
    }
}

/// How high a clock tower's faces are.
const CLOCK_TOWER_FACE: f32 = 18.0;

/// A town clock tower on its own little square: a stone tower with a clock on every
/// side near the top under a copper spire, a lamp post at each corner of the paving and
/// benches round it.
fn clock_tower(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(11.0, 0.15, 11.0),
        Mat::Stone,
    )));
    out.push(bx(
        Vec3::ZERO,
        Vec3::new(3.0, CLOCK_TOWER_FACE + 3.0, 3.0),
        Mat::Stone,
    ));
    out.push(bx(
        Vec3::Y * (CLOCK_TOWER_FACE + 3.0),
        Vec3::new(3.4, 0.6, 3.4),
        Mat::White,
    ));
    out.push(round(
        Vec3::Y * (CLOCK_TOWER_FACE + 3.6),
        7.0,
        3.4,
        0.0,
        Mat::Copper,
    ));
    for (dx, dz) in [(0.0f32, 1.0f32), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
        let half = if dz != 0.0 {
            Vec3::new(2.0, 4.0, 0.06)
        } else {
            Vec3::new(0.06, 4.0, 2.0)
        };
        out.push(soft(bx(
            Vec3::new(dx * 3.02, CLOCK_TOWER_FACE - 2.0, dz * 3.02),
            half,
            Mat::White,
        )));
    }
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let at = Vec3::new(x * 9.5, 0.15, z * 9.5);
        out.push(soft(round(at, 4.5, 0.1, 0.08, Mat::Metal)));
        out.push(soft(ball(at + Vec3::Y * 4.8, 0.4, Mat::Lamp)));
    }
    for (x, z) in [(0.0f32, 6.5f32), (0.0, -6.5), (6.5, 0.0), (-6.5, 0.0)] {
        let half = if z != 0.0 {
            Vec3::new(1.4, 0.5, 0.35)
        } else {
            Vec3::new(0.35, 0.5, 1.4)
        };
        out.push(soft(bx(Vec3::new(x, 0.15, z), half, Mat::Wood)));
    }
}

/// A lantern-lit cafe: a kiosk with a striped awning and a counter, round tables with
/// stools on a paved terrace, and strings of lights on posts round it.
fn cafe(out: &mut Vec<Piece>) {
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(11.0, 0.12, 8.0),
        Mat::Stone,
    )));
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 0.0, -4.5),
            half: Vec3::new(5.0, 3.0, 2.5),
            yaw: 0.0,
            roof: 1.0,
        },
        mat: Mat::Wood,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 1.2, -1.95),
        Vec3::new(3.5, 0.9, 0.06),
        Mat::Lamp,
    )));
    for s in 0..5u32 {
        out.push(soft(Piece {
            form: Form::Box {
                at: Vec3::new(-4.0 + s as f32 * 2.0, 3.0, -1.2),
                half: Vec3::new(1.0, 0.08, 1.3),
                yaw: 0.0,
                roof: 0.3,
            },
            mat: if s % 2 == 0 {
                Mat::Canvas(0)
            } else {
                Mat::White
            },
            solid: false,
        }));
    }
    for t in CAFE_TABLES {
        let at = Vec3::new(t.x, 0.12, t.y);
        out.push(soft(round(at, 0.75, 0.08, 0.08, Mat::Metal)));
        out.push(soft(round(at + Vec3::Y * 0.75, 0.05, 0.6, 0.6, Mat::White)));
        out.push(soft(bx(at + Vec3::Y * 0.8, Vec3::splat(0.08), Mat::Lamp)));
        for k in 0..4 {
            let a = k as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
            out.push(soft(round(
                at + Vec3::new(a.cos() * 1.0, 0.0, a.sin() * 1.0),
                0.45,
                0.2,
                0.2,
                Mat::Wood,
            )));
        }
    }
    // Strings of lights on posts round the terrace.
    let posts = [
        Vec2::new(-10.5, -1.0),
        Vec2::new(10.5, -1.0),
        Vec2::new(10.5, 12.5),
        Vec2::new(-10.5, 12.5),
    ];
    for (i, p) in posts.iter().enumerate() {
        out.push(soft(round(
            Vec3::new(p.x, 0.0, p.y),
            3.6,
            0.07,
            0.07,
            Mat::Wood,
        )));
        let q = posts[(i + 1) % 4];
        for k in 1..8 {
            let u = k as f32 / 8.0;
            let sag = 0.6 * 4.0 * u * (1.0 - u);
            let at = p.lerp(q, u);
            out.push(soft(bx(
                Vec3::new(at.x, 3.4 - sag, at.y),
                Vec3::splat(0.1),
                Mat::Lamp,
            )));
        }
    }
}

/// A market square: two rows of stalls facing each other across a cobbled lane, each a
/// counter piled with fruit, greens or bread under a striped awning on poles.
fn market(out: &mut Vec<Piece>, seed: u32) {
    let r = |k: u32| crate::shape::hash(seed.wrapping_add(k.wrapping_mul(2_654_435_761)));
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(22.0, 0.12, 13.0),
        Mat::Stone,
    )));
    for (k, (at, face)) in STALLS.into_iter().enumerate() {
        let k = k as u32;
        let c = Vec3::new(at.x, 0.0, at.y);
        out.push(bx(c, Vec3::new(3.6, 0.95, 1.2), Mat::Wood));
        // Produce in little heaps along the counter.
        let goods = [
            Mat::Red,
            Mat::Leaf,
            Mat::Hay,
            Mat::Canvas(5),
            Mat::Canvas(2),
        ];
        for j in 0..4u32 {
            let x = at.x - 2.7 + j as f32 * 1.8;
            out.push(soft(ball(
                Vec3::new(x, 1.15, at.y + face * 0.3),
                0.45,
                goods[(r(k * 7 + j) * goods.len() as f32) as usize % goods.len()],
            )));
        }
        for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            out.push(soft(round(
                c + Vec3::new(x * 3.8, 0.0, z * 1.4),
                2.8,
                0.07,
                0.07,
                Mat::Wood,
            )));
        }
        // The awning, striped in two colors, sloping toward the shoppers.
        let (a, b) = (Mat::Canvas((r(k * 3 + 1) * 6.0) as u8), Mat::White);
        for s in 0..4u32 {
            out.push(soft(Piece {
                form: Form::Box {
                    at: c + Vec3::new(-3.0 + s as f32 * 2.0, 2.8, 0.0),
                    half: Vec3::new(1.0, 0.1, 1.8),
                    yaw: 0.0,
                    roof: 0.5,
                },
                mat: if s % 2 == 0 { a } else { b },
                solid: false,
            }));
        }
        // Crates stacked behind the counter.
        out.push(soft(bx(
            c + Vec3::new(2.4, 0.0, -face * 1.9),
            Vec3::new(0.6, 0.9, 0.5),
            Mat::Wood,
        )));
    }
}

/// A hospital: a tall white block with glass bands and a red cross, lower wings either
/// side, an entrance canopy with an ambulance waiting, and a helipad on the roof.
fn hospital(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (30.0, 28.0, 16.0);
    out.push(bx(Vec3::ZERO, Vec3::new(hx, tall, hz), Mat::White));
    for x in [-1.0f32, 1.0] {
        out.push(bx(
            Vec3::new(x * (hx + 10.0), 0.0, -2.0),
            Vec3::new(10.0, 16.0, 13.0),
            Mat::White,
        ));
        for y in [3.0f32, 7.0, 11.0] {
            out.push(soft(bx(
                Vec3::new(x * (hx + 10.0), y, 11.05),
                Vec3::new(9.0, 1.4, 0.1),
                Mat::Glass,
            )));
        }
    }
    for k in 0..6 {
        let y = 3.0 + k as f32 * 4.0;
        for z in [hz + 0.05, -hz - 0.05] {
            out.push(soft(bx(
                Vec3::new(0.0, y, z),
                Vec3::new(hx - 1.0, 1.5, 0.1),
                Mat::Glass,
            )));
        }
    }
    // The red cross high on the front.
    out.push(soft(bx(
        Vec3::new(0.0, tall - 9.0, hz + 0.2),
        Vec3::new(0.9, 6.0, 0.1),
        Mat::Red,
    )));
    out.push(soft(bx(
        Vec3::new(0.0, tall - 6.8, hz + 0.2),
        Vec3::new(3.0, 1.8, 0.1),
        Mat::Red,
    )));
    // The helipad: a yellow ring round a pale pad with a white H.
    let top = tall;
    out.push(soft(round(Vec3::Y * top, 0.06, 11.0, 11.0, Mat::Canvas(2))));
    out.push(soft(round(
        Vec3::Y * (top + 0.06),
        0.06,
        9.8,
        9.8,
        Mat::Concrete,
    )));
    for x in [-2.6f32, 2.6] {
        out.push(soft(bx(
            Vec3::new(x, top + 0.12, 0.0),
            Vec3::new(0.6, 0.05, 4.0),
            Mat::White,
        )));
    }
    out.push(soft(bx(
        Vec3::new(0.0, top + 0.12, 0.0),
        Vec3::new(2.6, 0.05, 0.6),
        Mat::White,
    )));
    // The entrance, its canopy, and an ambulance under it.
    out.push(bx(
        Vec3::new(0.0, 4.6, hz + 3.5),
        Vec3::new(9.0, 0.4, 3.5),
        Mat::White,
    ));
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 0.1),
        Vec3::new(3.0, 3.6, 0.1),
        Mat::Lamp,
    )));
    let amb = Vec3::new(6.0, 0.0, hz + 4.0);
    out.push(bx(amb, Vec3::new(3.0, 2.4, 1.2), Mat::White));
    out.push(soft(bx(
        amb + Vec3::Y * 1.0,
        Vec3::new(3.05, 0.35, 1.25),
        Mat::Red,
    )));
    out.push(soft(bx(
        amb + Vec3::new(1.8, 1.4, 0.0),
        Vec3::new(1.1, 0.6, 1.25),
        Mat::Glass,
    )));
    // A car park out front.
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 20.0),
        Vec3::new(hx + 18.0, 0.12, 11.0),
        Mat::Asphalt,
    )));
    for k in 0..16 {
        out.push(soft(bx(
            Vec3::new(-44.0 + k as f32 * 6.0, 0.0, hz + 20.0),
            Vec3::new(0.08, 0.15, 4.0),
            Mat::White,
        )));
    }
}

/// A fire station: a red brick hall with three tall doors, one up with the engine pulled
/// out front, a hose tower behind, and FIRE over the doors.
fn fire_station(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (14.0, 9.0, 9.0);
    out.push(Piece {
        form: Form::Box {
            at: Vec3::ZERO,
            half: Vec3::new(hx, tall, hz),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::Red,
        solid: true,
    });
    for (k, x) in [-9.0f32, 0.0, 9.0].into_iter().enumerate() {
        out.push(soft(bx(
            Vec3::new(x, 0.0, hz + 0.05),
            Vec3::new(3.2, 6.0, 0.1),
            if k == 2 { Mat::Dark } else { Mat::White },
        )));
    }
    spell(
        out,
        "FIRE",
        Vec3::new(0.0, 6.6, hz + 0.15),
        0.4,
        0.05,
        Mat::White,
        false,
    );
    // The engine: a red body, cab glass, a ladder on top and a white stripe.
    let truck = Vec3::new(9.0, 0.0, hz + 6.0);
    out.push(bx(truck, Vec3::new(1.3, 2.6, 4.8), Mat::Red));
    out.push(soft(bx(
        truck + Vec3::Y * 1.1,
        Vec3::new(1.35, 0.25, 4.85),
        Mat::White,
    )));
    out.push(soft(bx(
        truck + Vec3::new(0.0, 1.5, 3.6),
        Vec3::new(1.35, 0.8, 1.0),
        Mat::Glass,
    )));
    out.push(soft(bx(
        truck + Vec3::new(0.0, 2.6, -0.6),
        Vec3::new(0.6, 0.2, 4.0),
        Mat::Metal,
    )));
    // The hose tower, and an apron of concrete out front.
    let tower = Vec3::new(-hx - 3.0, 0.0, -4.0);
    out.push(Piece {
        form: Form::Box {
            at: tower,
            half: Vec3::new(3.0, 22.0, 3.0),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::Red,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 7.0),
        Vec3::new(hx, 0.12, 7.0),
        Mat::Concrete,
    )));
    out.push(soft(round(
        Vec3::new(hx + 3.0, 0.0, hz + 2.0),
        8.0,
        0.08,
        0.06,
        Mat::Metal,
    )));
    out.push(soft(bx(
        Vec3::new(hx + 4.0, 6.6, hz + 2.0),
        Vec3::new(1.0, 1.3, 0.05),
        Mat::Canvas(0),
    )));
}

/// Where a school's children run about: the playground and the playing field, in its
/// frame, as middle and half size.
pub const PLAYGROUND: (Vec2, Vec2) = (Vec2::new(22.0, 12.0), Vec2::new(9.0, 7.0));
pub const PLAYING_FIELD: (Vec2, Vec2) = (Vec2::new(-4.0, 34.0), Vec2::new(24.0, 12.0));

/// A school: a long two-storey building in warm yellow with a red roof and a little bell
/// tower, a wing to one side, a playground with swings and a slide, and a playing field
/// with goals.
fn school(out: &mut Vec<Piece>) {
    for (at, half) in [
        (Vec3::new(0.0, 0.0, -10.0), Vec3::new(28.0, 8.0, 7.0)),
        (Vec3::new(-24.0, 0.0, 4.0), Vec3::new(6.0, 8.0, 8.0)),
    ] {
        out.push(Piece {
            form: Form::Box {
                at,
                half,
                yaw: 0.0,
                roof: 3.5,
            },
            mat: Mat::Canvas(2),
            solid: true,
        });
    }
    for y in [1.5f32, 5.0] {
        out.push(soft(bx(
            Vec3::new(2.0, y, -2.95),
            Vec3::new(24.0, 1.6, 0.1),
            Mat::Glass,
        )));
    }
    out.push(soft(bx(
        Vec3::new(2.0, 0.0, -2.9),
        Vec3::new(2.0, 3.0, 0.1),
        Mat::Wood,
    )));
    let bell = Vec3::new(2.0, 11.5, -10.0);
    out.push(bx(bell, Vec3::new(1.6, 4.0, 1.6), Mat::White));
    out.push(round(bell + Vec3::Y * 4.0, 3.0, 2.2, 0.0, Mat::Red));
    spell(
        out,
        "SCHOOL",
        Vec3::new(2.0, 8.4, -2.8),
        0.32,
        0.05,
        Mat::White,
        false,
    );
    // The playground: soft sand, swings, a slide and a climbing frame.
    let (pc, ph) = PLAYGROUND;
    out.push(soft(bx(
        Vec3::new(pc.x, 0.0, pc.y),
        Vec3::new(ph.x, 0.1, ph.y),
        Mat::Sand,
    )));
    for x in [-3.0f32, 3.0] {
        out.push(soft(round(
            Vec3::new(pc.x + x, 0.0, pc.y - 4.0),
            3.0,
            0.08,
            0.08,
            Mat::Canvas(1),
        )));
    }
    out.push(soft(bx(
        Vec3::new(pc.x, 3.0, pc.y - 4.0),
        Vec3::new(3.2, 0.1, 0.1),
        Mat::Canvas(1),
    )));
    for k in 0..6 {
        out.push(soft(bx(
            Vec3::new(pc.x + 4.0, k as f32 * 0.4, pc.y + 2.0 + k as f32 * 0.6),
            Vec3::new(0.5, 0.4, 0.4),
            Mat::Canvas(0),
        )));
    }
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        out.push(soft(round(
            Vec3::new(pc.x - 4.0 + x * 1.5, 0.0, pc.y + 3.0 + z * 1.5),
            2.4,
            0.08,
            0.08,
            Mat::Canvas(3),
        )));
    }
    out.push(soft(bx(
        Vec3::new(pc.x - 4.0, 2.3, pc.y + 3.0),
        Vec3::new(1.6, 0.1, 1.6),
        Mat::Canvas(3),
    )));
    // The playing field, its lines and goals.
    let (fc, fh) = PLAYING_FIELD;
    out.push(soft(bx(
        Vec3::new(fc.x, 0.0, fc.y),
        Vec3::new(fh.x, 0.1, fh.y),
        Mat::Grass,
    )));
    out.push(soft(bx(
        Vec3::new(fc.x, 0.1, fc.y),
        Vec3::new(0.08, 0.03, fh.y),
        Mat::White,
    )));
    for side in [-1.0f32, 1.0] {
        let x = fc.x + side * (fh.x - 1.0);
        for z in [-2.5f32, 2.5] {
            out.push(soft(round(
                Vec3::new(x, 0.0, fc.y + z),
                2.2,
                0.08,
                0.08,
                Mat::White,
            )));
        }
        out.push(soft(bx(
            Vec3::new(x, 2.2, fc.y),
            Vec3::new(0.08, 0.08, 2.5),
            Mat::White,
        )));
    }
}

/// A town hall: a stone hall with a columned porch over wide steps, and a clock tower
/// rising from the middle with a flag on top.
/// How tall a town hall's walls stand, its clock tower on top.
const TOWN_HALL_TALL: f32 = 12.0;

fn town_hall(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (18.0, TOWN_HALL_TALL, 11.0);
    out.push(Piece {
        form: Form::Box {
            at: Vec3::ZERO,
            half: Vec3::new(hx, tall, hz),
            yaw: 0.0,
            roof: 4.0,
        },
        mat: Mat::Stone,
        solid: true,
    });
    for k in 0..7 {
        let x = -12.0 + k as f32 * 4.0;
        out.push(round(
            Vec3::new(x, 1.2, hz + 3.0),
            9.0,
            0.7,
            0.6,
            Mat::White,
        ));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 10.2, hz + 3.0),
            half: Vec3::new(14.0, 1.2, 3.2),
            yaw: 0.0,
            roof: 2.4,
        },
        mat: Mat::White,
        solid: true,
    });
    for (k, z) in [hz + 5.5, hz + 4.0, hz + 2.5].into_iter().enumerate() {
        out.push(bx(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(14.0 + k as f32, 0.4 * (k + 1) as f32, 1.5),
            Mat::White,
        ));
    }
    // The clock tower and its four faces.
    let t = Vec3::new(0.0, tall, -1.0);
    out.push(bx(t, Vec3::new(4.0, 16.0, 4.0), Mat::Stone));
    out.push(round(t + Vec3::Y * 16.0, 6.0, 4.6, 0.0, Mat::Copper));
    for (dx, dz) in [(0.0f32, 1.0f32), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
        let face = t + Vec3::new(dx * 4.05, 10.0, dz * 4.05);
        let half = if dz != 0.0 {
            Vec3::new(2.4, 4.8, 0.08)
        } else {
            Vec3::new(0.08, 4.8, 2.4)
        };
        // The hands keep the game's time, drawn by the client (see `clocks`).
        out.push(soft(bx(face, half, Mat::White)));
    }
    out.push(soft(round(t + Vec3::Y * 22.0, 6.0, 0.08, 0.06, Mat::Metal)));
    out.push(soft(bx(
        t + Vec3::new(1.2, 26.5, 0.0),
        Vec3::new(1.2, 1.4, 0.05),
        Mat::Canvas(1),
    )));
    // A paved square in front with a lamp each side.
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, hz + 16.0),
        Vec3::new(20.0, 0.12, 9.0),
        Mat::Concrete,
    )));
    for x in [-12.0f32, 12.0] {
        out.push(soft(round(
            Vec3::new(x, 0.0, hz + 12.0),
            4.5,
            0.12,
            0.1,
            Mat::Dark,
        )));
        out.push(soft(ball(Vec3::new(x, 4.9, hz + 12.0), 0.45, Mat::Lamp)));
    }
}

/// A farm's fields in its frame: middle, half size, and crop (0 wheat, 1 greens, 2
/// ploughed). The tractor works the first.
pub const FIELDS: [(Vec2, Vec2, u8); 3] = [
    (Vec2::new(-40.0, 30.0), Vec2::new(32.0, 26.0), 0),
    (Vec2::new(32.0, 30.0), Vec2::new(32.0, 26.0), 1),
    (Vec2::new(60.0, -40.0), Vec2::new(22.0, 24.0), 2),
];

/// The bridle path round a farm, outside its fence: the corners in order, a loop.
pub const FARM_TRAIL: [Vec2; 4] = [
    Vec2::new(-84.0, -74.0),
    Vec2::new(94.0, -74.0),
    Vec2::new(94.0, 68.0),
    Vec2::new(-84.0, 68.0),
];
/// A farm's pasture between the barn and the fields, where cows and sheep graze: its
/// middle and half size.
pub const PASTURE: (Vec2, Vec2) = (Vec2::new(47.0, -6.0), Vec2::new(33.0, 7.0));

/// A farm: a red barn with white trim, two silos, a white farmhouse with a red roof,
/// fields in rows of wheat, greens and ploughed earth, hay rolls and a fence round all.
fn farm(out: &mut Vec<Piece>) {
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-5.0, 0.0, -32.0),
            half: Vec3::new(13.0, 9.0, 9.0),
            yaw: 0.0,
            roof: 6.0,
        },
        mat: Mat::Red,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(-5.0, 0.0, -22.95),
        Vec3::new(3.5, 6.5, 0.1),
        Mat::White,
    )));
    out.push(soft(bx(
        Vec3::new(-5.0, 0.0, -22.9),
        Vec3::new(2.9, 6.0, 0.1),
        Mat::Dark,
    )));
    for x in [14.0f32, 22.0] {
        out.push(round(Vec3::new(x, 0.0, -34.0), 18.0, 3.4, 3.4, Mat::Metal));
        out.push(ball(Vec3::new(x, 18.0, -34.0), 3.4, Mat::Metal));
    }
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(-38.0, 0.0, -30.0),
            half: Vec3::new(8.0, 6.0, 6.0),
            yaw: 0.0,
            roof: 4.0,
        },
        mat: Mat::White,
        solid: true,
    });
    out.push(soft(bx(
        Vec3::new(-38.0, 0.0, -23.95),
        Vec3::new(1.0, 2.4, 0.1),
        Mat::Wood,
    )));
    // The fields, each in rows across its soil.
    for (c, h, crop) in FIELDS {
        out.push(soft(bx(
            Vec3::new(c.x, 0.0, c.y),
            Vec3::new(h.x, 0.08, h.y),
            Mat::Wood,
        )));
        let rows = (h.x * 2.0 / 3.0) as u32;
        for k in 0..rows {
            let x = c.x - h.x + 1.5 + k as f32 * 3.0;
            let (mat, tall) = match crop {
                0 => (Mat::Hay, 0.9),
                1 => (Mat::Leaf, 0.5),
                _ => (Mat::Dark, 0.18),
            };
            out.push(soft(bx(
                Vec3::new(x, 0.0, c.y),
                Vec3::new(0.7, tall, h.y - 1.0),
                mat,
            )));
        }
    }
    // Hay rolls by the barn.
    for k in 0..5 {
        let at = Vec3::new(-24.0 + k as f32 * 2.6, 0.0, -18.0 + (k % 2) as f32 * 2.6);
        out.push(round(at, 1.5, 1.1, 1.1, Mat::Hay));
    }
    fence(
        out,
        &[
            Vec2::new(-74.0, -46.0),
            Vec2::new(84.0, -46.0),
            Vec2::new(84.0, 58.0),
            Vec2::new(-74.0, 58.0),
        ],
        6.0,
        Mat::Wood,
    );
    // A sandy bridle path all the way round, for riders out on their horses.
    for (k, &a) in FARM_TRAIL.iter().enumerate() {
        let b = FARM_TRAIL[(k + 1) % FARM_TRAIL.len()];
        let (mid, d) = ((a + b) * 0.5, b - a);
        out.push(soft(turned(
            Vec3::new(mid.x, 0.0, mid.y),
            Vec3::new(d.length() * 0.5 + 1.6, 0.06, 1.6),
            (-d.y).atan2(d.x),
            Mat::Sand,
        )));
    }
}

/// Where a port's cranes stand along the quay, in its frame, and how far each one's
/// trolley runs out over the water.
pub const PORT_CRANES: [f32; 2] = [-24.0, 18.0];
pub const PORT_QUAY: f32 = 40.0;
pub const CRANE_REACH: f32 = 34.0;

/// A container port on its quay, facing the water at +z: stacks of colored containers,
/// two tall gantry cranes reaching out over the water, a warehouse and bollards.
fn port(out: &mut Vec<Piece>, seed: u32) {
    let r = |k: u32| crate::shape::hash(seed.wrapping_add(k));
    out.push(soft(bx(
        Vec3::ZERO,
        Vec3::new(62.0, 0.3, PORT_QUAY),
        Mat::Concrete,
    )));
    out.push(bx(
        Vec3::new(0.0, -6.0, PORT_QUAY - 1.0),
        Vec3::new(62.0, 6.3, 1.0),
        Mat::Concrete,
    ));
    for k in 0..12 {
        out.push(soft(round(
            Vec3::new(-55.0 + k as f32 * 10.0, 0.3, PORT_QUAY - 1.2),
            0.7,
            0.35,
            0.45,
            Mat::Dark,
        )));
    }
    // Containers in rows, stacked one to three high.
    for row in 0..5u32 {
        for col in 0..9u32 {
            let at = Vec3::new(-44.0 + col as f32 * 7.0, 0.3, -20.0 + row as f32 * 6.0);
            let high = 1 + (r(row * 31 + col) * 3.0) as u32;
            for level in 0..high {
                out.push(bx(
                    at + Vec3::Y * (level as f32 * 2.6),
                    Vec3::new(3.0, 2.55, 1.2),
                    Mat::Canvas((r(row * 97 + col * 7 + level) * 6.0) as u8),
                ));
            }
        }
    }
    // The gantry cranes: legs either side of the lane, a high beam, a boom out over the
    // water and a cab. The trolley runs along the boom, drawn by the client.
    for x in PORT_CRANES {
        for (dx, dz) in [(-4.0f32, 10.0f32), (4.0, 10.0), (-4.0, 34.0), (4.0, 34.0)] {
            out.push(bx(
                Vec3::new(x + dx, 0.3, dz),
                Vec3::new(0.6, 30.0, 0.6),
                Mat::Canvas(1),
            ));
        }
        out.push(bx(
            Vec3::new(x, 30.3, 10.0 + CRANE_REACH * 0.5),
            Vec3::new(4.6, 2.0, CRANE_REACH * 0.5 + 12.0),
            Mat::Canvas(1),
        ));
        out.push(bx(
            Vec3::new(x + 3.0, 26.0, 30.0),
            Vec3::new(1.6, 2.6, 2.0),
            Mat::White,
        ));
        for dx in [-4.0f32, 4.0] {
            out.push(soft(bx(
                Vec3::new(x + dx, 15.0, 22.0),
                Vec3::new(0.4, 0.8, 12.0),
                Mat::Canvas(1),
            )));
        }
    }
    // The warehouse at the back.
    out.push(Piece {
        form: Form::Box {
            at: Vec3::new(0.0, 0.3, -34.0),
            half: Vec3::new(40.0, 10.0, 6.0),
            yaw: 0.0,
            roof: 2.0,
        },
        mat: Mat::Concrete,
        solid: true,
    });
    for k in 0..4 {
        out.push(soft(bx(
            Vec3::new(-30.0 + k as f32 * 20.0, 0.3, -27.95),
            Vec3::new(3.5, 6.0, 0.1),
            Mat::Canvas(1),
        )));
    }
}

/// An office block over a road along z: two glass towers either side with stone fins at
/// their corners, joined from well above the traffic by four floors of offices with a
/// roof garden on top, and a mast on one tower.
fn span_tower(out: &mut Vec<Piece>) {
    const FLOOR: f32 = 4.0;
    // Wide enough underneath for the ring road and the railway beside it.
    let (gap, w, d, tall) = (24.0, 8.0, 12.0, 96.0);
    let (under, over) = (34.0, 30.0);
    for side in [-1.0f32, 1.0] {
        let at = Vec3::new(side * (gap + w), 0.0, 0.0);
        out.push(bx(at, Vec3::new(w, tall, d), Mat::Glass));
        for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            out.push(soft(bx(
                at + Vec3::new(x * (w - 0.5), 0.0, z * (d - 0.5)),
                Vec3::new(0.7, tall + 1.5, 0.7),
                Mat::Concrete,
            )));
        }
        out.push(bx(
            at + Vec3::Y * tall,
            Vec3::new(w + 0.4, 1.5, d + 0.4),
            Mat::Concrete,
        ));
        // A pale band at every floor, so the glass reads as storeys.
        for k in 1..(tall / FLOOR) as u32 {
            out.push(soft(bx(
                at + Vec3::Y * (k as f32 * FLOOR - 0.4),
                Vec3::new(w + 0.12, 0.8, d + 0.12),
                Mat::White,
            )));
        }
        // A lobby with doors at street level, facing the road.
        out.push(soft(bx(
            at + Vec3::new(-side * (w + 0.05), 0.0, 0.0),
            Vec3::new(0.1, 5.0, 4.0),
            Mat::Lamp,
        )));
    }
    let span = gap + w * 2.0;
    out.push(bx(
        Vec3::new(0.0, under, 0.0),
        Vec3::new(span, over, d),
        Mat::Glass,
    ));
    for k in 1..(over / FLOOR) as u32 {
        out.push(soft(bx(
            Vec3::new(0.0, under + k as f32 * FLOOR - 0.4, 0.0),
            Vec3::new(span + 0.12, 0.8, d + 0.12),
            Mat::White,
        )));
    }
    // A deep stone band under the opening's roof, midway and along the top.
    for y in [under, under + over * 0.5, under + over] {
        out.push(soft(bx(
            Vec3::new(0.0, y - 0.6, 0.0),
            Vec3::new(span + 0.3, 1.2, d + 0.3),
            Mat::Concrete,
        )));
    }
    let roof = under + over + 0.6;
    out.push(soft(bx(
        Vec3::new(0.0, roof, 0.0),
        Vec3::new(gap - 0.5, 0.3, d - 1.0),
        Mat::Grass,
    )));
    for x in [-10.0f32, -3.0, 5.0, 12.0] {
        out.push(soft(ball(
            Vec3::new(x, roof + 2.4, (x * 0.3).sin() * 4.0),
            1.9,
            Mat::Leaf,
        )));
    }
    out.push(round(
        Vec3::new(gap + w, tall + 1.5, 0.0),
        16.0,
        0.5,
        0.15,
        Mat::Metal,
    ));
    out.push(soft(ball(
        Vec3::new(gap + w, tall + 17.8, 0.0),
        0.5,
        Mat::Lamp,
    )));
}

/// A zoo's grand entrance with its front at `z`: a timber gatehouse arch between two
/// stone towers with thatched roofs, the name across the top, ticket halls either side,
/// turnstiles under the arch and a plaza out front with planters and flags.
fn zoo_entrance(out: &mut Vec<Piece>, z: f32) {
    // The towers and the beam that spans them, high enough to walk and fly under.
    for x in [-1.0f32, 1.0] {
        let at = Vec3::new(x * 8.0, 0.0, z);
        out.push(bx(at, Vec3::new(2.6, 14.0, 2.6), Mat::Stone));
        out.push(Piece {
            form: Form::Box {
                at: at + Vec3::Y * 14.0,
                half: Vec3::new(3.4, 0.4, 3.4),
                yaw: 0.0,
                roof: 4.5,
            },
            mat: Mat::Hay,
            solid: true,
        });
        // Lit windows up the tower faces.
        for y in [4.0f32, 9.0] {
            out.push(soft(bx(
                at + Vec3::new(0.0, y, 2.62),
                Vec3::new(0.7, 1.6, 0.05),
                Mat::Lamp,
            )));
        }
        // A flag on each tower.
        out.push(soft(round(
            at + Vec3::Y * 18.5,
            4.0,
            0.08,
            0.06,
            Mat::Metal,
        )));
        out.push(soft(bx(
            at + Vec3::new(x * 1.1, 21.0, 0.0),
            Vec3::new(1.1, 1.3, 0.05),
            Mat::Canvas(if x < 0.0 { 3 } else { 2 }),
        )));
    }
    out.push(bx(
        Vec3::new(0.0, 10.0, z),
        Vec3::new(5.4, 3.2, 1.6),
        Mat::Wood,
    ));
    // The name across the beam, in bright letters, and a board behind them.
    spell(
        out,
        "ZOO",
        Vec3::new(0.0, 10.2, z + 1.7),
        0.55,
        0.12,
        Mat::Canvas(2),
        false,
    );
    // Ticket halls either side, with thatched roofs, open counters and awnings.
    for x in [-1.0f32, 1.0] {
        let at = Vec3::new(x * 19.0, 0.0, z - 1.0);
        out.push(Piece {
            form: Form::Box {
                at,
                half: Vec3::new(8.0, 5.5, 5.0),
                yaw: 0.0,
                roof: 0.0,
            },
            mat: Mat::Wood,
            solid: true,
        });
        out.push(Piece {
            form: Form::Box {
                at: at + Vec3::Y * 5.5,
                half: Vec3::new(9.0, 0.3, 6.0),
                yaw: 0.0,
                roof: 3.5,
            },
            mat: Mat::Hay,
            solid: true,
        });
        for k in [-1.0f32, 1.0] {
            let w = at + Vec3::new(k * 4.0, 1.2, 5.02);
            out.push(soft(bx(w, Vec3::new(1.6, 1.6, 0.05), Mat::Lamp)));
            out.push(soft(bx(
                w + Vec3::new(0.0, 2.0, 0.6),
                Vec3::new(2.0, 0.12, 0.7),
                Mat::Canvas(if k < 0.0 { 3 } else { 4 }),
            )));
        }
    }
    // Turnstiles under the arch, and a rail either side to queue along.
    for k in 0..4 {
        let x = -3.6 + k as f32 * 2.4;
        out.push(soft(bx(
            Vec3::new(x, 0.0, z),
            Vec3::new(0.3, 1.0, 0.6),
            Mat::Metal,
        )));
    }
    for x in [-4.9f32, 4.9] {
        out.push(soft(bx(
            Vec3::new(x, 0.9, z + 6.0),
            Vec3::new(0.05, 0.05, 4.5),
            Mat::Metal,
        )));
    }
    // The plaza out front, planters with shrubs, and lamp posts.
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, z + 7.0),
        Vec3::new(30.0, 0.2, 7.0),
        Mat::Concrete,
    )));
    for x in [-24.0f32, -12.0, 12.0, 24.0] {
        let at = Vec3::new(x, 0.0, z + 9.0);
        out.push(bx(at, Vec3::new(1.6, 0.8, 1.6), Mat::Stone));
        out.push(ball(at + Vec3::Y * 1.8, 1.4, Mat::Hedge));
    }
    for x in [-7.0f32, 7.0] {
        let at = Vec3::new(x, 0.0, z + 12.0);
        out.push(soft(round(at, 4.0, 0.1, 0.08, Mat::Metal)));
        out.push(soft(ball(at + Vec3::Y * 4.2, 0.35, Mat::Lamp)));
    }
}

/// 3 by 5 letters, top row first, for the hillside signs.
pub fn glyph(c: char) -> [u8; 5] {
    match c {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        _ => [0; 5],
    }
}

/// The pieces of a landmark in its own frame: x along its axis, y up, z across.
/// A picnic spot's two tables in its frame, each with a bench either side along z.
pub fn picnic_tables(seed: u32) -> [Vec3; 2] {
    let r = |k: u32| hash(seed.wrapping_mul(31).wrapping_add(k));
    [
        Vec3::new(-2.5, 0.0, r(0) * 2.0 - 1.0),
        Vec3::new(2.5, 0.0, r(1) * 2.0 - 1.0),
    ]
}

/// Where people wait on a station's platform, in its frame: along x from `-half` to
/// `half`, across z from `near` to `far`, standing at height `y`, facing the track.
pub struct Platform {
    pub half: f32,
    pub near: f32,
    pub far: f32,
    pub y: f32,
}
pub const STATION_PLATFORM: Platform = Platform {
    half: 36.0,
    near: 4.6,
    far: 8.6,
    y: 1.0,
};
pub const METRO_WAIT: Platform = Platform {
    half: METRO_PLATFORM - 6.0,
    near: 4.4,
    far: 7.6,
    y: 1.0,
};
/// A bus stop's bench in its frame, and the curb people stand by.
pub const BUS_BENCH: Vec3 = Vec3::new(0.0, 0.5, -0.5);

pub fn pieces(kind: Kind, seed: u32, text: &str) -> Vec<Piece> {
    let r = |k: u32| hash(seed.wrapping_mul(31).wrapping_add(k));
    let mut out = Vec::new();
    match kind {
        Kind::Stones if text == "eye" => {
            // A ring of tall stones round the eye, the shaft down to the lagoon vault, so
            // it reads as a place people made welcome and not just a hole: each with a
            // lantern on top to see it by, and a flat stone to stand on at each quarter.
            let n = 16;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let at = Vec3::new(a.cos() * 34.0, -8.0, a.sin() * 34.0);
                out.push(Piece {
                    form: Form::Box {
                        at,
                        half: Vec3::new(1.0, 15.0, 1.6),
                        yaw: -a,
                        roof: 0.0,
                    },
                    mat: Mat::Stone,
                    solid: true,
                });
                if i % 2 == 0 {
                    out.push(soft(bx(
                        at + Vec3::Y * 15.0,
                        Vec3::new(0.7, 1.2, 0.7),
                        Mat::Lamp,
                    )));
                    out.push(bx(at + Vec3::Y * 16.2, Vec3::new(1.0, 0.4, 1.0), Mat::Dark));
                }
                if i % 4 == 0 {
                    out.push(bx(
                        Vec3::new(a.cos() * 40.0, -6.0, a.sin() * 40.0),
                        Vec3::new(3.0, 8.0, 3.0),
                        Mat::Stone,
                    ));
                }
            }
        }
        Kind::Stones => {
            // A ring of standing stones, some pairs capped, and a flat stone in the middle.
            let n = 10;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let capped = i % 4 < 2 && i < 8;
                let h = if capped { 4.4 } else { 3.0 + r(i) * 1.6 };
                out.push(Piece {
                    form: Form::Box {
                        at: Vec3::new(a.cos() * 9.0, -0.3, a.sin() * 9.0),
                        half: Vec3::new(0.55, h, 0.95),
                        yaw: -a,
                        roof: 0.0,
                    },
                    mat: Mat::Stone,
                    solid: true,
                });
                if capped && i % 4 == 0 {
                    let mid = a + std::f32::consts::TAU / n as f32 * 0.5;
                    out.push(Piece {
                        form: Form::Box {
                            at: Vec3::new(mid.cos() * 8.6, 4.1, mid.sin() * 8.6),
                            half: Vec3::new(0.6, 0.8, 3.6),
                            yaw: -mid,
                            roof: 0.0,
                        },
                        mat: Mat::Stone,
                        solid: true,
                    });
                }
            }
            out.push(bx(Vec3::ZERO, Vec3::new(1.6, 0.8, 0.9), Mat::Stone));
        }
        Kind::Turbine => {
            out.push(round(Vec3::ZERO, 40.0, 1.7, 1.0, Mat::White));
            out.push(bx(
                Vec3::new(-0.6, 39.6, 0.0),
                Vec3::new(3.0, 2.6, 1.3),
                Mat::White,
            ));
            out.push(soft(round(
                Vec3::new(ROTOR.x, ROTOR.y - 0.7, 0.0),
                1.4,
                0.7,
                0.7,
                Mat::White,
            )));
        }
        Kind::Campsite => {
            for i in 0..3 {
                let a = 0.6 + i as f32 * 2.0 + r(i) * 0.4;
                let at = Vec3::new(a.cos() * 6.0, 0.0, a.sin() * 6.0);
                out.push(Piece {
                    form: Form::Box {
                        at,
                        half: Vec3::new(1.9, 0.25, 1.5),
                        yaw: -a,
                        roof: 1.9,
                    },
                    mat: Mat::Canvas((r(i + 10) * 6.0) as u8),
                    solid: true,
                });
            }
            out.push(soft(round(CAMPFIRE, 0.35, 1.2, 1.0, Mat::Stone)));
            for s in [-1.0f32, 1.0] {
                out.push(bx(
                    Vec3::new(0.0, 0.0, s * 2.6),
                    Vec3::new(1.4, 0.45, 0.3),
                    Mat::Wood,
                ));
            }
        }
        Kind::Picnic => {
            for c in picnic_tables(seed) {
                out.push(bx(c + Vec3::Y * 0.7, Vec3::new(1.2, 0.08, 0.5), Mat::Wood));
                for s in [-1.0f32, 1.0] {
                    out.push(soft(bx(
                        c + Vec3::new(0.0, 0.0, s * 0.85),
                        Vec3::new(1.2, 0.45, 0.16),
                        Mat::Wood,
                    )));
                    out.push(soft(bx(
                        c + Vec3::new(s * 0.9, 0.0, 0.0),
                        Vec3::new(0.06, 0.7, 0.4),
                        Mat::Wood,
                    )));
                }
            }
            out.push(soft(round(
                Vec3::new(0.0, 0.0, 2.2),
                0.9,
                0.35,
                0.3,
                Mat::Metal,
            )));
        }
        Kind::PhoneBox => {
            out.push(bx(Vec3::ZERO, Vec3::new(0.62, 2.45, 0.62), Mat::Red));
            out.push(soft(bx(
                Vec3::new(0.0, 2.45, 0.0),
                Vec3::new(0.7, 0.18, 0.7),
                Mat::Red,
            )));
            for s in [-1.0f32, 1.0] {
                out.push(soft(bx(
                    Vec3::new(s * 0.63, 0.9, 0.0),
                    Vec3::new(0.02, 1.2, 0.45),
                    Mat::Glass,
                )));
                out.push(soft(bx(
                    Vec3::new(0.0, 0.9, s * 0.63),
                    Vec3::new(0.45, 1.2, 0.02),
                    Mat::Glass,
                )));
            }
        }
        Kind::Umbrellas => {
            for i in 0..4u32 {
                let at = Vec3::new(i as f32 * 4.5 - 6.75, 0.0, r(i) * 3.0 - 1.5);
                out.push(soft(round(at, 2.3, 0.06, 0.05, Mat::White)));
                out.push(round(
                    at + Vec3::Y * 2.0,
                    0.6,
                    1.9,
                    0.08,
                    Mat::Canvas((i + seed) as u8 % 6),
                ));
                out.push(soft(bx(
                    at + Vec3::new(0.4, 0.0, 1.6),
                    Vec3::new(0.95, 0.03, 0.45),
                    Mat::Canvas((i + seed + 3) as u8 % 6),
                )));
            }
        }
        Kind::Scarecrow => {
            out.push(soft(round(Vec3::ZERO, 2.6, 0.08, 0.08, Mat::Wood)));
            out.push(soft(bx(
                Vec3::new(0.0, 1.85, 0.0),
                Vec3::new(1.1, 0.09, 0.09),
                Mat::Wood,
            )));
            out.push(soft(bx(
                Vec3::new(0.0, 1.1, 0.0),
                Vec3::new(0.32, 0.8, 0.18),
                Mat::Canvas(seed as u8 % 6),
            )));
            out.push(soft(round(
                Vec3::new(0.0, 2.25, 0.0),
                0.42,
                0.22,
                0.2,
                Mat::Hay,
            )));
            out.push(soft(round(
                Vec3::new(0.0, 2.62, 0.0),
                0.08,
                0.48,
                0.48,
                Mat::Dark,
            )));
            out.push(soft(round(
                Vec3::new(0.0, 2.7, 0.0),
                0.32,
                0.24,
                0.18,
                Mat::Dark,
            )));
            for i in 0..5u32 {
                let a = i as f32 * 1.25 + r(i);
                let d = 5.0 + r(i + 5) * 5.0;
                out.push(round(
                    Vec3::new(a.cos() * d, 0.0, a.sin() * d),
                    1.3,
                    0.85,
                    0.85,
                    Mat::Hay,
                ));
            }
        }
        Kind::Telescope => {
            out.push(bx(Vec3::ZERO, Vec3::new(3.2, 6.0, 3.2), Mat::White));
            out.push(round(Vec3::new(0.0, 6.0, 0.0), 4.0, 1.6, 1.2, Mat::White));
            out.push(round(Vec3::new(0.0, 10.0, 0.0), 4.5, 2.0, 12.5, Mat::White));
            out.push(soft(round(
                Vec3::new(0.0, 14.5, 0.0),
                8.0,
                0.15,
                0.1,
                Mat::Metal,
            )));
            out.push(soft(bx(
                Vec3::new(0.0, 22.5, 0.0),
                Vec3::new(0.5, 0.9, 0.5),
                Mat::Metal,
            )));
        }
        Kind::DriveIn => {
            out.push(bx(
                Vec3::new(0.0, 3.0, 0.0),
                Vec3::new(16.0, 13.0, 0.4),
                Mat::Screen,
            ));
            out.push(soft(bx(
                Vec3::new(0.0, 3.0, -0.45),
                Vec3::new(16.2, 13.0, 0.05),
                Mat::Dark,
            )));
            for x in [-12.0f32, -4.0, 4.0, 12.0] {
                out.push(round(Vec3::new(x, 0.0, -1.0), 3.0, 0.4, 0.4, Mat::Dark));
            }
            out.push(bx(
                Vec3::new(0.0, 0.0, 42.0),
                Vec3::new(2.6, 3.0, 2.0),
                Mat::Wood,
            ));
        }
        Kind::Sign => {
            let cell = SIGN_CELL;
            let width = text.chars().count() as f32 * 4.0 * cell - cell;
            spell(&mut out, text, Vec3::Y, cell, 0.5, Mat::White, true);
            for i in 0..text.chars().count() {
                out.push(soft(bx(
                    Vec3::new(
                        (i as f32 * 4.0 + 1.0) * cell - width * 0.5 + cell * 0.5,
                        0.0,
                        0.8,
                    ),
                    Vec3::new(0.15, 1.0, 0.15),
                    Mat::Wood,
                )));
            }
        }
        Kind::Ferris => {
            // The wheel itself turns, drawn by the client. These are its legs and deck.
            for z in [-2.6f32, 2.6] {
                for x in [-7.0f32, 7.0] {
                    out.push(turned(
                        Vec3::new(x * 0.5, 0.0, z),
                        Vec3::new(0.45, FERRIS_HUB.y, 0.45),
                        0.0,
                        Mat::Metal,
                    ));
                }
                out.push(bx(
                    Vec3::new(0.0, FERRIS_HUB.y - 0.6, z),
                    Vec3::new(0.6, 1.2, 0.6),
                    Mat::Metal,
                ));
            }
            out.push(bx(Vec3::ZERO, Vec3::new(9.0, 0.6, 5.0), Mat::Wood));
            out.push(bx(
                Vec3::new(11.0, 0.0, 6.0),
                Vec3::new(1.5, 2.6, 1.5),
                Mat::Canvas(0),
            ));
            for i in 0..6u32 {
                let x = -15.0 + i as f32 * 6.0;
                out.push(soft(round(
                    Vec3::new(x, 0.0, 11.0),
                    3.0,
                    0.08,
                    0.08,
                    Mat::Wood,
                )));
                out.push(soft(round(
                    Vec3::new(x, 3.0, 11.0),
                    0.9,
                    0.5,
                    0.0,
                    Mat::Canvas((i + seed) as u8 % 6),
                )));
            }
        }
        Kind::Swing => {
            // A big lone tree with a rope swing on its long branch. The swing is drawn by
            // the client, swaying.
            out.push(round(Vec3::ZERO, 11.0, 1.5, 1.0, Mat::Wood));
            out.push(ball(Vec3::new(0.0, 15.0, 0.0), 7.5, Mat::Leaf));
            out.push(ball(Vec3::new(4.5, 13.0, 2.0), 4.5, Mat::Leaf));
            out.push(ball(Vec3::new(-3.5, 13.5, -2.5), 4.8, Mat::Leaf));
            out.push(bx(
                SWING_PIVOT - Vec3::new(4.2, 0.6, 0.0),
                Vec3::new(5.4, 0.6, 0.5),
                Mat::Wood,
            ));
        }
        Kind::Maze => {
            // A hedge maze, carved from a grid by a seeded walk so every cell connects.
            const N: usize = 9;
            const CELL: f32 = 4.5;
            let mut open = [[false; 4]; N * N];
            let mut seen = [false; N * N];
            let mut stack = vec![0usize];
            seen[0] = true;
            let mut k = 0u32;
            while let Some(&c) = stack.last() {
                let (x, z) = (c % N, c / N);
                let mut next = Vec::new();
                for (d, (dx, dz)) in [(1i32, 0i32), (0, 1), (-1, 0), (0, -1)].iter().enumerate() {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    if nx >= 0 && nz >= 0 && (nx as usize) < N && (nz as usize) < N {
                        let n = nz as usize * N + nx as usize;
                        if !seen[n] {
                            next.push((d, n));
                        }
                    }
                }
                if next.is_empty() {
                    stack.pop();
                    continue;
                }
                k += 1;
                let (d, n) = next[(r(k) * next.len() as f32) as usize % next.len()];
                open[c][d] = true;
                open[n][(d + 2) % 4] = true;
                seen[n] = true;
                stack.push(n);
            }
            let half = N as f32 * CELL * 0.5;
            let wall = |out: &mut Vec<Piece>, at: Vec3, along_x: bool| {
                let half = if along_x {
                    Vec3::new(CELL * 0.5 + 0.35, 2.4, 0.35)
                } else {
                    Vec3::new(0.35, 2.4, CELL * 0.5 + 0.35)
                };
                out.push(bx(at, half, Mat::Hedge));
            };
            for z in 0..N {
                for x in 0..N {
                    let c = z * N + x;
                    let (cx, cz) = (
                        x as f32 * CELL - half + CELL * 0.5,
                        z as f32 * CELL - half + CELL * 0.5,
                    );
                    // East and south walls of each cell, plus the outer west and north.
                    let gate = (z == N / 2 && x == N - 1) || (z == N / 2 && x == 0);
                    if !open[c][0] && !(gate && x == N - 1) {
                        wall(&mut out, Vec3::new(cx + CELL * 0.5, 0.0, cz), false);
                    }
                    if !open[c][1] {
                        wall(&mut out, Vec3::new(cx, 0.0, cz + CELL * 0.5), true);
                    }
                    if x == 0 && !gate {
                        wall(&mut out, Vec3::new(cx - CELL * 0.5, 0.0, cz), false);
                    }
                    if z == 0 {
                        wall(&mut out, Vec3::new(cx, 0.0, cz - CELL * 0.5), true);
                    }
                }
            }
            // Something to find in the middle.
            out.push(round(Vec3::ZERO, 1.2, 0.8, 0.6, Mat::Stone));
            out.push(ball(Vec3::new(0.0, 1.8, 0.0), 0.6, Mat::Canvas(2)));
        }
        Kind::Sandcastle => {
            out.push(bx(Vec3::ZERO, Vec3::new(6.0, 2.0, 6.0), Mat::Sand));
            for (x, z) in [(-6.0f32, -6.0f32), (6.0, -6.0), (-6.0, 6.0), (6.0, 6.0)] {
                out.push(round(Vec3::new(x, 0.0, z), 5.0, 1.8, 1.5, Mat::Sand));
                for k in 0..4 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_2;
                    out.push(soft(bx(
                        Vec3::new(x + a.cos() * 1.3, 5.0, z + a.sin() * 1.3),
                        Vec3::new(0.35, 0.6, 0.35),
                        Mat::Sand,
                    )));
                }
            }
            out.push(round(Vec3::new(0.0, 2.0, 0.0), 6.0, 2.8, 2.2, Mat::Sand));
            out.push(round(Vec3::new(0.0, 8.0, 0.0), 2.5, 2.3, 0.0, Mat::Sand));
            out.push(soft(round(
                Vec3::new(0.0, 10.5, 0.0),
                2.5,
                0.06,
                0.06,
                Mat::Wood,
            )));
            out.push(soft(bx(
                Vec3::new(0.6, 12.2, 0.0),
                Vec3::new(0.6, 0.7, 0.03),
                Mat::Canvas(0),
            )));
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 6.1),
                Vec3::new(1.0, 1.6, 0.2),
                Mat::Dark,
            )));
            // A bucket and spade left behind.
            out.push(soft(round(
                Vec3::new(10.0, 0.0, 3.0),
                1.0,
                0.6,
                0.8,
                Mat::Canvas(1),
            )));
            out.push(soft(bx(
                Vec3::new(10.0, 0.0, 5.0),
                Vec3::new(0.12, 1.4, 0.3),
                Mat::Canvas(2),
            )));
        }
        Kind::Snowman => {
            out.push(ball(Vec3::new(0.0, 2.6, 0.0), 3.4, Mat::Snow));
            out.push(ball(Vec3::new(0.0, 7.3, 0.0), 2.5, Mat::Snow));
            out.push(ball(Vec3::new(0.0, 10.9, 0.0), 1.8, Mat::Snow));
            out.push(round(Vec3::new(0.0, 12.4, 0.0), 0.15, 2.0, 2.0, Mat::Dark));
            out.push(round(Vec3::new(0.0, 12.5, 0.0), 1.9, 1.2, 1.1, Mat::Dark));
            out.push(soft(bx(
                Vec3::new(2.4, 10.8, 0.0),
                Vec3::new(1.0, 0.25, 0.25),
                Mat::Canvas(5),
            )));
            out.push(soft(round(
                Vec3::new(0.0, 8.9, 0.0),
                0.8,
                1.9,
                1.7,
                Mat::Canvas(0),
            )));
            for z in [-0.6f32, 0.6] {
                out.push(soft(ball(Vec3::new(1.6, 11.5, z), 0.2, Mat::Dark)));
            }
            for y in [6.4f32, 7.4, 8.2] {
                out.push(soft(ball(Vec3::new(2.45, y, 0.0), 0.25, Mat::Dark)));
            }
            for z in [-1.0f32, 1.0] {
                out.push(soft(turned(
                    Vec3::new(0.0, 7.6, z * 3.6),
                    Vec3::new(0.1, 0.1, 1.6),
                    z * 0.3,
                    Mat::Wood,
                )));
            }
        }
        Kind::Bench => {
            out.push(bx(
                Vec3::new(0.0, 0.9, 0.0),
                Vec3::new(2.2, 0.08, 0.45),
                Mat::Wood,
            ));
            out.push(soft(bx(
                Vec3::new(0.0, 1.0, -0.45),
                Vec3::new(2.2, 0.5, 0.06),
                Mat::Wood,
            )));
            for x in [-1.9f32, 1.9] {
                out.push(soft(bx(
                    Vec3::new(x, 0.0, 0.0),
                    Vec3::new(0.08, 0.9, 0.45),
                    Mat::Metal,
                )));
            }
            out.push(soft(round(
                Vec3::new(3.2, 0.0, 0.0),
                0.8,
                0.3,
                0.35,
                Mat::Metal,
            )));
        }
        Kind::Gnomes => {
            // A crowd of garden gnomes gathered as if for a meeting.
            for i in 0..14u32 {
                let a = i as f32 * 2.4 + r(i);
                let d = if i == 0 { 0.0 } else { 2.0 + r(i + 30) * 4.0 };
                let at = Vec3::new(a.cos() * d, 0.0, a.sin() * d);
                let s = if i == 0 { 1.6 } else { 0.9 + r(i + 60) * 0.4 };
                out.push(round(
                    at,
                    0.9 * s,
                    0.4 * s,
                    0.32 * s,
                    Mat::Canvas((i * 7 + seed) as u8 % 6),
                ));
                out.push(soft(ball(at + Vec3::Y * 1.15 * s, 0.3 * s, Mat::Skin)));
                out.push(soft(ball(
                    at + Vec3::new(0.2, 0.95, 0.0) * s,
                    0.25 * s,
                    Mat::White,
                )));
                out.push(soft(round(
                    at + Vec3::Y * 1.3 * s,
                    0.9 * s,
                    0.33 * s,
                    0.0,
                    Mat::Red,
                )));
            }
        }
        Kind::Beehives => {
            for i in 0..6u32 {
                let at = Vec3::new(i as f32 * 2.2 - 5.5, 0.0, r(i) * 0.8);
                let tall = 1.2 + (i % 3) as f32 * 0.5;
                out.push(bx(at, Vec3::new(0.6, tall, 0.6), Mat::White));
                out.push(soft(Piece {
                    form: Form::Box {
                        at: at + Vec3::Y * tall,
                        half: Vec3::new(0.7, 0.05, 0.7),
                        yaw: 0.0,
                        roof: 0.35,
                    },
                    mat: Mat::Canvas(if i % 2 == 0 { 2 } else { 5 }),
                    solid: false,
                }));
            }
        }
        Kind::Pumpkins => {
            out.push(ball(Vec3::new(0.0, 2.4, 0.0), 3.4, Mat::Canvas(5)));
            out.push(soft(round(
                Vec3::new(0.0, 5.4, 0.0),
                1.2,
                0.35,
                0.25,
                Mat::Wood,
            )));
            for i in 0..18u32 {
                let a = i as f32 * 2.4 + r(i);
                let d = 5.0 + r(i + 40) * 9.0;
                let s = 0.4 + r(i + 80) * 0.5;
                out.push(soft(ball(
                    Vec3::new(a.cos() * d, s * 0.7, a.sin() * d),
                    s,
                    Mat::Canvas(5),
                )));
            }
        }
        Kind::BusStop => {
            out.push(bx(
                Vec3::new(0.0, 0.0, -0.9),
                Vec3::new(2.2, 2.4, 0.06),
                Mat::Glass,
            ));
            out.push(bx(
                Vec3::new(0.0, 2.4, 0.0),
                Vec3::new(2.4, 0.12, 1.1),
                Mat::Metal,
            ));
            for x in [-2.2f32, 2.2] {
                out.push(soft(bx(
                    Vec3::new(x, 0.0, 0.9),
                    Vec3::new(0.06, 2.4, 0.06),
                    Mat::Metal,
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 0.5, -0.5),
                Vec3::new(1.8, 0.06, 0.3),
                Mat::Wood,
            )));
            out.push(soft(round(
                Vec3::new(3.4, 0.0, 0.8),
                3.0,
                0.06,
                0.06,
                Mat::Metal,
            )));
            out.push(soft(ball(Vec3::new(3.4, 3.2, 0.8), 0.45, Mat::Canvas(1))));
        }
        Kind::WaterTower => {
            for (x, z) in [(-3.0f32, -3.0f32), (3.0, -3.0), (-3.0, 3.0), (3.0, 3.0)] {
                out.push(round(Vec3::new(x, 0.0, z), 14.0, 0.4, 0.35, Mat::Metal));
            }
            out.push(round(Vec3::new(0.0, 14.0, 0.0), 6.0, 4.6, 4.6, Mat::White));
            out.push(round(Vec3::new(0.0, 20.0, 0.0), 2.6, 4.8, 0.3, Mat::Red));
            out.push(soft(round(
                Vec3::new(0.0, 16.4, 0.0),
                1.2,
                4.65,
                4.65,
                Mat::Canvas(1),
            )));
        }
        Kind::HotSpring => {
            // Warm water in a ring of rocks, steaming (the steam is drawn by the client).
            out.push(soft(round(
                Vec3::new(0.0, -0.4, 0.0),
                0.55,
                6.2,
                6.4,
                Mat::Water,
            )));
            for i in 0..11u32 {
                let a = i as f32 / 11.0 * std::f32::consts::TAU + r(i) * 0.3;
                let s = 1.1 + r(i + 20) * 1.0;
                out.push(ball(
                    Vec3::new(a.cos() * 6.8, 0.2, a.sin() * 6.8),
                    s,
                    Mat::Stone,
                ));
            }
            out.push(soft(bx(
                Vec3::new(9.5, 0.0, 0.0),
                Vec3::new(0.8, 1.4, 0.06),
                Mat::Wood,
            )));
        }
        Kind::Chess => {
            // A giant chess game left half played.
            const CELL: f32 = 3.0;
            out.push(bx(Vec3::ZERO, Vec3::new(12.4, 0.3, 12.4), Mat::White));
            for i in 0..8 {
                for j in 0..8 {
                    if (i + j) % 2 == 1 {
                        out.push(soft(bx(
                            Vec3::new((i as f32 - 3.5) * CELL, 0.0, (j as f32 - 3.5) * CELL),
                            Vec3::new(CELL * 0.5, 0.33, CELL * 0.5),
                            Mat::Dark,
                        )));
                    }
                }
            }
            // File, rank, piece (pawn, rook, knight, bishop, queen, king) and side.
            let set: [(i32, i32, u8, bool); 18] = [
                (0, 0, 1, true),
                (4, 0, 5, true),
                (5, 0, 1, true),
                (0, 1, 0, true),
                (1, 1, 0, true),
                (2, 2, 0, true),
                (5, 1, 0, true),
                (6, 2, 0, true),
                (3, 3, 0, true),
                (2, 3, 2, true),
                (4, 5, 4, true),
                (0, 6, 0, false),
                (1, 5, 0, false),
                (5, 6, 0, false),
                (6, 6, 0, false),
                (7, 7, 1, false),
                (6, 7, 5, false),
                (2, 6, 3, false),
            ];
            for (f, k, piece, light) in set {
                let mat = if light { Mat::White } else { Mat::Dark };
                let at = Vec3::new((f as f32 - 3.5) * CELL, 0.33, (k as f32 - 3.5) * CELL);
                let base = [1.4, 3.0, 2.2, 3.2, 3.6, 4.0][piece as usize];
                out.push(round(at, 0.4, 1.15, 1.05, mat));
                out.push(round(
                    at + Vec3::Y * 0.4,
                    base,
                    0.85,
                    if piece == 1 { 0.85 } else { 0.4 },
                    mat,
                ));
                let up = at + Vec3::Y * (0.4 + base);
                match piece {
                    0 => out.push(ball(up + Vec3::Y * 0.4, 0.65, mat)),
                    1 => {
                        out.push(round(up, 0.8, 1.1, 1.1, mat));
                        for q in 0..4 {
                            let a = q as f32 * std::f32::consts::FRAC_PI_2;
                            out.push(soft(bx(
                                up + Vec3::new(a.cos() * 0.75, 0.8, a.sin() * 0.75),
                                Vec3::new(0.3, 0.45, 0.3),
                                mat,
                            )));
                        }
                    }
                    2 => out.push(turned(
                        up + Vec3::new(0.3, -0.4, 0.0),
                        Vec3::new(1.0, 1.8, 0.45),
                        if light { 0.0 } else { std::f32::consts::PI },
                        mat,
                    )),
                    3 => {
                        out.push(ball(up + Vec3::Y * 0.6, 0.8, mat));
                        out.push(soft(ball(up + Vec3::Y * 1.55, 0.28, mat)));
                    }
                    4 => {
                        out.push(ball(up + Vec3::Y * 0.6, 0.95, mat));
                        out.push(soft(round(up + Vec3::Y * 1.3, 0.6, 0.5, 0.75, mat)));
                    }
                    _ => {
                        out.push(round(up, 0.9, 0.6, 1.0, mat));
                        out.push(soft(bx(
                            up + Vec3::Y * 0.9,
                            Vec3::new(0.15, 1.3, 0.15),
                            mat,
                        )));
                        out.push(soft(bx(up + Vec3::Y * 1.5, Vec3::new(0.5, 0.3, 0.15), mat)));
                    }
                }
            }
            // A taken piece knocked off the board.
            out.push(soft(ball(Vec3::new(14.5, 0.6, 5.0), 0.65, Mat::Dark)));
        }
        Kind::Cairns => {
            // Stones balanced in stacks by walkers, some of them improbably tall.
            for i in 0..7u32 {
                let a = i as f32 * 2.3 + r(i);
                let d = if i == 0 { 0.0 } else { 3.0 + r(i + 10) * 5.0 };
                let mut at = Vec3::new(a.cos() * d, -0.6, a.sin() * d);
                let n = 4 + (r(i + 20) * 5.0) as u32;
                for k in 0..n {
                    let w = (1.3 - k as f32 * 0.12) * (0.8 + r(i * 13 + k) * 0.4);
                    let h = 0.35 + r(i * 17 + k) * 0.35;
                    out.push(round(at, h, w, w * 0.85, Mat::Stone));
                    at += Vec3::new((r(i * 19 + k) - 0.5) * 0.3, h, (r(i * 23 + k) - 0.5) * 0.3);
                }
                out.push(soft(ball(at + Vec3::Y * 0.25, 0.3, Mat::Stone)));
            }
        }
        Kind::Treehouse => {
            // A big tree with a little house in its arms and a ladder up.
            out.push(round(Vec3::ZERO, 16.0, 1.7, 1.1, Mat::Wood));
            out.push(bx(
                Vec3::new(0.0, 8.0, 0.0),
                Vec3::new(4.2, 0.4, 4.2),
                Mat::Wood,
            ));
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.3, 8.4, 0.3),
                    half: Vec3::new(2.6, 2.6, 2.2),
                    yaw: 0.0,
                    roof: 1.6,
                },
                mat: Mat::Canvas(0),
                solid: true,
            });
            for z in [-1.0f32, 1.0] {
                out.push(soft(bx(
                    Vec3::new(2.9, 9.4, z * 1.0),
                    Vec3::new(0.05, 0.9, 0.55),
                    Mat::Glass,
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.3, 8.4, 2.53),
                Vec3::new(0.7, 1.9, 0.05),
                Mat::Wood,
            )));
            for x in [-0.5f32, 0.5] {
                out.push(soft(bx(
                    Vec3::new(x, 0.0, 4.4),
                    Vec3::new(0.08, 8.4, 0.08),
                    Mat::Wood,
                )));
            }
            for k in 0..10 {
                out.push(soft(bx(
                    Vec3::new(0.0, 0.6 + k as f32 * 0.8, 4.4),
                    Vec3::new(0.5, 0.08, 0.06),
                    Mat::Wood,
                )));
            }
            out.push(ball(Vec3::new(-0.5, 18.0, -0.5), 5.8, Mat::Leaf));
            out.push(ball(Vec3::new(-5.0, 14.5, -3.5), 3.6, Mat::Leaf));
            out.push(ball(Vec3::new(4.0, 15.5, -4.5), 3.4, Mat::Leaf));
            out.push(soft(ball(Vec3::new(3.6, 7.2, 3.6), 0.3, Mat::Lamp)));
        }
        Kind::Well => {
            // A wishing well with a little roof, a bucket and a crank.
            out.push(round(Vec3::ZERO, 1.4, 1.9, 1.8, Mat::Stone));
            out.push(soft(round(Vec3::Y * 1.4, 0.04, 1.45, 1.45, Mat::Dark)));
            for x in [-1.6f32, 1.6] {
                out.push(bx(
                    Vec3::new(x, 1.4, 0.0),
                    Vec3::new(0.15, 2.5, 0.15),
                    Mat::Wood,
                ));
            }
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.0, 3.9, 0.0),
                    half: Vec3::new(2.2, 0.1, 1.6),
                    yaw: 0.0,
                    roof: 1.0,
                },
                mat: Mat::Canvas(0),
                solid: true,
            });
            out.push(soft(bx(
                Vec3::new(0.0, 3.1, 0.0),
                Vec3::new(1.6, 0.2, 0.1),
                Mat::Wood,
            )));
            out.push(soft(round(
                Vec3::new(0.0, 2.3, 0.0),
                0.8,
                0.03,
                0.03,
                Mat::Dark,
            )));
            out.push(soft(round(
                Vec3::new(0.0, 1.7, 0.0),
                0.6,
                0.32,
                0.42,
                Mat::Wood,
            )));
            out.push(soft(bx(
                Vec3::new(1.95, 3.0, 0.3),
                Vec3::new(0.1, 0.1, 0.4),
                Mat::Metal,
            )));
        }
        Kind::Lemonade => {
            // A lemonade stand with a striped awning and a handwritten sign.
            out.push(bx(Vec3::ZERO, Vec3::new(1.6, 1.2, 0.6), Mat::Wood));
            for (x, z) in [(-1.5f32, -0.5f32), (1.5, -0.5), (-1.5, 0.5), (1.5, 0.5)] {
                out.push(soft(bx(
                    Vec3::new(x, 1.2, z),
                    Vec3::new(0.06, 1.2, 0.06),
                    Mat::Wood,
                )));
            }
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.0, 2.4, 0.0),
                    half: Vec3::new(1.9, 0.06, 0.9),
                    yaw: 0.0,
                    roof: 0.5,
                },
                mat: Mat::Canvas(2),
                solid: true,
            });
            out.push(soft(bx(
                Vec3::new(0.0, 0.35, 0.63),
                Vec3::new(1.3, 0.6, 0.03),
                Mat::White,
            )));
            out.push(soft(round(
                Vec3::new(0.7, 1.2, 0.0),
                0.6,
                0.26,
                0.22,
                Mat::Canvas(2),
            )));
            for k in 0..4 {
                out.push(soft(round(
                    Vec3::new(-0.8 + k as f32 * 0.35, 1.2, 0.25),
                    0.25,
                    0.1,
                    0.12,
                    Mat::White,
                )));
            }
            for k in 0..5u32 {
                out.push(soft(ball(
                    Vec3::new(
                        2.4 + r(k) * 0.5,
                        0.25 + k as f32 * 0.05,
                        r(k + 9) * 0.6 - 0.3,
                    ),
                    0.2,
                    Mat::Canvas(2),
                )));
            }
            out.push(soft(round(
                Vec3::new(0.0, 0.0, -1.2),
                0.7,
                0.3,
                0.25,
                Mat::Wood,
            )));
        }
        Kind::Igloo => {
            // A snow dome, half sunk in the drift, warm light at its door.
            out.push(ball(Vec3::new(0.0, -0.8, 0.0), 5.2, Mat::Snow));
            out.push(bx(
                Vec3::new(0.0, -0.5, 5.0),
                Vec3::new(1.5, 2.8, 1.8),
                Mat::Snow,
            ));
            out.push(soft(bx(
                Vec3::new(0.0, -0.4, 6.82),
                Vec3::new(0.85, 2.0, 0.03),
                Mat::Lamp,
            )));
            out.push(soft(round(
                Vec3::new(7.0, -0.02, -3.0),
                0.06,
                0.9,
                0.9,
                Mat::Dark,
            )));
            out.push(soft(bx(
                Vec3::new(4.5, 0.0, 6.5),
                Vec3::new(1.4, 0.3, 0.5),
                Mat::Red,
            )));
        }
        Kind::Laundry => {
            // A washing line hung out to dry, sheets and socks and all.
            for x in [-8.0f32, 8.0] {
                out.push(round(Vec3::new(x, 0.0, 0.0), 3.2, 0.14, 0.12, Mat::Wood));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 3.0, 0.0),
                Vec3::new(8.0, 0.03, 0.03),
                Mat::Dark,
            )));
            let mut x = -7.0;
            let mut k = 0u32;
            while x < 6.8 {
                let wide = if r(k) < 0.5 { 0.9 } else { 0.3 };
                let tall = if wide > 0.5 { 1.6 } else { 0.5 };
                out.push(soft(bx(
                    Vec3::new(x + wide, 3.0 - tall * 1.0, 0.0),
                    Vec3::new(wide, tall, 0.03),
                    if r(k + 40) < 0.4 {
                        Mat::White
                    } else {
                        Mat::Canvas((r(k + 80) * 6.0) as u8)
                    },
                )));
                x += wide * 2.0 + 0.25;
                k += 1;
            }
            out.push(soft(round(
                Vec3::new(5.0, 0.0, 1.6),
                0.7,
                0.6,
                0.75,
                Mat::Hay,
            )));
        }
        Kind::Sundial => {
            // A great stone sundial on steps, its blade casting the hour.
            out.push(round(Vec3::ZERO, 0.6, 8.0, 7.6, Mat::Stone));
            out.push(round(Vec3::Y * 0.6, 0.6, 6.6, 6.4, Mat::White));
            for k in 0..12u32 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                out.push(soft(bx(
                    Vec3::new(a.cos() * 5.6, 1.2, a.sin() * 5.6),
                    Vec3::new(0.25, 0.06, 0.25),
                    Mat::Dark,
                )));
            }
            for k in 0..5 {
                let h = 5.0 - k as f32 * 1.0;
                out.push(bx(
                    Vec3::new(k as f32 * 0.9, 1.2, 0.0),
                    Vec3::new(0.45, h, 0.15),
                    Mat::Metal,
                ));
            }
        }
        Kind::Circus => {
            // A striped big top with its flag, and a wagon parked round the side.
            out.push(round(Vec3::ZERO, 6.0, 12.0, 12.0, Mat::White));
            for k in 0..12u32 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                out.push(soft(turned(
                    Vec3::new(a.cos() * 11.75, 0.0, a.sin() * 11.75),
                    Vec3::new(0.3, 6.0, 1.7),
                    -a,
                    Mat::Red,
                )));
            }
            out.push(round(Vec3::Y * 6.0, 8.0, 13.0, 0.6, Mat::Red));
            out.push(soft(round(Vec3::Y * 6.0, 0.5, 13.2, 13.0, Mat::Canvas(2))));
            out.push(soft(round(Vec3::Y * 14.0, 3.0, 0.15, 0.15, Mat::Metal)));
            out.push(soft(bx(
                Vec3::new(0.9, 16.0, 0.0),
                Vec3::new(0.9, 0.9, 0.04),
                Mat::Canvas(2),
            )));
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 11.9),
                Vec3::new(1.8, 4.0, 0.4),
                Mat::Dark,
            )));
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(18.0, 0.8, 6.0),
                    half: Vec3::new(3.0, 2.6, 1.5),
                    yaw: 0.4,
                    roof: 0.8,
                },
                mat: Mat::Canvas(1),
                solid: true,
            });
            for (x, z) in [(-2.0f32, -1.5f32), (2.0, -1.5), (-2.0, 1.5), (2.0, 1.5)] {
                let a = 0.4f32;
                let at = Vec3::new(
                    18.0 + x * a.cos() + z * a.sin(),
                    0.7,
                    6.0 - x * a.sin() + z * a.cos(),
                );
                out.push(soft(ball(at, 0.7, Mat::Canvas(2))));
            }
        }
        Kind::Fountain => {
            // A tiered stone fountain, its pools glowing softly at night.
            out.push(round(Vec3::ZERO, 0.8, 6.2, 6.0, Mat::Stone));
            out.push(soft(round(Vec3::Y * 0.78, 0.06, 5.6, 5.6, Mat::Water)));
            out.push(round(Vec3::Y * 0.8, 3.0, 0.7, 0.5, Mat::Stone));
            out.push(round(Vec3::Y * 3.8, 0.6, 0.6, 2.4, Mat::Stone));
            out.push(soft(round(Vec3::Y * 4.38, 0.06, 2.2, 2.2, Mat::Water)));
            out.push(round(Vec3::Y * 4.4, 1.5, 0.3, 0.25, Mat::Stone));
            out.push(round(Vec3::Y * 5.9, 0.4, 0.3, 1.2, Mat::Stone));
            out.push(soft(ball(Vec3::Y * 6.5, 0.4, Mat::Water)));
        }
        Kind::Postbox => {
            // A red pillar box. Someone still posts letters here.
            out.push(round(Vec3::ZERO, 1.5, 0.42, 0.4, Mat::Red));
            out.push(ball(Vec3::Y * 1.5, 0.43, Mat::Red));
            out.push(soft(bx(
                Vec3::new(0.0, 1.2, 0.38),
                Vec3::new(0.2, 0.05, 0.05),
                Mat::Dark,
            )));
            out.push(soft(round(Vec3::ZERO, 0.15, 0.48, 0.48, Mat::Dark)));
        }
        Kind::Bandstand => {
            // A little bandstand on the atoll's islet, music stands still out.
            out.push(round(Vec3::ZERO, 1.0, 6.2, 6.0, Mat::Wood));
            for k in 0..8u32 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                out.push(round(
                    Vec3::new(a.cos() * 5.6, 1.0, a.sin() * 5.6),
                    4.0,
                    0.16,
                    0.16,
                    Mat::White,
                ));
            }
            out.push(round(Vec3::Y * 5.0, 2.6, 6.8, 0.3, Mat::Canvas(0)));
            out.push(soft(round(Vec3::Y * 5.0, 0.4, 6.9, 6.8, Mat::White)));
            out.push(soft(round(Vec3::Y * 7.6, 1.6, 0.08, 0.08, Mat::Metal)));
            out.push(soft(bx(
                Vec3::new(0.6, 8.6, 0.0),
                Vec3::new(0.6, 0.5, 0.03),
                Mat::Canvas(2),
            )));
            for k in 0..3u32 {
                let a = k as f32 * 1.1 - 1.1;
                let at = Vec3::new(a.cos() * 3.0, 1.0, a.sin() * 3.0);
                out.push(soft(round(at, 1.2, 0.04, 0.04, Mat::Dark)));
                out.push(soft(turned(
                    at + Vec3::Y * 1.2,
                    Vec3::new(0.05, 0.4, 0.35),
                    -a,
                    Mat::Dark,
                )));
            }
        }
        Kind::Stadium => {
            // A football ground: the pitch, four stands climbing in tiers, a roof over the
            // long sides, floodlights on the corners and a scoreboard.
            const L: f32 = 55.0;
            const W: f32 = 36.0;
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(L + 6.0, 0.15, W + 6.0),
                Mat::Grass,
            )));
            for k in 0..9 {
                let x = -L + 6.0 + k as f32 * (L * 2.0 - 12.0) / 8.0;
                if k % 2 == 0 {
                    out.push(soft(bx(
                        Vec3::new(x, 0.0, 0.0),
                        Vec3::new((L * 2.0 - 12.0) / 16.0, 0.17, W),
                        Mat::Hedge,
                    )));
                }
            }
            for (at, half) in [
                (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.15, 0.2, W)),
                (Vec3::new(-L, 0.0, 0.0), Vec3::new(0.15, 0.2, W)),
                (Vec3::new(L, 0.0, 0.0), Vec3::new(0.15, 0.2, W)),
                (Vec3::new(0.0, 0.0, -W), Vec3::new(L, 0.2, 0.15)),
                (Vec3::new(0.0, 0.0, W), Vec3::new(L, 0.2, 0.15)),
            ] {
                out.push(soft(bx(at, half, Mat::White)));
            }
            out.push(soft(round(Vec3::ZERO, 0.2, 9.0, 9.0, Mat::White)));
            out.push(soft(round(Vec3::Y * 0.01, 0.2, 8.6, 8.6, Mat::Grass)));
            for s in [-1.0f32, 1.0] {
                // Goals: posts and a crossbar.
                for z in [-3.7f32, 3.7] {
                    out.push(soft(bx(
                        Vec3::new(s * L, 0.0, z),
                        Vec3::new(0.12, 2.4, 0.12),
                        Mat::White,
                    )));
                }
                out.push(soft(bx(
                    Vec3::new(s * L, 2.3, 0.0),
                    Vec3::new(0.12, 0.12, 3.8),
                    Mat::White,
                )));
            }
            // Stands: three tiers on every side, two colors.
            for tier in 0..3 {
                let t = tier as f32;
                let (gap, h) = (8.0 + t * 7.0, 3.0 + t * 4.5);
                let seats = Mat::Canvas(if tier % 2 == 0 { 0 } else { 1 });
                for s in [-1.0f32, 1.0] {
                    out.push(bx(
                        Vec3::new(0.0, 0.0, s * (W + gap)),
                        Vec3::new(L + gap, h, 3.6),
                        seats,
                    ));
                    out.push(bx(
                        Vec3::new(s * (L + gap), 0.0, 0.0),
                        Vec3::new(3.6, h, W + gap - 3.6),
                        seats,
                    ));
                }
            }
            for s in [-1.0f32, 1.0] {
                out.push(bx(
                    Vec3::new(0.0, 0.0, s * (W + 29.0)),
                    Vec3::new(L + 30.0, 18.0, 1.0),
                    Mat::Concrete,
                ));
                // A roof over each long stand.
                out.push(bx(
                    Vec3::new(0.0, 18.0, s * (W + 18.0)),
                    Vec3::new(L + 24.0, 0.8, 12.0),
                    Mat::White,
                ));
            }
            for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let at = Vec3::new(x * (L + 26.0), 0.0, z * (W + 26.0));
                out.push(round(at, 42.0, 1.0, 0.7, Mat::Metal));
                out.push(soft(turned(
                    at + Vec3::Y * 40.0,
                    Vec3::new(3.5, 3.0, 0.6),
                    (-z).atan2(-x) + std::f32::consts::FRAC_PI_2,
                    Mat::Lamp,
                )));
            }
            out.push(bx(
                Vec3::new(L + 30.0, 18.0, 0.0),
                Vec3::new(1.0, 8.0, 14.0),
                Mat::Dark,
            ));
            out.push(soft(bx(
                Vec3::new(L + 28.9, 19.0, 0.0),
                Vec3::new(0.1, 6.0, 12.5),
                Mat::Screen,
            )));
        }
        Kind::Waterpark => {
            // Pools, a lazy river round them, a tower of slides, a splash playground with a
            // tipping bucket, and loungers in rows.
            out.push(soft(bx(
                Vec3::ZERO,
                Vec3::new(70.0, 0.2, 50.0),
                Mat::Concrete,
            )));
            // Each pool sits in a raised white rim, so the water reads as pools on a deck.
            let pool = Vec3::new(-25.0, 0.0, -10.0);
            out.push(soft(bx(pool, Vec3::new(18.2, 0.3, 12.2), Mat::White)));
            out.push(soft(bx(pool, Vec3::new(17.0, 0.36, 11.0), Mat::Water)));
            // Lap lanes painted along the main pool's floor.
            for z in [-6.6f32, -2.2, 2.2, 6.6] {
                out.push(soft(bx(
                    pool + Vec3::new(0.0, 0.36, z),
                    Vec3::new(14.5, 0.02, 0.18),
                    Mat::Canvas(1),
                )));
            }
            for (at, r) in [
                (Vec3::new(30.0, 0.0, -15.0), 14.0),
                (Vec3::new(30.0, 0.0, 22.0), 8.0),
            ] {
                out.push(soft(round(at, 0.3, r + 1.0, r + 1.0, Mat::White)));
                out.push(soft(round(at, 0.36, r, r, Mat::Water)));
            }
            // The lazy river: a winding channel round the middle pool between low white
            // banks, with a strip of lawn along its inside.
            let (mid, radii) = LAZY_RIVER;
            for k in 0..28u32 {
                let a = k as f32 / 28.0 * std::f32::consts::TAU;
                let at = |grow: f32| {
                    Vec3::new(
                        mid.x + a.cos() * (radii.x + grow),
                        0.0,
                        mid.y + a.sin() * (radii.y + grow),
                    )
                };
                let yaw = -a + std::f32::consts::FRAC_PI_2;
                out.push(soft(turned(
                    at(0.0),
                    Vec3::new(3.9, 0.3, 3.4),
                    yaw,
                    Mat::White,
                )));
                out.push(soft(turned(
                    at(-4.6),
                    Vec3::new(3.0, 0.25, 1.3),
                    yaw,
                    Mat::Grass,
                )));
                out.push(soft(turned(
                    at(0.0),
                    Vec3::new(3.6, 0.36, 2.2),
                    yaw,
                    Mat::Water,
                )));
            }
            // The slide tower: an open steel frame with stair landings zigzagging up to a
            // roofed deck. The slides themselves are drawn by the client, on posts.
            let tower = Vec3::new(SLIDE_TOP.x, 0.0, SLIDE_TOP.z);
            for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                out.push(round(
                    tower + Vec3::new(x * 3.2, 0.0, z * 3.2),
                    SLIDE_TOP.y + 3.0,
                    0.22,
                    0.18,
                    Mat::Metal,
                ));
            }
            for k in 1..6u32 {
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                out.push(bx(
                    tower + Vec3::new(side * 1.6, k as f32 * 3.0 - 0.2, 0.0),
                    Vec3::new(1.6, 0.2, 3.2),
                    Mat::Canvas(1),
                ));
                // The flight up to the next landing, rising across the middle.
                out.push(soft(Piece {
                    form: Form::Box {
                        at: tower + Vec3::new(0.0, k as f32 * 3.0 - 1.5, side * 1.6),
                        half: Vec3::new(3.0, 0.2, 0.9),
                        yaw: side * 0.45,
                        roof: 0.0,
                    },
                    mat: Mat::White,
                    solid: false,
                }));
            }
            out.push(bx(
                tower + Vec3::Y * (SLIDE_TOP.y - 0.3),
                Vec3::new(4.4, 0.3, 4.4),
                Mat::Canvas(1),
            ));
            for (x, z, hx, hz) in [
                (0.0f32, 4.3f32, 2.0f32, 0.05f32),
                (0.0, -4.3, 2.0, 0.05),
                (4.3, 0.0, 0.05, 2.0),
                (-4.3, 0.0, 0.05, 2.0),
            ] {
                out.push(soft(bx(
                    tower + Vec3::new(x, SLIDE_TOP.y + 0.9, z),
                    Vec3::new(hx, 0.08, hz),
                    Mat::White,
                )));
            }
            out.push(Piece {
                form: Form::Box {
                    at: tower + Vec3::Y * (SLIDE_TOP.y + 3.0),
                    half: Vec3::new(4.8, 0.3, 4.8),
                    yaw: 0.0,
                    roof: 2.5,
                },
                mat: Mat::Canvas(2),
                solid: true,
            });
            // Posts under the slides.
            for sl in 0..SLIDES {
                let n = 8 + sl * 2;
                for k in 1..n {
                    let at = slide(sl, k as f32 / n as f32);
                    out.push(soft(round(
                        Vec3::new(at.x, 0.0, at.z),
                        at.y - 0.6,
                        0.15,
                        0.12,
                        Mat::Metal,
                    )));
                }
            }
            for k in 0..12u32 {
                let at = Vec3::new(-55.0 + k as f32 * 5.0, 0.2, 38.0);
                out.push(soft(bx(at, Vec3::new(0.8, 0.4, 1.8), Mat::White)));
                if k % 3 == 0 {
                    out.push(soft(round(at + Vec3::Z * 3.0, 3.0, 0.06, 0.06, Mat::Metal)));
                    out.push(soft(round(
                        at + Vec3::new(0.0, 3.0, 3.0),
                        1.0,
                        2.2,
                        0.1,
                        Mat::Canvas((k / 3) as u8),
                    )));
                }
            }
            splash_playground(&mut out);
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(-55.0, 0.2, -38.0),
                    half: Vec3::new(10.0, 4.0, 6.0),
                    yaw: 0.0,
                    roof: 2.0,
                },
                mat: Mat::Canvas(1),
                solid: true,
            });
        }
        Kind::ThemePark => {
            // A roller coaster round a looping track on stilts, a castle, a carousel, a drop
            // tower and stalls. The coaster train and the carousel move, drawn by the client.
            themepark_grounds(&mut out);
            // Posts hold up the track, which the client draws, rails and all.
            let n = 48;
            for k in 0..n {
                let a = coaster(k as f32 / n as f32);
                out.push(soft(round(
                    Vec3::new(a.x, 0.0, a.z),
                    a.y - 0.55,
                    0.3,
                    0.2,
                    Mat::White,
                )));
            }
            // The castle in the middle.
            out.push(bx(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(9.0, 12.0, 7.0),
                Mat::Snow,
            ));
            for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let at = Vec3::new(x * 9.0, 0.0, z * 7.0);
                out.push(round(at, 18.0, 2.4, 2.2, Mat::Snow));
                out.push(round(at + Vec3::Y * 18.0, 7.0, 2.8, 0.0, Mat::Canvas(1)));
            }
            out.push(round(Vec3::Y * 12.0, 10.0, 3.0, 2.6, Mat::Snow));
            out.push(round(Vec3::Y * 22.0, 10.0, 3.4, 0.0, Mat::Canvas(4)));
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 7.05),
                Vec3::new(2.5, 5.0, 0.1),
                Mat::Dark,
            )));
            // The carousel: a striped canopy over a turning platform.
            let c = CAROUSEL;
            out.push(round(c, 0.8, 9.0, 9.0, Mat::Wood));
            out.push(soft(round(c + Vec3::Y * 0.8, 6.0, 0.8, 0.8, Mat::Lamp)));
            out.push(round(c + Vec3::Y * 6.8, 3.5, 10.0, 0.4, Mat::Canvas(0)));
            out.push(soft(round(
                c + Vec3::Y * 6.3,
                0.5,
                10.1,
                10.0,
                Mat::Canvas(2),
            )));
            // The drop tower. Its ring of seats rides up and down, drawn by the client.
            let drop = DROP;
            out.push(round(drop, DROP_TALL, 2.2, 1.6, Mat::Metal));
            out.push(soft(round(
                drop + Vec3::Y * DROP_TALL,
                2.0,
                1.6,
                0.4,
                Mat::Red,
            )));
            out.push(round(drop, 0.6, 6.0, 6.0, Mat::Concrete));
            // Food stalls along the front.
            for k in 0..6u32 {
                let at = Vec3::new(-50.0 + k as f32 * 20.0, 0.0, 52.0);
                out.push(bx(at, Vec3::new(3.0, 2.6, 2.0), Mat::White));
                out.push(Piece {
                    form: Form::Box {
                        at: at + Vec3::Y * 2.6,
                        half: Vec3::new(3.4, 0.1, 2.4),
                        yaw: 0.0,
                        roof: 0.9,
                    },
                    mat: Mat::Canvas((k % 6) as u8),
                    solid: true,
                });
            }
            // The gate.
            for x in [-8.0f32, 8.0] {
                out.push(round(
                    Vec3::new(x, 0.0, 60.0),
                    9.0,
                    1.4,
                    1.2,
                    Mat::Canvas(4),
                ));
                out.push(round(
                    Vec3::new(x, 9.0, 60.0),
                    3.0,
                    1.6,
                    0.0,
                    Mat::Canvas(1),
                ));
            }
            out.push(bx(
                Vec3::new(0.0, 7.0, 60.0),
                Vec3::new(8.0, 1.6, 0.8),
                Mat::Canvas(2),
            ));
        }
        Kind::Zoo => {
            // Pens fenced with posts and rails, each with its own ground: dry grass and
            // sand for the savanna, ice and a pool for the penguins. Ponds, rocks and shade
            // trees inside, paved paths between with a viewing deck at every pen, a keeper's
            // house and the gate.
            out.push(soft(bx(Vec3::ZERO, Vec3::new(80.0, 0.1, 62.0), Mat::Grass)));
            for (c, r, kind) in ZOO_PENS {
                let at = Vec3::new(c.x, 0.0, c.y);
                let ground = match kind {
                    3 => Mat::Sand,
                    4 => Mat::Snow,
                    1 | 2 => Mat::Hay,
                    _ => Mat::Grass,
                };
                out.push(soft(round(at, 0.16, r, r, ground)));
                // Posts every few meters, two rails between them, gaps you can see through.
                let posts = (r * std::f32::consts::TAU / 4.0) as u32;
                for k in 0..posts {
                    let a = k as f32 / posts as f32 * std::f32::consts::TAU;
                    let b = (k + 1) as f32 / posts as f32 * std::f32::consts::TAU;
                    let (p, q) = (
                        at + Vec3::new(a.cos(), 0.0, a.sin()) * r,
                        at + Vec3::new(b.cos(), 0.0, b.sin()) * r,
                    );
                    out.push(soft(round(p, 1.5, 0.12, 0.1, Mat::Wood)));
                    let mid = (p + q) * 0.5;
                    let d = q - p;
                    for y in [0.55f32, 1.25] {
                        out.push(soft(turned(
                            mid + Vec3::Y * y,
                            Vec3::new(d.length() * 0.5, 0.1, 0.05),
                            (-d.z).atan2(d.x),
                            Mat::Wood,
                        )));
                    }
                }
                // A viewing deck on the side facing the middle of the zoo.
                let out_dir = (-at).normalize_or(Vec3::X);
                let deck = at + out_dir * (r + 2.5);
                out.push(turned(
                    deck,
                    Vec3::new(2.5, 0.3, 4.0),
                    (-out_dir.z).atan2(out_dir.x),
                    Mat::Wood,
                ));
                let water = if kind == 4 { r * 0.45 } else { r * 0.22 };
                out.push(soft(round(
                    at + Vec3::new(r * 0.35, 0.02, -r * 0.3),
                    0.2,
                    water,
                    water,
                    Mat::Water,
                )));
                for k in 0..3u32 {
                    let a = k as f32 * 2.1 + kind as f32;
                    out.push(ball(
                        at + Vec3::new(a.cos() * r * 0.55, 0.4, a.sin() * r * 0.55),
                        r * (0.06 + 0.03 * k as f32),
                        if kind == 4 { Mat::Snow } else { Mat::Stone },
                    ));
                }
                if kind != 4 {
                    let t = at + Vec3::new(-r * 0.45, 0.0, -r * 0.35);
                    out.push(round(t, 5.0, 0.6, 0.45, Mat::Wood));
                    out.push(ball(t + Vec3::Y * 6.5, 3.4, Mat::Leaf));
                }
                if kind == 0 {
                    // The elephants' barn.
                    out.push(Piece {
                        form: Form::Box {
                            at: at + Vec3::new(r * 0.3, 0.0, r * 0.45),
                            half: Vec3::new(5.0, 4.5, 4.0),
                            yaw: 0.4,
                            roof: 2.5,
                        },
                        mat: Mat::Wood,
                        solid: true,
                    });
                }
            }
            // Paths: a loop round the middle, outside every pen, and the way in from the
            // gate.
            let path = zoo_path();
            for (k, a) in path.iter().enumerate() {
                let b = path[(k + 1) % path.len()];
                let d = b - *a;
                out.push(soft(turned(
                    Vec3::new((a.x + b.x) * 0.5, 0.0, (a.y + b.y) * 0.5),
                    Vec3::new(d.length() * 0.5 + 1.0, 0.14, 2.0),
                    (-d.y).atan2(d.x),
                    Mat::Concrete,
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 46.0),
                Vec3::new(2.5, 0.14, 16.5),
                Mat::Concrete,
            )));
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(55.0, 0.0, -42.0),
                    half: Vec3::new(9.0, 4.0, 6.0),
                    yaw: 0.0,
                    roof: 3.0,
                },
                mat: Mat::Wood,
                solid: true,
            });
            zoo_entrance(&mut out, 66.0);
        }
        Kind::Faces if text == "spiral" => spiral(&mut out),
        Kind::Faces if text == "serpent" => serpent(&mut out),
        Kind::Statue if text == "effigy" => effigy(&mut out),
        Kind::Faces => faces(&mut out),
        Kind::Warden => warden(&mut out, text == "lamp"),
        Kind::Statue => {
            // The harbor dragon: a great copper sea dragon coiled up a rock pillar on its
            // island pedestal, neck stretched out to sea and a lamp held in its jaws to
            // guide ships in. The lamp shines at night.
            out.push(round(Vec3::Y * -6.0, 10.0, 22.0, 20.0, Mat::Stone));
            out.push(round(Vec3::Y * 4.0, 28.0, 7.0, 4.5, Mat::Stone));
            let coil = 40;
            let mut last = Vec3::ZERO;
            for k in 0..coil {
                let t = k as f32 / (coil - 1) as f32;
                let a = t * 2.3 * std::f32::consts::TAU;
                let rr = 8.5 - t * 3.0;
                let at = Vec3::new(a.cos() * rr, 6.0 + t * 26.0, a.sin() * rr);
                let size = 3.4 - t * 1.3;
                out.push(ball(at, size, Mat::Copper));
                if k % 4 == 0 {
                    out.push(soft(round(
                        at + Vec3::Y * size * 0.7,
                        size * 0.9,
                        size * 0.35,
                        0.0,
                        Mat::Copper,
                    )));
                }
                last = at;
            }
            // The neck reaching out over the water, then the head with its lamp.
            let head = Vec3::new(9.0, 44.0, 0.0);
            for k in 1..12 {
                let t = k as f32 / 12.0;
                let at = last.lerp(head, t) + Vec3::Y * (t * (1.0 - t) * 8.0);
                out.push(ball(at, 2.1 - t * 0.5, Mat::Copper));
            }
            out.push(bx(
                head - Vec3::Y * 1.6,
                Vec3::new(2.6, 3.2, 1.8),
                Mat::Copper,
            ));
            out.push(bx(
                head + Vec3::new(3.0, -1.2, 0.0),
                Vec3::new(1.6, 1.0, 1.2),
                Mat::Copper,
            ));
            out.push(bx(
                head + Vec3::new(3.0, -2.6, 0.0),
                Vec3::new(1.5, 0.5, 1.1),
                Mat::Copper,
            ));
            for z in [-1.0f32, 1.0] {
                out.push(soft(turned(
                    head + Vec3::new(-1.6, 1.4, z * 1.0),
                    Vec3::new(0.3, 3.0, 0.3),
                    0.6,
                    Mat::Copper,
                )));
                // Fins folded along the sides of the coil, like a sea creature's.
                out.push(soft(turned(
                    Vec3::new(z * 6.5, 18.0, z * 2.0),
                    Vec3::new(4.0, 6.0, 0.3),
                    z * 0.8,
                    Mat::Copper,
                )));
            }
            out.push(soft(ball(head + Vec3::new(4.6, -1.8, 0.0), 1.3, Mat::Lamp)));
            // The tail's fluke curling out of the water at the foot.
            out.push(soft(turned(
                Vec3::new(-12.0, 3.0, -6.0),
                Vec3::new(3.0, 4.0, 0.4),
                0.9,
                Mat::Copper,
            )));
        }
        Kind::Mall => {
            // A big shopping mall: a long low building with a glass entrance and a sign
            // band, a car park in front with cars and lamps.
            out.push(bx(Vec3::ZERO, Vec3::new(60.0, 12.0, 30.0), Mat::Concrete));
            // The sign band stops short of the roof, or the two would flicker.
            out.push(soft(bx(
                Vec3::new(0.0, 9.6, 0.0),
                Vec3::new(60.5, 2.1, 30.5),
                Mat::Canvas(1),
            )));
            out.push(bx(
                Vec3::new(0.0, 0.0, 30.0),
                Vec3::new(10.0, 15.0, 4.0),
                Mat::Glass,
            ));
            out.push(soft(bx(
                Vec3::new(0.0, 15.0, 30.0),
                Vec3::new(11.0, 1.0, 4.5),
                Mat::White,
            )));
            for x in [-40.0f32, 40.0] {
                out.push(soft(bx(
                    Vec3::new(x, 0.0, 30.1),
                    Vec3::new(6.0, 4.0, 0.1),
                    Mat::Glass,
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 58.0),
                Vec3::new(60.0, 0.18, 24.0),
                Mat::Asphalt,
            )));
            for row in [44.0f32, 62.0, 74.0] {
                for k in 0..14u32 {
                    let x = -52.0 + k as f32 * 8.0;
                    out.push(soft(bx(
                        Vec3::new(x - 2.0, 0.0, row),
                        Vec3::new(0.08, 0.2, 3.0),
                        Mat::White,
                    )));
                    if r(k * 7 + row as u32) < 0.6 {
                        let col = Mat::Canvas((r(k * 11 + row as u32) * 6.0) as u8);
                        out.push(bx(Vec3::new(x, 0.0, row), Vec3::new(1.0, 1.5, 2.1), col));
                    }
                }
            }
            for k in 0..4 {
                let at = Vec3::new(-45.0 + k as f32 * 30.0, 0.0, 53.0);
                out.push(soft(round(at, 7.0, 0.12, 0.1, Mat::Metal)));
                out.push(soft(bx(
                    at + Vec3::Y * 7.0,
                    Vec3::new(0.8, 0.2, 0.4),
                    Mat::Lamp,
                )));
            }
        }
        Kind::Grocery => {
            // A corner grocery store with a striped awning and a little car park.
            out.push(bx(Vec3::ZERO, Vec3::new(14.0, 6.0, 10.0), Mat::White));
            out.push(soft(bx(
                Vec3::new(0.0, 5.0, 0.0),
                Vec3::new(14.3, 1.4, 10.3),
                Mat::Canvas(3),
            )));
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 10.05),
                Vec3::new(9.0, 3.2, 0.1),
                Mat::Glass,
            )));
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.0, 3.4, 11.3),
                    half: Vec3::new(10.0, 0.08, 1.3),
                    yaw: 0.0,
                    roof: 0.5,
                },
                mat: Mat::Canvas(0),
                solid: false,
            });
            for k in 0..4u32 {
                out.push(soft(bx(
                    Vec3::new(-8.0 + k as f32 * 2.2, 0.0, 12.0),
                    Vec3::new(0.8, 1.0, 0.5),
                    Mat::Canvas([2, 5, 3, 0][k as usize]),
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 22.0),
                Vec3::new(16.0, 0.16, 9.0),
                Mat::Asphalt,
            )));
            for k in 0..5u32 {
                if r(k) < 0.7 {
                    out.push(bx(
                        Vec3::new(-12.0 + k as f32 * 6.0, 0.0, 24.0),
                        Vec3::new(1.0, 1.5, 2.1),
                        Mat::Canvas((r(k + 5) * 6.0) as u8),
                    ));
                }
            }
        }
        Kind::Park => {
            // A town park: a lawn with soft ragged edges, a path winding through it and a
            // loop round the duck pond, shade trees dotted about, flower beds, benches and a
            // playground with a slide, swings, a seesaw, a climbing frame and a sandpit.
            for k in 0..9u32 {
                let a = k as f32 / 9.0 * std::f32::consts::TAU + r(k + 40) * 0.5;
                let at = Vec3::new(a.cos() * 18.0, 0.0, a.sin() * 11.0);
                out.push(soft(round(
                    at,
                    0.12,
                    13.0 + r(k + 50) * 4.0,
                    12.5,
                    Mat::Grass,
                )));
            }
            out.push(soft(round(Vec3::ZERO, 0.12, 16.0, 15.5, Mat::Grass)));
            // The winding path, and the loop round the pond, in short turned pieces.
            let path = |x: f32| 1.0 + 2.5 * (std::f32::consts::PI * x / 14.0).sin();
            let piece = |out: &mut Vec<Piece>, a: Vec2, b: Vec2| {
                let d = b - a;
                let mid = (a + b) * 0.5;
                out.push(soft(turned(
                    Vec3::new(mid.x, 0.0, mid.y),
                    Vec3::new(d.length() * 0.5 + 0.3, 0.16, 1.2),
                    (-d.y).atan2(d.x),
                    Mat::Sand,
                )));
            };
            let n = 16;
            for k in 0..n {
                let (x0, x1) = (-32.0 + k as f32 * 4.0, -28.0 + k as f32 * 4.0);
                piece(&mut out, Vec2::new(x0, path(x0)), Vec2::new(x1, path(x1)));
            }
            let pond = Vec2::new(-16.0, 12.0);
            for k in 0..12 {
                let (a0, a1) = (
                    k as f32 / 12.0 * std::f32::consts::TAU,
                    (k + 1) as f32 / 12.0 * std::f32::consts::TAU,
                );
                let ring = |a: f32| pond + Vec2::new(a.cos() * 10.0, a.sin() * 7.5);
                piece(&mut out, ring(a0), ring(a1));
            }
            // A little spur from the winding path down to the pond loop.
            piece(
                &mut out,
                Vec2::new(-16.0, path(-16.0)),
                pond + Vec2::new(0.0, -7.5),
            );
            out.push(soft(round(
                Vec3::new(pond.x, 0.0, pond.y),
                0.25,
                6.0,
                6.0,
                Mat::Water,
            )));
            out.push(soft(round(
                Vec3::new(pond.x + 3.5, 0.0, pond.y + 1.5),
                0.25,
                4.0,
                4.0,
                Mat::Water,
            )));
            // Reeds round the shore, never out in the water of either lobe.
            for k in 0..8u32 {
                let a = k as f32 * 0.8 + 0.4;
                let mut at = pond + Vec2::new(a.cos(), a.sin()) * 6.4;
                let lobe = pond + Vec2::new(3.5, 1.5);
                if at.distance(lobe) < 4.4 {
                    at = lobe + (at - lobe).normalize_or(Vec2::X) * 4.4;
                }
                out.push(soft(round(
                    Vec3::new(at.x, 0.0, at.y),
                    1.1,
                    0.4,
                    0.08,
                    Mat::Hedge,
                )));
            }
            // Flower beds, round and bright.
            for (k, (x, z)) in [(-4.0f32, 9.0f32), (6.0, 11.5), (-24.0, -8.0), (24.0, 6.0)]
                .into_iter()
                .enumerate()
            {
                out.push(soft(round(
                    Vec3::new(x, 0.0, z),
                    0.35,
                    2.4,
                    2.0,
                    Mat::Canvas(k as u8 + 1),
                )));
            }
            for k in 0..11u32 {
                let a = k as f32 * 2.3 + r(k);
                let at = Vec3::new(
                    a.cos() * (10.0 + r(k + 9) * 18.0),
                    0.0,
                    a.sin() * (8.0 + r(k + 19) * 9.0),
                );
                // Clear of the paths, the pond and the playground.
                let clear = (at.z - path(at.x)).abs() > 3.5
                    && Vec2::new(at.x, at.z).distance(pond) > 12.0
                    && !(at.x > 4.0 && at.z < -2.0);
                if !clear {
                    continue;
                }
                let tall = 3.5 + r(k + 30) * 2.5;
                out.push(round(at, tall, 0.5, 0.4, Mat::Wood));
                out.push(ball(
                    at + Vec3::Y * (tall + 1.5),
                    2.6 + r(k + 3) * 1.4,
                    Mat::Leaf,
                ));
            }
            for at in PARK_BENCHES {
                out.push(soft(bx(at, Vec3::new(1.4, 0.5, 0.35), Mat::Wood)));
            }
            // The playground.
            let pg = Vec3::new(15.0, 0.0, -11.0);
            out.push(soft(bx(pg, Vec3::new(10.0, 0.18, 8.0), Mat::Sand)));
            out.push(bx(
                pg + Vec3::new(-5.0, 0.0, -3.0),
                Vec3::new(1.2, 2.8, 1.2),
                Mat::Canvas(0),
            ));
            for k in 0..5 {
                out.push(soft(bx(
                    pg + Vec3::new(-3.2 + k as f32 * 0.9, 2.6 - k as f32 * 0.6, -3.0),
                    Vec3::new(0.5, 0.15, 0.6),
                    Mat::Canvas(2),
                )));
            }
            for x in [-1.0f32, 1.0] {
                out.push(soft(bx(
                    pg + Vec3::new(4.0 + x * 2.5, 0.0, 3.0),
                    Vec3::new(0.1, 3.0, 0.1),
                    Mat::Canvas(1),
                )));
            }
            out.push(soft(bx(
                pg + Vec3::new(4.0, 3.0, 3.0),
                Vec3::new(2.7, 0.12, 0.12),
                Mat::Canvas(1),
            )));
            for x in [-1.0f32, 1.0] {
                out.push(soft(bx(
                    pg + Vec3::new(4.0 + x * 1.2, 0.6, 3.0),
                    Vec3::new(0.4, 0.06, 0.25),
                    Mat::Dark,
                )));
            }
            out.push(soft(turned(
                pg + Vec3::new(-4.0, 0.5, 4.0),
                Vec3::new(2.5, 0.12, 0.2),
                0.0,
                Mat::Canvas(4),
            )));
            out.push(soft(bx(
                pg + Vec3::new(-4.0, 0.0, 4.0),
                Vec3::new(0.2, 0.5, 0.2),
                Mat::Metal,
            )));
            for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                out.push(soft(bx(
                    pg + Vec3::new(4.0 + x * 1.5, 0.0, -3.0 + z * 1.5),
                    Vec3::new(0.08, 2.2, 0.08),
                    Mat::Canvas(3),
                )));
            }
            out.push(bx(
                pg + Vec3::new(4.0, 2.2, -3.0),
                Vec3::new(1.6, 0.1, 1.6),
                Mat::Canvas(3),
            ));
        }
        Kind::Metro => metro(&mut out, text),
        Kind::MetroGate => metro_gate(&mut out),
        Kind::MetroStairs => metro_stairs(&mut out, text),
        Kind::Restaurant => restaurant(&mut out),
        Kind::Drawbridge => drawbridge(&mut out, text),
        Kind::Gallery => gallery(&mut out, text),
        Kind::SkatePark => skate_park(&mut out),
        Kind::Helipad => helipad(&mut out),
        Kind::ClockTower => clock_tower(&mut out),
        Kind::SpanTower => span_tower(&mut out),
        Kind::Market => market(&mut out, seed),
        Kind::ShroomFarm => shroom_farm(&mut out, seed),
        Kind::Mine => mine(&mut out),
        Kind::Cafe => cafe(&mut out),
        Kind::Library => library(&mut out),
        Kind::Museum => museum(&mut out),
        Kind::Cinema => cinema(&mut out),
        Kind::Weir => weir(&mut out),
        Kind::Penstock => penstock(&mut out, text),
        Kind::Hydro => hydro(&mut out),
        Kind::Racetrack => racetrack(&mut out),
        Kind::Aquarium => aquarium(&mut out),
        Kind::University => university(&mut out),
        Kind::Police => police(&mut out),
        Kind::GasStation => gas_station(&mut out),
        Kind::Amphitheater => amphitheater(&mut out),
        Kind::SolarFarm => solar_farm(&mut out),
        Kind::Golf => golf(&mut out, seed),
        Kind::Observatory => observatory(&mut out),
        Kind::Lookout => lookout(&mut out),
        Kind::SurfShack => surf_shack(&mut out),
        Kind::Castaway => castaway(&mut out),
        Kind::BalloonField => balloon_field(&mut out),
        Kind::Seals => seal_rocks(&mut out),
        Kind::Carnival => carnival(&mut out),
        Kind::Cemetery => cemetery(&mut out),
        Kind::Factory => factory(&mut out),
        Kind::PowerStation => power_station(&mut out),
        Kind::OilRig => oil_rig(&mut out),
        Kind::Billboard => billboard(&mut out, seed, text),
        Kind::Dam => dam(&mut out),
        Kind::RopeBridge => rope_bridge(&mut out, text),
        Kind::Hospital => hospital(&mut out),
        Kind::FireStation => fire_station(&mut out),
        Kind::School => school(&mut out),
        Kind::TownHall => town_hall(&mut out),
        Kind::Farm => farm(&mut out),
        Kind::Port => port(&mut out, seed),
        Kind::SeaDoors => sea_doors(&mut out),
        Kind::BasaltGate => basalt_gate(&mut out, r),
        Kind::Station => {
            // A railway station: a long platform with a canopy, a waiting hall, a clock
            // and benches.
            out.push(bx(
                Vec3::new(0.0, 0.0, 6.5),
                Vec3::new(40.0, 1.0, 3.0),
                Mat::Concrete,
            ));
            out.push(soft(bx(
                Vec3::new(0.0, 1.0, 3.65),
                Vec3::new(40.0, 0.02, 0.15),
                Mat::Canvas(2),
            )));
            for k in 0..9 {
                let x = -32.0 + k as f32 * 8.0;
                out.push(soft(round(
                    Vec3::new(x, 1.0, 7.5),
                    4.0,
                    0.15,
                    0.15,
                    Mat::Metal,
                )));
            }
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.0, 5.0, 6.5),
                    half: Vec3::new(36.0, 0.1, 3.4),
                    yaw: 0.0,
                    roof: 0.8,
                },
                mat: Mat::Canvas(1),
                solid: true,
            });
            out.push(Piece {
                form: Form::Box {
                    at: Vec3::new(0.0, 0.0, 15.0),
                    half: Vec3::new(12.0, 6.0, 5.5),
                    yaw: 0.0,
                    roof: 3.0,
                },
                mat: Mat::Stone,
                solid: true,
            });
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 9.45),
                Vec3::new(2.0, 3.0, 0.1),
                Mat::Glass,
            )));
            out.push(soft(round(
                Vec3::new(0.0, 7.0, 9.4),
                0.2,
                1.2,
                1.2,
                Mat::White,
            )));
            for x in [-16.0f32, 16.0] {
                out.push(soft(bx(
                    Vec3::new(x, 1.0, 8.0),
                    Vec3::new(1.4, 0.5, 0.35),
                    Mat::Wood,
                )));
            }
        }
        _ => {}
    }
    // Homely things are built at a giant's size, so they read from the air.
    let grow = match kind {
        Kind::Lemonade | Kind::Well | Kind::Laundry => 1.5,
        Kind::Treehouse | Kind::Faces => 1.3,
        Kind::Warden => 1.4,
        _ => 1.0,
    };
    if grow != 1.0 {
        for p in &mut out {
            p.form = match p.form {
                Form::Box {
                    at,
                    half,
                    yaw,
                    roof,
                } => Form::Box {
                    at: at * grow,
                    half: half * grow,
                    yaw,
                    roof: roof * grow,
                },
                Form::Round { at, h, r0, r1 } => Form::Round {
                    at: at * grow,
                    h: h * grow,
                    r0: r0 * grow,
                    r1: r1 * grow,
                },
                Form::Ball { at, r } => Form::Ball {
                    at: at * grow,
                    r: r * grow,
                },
                Form::Letter {
                    at,
                    cell,
                    depth,
                    ch,
                } => Form::Letter {
                    at: at * grow,
                    cell: cell * grow,
                    depth: depth * grow,
                    ch,
                },
            };
        }
    }
    // Broad flat paving on the ground reaches down into it, so on a slope it stands
    // on a plinth instead of floating over the downhill side.
    // A plinth only for looks: roads and walls still meet the paving itself.
    // Each sits a hair lower than the last, so two never share a face.
    let plinths: Vec<Piece> = out
        .iter()
        .filter_map(|p| match p.form {
            Form::Box {
                at,
                half,
                yaw,
                roof: _,
            } if at.y.abs() < 0.5
                && half.y < 0.6
                && half.x.min(half.z) > 2.5
                && !matches!(
                    p.mat,
                    Mat::Water | Mat::Hidden | Mat::Asphalt | Mat::Concrete
                ) =>
            {
                Some(soft(Piece {
                    form: Form::Box {
                        at: at - Vec3::Y * PLINTH,
                        half: Vec3::new(half.x - 0.01, PLINTH - 0.03, half.z - 0.01),
                        yaw,
                        roof: 0.0,
                    },
                    mat: p.mat,
                    solid: false,
                }))
            }
            _ => None,
        })
        .enumerate()
        .map(|(k, mut p)| {
            if let Form::Box { at, .. } = &mut p.form {
                at.y -= k as f32 * 0.013;
            }
            p
        })
        .collect();
    out.extend(plinths);
    out
}

/// A box spanning `x`, `y` and `z` in the landmark's frame.
fn slab(out: &mut Vec<Piece>, x: [f32; 2], y: [f32; 2], z: [f32; 2], mat: Mat) {
    out.push(bx(
        Vec3::new((x[0] + x[1]) * 0.5, y[0], (z[0] + z[1]) * 0.5),
        Vec3::new((x[1] - x[0]) * 0.5, y[1] - y[0], (z[1] - z[0]) * 0.5),
        mat,
    ));
}

/// The same box and its mirror across x = 0.
fn slabs(out: &mut Vec<Piece>, x: [f32; 2], y: [f32; 2], z: [f32; 2], mat: Mat) {
    slab(out, x, y, z, mat);
    slab(out, [-x[1], -x[0]], y, z, mat);
}

/// The Sea Doors' inside: half the hall's width and its height.
const SEA_W: f32 = 26.0;
const SEA_H: f32 = 78.0;
/// Where its two fronts face out: to sea (+z) and into the cavern (-z).
const SEA_FRONT: f32 = 44.0;
const SEA_BACK: f32 = -374.0;

/// The Sea Doors: a hall of pale stone built into a sea cliff, through the rock to a
/// cavern. Its doors open on a quay at the water (+z), tall and stepped at the top, under
/// a portico of round columns, a band of waves and a copper sun. Inside, rows of columns
/// and lamps run the length of the hall to doors just like them on the far side, high in
/// the cavern's wall, where a broad stair comes down to an underground lake. The rock is
/// cut a little wider than the hall, and both fronts reach past the cuts into the rock,
/// so no cut edge shows. The frame's origin is on the hall floor at the sea doors.
fn sea_doors(out: &mut Vec<Piece>) {
    let (w, h) = (SEA_W, SEA_H);
    for (face, inner, out_dir) in [(SEA_FRONT, -40.0f32, 1.0f32), (SEA_BACK, -318.0, -1.0)] {
        let z = if out_dir > 0.0 {
            [inner, face]
        } else {
            [face, inner]
        };
        let (bottom, top) = if out_dir > 0.0 {
            (-14.0, 170.0)
        } else {
            (-54.0, 154.0)
        };
        // The front itself, round the door.
        slabs(out, [w, 104.0], [bottom, top], z, Mat::Pale);
        slab(out, [-w, w], [h, top], z, Mat::Stone);
        if out_dir < 0.0 {
            slab(out, [-w, w], [bottom, -5.0], z, Mat::Pale);
        }
        // The door's top steps in three times.
        let lip = [face - out_dir * 10.0, face];
        let lip = [lip[0].min(lip[1]), lip[0].max(lip[1])];
        for k in 1..=3 {
            let k = k as f32;
            slabs(
                out,
                [w - 5.0 * k, w],
                [h - 6.0 * (4.0 - k), h],
                lip,
                Mat::Pale,
            );
        }
        // Everything standing proud of it, `d` out from the face.
        let at = |d: f32| face + out_dir * d;
        let span = |a: f32, b: f32| [at(a).min(at(b)), at(a).max(at(b))];
        slabs(out, [w, w + 5.0], [0.0, h], span(0.0, 3.0), Mat::White);
        slab(
            out,
            [-w - 5.0, w + 5.0],
            [h, h + 5.0],
            span(0.0, 3.0),
            Mat::White,
        );
        // The portico: a tall pair of columns by the doors, a shorter pair outside.
        let foot = if out_dir > 0.0 { 0.0 } else { -50.0 };
        for (x, tall, r) in [(42.0f32, 118.0f32, 5.5f32), (78.0, 104.0, 4.6)] {
            for s in [-1.0f32, 1.0] {
                let cz = at(8.0);
                slab(
                    out,
                    [s * x - 7.0, s * x + 7.0],
                    [foot, foot + 5.0],
                    [cz - 7.0, cz + 7.0],
                    Mat::Pale,
                );
                out.push(round(
                    Vec3::new(s * x, foot + 5.0, cz),
                    tall - foot - 10.0,
                    r,
                    r * 0.88,
                    Mat::White,
                ));
                slab(
                    out,
                    [s * x - 7.5, s * x + 7.5],
                    [tall - 5.0, tall],
                    [cz - 7.5, cz + 7.5],
                    Mat::Pale,
                );
            }
        }
        slab(
            out,
            [-52.0, 52.0],
            [118.0, 126.0],
            span(-2.0, 16.0),
            Mat::Stone,
        );
        slabs(
            out,
            [70.0, 86.0],
            [104.0, 110.0],
            span(-2.0, 16.0),
            Mat::Stone,
        );
        // A band of waves along the top: green copper crests on white.
        for i in -8..=8 {
            let x = i as f32 * 6.0;
            let mat = if i % 2 == 0 { Mat::Copper } else { Mat::White };
            let lift = if i % 2 == 0 { 4.0 } else { 2.5 };
            slab(
                out,
                [x - 3.0, x + 3.0],
                [126.0, 126.0 + lift],
                span(10.0, 16.0),
                mat,
            );
        }
        // A copper sun over the doors, and tall lit slits either side of them.
        out.push(ball(Vec3::new(0.0, 99.0, face), 10.0, Mat::Copper));
        slabs(out, [59.0, 63.0], [20.0, 60.0], span(0.0, 0.6), Mat::Lamp);
        // Three windows up top, lit from within.
        for x in [-40.0f32, 0.0, 40.0] {
            slab(
                out,
                [x - 6.0, x + 6.0],
                [131.0, 152.0],
                span(0.0, 0.4),
                Mat::Dark,
            );
            slab(
                out,
                [x - 2.0, x + 2.0],
                [136.0, 146.0],
                span(0.4, 0.8),
                Mat::Lamp,
            );
        }
        // A stepped crown.
        for (k, half) in [80.0f32, 56.0, 32.0].into_iter().enumerate() {
            let y = top + k as f32 * 6.0;
            slab(
                out,
                [-half, half],
                [y, y + 6.0],
                span(-60.0, 2.0 - k as f32 * 4.0),
                Mat::Pale,
            );
        }
        out.push(ball(Vec3::new(0.0, top + 24.0, at(-8.0)), 6.0, Mat::Copper));
    }

    // Wings either side of the cavern front, stepping back into the rock, so the front
    // meets the curving cave wall without a narrowing gap a dragon could wedge into.
    for k in 0..4 {
        let k = k as f32;
        slabs(
            out,
            [104.0 + 12.0 * k, 116.0 + 12.0 * k],
            [-54.0, 150.0 - 20.0 * k],
            [SEA_BACK + 12.0 * (k + 1.0), -318.0],
            Mat::Pale,
        );
    }

    // The quay at the sea doors, with steps down to the water and two braziers.
    let f = SEA_FRONT;
    slab(
        out,
        [-110.0, 110.0],
        [-16.0, 0.0],
        [f, f + 40.0],
        Mat::Stone,
    );
    for k in 1..=3 {
        let z = f + 36.0 + k as f32 * 4.0;
        slab(
            out,
            [-60.0, 60.0],
            [-16.0, -1.3 * k as f32],
            [z, z + 4.0],
            Mat::Stone,
        );
    }
    for x in [-50.0f32, 50.0] {
        let z = f + 30.0;
        out.push(round(Vec3::new(x, 0.0, z), 7.0, 0.7, 0.7, Mat::Metal));
        out.push(round(Vec3::new(x, 7.0, z), 1.8, 1.2, 2.4, Mat::Copper));
        out.push(soft(ball(Vec3::new(x, 9.2, z), 1.4, Mat::Lamp)));
    }
    for i in 0..10 {
        let x = -90.0 + i as f32 * 20.0;
        out.push(round(Vec3::new(x, 0.0, f + 38.0), 1.6, 0.8, 0.7, Mat::Dark));
    }

    // The hall: floor, walls and ceiling lining the rock, the length of it.
    let (inner_f, inner_b) = (-40.0f32, -318.0f32);
    slab(out, [-33.0, 33.0], [-5.0, 0.0], [SEA_BACK, f], Mat::Pale);
    slabs(out, [w, 33.0], [0.0, h], [inner_b, inner_f], Mat::Pale);
    slab(
        out,
        [-33.0, 33.0],
        [h, 85.0],
        [inner_b, inner_f],
        Mat::Stone,
    );
    // A water channel down the middle of the floor, between two white lines.
    out.push(soft(bx(
        Vec3::new(0.0, 0.0, (SEA_BACK + f) * 0.5),
        Vec3::new(3.0, 0.15, (f - SEA_BACK) * 0.5 - 2.0),
        Mat::Water,
    )));
    for s in [-11.0f32, 11.0] {
        out.push(soft(bx(
            Vec3::new(s, 0.0, (SEA_BACK + f) * 0.5),
            Vec3::new(1.0, 0.08, (f - SEA_BACK) * 0.5 - 2.0),
            Mat::White,
        )));
    }
    // Two rows of columns set against the walls, out of the way of anyone flying
    // through, with beams across, and lamps on the walls between.
    for k in 0..9 {
        let z = -60.0 - k as f32 * 30.0;
        for x in [-(w - 2.6), w - 2.6] {
            slab(
                out,
                [x - 3.5, x + 3.5],
                [0.0, 3.0],
                [z - 3.5, z + 3.5],
                Mat::Stone,
            );
            out.push(round(Vec3::new(x, 3.0, z), 66.0, 2.6, 2.3, Mat::White));
            slab(
                out,
                [x - 4.0, x + 4.0],
                [69.0, 73.0],
                [z - 4.0, z + 4.0],
                Mat::Stone,
            );
        }
        slab(out, [-w, w], [73.0, h], [z - 2.0, z + 2.0], Mat::Stone);
        let z = z - 15.0;
        slabs(
            out,
            [w - 0.6, w],
            [24.0, 36.0],
            [z - 1.0, z + 1.0],
            Mat::Lamp,
        );
        slabs(
            out,
            [w - 1.5, w],
            [22.5, 24.0],
            [z - 2.0, z + 2.0],
            Mat::Stone,
        );
    }
    // Halfway along, a round pool and a copper wave rising from it.
    let mid = -180.0;
    out.push(round(Vec3::new(0.0, 0.0, mid), 1.2, 10.0, 10.0, Mat::Stone));
    out.push(soft(round(
        Vec3::new(0.0, 1.2, mid),
        0.1,
        9.0,
        9.0,
        Mat::Water,
    )));
    out.push(round(Vec3::new(0.0, 1.2, mid), 10.0, 2.0, 1.2, Mat::Copper));
    out.push(ball(Vec3::new(0.0, 13.0, mid), 3.0, Mat::Copper));

    // From the cavern doors a broad stair comes down to the lake, between low walls with
    // lamps, onto a landing at the water.
    let b = SEA_BACK;
    for k in 1..=10 {
        let (k, z) = (k as f32, b - k as f32 * 4.0);
        slab(out, [-w, w], [-54.0, -2.0 * k], [z, z + 4.0], Mat::Pale);
        slabs(
            out,
            [w + 0.3, w + 3.3],
            [-54.0, -2.0 * k + 3.0],
            [z, z + 3.9],
            Mat::Stone,
        );
        if (k as u32).is_multiple_of(5) {
            for s in [-1.0f32, 1.0] {
                out.push(soft(ball(
                    Vec3::new(s * (w + 1.5), -2.0 * k + 4.0, z + 2.0),
                    1.0,
                    Mat::Lamp,
                )));
            }
        }
    }
    slab(
        out,
        [-40.0, 40.0],
        [-54.0, -19.5],
        [b - 56.0, b - 40.0],
        Mat::Stone,
    );
}

/// Basalt Gate's inside: half the hall's width and its height.
const BASALT_W: f32 = 34.0;
const BASALT_H: f32 = 66.0;
/// Where its two fronts face out: down the gorge (+z) and into the cavern (-z).
pub const BASALT_FRONT: f32 = 16.0;
const BASALT_BACK: f32 = -334.0;
/// Where the railway ends, at the cave station.
pub const BASALT_END: f32 = -590.0;

/// Basalt Gate: a railway station built through a mountain, from a gorge (+z) to a
/// cavern (-z). Each front is a wall of dark stone round a great round portal ringed in
/// copper, with the station's name and a clock above, between banks of six-sided basalt
/// columns that rise like the rock they came from. Inside, platforms either side of the
/// track under copper arches and hanging lamps. Out the back the track crosses the
/// cavern on a viaduct to a station on a tall stone pier at the far side. The rock is cut
/// a little wider than the hall, and both fronts reach past the cuts into the rock, so no
/// cut edge shows. The frame's origin is on the track at the gorge front.
fn basalt_gate(out: &mut Vec<Piece>, r: impl Fn(u32) -> f32) {
    let (w, h) = (BASALT_W, BASALT_H);
    // The portal: straight sides up to the spring, then round over.
    let (spring, rad) = (30.0f32, 34.0f32);
    for (face, inner, out_dir) in [
        (BASALT_FRONT, -45.0f32, 1.0f32),
        (BASALT_BACK, -270.0, -1.0),
    ] {
        let z = if out_dir > 0.0 {
            [inner, face]
        } else {
            [face, inner]
        };
        let (bottom, top) = if out_dir > 0.0 {
            (-4.0, 166.0)
        } else {
            (-76.0, 156.0)
        };
        let at = |d: f32| face + out_dir * d;
        let span = |a: f32, b: f32| [at(a).min(at(b)), at(a).max(at(b))];
        slabs(out, [w, 94.0], [bottom, top], z, Mat::Stone);
        slab(out, [-w, w], [h, top], z, Mat::Stone);
        if out_dir < 0.0 {
            slab(out, [-w, w], [bottom, -1.8], z, Mat::Stone);
        }
        // Round the portal's top through the front's thickness.
        let mut y = spring;
        while y < h {
            let mid = (y + 1.5 - spring).min(rad);
            let open = (rad * rad - mid * mid).max(0.0).sqrt();
            if open < w {
                slabs(out, [open, w], [y, y + 3.0], z, Mat::Stone);
            }
            y += 3.0;
        }
        // A copper ring round it, a keystone, and the station's name and clock above.
        slabs(
            out,
            [rad + 2.0, rad + 6.0],
            [0.0, spring],
            span(0.0, 2.0),
            Mat::Copper,
        );
        for i in 0..=15 {
            let a = i as f32 / 15.0 * std::f32::consts::PI;
            let (x, yy) = (a.cos() * (rad + 4.0), spring + a.sin() * (rad + 4.0));
            slab(
                out,
                [x - 2.4, x + 2.4],
                [yy - 2.4, yy + 2.4],
                span(0.0, 2.0),
                Mat::Copper,
            );
        }
        slab(out, [-4.0, 4.0], [h, h + 10.0], span(0.0, 4.0), Mat::Copper);
        let read = if out_dir > 0.0 { 2.0 } else { -2.0 };
        spell(
            out,
            "BASALT GATE",
            Vec3::new(0.0, 80.0, at(0.3)),
            read,
            0.2,
            Mat::Lamp,
            false,
        );
        slab(out, [-9.0, 9.0], [97.0, 115.0], span(0.0, 1.0), Mat::Copper);
        slab(out, [-8.0, 8.0], [98.0, 114.0], span(1.0, 1.4), Mat::White);
        // Lit windows in a row, and a green copper cornice.
        // Clear of the clock's frame.
        for i in -4i32..=4 {
            if i.abs() < 2 {
                continue;
            }
            let x = i as f32 * 9.0;
            slab(
                out,
                [x - 2.5, x + 2.5],
                [100.0, 114.0],
                span(0.0, 0.5),
                Mat::Lamp,
            );
        }
        slab(
            out,
            [-95.0, 95.0],
            [140.0, 146.0],
            span(-60.0, 8.0),
            Mat::Copper,
        );
        // Banks of basalt columns either side, taller toward the gorge walls, so the
        // front grows out of the mountain.
        for (row, dz) in [(0u32, 6.0f32), (1, 11.0)] {
            let mut x = 46.0 + row as f32 * 2.7;
            let mut k = row * 100;
            while x < 92.0 {
                for s in [-1.0f32, 1.0] {
                    k += 1;
                    let tall = 90.0 + (x - 46.0) * 1.1 + r(k) * 18.0 - row as f32 * 22.0;
                    out.push(round(
                        Vec3::new(s * x, bottom, at(dz)),
                        tall - bottom,
                        3.0,
                        3.0,
                        Mat::Basalt,
                    ));
                }
                x += 5.4;
            }
        }
        // A stepped crown on top, set back into the mountain.
        for (k, half) in [70.0f32, 46.0].into_iter().enumerate() {
            let y = 146.0 + k as f32 * 8.0;
            slab(
                out,
                [-half, half],
                [y, y + 8.0],
                span(-60.0, 2.0 - k as f32 * 4.0),
                Mat::Basalt,
            );
        }
    }

    // The hall: floor, walls and copper ceiling lining the rock.
    let (inner_f, inner_b) = (-45.0f32, -270.0f32);
    slab(
        out,
        [-40.8, 40.8],
        [-1.8, -0.6],
        [BASALT_BACK + 0.4, BASALT_FRONT - 0.4],
        Mat::Basalt,
    );
    slabs(out, [w, 41.0], [-1.8, h], [inner_b, inner_f], Mat::Stone);
    slab(
        out,
        [-41.0, 41.0],
        [h, 73.0],
        [inner_b, inner_f],
        Mat::Copper,
    );
    // Platforms either side of the track, with lamps and benches.
    slabs(
        out,
        [4.5, 22.0],
        [-0.3, 1.3],
        [-250.0, -60.0],
        Mat::Concrete,
    );
    slabs(out, [4.5, 5.0], [1.3, 1.36], [-250.0, -60.0], Mat::Lamp);
    for k in 0..8 {
        let z = -72.0 - k as f32 * 25.0;
        for s in [-1.0f32, 1.0] {
            out.push(round(
                Vec3::new(s * 16.0, 1.3, z),
                5.0,
                0.25,
                0.25,
                Mat::Metal,
            ));
            out.push(soft(ball(Vec3::new(s * 16.0, 6.8, z), 0.8, Mat::Lamp)));
            slab(
                out,
                [s * 19.0 - 0.6, s * 19.0 + 0.6],
                [1.3, 2.0],
                [z + 6.0, z + 9.0],
                Mat::Wood,
            );
        }
    }
    // Copper arches across the hall, basalt columns down the walls, lamps hung between.
    for k in 0..9 {
        let z = -50.0 - k as f32 * 25.0;
        for i in 1..=11 {
            let a = i as f32 / 12.0 * std::f32::consts::PI;
            let (x, y) = (a.cos() * w, 32.0 + a.sin() * w);
            slab(
                out,
                [x - 1.6, x + 1.6],
                [y - 1.6, y + 1.6],
                [z - 1.2, z + 1.2],
                Mat::Copper,
            );
        }
        for s in [-1.0f32, 1.0] {
            out.push(round(
                Vec3::new(s * (w - 2.6), -0.3, z),
                32.3,
                2.6,
                2.6,
                Mat::Basalt,
            ));
            // Hung on thin rods that wings pass through.
            let z = z - 12.5;
            out.push(soft(round(
                Vec3::new(s * 12.0, 42.0, z),
                24.0,
                0.15,
                0.15,
                Mat::Metal,
            )));
            out.push(soft(ball(Vec3::new(s * 12.0, 41.0, z), 1.6, Mat::Lamp)));
        }
    }

    // The viaduct across the cavern, and the cave station on its pier.
    let (b, end) = (BASALT_BACK, BASALT_END);
    slab(out, [-6.0, 6.0], [-3.0, -0.6], [end + 30.0, b], Mat::Stone);
    slabs(out, [5.0, 6.0], [-0.3, 1.2], [end + 30.0, b], Mat::Stone);
    let mut z = b - 30.0;
    while z > end + 40.0 {
        slab(
            out,
            [-5.0, 5.0],
            [-84.0, -3.0],
            [z - 3.0, z + 3.0],
            Mat::Basalt,
        );
        for (d, drop) in [(3.0f32, 10.0f32), (8.0, 6.0)] {
            slab(
                out,
                [-6.0, 6.0],
                [-3.0 - drop, -3.0],
                [z + d, z + d + 5.0],
                Mat::Stone,
            );
            slab(
                out,
                [-6.0, 6.0],
                [-3.0 - drop, -3.0],
                [z - d - 5.0, z - d],
                Mat::Stone,
            );
        }
        z -= 30.0;
    }
    slab(
        out,
        [-24.0, 24.0],
        [-84.0, -0.6],
        [end - 20.0, end + 30.0],
        Mat::Basalt,
    );
    slabs(
        out,
        [4.5, 20.0],
        [-0.3, 1.3],
        [end - 15.0, end + 25.0],
        Mat::Concrete,
    );
    slabs(
        out,
        [4.5, 5.0],
        [1.3, 1.36],
        [end - 15.0, end + 25.0],
        Mat::Lamp,
    );
    slab(
        out,
        [10.0, 20.0],
        [1.3, 8.0],
        [end - 12.0, end + 2.0],
        Mat::Stone,
    );
    slab(
        out,
        [9.0, 21.0],
        [8.0, 9.0],
        [end - 13.0, end + 3.0],
        Mat::Copper,
    );
    slab(
        out,
        [-2.0, 2.0],
        [-0.3, 1.5],
        [end - 6.0, end - 3.0],
        Mat::Red,
    );
    for (x, z) in [
        (-20.0f32, end - 15.0),
        (-20.0, end + 25.0),
        (20.0, end + 25.0),
    ] {
        out.push(round(Vec3::new(x, 1.3, z), 5.0, 0.25, 0.25, Mat::Metal));
        out.push(soft(ball(Vec3::new(x, 6.8, z), 0.8, Mat::Lamp)));
    }
}

/// How far broad ground paving reaches down under a landmark's foot.
const PLINTH: f32 = 6.0;

/// How big a landmark is, from its pieces: width along x, height, depth along z.
/// Half the width and depth of the ground a landmark covers, in its own frame.
pub fn footprint(pieces: &[Piece]) -> Vec2 {
    let mut e = Vec2::ZERO;
    for p in pieces {
        let (at, reach) = match p.form {
            Form::Box {
                at, half, yaw: 0.0, ..
            } => (at, Vec2::new(half.x, half.z)),
            Form::Box { at, half, .. } => (at, Vec2::splat(Vec2::new(half.x, half.z).length())),
            Form::Round { at, r0, r1, .. } => (at, Vec2::splat(r0.max(r1))),
            Form::Ball { at, r } => (at, Vec2::splat(r)),
            Form::Letter { at, cell, .. } => (at, Vec2::splat(cell.abs() * 1.5)),
        };
        e = e.max(Vec2::new(at.x.abs(), at.z.abs()) + reach);
    }
    e
}

pub fn extent(pieces: &[Piece]) -> Vec3 {
    let mut e = Vec3::splat(1.0);
    for p in pieces {
        let (at, reach, top) = match p.form {
            Form::Box { at, half, roof, .. } => {
                (at, Vec2::new(half.x, half.z).length(), half.y + roof)
            }
            Form::Round { at, h, r0, r1 } => (at, r0.max(r1), h),
            Form::Ball { at, r } => (at - Vec3::Y * r, r, r * 2.0),
            Form::Letter { at, cell, .. } => (at, cell.abs() * 1.5, cell.abs() * 5.0),
        };
        e.x = e.x.max((at.x.abs() + reach) * 2.0);
        e.z = e.z.max((at.z.abs() + reach) * 2.0);
        e.y = e.y.max(at.y + top);
    }
    e
}
