//! Happenings and town favor. A happening (a regatta, a wedding, a market day, fireworks)
//! is always on somewhere on the islands, noticed by its sound and lights. It stays until
//! you have been there, and once you have flown well away the next one begins
//! somewhere else. Nothing is missed by being late.
//! Towns thank you as you help their people: at each step of favor something new goes
//! up for you, like bunting, a band, or a dragon weather vane, and stays.
//! Both are drawn as effects, like finished sets. None of it touches the world.

use glam::Vec3;
use zawnabout_sim::dragon::{Dragon, Mode};

use crate::{EffectDef, Event, Play};

/// Something going on in a town for a while.
pub struct HappeningDef {
    pub id: &'static str,
    pub name: &'static str,
    /// Where, as people say it.
    pub place: &'static str,
    /// What townsfolk say about it, to tempt you over.
    pub rumor: &'static str,
    /// Said when you get there.
    pub seen: &'static str,
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub effects: &'static [EffectDef],
}

/// A town's thanks at `at` favors: what goes up, and what the card says.
pub struct FavorDef {
    pub id: &'static str,
    pub town: &'static str,
    pub at: usize,
    pub done: &'static str,
    pub effects: &'static [EffectDef],
}

/// Which happening is on, whether it has been seen, and every one seen so far.
#[derive(Clone, Debug, Default)]
pub struct Happenings {
    pub current: usize,
    pub seen_now: bool,
    pub seen: Vec<bool>,
}

/// Flying this far from a happening you have seen starts the next one (m).
const MOVE_ON: f32 = 1500.0;

impl Play {
    /// The happening on now, if there are any.
    pub fn happening(&self) -> Option<usize> {
        (!self.def.happenings.is_empty()).then_some(self.happen.current)
    }

    /// Seeing the happening on now, and moving on to the next once well away from it.
    /// Towns' thanks go up as their favor grows.
    pub fn happen_tick(&mut self, me: &Dragon) -> Vec<Event> {
        let mut out = Vec::new();
        if let Some(i) = self.happening() {
            let h = &self.def.happenings[i];
            let far = (me.pos.x - h.x).hypot(me.pos.z - h.z);
            // Most are seen by stopping there, geese by flying up among them.
            let geese = self.happen_at[i]
                .iter()
                .zip(h.effects)
                .any(|((a, _), e)| e.kind == "geese" && (me.pos.y - a.y).abs() < 50.0);
            // The ghost ship is only out at night, and seen by flying in low beside it.
            let ghost = h.effects.iter().any(|e| e.kind == "ghostship");
            let low = self.happen_at[i]
                .iter()
                .zip(h.effects)
                .any(|((a, _), e)| e.kind == "ghostship" && me.pos.y - a.y < 60.0);
            let down = if ghost {
                self.night && low
            } else {
                me.mode == Mode::Ground || me.vel.length() < 10.0 || geese
            };
            if !self.happen.seen_now && far < h.r && down {
                self.happen.seen_now = true;
                self.happen.seen[i] = true;
                self.changed = true;
                out.push(Event::Happening(i));
            } else if self.happen.seen_now && far > MOVE_ON {
                self.happen.current = (i + 1) % self.def.happenings.len();
                self.happen.seen_now = false;
                self.changed = true;
            }
        }
        for (i, f) in self.def.favors.iter().enumerate() {
            if !self.favored[i] && self.favor(f.town) >= f.at {
                self.favored[i] = true;
                self.changed = true;
                out.push(Event::Favor(i));
            }
        }
        out
    }

    /// The happening on now, as effects with their ends on the ground.
    pub(crate) fn happening_effects(
        &self,
    ) -> impl Iterator<Item = (&'static EffectDef, Vec3, Vec3)> + '_ {
        let i = self.happening();
        let defs: &'static [EffectDef] = i.map_or(&[], |i| self.def.happenings[i].effects);
        defs.iter()
            .zip(i.map_or(&[][..], |i| &self.happen_at[i][..]))
            .map(|(e, (a, b))| (e, *a, *b))
    }

    /// What towns have put up for you, as effects with their ends on the ground.
    pub(crate) fn favor_effects(
        &self,
    ) -> impl Iterator<Item = (&'static EffectDef, Vec3, Vec3)> + '_ {
        self.def
            .favors
            .iter()
            .enumerate()
            .filter(|(i, _)| self.favored[*i])
            .flat_map(move |(i, f)| {
                f.effects
                    .iter()
                    .zip(&self.favor_at[i])
                    .map(|(e, (a, b))| (e, *a, *b))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLAY;
    use zawnabout_sim::Sim;

    /// A happening is seen by landing at it, stays until you are well away, then the
    /// next begins.
    #[test]
    fn happenings_move_on() {
        if PLAY.happenings.len() < 2 {
            return;
        }
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        let h = &PLAY.happenings[0];
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), Vec3::new(h.x, 0.0, h.z), 0.0);
        me.mode = Mode::Ground;
        assert_eq!(play.happen_tick(&me), vec![Event::Happening(0)]);
        assert!(play.happen_tick(&me).is_empty());
        assert_eq!(play.happening(), Some(0));
        me.pos.x += MOVE_ON * 2.0;
        play.happen_tick(&me);
        assert_eq!(play.happening(), Some(1));
        assert!(play.happen.seen[0]);
    }

    /// The geese are seen by flying up among them, not by passing far below.
    #[test]
    fn geese_seen_in_flight() {
        let Some(i) = PLAY.happenings.iter().position(|h| h.id == "geese-south") else {
            return;
        };
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        play.happen.current = i;
        let h = &PLAY.happenings[i];
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), Vec3::new(h.x, 5.0, h.z), 0.0);
        me.mode = Mode::Air;
        me.vel = Vec3::new(0.0, 0.0, 20.0);
        assert!(play.happen_tick(&me).is_empty());
        me.pos.y = 75.0;
        assert_eq!(play.happen_tick(&me), vec![Event::Happening(i)]);
    }

    /// The ghost ship is only seen at night, flying in low beside it.
    #[test]
    fn ghost_ship_seen_at_night() {
        let Some(i) = PLAY.happenings.iter().position(|h| h.id == "ghost-ship") else {
            return;
        };
        let sim = Sim::new(1);
        let mut play = Play::new(&sim.world);
        play.happen.current = i;
        let h = &PLAY.happenings[i];
        let mut me = Dragon::new(zawnabout_sim::EntityId(9), Vec3::new(h.x, 20.0, h.z), 0.0);
        me.mode = Mode::Air;
        me.vel = Vec3::new(0.0, 0.0, 20.0);
        assert!(play.happen_tick(&me).is_empty());
        play.night = true;
        me.pos.y = 200.0;
        assert!(play.happen_tick(&me).is_empty());
        me.pos.y = 20.0;
        assert_eq!(play.happen_tick(&me), vec![Event::Happening(i)]);
    }

    /// Every favor names a town that has locals to help, at a step they can reach.
    #[test]
    fn favors_are_reachable() {
        for f in PLAY.favors {
            let helpers = PLAY
                .locals
                .iter()
                .filter(|l| l.town == f.town && l.request.is_some())
                .count();
            assert!(
                f.at >= 1 && f.at <= helpers,
                "{}: {} of {helpers}",
                f.id,
                f.at
            );
        }
    }
}
