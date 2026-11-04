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
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub form: Form,
    pub mat: Mat,
    /// Holds you up and stops you.
    pub solid: bool,
}

/// The kinds a world file can name, and what they become.
pub const KINDS: [(&str, Kind); 57] = [
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
    ("span-tower", Kind::SpanTower),
    ("market", Kind::Market),
    ("observatory", Kind::Observatory),
    ("lookout", Kind::Lookout),
    ("surfshack", Kind::SurfShack),
    ("ropebridge", Kind::RopeBridge),
    ("hospital", Kind::Hospital),
    ("firestation", Kind::FireStation),
    ("school", Kind::School),
    ("townhall", Kind::TownHall),
    ("farm", Kind::Farm),
    ("port", Kind::Port),
];

/// Where a fairground wheel turns, in the landmark's frame, and its radius.
pub const FERRIS_HUB: Vec3 = Vec3::new(0.0, 21.0, 0.0);
pub const FERRIS_R: f32 = 17.0;
/// Where a rope swing hangs from its branch, and how long its ropes are.
pub const SWING_PIVOT: Vec3 = Vec3::new(9.0, 8.0, 0.0);
pub const SWING_ROPE: f32 = 6.6;
/// A town park's benches, in its frame.
pub const PARK_BENCHES: [Vec3; 2] = [Vec3::new(-8.0, 0.0, 2.6), Vec3::new(8.0, 0.0, 2.6)];

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

/// A theme park's drop tower, and how tall it stands.
pub const DROP: Vec3 = Vec3::new(-55.0, 0.0, 35.0);
pub const DROP_TALL: f32 = 60.0;

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

/// Big blocky letters centered on `at` along x, standing up from it, each `cell` square
/// and `depth` deep either side: each column of cells merged into one block. A negative
/// `cell` runs the text toward -x, to read from the -z side.
fn spell(out: &mut Vec<Piece>, text: &str, at: Vec3, step: f32, depth: f32, mat: Mat, solid: bool) {
    let cell = step.abs();
    let letters: Vec<char> = text.chars().map(|c| c.to_ascii_uppercase()).collect();
    let width = letters.len() as f32 * 4.0 * step - step;
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
fn metro_gate(out: &mut Vec<Piece>) {
    let (hole, pave) = (10.8, 17.0);
    let rim = (pave + hole) * 0.5;
    let band = (pave - hole) * 0.5;
    for (x, z, hx, hz) in [
        (0.0f32, rim, pave, band),
        (0.0, -rim, pave, band),
        (rim, 0.0, band, hole),
        (-rim, 0.0, band, hole),
    ] {
        out.push(soft(bx(
            Vec3::new(x, -0.3, z),
            Vec3::new(hx, 0.5, hz),
            Mat::Concrete,
        )));
    }
    let posts = 20;
    for k in 0..posts {
        let (a, b) = (
            k as f32 / posts as f32 * std::f32::consts::TAU,
            (k + 1) as f32 / posts as f32 * std::f32::consts::TAU,
        );
        let r = hole + 0.4;
        let (p, q) = (
            Vec3::new(a.cos() * r, 0.2, a.sin() * r),
            Vec3::new(b.cos() * r, 0.2, b.sin() * r),
        );
        out.push(soft(round(p, 1.1, 0.06, 0.06, Mat::Canvas(3))));
        let d = q - p;
        out.push(soft(turned(
            (p + q) * 0.5 + Vec3::Y * 1.2,
            Vec3::new(d.length() * 0.5, 0.08, 0.06),
            (-d.z).atan2(d.x),
            Mat::Canvas(3),
        )));
    }
    for (x, z) in [(1.0f32, 1.0f32), (-1.0, -1.0)] {
        let at = Vec3::new(x * (hole + 3.0), 0.2, z * (hole + 3.0));
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
fn town_hall(out: &mut Vec<Piece>) {
    let (hx, tall, hz) = (18.0, 12.0, 11.0);
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
        out.push(soft(bx(face, half, Mat::White)));
        let hand = face + Vec3::new(dx * 0.1, 2.4, dz * 0.1);
        let (a, b) = if dz != 0.0 {
            (Vec3::new(0.12, 1.8, 0.05), Vec3::new(1.4, 0.12, 0.05))
        } else {
            (Vec3::new(0.05, 1.8, 0.12), Vec3::new(0.05, 0.12, 1.4))
        };
        out.push(soft(bx(hand, a, Mat::Dark)));
        out.push(soft(bx(hand, b, Mat::Dark)));
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
    let (gap, w, d, tall) = (17.0, 8.0, 12.0, 96.0);
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
fn glyph(c: char) -> [u8; 5] {
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
pub fn pieces(kind: Kind, seed: u32, text: &str) -> Vec<Piece> {
    let r = |k: u32| hash(seed.wrapping_mul(31).wrapping_add(k));
    let mut out = Vec::new();
    match kind {
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
            for (k, x) in [-2.5f32, 2.5].into_iter().enumerate() {
                let c = Vec3::new(x, 0.0, r(k as u32) * 2.0 - 1.0);
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
            // Pools, a lazy river round them, a tower of slides and loungers in rows.
            out.push(soft(bx(
                Vec3::ZERO,
                Vec3::new(70.0, 0.2, 50.0),
                Mat::Concrete,
            )));
            out.push(soft(bx(
                Vec3::new(-25.0, 0.0, -10.0),
                Vec3::new(22.0, 0.35, 16.0),
                Mat::Water,
            )));
            out.push(soft(round(
                Vec3::new(30.0, 0.0, -15.0),
                0.35,
                14.0,
                14.0,
                Mat::Water,
            )));
            out.push(soft(round(
                Vec3::new(30.0, 0.0, 22.0),
                0.35,
                8.0,
                8.0,
                Mat::Water,
            )));
            // The lazy river: a ring of water round the middle pool.
            let (mid, radii) = LAZY_RIVER;
            for k in 0..28u32 {
                let a = k as f32 / 28.0 * std::f32::consts::TAU;
                let at = Vec3::new(mid.x + a.cos() * radii.x, 0.0, mid.y + a.sin() * radii.y);
                out.push(soft(turned(
                    at,
                    Vec3::new(3.6, 0.36, 2.2),
                    -a + std::f32::consts::FRAC_PI_2,
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
            out.push(soft(bx(Vec3::ZERO, Vec3::new(85.0, 0.15, 60.0), Mat::Sand)));
            let n = 64;
            for k in 0..n {
                let (a, b) = (
                    coaster(k as f32 / n as f32),
                    coaster((k + 1) as f32 / n as f32),
                );
                let d = b - a;
                let mid = (a + b) * 0.5;
                out.push(soft(turned(
                    mid - Vec3::Y * 0.3,
                    Vec3::new(d.length() * 0.5 + 0.2, 0.3, 1.0),
                    (-d.z).atan2(d.x),
                    Mat::Red,
                )));
                if k % 2 == 0 {
                    out.push(soft(round(
                        Vec3::new(a.x, 0.0, a.z),
                        a.y - 0.3,
                        0.25,
                        0.2,
                        Mat::White,
                    )));
                }
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
            // Paths: a loop round the middle and the way in from the gate.
            for k in 0..24u32 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                out.push(soft(turned(
                    Vec3::new(a.cos() * 38.0, 0.0, a.sin() * 28.0),
                    Vec3::new(5.5, 0.14, 2.0),
                    -a + std::f32::consts::FRAC_PI_2,
                    Mat::Concrete,
                )));
            }
            out.push(soft(bx(
                Vec3::new(0.0, 0.0, 47.0),
                Vec3::new(2.5, 0.14, 15.0),
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
        Kind::Statue => {
            // A great copper figure on a stone island pedestal, holding a lamp up high to
            // welcome ships in. The lamp shines at night.
            out.push(round(Vec3::Y * -6.0, 10.0, 22.0, 20.0, Mat::Stone));
            out.push(bx(Vec3::Y * 4.0, Vec3::new(9.0, 14.0, 9.0), Mat::Stone));
            out.push(bx(Vec3::Y * 18.0, Vec3::new(7.0, 2.0, 7.0), Mat::Stone));
            out.push(round(Vec3::Y * 20.0, 22.0, 6.5, 3.4, Mat::Copper));
            out.push(round(Vec3::Y * 42.0, 6.0, 3.6, 3.0, Mat::Copper));
            out.push(ball(Vec3::Y * 51.0, 3.0, Mat::Copper));
            for k in 0..7 {
                let a = k as f32 / 7.0 * std::f32::consts::TAU;
                out.push(soft(round(
                    Vec3::new(a.cos() * 2.6, 52.5, a.sin() * 2.6),
                    3.0,
                    0.5,
                    0.0,
                    Mat::Copper,
                )));
            }
            // The raised arm and the lamp.
            out.push(round(
                Vec3::new(3.6, 44.0, 0.0),
                16.0,
                1.4,
                1.1,
                Mat::Copper,
            ));
            out.push(round(Vec3::new(3.6, 60.0, 0.0), 2.0, 1.4, 2.2, Mat::Copper));
            out.push(soft(ball(Vec3::new(3.6, 63.5, 0.0), 2.0, Mat::Lamp)));
            // A tablet held to the chest.
            out.push(turned(
                Vec3::new(-3.0, 30.0, 2.8),
                Vec3::new(1.4, 6.0, 0.6),
                0.3,
                Mat::Copper,
            ));
        }
        Kind::Mall => {
            // A big shopping mall: a long low building with a glass entrance and a sign
            // band, a car park in front with cars and lamps.
            out.push(bx(Vec3::ZERO, Vec3::new(60.0, 12.0, 30.0), Mat::Concrete));
            out.push(soft(bx(
                Vec3::new(0.0, 10.0, 0.0),
                Vec3::new(60.5, 2.0, 30.5),
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
            // A town park: lawn, paths, shade trees, benches, a duck pond and a playground
            // with a slide, swings, a seesaw, a climbing frame and a sandpit.
            out.push(soft(bx(
                Vec3::ZERO,
                Vec3::new(30.0, 0.12, 22.0),
                Mat::Grass,
            )));
            out.push(soft(bx(Vec3::ZERO, Vec3::new(30.0, 0.16, 1.4), Mat::Sand)));
            out.push(soft(bx(Vec3::ZERO, Vec3::new(1.4, 0.16, 22.0), Mat::Sand)));
            for k in 0..7u32 {
                let a = k as f32 * 2.3 + r(k);
                let at = Vec3::new(a.cos() * (12.0 + r(k + 9) * 12.0), 0.0, a.sin() * 15.0);
                if at.x.abs() < 3.0 || at.z.abs() < 3.0 {
                    continue;
                }
                out.push(round(at, 4.0, 0.5, 0.4, Mat::Wood));
                out.push(ball(at + Vec3::Y * 5.5, 2.8 + r(k + 3), Mat::Leaf));
            }
            out.push(soft(round(
                Vec3::new(-16.0, 0.0, 12.0),
                0.25,
                6.0,
                6.0,
                Mat::Water,
            )));
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
        Kind::SpanTower => span_tower(&mut out),
        Kind::Market => market(&mut out, seed),
        Kind::Observatory => observatory(&mut out),
        Kind::Lookout => lookout(&mut out),
        Kind::SurfShack => surf_shack(&mut out),
        Kind::RopeBridge => rope_bridge(&mut out, text),
        Kind::Hospital => hospital(&mut out),
        Kind::FireStation => fire_station(&mut out),
        Kind::School => school(&mut out),
        Kind::TownHall => town_hall(&mut out),
        Kind::Farm => farm(&mut out),
        Kind::Port => port(&mut out, seed),
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
        Kind::Treehouse => 1.3,
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
            };
        }
    }
    out
}

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
        };
        e.x = e.x.max((at.x.abs() + reach) * 2.0);
        e.z = e.z.max((at.z.abs() + reach) * 2.0);
        e.y = e.y.max(at.y + top);
    }
    e
}
