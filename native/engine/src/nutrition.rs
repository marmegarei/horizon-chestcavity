//! Eating, tiring and mending. The game says when a meal was eaten (`living_entity_use_item_finish`); how the food bar
//! moved is seen on the player's tick, and what the body's stomach, intestines and endurance make of it is set right
//! back, as Chest Cavity's hunger mixin does.
use crate::body::Body;
use crate::opening::Player;
use chestcavity_shared::cavity::get;
use chestcavity_shared::food::{self, Meal};
use horizon_client::mod_api::{Host, NativeEffect};

/// How many ticks after a meal the food bar's rise still is that meal's.
const MEAL_TICKS: u64 = 3;
/// Vanilla's natural regeneration: a fed body mends 1 health every 4 seconds, each costing 6 exhaustion.
const REGENERATION: f32 = 0.25;
const HEAL_EXHAUSTION: f32 = 6.0;

impl Body {
    /// `meal` has just been eaten.
    pub fn ate(&mut self, meal: Meal) {
        self.meal = Some((meal, self.ticks));
    }

    /// Once a tick: the food bar of `uuid` against last tick's. A rise that follows a meal is what its stomach digested
    /// and its intestines nourished (with the nausea and hunger a bad stomach or gut brings); a rise in exhaustion is
    /// what exercise cost, less or more for the endurance of the lungs.
    pub fn feed(&mut self, host: &mut dyn Host, uuid: u128) {
        let Some(now) = host.player_food(uuid) else { return };
        let Some((level, saturation, exhaustion)) = self.food.replace(now) else { return };
        let mut out = now;
        let meal = self.meal.filter(|&(_, at)| self.ticks <= at + MEAL_TICKS).filter(|_| now.0 > level || now.1 > saturation);
        if let Some((meal, _)) = meal {
            self.meal = None;
            let eaten = food::eat(&self.scores, meal, (now.0 - level).max(0), (now.1 - saturation).max(0.0));
            out.0 = (level + eaten.food).min(20);
            out.1 = (saturation + eaten.saturation).min(out.0 as f32);
            for (id, ticks) in [("minecraft:nausea", eaten.nausea), ("minecraft:hunger", eaten.hunger)] {
                if ticks > 0 {
                    host.add_effect(uuid, &NativeEffect { id: id.into(), amplifier: 0, duration: ticks, ambient: false, particles: true });
                }
            }
        }
        if now.2 > exhaustion {
            out.2 = exhaustion + (now.2 - exhaustion) * food::endurance_factor(&self.scores, &self.normal);
        }
        if out.0 != now.0 || (out.1 - now.1).abs() > 1e-3 || (out.2 - now.2).abs() > 1e-4 {
            host.set_player_status(uuid, None, Some(out), None);
            self.food = Some(out);
        }
    }

    /// Once a second: a body that burns faster than normal mends faster while it is fed, at the cost of food.
    pub fn mend(&mut self, host: &mut dyn Host, who: &Player) {
        let rate = food::metabolism_rate(&self.scores, &self.normal);
        let max = 20.0 + 4.0 * (get(&self.scores, "health") - get(&self.normal, "health"));
        let Some((level, saturation, exhaustion)) = self.food.filter(|f| f.0 >= 18) else { return };
        if rate <= 1.0 || who.health <= 0.0 || who.health >= max {
            return;
        }
        let heal = ((rate - 1.0) * REGENERATION).min(max - who.health);
        let food = (level, saturation, exhaustion + heal * HEAL_EXHAUSTION);
        host.set_player_status(who.uuid, Some(who.health + heal), Some(food), None);
        self.food = Some(food);
    }
}

impl Body {
    /// `HungerManager.add`, then `heal`: food if the body isn't full, else saturation, else health.
    pub fn nourish(&mut self, host: &mut dyn Host, who: &Player, hunger: i32, saturation: f32, heal: f32) {
        let Some((level, held, exhaustion)) = self.food else { return };
        let max = 20.0 + 4.0 * (get(&self.scores, "health") - get(&self.normal, "health"));
        let (health, food) = if level < 20 {
            (None, (level + hunger).min(20))
        } else if held < level as f32 {
            (None, level)
        } else if who.health < max {
            (Some((who.health + heal).min(max)), level)
        } else {
            return;
        };
        let held = if level < 20 { held } else { (held + saturation).min(food as f32) };
        host.set_player_status(who.uuid, health, Some((food, held, exhaustion)), None);
        self.food = Some((food, held, exhaustion));
    }
}
