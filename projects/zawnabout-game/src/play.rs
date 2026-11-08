//! Glue between the game and its gameplay layers (the `zawnabout-play` crate, behind
//! the `play` feature). The game holds an `Option<Play>`: none when gameplay is compiled
//! out or switched off, and then the islands are exactly as they are without it.

use glam::Vec3;
use zawnabout_sim::Dragon;
use zawnabout_sim::world::World;

use glam::Vec4;

use crate::book::{Card, HoardView};

#[cfg(feature = "play")]
pub use zawnabout_play::{Chore, Play};

/// Gameplay compiled out: there is never one.
#[cfg(not(feature = "play"))]
pub enum Play {}

/// Gameplay compiled out: there is never one.
#[cfg(not(feature = "play"))]
#[derive(Clone, Debug)]
pub enum Chore {}

/// What came of finding a dragon: met, with its card, or the shy one flying off.
pub enum Meet {
    Card(Card),
    Flee(String, String),
}

/// An unfound find to draw: where, its model, and whether it is a coin pile.
pub struct Shiny {
    pub id: usize,
    pub pos: Vec3,
    pub kind: &'static str,
}

/// What a tick of gameplay brought: cards to show, postcards wanting their picture, and
/// small lines that show without stopping anything (who or what, and the line). `cheer`
/// says a passenger would whoop at it.
#[derive(Default)]
pub struct News {
    pub cards: Vec<Card>,
    pub snaps: Vec<usize>,
    pub toasts: Vec<(String, String)>,
    pub cheer: bool,
}

/// Postcard pictures for the atlas: their version, and each one's index and RGBA.
pub type Pics = (u32, Vec<(usize, Vec<u8>)>);

/// A named local to draw: where they stand, which way they face, how they look, and
/// whether they have a favor to ask or be thanked for.
pub struct Local {
    pub id: usize,
    pub pos: Vec3,
    pub facing: f32,
    pub look: u32,
    pub ask: bool,
}

/// Who rides on the dragon's back: their look, name and where they would like to go.
pub struct Rider {
    pub look: u32,
    pub name: String,
    pub wants: String,
    /// A little about the place they want to see, and which island it is on.
    pub about: String,
}

/// Where whoever or whatever the dragon carries is going: a passenger's spot, or the
/// owner of a lost thing in its arms. Shown on screen the whole way.
pub struct Goal {
    pub name: String,
    pub at: Vec3,
    /// How near counts as there (m).
    pub r: f32,
}

/// A lost item lying about for a fetch request, or carried.
pub struct Lost {
    pub pos: Vec3,
    pub kind: &'static str,
}

/// How high over its foot a story's lamp burns.
pub const LAMP_FLAME: f32 = 2.2;

/// Something a finished set brings, to draw: `folk` stands at `a`, `bunting` and
/// `lanterns` hang from `a` to `b`. `id` counts every one shown, for talking to folk.
pub struct Effect {
    pub id: usize,
    pub kind: &'static str,
    pub a: Vec3,
    pub b: Vec3,
    pub facing: f32,
    pub look: u32,
}

/// The hoard heaped in its lair, to draw.
pub struct Heap {
    pub at: Vec3,
    pub gold: u32,
    /// Models of the treasures set round it.
    pub kinds: Vec<&'static str>,
}

#[cfg(feature = "play")]
mod on {
    use super::*;
    use crate::book::{
        CardKind, FlyRow, KinAsk, KinRow, Mark, MarkGroup, PersonRow, PlaceRow, SetRow, StoryRow,
    };
    use zawnabout_play::{Chore, ChoreKind, Errand, Talk};

    /// An island's name as people say it.
    fn island_name(id: &str) -> &'static str {
        match id {
            "home" => "Home",
            "crescent" => "The Crescent",
            "twins" => "The Twins",
            "fang" => "Fang",
            "needles" => "The Needles",
            "atoll" => "The Atoll",
            _ => "Out at sea",
        }
    }
    use zawnabout_play::Event;

    pub fn start(world: &World) -> Option<Play> {
        Some(Play::new(world))
    }

    /// What a vehicle is called in a race.
    fn vehicle_name(kind: &str) -> &'static str {
        match kind {
            "train" => "the train",
            "ferry" => "the ferry",
            "seaplane" => "the seaplane",
            "liftcabin" => "the cable car",
            "balloon" => "the balloon",
            _ => "the boat",
        }
    }

    fn capital(s: &str) -> String {
        let mut c = s.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    }

    /// One tick of gameplay at sim `time`: the cards for what happened, postcards to
    /// picture, and lines for stunts, courses and races.
    pub fn tick(play: &mut Play, me: &Dragon, world: &World, time: f32, breath: f32) -> News {
        let def = play.def;
        let mut news = News::default();
        for e in play.story_tick(me, breath) {
            match e {
                Event::Lamp(s, r, _) => {
                    let region = &def.stories[s].regions[r];
                    let lit = play.stories.lit[s][r].iter().filter(|l| **l).count();
                    news.toasts.push((
                        def.stories[s].name.into(),
                        format!(
                            "A lamp is lit! {lit} of {} on {}.",
                            region.lamps.len(),
                            region.name.to_lowercase()
                        ),
                    ));
                }
                Event::RegionLit(s, r) => {
                    news.cheer = true;
                    let (lit, all) = play.lamps_lit(s);
                    news.toasts.push((
                        def.stories[s].name.into(),
                        format!(
                            "The lights are back on {}! {lit} of {all} lamps lit.",
                            def.stories[s].regions[r].name.to_lowercase()
                        ),
                    ));
                }
                Event::StoryDone(s) => {
                    let st = &def.stories[s];
                    news.cards.push(Card {
                        title: st.name.into(),
                        body: st.done.into(),
                        foot: format!("+{} gold, and {} for your hoard", st.gold, st.prize.name),
                        kind: CardKind::Find,
                    });
                }
                _ => {}
            }
        }
        for e in play.fly(me, world, time) {
            let toast = match e {
                Event::Stunt(i) => {
                    news.cheer = true;
                    let s = &def.stunts[i];
                    (format!("New stunt: {}", s.name), s.line.to_string())
                }
                Event::Gate(c, _) => {
                    let course = &def.courses[c];
                    let (n, all) = (play.gates_passed(c), course.gates.len());
                    let text = if n == 1 {
                        format!("{} That's 1 of {all}.", course.line)
                    } else {
                        format!("{n} of {all}. Keep going!")
                    };
                    (course.name.to_string(), text)
                }
                Event::Course(c) => {
                    news.cheer = true;
                    let course = &def.courses[c];
                    (
                        course.name.to_string(),
                        format!(
                            "All {}! {} for your hoard, and {} gold.",
                            course.gates.len(),
                            course.prize.name,
                            course.gold
                        ),
                    )
                }
                Event::RaceOn(i) => {
                    let r = &def.races[i];
                    (
                        r.name.to_string(),
                        format!(
                            "{} is off to {}! Land there first to win.",
                            capital(vehicle_name(r.vehicle)),
                            r.to
                        ),
                    )
                }
                Event::RaceWon(i, first) => {
                    news.cheer = true;
                    let r = &def.races[i];
                    let text = if first {
                        format!(
                            "You beat {} to {}! {} for your hoard, and {} gold.",
                            vehicle_name(r.vehicle),
                            r.to,
                            r.prize.name,
                            r.gold
                        )
                    } else {
                        format!("You beat {} again!", vehicle_name(r.vehicle))
                    };
                    (r.name.to_string(), text)
                }
                Event::RaceLost(i) => {
                    let r = &def.races[i];
                    (
                        r.name.to_string(),
                        format!(
                            "{} got there first. What a race, though!",
                            capital(vehicle_name(r.vehicle))
                        ),
                    )
                }
                _ => continue,
            };
            news.toasts.push(toast);
        }
        for e in play.tick(me) {
            let card = match e {
                Event::Postcard(i) => {
                    news.snaps.push(i);
                    continue;
                }
                Event::SetDone(s) => {
                    let set = &def.sets[s];
                    Card {
                        title: set.name.into(),
                        body: set.done.into(),
                        foot: "Every piece found!".into(),
                        kind: CardKind::News,
                    }
                }
                Event::Happening(h) => {
                    let h = &def.happenings[h];
                    news.cheer = true;
                    news.toasts.push((h.name.into(), h.seen.into()));
                    continue;
                }
                Event::Favor(f) => {
                    let f = &def.favors[f];
                    Card {
                        title: format!("{} thanks you!", f.town),
                        body: f.done.into(),
                        foot: "Go and see!".into(),
                        kind: CardKind::News,
                    }
                }
                Event::Lamp(..)
                | Event::RegionLit(..)
                | Event::StoryDone(_)
                | Event::Stunt(_)
                | Event::Gate(..)
                | Event::Course(_)
                | Event::RaceOn(_)
                | Event::RaceWon(..)
                | Event::RaceLost(_) => continue,
                Event::Arrived(i) => {
                    let l = &def.locals[i];
                    let r = l.request.as_ref().expect("a ride request");
                    let mut foot = format!("+{} gold", r.gold);
                    if let Some(g) = &r.gift {
                        foot += &format!(", and {} for your hoard", g.name);
                    }
                    Card {
                        title: l.name.into(),
                        body: r.thanks.into(),
                        foot,
                        kind: CardKind::Say,
                    }
                }
                Event::HomeAgain(i) => {
                    let l = &def.locals[i];
                    Card {
                        title: l.name.into(),
                        body: "Home again? That's all right. It was a lovely flight anyway. Ask me again some time!".into(),
                        foot: l.town.into(),
                        kind: CardKind::Say,
                    }
                }
                Event::ChoreCarry => {
                    let kind = match play.chore.as_ref().map(|c| c.kind) {
                        Some(ChoreKind::Fetch { kind, .. }) => kind,
                        _ => "thing",
                    };
                    Card {
                        title: format!("Found the {}!", thing_name(kind)),
                        body: "Take it back to whoever dropped it. They're not far.".into(),
                        foot: String::new(),
                        kind: CardKind::Carry,
                    }
                }
                Event::ChoreArrived => Card {
                    title: "Here we are!".into(),
                    body: "Thank you, dragon! What a ride. I'll remember that one for ever.".into(),
                    foot: String::new(),
                    kind: CardKind::Say,
                },
                Event::ChoreDelivered => Card {
                    title: "Delivered!".into(),
                    body: "Somebody here was waiting for that, and now they're beaming. Special delivery, by dragon.".into(),
                    foot: String::new(),
                    kind: CardKind::Say,
                },
                Event::Carry(i) => {
                    let l = &def.locals[i];
                    let item = l.request.as_ref().and_then(|r| r.item.as_ref());
                    Card {
                        title: item.map_or("Found it!", |it| it.name).into(),
                        body: format!("Take it back to {}. They'll be so pleased.", l.name),
                        foot: l.town.into(),
                        kind: CardKind::Carry,
                    }
                }
                Event::Found(i) => {
                    let f = &def.finds[i];
                    let foot = f
                        .set
                        .and_then(|s| play.sets().into_iter().find(|v| v.def.id == s))
                        .map(|v| format!("{}: {} of {}", v.def.name, v.have(), v.pieces.len()))
                        .unwrap_or_default();
                    if f.kind == "coins" {
                        Card {
                            title: format!("{} gold coins!", f.gold),
                            body: "A little pile of coins, glinting in the grass. Every bit helps a hoard grow.".into(),
                            foot,
                            kind: CardKind::Find,
                        }
                    } else {
                        Card {
                            title: f.name.into(),
                            body: f.story.into(),
                            foot,
                            kind: CardKind::Find,
                        }
                    }
                }
                Event::Lair(i) => {
                    let l = &def.lairs[i];
                    let body = if play.lair.is_some() {
                        "A snug, hidden cave. Move your hoard here? Everything comes along."
                    } else {
                        "A snug, hidden cave, just right for a dragon. Make it your lair? Everything you find will gather here."
                    };
                    Card {
                        title: l.name.into(),
                        body: body.into(),
                        foot: String::new(),
                        kind: CardKind::Lair(i),
                    }
                }
            };
            news.cards.push(card);
        }
        news
    }

    /// Postcard `id`'s number.
    pub fn card_index(play: &Play, id: &str) -> Option<usize> {
        play.def.postcards.iter().position(|c| c.id == id)
    }

    /// Whether postcard `i` looks best from below, as a falls does from its pool.
    pub fn card_low(play: &Play, i: usize) -> bool {
        play.def.postcards.get(i).is_some_and(|c| c.aim < 0.0)
    }

    /// Where postcard `i`'s place is, and how big round.
    pub fn card_spot(play: &Play, i: usize) -> Option<(Vec3, f32)> {
        Some((*play.cards.get(i)?, play.def.postcards[i].r))
    }

    /// The card for postcard `i`, once its picture is in.
    pub fn postcard_card(play: &Play, i: usize) -> Card {
        let c = &play.def.postcards[i];
        let have = play.taken.iter().filter(|t| **t).count();
        Card {
            title: c.name.into(),
            body: c.line.into(),
            foot: format!("Postcards: {have}"),
            kind: CardKind::Postcard(play.pics[i].is_some().then_some(i)),
        }
    }

    /// Paints a frame into postcard `i`'s picture.
    pub fn set_pic(play: &mut Play, i: usize, rgba: &[u8], w: usize, h: usize) {
        let pic = zawnabout_play::paint::paint(rgba, w, h);
        play.set_pic(i, &pic);
    }

    /// Pictures to put in the atlas, when they changed since `version`: the new
    /// version, and each postcard's picture as RGBA.
    pub fn pics(play: &Play, version: u32) -> Option<Pics> {
        if play.pics_version == version {
            return None;
        }
        let pics = play
            .pics
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Some((i, zawnabout_play::paint::decode(p.as_ref()?)?)))
            .collect();
        Some((play.pics_version, pics))
    }

    pub fn claim(play: &mut Play, i: usize) {
        play.claim(i);
    }

    /// The named local within `reach` of `at`, nearest first.
    pub fn nearest_local(play: &Play, at: Vec3, reach: f32) -> Option<(usize, Vec3)> {
        play.folk
            .iter()
            .enumerate()
            .map(|(i, p)| (i, *p, (p.x - at.x).hypot(p.z - at.z)))
            .filter(|(_, p, d)| *d < reach && (p.y - at.y).abs() < 5.0)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(i, p, _)| (i, p))
    }

    /// Whether local `i` has something to ask or be thanked for, which their bubble shows.
    /// The story local `i` gives, if any.
    fn story_of(play: &Play, i: usize) -> Option<usize> {
        let id = play.def.locals[i].id;
        play.def.stories.iter().position(|s| s.giver == id)
    }

    pub fn asks(play: &Play, i: usize, lit: &dyn Fn(&str) -> bool) -> bool {
        if let Some(s) = story_of(play, i) {
            return play.stories.active != Some(s) && !play.stories.done[s];
        }
        let Some(r) = &play.def.locals[i].request else {
            return false;
        };
        match play.errands[i] {
            zawnabout_play::Errand::New if r.kind == "return" => play
                .def
                .finds
                .iter()
                .position(|f| f.id == r.target)
                .is_some_and(|f| play.found[f]),
            zawnabout_play::Errand::New => true,
            zawnabout_play::Errand::Accepted => play.ready(i, lit),
            zawnabout_play::Errand::Done => false,
        }
    }

    /// Talks to local `i`: the card for what they say.
    pub fn talk_card(play: &mut Play, i: usize, lit: &dyn Fn(&str) -> bool) -> Card {
        let def = play.def;
        let l = &def.locals[i];
        let r = l.request.as_ref();
        // A rider waits for a free back.
        if r.is_some_and(|r| r.kind == "ride")
            && play.errands[i] == Errand::New
            && !play.back_free()
        {
            play.met[i] = true;
            return Card {
                title: l.name.into(),
                body: "You've got someone riding already! Ask me again when your back is free."
                    .into(),
                foot: l.town.into(),
                kind: CardKind::Say,
            };
        }
        // A storyteller asks for help, or says how it is going.
        if let Some(s) = story_of(play, i) {
            play.met[i] = true;
            let st = &def.stories[s];
            let (lit, all) = play.lamps_lit(s);
            let (body, kind) = if play.stories.active == Some(s) {
                (
                    format!("{lit} of {all} lamps lit so far. Thank you, dragon. No hurry at all."),
                    CardKind::Say,
                )
            } else if !play.stories.done[s] {
                (st.ask.to_string(), CardKind::Story(s))
            } else {
                let t = play.talk(i, &lit_never);
                let Talk::Line(t) = t else { unreachable!() };
                (t.to_string(), CardKind::Say)
            };
            return Card {
                title: l.name.into(),
                body,
                foot: l.town.into(),
                kind,
            };
        }
        let said = play.talk(i, lit);
        let (body, foot, kind) = match said {
            Talk::Line(t) => (t.to_string(), l.town.to_string(), CardKind::Say),
            Talk::Offer => (
                r.map_or("", |r| r.ask).into(),
                l.town.into(),
                CardKind::Offer(i),
            ),
            Talk::Return => (
                r.map_or("", |r| r.ask).into(),
                l.town.into(),
                CardKind::Return(i),
            ),
            Talk::Reminder => (
                r.map_or("", |r| r.hint).into(),
                "Still to do".into(),
                CardKind::Say,
            ),
            Talk::Thanks => {
                let r = r.expect("thanks come with a request");
                let mut foot = format!("+{} gold", r.gold);
                if let Some(g) = &r.gift {
                    foot += &format!(", and {} for your hoard", g.name);
                }
                (r.thanks.into(), foot, CardKind::Say)
            }
        };
        Card {
            title: l.name.into(),
            body,
            foot,
            kind,
        }
    }

    fn lit_never(_: &str) -> bool {
        false
    }

    pub fn accept(play: &mut Play, i: usize) {
        play.accept(i);
    }

    pub fn hop_off(play: &mut Play) {
        play.hop_off();
    }

    /// Starts story `s`, or sets aside the one under way.
    pub fn begin(play: &mut Play, s: usize) {
        play.begin(s);
    }

    pub fn set_aside(play: &mut Play) {
        play.set_aside();
    }

    /// Where the lights are out: each dark region's middle (x, z) and reach.
    pub fn dark(play: &Play) -> Vec<Vec4> {
        play.dark()
            .into_iter()
            .map(|(c, r)| Vec4::new(c.x, c.y, r, 0.0))
            .collect()
    }

    /// The story's lamps, each one's foot and whether it burns.
    pub fn lamps(play: &Play) -> Vec<(Vec3, bool)> {
        play.lamps()
    }

    /// While a story runs: whether `at` is still dark (true) or lit again (false).
    pub fn dark_here(play: &Play, at: Vec3) -> Option<bool> {
        let s = play.stories.active?;
        let r = play.def.stories[s]
            .regions
            .iter()
            .position(|g| (g.x - at.x).hypot(g.z - at.z) < g.r)?;
        Some(!play.region_lit(s, r))
    }

    /// Lamp `k` of the story under way, counted across its regions, for scripts.
    pub fn lamp_at(play: &Play, k: usize) -> Option<Vec3> {
        play.lamps().get(k).map(|(at, _)| *at)
    }

    /// Whether story `id` is told, none when there is no such story.
    pub fn story_done(play: &Play, id: &str) -> Option<bool> {
        let s = play.def.stories.iter().position(|s| s.id == id)?;
        Some(play.stories.done[s])
    }

    pub fn story_index(play: &Play, id: &str) -> Option<usize> {
        play.def.stories.iter().position(|s| s.id == id)
    }

    pub fn decline(play: &mut Play, i: usize) {
        play.decline(i);
    }

    /// A townsperson's thing, as they'd say it.
    fn thing_name(kind: &str) -> &'static str {
        match kind {
            "hat" => "hat",
            "basket" => "basket",
            "kite" => "kite",
            "map" => "map",
            "book" => "book",
            "chest" => "box",
            _ => "thing",
        }
    }

    /// What a townsperson would send, as they'd say it.
    fn parcel_name(kind: &str) -> &'static str {
        match kind {
            "map" => "letter",
            "basket" => "basket of buns",
            "book" => "book",
            _ => "parcel",
        }
    }

    /// Talks to townsperson `seed` standing at `home` about their small ask: a card,
    /// or none when they have nothing to ask. `rest` finds the ground under a point.
    pub fn chore_card(
        play: &mut Play,
        seed: u32,
        home: Vec3,
        rest: &dyn Fn(f32, f32) -> Vec3,
        name: &str,
    ) -> Option<(Card, Option<Chore>)> {
        if let Some(ch) = play.chore.clone() {
            if ch.seed != seed {
                return None;
            }
            let card = |body: String, kind| Card {
                title: name.into(),
                body,
                foot: String::new(),
                kind,
            };
            return Some(match (ch.kind, ch.accepted, ch.aboard) {
                (ChoreKind::Fetch { kind, .. }, true, true) => {
                    play.chore = None;
                    (
                        card(
                            format!(
                                "My {}! Oh, thank you. You're a very kind dragon.",
                                thing_name(kind)
                            ),
                            CardKind::Say,
                        ),
                        None,
                    )
                }
                (ChoreKind::Fetch { kind, .. }, true, false) => (
                    card(
                        format!(
                            "My {} should be just around here somewhere. Look for a glint!",
                            thing_name(kind)
                        ),
                        CardKind::Say,
                    ),
                    None,
                ),
                (ChoreKind::Deliver { card: c, kind }, true, _) => (
                    card(
                        format!(
                            "My {} goes to {}. Somebody there is ever so excited about it.",
                            parcel_name(kind),
                            play.def.postcards[c].name
                        ),
                        CardKind::Say,
                    ),
                    None,
                ),
                (ChoreKind::Ride { .. }, true, true) => (
                    card(
                        "Are we there yet? Only joking. Take your time, dragon!".into(),
                        CardKind::Say,
                    ),
                    None,
                ),
                _ => (card(chore_ask(play, ch.kind), CardKind::Chore), Some(ch)),
            });
        }
        let kind = play.chore_for(seed, home, rest)?;
        if matches!(kind, ChoreKind::Ride { .. }) && !play.back_free() {
            return None;
        }
        if matches!(kind, ChoreKind::Deliver { .. }) && !play.arms_free() {
            return None;
        }
        let ch = Chore {
            seed,
            home,
            kind,
            accepted: false,
            aboard: false,
        };
        Some((
            Card {
                title: name.into(),
                body: chore_ask(play, kind),
                foot: String::new(),
                kind: CardKind::Chore,
            },
            Some(ch),
        ))
    }

    fn chore_ask(play: &Play, kind: ChoreKind) -> String {
        match kind {
            ChoreKind::Fetch { kind, .. } => format!(
                "Oh dear, I've dropped my {} somewhere around here. Could you have a look?",
                thing_name(kind)
            ),
            ChoreKind::Ride { card } => format!(
                "I've always wanted to see {}. Could you fly me there?",
                play.def.postcards[card].name
            ),
            ChoreKind::Deliver { card, kind } => format!(
                "Could you take this {} to {} for me? Somebody there is waiting for it. Just land there and they'll find you.",
                parcel_name(kind),
                play.def.postcards[card].name
            ),
        }
    }

    /// Takes on a townsperson's small ask. A rider climbs aboard.
    pub fn take_chore(play: &mut Play, mut ch: Chore) {
        ch.accepted = true;
        ch.aboard = matches!(ch.kind, ChoreKind::Ride { .. } | ChoreKind::Deliver { .. });
        play.chore = Some(ch);
    }

    /// Whether townsperson `seed` has something to ask or be thanked for.
    pub fn folk_asks(play: &Play, seed: u32, home: Vec3) -> bool {
        match &play.chore {
            Some(ch) => ch.seed == seed && (!ch.accepted || ch.aboard),
            None => play
                .chore_for(seed, home, &|x, z| Vec3::new(x, home.y, z))
                .is_some(),
        }
    }

    /// Who rides on the dragon's back.
    pub fn rider(play: &Play) -> Option<Rider> {
        let about = |c: Option<usize>| {
            c.map_or(String::new(), |c| {
                let p = &play.def.postcards[c];
                let island = match p.island {
                    "home" => "the home island".to_string(),
                    id => island_name(id).replacen("The ", "the ", 1),
                };
                format!("{} It's on {island}.", p.line)
            })
        };
        if let Some(i) = play.riding {
            let l = &play.def.locals[i];
            let target = l.request.as_ref().map_or("", |r| r.target);
            let c = play.def.postcards.iter().position(|c| c.id == target);
            return Some(Rider {
                look: l.look,
                name: l.name.into(),
                wants: c
                    .map_or("somewhere new", |c| play.def.postcards[c].name)
                    .into(),
                about: about(c),
            });
        }
        let ch = play.chore.as_ref()?;
        let ChoreKind::Ride { card } = ch.kind else {
            return None;
        };
        ch.aboard.then(|| Rider {
            look: ch.seed / 31,
            name: crate::talk::name(ch.seed).into(),
            wants: play.def.postcards[card].name.into(),
            about: about(Some(card)),
        })
    }

    /// Where what the dragon carries is going: a passenger, a parcel, or a lost thing on
    /// its way back to its owner.
    pub fn goal(play: &Play) -> Option<Goal> {
        let def = play.def;
        let card = |c: usize| Goal {
            name: def.postcards[c].name.into(),
            at: play.cards[c],
            r: def.postcards[c].r,
        };
        if let Some(i) = play.riding {
            let target = def.locals[i].request.as_ref().map_or("", |r| r.target);
            return def.postcards.iter().position(|c| c.id == target).map(card);
        }
        if let Some(i) = play.carrying {
            return Some(Goal {
                name: def.locals[i].name.into(),
                at: play.folk[i],
                r: 8.0,
            });
        }
        let ch = play.chore.as_ref().filter(|c| c.accepted && c.aboard)?;
        Some(match ch.kind {
            ChoreKind::Ride { card: c } | ChoreKind::Deliver { card: c, .. } => card(c),
            ChoreKind::Fetch { .. } => Goal {
                name: crate::talk::name(ch.seed).into(),
                at: ch.home,
                r: 8.0,
            },
        })
    }

    /// The townsperson riding along, so the town does not also draw them walking.
    pub fn riding_folk(play: &Play) -> Option<u32> {
        let ch = play.chore.as_ref()?;
        (ch.aboard && matches!(ch.kind, ChoreKind::Ride { .. })).then_some(ch.seed)
    }

    /// Gives local `i` back their lost thing: the thanks card.
    pub fn give_back(play: &mut Play, i: usize) -> Card {
        play.give_back(i);
        let l = &play.def.locals[i];
        let r = l.request.as_ref().expect("a return request");
        let mut foot = format!("+{} gold", r.gold);
        if let Some(g) = &r.gift {
            foot += &format!(", and {} for your hoard", g.name);
        }
        Card {
            title: l.name.into(),
            body: r.thanks.into(),
            foot,
            kind: CardKind::Say,
        }
    }

    /// Named locals to draw, and lost items lying about or being carried.
    pub fn people(
        play: &Play,
        lit: &dyn Fn(&str) -> bool,
        out: &mut Vec<Local>,
        lost: &mut Vec<Lost>,
    ) -> Option<&'static str> {
        for (i, (l, p)) in play.def.locals.iter().zip(&play.folk).enumerate() {
            if play.riding == Some(i) {
                continue;
            }
            out.push(Local {
                id: i,
                pos: *p,
                facing: l.facing.to_radians(),
                look: l.look,
                ask: asks(play, i, lit),
            });
            if let (Some(at), zawnabout_play::Errand::Accepted) = (play.items[i], play.errands[i])
                && play.carrying != Some(i)
                && let Some(item) = l.request.as_ref().and_then(|r| r.item.as_ref())
            {
                lost.push(Lost {
                    pos: at,
                    kind: item.kind,
                });
            }
        }
        if let Some(Chore {
            kind: ChoreKind::Fetch { kind, at },
            accepted: true,
            aboard,
            ..
        }) = play.chore
        {
            if aboard {
                return Some(kind);
            }
            lost.push(Lost { pos: at, kind });
        }
        if let Some(Chore {
            kind: ChoreKind::Deliver { kind, .. },
            accepted: true,
            ..
        }) = play.chore
        {
            return Some(kind);
        }
        let c = play.carrying?;
        Some(play.def.locals[c].request.as_ref()?.item.as_ref()?.kind)
    }

    /// A place with a treasure still to find, by a postcard's name, picked by `seed`.
    pub fn rumor(play: &Play, seed: u32) -> Option<String> {
        let left: Vec<usize> = (0..play.def.finds.len())
            .filter(|i| !play.found[*i] && play.def.finds[*i].kind != "coins")
            .collect();
        let f = *left.get(seed as usize % left.len().max(1))?;
        let at = play.spots[f].pos;
        play.def
            .postcards
            .iter()
            .zip(&play.cards)
            .map(|(c, p)| (c, p.distance(at)))
            .filter(|(_, d)| *d < 900.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c.name.to_string())
    }

    /// While there is no lair yet, a townsperson's tip about one, picked by `seed`.
    pub fn lair_tip(play: &Play, seed: u32) -> Option<String> {
        // Now and then, word of what is going on somewhere.
        if let Some(i) = play.happening()
            && !play.happen.seen_now
            && seed.is_multiple_of(3)
        {
            return Some(play.def.happenings[i].rumor.to_string());
        }
        // Every other tip is word of a dragon not met yet.
        let unmet: Vec<_> = play
            .def
            .kin
            .iter()
            .zip(&play.kin.met)
            .filter(|(_, m)| !**m)
            .map(|(k, _)| k.rumor)
            .collect();
        if !unmet.is_empty() && (seed / 3).is_multiple_of(2) || play.lair.is_some() {
            return unmet
                .get(seed as usize % unmet.len().max(1))
                .map(|r| r.to_string());
        }
        const TIPS: [&str; 4] = [
            "A dragon needs a lair, my gran always said. There's a snug little cave at the foot of the big cliff, west of the big meadow. Fern Hollow, they call it.",
            "Have you got a cave to keep your treasure in? There's a hollow in one of the Needles, they say. Big enough for a dragon.",
            "Up on the volcano there's a warm cave called Ember Hall. Just right for a dragon's hoard, I'd think.",
            "Every dragon in the stories had a cave for its treasure. The journal's map would show you where, I bet.",
        ];
        Some(TIPS[seed as usize % TIPS.len()].to_string())
    }

    /// Something townsfolk say about a course not flown or a race not won, picked by
    /// `seed`.
    pub fn dare(play: &Play, seed: u32) -> Option<String> {
        let f = &play.flying;
        let left: Vec<&'static str> = play
            .def
            .courses
            .iter()
            .zip(&f.courses)
            .filter(|(_, d)| !**d)
            .map(|(c, _)| c.rumor)
            .chain(
                play.def
                    .races
                    .iter()
                    .zip(&f.won)
                    .filter(|(_, w)| !**w)
                    .map(|(r, _)| r.rumor),
            )
            .collect();
        left.get(seed as usize % left.len().max(1))
            .map(|s| s.to_string())
    }

    /// The journal's flying rows: courses heard of or started, races run, then stunts.
    fn flying(play: &Play) -> Vec<FlyRow> {
        let def = play.def;
        let f = &play.flying;
        let mut rows = Vec::new();
        for (c, course) in def.courses.iter().enumerate() {
            let n = play.gates_passed(c);
            if n == 0 {
                continue;
            }
            rows.push(FlyRow {
                name: course.name.into(),
                kind: "Course".into(),
                status: if f.courses[c] {
                    "Flown".into()
                } else {
                    format!("{n} of {}", course.gates.len())
                },
                line: course.line.into(),
                done: f.courses[c],
                mark: Some(format!("course:{}", course.id)),
            });
        }
        for (i, r) in def.races.iter().enumerate() {
            if !f.tried[i] {
                continue;
            }
            rows.push(FlyRow {
                name: r.name.into(),
                kind: "Race".into(),
                status: if f.won[i] { "Won" } else { "Not yet" }.into(),
                line: format!(
                    "Wait at {} until {} sets off, then land at {} first.",
                    r.from,
                    vehicle_name(r.vehicle),
                    r.to
                ),
                done: f.won[i],
                mark: Some("races".into()),
            });
        }
        for s in play.stunts_done() {
            rows.push(FlyRow {
                name: s.name.into(),
                kind: "Stunt".into(),
                status: String::new(),
                line: s.line.into(),
                done: true,
                mark: None,
            });
        }
        rows
    }

    /// A clear way through a stunt's gate (`ID`) or a course's (`ID N`), for scripts.
    pub fn gate_way(play: &Play, world: &World, id: &str, n: usize) -> Option<(Vec3, Vec3)> {
        let def = play.def;
        let gate = def
            .stunts
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.gate.as_ref())
            .or_else(|| def.courses.iter().find(|c| c.id == id)?.gates.get(n))?;
        zawnabout_play::flying::way_through(world, gate)
    }

    /// Whether stunt `id` is done, course `id` flown or race `id` won, for scripts.
    pub fn flown(play: &Play, id: &str) -> Option<bool> {
        let def = play.def;
        let f = &play.flying;
        if let Some(i) = def.stunts.iter().position(|s| s.id == id) {
            return Some(f.stunts[i]);
        }
        if let Some(c) = def.courses.iter().position(|c| c.id == id) {
            return Some(f.courses[c]);
        }
        let i = def.races.iter().position(|r| r.id == id)?;
        Some(f.won[i])
    }

    /// Whether gate `n` of course `id` is passed, for scripts.
    pub fn gate_passed(play: &Play, id: &str, n: usize) -> Option<bool> {
        let c = play.def.courses.iter().position(|c| c.id == id)?;
        play.flying.gates[c].get(n).copied()
    }

    /// Race `id`'s start or finish, for scripts.
    pub fn race_spot(play: &Play, id: &str, end: bool) -> Option<(f32, f32)> {
        let r = play.def.races.iter().find(|r| r.id == id)?;
        let [x, z] = if end { r.to_at } else { r.from_at };
        Some((x, z))
    }

    pub fn racing(play: &Play) -> bool {
        play.flying.race.is_some()
    }

    /// What finished sets bring to the islands, to draw.
    pub fn effects(play: &Play) -> Vec<Effect> {
        play.effects()
            .enumerate()
            .map(|(id, (e, a, b))| Effect {
                id,
                kind: e.kind,
                a,
                b,
                facing: e.facing.to_radians(),
                look: e.look,
            })
            .collect()
    }

    /// The nearest of the folk finished sets bring, within `reach` on the ground.
    pub fn guest_near(play: &Play, at: Vec3, reach: f32) -> Option<(usize, Vec3)> {
        play.effects()
            .enumerate()
            .filter(|(_, (e, _, _))| e.kind == "folk")
            .map(|(k, (_, a, _))| (k, a, (a.x - at.x).hypot(a.z - at.z)))
            .filter(|(_, a, d)| *d < reach && (a.y - at.y).abs() < 6.0)
            .min_by(|x, y| x.2.total_cmp(&y.2))
            .map(|(k, a, _)| (k, a))
    }

    /// What guest `k` says.
    pub fn guest_card(play: &Play, k: usize) -> Option<Card> {
        let (e, _, _) = play.effects().nth(k)?;
        Some(Card {
            title: e.name.into(),
            body: e.line.into(),
            foot: String::new(),
            kind: CardKind::Say,
        })
    }

    /// Whether local `id`'s favor is done, none when there is no such local.
    pub fn helped(play: &Play, id: &str) -> Option<bool> {
        let i = play.def.locals.iter().position(|l| l.id == id)?;
        Some(play.errands[i] == zawnabout_play::Errand::Done)
    }

    /// The postcard spot nearest `at` within `reach`: its name and where it is.
    pub fn spot_near(play: &Play, at: Vec3, reach: f32) -> Option<(&'static str, Vec3)> {
        play.def
            .postcards
            .iter()
            .zip(&play.cards)
            .map(|(c, p)| (c.name, *p, p.distance(at)))
            .filter(|(_, _, d)| *d < reach)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(n, p, _)| (n, p))
    }

    /// Favors done anywhere.
    pub fn kindness(play: &Play) -> usize {
        play.errands
            .iter()
            .filter(|e| **e == zawnabout_play::Errand::Done)
            .count()
    }

    /// A point's x and z, for the map.
    fn flat(p: Vec3) -> glam::Vec2 {
        glam::Vec2::new(p.x, p.z)
    }

    /// Everything the journal's map can show, group by group. `beacon` says where a
    /// beacon's flame is.
    fn marks(play: &Play, beacon: &dyn Fn(&str) -> Option<Vec3>) -> Vec<MarkGroup> {
        let def = play.def;
        let mut out = Vec::new();
        let mark = |at: Vec3, done: bool, label: &str| Mark {
            at: flat(at),
            done,
            label: label.into(),
        };
        // Favors under way: what to go and get, and who it goes back to.
        let mut favors = Vec::new();
        for (i, l) in def.locals.iter().enumerate() {
            let Some(r) = &l.request else { continue };
            let home = play.folk[i];
            match play.errands[i] {
                Errand::Accepted => match r.kind {
                    "fetch" => {
                        if play.carrying == Some(i) {
                            favors.push(mark(home, false, &format!("Back to {}", l.name)));
                        } else if let (Some(at), Some(item)) = (play.items[i], &r.item) {
                            favors.push(mark(at, false, item.name));
                        }
                    }
                    "light" => {
                        if let Some(at) = beacon(r.target) {
                            favors.push(mark(at, false, "The beacon"));
                        }
                    }
                    "postcard" | "ride" => {
                        if let Some(c) = def.postcards.iter().position(|c| c.id == r.target) {
                            if play.taken[c] && r.kind == "postcard" {
                                favors.push(mark(home, false, &format!("Back to {}", l.name)));
                            } else {
                                favors.push(mark(play.cards[c], false, def.postcards[c].name));
                            }
                        }
                    }
                    _ => {}
                },
                Errand::New if r.kind == "return" => {
                    let have = def
                        .finds
                        .iter()
                        .position(|f| f.id == r.target)
                        .is_some_and(|f| play.found[f]);
                    if have {
                        favors.push(mark(home, false, &format!("Back to {}", l.name)));
                    }
                }
                _ => {}
            }
        }
        if let Some(ch) = &play.chore
            && ch.accepted
        {
            let who = crate::talk::name(ch.seed);
            match ch.kind {
                ChoreKind::Fetch { at, kind } if !ch.aboard => {
                    favors.push(mark(at, false, &format!("{who}'s {}", thing_name(kind))))
                }
                ChoreKind::Fetch { .. } => {
                    favors.push(mark(ch.home, false, &format!("Back to {who}")))
                }
                ChoreKind::Ride { card } | ChoreKind::Deliver { card, .. } => {
                    favors.push(mark(play.cards[card], false, def.postcards[card].name))
                }
            }
        }
        out.push(MarkGroup {
            key: "favors".into(),
            name: "Favors under way".into(),
            about: "Where to go for the favors you said yes to, and who to take things back to."
                .into(),
            spots: favors,
        });
        out.push(MarkGroup {
            key: "people".into(),
            name: "Townsfolk with a favor".into(),
            about: "People in town who would love a dragon's help. Grey ones are already helped."
                .into(),
            spots: def
                .locals
                .iter()
                .enumerate()
                .filter(|(_, l)| l.request.is_some())
                .map(|(i, l)| mark(play.folk[i], play.errands[i] == Errand::Done, l.name))
                .collect(),
        });
        out.push(MarkGroup {
            key: "lairs".into(),
            name: "Lairs".into(),
            about: "Snug caves just right for a dragon, each with a short way in. Land inside one to make it home."
                .into(),
            spots: def
                .lairs
                .iter()
                .enumerate()
                .map(|(i, l)| mark(play.lairs[i], play.lair == Some(i), l.name))
                .collect(),
        });
        if let Some(s) = play.stories.active {
            let st = &def.stories[s];
            let mut spots = Vec::new();
            for (r, region) in st.regions.iter().enumerate() {
                for (k, lamp) in region.lamps.iter().enumerate() {
                    spots.push(mark(
                        play.story_lamps[s][r][k],
                        play.stories.lit[s][r][k],
                        lamp.name,
                    ));
                }
            }
            out.push(MarkGroup {
                key: "story".into(),
                name: st.name.into(),
                about: "The old lamps to light with fire, island by island.".into(),
                spots,
            });
        }
        for set in def.sets {
            out.push(MarkGroup {
                key: format!("set:{}", set.id),
                name: set.name.into(),
                about: format!(
                    "Where the pieces of the {} are. Land right beside one to pick it up.",
                    set.name.to_lowercase()
                ),
                spots: def
                    .finds
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.set == Some(set.id))
                    .map(|(i, f)| mark(play.spots[i].pos, play.found[i], f.name))
                    .collect(),
            });
        }
        out.push(MarkGroup {
            key: "lone".into(),
            name: "Treasures".into(),
            about: "Every treasure that isn't part of a set. Land right beside one to pick it up."
                .into(),
            spots: def
                .finds
                .iter()
                .enumerate()
                .filter(|(_, f)| f.set.is_none() && f.kind != "coins")
                .map(|(i, f)| mark(play.spots[i].pos, play.found[i], f.name))
                .collect(),
        });
        out.push(MarkGroup {
            key: "coins".into(),
            name: "Coin piles".into(),
            about: "Little piles of gold coins, glinting in the grass and in caves.".into(),
            spots: def
                .finds
                .iter()
                .enumerate()
                .filter(|(_, f)| f.kind == "coins")
                .map(|(i, _)| mark(play.spots[i].pos, play.found[i], "Coins"))
                .collect(),
        });
        out.push(MarkGroup {
            key: "postcards".into(),
            name: "Postcard spots".into(),
            about: "Beautiful spots. Land anywhere near one for a postcard of it.".into(),
            spots: def
                .postcards
                .iter()
                .enumerate()
                .map(|(i, c)| mark(play.cards[i], play.taken[i], c.name))
                .collect(),
        });
        for (c, course) in def.courses.iter().enumerate() {
            out.push(MarkGroup {
                key: format!("course:{}", course.id),
                name: course.name.into(),
                about: course.line.into(),
                spots: course
                    .gates
                    .iter()
                    .enumerate()
                    .map(|(g, gate)| {
                        let at = if gate.r > 0.0 {
                            glam::Vec2::from(gate.a)
                        } else {
                            (glam::Vec2::from(gate.a) + glam::Vec2::from(gate.b)) * 0.5
                        };
                        Mark {
                            at,
                            done: play.flying.gates[c][g],
                            label: String::new(),
                        }
                    })
                    .collect(),
            });
        }
        out.push(MarkGroup {
            key: "races".into(),
            name: "Races".into(),
            about: "Where each race starts. Wait on the ground there until the vehicle sets off."
                .into(),
            spots: def
                .races
                .iter()
                .enumerate()
                .map(|(i, r)| Mark {
                    at: glam::Vec2::from(r.from_at),
                    done: play.flying.won[i],
                    label: r.name.into(),
                })
                .collect(),
        });
        if let Some(i) = play.happening() {
            let h = &def.happenings[i];
            out.push(MarkGroup {
                key: "happening".into(),
                name: "Going on now".into(),
                about: "Something is happening here. Go and see! Once you have, another starts somewhere else.".into(),
                spots: vec![Mark {
                    at: glam::Vec2::new(h.x, h.z),
                    done: play.happen.seen_now,
                    label: format!("{}, {}", h.name, h.place),
                }],
            });
        }
        out.push(MarkGroup {
            key: "dragons".into(),
            name: "Dragon friends".into(),
            about: "Where the dragons you have met like to sit.".into(),
            spots: def
                .kin
                .iter()
                .enumerate()
                .filter(|(i, _)| play.kin.met[*i])
                .map(|(i, k)| mark(play.kin_at[i][0], false, k.name))
                .collect(),
        });
        out
    }

    pub fn view(play: &Play, beacon: &dyn Fn(&str) -> Option<Vec3>) -> HoardView {
        HoardView {
            marks: marks(play, beacon),
            postcards: play
                .def
                .postcards
                .iter()
                .enumerate()
                .filter(|(i, _)| play.taken[*i])
                .map(|(i, c)| PlaceRow {
                    name: c.name.into(),
                    line: c.line.into(),
                    island: island_name(c.island).into(),
                    pic: play.pics[i].is_some().then_some(i),
                })
                .collect(),
            gold: play.gold(),
            lair: play.lair.map(|l| play.def.lairs[l].name.to_string()),
            treasures: play
                .treasures()
                .filter(|f| f.set.is_none())
                .map(|f| (f.name.to_string(), f.story.to_string(), f.kind.to_string()))
                .chain(
                    play.gifts()
                        .map(|g| (g.name.to_string(), g.story.to_string(), g.kind.to_string())),
                )
                .collect(),
            people: play
                .def
                .locals
                .iter()
                .enumerate()
                .filter(|(i, _)| play.met[*i])
                .map(|(i, l)| PersonRow {
                    name: l.name.into(),
                    town: l.town.into(),
                    status: match (play.errands[i], &l.request) {
                        (_, None) => "A friend".into(),
                        (Errand::Done, _) => "Helped".into(),
                        (Errand::Accepted, Some(r)) => r.hint.into(),
                        (Errand::New, Some(r)) if r.kind == "return" => {
                            "Lost something. Maybe it will turn up.".into()
                        }
                        (Errand::New, Some(_)) => "Has something to ask you".into(),
                    },
                    helped: play.errands[i] == Errand::Done && l.request.is_some(),
                    mark: match (play.errands[i], &l.request) {
                        (Errand::Accepted, Some(_)) => Some("favors".into()),
                        (Errand::New, Some(r)) if r.kind == "return" => Some("favors".into()),
                        _ => None,
                    },
                })
                .collect(),
            towns: {
                let mut towns: Vec<(String, usize)> = Vec::new();
                for l in play.def.locals {
                    if towns.iter().all(|(t, _)| t != l.town) && play.favor(l.town) > 0 {
                        towns.push((l.town.into(), play.favor(l.town)));
                    }
                }
                towns
            },
            flying: flying(play),
            kin: kin_rows(play),
            stories: play
                .def
                .stories
                .iter()
                .enumerate()
                .map(|(s, st)| {
                    let (lit, all) = play.lamps_lit(s);
                    let active = play.stories.active == Some(s);
                    StoryRow {
                        name: st.name.into(),
                        status: if active {
                            format!("{lit} of {all} lamps")
                        } else if play.stories.done[s] {
                            "Told".into()
                        } else if lit > 0 {
                            "Set aside".into()
                        } else {
                            "Not started".into()
                        },
                        about: st.about.into(),
                        regions: st
                            .regions
                            .iter()
                            .enumerate()
                            .map(|(r, g)| (g.name.to_string(), play.region_lit(s, r)))
                            .collect(),
                        active,
                        mark: active.then(|| "story".into()),
                    }
                })
                .collect(),
            sets: play
                .sets()
                .into_iter()
                .map(|s| SetRow {
                    name: s.def.name.into(),
                    pieces: s
                        .pieces
                        .iter()
                        .map(|(f, found)| {
                            let text = if *found { f.story } else { f.hint };
                            (
                                f.name.to_string(),
                                *found,
                                text.to_string(),
                                f.kind.to_string(),
                            )
                        })
                        .collect(),
                    mark: Some(format!("set:{}", s.def.id)),
                })
                .collect(),
        }
    }

    pub fn shinies(play: &Play, out: &mut Vec<Shiny>) {
        for (i, (f, s)) in play.def.finds.iter().zip(&play.spots).enumerate() {
            if !play.found[i] && (play.night || !f.night) {
                out.push(Shiny {
                    id: i,
                    pos: s.pos,
                    kind: f.kind,
                });
            }
        }
    }

    pub fn set_night(play: &mut Play, night: bool) {
        play.night = night;
    }

    pub fn thank_all(play: &mut Play) {
        play.favored.iter_mut().for_each(|f| *f = true);
    }

    /// The named dragons, as the game flies and draws them.
    pub fn kin_info(play: &Play) -> Vec<crate::kin::KinInfo> {
        play.def
            .kin
            .iter()
            .enumerate()
            .map(|(i, k)| crate::kin::KinInfo {
                name: k.name.into(),
                kind: k.kind,
                tint: k.tint,
                size: k.size,
                spots: play.kin_at[i].clone(),
                way: play.kin_way[i].clone(),
                // The finish's height is left for the game to find on the ground.
                race: k
                    .race
                    .as_ref()
                    .map(|(to, at, r, _)| (to.to_string(), Vec3::new(at[0], f32::NAN, at[1]), *r)),
            })
            .collect()
    }

    /// Whether dragon `i` is met.
    pub fn kin_met(play: &Play, i: usize) -> bool {
        play.kin.met[i]
    }

    pub fn kin_name(play: &Play, i: usize) -> &'static str {
        play.def.kin[i].name
    }

    /// Dragon `i` is found: met with a card, or the shy one flies off.
    pub fn find_kin(play: &mut Play, i: usize) -> Option<Meet> {
        let k = &play.def.kin[i];
        Some(match play.find_kin(i)? {
            zawnabout_play::KinNews::Shy(_, n) => Meet::Flee(
                "A little dragon!".into(),
                if n == 1 {
                    "It squeaked and flew off to another needle! Maybe it will stay if you find it again.".into()
                } else {
                    "Off it goes again! It peeked back at you, though. One more try.".into()
                },
            ),
            _ => Meet::Card(Card {
                title: k.name.into(),
                body: k.meet.into(),
                foot: "A new dragon friend! See the Dragons page in your journal.".into(),
                kind: CardKind::Say,
            }),
        })
    }

    /// What dragon `i` would like to do now, most wanted first.
    fn kin_ask(play: &Play, i: usize) -> KinAsk {
        let (k, s) = (&play.def.kin[i], &play.kin);
        if k.kind == "wise" && play.all_kin_met() && s.egg == zawnabout_play::Egg::None {
            KinAsk::Egg
        } else if play.trade_offer(i).is_some() {
            KinAsk::Trade
        } else if k.race.is_some() && (!s.won[i] || s.followed[i]) {
            // A racer wants a race first, and then shows its secret to whoever won.
            KinAsk::Race
        } else if !s.followed[i] {
            KinAsk::Follow
        } else {
            KinAsk::Chat
        }
    }

    /// Talking to dragon `i`: a line, and maybe something to do together.
    pub fn kin_talk(play: &Play, i: usize, seed: u32) -> Card {
        let k = &play.def.kin[i];
        let ask = kin_ask(play, i);
        let line = k
            .lines
            .get(seed as usize % k.lines.len().max(1))
            .copied()
            .unwrap_or("");
        let (body, foot) = match ask {
            KinAsk::Egg => (
                format!(
                    "You found them all! Every dragon on the islands, after all these years. Here, I've kept this egg warm for a long, long time. It should be with a young dragon. Take it to your lair and warm it with your fire{}.",
                    if play.lair.is_none() {
                        ", once you have a lair"
                    } else {
                        ""
                    }
                ),
                "An egg of your own!".to_string(),
            ),
            KinAsk::Trade => {
                let f = play.trade_offer(i).map_or("", |f| play.def.finds[f].name);
                let gift = k.trade.as_ref().map_or("", |t| t.1.name);
                (
                    format!(
                        "Ooh! Is that {f}? I'd trade you {gift} for it. Only if you like, of course!"
                    ),
                    String::new(),
                )
            }
            KinAsk::Follow => (
                format!("{line} Want to see something? Follow me, I'll wait for you at each stop."),
                String::new(),
            ),
            KinAsk::Race => {
                let to = k.race.as_ref().map_or("", |r| r.0);
                (
                    format!("{line} Race you to {to}? Last one there is a slow old cloud!"),
                    if play.kin.won[i] {
                        "You've beaten Blaze before. Blaze would like that not to happen again."
                            .replace("Blaze", k.name)
                    } else {
                        String::new()
                    },
                )
            }
            KinAsk::Chat => {
                let hint = if k.kind == "wise" {
                    play.sets()
                        .into_iter()
                        .flat_map(|s| s.pieces.into_iter())
                        .find(|p| !p.1)
                        .map(|p| format!(" A little hint, if you like: {}", p.0.hint))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                (format!("{line}{hint}"), String::new())
            }
        };
        Card {
            title: k.name.into(),
            body,
            foot,
            kind: CardKind::Kin(i, ask),
        }
    }

    /// The egg is given or the trade made. Its card, when there is one.
    pub fn kin_do(play: &mut Play, i: usize, ask: KinAsk) -> Option<Card> {
        let k = &play.def.kin[i];
        match ask {
            KinAsk::Egg if play.give_egg() => Some(Card {
                title: "A dragon egg".into(),
                body: "It's warm and smooth, with little gold speckles. Something inside is already wriggling. Keep it in your lair and warm it with your fire to hatch it.".into(),
                foot: "Breathe fire on it by your heap.".into(),
                kind: CardKind::Find,
            }),
            KinAsk::Trade if play.trade(i) => {
                let t = &k.trade.as_ref()?.1;
                Some(Card {
                    title: t.name.into(),
                    body: t.story.into(),
                    foot: format!("A swap with {}!", k.name),
                    kind: CardKind::Find,
                })
            }
            _ => None,
        }
    }

    /// Dragon `i` showed you its secret.
    pub fn kin_shown(play: &mut Play, i: usize) -> (String, String) {
        play.followed(i);
        (
            play.def.kin[i].name.into(),
            "Here it is! My secret spot. Have a look around, there's something here for you."
                .into(),
        )
    }

    /// The race is over: a card for a first win, a line otherwise.
    pub fn kin_raced(play: &mut Play, i: usize, won: bool) -> Result<Card, (String, String)> {
        let k = &play.def.kin[i];
        let name = k.name.to_string();
        if !won {
            return Err((
                name,
                "Ha! Beat you! Good race though. Come back for a rematch any time!".into(),
            ));
        }
        if play.beat(i)
            && let Some((_, _, _, prize)) = &k.race
        {
            return Ok(Card {
                title: prize.name.into(),
                body: prize.story.into(),
                foot: format!("You beat {name}!"),
                kind: CardKind::Find,
            });
        }
        Err((name, "You won again?! I'm going to practice. A lot.".into()))
    }

    /// A friend has left a gift by the heap, if one has not yet.
    pub fn kin_gift(play: &mut Play) -> Option<Card> {
        let i = (0..play.def.kin.len()).find(|&i| play.leave_gift(i))?;
        let k = &play.def.kin[i];
        Some(Card {
            title: k.gift.name.into(),
            body: format!(
                "{} left this by your heap while you were out. {}",
                k.name, k.gift.story
            ),
            foot: "A gift from a friend!".into(),
            kind: CardKind::Find,
        })
    }

    /// 0 with no egg, 1 waiting to hatch, 2 hatched.
    pub fn egg(play: &Play) -> u8 {
        match play.kin.egg {
            zawnabout_play::Egg::None => 0,
            zawnabout_play::Egg::Given => 1,
            zawnabout_play::Egg::Hatched => 2,
        }
    }

    pub fn hatch(play: &mut Play) -> Option<Card> {
        play.hatch().then(|| Card {
            title: "It hatched!".into(),
            body: "Crack, crack, pop! A tiny dragon tumbles out, blinks at you, and sneezes a little puff of smoke. It's going to follow you everywhere now.".into(),
            foot: "Share some firsts with it and watch it grow. See the Dragons page.".into(),
            kind: CardKind::News,
        })
    }

    /// The hatchling's first `name`: a line about it, the first time.
    pub fn first(play: &mut Play, name: &str) -> Option<(String, String)> {
        let k = play.first(name)?;
        let line = match k {
            0 => "Your little one flew beside you the whole way! Its wings are getting stronger.",
            1 => "Splash! Your little one loved the water. It's a bit bigger now.",
            2 => "Your little one breathed its very first flame. A tiny one, but a real one!",
            _ => "Your little one tried that stunt too! Upside down, but it counts.",
        };
        Some(("Little one".into(), line.into()))
    }

    pub fn hatchling_size(play: &Play) -> f32 {
        play.kin.hatchling_size()
    }

    /// The dragons page: each named dragon met or heard of, then the family.
    fn kin_rows(play: &Play) -> Vec<KinRow> {
        let s = &play.kin;
        let mut out: Vec<KinRow> = play
            .def
            .kin
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let island = match k.island {
                    "home" => "the home island".to_string(),
                    id => island_name(id).replacen("The ", "the ", 1),
                };
                if !s.met[i] {
                    return KinRow {
                        name: "A dragon?".into(),
                        status: "Not met".into(),
                        about: format!("Folk say there's a dragon living on {island}."),
                        done: Vec::new(),
                        mark: None,
                    };
                }
                let mut done = vec![format!("Lives on {island}")];
                if s.followed[i] {
                    done.push("Showed you its secret".into());
                }
                if s.won[i] {
                    done.push("You beat it in a race".into());
                }
                if s.traded[i] {
                    done.push("Made a swap with you".into());
                }
                if s.gifted[i] {
                    done.push(format!("Left you {}", k.gift.name));
                }
                KinRow {
                    name: k.name.into(),
                    status: match k.kind {
                        "wise" => "Wise",
                        "racer" => "Racer",
                        "shy" => "Shy",
                        _ => "Collector",
                    }
                    .into(),
                    about: k.about.into(),
                    done,
                    mark: Some("dragons".into()),
                }
            })
            .collect();
        let family = match s.egg {
            zawnabout_play::Egg::None => None,
            zawnabout_play::Egg::Given => Some(KinRow {
                name: "Your egg".into(),
                status: "Waiting".into(),
                about: "A gift from Grandmother Cinder. Keep it in your lair and warm it with your fire to hatch it.".into(),
                done: Vec::new(),
                mark: None,
            }),
            zawnabout_play::Egg::Hatched => Some(KinRow {
                name: "Your little one".into(),
                status: format!(
                    "{} of {} firsts",
                    s.firsts.iter().filter(|f| **f).count(),
                    s.firsts.len()
                ),
                about: "Your hatchling, always at your side. It grows a little with every first you share.".into(),
                done: zawnabout_play::kin::FIRSTS
                    .iter()
                    .zip(s.firsts)
                    .map(|((_, label), done)| {
                        if done {
                            label.to_string()
                        } else {
                            "...".into()
                        }
                    })
                    .collect(),
                mark: None,
            }),
        };
        out.extend(family);
        out
    }

    pub fn heap(play: &Play) -> Option<Heap> {
        Some(Heap {
            at: play.lairs[play.lair?],
            gold: play.gold(),
            kinds: play
                .treasures()
                .map(|f| f.kind)
                .chain(play.gifts().map(|g| g.kind))
                .chain((play.kin.egg == zawnabout_play::Egg::Given).then_some("egg"))
                .collect(),
        })
    }

    pub fn save(play: &Play) -> String {
        play.save()
    }

    pub fn load(play: &mut Play, text: &str) -> Result<(), String> {
        play.load(text)
    }

    pub fn reset(play: &mut Play) {
        play.reset();
    }

    /// Whether something to save changed since last asked.
    pub fn take_changed(play: &mut Play) -> bool {
        std::mem::take(&mut play.changed)
    }

    pub fn status(play: &Play) -> String {
        let found = play.found.iter().filter(|f| **f).count();
        format!(
            "found {found} of {}, gold {}, lair {}",
            play.found.len(),
            play.gold(),
            play.lair.map_or("none", |l| play.def.lairs[l].id)
        )
    }

    /// Every find and lair with where it rests, for scripts.
    pub fn list(play: &Play) -> String {
        let mut out = Vec::new();
        for (i, (f, s)) in play.def.finds.iter().zip(&play.spots).enumerate() {
            let p = s.pos;
            let got = if play.found[i] { "found" } else { "out" };
            out.push(format!(
                "find {} {} at ({:.1},{:.1},{:.1}) {got}",
                f.id, f.kind, p.x, p.y, p.z
            ));
        }
        for (l, p) in play.def.locals.iter().zip(&play.folk) {
            out.push(format!(
                "local {} at ({:.1},{:.1},{:.1})",
                l.id, p.x, p.y, p.z
            ));
        }
        for (l, p) in play.def.lairs.iter().zip(&play.lairs) {
            out.push(format!(
                "lair {} at ({:.1},{:.1},{:.1})",
                l.id, p.x, p.y, p.z
            ));
        }
        for (s, d) in play.def.stunts.iter().zip(&play.flying.stunts) {
            out.push(format!(
                "stunt {} {} {}",
                s.id,
                s.kind,
                if *d { "done" } else { "-" }
            ));
        }
        for (c, course) in play.def.courses.iter().enumerate() {
            out.push(format!(
                "course {} {} of {}",
                course.id,
                play.gates_passed(c),
                course.gates.len()
            ));
        }
        for (r, w) in play.def.races.iter().zip(&play.flying.won) {
            out.push(format!("race {} {}", r.id, if *w { "won" } else { "-" }));
        }
        out.join(
            "
",
        )
    }

    /// Whether find `id` is found, none when there is no such find.
    pub fn is_found(play: &Play, id: &str) -> Option<bool> {
        let i = play.def.finds.iter().position(|f| f.id == id)?;
        Some(play.found[i])
    }

    pub fn lair_id(play: &Play) -> Option<&'static str> {
        Some(play.def.lairs[play.lair?].id)
    }

    /// Where find `id` lies, for scripts.
    pub fn find(play: &Play, id: &str) -> Option<Vec3> {
        let i = play.def.finds.iter().position(|f| f.id == id)?;
        Some(play.spots[i].pos)
    }

    /// Where a lair, a local, a local's lost item (`ID-item`) or a postcard spot is, for
    /// scripts.
    pub fn lair_at(play: &Play, id: &str) -> Option<Vec3> {
        if let Some(i) = play.def.postcards.iter().position(|c| c.id == id) {
            return Some(play.cards[i]);
        }
        if let Some(i) = play.def.locals.iter().position(|l| l.id == id) {
            return Some(play.folk[i]);
        }
        if let Some(local) = id.strip_suffix("-item")
            && let Some(i) = play.def.locals.iter().position(|l| l.id == local)
        {
            return play.items[i];
        }
        let i = play.def.lairs.iter().position(|l| l.id == id)?;
        Some(play.lairs[i])
    }
}

#[cfg(not(feature = "play"))]
mod on {
    use super::*;
    use crate::book::KinAsk;

    pub fn start(_: &World) -> Option<Play> {
        None
    }
    pub fn tick(play: &mut Play, _: &Dragon, _: &World, _: f32, _: f32) -> News {
        match *play {}
    }
    pub fn begin(play: &mut Play, _: usize) {
        match *play {}
    }
    pub fn set_aside(play: &mut Play) {
        match *play {}
    }
    pub fn dark(play: &Play) -> Vec<Vec4> {
        match *play {}
    }
    pub fn lamps(play: &Play) -> Vec<(Vec3, bool)> {
        match *play {}
    }
    pub fn dark_here(play: &Play, _: Vec3) -> Option<bool> {
        match *play {}
    }
    pub fn lamp_at(play: &Play, _: usize) -> Option<Vec3> {
        match *play {}
    }
    pub fn story_done(play: &Play, _: &str) -> Option<bool> {
        match *play {}
    }
    pub fn story_index(play: &Play, _: &str) -> Option<usize> {
        match *play {}
    }
    pub fn dare(play: &Play, _: u32) -> Option<String> {
        match *play {}
    }
    pub fn lair_tip(play: &Play, _: u32) -> Option<String> {
        match *play {}
    }
    pub fn effects(play: &Play) -> Vec<Effect> {
        match *play {}
    }
    pub fn guest_near(play: &Play, _: Vec3, _: f32) -> Option<(usize, Vec3)> {
        match *play {}
    }
    pub fn guest_card(play: &Play, _: usize) -> Option<Card> {
        match *play {}
    }
    pub fn gate_way(play: &Play, _: &World, _: &str, _: usize) -> Option<(Vec3, Vec3)> {
        match *play {}
    }
    pub fn flown(play: &Play, _: &str) -> Option<bool> {
        match *play {}
    }
    pub fn race_spot(play: &Play, _: &str, _: bool) -> Option<(f32, f32)> {
        match *play {}
    }
    pub fn racing(play: &Play) -> bool {
        match *play {}
    }
    pub fn gate_passed(play: &Play, _: &str, _: usize) -> Option<bool> {
        match *play {}
    }
    pub fn nearest_local(play: &Play, _: Vec3, _: f32) -> Option<(usize, Vec3)> {
        match *play {}
    }
    pub fn asks(play: &Play, _: usize, _: &dyn Fn(&str) -> bool) -> bool {
        match *play {}
    }
    pub fn talk_card(play: &mut Play, _: usize, _: &dyn Fn(&str) -> bool) -> Card {
        match *play {}
    }
    pub fn accept(play: &mut Play, _: usize) {
        match *play {}
    }
    pub fn hop_off(play: &mut Play) {
        match *play {}
    }
    pub fn give_back(play: &mut Play, _: usize) -> Card {
        match *play {}
    }
    pub fn people(
        play: &Play,
        _: &dyn Fn(&str) -> bool,
        _: &mut Vec<Local>,
        _: &mut Vec<Lost>,
    ) -> Option<&'static str> {
        match *play {}
    }
    pub fn rumor(play: &Play, _: u32) -> Option<String> {
        match *play {}
    }
    pub fn kindness(play: &Play) -> usize {
        match *play {}
    }
    pub fn spot_near(play: &Play, _: Vec3, _: f32) -> Option<(&'static str, Vec3)> {
        match *play {}
    }
    pub fn helped(play: &Play, _: &str) -> Option<bool> {
        match *play {}
    }
    pub fn decline(play: &mut Play, _: usize) {
        match *play {}
    }
    pub fn chore_card(
        play: &mut Play,
        _: u32,
        _: Vec3,
        _: &dyn Fn(f32, f32) -> Vec3,
        _: &str,
    ) -> Option<(Card, Option<Chore>)> {
        match *play {}
    }
    pub fn take_chore(play: &mut Play, _: Chore) {
        match *play {}
    }
    pub fn folk_asks(play: &Play, _: u32, _: Vec3) -> bool {
        match *play {}
    }
    pub fn rider(play: &Play) -> Option<Rider> {
        match *play {}
    }
    pub fn goal(play: &Play) -> Option<Goal> {
        match *play {}
    }
    pub fn riding_folk(play: &Play) -> Option<u32> {
        match *play {}
    }
    pub fn postcard_card(play: &Play, _: usize) -> Card {
        match *play {}
    }
    pub fn card_spot(play: &Play, _: usize) -> Option<(Vec3, f32)> {
        match *play {}
    }
    pub fn card_low(play: &Play, _: usize) -> bool {
        match *play {}
    }
    pub fn card_index(play: &Play, _: &str) -> Option<usize> {
        match *play {}
    }
    pub fn set_pic(play: &mut Play, _: usize, _: &[u8], _: usize, _: usize) {
        match *play {}
    }
    pub fn pics(play: &Play, _: u32) -> Option<Pics> {
        match *play {}
    }
    pub fn claim(play: &mut Play, _: usize) {
        match *play {}
    }
    pub fn view(play: &Play, _: &dyn Fn(&str) -> Option<Vec3>) -> HoardView {
        match *play {}
    }
    pub fn shinies(play: &Play, _: &mut Vec<Shiny>) {
        match *play {}
    }
    pub fn kin_info(play: &Play) -> Vec<crate::kin::KinInfo> {
        match *play {}
    }
    pub fn set_night(play: &mut Play, _: bool) {
        match *play {}
    }
    pub fn thank_all(play: &mut Play) {
        match *play {}
    }
    pub fn kin_met(play: &Play, _: usize) -> bool {
        match *play {}
    }
    pub fn kin_name(play: &Play, _: usize) -> &'static str {
        match *play {}
    }
    pub fn find_kin(play: &mut Play, _: usize) -> Option<Meet> {
        match *play {}
    }
    pub fn kin_talk(play: &Play, _: usize, _: u32) -> Card {
        match *play {}
    }
    pub fn kin_do(play: &mut Play, _: usize, _: KinAsk) -> Option<Card> {
        match *play {}
    }
    pub fn kin_shown(play: &mut Play, _: usize) -> (String, String) {
        match *play {}
    }
    pub fn kin_raced(play: &mut Play, _: usize, _: bool) -> Result<Card, (String, String)> {
        match *play {}
    }
    pub fn kin_gift(play: &mut Play) -> Option<Card> {
        match *play {}
    }
    pub fn egg(play: &Play) -> u8 {
        match *play {}
    }
    pub fn hatch(play: &mut Play) -> Option<Card> {
        match *play {}
    }
    pub fn first(play: &mut Play, _: &str) -> Option<(String, String)> {
        match *play {}
    }
    pub fn hatchling_size(play: &Play) -> f32 {
        match *play {}
    }
    pub fn heap(play: &Play) -> Option<Heap> {
        match *play {}
    }
    pub fn save(play: &Play) -> String {
        match *play {}
    }
    pub fn load(play: &mut Play, _: &str) -> Result<(), String> {
        match *play {}
    }
    pub fn reset(play: &mut Play) {
        match *play {}
    }
    pub fn take_changed(play: &mut Play) -> bool {
        match *play {}
    }
    pub fn status(play: &Play) -> String {
        match *play {}
    }
    pub fn list(play: &Play) -> String {
        match *play {}
    }
    pub fn is_found(play: &Play, _: &str) -> Option<bool> {
        match *play {}
    }
    pub fn lair_id(play: &Play) -> Option<&'static str> {
        match *play {}
    }
    pub fn find(play: &Play, _: &str) -> Option<Vec3> {
        match *play {}
    }
    pub fn lair_at(play: &Play, _: &str) -> Option<Vec3> {
        match *play {}
    }
}

pub use on::*;

#[cfg(all(test, feature = "play"))]
mod tests {
    use crate::hud::{ATLAS, CARD_CELL, card_slot};
    use zawnabout_play::{PLAY, paint::PIC};

    /// Postcard pictures fit their atlas cells, and every postcard has a cell.
    #[test]
    fn pictures_fit_the_atlas() {
        assert_eq!((CARD_CELL.0 as usize - 4, CARD_CELL.1 as usize - 4), PIC);
        let (x, y) = card_slot(PLAY.postcards.len() - 1);
        assert!(x + CARD_CELL.0 <= ATLAS.0 && y + CARD_CELL.1 <= ATLAS.1);
    }
}
