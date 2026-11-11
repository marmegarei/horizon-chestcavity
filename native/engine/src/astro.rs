//! Astro Adds' organs that work on the other organs, on the body's clock (`CCAAOrganTickListeners`): regrowth, the tumors,
//! the hunters, the sculk infection and autophagy, each at its own pace. The rules are in `chestcavity_shared::astro`.
use crate::body::Body;
use crate::opening::Player;
use chestcavity_shared::astro::{self, Used};
use chestcavity_shared::cavity::{get, player_kind};
use chestcavity_shared::drops::Rng;
use chestcavity_shared::food::{self, Meal};
use horizon_client::mod_api::{Host, NativeEffect};

impl Body {
    /// Once a tick: whichever of the five is due. A change of the cavity is saved and rescored, and what regrowing cost a
    /// player is exhaustion.
    pub fn astro(&mut self, host: &mut dyn Host, who: &Player, rng: &mut Rng) {
        let t = self.ticks;
        if ![astro::REGROWTH, astro::AUTOPHAGY, astro::HUNTING, astro::INFECTION].iter().any(|&pace| t > 0 && t % pace == 0) {
            return;
        }
        let before = self.cavity.clone();
        let mut tired = 0.0;
        // What each reads is the cavity as the one before it left it.
        let level = |body: &Body, score: &str| get(&body.scores, score).ceil().max(0.0) as u32;

        let growth = get(&self.scores, "regrowth") - get(&self.normal, "regrowth");
        if growth > 0.0 && t % astro::REGROWTH == 0 {
            tired += astro::grow_back(&mut self.cavity, player_kind(), growth.ceil() as u32, rng, None);
            self.rescore();
        }
        let n = level(self, "tumor_autophagy");
        if n > 0 && t % astro::AUTOPHAGY == 0 {
            self.eat(host, who, n, true, rng);
            self.rescore();
        }
        let n = level(self, "tumor_hunting");
        if n > 0 && t % astro::HUNTING == 0 {
            astro::hunt_tumors(&mut self.cavity, n);
            self.rescore();
        }
        let n = level(self, "sculk_infection");
        if n > 0 && t % astro::INFECTION == 0 {
            astro::infect(&mut self.cavity, n);
            self.rescore();
        }
        let n = level(self, "autophagy");
        if n > 0 && t % astro::AUTOPHAGY == 0 {
            self.eat(host, who, n, false, rng);
        }
        if self.cavity != before {
            self.changed(host, who.uuid);
        }
        if tired > 0.0 {
            self.tire(host, who.uuid, tired);
        }
    }

    /// Autophagy: the body eats `n` units of the food in its cavity. One that is hungry is fed, as the meal would feed it
    /// (the effects of the food itself are left out); one that is full mends 1 health for each.
    fn eat(&mut self, host: &mut dyn Host, who: &Player, n: u32, tumorous: bool, rng: &mut Rng) {
        let mut cavity = self.cavity.clone();
        let (mut health, max) = (who.health, self.max_health());
        astro::eat_organs(&mut cavity, n, tumorous, rng, |id| {
            let Some((points, modifier)) = astro::food_of(id) else { return Used::Spared };
            if self.food.is_some_and(|f| f.0 < 20) {
                let eaten = food::eat(&self.scores, Meal::of(id), points, points as f32 * modifier * 2.0);
                self.nourish(host, who, eaten.food, eaten.saturation, 0.0);
                for (effect, ticks) in [("minecraft:nausea", eaten.nausea), ("minecraft:hunger", eaten.hunger)] {
                    if ticks > 0 {
                        host.add_effect(who.uuid, &NativeEffect { id: effect.into(), amplifier: 0, duration: ticks, ambient: false, particles: true });
                    }
                }
                Used::Fed
            } else if health < max {
                health = (health + 1.0).min(max);
                host.set_player_status(who.uuid, Some(health), None, None);
                Used::Healed
            } else {
                Used::Spared
            }
        });
        self.cavity = cavity;
    }
}

#[cfg(test)]
mod tests;
