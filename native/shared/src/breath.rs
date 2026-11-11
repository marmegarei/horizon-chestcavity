//! Lungs, as Chest Cavity's `applyBreathOnLand` and `applyBreathInWater` make of a tick of air: how much a
//! body's air changes in a tick given its lungs and gills, as the game's own change would have been.
//! Plain numbers in and out; the engine sets the game's per-tick air knobs from the answer.
use crate::cavity::{Scores, get};

/// A body's lungs: `recovery` (what refills air on land), `capacity` (how long a breath lasts) and `water`
/// (breathing under water).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lungs {
    pub recovery: f32,
    pub capacity: f32,
    pub water: f32,
}

impl Lungs {
    /// The lungs of a body with `now` scores, when they are not the normal ones' (`normal`): a normal body
    /// breathes as the game does and none of this is asked.
    pub fn differing(now: &Scores, normal: &Scores) -> Option<Lungs> {
        let of = |s: &Scores| Lungs { recovery: get(s, "breath_recovery"), capacity: get(s, "breath_capacity"), water: get(s, "water_breath") };
        (of(now) != of(normal)).then(|| of(now))
    }

    /// How fast air goes: 1 with the capacity of two lungs, slower with more, 20 times faster with none.
    fn ratio(&self) -> f32 {
        if self.capacity == 0.0 { 20.0 } else { (2.0 / self.capacity).min(20.0) }
    }

    /// The change of air this tick, as the game would have made it: out of water (`can_breathe`) it gains `refill`,
    /// under water it loses `consume`. Positive: the air rises by that much; negative: it falls. The part of a
    /// unit that doesn't fit is kept in `remainder` for the next tick (`lungRemainder`).
    pub fn tick(&self, can_breathe: bool, consume: i32, refill: i32, remainder: &mut f32) -> i32 {
        let mut loss;
        if can_breathe {
            // Breath refills: each point of recovery wins back half a refill over the unit the body spends a tick.
            loss = 1.0;
            if self.recovery > 0.0 {
                loss -= refill as f32 * self.recovery / 2.0;
            }
            if loss > 0.0 {
                loss = loss * self.ratio() + *remainder;
            }
        } else {
            // Gills win air back under water, two units a tick for each point.
            loss = 1.0;
            if self.water > 0.0 {
                loss -= 2.0 * self.water;
            }
            if loss > 0.0 {
                loss = loss * consume as f32 * self.ratio() + *remainder;
            }
        }
        *remainder = loss.fract();
        -(loss.trunc() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Cavity, player_kind};

    fn scores(recovery: f32, capacity: f32, water: f32) -> Scores {
        let kind = player_kind();
        let mut s = Cavity::of(kind).scores(kind);
        s.insert("breath_recovery", recovery);
        s.insert("breath_capacity", capacity);
        s.insert("water_breath", water);
        s
    }

    #[test]
    fn a_normal_body_is_left_to_the_game() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        assert_eq!(Lungs::differing(&normal, &normal), None);
        assert!(Lungs::differing(&scores(0.0, 0.0, 0.0), &normal).is_some());
    }

    #[test]
    fn lungs_set_how_long_a_breath_lasts_and_how_fast_it_comes_back() {
        let mut rem = 0.0;
        // Four lungs hold twice as long under water: half a unit a tick, the other half carried over.
        let four = Lungs { recovery: 4.0, capacity: 4.0, water: 0.0 };
        assert_eq!(four.tick(false, 1, 0, &mut rem), 0);
        assert_eq!(four.tick(false, 1, 0, &mut rem), -1);
        // Two lungs on land gain 3 a tick (a refill of 4 less the unit the body uses).
        rem = 0.0;
        let two = Lungs { recovery: 2.0, capacity: 2.0, water: 0.0 };
        assert_eq!(two.tick(true, 0, 4, &mut rem), 3);
        // One lung on land still gains, just slowly; none holds nothing back: 20 units a tick.
        assert_eq!(Lungs { recovery: 1.0, capacity: 1.0, water: 0.0 }.tick(true, 0, 4, &mut rem), 1);
        assert_eq!(Lungs::default().tick(true, 0, 4, &mut 0.0), -20);
        assert_eq!(Lungs::default().tick(false, 1, 0, &mut 0.0), -20);
    }

    #[test]
    fn gills_breathe_under_water() {
        let gills = Lungs { recovery: 2.0, capacity: 2.0, water: 1.0 };
        // Under water the unit a tick is more than paid back: the air rises by one.
        assert_eq!(gills.tick(false, 1, 0, &mut 0.0), 1);
        // Half a gill is not enough to keep the air from falling, but it slows it.
        assert_eq!(Lungs { recovery: 2.0, capacity: 2.0, water: 0.25 }.tick(false, 1, 0, &mut 0.0), 0);
    }
}
