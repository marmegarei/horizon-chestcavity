//! Potion effects and what a body's organs make of them. Each tick the player's effects are looked at: one that was just
//! applied (or made longer) lasts as long as the organs say (`chestcavity_shared::potions`), and one that came with a
//! meal is dropped by a gut that can stand it.
use crate::body::Body;
use chestcavity_shared::cavity::get;
use chestcavity_shared::potions::{self, Category};
use horizon_client::mod_api::Host;
use std::collections::HashMap;

/// How many ticks after a meal a harmful effect is still the meal's.
const MEAL_TICKS: u64 = 3;

impl Body {
    /// Whether the effects of this body are worth watching.
    fn purges(&self) -> bool {
        potions::matters(&self.scores, &self.normal) || get(&self.scores, "iron_gut") > 0.0 || get(&self.scores, "rotgut") + get(&self.scores, "rot_digestion") > 0.0
    }

    /// Once a tick: shortens or lengthens the effects that just arrived, and drops those a gut can stand.
    pub fn purge(&mut self, host: &mut dyn Host, uuid: u128) {
        if !self.purges() {
            self.seen.clear();
            return;
        }
        let Some(effects) = host.effects(uuid) else { return };
        let (iron_gut, rot) = (get(&self.scores, "iron_gut") > 0.0, get(&self.scores, "rotgut") + get(&self.scores, "rot_digestion") > 0.0);
        let meal = self.last_meal.filter(|&(_, at)| self.ticks <= at + MEAL_TICKS).map(|(m, _)| m);
        let (mut out, mut seen, mut changed) = (Vec::with_capacity(effects.len()), HashMap::new(), false);
        for mut effect in effects {
            let id = effect.id.to_string();
            // Each tick takes one off what is left: a duration above what was expected is a new one.
            if self.seen.get(&id).is_none_or(|&expected| effect.duration > expected) {
                if let Some(meal) = meal {
                    let harmful = potions::category(&id) == Category::Harmful;
                    if harmful && (iron_gut || (rot && meal.rotten && id.ends_with("hunger"))) {
                        changed = true;
                        continue;
                    }
                }
                let duration = potions::adjust(&id, effect.duration, &self.scores, &self.normal);
                if duration != effect.duration {
                    effect.duration = duration;
                    changed = true;
                }
            }
            seen.insert(id, if effect.duration > 0 { effect.duration - 1 } else { effect.duration });
            out.push(effect);
        }
        self.seen = seen;
        if changed {
            host.set_effects(uuid, &out);
        }
    }
}

#[cfg(test)]
mod tests;
