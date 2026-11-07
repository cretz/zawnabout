//! Stories: campaigns that change the islands for a while, one at a time, only once the
//! player starts one. The first is the lights going out: island by island the lights
//! are dark at night until the dragon relights that island's lamps with fire. Set it
//! aside any time and everything is as it was, with the lamps lit so far kept for
//! later. Finish it and the islands go back to normal, with a keepsake for the hoard.
//! None of it touches the world: the game draws the darkness and the lamps.

use glam::{Vec2, Vec3};
use zawnabout_sim::dragon::Dragon;

use crate::{Event, GiftDef, Play};

/// A lamp to light with fire, standing on whatever is under `x`, `z` (below `y` when
/// given).
pub struct LampDef {
    pub name: &'static str,
    pub x: f32,
    pub y: Option<f32>,
    pub z: f32,
}

/// A part of the islands that goes dark: everything within `r` of `x`, `z`, with the
/// lamps that bring its lights back.
pub struct RegionDef {
    pub id: &'static str,
    pub name: &'static str,
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub lamps: &'static [LampDef],
}

pub struct StoryDef {
    pub id: &'static str,
    pub name: &'static str,
    /// The local who asks for help with it.
    pub giver: &'static str,
    /// What they ask.
    pub ask: &'static str,
    /// What the journal says about it.
    pub about: &'static str,
    /// Said when it is all done.
    pub done: &'static str,
    pub gold: u32,
    pub prize: GiftDef,
    pub regions: &'static [RegionDef],
}

/// Where a lamp's fire burns, over its foot.
pub const FLAME: f32 = 2.2;

/// One player's stories: which one is on, the lamps lit in each, and which are told.
#[derive(Clone, Debug, Default)]
pub struct Stories {
    pub active: Option<usize>,
    /// Story, region, lamp.
    pub lit: Vec<Vec<Vec<bool>>>,
    pub done: Vec<bool>,
}

impl Stories {
    pub fn new(defs: &[StoryDef]) -> Self {
        Self {
            active: None,
            lit: defs
                .iter()
                .map(|s| {
                    s.regions
                        .iter()
                        .map(|r| vec![false; r.lamps.len()])
                        .collect()
                })
                .collect(),
            done: vec![false; defs.len()],
        }
    }

    pub fn reset(&mut self) {
        self.active = None;
        self.lit
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(|l| *l = false);
        self.done.iter_mut().for_each(|d| *d = false);
    }
}

impl Play {
    /// Starts story `s`. A story told before starts over, dark again.
    pub fn begin(&mut self, s: usize) {
        if self.stories.done[s] {
            self.stories.lit[s]
                .iter_mut()
                .flatten()
                .for_each(|l| *l = false);
        }
        self.stories.active = Some(s);
        self.changed = true;
    }

    /// Sets the story aside: the islands are as they were, and its lamps stay lit for
    /// when it is picked up again.
    pub fn set_aside(&mut self) {
        self.stories.active = None;
        self.changed = true;
    }

    /// Whether every lamp of region `r` of story `s` is lit.
    pub fn region_lit(&self, s: usize, r: usize) -> bool {
        self.stories.lit[s][r].iter().all(|l| *l)
    }

    /// Lamps lit in story `s`, and how many it has.
    pub fn lamps_lit(&self, s: usize) -> (usize, usize) {
        let all = self.stories.lit[s].iter().flatten();
        (all.clone().filter(|l| **l).count(), all.count())
    }

    /// The story under way and the region at `at`, if it is still dark there.
    pub fn dark_at(&self, at: Vec3) -> Option<(usize, usize)> {
        let s = self.stories.active?;
        self.def.stories[s]
            .regions
            .iter()
            .position(|g| Vec2::new(g.x - at.x, g.z - at.z).length() < g.r)
            .filter(|r| !self.region_lit(s, *r))
            .map(|r| (s, r))
    }

    /// Where the lights are out: each dark region's middle and reach.
    pub fn dark(&self) -> Vec<(Vec2, f32)> {
        let Some(s) = self.stories.active else {
            return Vec::new();
        };
        self.def.stories[s]
            .regions
            .iter()
            .enumerate()
            .filter(|(r, _)| !self.region_lit(s, *r))
            .map(|(_, g)| (Vec2::new(g.x, g.z), g.r))
            .collect()
    }

    /// The story's lamps, where each stands and whether it burns.
    pub fn lamps(&self) -> Vec<(Vec3, bool)> {
        let Some(s) = self.stories.active else {
            return Vec::new();
        };
        self.story_lamps[s]
            .iter()
            .zip(&self.stories.lit[s])
            .flat_map(|(at, lit)| at.iter().copied().zip(lit.iter().copied()))
            .collect()
    }

    /// Fire breath lights the story's lamps it reaches, out to `range`, a little more
    /// forgiving than the world's beacons.
    pub fn story_tick(&mut self, me: &Dragon, range: f32) -> Vec<Event> {
        let mut out = Vec::new();
        let Some(s) = self.stories.active else {
            return out;
        };
        if me.breath <= 0.5 {
            return out;
        }
        let (mouth, dir) = me.breath_ray();
        for (r, region) in self.story_lamps[s].iter().enumerate() {
            for (k, foot) in region.iter().enumerate() {
                let to = *foot + Vec3::Y * FLAME - mouth;
                let dist = to.length();
                if self.stories.lit[s][r][k]
                    || !(dist < 8.0 || (dist < range && to.dot(dir) > dist * 0.7))
                {
                    continue;
                }
                self.stories.lit[s][r][k] = true;
                self.changed = true;
                if self.region_lit(s, r) {
                    out.push(Event::RegionLit(s, r));
                } else {
                    out.push(Event::Lamp(s, r, k));
                }
            }
        }
        if self.stories.lit[s].iter().flatten().all(|l| *l) {
            self.stories.done[s] = true;
            self.stories.active = None;
            out.push(Event::StoryDone(s));
        }
        out
    }

    /// Gold from stories told.
    pub fn story_gold(&self) -> u32 {
        self.def
            .stories
            .iter()
            .zip(&self.stories.done)
            .filter(|(_, d)| **d)
            .map(|(s, _)| s.gold)
            .sum()
    }

    /// Keepsakes from stories told.
    pub fn story_prizes(&self) -> impl Iterator<Item = &'static GiftDef> + '_ {
        self.def
            .stories
            .iter()
            .zip(&self.stories.done)
            .filter(|(_, d)| **d)
            .map(|(s, _)| &s.prize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLAY;
    use zawnabout_sim::dragon::Mode;
    use zawnabout_sim::{EntityId, Sim};

    /// Breathing on every lamp relights the islands one by one and tells the story. Set
    /// aside part way, the islands are lit again and the lamps stay lit.
    #[test]
    fn relight_the_islands() {
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let s = 0;
        let story = &PLAY.stories[s];
        assert!(play.dark().is_empty());
        play.begin(s);
        assert_eq!(play.dark().len(), story.regions.len());
        let mut me = Dragon::new(EntityId(9), Vec3::ZERO, 0.0);
        me.mode = Mode::Ground;
        me.breath = 1.0;
        let gold = play.gold();
        let mut events = Vec::new();
        for (r, region) in play.story_lamps[s].clone().iter().enumerate() {
            for foot in region {
                // Stand right by it and breathe.
                me.pos = *foot + Vec3::new(0.0, 1.0, -3.0);
                events.extend(play.story_tick(&me, 30.0));
            }
            assert!(play.region_lit(s, r));
            if r == 0 {
                play.set_aside();
                assert!(play.dark().is_empty());
                play.begin(s);
                assert_eq!(play.dark().len(), story.regions.len() - 1);
            }
        }
        assert!(events.contains(&Event::StoryDone(s)));
        assert!(play.stories.active.is_none());
        assert!(play.gold() > gold);
        assert!(play.gifts().any(|g| g.name == story.prize.name));
    }

    /// Lamps stand on open ground, not on roofs or in the sea, and every region sits
    /// apart from the others.
    #[test]
    fn lamps_on_the_ground() {
        let sim = Sim::new(1);
        let play = Play::new(&sim.world);
        let w = &sim.world;
        for (s, story) in PLAY.stories.iter().enumerate() {
            for (r, region) in story.regions.iter().enumerate() {
                for (k, lamp) in region.lamps.iter().enumerate() {
                    let at = play.story_lamps[s][r][k];
                    if lamp.y.is_none() {
                        let ground = w.terrain.height(at.x, at.z);
                        assert!(
                            (at.y - ground).abs() < 2.5,
                            "{} sits at {} over ground {ground}",
                            lamp.name,
                            at.y
                        );
                    }
                    assert!(!w.afloat(at.x, at.z, at.y), "{} is in water", lamp.name);
                    assert!(
                        Vec2::new(region.x - at.x, region.z - at.z).length() < region.r,
                        "{} is outside {}",
                        lamp.name,
                        region.id
                    );
                }
                for other in &story.regions[r + 1..] {
                    let d = Vec2::new(region.x - other.x, region.z - other.z).length();
                    assert!(
                        d > region.r + other.r,
                        "{} overlaps {}",
                        region.id,
                        other.id
                    );
                }
            }
        }
    }
}
