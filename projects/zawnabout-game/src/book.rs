//! The journal, which is also the pause screen: opening it pauses, closing it flies on.
//! Its pages are the hoard, help with the controls (and changing keys), and settings. It
//! opens where it was left, on help the first time. Cards for things found show over it
//! and wait to be dismissed. Keys, gamepad, mouse and touch all work it.

use std::cell::{Cell, RefCell};

use glam::Vec2;

use crate::hud::{Draw, key_label, rgba};
use crate::input::{Action, Bindings};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    Hoard,
    Places,
    People,
    Flying,
    Stories,
    Dragons,
    Map,
    #[default]
    Help,
    Settings,
}

impl Page {
    pub const ALL: [Page; 9] = [
        Page::Hoard,
        Page::Places,
        Page::People,
        Page::Flying,
        Page::Stories,
        Page::Dragons,
        Page::Map,
        Page::Help,
        Page::Settings,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Page::Hoard => "hoard",
            Page::Places => "places",
            Page::People => "people",
            Page::Flying => "flying",
            Page::Stories => "stories",
            Page::Dragons => "dragons",
            Page::Map => "map",
            Page::Help => "help",
            Page::Settings => "settings",
        }
    }

    pub fn from_name(s: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|p| p.name() == s)
    }

    fn title(self) -> &'static str {
        match self {
            Page::Hoard => "Hoard",
            Page::Places => "Places",
            Page::People => "People",
            Page::Flying => "Flying",
            Page::Stories => "Stories",
            Page::Dragons => "Dragons",
            Page::Map => "Map",
            Page::Help => "Help",
            Page::Settings => "Settings",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CardKind {
    /// Something picked up.
    Find,
    /// Good news: a set done, a town's thanks, a hatching.
    News,
    /// Landed in a lair: make it home?
    Lair(usize),
    /// A postcard taken, with its picture's slot once painted.
    Postcard(Option<usize>),
    /// Someone says something.
    Say,
    /// Picked up someone's lost thing, to carry back.
    Carry,
    /// A townsperson's small ask.
    Chore,
    /// Local `i` asks a favor.
    Offer(usize),
    /// Local `i` sees their lost thing in the hoard.
    Return(usize),
    /// Someone asks for help with story `s`.
    Story(usize),
    /// The passenger: where to, or hop off.
    Rider,
    /// Dragon `i` talks, maybe with something to do together.
    Kin(usize, KinAsk),
}

/// What a dragon asks to do with you.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KinAsk {
    Chat,
    Follow,
    Race,
    Trade,
    Egg,
}

/// A card over everything, waiting to be dismissed.
#[derive(Clone, Debug)]
pub struct Card {
    pub title: String,
    pub body: String,
    /// A small line under the body, like a set's progress.
    pub foot: String,
    pub kind: CardKind,
}

/// A set's pieces for the hoard page: name, found, the hint or story, and its model.
#[derive(Clone, Debug, Default)]
pub struct SetRow {
    pub name: String,
    pub pieces: Vec<(String, bool, String, String)>,
    /// Its group on the map.
    pub mark: Option<String>,
}

/// One spot on the map: where (x, z), whether it is done with, and what it is.
#[derive(Clone, Debug, Default)]
pub struct Mark {
    pub at: Vec2,
    pub done: bool,
    pub label: String,
}

/// Things to show together on the map, picked from its list or from Show me.
#[derive(Clone, Debug, Default)]
pub struct MarkGroup {
    pub key: String,
    pub name: String,
    pub about: String,
    pub spots: Vec<Mark>,
}

/// What the map page needs from the game: the map's size in the atlas (zero until
/// drawn), the world it covers, and where the dragon is and heads.
#[derive(Clone, Copy, Debug, Default)]
pub struct MapView {
    pub px: f32,
    pub bounds: [f32; 4],
    pub me: Vec2,
    pub heading: f32,
}

/// Someone met, for the people page.
#[derive(Clone, Debug, Default)]
pub struct PersonRow {
    pub name: String,
    pub town: String,
    /// Where their favor stands, or what to do for it.
    pub status: String,
    pub helped: bool,
    pub mark: Option<String>,
}

/// A course, race or stunt, for the flying page.
#[derive(Clone, Debug, Default)]
pub struct FlyRow {
    pub name: String,
    /// Course, race or stunt.
    pub kind: String,
    /// How far along, such as 3 of 7.
    pub status: String,
    pub line: String,
    pub done: bool,
    pub mark: Option<String>,
}

/// A story, for the stories page: how far along, what it is about, and each region and
/// whether its lights are back.
#[derive(Clone, Debug, Default)]
pub struct StoryRow {
    pub name: String,
    pub status: String,
    pub about: String,
    pub regions: Vec<(String, bool)>,
    pub active: bool,
    pub mark: Option<String>,
}

/// A dragon for the dragons page, or the family at the end.
#[derive(Clone, Debug, Default)]
pub struct KinRow {
    pub name: String,
    pub status: String,
    pub about: String,
    /// What you have done together, a line each.
    pub done: Vec<String>,
    pub mark: Option<String>,
}

/// A postcard taken, for the places page.
#[derive(Clone, Debug, Default)]
pub struct PlaceRow {
    pub name: String,
    pub line: String,
    pub island: String,
    /// Its picture's slot in the atlas, once painted.
    pub pic: Option<usize>,
}

/// What the hoard and places pages show.
#[derive(Clone, Debug, Default)]
pub struct HoardView {
    pub postcards: Vec<PlaceRow>,
    pub people: Vec<PersonRow>,
    pub flying: Vec<FlyRow>,
    pub stories: Vec<StoryRow>,
    /// The dragons met or heard of, then the family.
    pub kin: Vec<KinRow>,
    /// Towns helped, and how many favors done for each.
    pub towns: Vec<(String, usize)>,
    pub gold: u32,
    pub lair: Option<String>,
    /// Name, story and model of each treasure found, sets aside.
    pub treasures: Vec<(String, String, String)>,
    pub sets: Vec<SetRow>,
    /// Everything the map can show, group by group.
    pub marks: Vec<MarkGroup>,
}

impl HoardView {
    /// Which map group has `key`.
    pub fn group(&self, key: &str) -> Option<usize> {
        self.marks.iter().position(|g| g.key == key)
    }
}

/// The times of day the day can be held at, in the order Settings steps through them.
pub const HOLD_HOURS: [(&str, f32); 4] = [
    ("Morning", 8.0),
    ("Noon", 12.5),
    ("Sunset", 19.2),
    ("Night", 23.0),
];

/// What Settings calls a held hour, or the day passing.
pub fn time_name(hold: Option<f32>) -> &'static str {
    match hold {
        None => "Passing",
        Some(h) => HOLD_HOURS
            .iter()
            .min_by(|a, b| (a.1 - h).abs().total_cmp(&(b.1 - h).abs()))
            .map_or("Held", |t| t.0),
    }
}

/// Everything the book shows that lives elsewhere.
pub struct View<'a> {
    /// The hoard, or none with gameplay off.
    pub hoard: Option<&'a HoardView>,
    pub binds: &'a Bindings,
    pub muted: bool,
    pub pause_away: bool,
    /// The hour the day is held at, none while it passes.
    pub hold_hour: Option<f32>,
    pub touch: bool,
    /// The game draws its own map and notes, so their keys get a line.
    pub hud: bool,
    pub map: MapView,
}

/// What the book asks the game to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Do {
    Close,
    Sound,
    PauseAway,
    /// Do something with dragon `i`.
    Kin(usize, KinAsk),
    /// The next time of day: passing, then held at morning, noon, sunset or night.
    TimeOfDay,
    Export,
    Import,
    ResetKeys,
    Bind(Action, String),
    Claim(usize),
    /// The page changed, worth remembering.
    Turned,
    /// Open the game's source code page.
    Source,
    /// Take on local `i`'s favor.
    Accept(usize),
    /// Give local `i` back their lost thing.
    GiveBack(usize),
    /// Lair `i` is not wanted now.
    Decline(usize),
    /// Take on the townsperson's small ask that was just offered.
    TakeChore,
    /// Start story `s`.
    Begin(usize),
    /// Set the story under way aside.
    SetAside,
    /// Show the passenger's way on the map.
    RiderMap,
    /// Let the passenger off here.
    HopOff,
}

/// Where the game's source code lives.
pub const SOURCE: &str = "https://github.com/cretz/zawnabout";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Icon,
    Source,
    Tab(Page),
    Row(usize),
    Close,
    Panel,
    Yes,
    No,
    /// A card's third answer.
    Alt,
    /// Show map group `g`.
    ShowMe(usize),
    /// Put the big postcard away.
    Zoom,
}

#[derive(Clone, Debug, Default)]
pub struct Book {
    pub open: bool,
    pub page: Page,
    /// Cards waiting, the first showing.
    pub cards: Vec<Card>,
    /// The picked row on the page.
    sel: usize,
    /// The next key goes to the picked action.
    waiting: bool,
    /// What the pointer can hit, as of the last draw, and the screen it was drawn on.
    hits: RefCell<Vec<(Vec2, Vec2, Hit)>>,
    size: Cell<Vec2>,
    /// The postcard shown big, on the places page.
    zoom: Option<usize>,
    /// How far a page turn has left to go: 1 just turned, 0 done.
    flip: Cell<f32>,
}

const PAPER: u32 = 0xFFF6E6;
const INK: u32 = 0x4A3426;
const RED: u32 = 0xE0564A;
const GOLD: u32 = 0xC8902A;
const TAN: u32 = 0xEAD8BC;
const COVER: u32 = 0x7A4A2E;
const RULE: u32 = 0x9CC3E0;
/// Each page's tab color, in the order of [`Page::ALL`].
const TABS: [u32; 9] = [
    0xF2C14E, 0x8FC7E8, 0xF4A3A3, 0xA8D88F, 0xC9A8E8, 0xE89A8A, 0x9FD6CF, 0xF7B983, 0xD9C7AE,
];

impl Book {
    /// The screen size it was last drawn at, in pixels.
    pub fn screen(&self) -> Vec2 {
        self.size.get()
    }

    /// Whether it covers the game, which pauses it.
    pub fn showing(&self) -> bool {
        self.open || !self.cards.is_empty()
    }

    fn pages(view: &View) -> Vec<Page> {
        Page::ALL
            .into_iter()
            .filter(|p| matches!(p, Page::Help | Page::Settings) || view.hoard.is_some())
            .collect()
    }

    /// Settings rows before the keys.
    fn settings_rows(view: &View) -> usize {
        3 + 2 * view.hoard.is_some() as usize
    }

    /// Turns to `page`, with a page turning over.
    fn go(&mut self, page: Page, sel: usize) -> Vec<Do> {
        if page != self.page {
            self.flip.set(1.0);
        }
        self.page = page;
        self.sel = sel;
        self.zoom = None;
        vec![Do::Turned]
    }

    /// Opens the map on group `key`, as Show me does. False if there is no such group.
    pub fn show_me(&mut self, key: &str, view: &View) -> bool {
        let Some(g) = view.hoard.and_then(|h| h.group(key)) else {
            return false;
        };
        self.show(view);
        self.go(Page::Map, g);
        true
    }

    fn rows(&self, view: &View) -> usize {
        match self.page {
            Page::Hoard => view.hoard.map_or(0, |h| {
                h.sets.iter().map(|s| s.pieces.len()).sum::<usize>() + h.treasures.len()
            }),
            Page::Places => view.hoard.map_or(0, |h| h.postcards.len()),
            Page::People => view.hoard.map_or(0, |h| h.people.len()),
            Page::Flying => view.hoard.map_or(0, |h| h.flying.len()),
            Page::Stories => view.hoard.map_or(0, |h| h.stories.len()),
            Page::Dragons => view.hoard.map_or(0, |h| h.kin.len()),
            Page::Map => view.hoard.map_or(0, |h| h.marks.len()),
            Page::Help => 0,
            Page::Settings if view.touch => Self::settings_rows(view),
            Page::Settings => Self::settings_rows(view) + Action::ALL.len() + 1,
        }
    }

    fn turn(&mut self, by: isize, view: &View) -> Vec<Do> {
        let pages = Self::pages(view);
        let at = pages.iter().position(|p| *p == self.page).unwrap_or(0) as isize;
        let n = pages.len() as isize;
        let page = pages[((at + by) % n + n) as usize % pages.len()];
        self.go(page, 0)
    }

    /// Opens on the page it was left on, unless that page is not there.
    pub fn show(&mut self, view: &View) {
        if !Self::pages(view).contains(&self.page) {
            self.page = Page::Help;
        }
        self.open = true;
        self.waiting = false;
    }

    fn activate(&mut self, view: &View) -> Vec<Do> {
        let base = Self::settings_rows(view);
        match self.page {
            Page::Places => {
                let has = view.hoard.is_some_and(|h| self.sel < h.postcards.len());
                self.zoom = (self.zoom.is_none() && has).then_some(self.sel);
                vec![]
            }
            Page::Settings if self.sel >= base && !view.touch => {
                if self.sel - base < Action::ALL.len() {
                    self.waiting = true;
                    vec![]
                } else {
                    vec![Do::ResetKeys]
                }
            }
            Page::Stories => match view.hoard.and_then(|h| h.stories.get(self.sel)) {
                Some(s) if s.active => vec![Do::SetAside],
                Some(_) => vec![Do::Begin(self.sel)],
                None => vec![],
            },
            Page::Settings => match self.sel {
                0 => vec![Do::Sound],
                1 => vec![Do::PauseAway],
                2 => vec![Do::TimeOfDay],
                3 => vec![Do::Export],
                _ => vec![Do::Import],
            },
            _ => vec![],
        }
    }

    fn card_answer(&mut self, yes: bool) -> Vec<Do> {
        let card = self.cards.remove(0);
        match card.kind {
            CardKind::Lair(i) if yes => vec![Do::Claim(i)],
            CardKind::Lair(i) => vec![Do::Decline(i)],
            CardKind::Chore if yes => vec![Do::TakeChore],
            CardKind::Offer(i) if yes => vec![Do::Accept(i)],
            CardKind::Return(i) if yes => vec![Do::GiveBack(i)],
            CardKind::Story(s) if yes => vec![Do::Begin(s)],
            CardKind::Rider if yes => vec![Do::RiderMap],
            CardKind::Kin(i, ask) if yes && ask != KinAsk::Chat => vec![Do::Kin(i, ask)],
            _ => vec![],
        }
    }

    /// A card's third answer.
    fn card_alt(&mut self) -> Vec<Do> {
        match self.cards.remove(0).kind {
            CardKind::Rider => vec![Do::HopOff],
            _ => vec![],
        }
    }

    /// Offers a key to the book. None when it is not the book's.
    pub fn key(&mut self, code: &str, view: &View) -> Option<Vec<Do>> {
        if !self.cards.is_empty() {
            return Some(match code {
                "Enter" | "NumpadEnter" | "Space" => self.card_answer(true),
                "Escape" | "Backspace" | "KeyP" => self.card_answer(false),
                "KeyH" if self.cards[0].kind == CardKind::Rider => self.card_alt(),
                _ => vec![],
            });
        }
        if !self.open {
            if matches!(code, "Escape" | "KeyP") {
                self.show(view);
                return Some(vec![]);
            }
            return None;
        }
        if self.waiting {
            self.waiting = false;
            if code == "Escape" {
                return Some(vec![]);
            }
            let k = self.sel.saturating_sub(Self::settings_rows(view));
            return Some(vec![Do::Bind(Action::ALL[k], code.to_string())]);
        }
        if self.zoom.is_some()
            && matches!(code, "Escape" | "Enter" | "NumpadEnter" | "Space" | "KeyP")
        {
            self.zoom = None;
            return Some(vec![]);
        }
        let rows = self.rows(view);
        Some(match code {
            "Escape" | "KeyP" | "Space" => {
                self.open = false;
                vec![Do::Close]
            }
            "ArrowLeft" | "KeyA" | "BracketLeft" | "PageUp" => self.turn(-1, view),
            "ArrowRight" | "KeyD" | "BracketRight" | "PageDown" | "Tab" => self.turn(1, view),
            "ArrowUp" | "KeyW" => {
                self.sel = (self.sel + rows.max(1) - 1) % rows.max(1);
                vec![]
            }
            "ArrowDown" | "KeyS" => {
                self.sel = (self.sel + 1) % rows.max(1);
                vec![]
            }
            "Enter" | "NumpadEnter" => self.activate(view),
            // Sound still toggles with the book open.
            "KeyU" => return None,
            _ => vec![],
        })
    }

    /// What is under `n`, a fraction of the screen.
    fn hit(&self, n: Vec2) -> Option<Hit> {
        let p = n * self.size.get();
        self.hits
            .borrow()
            .iter()
            .rev()
            .find(|(a, b, _)| p.x >= a.x && p.y >= a.y && p.x <= b.x && p.y <= b.y)
            .map(|(_, _, h)| *h)
    }

    /// A click or tap at `n`, a fraction of the screen. None when it falls through to
    /// the world.
    pub fn click(&mut self, n: Vec2, view: &View) -> Option<Vec<Do>> {
        let hit = self.hit(n);
        if !self.cards.is_empty() {
            return Some(match hit {
                Some(Hit::Yes) => self.card_answer(true),
                Some(Hit::No) => self.card_answer(false),
                Some(Hit::Alt) => self.card_alt(),
                _ => vec![],
            });
        }
        if !self.open {
            return (hit == Some(Hit::Icon)).then(|| {
                self.show(view);
                vec![]
            });
        }
        self.waiting = false;
        if self.zoom.is_some() && hit != Some(Hit::Close) {
            self.zoom = None;
            return Some(vec![]);
        }
        Some(match hit {
            Some(Hit::Tab(page)) => self.go(page, 0),
            Some(Hit::ShowMe(g)) => self.go(Page::Map, g),
            Some(Hit::Zoom) => {
                self.zoom = None;
                vec![]
            }
            Some(Hit::Row(i)) => {
                self.sel = i;
                self.activate(view)
            }
            Some(Hit::Panel) => vec![],
            Some(Hit::Source) => vec![Do::Source],
            // The button, or anywhere outside the book, flies on.
            _ => {
                self.open = false;
                vec![Do::Close]
            }
        })
    }

    /// The pointer moved to `n`, a fraction of the screen: a row under it gets picked,
    /// so its details show.
    pub fn hover(&mut self, n: Vec2) {
        if self.open
            && !self.waiting
            && let Some(Hit::Row(i)) = self.hit(n)
        {
            self.sel = i;
        }
    }
}

/// Text sizes and spacing for a screen.
struct Ui {
    px: f32,
    line: f32,
}

impl Book {
    /// Draws the book, its cards and the icon that opens it.
    pub(crate) fn draw(&self, d: &mut Draw, view: &View) {
        let mut hits = self.hits.borrow_mut();
        hits.clear();
        self.size.set(d.size);
        let (w, h) = (d.size.x, d.size.y);
        let px = (h / 34.0).min(w / 34.0).clamp(11.0, 26.0);
        let ui = Ui {
            px,
            line: px * 1.45,
        };
        if !self.showing() {
            // The icon: a little open book in the top left corner. The web page has a
            // button of its own beside its others.
            if !view.hud {
                return;
            }
            let s = (px * 2.2).max(40.0);
            let p0 = Vec2::splat(12.0);
            d.rect(p0, p0 + s, rgba(0x1E1A16, 0.55));
            let (c, half) = (p0 + s * 0.5, s * 0.3);
            d.rect(
                c + Vec2::new(-half, -half * 0.8),
                c + Vec2::new(-half * 0.08, half * 0.8),
                rgba(PAPER, 1.0),
            );
            d.rect(
                c + Vec2::new(half * 0.08, -half * 0.8),
                c + Vec2::new(half, half * 0.8),
                rgba(PAPER, 1.0),
            );
            hits.push((p0, p0 + s, Hit::Icon));
            return;
        }
        if self.open {
            self.draw_book(d, view, &ui, &mut hits);
        }
        if let Some(card) = self.cards.first() {
            hits.clear();
            draw_card(d, card, &ui, &mut hits);
        }
    }

    fn draw_book(&self, d: &mut Draw, view: &View, ui: &Ui, hits: &mut Vec<(Vec2, Vec2, Hit)>) {
        let (w, h) = (d.size.x, d.size.y);
        let (px, line) = (ui.px, ui.line);
        let f = crate::font::get();
        d.rect(Vec2::ZERO, d.size, rgba(0x14202C, 0.45));
        // A spiral-bound journal in a leather cover, its tabs standing up off the top.
        let tab_h = line * 1.3;
        let bw = (w - 30.0).min(px * 46.0);
        let bh = (h - 30.0 - tab_h).min(px * 31.0);
        let p0 = Vec2::new((w - bw) * 0.5 + 6.0, (h - bh + tab_h) * 0.5);
        let p1 = p0 + Vec2::new(bw - 6.0, bh);
        d.rect(p0 + Vec2::new(4.0, 10.0), p1 + 14.0, rgba(0x000000, 0.25));
        d.rect(p0 - Vec2::new(16.0, 9.0), p1 + 9.0, rgba(0x4E2E1C, 1.0));
        d.rect(p0 - Vec2::new(14.0, 7.0), p1 + 7.0, rgba(COVER, 1.0));

        // Tabs along the top, smaller when they would not fit, the open one raised and
        // joined to its page.
        let pages = Self::pages(view);
        let room = bw - px * 4.0;
        let wide: f32 = pages
            .iter()
            .map(|p| f.measure(px * 1.0, p.title()) + px * 1.9)
            .sum();
        let fit = (room / wide).min(1.0);
        let tpx = px * fit;
        let mut x = p0.x + px * 2.6;
        let mut open_tab = None;
        for page in &pages {
            let k = Page::ALL.iter().position(|p| p == page).unwrap_or(0);
            let tw = f.measure(tpx, page.title()) + px * 1.5 * fit;
            let on = *page == self.page;
            let lift = if on { px * 0.35 } else { 0.0 };
            let (a, b) = (
                Vec2::new(x, p0.y - tab_h - lift),
                Vec2::new(x + tw, p0.y + 3.0),
            );
            if on {
                open_tab = Some((a, b, k, *page));
            } else {
                d.rect(
                    a - Vec2::new(1.0, 1.0),
                    b + Vec2::new(1.0, 0.0),
                    rgba(0x4E2E1C, 0.5),
                );
                d.rect(a, b, rgba(TABS[k], 1.0));
                d.write(
                    a + Vec2::new(px * 0.75 * fit, (tab_h - tpx * 1.3) * 0.5),
                    tpx,
                    page.title(),
                    rgba(INK, 0.85),
                );
            }
            hits.push((a, b, Hit::Tab(*page)));
            x += tw + px * 0.35 * fit;
        }
        // The pages under the top one, then the top one, ruled, with a margin.
        d.rect(p0 + 5.0, p1 + 5.0, rgba(0xE2D2B4, 1.0));
        d.rect(p0 + 2.5, p1 + 2.5, rgba(0xEFE3CB, 1.0));
        d.rect(p0, p1, rgba(PAPER, 1.0));
        if let Some((a, b, k, page)) = open_tab {
            d.rect(
                a - Vec2::new(1.0, 1.0),
                b + Vec2::new(1.0, 0.0),
                rgba(0x4E2E1C, 0.6),
            );
            d.rect(a, b, rgba(TABS[k], 1.0));
            d.rect(a, Vec2::new(b.x, a.y + 3.0), rgba(0xFFFFFF, 0.4));
            d.write(
                a + Vec2::new(px * 0.75 * fit, (tab_h - tpx * 1.3) * 0.5),
                tpx,
                page.title(),
                rgba(INK, 1.0),
            );
        }
        let mut ry = p0.y + line * 2.4;
        while ry < p1.y - line * 0.6 {
            d.rect(
                Vec2::new(p0.x + px * 1.6, ry),
                Vec2::new(p1.x - px * 0.5, ry + 1.0),
                rgba(RULE, 0.22),
            );
            ry += line;
        }
        d.rect(
            Vec2::new(p0.x + px * 2.2, p0.y),
            Vec2::new(p0.x + px * 2.2 + 1.5, p1.y),
            rgba(RED, 0.3),
        );
        // The spiral binding down the left edge.
        let mut ry = p0.y + px * 0.9;
        while ry < p1.y - px * 0.5 {
            d.rect(
                Vec2::new(p0.x + px * 0.5, ry - px * 0.17),
                Vec2::new(p0.x + px * 0.9, ry + px * 0.17),
                rgba(0x3A2A20, 0.85),
            );
            d.rect(
                Vec2::new(p0.x - px * 0.75, ry - px * 0.13),
                Vec2::new(p0.x + px * 0.72, ry + px * 0.13),
                rgba(0x9EA2AC, 1.0),
            );
            d.rect(
                Vec2::new(p0.x - px * 0.75, ry - px * 0.13),
                Vec2::new(p0.x + px * 0.72, ry - px * 0.02),
                rgba(0xEEF0F4, 1.0),
            );
            ry += px * 1.45;
        }
        hits.push((p0, p1, Hit::Panel));
        // The page's name at its head.
        let head = p0.y + px * 0.55;
        d.write(
            Vec2::new(p0.x + px * 2.7, head),
            px * 1.35,
            self.page.title(),
            rgba(GOLD, 1.0),
        );
        let top = head + line * 1.45;

        // The way out, bottom right.
        let label = "Keep flying";
        let bw2 = f.measure(px, label) + px * 2.0;
        let (a, b) = (
            Vec2::new(p1.x - px - bw2, p1.y - px * 0.7 - line * 1.2),
            Vec2::new(p1.x - px, p1.y - px * 0.7),
        );
        button(d, a, b, px, label, true);
        hits.push((a, b, Hit::Close));
        let tip = if view.touch {
            "Tap a tab to turn the page"
        } else {
            "Esc or P keeps flying.  Left and right turn pages"
        };
        d.write(
            Vec2::new(p0.x + px * 2.7, p1.y - px * 0.6 - line * 1.0),
            px * 0.8,
            tip,
            rgba(0x8A6A50, 1.0),
        );

        let area = (
            Vec2::new(p0.x + px * 2.7, top),
            Vec2::new(p1.x - px * 1.2, a.y - px * 0.5),
        );
        match self.page {
            Page::Hoard => self.hoard_page(d, view, ui, area, hits),
            Page::Places => self.places_page(d, view, ui, area, hits),
            Page::People => self.people_page(d, view, ui, area, hits),
            Page::Flying => self.flying_page(d, view, ui, area, hits),
            Page::Stories => self.stories_page(d, view, ui, area, hits),
            Page::Dragons => self.dragons_page(d, view, ui, area, hits),
            Page::Map => self.map_page(d, view, ui, area, hits),
            Page::Help => self.help_page(d, view, ui, area, hits),
            Page::Settings => self.settings_page(d, view, ui, area, hits),
        }

        // A page turning over: the old leaf sweeps away to the left, its edge curling.
        let turn = self.flip.get();
        if turn > 0.0 {
            self.flip.set((turn - 0.08).max(0.0));
            let e = turn * turn * (3.0 - 2.0 * turn);
            let edge = p0.x + (p1.x - p0.x) * e;
            d.rect(
                Vec2::new(edge, p0.y),
                Vec2::new(edge + px * 2.0 * e, p1.y),
                rgba(0x000000, 0.12 * e),
            );
            d.rect(p0, Vec2::new(edge, p1.y), rgba(0xF7EDDA, 1.0));
            d.rect(
                Vec2::new(edge - px * 1.2 * e, p0.y),
                Vec2::new(edge, p1.y),
                rgba(0xE6D5B5, 1.0),
            );
        }
    }

    /// A row that can be picked: a soft band that shows it can be, stronger when it is,
    /// and hittable.
    fn row(
        &self,
        d: &mut Draw,
        i: usize,
        a: Vec2,
        b: Vec2,
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) -> bool {
        let on = self.sel == i;
        if on {
            d.rect(a, b, rgba(RED, 0.2));
            d.rect(a, Vec2::new(a.x + 3.0, b.y), rgba(RED, 0.9));
        } else {
            d.rect(a, b, rgba(0xE6D3B3, 0.45));
        }
        hits.push((a, b, Hit::Row(i)));
        on
    }

    /// A Show me button for map group `key`, its right edge at `right`, if the group is
    /// there. How wide it is, or zero.
    fn show_me_button(
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        key: Option<&str>,
        right: Vec2,
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) -> f32 {
        let Some(g) = key.and_then(|k| view.hoard?.group(k)) else {
            return 0.0;
        };
        let px = ui.px * 0.85;
        let label = "Show me";
        let bw = crate::font::get().measure(px, label) + px * 1.6;
        let (a, b) = (
            Vec2::new(right.x - bw, right.y),
            Vec2::new(right.x, right.y + px * 1.7),
        );
        button(d, a, b, px, label, false);
        hits.push((a, b, Hit::ShowMe(g)));
        bw
    }

    fn help_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x7A5A44, 1.0);
        let mut y = a.y;
        let lead = "You're a little dragon with a big sky. Go anywhere!";
        for l in crate::font::get()
            .wrap(px, lead, b.x - a.x)
            .into_iter()
            .take(2)
        {
            d.write(Vec2::new(a.x, y), px, &l, ink);
            y += line;
        }
        y += line * 0.4;
        let col = a.x + (b.x - a.x) * 0.42;
        let pairs: Vec<(String, String)> = if view.touch {
            [
                ("Left side", "Drag to steer"),
                ("Flap", "Tap, or hold to keep flapping"),
                ("Dive", "Hold to tuck your wings"),
                ("Brake", "Hold to slow down, land or turn tight"),
                ("Brake and Flap", "Hover in place"),
                ("Fire", "Hold to breathe fire. Light the beacons!"),
                ("On the ground", "Push the stick all the way to run"),
                ("Tap a glowing spot", "Fly there and land"),
                ("Something glinting?", "Land right beside it to pick it up"),
                ("A beautiful spot?", "Land there for a postcard"),
                ("Somebody nearby?", "Walk up and tap their bubble to talk"),
                ("Lost?", "The Map page shows where things are"),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
        } else {
            Vec::new()
        };
        if !view.touch {
            // Controls down the left half, other tips down the right.
            let half = (b.x - a.x) * 0.5;
            let keys_x = a.x + half * 0.5;
            let top = y;
            for act in Action::ALL {
                d.write(Vec2::new(a.x, y), px * 0.9, action_text(act), ink);
                let mut keys: Vec<String> = Vec::new();
                for (k, _) in view.binds.keys.iter().filter(|(_, x)| *x == act) {
                    let l = key_label(k);
                    if !keys.contains(&l) {
                        keys.push(l);
                    }
                }
                d.write(
                    Vec2::new(keys_x, y),
                    px * 0.9,
                    &keys.join("  "),
                    rgba(RED, 1.0),
                );
                y += line * 0.95;
            }
            d.write(
                Vec2::new(a.x, y),
                px * 0.8,
                "Change any key on the Settings page",
                soft,
            );
            // The gamepad goes under the controls, the other tips down the right.
            y += line * 1.2;
            d.write(Vec2::new(a.x, y), px * 0.8, "Gamepad", soft);
            y += line * 0.68;
            for l in crate::font::get().wrap(
                px * 0.85,
                "Stick, A flap, X fire, B brake, Y talk, triggers dive",
                half - px,
            ) {
                d.write(Vec2::new(a.x, y), px * 0.85, &l, ink);
                y += line * 0.8;
            }
            let mut more = vec![
                ("I J K L or right drag", "Look around"),
                ("Click a glowing spot", "Fly there and land"),
                ("Something glinting?", "Land right beside it to pick it up"),
                ("A beautiful spot?", "Land there for a postcard"),
                (
                    "Somebody nearby?",
                    "Walk up and press Enter or T, or click their bubble",
                ),
                ("Lost?", "The Map page shows where things are"),
            ];
            more.push(if view.hud {
                ("U, M and N", "Sound, big map and notes")
            } else {
                ("U", "Sound on or off")
            });
            let mut ry = top;
            let rx = a.x + half + px * 0.5;
            for (key, what) in more {
                d.write(Vec2::new(rx, ry), px * 0.8, key, soft);
                ry += line * 0.68;
                for l in crate::font::get().wrap(px * 0.85, what, half - px) {
                    d.write(Vec2::new(rx, ry), px * 0.85, &l, ink);
                    ry += line * 0.8;
                }
                ry += line * 0.2;
            }
            about(d, ui, a, b, hits);
        }
        for (k, v) in pairs {
            d.write(Vec2::new(a.x, y), px, &k, rgba(RED, 1.0));
            d.write(Vec2::new(col, y), px, &v, ink);
            y += line;
        }
        if view.touch {
            about(d, ui, a, b, hits);
        }
    }

    fn people_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let mut y = a.y;
        if hoard.people.is_empty() {
            for l in f.wrap(
                px,
                "Nobody yet. Walk up to anyone in town and say hello. Some of them could use a dragon's help.",
                b.x - a.x,
            ) {
                d.write(Vec2::new(a.x, y), px, &l, soft);
                y += line;
            }
            return;
        }
        // Towns helped, a heart a favor.
        if !hoard.towns.is_empty() {
            let mut x = a.x;
            for (town, n) in &hoard.towns {
                let w = d.write(Vec2::new(x, y), px * 0.85, town, ink);
                x += w + px * 0.4;
                for _ in 0..*n {
                    heart(d, Vec2::new(x + px * 0.4, y + px * 0.5), px * 0.4);
                    x += px * 0.95;
                }
                x += px * 1.2;
                if x > b.x - px * 8.0 {
                    x = a.x;
                    y += line;
                }
            }
            y += line * 1.2;
        }
        let detail_h = line * 3.0;
        let bottom = b.y - detail_h - px * 0.4;
        let rows = (((bottom - y) / line).floor() as usize).max(1);
        let first = (self.sel + 1).saturating_sub(rows);
        for (i, p) in hoard.people.iter().enumerate().skip(first).take(rows) {
            let (r0, r1) = (
                Vec2::new(a.x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(a.x, y), px * 0.9, &p.name, ink);
            let tw = f.measure(px * 0.8, &p.town);
            d.write(
                Vec2::new(a.x + (b.x - a.x) * 0.36, y + px * 0.1),
                px * 0.8,
                &p.town,
                soft,
            );
            let _ = tw;
            if p.helped {
                heart(d, Vec2::new(b.x - px, y + px * 0.5), px * 0.4);
            }
            y += line;
        }
        let (d0, _) = (
            Vec2::new(a.x - px * 0.4, b.y - detail_h),
            Vec2::new(b.x, b.y),
        );
        d.rect(d0, b, rgba(TAN, 0.7));
        if let Some(p) = hoard.people.get(self.sel) {
            let mut ty = d0.y + px * 0.4;
            d.write(Vec2::new(a.x, ty), px, &p.name, rgba(RED, 1.0));
            Self::show_me_button(
                d,
                view,
                ui,
                p.mark.as_deref(),
                Vec2::new(b.x - px * 0.4, ty - px * 0.1),
                hits,
            );
            ty += line;
            for l in f
                .wrap(px * 0.9, &p.status, b.x - a.x - px)
                .into_iter()
                .take(2)
            {
                d.write(Vec2::new(a.x, ty), px * 0.9, &l, ink);
                ty += line * 0.9;
            }
        }
    }

    fn flying_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let mut y = a.y;
        if hoard.flying.is_empty() {
            for l in f.wrap(
                px,
                "Nothing yet. Fly under bridges, through arches, low over the waves, or hitch a ride on something that moves. Good flying gets a name. Townsfolk know of courses and races, too.",
                b.x - a.x,
            ) {
                d.write(Vec2::new(a.x, y), px, &l, soft);
                y += line;
            }
            return;
        }
        let stunts = hoard.flying.iter().filter(|r| r.kind == "Stunt").count();
        if stunts > 0 {
            let s = if stunts == 1 { "" } else { "s" };
            d.write(
                Vec2::new(a.x, y),
                px * 0.85,
                &format!("{stunts} stunt{s} named"),
                soft,
            );
            y += line * 1.1;
        }
        let detail_h = line * 3.0;
        let bottom = b.y - detail_h - px * 0.4;
        let rows = (((bottom - y) / line).floor() as usize).max(1);
        let first = (self.sel + 1).saturating_sub(rows);
        for (i, r) in hoard.flying.iter().enumerate().skip(first).take(rows) {
            let (r0, r1) = (
                Vec2::new(a.x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(a.x, y), px * 0.9, &r.name, ink);
            d.write(
                Vec2::new(a.x + (b.x - a.x) * 0.52, y + px * 0.1),
                px * 0.8,
                &r.kind,
                soft,
            );
            let sw = f.measure(px * 0.8, &r.status);
            let color = if r.done { rgba(GOLD, 1.0) } else { soft };
            d.write(
                Vec2::new(b.x - sw - px * 0.4, y + px * 0.1),
                px * 0.8,
                &r.status,
                color,
            );
            y += line;
        }
        let d0 = Vec2::new(a.x - px * 0.4, b.y - detail_h);
        d.rect(d0, b, rgba(TAN, 0.7));
        if let Some(r) = hoard.flying.get(self.sel) {
            let mut ty = d0.y + px * 0.4;
            d.write(Vec2::new(a.x, ty), px, &r.name, rgba(RED, 1.0));
            Self::show_me_button(
                d,
                view,
                ui,
                r.mark.as_deref(),
                Vec2::new(b.x - px * 0.4, ty - px * 0.1),
                hits,
            );
            ty += line;
            for l in f
                .wrap(px * 0.9, &r.line, b.x - a.x - px)
                .into_iter()
                .take(2)
            {
                d.write(Vec2::new(a.x, ty), px * 0.9, &l, ink);
                ty += line * 0.9;
            }
        }
    }

    fn dragons_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let mut y = a.y;
        for l in f.wrap(
            px * 0.85,
            "Other dragons live out on the islands. Find them, and see what they would like to do with you.",
            b.x - a.x,
        ) {
            d.write(Vec2::new(a.x, y), px * 0.85, &l, soft);
            y += line * 0.9;
        }
        y += line * 0.3;
        for (i, k) in hoard.kin.iter().enumerate() {
            let (r0, r1) = (
                Vec2::new(a.x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(a.x, y), px * 0.9, &k.name, ink);
            let sw = f.measure(px * 0.8, &k.status);
            d.write(
                Vec2::new(b.x - sw - px * 0.4, y + px * 0.1),
                px * 0.8,
                &k.status,
                soft,
            );
            y += line;
        }
        let Some(k) = hoard.kin.get(self.sel) else {
            return;
        };
        y += line * 0.4;
        d.rect(Vec2::new(a.x - px * 0.4, y - px * 0.3), b, rgba(TAN, 0.7));
        Self::show_me_button(
            d,
            view,
            ui,
            k.mark.as_deref(),
            Vec2::new(b.x - px * 0.4, b.y - px * 2.2),
            hits,
        );
        for l in f
            .wrap(px * 0.9, &k.about, b.x - a.x - px)
            .into_iter()
            .take(4)
        {
            d.write(Vec2::new(a.x, y), px * 0.9, &l, ink);
            y += line * 0.9;
        }
        y += line * 0.2;
        for done in &k.done {
            if y > b.y - line * 2.4 {
                break;
            }
            d.rect(
                Vec2::new(a.x, y + px * 0.25),
                Vec2::new(a.x + px * 0.5, y + px * 0.75),
                rgba(GOLD, 1.0),
            );
            d.write(Vec2::new(a.x + px * 0.9, y), px * 0.85, done, ink);
            y += line * 0.85;
        }
    }

    fn stories_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let mut y = a.y;
        for l in f.wrap(
            px * 0.85,
            "Stories change the islands for a while, only once you start one. Set one aside any time and everything is as it was, with your progress kept.",
            b.x - a.x,
        ) {
            d.write(Vec2::new(a.x, y), px * 0.85, &l, soft);
            y += line * 0.9;
        }
        y += line * 0.3;
        for (i, s) in hoard.stories.iter().enumerate() {
            let (r0, r1) = (
                Vec2::new(a.x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(a.x, y), px * 0.9, &s.name, ink);
            let sw = f.measure(px * 0.8, &s.status);
            let color = if s.active { rgba(RED, 1.0) } else { soft };
            d.write(
                Vec2::new(b.x - sw - px * 0.4, y + px * 0.1),
                px * 0.8,
                &s.status,
                color,
            );
            y += line;
        }
        let Some(s) = hoard.stories.get(self.sel) else {
            return;
        };
        y += line * 0.4;
        d.rect(Vec2::new(a.x - px * 0.4, y - px * 0.3), b, rgba(TAN, 0.7));
        Self::show_me_button(
            d,
            view,
            ui,
            s.mark.as_deref(),
            Vec2::new(b.x - px * 0.4, b.y - px * 2.2),
            hits,
        );
        for l in f
            .wrap(px * 0.9, &s.about, b.x - a.x - px)
            .into_iter()
            .take(4)
        {
            d.write(Vec2::new(a.x, y), px * 0.9, &l, ink);
            y += line * 0.9;
        }
        y += line * 0.2;
        // Each region, with a lit lamp or a dark one.
        let mut x = a.x;
        for (name, lit) in &s.regions {
            let w = f.measure(px * 0.8, name) + px * 1.6;
            if x + w > b.x {
                x = a.x;
                y += line * 0.9;
            }
            let c = if *lit { GOLD } else { 0x6A5A50 };
            d.rect(
                Vec2::new(x, y + px * 0.2),
                Vec2::new(x + px * 0.6, y + px * 0.8),
                rgba(c, 1.0),
            );
            d.write(Vec2::new(x + px * 0.9, y), px * 0.8, name, ink);
            x += w + px * 0.6;
        }
        y += line * 1.1;
        let tip = match (s.active, view.touch) {
            (true, false) => {
                "Enter sets it aside. The lights come back until you pick it up again."
            }
            (true, true) => {
                "Tap it to set it aside. The lights come back until you pick it up again."
            }
            (false, false) if s.status == "Told" => "Enter tells it again from the start.",
            (false, true) if s.status == "Told" => "Tap it to tell it again from the start.",
            (false, false) => "Enter starts it, or picks it up where you left off.",
            (false, true) => "Tap it to start, or pick it up where you left off.",
        };
        if y < b.y - line * 0.5 {
            d.write(Vec2::new(a.x, y), px * 0.8, tip, soft);
        }
    }

    fn places_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let cards = &hoard.postcards;
        if let Some(c) = self.zoom.and_then(|i| cards.get(i)) {
            // As big as the page allows, with its name and line beneath.
            let room = Vec2::new(b.x - a.x, b.y - a.y - line * 2.4);
            let pw = room.x.min(room.y * 16.0 / 9.0);
            let p0 = Vec2::new(a.x + (room.x - pw) * 0.5, a.y);
            let p1 = p0 + Vec2::new(pw, pw * 9.0 / 16.0);
            d.rect(p0 + 5.0, p1 + 9.0, rgba(0x000000, 0.2));
            d.rect(p0 - 6.0, p1 + 6.0, rgba(0xFFFFFF, 1.0));
            picture(d, c.pic, p0, p1);
            let mut ty = p1.y + px * 0.6;
            d.write(Vec2::new(p0.x, ty), px * 1.1, &c.name, rgba(RED, 1.0));
            let iw = f.measure(px * 0.8, &c.island);
            d.write(
                Vec2::new(p1.x - iw, ty + px * 0.2),
                px * 0.8,
                &c.island,
                soft,
            );
            ty += line;
            for l in f.wrap(px * 0.85, &c.line, pw).into_iter().take(1) {
                d.write(Vec2::new(p0.x, ty), px * 0.85, &l, ink);
            }
            hits.push((a, b, Hit::Zoom));
            return;
        }
        let mut y = a.y;
        d.write(
            Vec2::new(a.x, y),
            px,
            &format!("Postcards: {}", cards.len()),
            ink,
        );
        y += line * 1.3;
        let detail_h = line * 3.0;
        let bottom = b.y - detail_h - px * 0.4;
        if cards.is_empty() {
            for l in f.wrap(
                px,
                "No postcards yet. Land at a beautiful spot, a summit, a waterfall, a lighthouse, and the view becomes a postcard.",
                b.x - a.x,
            ) {
                d.write(Vec2::new(a.x, y), px, &l, soft);
                y += line;
            }
            return;
        }
        // A grid of pictures, scrolled so the picked one shows.
        let cols = 4;
        let gap = px * 0.8;
        let cw = (b.x - a.x - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = cw * 9.0 / 16.0 + line * 1.1;
        let rows = (((bottom - y) / (ch + gap)).floor() as usize).max(1);
        let first_row = (self.sel / cols + 1).saturating_sub(rows);
        for (i, c) in cards
            .iter()
            .enumerate()
            .skip(first_row * cols)
            .take(rows * cols)
        {
            let k = i - first_row * cols;
            let x = a.x + (k % cols) as f32 * (cw + gap);
            let ty = y + (k / cols) as f32 * (ch + gap);
            let (p0, p1) = (Vec2::new(x, ty), Vec2::new(x + cw, ty + cw * 9.0 / 16.0));
            let on = self.row(d, i, p0 - 4.0, Vec2::new(p1.x + 4.0, ty + ch), hits);
            d.rect(
                p0 - 3.0,
                p1 + 3.0,
                rgba(if on { RED } else { 0xFFFFFF }, 1.0),
            );
            picture(d, c.pic, p0, p1);
            let mut name = c.name.clone();
            while f.measure(px * 0.8, &name) > cw && name.chars().count() > 3 {
                name.pop();
            }
            d.write(Vec2::new(x, p1.y + px * 0.25), px * 0.8, &name, ink);
        }
        let (d0, d1) = (
            Vec2::new(a.x - px * 0.4, b.y - detail_h),
            Vec2::new(b.x, b.y),
        );
        d.rect(d0, d1, rgba(TAN, 0.7));
        if let Some(c) = cards.get(self.sel) {
            let mut ty = d0.y + px * 0.4;
            d.write(Vec2::new(a.x, ty), px, &c.name, rgba(RED, 1.0));
            let tip = if view.touch {
                "Tap again to see it big"
            } else {
                "Enter to see it big"
            };
            let tw = f.measure(px * 0.75, tip);
            d.write(
                Vec2::new((a.x + b.x - tw) * 0.5, ty + px * 0.2),
                px * 0.75,
                tip,
                soft,
            );
            let iw = f.measure(px * 0.8, &c.island);
            d.write(
                Vec2::new(b.x - iw - px * 0.4, ty),
                px * 0.8,
                &c.island,
                soft,
            );
            ty += line;
            for l in f
                .wrap(px * 0.9, &c.line, b.x - a.x - px)
                .into_iter()
                .take(2)
            {
                d.write(Vec2::new(a.x, ty), px * 0.9, &l, ink);
                ty += line * 0.9;
            }
        }
        let _ = d1;
    }

    fn settings_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let f = crate::font::get();
        let on_off = |on: bool| if on { "On" } else { "Off" };
        let mut rows = vec![
            ("Sound".to_string(), on_off(!view.muted).to_string()),
            (
                "Pause when I switch away".to_string(),
                on_off(view.pause_away).to_string(),
            ),
            (
                "Time of day".to_string(),
                time_name(view.hold_hour).to_string(),
            ),
        ];
        if view.hoard.is_some() {
            rows.push(("Save my game to a file".into(), String::new()));
            rows.push(("Load a game from a file".into(), String::new()));
        }
        let half = if view.touch {
            b.x - a.x
        } else {
            (b.x - a.x) * 0.48
        };
        let col = a.x + half * 0.74;
        let mut y = a.y;
        for (i, (k, v)) in rows.iter().enumerate() {
            let (r0, r1) = (
                Vec2::new(a.x - px * 0.4, y - px * 0.3),
                Vec2::new(a.x + half, y + line * 1.2 - px * 0.3),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(a.x, y), px * 0.9, k, ink);
            d.write(Vec2::new(col, y), px * 0.9, v, rgba(RED, 1.0));
            y += line * 1.4;
        }
        if view.hoard.is_some() {
            y += line * 0.3;
            for l in f.wrap(
                px * 0.85,
                "Your game keeps itself as you play, right where you left off. A saved file can come back later, here or on another computer.",
                half,
            ) {
                d.write(Vec2::new(a.x, y), px * 0.85, &l, soft);
                y += line * 0.85;
            }
        }
        if view.touch {
            return;
        }
        // Every key, picked and changed with Enter.
        let base = rows.len();
        let x = a.x + half + px * 1.4;
        let keys_x = x + (b.x - x) * 0.5;
        let mut y = a.y;
        let lead = if self.waiting {
            (
                "Press the new key, or Esc to keep the old one",
                rgba(RED, 1.0),
            )
        } else {
            ("Keys: pick one and press Enter to change it", soft)
        };
        d.write(Vec2::new(x, y), px * 0.8, lead.0, lead.1);
        y += line * 0.95;
        for (k, act) in Action::ALL.into_iter().enumerate() {
            let i = base + k;
            let (r0, r1) = (
                Vec2::new(x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line * 0.95 - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            d.write(Vec2::new(x, y), px * 0.85, action_text(act), ink);
            let mut keys: Vec<String> = Vec::new();
            for (k, _) in view.binds.keys.iter().filter(|(_, x)| *x == act) {
                let l = key_label(k);
                if !keys.contains(&l) {
                    keys.push(l);
                }
            }
            let shown = if self.waiting && i == self.sel {
                "Press a key...".to_string()
            } else {
                keys.join("  ")
            };
            d.write(Vec2::new(keys_x, y), px * 0.85, &shown, rgba(RED, 1.0));
            y += line * 0.95;
        }
        let i = base + Action::ALL.len();
        let (r0, r1) = (
            Vec2::new(x - px * 0.4, y - px * 0.15),
            Vec2::new(b.x, y + line * 0.95 - px * 0.15),
        );
        self.row(d, i, r0, r1, hits);
        d.write(Vec2::new(x, y), px * 0.85, "Put all keys back", soft);
    }

    fn hoard_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let mut y = a.y;
        let gold = format!("{} gold", hoard.gold);
        let gw = f.measure(px * 1.2, &gold);
        d.write(
            Vec2::new(b.x - gw, y - px * 0.1),
            px * 1.2,
            &gold,
            rgba(GOLD, 1.0),
        );
        let lair = match &hoard.lair {
            Some(l) => format!("Lair: {l}"),
            None => "No lair yet. Land in a cozy cave to make it home.".into(),
        };
        let lw = d.write(Vec2::new(a.x, y), px * 0.9, &lair, ink);
        if hoard.lair.is_none() {
            Self::show_me_button(
                d,
                view,
                ui,
                Some("lairs"),
                Vec2::new((a.x + lw + px * 7.5).min(b.x - gw - px), y - px * 0.25),
                hits,
            );
        }
        y += line * 1.4;
        // The picked thing's story or hint, in a box at the bottom.
        let detail_h = line * 3.4;
        let bottom = b.y - detail_h - px * 0.4;
        let mut detail: Option<(String, String, Option<String>)> = None;
        // Everything is laid out first, top down from zero: the sets two to a row, then
        // the treasures. Then it scrolls so the picked thing shows, and only what fits
        // whole between the lair line and the detail box is drawn.
        enum Item<'h> {
            Head(&'h str, String),
            Piece(usize, &'h SetRow, usize),
            Treasures(usize),
            Treasure(usize, usize),
            Empty,
        }
        let mut items: Vec<(Vec2, Vec2, Item)> = Vec::new();
        let s = px * 2.1;
        let gap = px * 0.3;
        let half = (b.x - a.x) * 0.5;
        let mut i = 0;
        let mut ly = 0.0f32;
        for pair in hoard.sets.chunks(2) {
            let mut tallest = 0.0f32;
            for (c, set) in pair.iter().enumerate() {
                let x0 = a.x + c as f32 * half;
                let have = set.pieces.iter().filter(|p| p.1).count();
                items.push((
                    Vec2::new(x0, ly),
                    Vec2::new(x0 + half, ly + line * 0.85),
                    Item::Head(&set.name, format!("{have} of {}", set.pieces.len())),
                ));
                let mut y = ly + line * 0.85;
                let mut x = x0;
                for k in 0..set.pieces.len() {
                    if x + s > x0 + half - gap && x > x0 {
                        x = x0;
                        y += s + gap;
                    }
                    items.push((
                        Vec2::new(x, y),
                        Vec2::new(x + s, y + s),
                        Item::Piece(i, set, k),
                    ));
                    x += s + gap;
                    i += 1;
                }
                tallest = tallest.max(y + s - ly);
            }
            ly += tallest + line * 0.4;
        }
        let pieces = i;
        let empty =
            hoard.treasures.is_empty() && hoard.sets.iter().all(|s| s.pieces.iter().all(|p| !p.1));
        if empty {
            items.push((
                Vec2::new(a.x, ly),
                Vec2::new(b.x, ly + line * 2.0),
                Item::Empty,
            ));
        } else if !hoard.treasures.is_empty() {
            items.push((
                Vec2::new(a.x, ly),
                Vec2::new(b.x, ly + line),
                Item::Treasures(hoard.treasures.len()),
            ));
            ly += line;
            let cw = px * 4.6;
            let cols = (((b.x - a.x) / cw).floor() as usize).max(1);
            let cw = (b.x - a.x) / cols as f32;
            let ch = s + px * 1.75;
            for k in 0..hoard.treasures.len() {
                let x = a.x + (k % cols) as f32 * cw;
                let ty = ly + (k / cols) as f32 * ch;
                items.push((
                    Vec2::new(x, ty),
                    Vec2::new(x + cw - px * 0.3, ty + ch - px * 0.2),
                    Item::Treasure(pieces + k, k),
                ));
            }
        }
        // Scroll just far enough for the picked one to show whole.
        let room = bottom - y;
        let scroll = items
            .iter()
            .find_map(|(_, p1, it)| match it {
                Item::Piece(n, ..) | Item::Treasure(n, _) if *n == self.sel => Some(p1.y),
                _ => None,
            })
            .map_or(0.0, |y1| (y1 + px * 0.3 - room).max(0.0));
        let (mut above, mut below) = (false, false);
        for (p0, p1, it) in &items {
            let (p0, p1) = (
                Vec2::new(p0.x, p0.y - scroll + y),
                Vec2::new(p1.x, p1.y - scroll + y),
            );
            if p0.y < y - 1.0 {
                above = true;
                continue;
            }
            if p1.y > bottom + 1.0 {
                below = true;
                continue;
            }
            match it {
                Item::Head(name, count) => {
                    let mut name = name.to_string();
                    let cw = f.measure(px * 0.85, count) + px * 0.6;
                    while f.measure(px * 0.85, &name) > half - cw - px && name.chars().count() > 3 {
                        name.pop();
                    }
                    let nw = d.write(p0, px * 0.85, &name, ink);
                    d.write(
                        p0 + Vec2::new(nw + px * 0.5, 0.0),
                        px * 0.85,
                        count,
                        rgba(GOLD, 1.0),
                    );
                }
                Item::Piece(n, set, k) => {
                    let (name, found, text, kind) = &set.pieces[*k];
                    let on = self.row(d, *n, p0 - 3.0, p1 + 3.0, hits);
                    d.rect(p0, p1, rgba(0xC9A57A, 1.0));
                    d.rect(
                        p0 + 2.0,
                        p1 - 2.0,
                        rgba(if *found { 0xFFF1C9 } else { 0xF3E7D2 }, 1.0),
                    );
                    if *found {
                        thumb(d, kind, p0 + 3.0, p1 - 3.0, rgba(0xFFFFFF, 1.0));
                    } else {
                        // A shadow of what is missing, with a question mark.
                        thumb(d, kind, p0 + 3.0, p1 - 3.0, rgba(0x8A6A50, 0.3));
                        let qw = f.measure(px * 0.9, "?");
                        d.write(
                            Vec2::new(p0.x + (s - qw) * 0.5, p0.y + (s - px * 1.2) * 0.5),
                            px * 0.9,
                            "?",
                            soft,
                        );
                    }
                    if on {
                        detail = Some(if *found {
                            (name.clone(), text.clone(), None)
                        } else {
                            (
                                "Still out there".into(),
                                format!("Hint: {text}"),
                                set.mark.clone(),
                            )
                        });
                    }
                }
                Item::Treasures(n) => {
                    d.write(p0, px * 0.85, &format!("Treasures ({n})"), ink);
                }
                Item::Treasure(n, k) => {
                    let (name, story, kind) = &hoard.treasures[*k];
                    let on = self.row(d, *n, p0, p1, hits);
                    let cw = p1.x - p0.x;
                    let t0 = Vec2::new(p0.x + (cw - s) * 0.5, p0.y + px * 0.1);
                    thumb(d, kind, t0, t0 + s, rgba(0xFFFFFF, 1.0));
                    let mut ny = t0.y + s - px * 0.1;
                    for l in f.wrap(px * 0.62, name, cw - px * 0.3).into_iter().take(2) {
                        let nw = f.measure(px * 0.62, &l);
                        d.write(Vec2::new(p0.x + (cw - nw) * 0.5, ny), px * 0.62, &l, ink);
                        ny += px * 0.78;
                    }
                    if on {
                        detail = Some((name.clone(), story.clone(), None));
                    }
                }
                Item::Empty => {
                    let mut ty = p0.y;
                    for l in f.wrap(
                        px,
                        "Nothing here yet. Things glint all over the islands. Land right beside one to pick it up.",
                        b.x - a.x,
                    ) {
                        d.write(Vec2::new(a.x, ty), px, &l, soft);
                        ty += line;
                    }
                }
            }
        }
        // Little arrows say there is more above or below.
        let mark = |d: &mut Draw, at: Vec2, up: bool| {
            let (tip, base) = if up { (-1.0, 1.0) } else { (1.0, -1.0) };
            d.tri(
                at + Vec2::new(0.0, tip * px * 0.4),
                at + Vec2::new(-px * 0.5, base * px * 0.4),
                at + Vec2::new(px * 0.5, base * px * 0.4),
                rgba(RED, 0.8),
            );
        };
        if above {
            mark(d, Vec2::new(b.x - px * 0.6, y - px * 0.1), true);
        }
        if below {
            mark(d, Vec2::new(b.x - px * 0.6, bottom - px * 0.2), false);
        }
        // The detail box.
        let (d0, d1) = (
            Vec2::new(a.x - px * 0.4, b.y - detail_h),
            Vec2::new(b.x, b.y),
        );
        d.rect(d0, d1, rgba(TAN, 0.7));
        let (title, body, mark) = detail.unwrap_or_else(|| {
            (
                String::new(),
                if view.touch {
                    "Tap anything here to read about it.".into()
                } else {
                    "Pick anything here to read about it.".into()
                },
                None,
            )
        });
        let mut ty = d0.y + px * 0.4;
        if !title.is_empty() {
            d.write(Vec2::new(a.x, ty), px, &title, rgba(RED, 1.0));
            Self::show_me_button(
                d,
                view,
                ui,
                mark.as_deref(),
                Vec2::new(b.x - px * 0.4, ty - px * 0.1),
                hits,
            );
            ty += line;
        }
        for l in f.wrap(px * 0.9, &body, b.x - a.x - px).into_iter().take(2) {
            d.write(Vec2::new(a.x, ty), px * 0.9, &l, ink);
            ty += line * 0.9;
        }
    }

    /// The map, with whatever group is picked marked on it.
    fn map_page(
        &self,
        d: &mut Draw,
        view: &View,
        ui: &Ui,
        (a, b): (Vec2, Vec2),
        hits: &mut Vec<(Vec2, Vec2, Hit)>,
    ) {
        let Some(hoard) = view.hoard else { return };
        let f = crate::font::get();
        let (px, line) = (ui.px, ui.line);
        let ink = rgba(INK, 1.0);
        let soft = rgba(0x8A6A50, 1.0);
        let side = (b.y - a.y).min((b.x - a.x) * 0.56);
        let (m0, m1) = (a, a + Vec2::splat(side));
        d.rect(m0 - 4.0, m1 + 4.0, rgba(0xC9A57A, 1.0));
        let map = view.map;
        if map.px > 0.0 {
            d.quad(m0, m1, Vec2::ZERO, Vec2::splat(map.px), rgba(0xFFFFFF, 1.0));
        } else {
            d.rect(m0, m1, rgba(0x6FA7D6, 1.0));
            d.write(
                m0 + Vec2::splat(px),
                px * 0.9,
                "Drawing the map...",
                rgba(0xFFFFFF, 1.0),
            );
        }
        let bd = map.bounds;
        let span = Vec2::new(bd[2] - bd[0], bd[3] - bd[1]).max(Vec2::ONE);
        let to = |p: Vec2| m0 + (p - Vec2::new(bd[0], bd[1])) / span * side;
        let group = hoard.marks.get(self.sel);
        if let Some(g) = group {
            for m in g.spots.iter().filter(|m| m.done) {
                let c = to(m.at);
                diamond(d, c, px * 0.32, rgba(0x6A5A50, 0.85));
            }
            let left: Vec<&Mark> = g.spots.iter().filter(|m| !m.done).collect();
            for m in &left {
                let c = to(m.at);
                diamond(d, c, px * 0.62, rgba(0x2A1E14, 0.9));
                diamond(d, c, px * 0.45, rgba(0xFFD54A, 1.0));
            }
            // A few marks get their names beside them.
            if left.len() <= 4 {
                for m in &left {
                    let c = to(m.at) + Vec2::new(px * 0.7, -px * 0.6);
                    let w = f.measure(px * 0.7, &m.label);
                    let c = Vec2::new(c.x.min(m1.x - w - 4.0), c.y.max(m0.y + 2.0));
                    d.rect(
                        c - Vec2::new(3.0, 1.0),
                        c + Vec2::new(w + 3.0, px * 1.05),
                        rgba(PAPER, 0.85),
                    );
                    d.write(c, px * 0.7, &m.label, ink);
                }
            }
        }
        d.arrow(
            to(map.me),
            Vec2::new(map.heading.sin(), map.heading.cos()),
            px * 0.55,
        );
        // What it can show, down the right.
        let x = m1.x + px * 1.4;
        let mut y = a.y;
        d.write(Vec2::new(x, y), px * 0.8, "Show on the map", soft);
        y += line * 0.9;
        let detail_h = line * 3.6;
        let bottom = b.y - detail_h - px * 0.3;
        let rows = (((bottom - y) / line).floor() as usize).max(1);
        let first = (self.sel + 1).saturating_sub(rows);
        for (i, g) in hoard.marks.iter().enumerate().skip(first).take(rows) {
            let (r0, r1) = (
                Vec2::new(x - px * 0.4, y - px * 0.15),
                Vec2::new(b.x, y + line - px * 0.15),
            );
            self.row(d, i, r0, r1, hits);
            let left = g.spots.iter().filter(|m| !m.done).count();
            let count = if g.spots.is_empty() {
                "None now".to_string()
            } else if left == 0 {
                "All done".to_string()
            } else {
                format!("{left} to go")
            };
            let cw = f.measure(px * 0.75, &count);
            let mut name = g.name.clone();
            while f.measure(px * 0.85, &name) > b.x - x - cw - px * 1.2 && name.chars().count() > 3
            {
                name.pop();
            }
            d.write(Vec2::new(x, y), px * 0.85, &name, ink);
            d.write(
                Vec2::new(b.x - cw - px * 0.3, y + px * 0.1),
                px * 0.75,
                &count,
                if left == 0 { rgba(GOLD, 1.0) } else { soft },
            );
            y += line;
        }
        if hoard.marks.is_empty() {
            d.write(Vec2::new(x, y), px * 0.85, "Nothing to show yet.", soft);
        }
        let d0 = Vec2::new(x - px * 0.4, b.y - detail_h);
        d.rect(d0, b, rgba(TAN, 0.7));
        let mut ty = d0.y + px * 0.35;
        if let Some(g) = group {
            for l in f
                .wrap(px * 0.8, &g.about, b.x - x - px * 0.4)
                .into_iter()
                .take(3)
            {
                d.write(Vec2::new(x, ty), px * 0.8, &l, ink);
                ty += line * 0.8;
            }
        }
        d.write(
            Vec2::new(x, b.y - line * 0.85),
            px * 0.7,
            "Gold: still to do.  Grey: done.  Red arrow: you.",
            soft,
        );
    }
}

fn draw_card(d: &mut Draw, card: &Card, ui: &Ui, hits: &mut Vec<(Vec2, Vec2, Hit)>) {
    let f = crate::font::get();
    let (w, h) = (d.size.x, d.size.y);
    let (px, line) = (ui.px * 1.1, ui.line * 1.1);
    let buttons: &[(&str, Hit)] = match card.kind {
        CardKind::Find => &[("Into the hoard!", Hit::Yes)],
        CardKind::News => &[("Hooray!", Hit::Yes)],
        CardKind::Lair(_) => &[("Make it my lair", Hit::Yes), ("Not now", Hit::No)],
        CardKind::Postcard(_) => &[("Keep it!", Hit::Yes)],
        CardKind::Say => &[("Bye!", Hit::Yes)],
        CardKind::Carry => &[("Got it!", Hit::Yes)],
        CardKind::Chore => &[("Sure!", Hit::Yes), ("Not right now", Hit::No)],
        CardKind::Offer(_) => &[("I'll help!", Hit::Yes), ("Maybe later", Hit::No)],
        CardKind::Return(_) => &[("Give it back", Hit::Yes), ("Keep it for now", Hit::No)],
        CardKind::Story(_) => &[("I'll light them!", Hit::Yes), ("Maybe later", Hit::No)],
        CardKind::Rider => &[
            ("Show the way", Hit::Yes),
            ("Hop off here", Hit::Alt),
            ("Keep flying", Hit::No),
        ],
        CardKind::Kin(_, KinAsk::Chat) => &[("See you!", Hit::Yes)],
        CardKind::Kin(_, KinAsk::Follow) => {
            &[("Lead the way!", Hit::Yes), ("Maybe later", Hit::No)]
        }
        CardKind::Kin(_, KinAsk::Race) => &[("Ready, set, go!", Hit::Yes), ("Not now", Hit::No)],
        CardKind::Kin(_, KinAsk::Trade) => &[("Swap!", Hit::Yes), ("I'll keep mine", Hit::No)],
        CardKind::Kin(_, KinAsk::Egg) => &[("I'll keep it warm", Hit::Yes)],
    };
    // Wide enough for every button, which shrink if the screen is not.
    let need: f32 = buttons
        .iter()
        .map(|(l, _)| f.measure(px, l) + px * 2.6)
        .sum::<f32>()
        + px * 1.8;
    let bw = (w - 32.0).min((px * 26.0).max(need));
    let bpx = px * ((bw - px * 1.8) / (need - px * 1.8)).min(1.0);
    let inner = bw - px * 2.4;
    let body = f.wrap(px, &card.body, inner);
    // The foot wraps too: a long hint stays on the card.
    let foot = if card.foot.is_empty() {
        Vec::new()
    } else {
        f.wrap(px * 0.9, &card.foot, inner)
    };
    // A postcard shows its picture above the words.
    let pic_h = match card.kind {
        CardKind::Postcard(_) => inner * 9.0 / 16.0 + px,
        _ => 0.0,
    };
    let bh = line * (3.6 + body.len() as f32 + foot.len() as f32 * 0.9) + px * 2.0 + pic_h;
    let p0 = Vec2::new((w - bw) * 0.5, (h - bh) * 0.5);
    let p1 = p0 + Vec2::new(bw, bh);
    d.rect(Vec2::ZERO, d.size, rgba(0x14202C, 0.35));
    d.rect(p0 + 6.0, p1 + 6.0, rgba(0x000000, 0.25));
    d.rect(p0 - 3.0, p1 + 3.0, rgba(GOLD, 1.0));
    d.rect(p0, p1, rgba(PAPER, 1.0));
    let x = p0.x + px * 1.2;
    let mut y = p0.y + px * 0.9;
    d.write(Vec2::new(x, y), px * 1.35, &card.title, rgba(RED, 1.0));
    y += line * 1.5;
    if let CardKind::Postcard(pic) = card.kind {
        let (a, b) = (
            Vec2::new(x, y),
            Vec2::new(x + inner, y + inner * 9.0 / 16.0),
        );
        d.rect(a - 3.0, b + 3.0, rgba(0xFFFFFF, 1.0));
        picture(d, pic, a, b);
        y += pic_h;
    }
    for l in &body {
        d.write(Vec2::new(x, y), px, l, rgba(INK, 1.0));
        y += line;
    }
    for l in &foot {
        d.write(Vec2::new(x, y), px * 0.9, l, rgba(GOLD, 1.0));
        y += line * 0.9;
    }
    let mut bx = p1.x - px * 1.2;
    for (k, (label, hit)) in buttons.iter().enumerate() {
        let lw = f.measure(bpx, label) + bpx * 2.0;
        let (a, b) = (
            Vec2::new(bx - lw, p1.y - px * 0.8 - line * 1.2),
            Vec2::new(bx, p1.y - px * 0.8),
        );
        button(d, a, b, bpx, label, k == 0);
        hits.push((a, b, *hit));
        bx -= lw + bpx * 0.6;
    }
}

/// What a passenger says as you fly, at the foot of the screen. It never pauses.
pub(crate) fn draw_toast(d: &mut Draw, who: &str, text: &str, fade: f32) {
    let f = crate::font::get();
    let (w, h) = (d.size.x, d.size.y);
    let px = (h / 34.0).min(w / 34.0).clamp(11.0, 26.0);
    let small = px * 0.9;
    // Who or what over the line, the line wrapped to at most three rows.
    let most = (w * 0.7).max(px * 16.0).min(w - 24.0 - px * 2.0);
    let lines: Vec<String> = f.wrap(small, text, most).into_iter().take(3).collect();
    let tw = lines
        .iter()
        .map(|l| f.measure(small, l))
        .fold(f.measure(px, who), f32::max);
    let bw = (tw + px * 2.0).min(w - 24.0);
    let bh = px * 1.9 + lines.len() as f32 * small * 1.25;
    let p0 = Vec2::new((w - bw) * 0.5, h - px * 3.3 - bh);
    d.rect(p0, p0 + Vec2::new(bw, bh), rgba(PAPER, 0.92 * fade));
    let x = p0.x + px;
    d.write(Vec2::new(x, p0.y + px * 0.45), px, who, rgba(RED, fade));
    let mut y = p0.y + px * 1.6;
    for l in &lines {
        d.write(Vec2::new(x, y), small, l, rgba(INK, fade));
        y += small * 1.25;
    }
}

/// A button that looks pressable: a face over a darker lip, its label centered.
fn button(d: &mut Draw, a: Vec2, b: Vec2, px: f32, label: &str, primary: bool) {
    let (face, lip, text) = if primary {
        (RED, 0xA8392F, 0xFFFFFF)
    } else {
        (0xF2E2C4, 0xB89A70, INK)
    };
    d.rect(
        a + Vec2::new(0.0, 3.0),
        b + Vec2::new(0.0, 3.0),
        rgba(lip, 1.0),
    );
    d.rect(a, b, rgba(face, 1.0));
    d.rect(a, Vec2::new(b.x, a.y + 2.0), rgba(0xFFFFFF, 0.3));
    let tw = crate::font::get().measure(px, label);
    d.write(
        Vec2::new((a.x + b.x - tw) * 0.5, a.y + (b.y - a.y - px * 1.3) * 0.5),
        px,
        label,
        rgba(text, 1.0),
    );
}

/// A diamond `r` from its middle `c` to each point, for a mark on the map.
fn diamond(d: &mut Draw, c: Vec2, r: f32, color: [f32; 4]) {
    let (n, e, s, w) = (
        c - Vec2::Y * r,
        c + Vec2::X * r,
        c + Vec2::Y * r,
        c - Vec2::X * r,
    );
    d.tri(n, e, s, color);
    d.tri(n, s, w, color);
}

/// Treasure `kind`'s picture in the box `a` to `b`, tinted by `color`.
fn thumb(d: &mut Draw, kind: &str, a: Vec2, b: Vec2, color: [f32; 4]) {
    let Some(i) = crate::treasure::KINDS.iter().position(|k| *k == kind) else {
        return;
    };
    let (x, y) = crate::hud::thumb_slot(i);
    let t = crate::thumbs::THUMB as f32;
    let uv0 = Vec2::new(x as f32, y as f32);
    d.quad(a, b, uv0, uv0 + Vec2::splat(t), color);
}

/// A little heart, `r` across each lobe, centered at `c`.
fn heart(d: &mut Draw, c: Vec2, r: f32) {
    let red = rgba(RED, 1.0);
    for side in [-1.0, 1.0] {
        let lobe = c + Vec2::new(side * r * 0.55, -r * 0.2);
        for k in 0..8 {
            let a0 = k as f32 / 8.0 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / 8.0 * std::f32::consts::TAU;
            d.tri(
                lobe,
                lobe + Vec2::new(a0.cos(), a0.sin()) * r * 0.6,
                lobe + Vec2::new(a1.cos(), a1.sin()) * r * 0.6,
                red,
            );
        }
    }
    d.tri(
        c + Vec2::new(-r * 1.1, -r * 0.05),
        c + Vec2::new(r * 1.1, -r * 0.05),
        c + Vec2::new(0.0, r * 1.05),
        red,
    );
}

/// A postcard's painted picture in the box `a` to `b`, or a plain sky if it has none.
fn picture(d: &mut Draw, pic: Option<usize>, a: Vec2, b: Vec2) {
    match pic {
        Some(i) => {
            let (x, y) = crate::hud::card_slot(i);
            let (w, h) = zawnabout_pic_size();
            let uv0 = Vec2::new(x as f32, y as f32);
            d.quad(a, b, uv0, uv0 + Vec2::new(w, h), rgba(0xFFFFFF, 1.0));
        }
        None => {
            let mid = Vec2::new(b.x, (a.y + b.y) * 0.5);
            d.rect(a, mid, rgba(0x9CCBF0, 1.0));
            d.rect(Vec2::new(a.x, mid.y), b, rgba(0x8FD16A, 1.0));
        }
    }
}

/// A postcard picture's size in pixels.
fn zawnabout_pic_size() -> (f32, f32) {
    (
        crate::hud::CARD_CELL.0 as f32 - 4.0,
        crate::hud::CARD_CELL.1 as f32 - 4.0,
    )
}

/// The about lines at the foot of help: the game runs only here and keeps nothing
/// anywhere else, and where its source is. Returns where the lines end.
fn about(d: &mut Draw, ui: &Ui, a: Vec2, b: Vec2, hits: &mut Vec<(Vec2, Vec2, Hit)>) -> f32 {
    let (px, line) = (ui.px * 0.8, ui.line * 0.8);
    let soft = rgba(0x8A6A50, 1.0);
    let mut y = b.y - line * 2.3;
    d.write(
        Vec2::new(a.x, y),
        px,
        "zawnabout runs right here and never sends anything anywhere. Your game stays with you.",
        soft,
    );
    y += line;
    let lead = "Made in the open, come see how: ";
    let lw = d.write(Vec2::new(a.x, y), px, lead, soft);
    let link = Vec2::new(a.x + lw, y);
    let w = d.write(link, px, SOURCE, rgba(0x2E6FBF, 1.0));
    d.rect(
        link + Vec2::new(0.0, px * 1.05),
        link + Vec2::new(w, px * 1.12),
        rgba(0x2E6FBF, 1.0),
    );
    hits.push((
        link - Vec2::new(4.0, 4.0),
        link + Vec2::new(w + 4.0, line),
        Hit::Source,
    ));
    y + line
}

/// What each action does, for the help page.
fn action_text(a: Action) -> &'static str {
    match a {
        Action::PitchUp => "Nose up",
        Action::PitchDown => "Nose down",
        Action::TurnLeft => "Turn left",
        Action::TurnRight => "Turn right",
        Action::Flap => "Flap",
        Action::Tuck => "Dive",
        Action::Brake => "Brake, land, hover",
        Action::Breath => "Breathe fire",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(binds: &Bindings, hoard: Option<&HoardView>) -> View<'static> {
        // Tests keep the bindings alive for the whole test.
        let binds: &'static Bindings = Box::leak(Box::new(binds.clone()));
        let hoard: Option<&'static HoardView> = hoard.map(|h| &*Box::leak(Box::new(h.clone())));
        View {
            hoard,
            binds,
            muted: false,
            pause_away: true,
            hold_hour: None,
            touch: false,
            hud: true,
            map: MapView::default(),
        }
    }

    /// Esc opens and closes it, pages turn and wrap, and the hoard page is gone with
    /// gameplay off.
    #[test]
    fn opens_turns_and_closes() {
        let b = Bindings::default();
        let hoard = HoardView::default();
        let v = view(&b, Some(&hoard));
        let mut book = Book::default();
        assert_eq!(book.key("KeyW", &v), None);
        assert_eq!(book.key("Escape", &v), Some(vec![]));
        assert!(book.open && book.page == Page::Help);
        book.key("ArrowRight", &v);
        assert_eq!(book.page, Page::Settings);
        book.key("ArrowRight", &v);
        assert_eq!(book.page, Page::Hoard);
        assert_eq!(book.key("Escape", &v), Some(vec![Do::Close]));
        assert!(!book.showing());
        let bare = view(&b, None);
        book.show(&bare);
        assert_eq!(book.page, Page::Help);
        book.key("ArrowLeft", &bare);
        assert_eq!(book.page, Page::Settings);
        book.key("ArrowLeft", &bare);
        assert_eq!(book.page, Page::Help);
    }

    /// Enter on a key in settings waits for its new key, and a lair card answers.
    #[test]
    fn rebinds_and_answers_cards() {
        let b = Bindings::default();
        let v = view(&b, None);
        let mut book = Book::default();
        book.show(&v);
        book.key("ArrowRight", &v);
        assert_eq!(book.page, Page::Settings);
        for _ in 0..4 {
            book.key("ArrowDown", &v);
        }
        assert_eq!(book.key("Enter", &v), Some(vec![]));
        assert_eq!(
            book.key("KeyZ", &v),
            Some(vec![Do::Bind(Action::PitchDown, "KeyZ".into())])
        );
        book.cards.push(Card {
            title: "Den".into(),
            body: String::new(),
            foot: String::new(),
            kind: CardKind::Lair(2),
        });
        assert_eq!(book.key("KeyW", &v), Some(vec![]));
        assert_eq!(book.key("Enter", &v), Some(vec![Do::Claim(2)]));
        assert!(book.cards.is_empty());
    }
}
