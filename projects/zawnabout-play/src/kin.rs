//! Other dragons, and a family of your own. Four named dragons live on the outer islands,
//! each found as a discovery: an old wise one, a show-off who races, a shy young one who
//! has to be coaxed out, and a collector who offers trades. Any of them can lead the way
//! to a secret spot. Once all four are met the old one gives an egg, which hatches when
//! warmed with fire in the lair, and the hatchling grows a little with each first it
//! shares with you. Nobody ever takes anything, and nothing is timed.
//! None of it touches the world: the game flies and draws them.

use glam::Vec3;

use crate::{GiftDef, Play};

/// A named dragon: where it lives, how it looks, and what it does with you.
pub struct KinDef {
    pub id: &'static str,
    pub name: &'static str,
    /// `wise`, `racer`, `shy` or `collector`.
    pub kind: &'static str,
    pub island: &'static str,
    /// Its home perch, on whatever is under `x`, `z` (below `y` when given).
    pub x: f32,
    pub y: Option<f32>,
    pub z: f32,
    /// Multiplies its colors, as `0xRRGGBB`, with 0x808080 leaving them as they are.
    pub tint: u32,
    pub size: f32,
    /// Said the first time you meet.
    pub meet: &'static str,
    /// What the journal says about it.
    pub about: &'static str,
    /// What townsfolk say of it, before you have met.
    pub rumor: &'static str,
    pub lines: &'static [&'static str],
    /// Other spots round its island it likes to sit, as `x, y, z` with y NaN to rest on
    /// whatever is under.
    pub perches: &'static [[f32; 3]],
    /// The way it leads you, stop by stop, to the find `secret` at the end.
    pub way: &'static [[f32; 3]],
    pub secret: &'static str,
    /// Left by your heap once you are friends.
    pub gift: GiftDef,
    /// A racer's finish line: its name, where, and how near counts, with the prize.
    pub race: Option<(&'static str, [f32; 2], f32, GiftDef)>,
    /// A collector's trade: the kind of find it would love, and what it gives for one.
    pub trade: Option<(&'static str, GiftDef)>,
}

/// How many times the shy one has to be found before it stays to talk.
pub const COAX: u8 = 3;

/// The hatchling's firsts, in the order the journal lists them.
pub const FIRSTS: [(&str, &str); 4] = [
    ("flight", "First flight together"),
    ("splash", "First splash"),
    ("fire", "First fire"),
    ("stunt", "First stunt"),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Egg {
    #[default]
    None,
    /// Given, waiting to be warmed in the lair.
    Given,
    Hatched,
}

/// One player's dragons: who is met and what was done together, and the family.
#[derive(Clone, Debug, Default)]
pub struct Kin {
    pub met: Vec<bool>,
    /// Times the shy one was found and flew off.
    pub coaxed: Vec<u8>,
    pub followed: Vec<bool>,
    pub won: Vec<bool>,
    pub traded: Vec<bool>,
    /// Left their gift by your heap.
    pub gifted: Vec<bool>,
    pub egg: Egg,
    pub firsts: [bool; 4],
}

impl Kin {
    pub fn new(n: usize) -> Self {
        Self {
            met: vec![false; n],
            coaxed: vec![0; n],
            followed: vec![false; n],
            won: vec![false; n],
            traded: vec![false; n],
            gifted: vec![false; n],
            egg: Egg::None,
            firsts: [false; 4],
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.met.len());
    }

    /// How big the hatchling is: small at first, a little bigger for each first.
    pub fn hatchling_size(&self) -> f32 {
        0.36 + 0.09 * self.firsts.iter().filter(|f| **f).count() as f32
    }
}

/// What came of meeting or doing something with a dragon.
#[derive(Clone, Debug, PartialEq)]
pub enum KinNews {
    /// Met for the first time.
    Met(usize),
    /// The shy one flew off to its next perch, `n` times so far.
    Shy(usize, u8),
    /// The old one gives the egg.
    Egg,
    Hatched,
    /// The hatchling's first `k`.
    First(usize),
}

impl Play {
    /// Every named dragon's home and perches, resting on what is under them.
    pub fn kin_spots(&self, i: usize) -> &[Vec3] {
        &self.kin_at[i]
    }

    /// Dragon `i` is found. The shy one has to be found a few times: it flies off to the
    /// next perch until it trusts you.
    pub fn find_kin(&mut self, i: usize) -> Option<KinNews> {
        if self.kin.met[i] {
            return None;
        }
        self.changed = true;
        if self.def.kin[i].kind == "shy" && self.kin.coaxed[i] + 1 < COAX {
            self.kin.coaxed[i] += 1;
            return Some(KinNews::Shy(i, self.kin.coaxed[i]));
        }
        self.kin.met[i] = true;
        Some(KinNews::Met(i))
    }

    /// Whether every named dragon is met.
    pub fn all_kin_met(&self) -> bool {
        !self.kin.met.is_empty() && self.kin.met.iter().all(|m| *m)
    }

    /// The old one hands over the egg, once all are met.
    pub fn give_egg(&mut self) -> bool {
        if self.kin.egg != crate::kin::Egg::None || !self.all_kin_met() {
            return false;
        }
        self.kin.egg = Egg::Given;
        self.changed = true;
        true
    }

    /// The egg is warm enough: it hatches.
    pub fn hatch(&mut self) -> bool {
        if self.kin.egg != Egg::Given {
            return false;
        }
        self.kin.egg = Egg::Hatched;
        self.changed = true;
        true
    }

    /// The hatchling's first `name` happens, true the first time.
    pub fn first(&mut self, name: &str) -> Option<usize> {
        if self.kin.egg != Egg::Hatched {
            return None;
        }
        let k = FIRSTS.iter().position(|(n, _)| *n == name)?;
        if self.kin.firsts[k] {
            return None;
        }
        self.kin.firsts[k] = true;
        self.changed = true;
        Some(k)
    }

    /// Dragon `i` led you to its secret.
    pub fn followed(&mut self, i: usize) {
        self.kin.followed[i] = true;
        self.changed = true;
    }

    /// You beat the racer to its finish. True the first time, when the prize comes.
    pub fn beat(&mut self, i: usize) -> bool {
        let first = !self.kin.won[i];
        self.kin.won[i] = true;
        self.changed = true;
        first
    }

    /// The find the collector would take, if the hoard has one of the kind it loves.
    pub fn trade_offer(&self, i: usize) -> Option<usize> {
        let (kind, _) = self.def.kin[i].trade.as_ref()?;
        if self.kin.traded[i] {
            return None;
        }
        self.def
            .finds
            .iter()
            .enumerate()
            .find(|(f, d)| d.kind == *kind && self.found[*f] && !self.returned(*f))
            .map(|(f, _)| f)
    }

    /// Swaps that find for the collector's gift.
    pub fn trade(&mut self, i: usize) -> bool {
        let Some(f) = self.trade_offer(i) else {
            return false;
        };
        self.kin.traded[i] = true;
        self.kin_traded_find[i] = Some(f);
        self.changed = true;
        true
    }

    /// A friend's gift is left by the heap: true the first time, once you have a lair
    /// and have met them.
    pub fn leave_gift(&mut self, i: usize) -> bool {
        if !self.kin.met[i] || self.kin.gifted[i] || self.lair.is_none() {
            return false;
        }
        self.kin.gifted[i] = true;
        self.changed = true;
        true
    }

    /// Whether find `f` went to a collector in a trade.
    pub fn traded_away(&self, f: usize) -> bool {
        self.kin_traded_find.contains(&Some(f))
    }

    /// Gifts from dragons: friends' gifts, race prizes and trades.
    pub fn kin_gifts(&self) -> impl Iterator<Item = &'static GiftDef> + '_ {
        let k = &self.kin;
        self.def.kin.iter().enumerate().flat_map(move |(i, d)| {
            let gift = k.gifted[i].then_some(&d.gift);
            let prize = d.race.as_ref().filter(|_| k.won[i]).map(|r| &r.3);
            let trade = d.trade.as_ref().filter(|_| k.traded[i]).map(|t| &t.1);
            [gift, prize, trade].into_iter().flatten()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLAY;
    use zawnabout_sim::Sim;

    /// The four are all there, one of each kind, and their spots are where they say.
    #[test]
    fn four_kinds() {
        for kind in ["wise", "racer", "shy", "collector"] {
            assert_eq!(
                PLAY.kin.iter().filter(|k| k.kind == kind).count(),
                1,
                "{kind}"
            );
        }
        for k in PLAY.kin {
            assert!(!k.way.is_empty(), "{} leads nowhere", k.id);
            assert!(
                PLAY.finds.iter().any(|f| f.id == k.secret),
                "{}: no find {}",
                k.id,
                k.secret
            );
        }
    }

    /// The shy one has to be found three times, the egg comes once all are met, hatches
    /// once, and each first counts once and makes the hatchling bigger.
    #[test]
    fn meet_hatch_and_grow() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let shy = PLAY.kin.iter().position(|k| k.kind == "shy").expect("shy");
        assert_eq!(play.find_kin(shy), Some(KinNews::Shy(shy, 1)));
        assert_eq!(play.find_kin(shy), Some(KinNews::Shy(shy, 2)));
        assert_eq!(play.find_kin(shy), Some(KinNews::Met(shy)));
        assert_eq!(play.find_kin(shy), None);
        assert!(!play.give_egg());
        for i in 0..PLAY.kin.len() {
            while play.find_kin(i).is_some() {}
        }
        assert!(!play.hatch());
        assert!(play.give_egg());
        assert!(!play.give_egg());
        assert_eq!(play.first("flight"), None);
        assert!(play.hatch());
        let small = play.kin.hatchling_size();
        assert_eq!(play.first("flight"), Some(0));
        assert_eq!(play.first("flight"), None);
        assert!(play.kin.hatchling_size() > small);
    }

    /// The collector takes one find of the kind it loves for its gift, once, and the
    /// find leaves the hoard.
    #[test]
    fn trades_once() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let c = PLAY
            .kin
            .iter()
            .position(|k| k.kind == "collector")
            .expect("collector");
        assert!(play.trade_offer(c).is_none());
        let (kind, gift) = PLAY.kin[c].trade.as_ref().expect("trade");
        let f = PLAY
            .finds
            .iter()
            .position(|f| f.kind == *kind)
            .expect("a find");
        play.found[f] = true;
        assert_eq!(play.trade_offer(c), Some(f));
        assert!(play.trade(c));
        assert!(!play.treasures().any(|t| t.id == PLAY.finds[f].id));
        assert!(play.gifts().any(|g| g.name == gift.name));
        assert!(!play.trade(c));
    }
}
