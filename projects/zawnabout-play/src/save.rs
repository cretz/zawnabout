//! Saves as plain text, so a player can export one, keep it and bring it back:
//!
//! ```text
//! zawnabout save 1
//! [hoard]
//! lair east-hollow
//! found gull-crown
//! [places]
//! postcard high-falls
//! pic high-falls PICTURE
//! [people]
//! met mara
//! asked pip
//! done mara
//! carry pip
//! [flying]
//! stunt under-sea-arch
//! gate kettle-bridges 2
//! course needle-weave
//! tried ferry-race
//! won ferry-race
//! [stories]
//! active lights-out
//! lamp lights-out home 2
//! done lights-out
//! ```
//!
//! A section a layer. Ids the game no longer has are dropped, and sections it does not
//! know are kept as they were, so an older build never loses a newer save's layers.

use std::fmt::Write;

use crate::{Errand, Play, paint};

const HEAD: &str = "zawnabout save 1";
/// The sections this build reads. Any other is kept as it was.
const KNOWN: [&str; 7] = [
    "hoard",
    "places",
    "people",
    "flying",
    "stories",
    "dragons",
    "happenings",
];

impl Play {
    pub fn save(&self) -> String {
        let mut s = format!("{HEAD}\n[hoard]\n");
        if let Some(l) = self.lair {
            let _ = writeln!(s, "lair {}", self.def.lairs[l].id);
        }
        for (d, _) in self.def.finds.iter().zip(&self.found).filter(|(_, f)| **f) {
            let _ = writeln!(s, "found {}", d.id);
        }
        s += "[places]\n";
        for (i, c) in self.def.postcards.iter().enumerate() {
            if self.taken[i] {
                let _ = writeln!(s, "postcard {}", c.id);
            }
            if let Some(pic) = &self.pics[i] {
                let _ = writeln!(s, "pic {} {pic}", c.id);
            }
        }
        s += "[people]\n";
        for (i, l) in self.def.locals.iter().enumerate() {
            if self.met[i] {
                let _ = writeln!(s, "met {}", l.id);
            }
            match self.errands[i] {
                Errand::Accepted => {
                    let _ = writeln!(s, "asked {}", l.id);
                }
                Errand::Done => {
                    let _ = writeln!(s, "done {}", l.id);
                }
                Errand::New => {}
            }
        }
        if let Some(c) = self.carrying {
            let _ = writeln!(s, "carry {}", self.def.locals[c].id);
        }
        if let Some(r) = self.riding {
            let _ = writeln!(s, "ride {}", self.def.locals[r].id);
        }
        s += "[flying]
";
        let f = &self.flying;
        for (d, _) in self.def.stunts.iter().zip(&f.stunts).filter(|(_, d)| **d) {
            let _ = writeln!(s, "stunt {}", d.id);
        }
        for (c, course) in self.def.courses.iter().enumerate() {
            if f.courses[c] {
                let _ = writeln!(s, "course {}", course.id);
                continue;
            }
            for (g, _) in f.gates[c].iter().enumerate().filter(|(_, p)| **p) {
                let _ = writeln!(s, "gate {} {g}", course.id);
            }
        }
        for (i, r) in self.def.races.iter().enumerate() {
            if f.tried[i] {
                let _ = writeln!(s, "tried {}", r.id);
            }
            if f.won[i] {
                let _ = writeln!(s, "won {}", r.id);
            }
        }
        s += "[stories]\n";
        let st = &self.stories;
        for (i, story) in self.def.stories.iter().enumerate() {
            if st.active == Some(i) {
                let _ = writeln!(s, "active {}", story.id);
            }
            if st.done[i] {
                let _ = writeln!(s, "done {}", story.id);
            }
            for (r, region) in story.regions.iter().enumerate() {
                for (k, _) in st.lit[i][r].iter().enumerate().filter(|(_, l)| **l) {
                    let _ = writeln!(s, "lamp {} {} {k}", story.id, region.id);
                }
            }
        }
        s += "[dragons]\n";
        let k = &self.kin;
        for (i, d) in self.def.kin.iter().enumerate() {
            if k.met[i] {
                let _ = writeln!(s, "dragon {}", d.id);
            } else if k.coaxed[i] > 0 {
                let _ = writeln!(s, "coaxed {} {}", d.id, k.coaxed[i]);
            }
            for (on, word) in [
                (k.followed[i], "followed"),
                (k.won[i], "beat"),
                (k.gifted[i], "gifted"),
            ] {
                if on {
                    let _ = writeln!(s, "{word} {}", d.id);
                }
            }
            if let Some(f) = self.kin_traded_find[i] {
                let _ = writeln!(s, "traded {} {}", d.id, self.def.finds[f].id);
            }
        }
        match k.egg {
            crate::Egg::None => {}
            crate::Egg::Given => s += "egg given\n",
            crate::Egg::Hatched => s += "egg hatched\n",
        }
        for (f, (name, _)) in crate::kin::FIRSTS.iter().enumerate() {
            if k.firsts[f] {
                let _ = writeln!(s, "first {name}");
            }
        }
        s += "[happenings]\n";
        if let Some(i) = self.happening() {
            let _ = writeln!(s, "now {}", self.def.happenings[i].id);
            if self.happen.seen_now {
                s += "here\n";
            }
        }
        for (h, _) in self
            .def
            .happenings
            .iter()
            .zip(&self.happen.seen)
            .filter(|(_, s)| **s)
        {
            let _ = writeln!(s, "saw {}", h.id);
        }
        for (f, _) in self
            .def
            .favors
            .iter()
            .zip(&self.favored)
            .filter(|(_, d)| **d)
        {
            let _ = writeln!(s, "thanks {}", f.id);
        }
        s + &self.other
    }

    /// Replaces everything with a save from [`Play::save`].
    pub fn load(&mut self, text: &str) -> Result<(), String> {
        let mut lines = text.lines().map(str::trim);
        if lines.next() != Some(HEAD) {
            return Err("not a zawnabout save".into());
        }
        self.reset();
        self.changed = false;
        let mut section = String::new();
        for line in lines.filter(|l| !l.is_empty()) {
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.to_string();
                if !KNOWN.contains(&name) {
                    let _ = writeln!(self.other, "{line}");
                }
                continue;
            }
            if !KNOWN.contains(&section.as_str()) {
                let _ = writeln!(self.other, "{line}");
                continue;
            }
            if line == "here" {
                self.happen.seen_now = true;
                continue;
            }
            let card = |id: &str| self.def.postcards.iter().position(|c| c.id == id);
            let local = |id: &str| self.def.locals.iter().position(|l| l.id == id);
            match line.split_once(' ') {
                Some(("met", id)) => {
                    if let Some(i) = local(id) {
                        self.met[i] = true;
                    }
                }
                Some(("asked", id)) => {
                    if let Some(i) = local(id) {
                        self.errands[i] = Errand::Accepted;
                    }
                }
                Some(("done", id)) if section != "stories" => {
                    if let Some(i) = local(id) {
                        self.errands[i] = Errand::Done;
                    }
                }
                Some(("carry", id)) => self.carrying = local(id),
                Some(("ride", id)) => self.riding = local(id),
                Some(("postcard", id)) => {
                    if let Some(i) = card(id) {
                        self.taken[i] = true;
                    }
                }
                Some(("pic", rest)) => {
                    if let Some((id, pic)) = rest.split_once(' ')
                        && let Some(i) = card(id)
                        && paint::decode(pic).is_some()
                    {
                        self.pics[i] = Some(pic.to_string());
                    }
                }
                Some(("lair", id)) => {
                    self.lair = self.def.lairs.iter().position(|l| l.id == id);
                }
                Some(("active", id)) => {
                    self.stories.active = self.def.stories.iter().position(|s| s.id == id);
                }
                Some(("done", id)) if section == "stories" => {
                    if let Some(i) = self.def.stories.iter().position(|s| s.id == id) {
                        self.stories.done[i] = true;
                    }
                }
                Some(("lamp", rest)) => {
                    let mut w = rest.split(' ');
                    if let (Some(id), Some(region), Some(k)) = (w.next(), w.next(), w.next())
                        && let Some(i) = self.def.stories.iter().position(|s| s.id == id)
                        && let Some(r) = self.def.stories[i]
                            .regions
                            .iter()
                            .position(|g| g.id == region)
                        && let Ok(k) = k.parse::<usize>()
                        && k < self.stories.lit[i][r].len()
                    {
                        self.stories.lit[i][r][k] = true;
                    }
                }
                Some(("stunt", id)) => {
                    if let Some(i) = self.def.stunts.iter().position(|s| s.id == id) {
                        self.flying.stunts[i] = true;
                    }
                }
                Some(("course", id)) => {
                    if let Some(c) = self.def.courses.iter().position(|c| c.id == id) {
                        self.flying.courses[c] = true;
                        self.flying.gates[c].iter_mut().for_each(|g| *g = true);
                    }
                }
                Some(("gate", rest)) => {
                    if let Some((id, g)) = rest.split_once(' ')
                        && let Some(c) = self.def.courses.iter().position(|c| c.id == id)
                        && let Ok(g) = g.parse::<usize>()
                        && g < self.flying.gates[c].len()
                    {
                        self.flying.gates[c][g] = true;
                    }
                }
                Some(("tried", id)) => {
                    if let Some(i) = self.def.races.iter().position(|r| r.id == id) {
                        self.flying.tried[i] = true;
                    }
                }
                Some(("won", id)) => {
                    if let Some(i) = self.def.races.iter().position(|r| r.id == id) {
                        self.flying.won[i] = true;
                    }
                }
                Some(("dragon", id)) => {
                    if let Some(i) = self.def.kin.iter().position(|k| k.id == id) {
                        self.kin.met[i] = true;
                    }
                }
                Some((word @ ("followed" | "beat" | "gifted"), id)) => {
                    if let Some(i) = self.def.kin.iter().position(|k| k.id == id) {
                        let k = &mut self.kin;
                        match word {
                            "followed" => k.followed[i] = true,
                            "beat" => k.won[i] = true,
                            _ => k.gifted[i] = true,
                        }
                    }
                }
                Some(("coaxed", rest)) => {
                    if let Some((id, n)) = rest.split_once(' ')
                        && let Some(i) = self.def.kin.iter().position(|k| k.id == id)
                        && let Ok(n) = n.parse::<u8>()
                    {
                        self.kin.coaxed[i] = n.min(crate::kin::COAX);
                    }
                }
                Some(("traded", rest)) => {
                    if let Some((id, find)) = rest.split_once(' ')
                        && let Some(i) = self.def.kin.iter().position(|k| k.id == id)
                        && let Some(f) = self.def.finds.iter().position(|d| d.id == find)
                    {
                        self.kin.traded[i] = true;
                        self.kin_traded_find[i] = Some(f);
                    }
                }
                Some(("egg", state)) => {
                    self.kin.egg = match state {
                        "hatched" => crate::Egg::Hatched,
                        "given" => crate::Egg::Given,
                        _ => crate::Egg::None,
                    };
                }
                Some(("first", name)) => {
                    if let Some(f) = crate::kin::FIRSTS.iter().position(|(n, _)| *n == name) {
                        self.kin.firsts[f] = true;
                    }
                }
                Some(("now", id)) => {
                    if let Some(i) = self.def.happenings.iter().position(|h| h.id == id) {
                        self.happen.current = i;
                    }
                }
                Some(("saw", id)) => {
                    if let Some(i) = self.def.happenings.iter().position(|h| h.id == id) {
                        self.happen.seen[i] = true;
                    }
                }
                Some(("thanks", id)) => {
                    if let Some(i) = self.def.favors.iter().position(|f| f.id == id) {
                        self.favored[i] = true;
                    }
                }
                Some(("found", id)) => {
                    if let Some(i) = self.def.finds.iter().position(|f| f.id == id) {
                        self.found[i] = true;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Play, paint};
    use zawnabout_sim::Sim;

    /// A save comes back the same, keeps sections it does not know, and refuses junk.
    #[test]
    fn round_trip() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        if !play.found.is_empty() {
            play.found[0] = true;
        }
        if !play.def.lairs.is_empty() {
            play.claim(0);
        }
        if play.def.locals.len() > 1 {
            play.met[0] = true;
            play.accept(0);
            play.give_back(1);
            play.carrying = Some(0);
        }
        if !play.def.stunts.is_empty() {
            play.flying.stunts[0] = true;
        }
        if let Some(g) = play.flying.gates.first_mut() {
            g[0] = true;
        }
        if !play.def.stories.is_empty() {
            play.begin(0);
            play.stories.lit[0][0][0] = true;
        }
        if !play.def.races.is_empty() {
            play.flying.tried[0] = true;
            play.flying.won[0] = true;
        }
        if !play.def.postcards.is_empty() {
            play.taken[0] = true;
            play.set_pic(0, &vec![200; paint::PIC.0 * paint::PIC.1 * 4]);
        }
        if !play.def.kin.is_empty() {
            play.kin.met[0] = true;
            play.kin.followed[0] = true;
            play.kin.egg = crate::Egg::Hatched;
            play.kin.firsts[1] = true;
        }
        let text = play.save() + "[later]\nthing 1\n";
        let mut back = Play::new(&sim.world);
        back.load(&text).expect("loads");
        assert_eq!(back.found, play.found);
        assert_eq!(back.lair, play.lair);
        assert_eq!(back.taken, play.taken);
        assert_eq!(back.pics, play.pics);
        assert_eq!(back.errands, play.errands);
        assert_eq!(back.carrying, play.carrying);
        assert_eq!(back.flying.stunts, play.flying.stunts);
        assert_eq!(back.flying.gates, play.flying.gates);
        assert_eq!(back.flying.won, play.flying.won);
        assert_eq!(back.stories.active, play.stories.active);
        assert_eq!(back.stories.lit, play.stories.lit);
        assert_eq!(back.kin.met, play.kin.met);
        assert_eq!(back.kin.egg, play.kin.egg);
        assert_eq!(back.kin.firsts, play.kin.firsts);
        assert_eq!(back.save(), text);
        assert!(back.load("hello").is_err());
        assert_eq!(back.found, play.found);
    }
}
