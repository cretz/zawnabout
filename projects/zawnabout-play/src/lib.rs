//! Gameplay, laid on top of the islands. It reads the world and one player's dragon and
//! never changes either, so with it gone the islands are exactly as they are. Each
//! player has their own: a hoard and its lair for now, more layers later. What it keeps
//! saves as text, a section a layer, keyed by the stable ids in `zawnabout-world/first/play.kdl`.

use glam::Vec3;
use zawnabout_sim::dragon::{Dragon, Mode};
use zawnabout_sim::world::World;

pub use flying::{CourseDef, Flying, GateDef, RaceDef, Run, StuntDef};
pub use happen::{FavorDef, HappeningDef, Happenings};
pub use kin::{Egg, Kin, KinDef, KinNews};
pub use story::{LampDef, RegionDef, Stories, StoryDef};

pub mod flying;
pub mod happen;
pub mod kin;
pub mod paint;
pub mod save;
pub mod story;

pub struct LairDef {
    pub id: &'static str,
    pub name: &'static str,
    /// Where the hoard heaps up: on the first surface below this, the cave floor.
    pub at: [f32; 3],
    /// Landing within this far of `at` counts as being in the lair.
    pub r: f32,
}

pub struct SetDef {
    pub id: &'static str,
    pub name: &'static str,
    /// Said when the last piece is found.
    pub done: &'static str,
    /// What finishing it brings to the islands.
    pub effects: &'static [EffectDef],
}

/// Something a finished set brings to the islands, drawn by the game and never part of
/// the world itself: `folk` (someone standing at `a`, with a name and a line to say),
/// `bunting` (flags strung from `a` to `b` on poles) or `lanterns` (paper lanterns
/// strung the same way, glowing at night). Points are x, height over the ground, z.
pub struct EffectDef {
    pub kind: &'static str,
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub name: &'static str,
    pub line: &'static str,
    pub facing: f32,
    pub look: u32,
}

pub struct FindDef {
    pub id: &'static str,
    /// The model, or `coins` for a plain pile with no card.
    pub kind: &'static str,
    pub name: &'static str,
    pub story: &'static str,
    pub x: f32,
    /// Where to look down from for the surface it rests on. Unset, it rests on the top.
    pub y: Option<f32>,
    pub z: f32,
    pub set: Option<&'static str>,
    /// For a set piece, a gentle riddle shown while it is still missing.
    pub hint: &'static str,
    /// How much it adds to the heap.
    pub gold: u32,
    /// Only out in the dark: it glows at night and is nowhere to be seen by day.
    pub night: bool,
    /// Wind chimes ring where it rests until it is found, for finding it by ear.
    pub chimes: bool,
}

/// A beautiful spot. Landing there takes a postcard: a painted picture of the view.
pub struct PostcardDef {
    pub id: &'static str,
    pub name: &'static str,
    /// One line about the place, written on the card.
    pub line: &'static str,
    /// The island it is on, for grouping in the journal.
    pub island: &'static str,
    pub x: f32,
    pub y: Option<f32>,
    pub z: f32,
    /// Landing within this far counts.
    pub r: f32,
    /// Below zero, its picture looks up at it from below, as at a falls whose spot is
    /// at the top.
    pub aim: f32,
}

/// A keepsake given in thanks, added to the hoard.
pub struct GiftDef {
    pub kind: &'static str,
    pub name: &'static str,
    pub story: &'static str,
}

/// Something lost for a fetch request, lying where it fell.
pub struct ItemDef {
    pub kind: &'static str,
    pub name: &'static str,
    pub x: f32,
    pub y: Option<f32>,
    pub z: f32,
}

/// A small favor a local asks: `return` a find from the hoard, `light` a beacon, bring a
/// `postcard`, `fetch` a lost item back to them, or give them a `ride` to a postcard spot.
pub struct RequestDef {
    pub kind: &'static str,
    /// The find, beacon or postcard it is about.
    pub target: &'static str,
    /// What they ask, the first time.
    pub ask: &'static str,
    /// A reminder of where to look, for the journal and when you come back.
    pub hint: &'static str,
    pub thanks: &'static str,
    pub gold: u32,
    pub gift: Option<GiftDef>,
    pub item: Option<ItemDef>,
}

/// A named local, standing at their spot in town, with things to say.
pub struct LocalDef {
    pub id: &'static str,
    pub name: &'static str,
    pub town: &'static str,
    pub x: f32,
    pub y: Option<f32>,
    pub z: f32,
    /// Which way they face when nobody is about, degrees from +z toward +x.
    pub facing: f32,
    /// Picks their head and clothes.
    pub look: u32,
    pub lines: &'static [&'static str],
    pub request: Option<RequestDef>,
}

pub struct PlayDef {
    pub lairs: &'static [LairDef],
    pub sets: &'static [SetDef],
    pub finds: &'static [FindDef],
    pub postcards: &'static [PostcardDef],
    pub locals: &'static [LocalDef],
    pub stunts: &'static [StuntDef],
    pub courses: &'static [CourseDef],
    pub races: &'static [RaceDef],
    pub stories: &'static [StoryDef],
    pub kin: &'static [KinDef],
    pub happenings: &'static [HappeningDef],
    pub favors: &'static [FavorDef],
}

include!(concat!(env!("OUT_DIR"), "/play.rs"));

/// How close a dragon on the ground must be to pick a find up.
pub const REACH: f32 = 4.5;

/// How many 2 m cells across a lair of radius `r` is.
fn lair_cells(r: f32) -> usize {
    (r / 1.0).ceil() as usize
}

/// The middle of cell `cx`, `cz` round a lair heap at `at`.
fn lair_cell_at(at: Vec3, r: f32, cx: usize, cz: usize) -> (f32, f32) {
    (
        at.x - r + cx as f32 * 2.0 + 1.0,
        at.z - r + cz as f32 * 2.0 + 1.0,
    )
}

/// Something that happened, for the game to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A find was picked up: its index in [`PLAY`]'s finds.
    Found(usize),
    /// The dragon landed in a lair that is not theirs yet.
    Lair(usize),
    /// The dragon landed by lair `i`'s way in, out on the open ground short of it.
    LairNear(usize),
    /// A postcard was taken: its index in [`PLAY`]'s postcards. Its picture comes after.
    Postcard(usize),
    /// The last piece of set `i` was found.
    SetDone(usize),
    /// Picked up local `i`'s lost item, to carry back to them.
    Carry(usize),
    /// Flew local `i` where they wanted to go. They hop off, delighted.
    Arrived(usize),
    /// Brought local `i` back home before getting there. No harm done.
    HomeAgain(usize),
    /// Picked up a townsperson's dropped thing.
    ChoreCarry,
    /// Flew a townsperson where they wanted to go.
    ChoreArrived,
    /// Took a townsperson's letter or parcel where it was going.
    ChoreDelivered,
    /// Did stunt `i` for the first time.
    Stunt(usize),
    /// Passed gate `g` of course `c`, not the last.
    Gate(usize, usize),
    /// Passed the last gate of course `c`.
    Course(usize),
    /// Race `i` is on: its vehicle is off.
    RaceOn(usize),
    /// Won race `i`, and whether for the first time.
    RaceWon(usize, bool),
    /// Race `i`'s vehicle got there first.
    RaceLost(usize),
    /// Lit lamp `k` of region `r` of story `s`, not the region's last.
    Lamp(usize, usize, usize),
    /// Lit the last lamp of region `r` of story `s`: its lights are back.
    RegionLit(usize, usize),
    /// Every lamp of story `s` is lit.
    StoryDone(usize),
    /// Got to happening `i` while it was on.
    Happening(usize),
    /// A town put up its thanks `i`.
    Favor(usize),
}

/// A small thing a townsperson asks, just for fun: no gold and no favor, so there is
/// nothing to gain by doing them over and over. One at a time, forgotten between visits.
#[derive(Clone, Debug, PartialEq)]
pub struct Chore {
    /// Who asked: the townsperson's number, and where they were.
    pub seed: u32,
    pub home: Vec3,
    pub kind: ChoreKind,
    pub accepted: bool,
    /// The dropped thing is in the dragon's arms, or the townsperson on its back.
    pub aboard: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChoreKind {
    /// Something dropped near where they asked.
    Fetch { kind: &'static str, at: Vec3 },
    /// A ride to postcard spot `card`.
    Ride { card: usize },
    /// Something to take to postcard spot `card`: a letter, a parcel, a present. It is in
    /// the dragon's arms from the start.
    Deliver { card: usize, kind: &'static str },
}

fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^ (x >> 16)
}

/// Where a local's request stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Errand {
    #[default]
    New,
    Accepted,
    Done,
}

/// What a local says when talked to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Talk {
    /// One of their lines.
    Line(&'static str),
    /// They ask their favor.
    Offer,
    /// They see their lost thing in your hoard and would love it back.
    Return,
    /// The favor is done: thanks, gold and maybe a gift.
    Thanks,
    /// Still waiting: a reminder of where to look.
    Reminder,
}

/// A find placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub pos: Vec3,
}

/// One set's progress, for the journal.
pub struct SetView {
    pub def: &'static SetDef,
    /// Each piece and whether it is found.
    pub pieces: Vec<(&'static FindDef, bool)>,
}

impl SetView {
    pub fn have(&self) -> usize {
        self.pieces.iter().filter(|(_, f)| *f).count()
    }
}

/// One player's gameplay.
pub struct Play {
    pub def: &'static PlayDef,
    pub spots: Vec<Spot>,
    /// Each lair's heap spot, on its floor.
    pub lairs: Vec<Vec3>,
    /// Each lair's ground in 2 m cells round the heap, true where rock is overhead, so
    /// the grass outside a cave mouth never counts. Empty for a lair out in the open.
    lair_in: Vec<Vec<bool>>,
    /// Each set's effects, their ends on the ground plus their heights.
    pub effect_at: Vec<Vec<(Vec3, Vec3)>>,
    pub found: Vec<bool>,
    /// Each postcard's spot, whether it is taken, and its painted picture as text.
    pub cards: Vec<Vec3>,
    pub taken: Vec<bool>,
    pub pics: Vec<Option<String>>,
    /// Counts up whenever a picture changes, so the game knows to show the new ones.
    pub pics_version: u32,
    /// Where each local stands, and where each fetch request's lost item lies.
    pub folk: Vec<Vec3>,
    pub items: Vec<Option<Vec3>>,
    /// Talked to yet, and each one's request.
    pub met: Vec<bool>,
    pub errands: Vec<Errand>,
    /// The local whose lost item the dragon is carrying.
    pub carrying: Option<usize>,
    /// The local riding on the dragon's back, and whether they have been away from home
    /// yet, so landing back there means a change of plans.
    pub riding: Option<usize>,
    ride_left: bool,
    /// A townsperson's small ask, if one is going.
    pub chore: Option<Chore>,
    /// Which line each local says next.
    said: Vec<usize>,
    /// The lair the hoard is in, once one is picked.
    pub lair: Option<usize>,
    /// The lair just offered, not offered again until the dragon has left it.
    offered: Option<usize>,
    /// The lair whose way in was just pointed out, not again until the dragon has left.
    hinted: Option<usize>,
    /// Stunts, courses and races.
    pub flying: Flying,
    /// Stories, and where each one's lamps stand: story, region, lamp.
    pub stories: Stories,
    pub story_lamps: Vec<Vec<Vec<Vec3>>>,
    /// Whether it is night now, set by the game, for finds that only come out then.
    pub night: bool,
    /// The happening on now, and every one seen.
    pub happen: Happenings,
    /// Each happening's effects, their ends on the ground.
    pub happen_at: Vec<Vec<(Vec3, Vec3)>>,
    /// Towns' thanks put up so far, and where each one's effects stand.
    pub favored: Vec<bool>,
    pub favor_at: Vec<Vec<(Vec3, Vec3)>>,
    /// Other dragons and the family.
    pub kin: Kin,
    /// Each named dragon's home, then its perches, resting on what is under them.
    pub kin_at: Vec<Vec<Vec3>>,
    /// The find each collector took in a trade.
    pub kin_traded_find: Vec<Option<usize>>,
    /// Each named dragon's way to its secret, stop by stop, resting on what is under.
    pub kin_way: Vec<Vec<Vec3>>,
    /// Sections of a loaded save this build does not know, kept as they were.
    other: String,
    /// Something to save changed.
    pub changed: bool,
}

impl Play {
    pub fn new(world: &World) -> Self {
        Self::with(&PLAY, world)
    }

    pub fn with(def: &'static PlayDef, world: &World) -> Self {
        let spots = def
            .finds
            .iter()
            .map(|f| {
                let below = f.y.map_or(5000.0, |y| y + 2.0);
                Spot {
                    pos: Vec3::new(f.x, world.support_still(f.x, f.z, below), f.z),
                }
            })
            .collect();
        let lairs = def
            .lairs
            .iter()
            .map(|l| {
                let [x, y, z] = l.at;
                Vec3::new(x, world.support_still(x, z, y), z)
            })
            .collect();
        let lair_in = def
            .lairs
            .iter()
            .zip(&lairs)
            .map(|(l, at): (&LairDef, &Vec3)| {
                let roofed = |x: f32, z: f32| world.terrain.height(x, z) > at.y + 6.0;
                if !roofed(at.x, at.z) {
                    return Vec::new();
                }
                let n = lair_cells(l.r);
                (0..n * n)
                    .map(|k| {
                        let (x, z) = lair_cell_at(*at, l.r, k % n, k / n);
                        roofed(x, z)
                    })
                    .collect()
            })
            .collect();
        let cards = def
            .postcards
            .iter()
            .map(|c| {
                let below = c.y.map_or(5000.0, |y| y + 2.0);
                Vec3::new(c.x, world.support_still(c.x, c.z, below), c.z)
            })
            .collect();
        let rest = |x: f32, y: Option<f32>, z: f32| {
            let below = y.map_or(5000.0, |y| y + 2.0);
            Vec3::new(x, world.support_still(x, z, below), z)
        };
        let folk = def.locals.iter().map(|l| rest(l.x, l.y, l.z)).collect();
        let items = def
            .locals
            .iter()
            .map(|l| {
                let i = l.request.as_ref()?.item.as_ref()?;
                Some(rest(i.x, i.y, i.z))
            })
            .collect();
        let lift = |p: [f32; 3]| {
            let y = world.support_still(p[0], p[2], 5000.0);
            Vec3::new(p[0], y + p[1], p[2])
        };
        let effect_at = def
            .sets
            .iter()
            .map(|s| s.effects.iter().map(|e| (lift(e.a), lift(e.b))).collect())
            .collect();
        // Geese fly their height up from the sea, wherever the ground below.
        let happen_at = def
            .happenings
            .iter()
            .map(|h| {
                h.effects
                    .iter()
                    .map(|e| match e.kind {
                        "geese" => (Vec3::from(e.a), Vec3::from(e.b)),
                        _ => (lift(e.a), lift(e.b)),
                    })
                    .collect()
            })
            .collect();
        let favor_at = def
            .favors
            .iter()
            .map(|f| f.effects.iter().map(|e| (lift(e.a), lift(e.b))).collect())
            .collect();
        let n = def.locals.len();
        let settle = |p: [f32; 3]| {
            let below = if p[1].is_nan() { 5000.0 } else { p[1] + 2.0 };
            Vec3::new(p[0], world.support_still(p[0], p[2], below), p[2])
        };
        let kin_at = def
            .kin
            .iter()
            .map(|k| {
                std::iter::once([k.x, k.y.unwrap_or(f32::NAN), k.z])
                    .chain(k.perches.iter().copied())
                    .map(settle)
                    .collect()
            })
            .collect();
        let kin_way = def
            .kin
            .iter()
            .map(|k| k.way.iter().copied().map(settle).collect())
            .collect();
        Self {
            night: false,
            happen: Happenings {
                current: 0,
                seen_now: false,
                seen: vec![false; def.happenings.len()],
            },
            happen_at,
            favored: vec![false; def.favors.len()],
            favor_at,
            kin: Kin::new(def.kin.len()),
            kin_at,
            kin_way,
            kin_traded_find: vec![None; def.kin.len()],
            def,
            spots,
            lairs,
            lair_in,
            effect_at,
            folk,
            items,
            met: vec![false; n],
            errands: vec![Errand::New; n],
            carrying: None,
            riding: None,
            ride_left: false,
            chore: None,
            said: vec![0; n],
            cards,
            taken: vec![false; def.postcards.len()],
            pics: vec![None; def.postcards.len()],
            pics_version: 0,
            found: vec![false; def.finds.len()],
            lair: None,
            offered: None,
            hinted: None,
            flying: Flying::new(
                def.stunts.len(),
                def.courses.iter().map(|c| c.gates.len()).collect(),
                def.races.len(),
            ),
            stories: Stories::new(def.stories),
            story_lamps: def
                .stories
                .iter()
                .map(|s| {
                    s.regions
                        .iter()
                        .map(|r| r.lamps.iter().map(|l| rest(l.x, l.y, l.z)).collect())
                        .collect()
                })
                .collect(),
            other: String::new(),
            changed: false,
        }
    }

    /// Looks at the player's dragon once a tick, and says what happened.
    pub fn tick(&mut self, me: &Dragon) -> Vec<Event> {
        let mut out = Vec::new();
        let grounded = me.mode == Mode::Ground;
        // Standing by a find picks it up, and so does hovering slowly right next to one:
        // some rest where landing is fiddly, like a lighthouse's roof.
        let hovering = me.mode == Mode::Air && me.vel.length() < 8.0;
        if grounded || hovering {
            for (i, s) in self.spots.iter().enumerate() {
                let flat = (s.pos.x - me.pos.x).hypot(s.pos.z - me.pos.z);
                let near = if grounded {
                    flat < REACH && (s.pos.y - me.pos.y).abs() < 5.0
                } else {
                    s.pos.distance(me.pos) < REACH + 1.0
                };
                if !self.found[i] && near && (self.night || !self.def.finds[i].night) {
                    self.found[i] = true;
                    self.changed = true;
                    out.push(Event::Found(i));
                    if let Some(set) = self.def.finds[i].set
                        && let Some(s) = self.def.sets.iter().position(|d| d.id == set)
                        && self.set_done(s)
                    {
                        out.push(Event::SetDone(s));
                    }
                }
            }
        }
        if grounded {
            for (i, (c, at)) in self.def.postcards.iter().zip(&self.cards).enumerate() {
                let flat = (at.x - me.pos.x).hypot(at.z - me.pos.z);
                if !self.taken[i] && flat < c.r && (at.y - me.pos.y).abs() < c.r.max(40.0) {
                    self.taken[i] = true;
                    self.changed = true;
                    out.push(Event::Postcard(i));
                }
            }
        }
        if grounded && self.carrying.is_none() {
            for (i, item) in self.items.iter().enumerate() {
                let Some(at) = item else { continue };
                let flat = (at.x - me.pos.x).hypot(at.z - me.pos.z);
                if self.errands[i] == Errand::Accepted
                    && flat < REACH
                    && (at.y - me.pos.y).abs() < 5.0
                {
                    self.carrying = Some(i);
                    self.changed = true;
                    out.push(Event::Carry(i));
                    break;
                }
            }
        }
        // A passenger hops off at their spot, or back home if plans change.
        if let Some(i) = self.riding {
            let home = self.folk[i];
            let flat_home = (home.x - me.pos.x).hypot(home.z - me.pos.z);
            if flat_home > 60.0 {
                self.ride_left = true;
            }
            let card = self.def.locals[i]
                .request
                .as_ref()
                .and_then(|r| self.def.postcards.iter().position(|c| c.id == r.target));
            if grounded
                && let Some(c) = card
                && self.near_card(c, me)
            {
                self.riding = None;
                self.errands[i] = Errand::Done;
                self.changed = true;
                out.push(Event::Arrived(i));
            } else if grounded && self.ride_left && flat_home < 15.0 {
                self.riding = None;
                self.errands[i] = Errand::New;
                self.changed = true;
                out.push(Event::HomeAgain(i));
            }
        }
        let mut arrived = false;
        let mut delivered = false;
        if let Some(ch) = &mut self.chore
            && ch.accepted
            && grounded
        {
            match ch.kind {
                ChoreKind::Fetch { at, .. } if !ch.aboard => {
                    if (at.x - me.pos.x).hypot(at.z - me.pos.z) < REACH
                        && (at.y - me.pos.y).abs() < 5.0
                    {
                        ch.aboard = true;
                        out.push(Event::ChoreCarry);
                    }
                }
                ChoreKind::Ride { card } if ch.aboard => {
                    let (c, at) = (&self.def.postcards[card], self.cards[card]);
                    if (at.x - me.pos.x).hypot(at.z - me.pos.z) < c.r
                        && (at.y - me.pos.y).abs() < c.r.max(40.0)
                    {
                        arrived = true;
                    }
                }
                ChoreKind::Deliver { card, .. } => {
                    let (c, at) = (&self.def.postcards[card], self.cards[card]);
                    if (at.x - me.pos.x).hypot(at.z - me.pos.z) < c.r
                        && (at.y - me.pos.y).abs() < c.r.max(40.0)
                    {
                        delivered = true;
                    }
                }
                _ => {}
            }
        }
        if arrived {
            self.chore = None;
            out.push(Event::ChoreArrived);
        }
        if delivered {
            self.chore = None;
            out.push(Event::ChoreDelivered);
        }
        for (i, (l, at)) in self.def.lairs.iter().zip(&self.lairs).enumerate() {
            let d = at.distance(me.pos);
            if self.offered == Some(i) && d > l.r * 1.5 {
                self.offered = None;
            }
            if self.hinted == Some(i) && d > l.r * 3.0 {
                self.hinted = None;
            }
            if !grounded || self.lair == Some(i) || self.offered == Some(i) {
                continue;
            }
            if d < l.r && self.in_lair(i, me.pos) {
                self.offered = Some(i);
                out.push(Event::Lair(i));
            } else if d < l.r * 2.5 && self.hinted != Some(i) {
                self.hinted = Some(i);
                out.push(Event::LairNear(i));
            }
        }
        out.extend(self.happen_tick(me));
        out
    }

    /// Whether `p` is under the rock of lair `i`, not out on the grass by its mouth.
    fn in_lair(&self, i: usize, p: Vec3) -> bool {
        let cells = &self.lair_in[i];
        if cells.is_empty() {
            return true;
        }
        let (at, r) = (self.lairs[i], self.def.lairs[i].r);
        let n = lair_cells(r);
        let cell = |v: f32| ((v + r) / 2.0).floor() as isize;
        let (cx, cz) = (cell(p.x - at.x), cell(p.z - at.z));
        (0..n as isize).contains(&cx)
            && (0..n as isize).contains(&cz)
            && cells[cz as usize * n + cx as usize]
    }

    /// Whether the dragon stands within postcard spot `c`.
    fn near_card(&self, c: usize, me: &Dragon) -> bool {
        let (card, at) = (&self.def.postcards[c], self.cards[c]);
        (at.x - me.pos.x).hypot(at.z - me.pos.z) < card.r
            && (at.y - me.pos.y).abs() < card.r.max(40.0)
    }

    /// Lair `i` is not wanted now: it is offered again once the dragon has left and come
    /// back.
    pub fn decline(&mut self, i: usize) {
        self.offered = Some(i);
    }

    /// Whether the dragon's back is free for a passenger.
    pub fn back_free(&self) -> bool {
        self.riding.is_none()
            && !self
                .chore
                .as_ref()
                .is_some_and(|c| c.aboard && matches!(c.kind, ChoreKind::Ride { .. }))
    }

    /// Whether the dragon's arms are free to carry something.
    pub fn arms_free(&self) -> bool {
        self.carrying.is_none()
            && !self.chore.as_ref().is_some_and(|c| {
                c.aboard && matches!(c.kind, ChoreKind::Fetch { .. } | ChoreKind::Deliver { .. })
            })
    }

    /// What townsperson `seed` might ask, standing at `home`: about one in two ask
    /// something. A ride to a postcard spot not too far off, something to take to one,
    /// or to look for something they dropped close by. `rest` finds the ground under a
    /// point.
    pub fn chore_for(
        &self,
        seed: u32,
        home: Vec3,
        rest: &dyn Fn(f32, f32) -> Vec3,
    ) -> Option<ChoreKind> {
        let h = mix(seed ^ 0xC0FF_EE11);
        if !h.is_multiple_of(2) {
            return None;
        }
        let near: Vec<usize> = (0..self.cards.len())
            .filter(|c| {
                let d = (self.cards[*c] - home).length();
                (250.0..1800.0).contains(&d)
            })
            .collect();
        let pick = (h / 2) % 3;
        if !near.is_empty() && pick < 2 {
            let card = near[(h / 8) as usize % near.len()];
            return Some(if pick == 0 {
                ChoreKind::Ride { card }
            } else {
                let kinds = ["map", "basket", "book", "chest"];
                ChoreKind::Deliver {
                    card,
                    kind: kinds[(h / 77) as usize % kinds.len()],
                }
            });
        }
        let a = (h % 360) as f32 * 0.0175;
        let r = 25.0 + (h / 360 % 25) as f32;
        let kinds = ["hat", "basket", "kite", "map"];
        Some(ChoreKind::Fetch {
            kind: kinds[(h / 9000) as usize % kinds.len()],
            at: rest(home.x + a.cos() * r, home.z + a.sin() * r),
        })
    }

    /// Makes lair `i` home, and the hoard moves there.
    pub fn claim(&mut self, i: usize) {
        self.lair = Some(i);
        self.changed = true;
    }

    /// Gold in the heap: every find adds some, coin piles the most, and favors done
    /// bring some more.
    pub fn gold(&self) -> u32 {
        let finds: u32 = self
            .def
            .finds
            .iter()
            .zip(&self.found)
            .filter(|(_, f)| **f)
            .map(|(d, _)| d.gold)
            .sum();
        let thanks: u32 = self
            .def
            .locals
            .iter()
            .zip(&self.errands)
            .filter(|(_, e)| **e == Errand::Done)
            .filter_map(|(l, _)| Some(l.request.as_ref()?.gold))
            .sum();
        finds + thanks + self.prize_gold() + self.story_gold()
    }

    /// Whether find `i` went back to whoever lost it.
    pub fn returned(&self, i: usize) -> bool {
        let id = self.def.finds[i].id;
        self.traded_away(i)
            || self.def.locals.iter().zip(&self.errands).any(|(l, e)| {
                *e == Errand::Done
                    && l.request
                        .as_ref()
                        .is_some_and(|r| r.kind == "return" && r.target == id)
            })
    }

    /// Named finds picked up and still in the hoard, in the order of the file.
    pub fn treasures(&self) -> impl Iterator<Item = &'static FindDef> + '_ {
        self.def
            .finds
            .iter()
            .enumerate()
            .filter(|(i, d)| self.found[*i] && d.kind != "coins" && !self.returned(*i))
            .map(|(_, d)| d)
    }

    /// Keepsakes given in thanks, and prizes from courses and races.
    pub fn gifts(&self) -> impl Iterator<Item = &'static GiftDef> + '_ {
        self.def
            .locals
            .iter()
            .zip(&self.errands)
            .filter(|(_, e)| **e == Errand::Done)
            .filter_map(|(l, _)| l.request.as_ref()?.gift.as_ref())
            .chain(self.prizes())
            .chain(self.story_prizes())
            .chain(self.kin_gifts())
    }

    /// Favors done for the people of `town`.
    pub fn favor(&self, town: &str) -> usize {
        self.def
            .locals
            .iter()
            .zip(&self.errands)
            .filter(|(l, e)| l.town == town && **e == Errand::Done && l.request.is_some())
            .count()
    }

    /// Whether local `i`'s request is ready to be thanked for. `lit` says whether a
    /// beacon burns, by its world id.
    pub fn ready(&self, i: usize, lit: &dyn Fn(&str) -> bool) -> bool {
        let Some(r) = &self.def.locals[i].request else {
            return false;
        };
        match r.kind {
            "light" => lit(r.target),
            "postcard" => self
                .def
                .postcards
                .iter()
                .position(|c| c.id == r.target)
                .is_some_and(|c| self.taken[c]),
            "fetch" => self.carrying == Some(i),
            _ => false,
        }
    }

    /// Talks to local `i`, and says what they say. Thanks are paid right away.
    pub fn talk(&mut self, i: usize, lit: &dyn Fn(&str) -> bool) -> Talk {
        let def = self.def;
        let l = &def.locals[i];
        if !self.met[i] {
            self.met[i] = true;
            self.changed = true;
        }
        let line = |said: &mut Vec<usize>| {
            let k = said[i] % l.lines.len().max(1);
            said[i] += 1;
            Talk::Line(l.lines.get(k).copied().unwrap_or(""))
        };
        let Some(r) = &l.request else {
            return line(&mut self.said);
        };
        match self.errands[i] {
            Errand::Done => line(&mut self.said),
            Errand::New if r.kind == "return" => {
                let have = self
                    .def
                    .finds
                    .iter()
                    .position(|f| f.id == r.target)
                    .is_some_and(|f| self.found[f]);
                if have {
                    Talk::Return
                } else {
                    line(&mut self.said)
                }
            }
            Errand::New => Talk::Offer,
            Errand::Accepted if self.ready(i, lit) => {
                self.errands[i] = Errand::Done;
                if self.carrying == Some(i) {
                    self.carrying = None;
                }
                self.changed = true;
                Talk::Thanks
            }
            Errand::Accepted => Talk::Reminder,
        }
    }

    /// Takes on local `i`'s request. A rider climbs aboard.
    pub fn accept(&mut self, i: usize) {
        self.errands[i] = Errand::Accepted;
        if self.def.locals[i]
            .request
            .as_ref()
            .is_some_and(|r| r.kind == "ride")
        {
            self.riding = Some(i);
            self.ride_left = false;
        }
        self.changed = true;
    }

    /// The passenger hops off where the dragon is: a local's ride is off for now, to ask
    /// for again some time, and a townsperson's is simply over.
    pub fn hop_off(&mut self) {
        if let Some(i) = self.riding.take() {
            self.errands[i] = Errand::New;
            self.ride_left = false;
            self.changed = true;
        }
        if self
            .chore
            .as_ref()
            .is_some_and(|c| c.aboard && matches!(c.kind, ChoreKind::Ride { .. }))
        {
            self.chore = None;
        }
    }

    /// Gives local `i` back what they lost, from the hoard.
    pub fn give_back(&mut self, i: usize) {
        self.errands[i] = Errand::Done;
        self.changed = true;
    }

    /// Whether every piece of set `s` is found.
    pub fn set_done(&self, s: usize) -> bool {
        let id = self.def.sets[s].id;
        self.def
            .finds
            .iter()
            .zip(&self.found)
            .filter(|(f, _)| f.set == Some(id))
            .all(|(_, found)| *found)
    }

    /// What finished sets bring to the islands, with where each one's ends are.
    /// Where wind chimes ring: finds with chimes not found yet.
    pub fn chimes(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.spots
            .iter()
            .zip(self.def.finds)
            .zip(&self.found)
            .filter(|((_, f), found)| f.chimes && !**found)
            .map(|((s, _), _)| s.pos)
    }

    pub fn effects(&self) -> impl Iterator<Item = (&'static EffectDef, Vec3, Vec3)> + '_ {
        let def = self.def;
        def.sets
            .iter()
            .enumerate()
            .filter(|(s, _)| self.set_done(*s))
            .flat_map(move |(s, set)| {
                set.effects
                    .iter()
                    .zip(&self.effect_at[s])
                    .map(|(e, (a, b))| (e, *a, *b))
            })
            .chain(self.happening_effects())
            .chain(self.favor_effects())
    }

    pub fn sets(&self) -> Vec<SetView> {
        self.def
            .sets
            .iter()
            .map(|s| SetView {
                def: s,
                pieces: self
                    .def
                    .finds
                    .iter()
                    .zip(&self.found)
                    .filter(|(d, _)| d.set == Some(s.id))
                    .map(|(d, f)| (d, *f))
                    .collect(),
            })
            .collect()
    }

    /// Keeps postcard `i`'s painted picture, from [`paint::paint`].
    pub fn set_pic(&mut self, i: usize, pic: &[u8]) {
        self.pics[i] = Some(paint::encode(pic));
        self.pics_version += 1;
        self.changed = true;
    }

    /// Forgets everything found, for a new game.
    pub fn reset(&mut self) {
        self.found.iter_mut().for_each(|f| *f = false);
        self.taken.iter_mut().for_each(|t| *t = false);
        self.pics.iter_mut().for_each(|p| *p = None);
        self.pics_version += 1;
        self.met.iter_mut().for_each(|m| *m = false);
        self.errands.iter_mut().for_each(|e| *e = Errand::New);
        self.carrying = None;
        self.riding = None;
        self.chore = None;
        self.lair = None;
        self.offered = None;
        self.hinted = None;
        self.flying.reset();
        self.stories.reset();
        self.kin.reset();
        self.happen.current = 0;
        self.happen.seen_now = false;
        self.happen.seen.iter_mut().for_each(|s| *s = false);
        self.favored.iter_mut().for_each(|f| *f = false);
        self.kin_traded_find.iter_mut().for_each(|t| *t = None);
        self.other.clear();
        self.changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zawnabout_sim::Sim;

    static TEST: PlayDef = PlayDef {
        lairs: &[LairDef {
            id: "den",
            name: "Den",
            at: [400.0, 0.0, 400.0],
            r: 20.0,
        }],
        sets: &[SetDef {
            id: "cups",
            name: "Cups",
            done: "All the cups!",
            effects: &[],
        }],
        finds: &[
            FindDef {
                id: "cup-a",
                kind: "goblet",
                name: "Cup A",
                story: "A cup.",
                x: 0.0,
                y: None,
                z: 0.0,
                set: Some("cups"),
                hint: "Somewhere.",
                gold: 5,
                night: false,
                chimes: false,
            },
            FindDef {
                id: "pile",
                kind: "coins",
                name: "",
                story: "",
                x: 100.0,
                y: None,
                z: 0.0,
                set: None,
                hint: "",
                gold: 20,
                night: false,
                chimes: false,
            },
        ],
        postcards: &[],
        locals: &[],
        stunts: &[],
        courses: &[],
        races: &[],
        stories: &[],
        kin: &[],
        happenings: &[],
        favors: &[],
    };

    /// Everything set out in the file rests somewhere a dragon can reach: finds and lost
    /// things out of the water, and townsfolk on their feet in the open, not on a roof,
    /// in the sea or inside a house.
    #[test]
    fn everything_rests_somewhere_reachable() {
        let sim = Sim::new(1);
        let w = &sim.world;
        let play = Play::new(w);
        let mut bad = Vec::new();
        for (f, s) in PLAY.finds.iter().zip(&play.spots) {
            if w.afloat(s.pos.x, s.pos.z, s.pos.y) {
                bad.push(format!("find {} is in the water", f.id));
            }
        }
        let standing = |id: &str, p: Vec3, y: Option<f32>, bad: &mut Vec<String>| {
            if w.afloat(p.x, p.z, p.y) {
                bad.push(format!("{id} stands in the water"));
            }
            // Piers, decks and paving are fine: a roof stands well up off the ground.
            let up = p.y - w.terrain.height(p.x, p.z).max(0.0);
            if y.is_none() && up > 3.5 && w.in_building(p - Vec3::Y * 0.5) {
                bad.push(format!("{id} stands on a roof, {up:.1} m up at {p}"));
            }
            if w.in_building(p + Vec3::Y) {
                bad.push(format!("{id} stands inside a building"));
            }
        };
        for (l, p) in PLAY.locals.iter().zip(&play.folk) {
            standing(l.id, *p, l.y, &mut bad);
        }
        for (l, item) in PLAY.locals.iter().zip(&play.items) {
            if let Some(p) = item
                && w.afloat(p.x, p.z, p.y)
            {
                bad.push(format!("{}'s lost thing is in the water", l.id));
            }
        }
        let groups = PLAY
            .sets
            .iter()
            .map(|s| s.effects)
            .zip(&play.effect_at)
            .chain(
                PLAY.happenings
                    .iter()
                    .map(|h| h.effects)
                    .zip(&play.happen_at),
            )
            .chain(PLAY.favors.iter().map(|f| f.effects).zip(&play.favor_at));
        for (effects, ends) in groups {
            for (e, (a, _)) in effects.iter().zip(ends) {
                if e.kind == "folk" {
                    let foot = *a - Vec3::Y * e.a[1];
                    standing(e.name, foot, None, &mut bad);
                }
            }
        }
        assert!(
            bad.is_empty(),
            "{}",
            bad.join(
                "
"
            )
        );
    }

    /// Finds are picked up on the ground close by, or hovering right beside one, never
    /// flying past, and only once.
    #[test]
    fn picks_up_on_the_ground_only() {
        let mut sim = Sim::new(1);
        let id = sim.spawn_dragon(0.0, 0.0, 0.0);
        let mut play = Play::with(&TEST, &sim.world);
        let mut me = sim.dragon(id).expect("dragon").clone();
        me.pos = play.spots[0].pos + Vec3::new(1.0, 0.5, 0.0);
        me.mode = Mode::Air;
        me.vel = Vec3::new(30.0, 0.0, 0.0);
        assert!(play.tick(&me).is_empty());
        me.vel = Vec3::ZERO;
        me.mode = Mode::Ground;
        me.pos.x += REACH + 1.0;
        assert!(play.tick(&me).is_empty());
        me.pos.x -= REACH;
        // The only cup, so the set is done too.
        assert_eq!(play.tick(&me), vec![Event::Found(0), Event::SetDone(0)]);
        assert!(play.tick(&me).is_empty());
        assert_eq!(play.gold(), 5);
        assert_eq!(play.sets()[0].have(), 1);
        sim.dragons.clear();
    }

    /// The last piece of a set says so, and the set's effects come out.
    #[test]
    fn finished_sets_bring_effects() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let s = PLAY
            .sets
            .iter()
            .position(|s| s.id == "moon-chess")
            .expect("set");
        // The happening on now has its own effects, there from the start.
        let base = play.effects().count();
        let pieces: Vec<usize> = (0..PLAY.finds.len())
            .filter(|i| PLAY.finds[*i].set == Some("moon-chess"))
            .collect();
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), Vec3::ZERO, 0.0);
        me.mode = Mode::Ground;
        for (k, i) in pieces.iter().enumerate() {
            me.pos = play.spots[*i].pos;
            let events = play.tick(&me);
            assert!(events.contains(&Event::Found(*i)));
            assert_eq!(events.contains(&Event::SetDone(s)), k + 1 == pieces.len());
        }
        assert_eq!(play.effects().count(), base + PLAY.sets[s].effects.len());
        // Everything stands on the ground, not under it or up in the air.
        for (e, a, _) in play.effects() {
            let ground = sim.world.support_still(a.x, a.z, a.y + 1.0);
            assert!((a.y - e.a[1] - ground).abs() < 0.5, "{}", e.name);
        }
    }

    /// A lair is offered once a visit, and not when it is already home.
    #[test]
    fn lair_offered_once_a_visit() {
        let sim = Sim::new(1);
        let mut play = Play::with(&TEST, &sim.world);
        let mut me = Dragon::new(
            zawnabout_sim::EntityId(9),
            Vec3::new(405.0, 0.0, 400.0),
            0.0,
        );
        me.mode = Mode::Ground;
        assert_eq!(play.tick(&me), vec![Event::Lair(0)]);
        assert!(play.tick(&me).is_empty());
        me.pos.x = 500.0;
        play.tick(&me);
        me.pos.x = 405.0;
        assert_eq!(play.tick(&me), vec![Event::Lair(0)]);
        play.claim(0);
        me.pos.x = 500.0;
        play.tick(&me);
        me.pos.x = 405.0;
        assert!(play.tick(&me).is_empty());
    }

    /// Fern Hollow is offered inside, under the rock, never on the grass by its mouth.
    #[test]
    fn lair_only_inside() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let fern = PLAY
            .lairs
            .iter()
            .position(|l| l.id == "fern-hollow")
            .expect("fern");
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), play.lairs[fern], 0.0);
        me.mode = Mode::Ground;
        assert!(play.tick(&me).contains(&Event::Lair(fern)));
        let mut play = Play::new(&sim.world);
        me.pos = Vec3::new(-2294.0, 45.0, 1395.2);
        assert!(!play.tick(&me).contains(&Event::Lair(fern)));
        // Landing out on the grass by its mouth points the way in, once a visit.
        let mut play = Play::new(&sim.world);
        me.pos = play.lairs[fern] + Vec3::new(30.0, 0.0, 10.0);
        assert_eq!(play.tick(&me), vec![Event::LairNear(fern)]);
        assert!(play.tick(&me).is_empty());
    }

    /// Every light request names a beacon the world has.
    #[test]
    fn requests_point_at_real_beacons() {
        let sim = Sim::new(1);
        for l in PLAY.locals {
            if let Some(r) = &l.request
                && r.kind == "light"
            {
                assert!(sim.beacon(r.target).is_some(), "{}: {}", l.id, r.target);
            }
        }
    }

    /// A favor from asking to thanks, and a lost thing given back.
    #[test]
    fn favors_from_ask_to_thanks() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let at = |id: &str| PLAY.locals.iter().position(|l| l.id == id).expect(id);
        let (fen, mara) = (at("fen"), at("mara"));
        assert_eq!(play.talk(fen, &|_| false), Talk::Offer);
        play.accept(fen);
        assert_eq!(play.talk(fen, &|_| false), Talk::Reminder);
        let gold = play.gold();
        assert_eq!(play.talk(fen, &|id| id == "home/home-beacon"), Talk::Thanks);
        assert!(play.gold() > gold);
        assert_eq!(play.favor("Home Village"), 1);
        assert!(matches!(play.talk(fen, &|_| true), Talk::Line(_)));

        assert!(matches!(play.talk(mara, &|_| false), Talk::Line(_)));
        let ring = PLAY
            .finds
            .iter()
            .position(|f| f.id == "nook-ring")
            .expect("ring");
        play.found[ring] = true;
        assert!(play.treasures().any(|f| f.id == "nook-ring"));
        assert_eq!(play.talk(mara, &|_| false), Talk::Return);
        play.give_back(mara);
        assert!(!play.treasures().any(|f| f.id == "nook-ring"));
        assert!(play.gifts().any(|g| g.name == "Mara's Seashell"));
    }

    /// A rider climbs on, hops off with thanks at their spot, or goes home if plans
    /// change. A lair said no to is not asked about again.
    #[test]
    fn rides_and_declines() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let pim = PLAY.locals.iter().position(|l| l.id == "pim").expect("pim");
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), play.folk[pim], 0.0);
        me.mode = Mode::Ground;
        assert_eq!(play.talk(pim, &|_| false), Talk::Offer);
        play.accept(pim);
        assert_eq!(play.riding, Some(pim));
        assert!(!play.back_free());
        // Out and straight back home: they hop off, no harm done.
        me.pos.x += 200.0;
        play.tick(&me);
        me.pos = play.folk[pim];
        assert!(play.tick(&me).contains(&Event::HomeAgain(pim)));
        assert_eq!(play.errands[pim], Errand::New);
        play.accept(pim);
        let card = PLAY
            .postcards
            .iter()
            .position(|c| c.id == "summit-snowman")
            .expect("card");
        me.pos = play.cards[card];
        assert!(play.tick(&me).contains(&Event::Arrived(pim)));
        assert_eq!(play.errands[pim], Errand::Done);
        assert!(play.back_free());

        let mut play = Play::new(&sim.world);
        me.pos = play.lairs[0];
        assert_eq!(play.tick(&me), vec![Event::Lair(0)]);
        play.decline(0);
        assert!(!play.tick(&me).contains(&Event::Lair(0)));
        me.pos.x += 500.0;
        play.tick(&me);
        me.pos = play.lairs[0];
        assert!(play.tick(&me).contains(&Event::Lair(0)));
    }

    /// About one townsperson in two asks something small, always the same thing, and a
    /// dropped thing can be found where they say.
    #[test]
    fn chores_are_small_and_steady() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let home = Vec3::new(-1800.0, 2.0, 1600.0);
        let rest = |x: f32, z: f32| Vec3::new(x, 2.0, z);
        let asks = (0..400)
            .filter(|s| play.chore_for(*s, home, &rest).is_some())
            .count();
        assert!((150..250).contains(&asks), "{asks}");
        let (seed, kind) = (0..400)
            .find_map(|s| match play.chore_for(s, home, &rest) {
                Some(k @ ChoreKind::Fetch { .. }) => Some((s, k)),
                _ => None,
            })
            .expect("someone dropped something");
        assert_eq!(play.chore_for(seed, home, &rest), Some(kind));
        let ChoreKind::Fetch { at, .. } = kind else {
            unreachable!()
        };
        play.chore = Some(Chore {
            seed,
            home,
            kind,
            accepted: true,
            aboard: false,
        });
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), at, 0.0);
        me.mode = Mode::Ground;
        assert!(play.tick(&me).contains(&Event::ChoreCarry));
        assert!(!play.arms_free());
        let gold = play.gold();
        play.chore = None;
        assert_eq!(play.gold(), gold);
    }
}
