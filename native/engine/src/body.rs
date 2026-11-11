//! A player's body: its cavity, what the organs do to them, and the screen
//! that edits it. Everything goes through `Host` with the player's UUID, so a
//! test double can stand in for the game.
use crate::opening::Player;
use crate::screen::{self, Click};
use crate::teleport;
use chestcavity_shared::breath::Lungs;
use chestcavity_shared::cavity::{Cavity, Scores, organ, player_kind};
use chestcavity_shared::damage::Resist;
use chestcavity_shared::drops::Rng;
use chestcavity_shared::effects::{self, Action, Env, Progress};
use chestcavity_shared::food::Meal;
use horizon_client::mod_api::{Host, ItemStack, NativeEffect, Tag, Text};
use std::collections::HashMap;

/// The key of the cavity in the player's persistent data (saved with the player).
const KEY: &str = "chestcavity";
pub const NO_HELD: &str = "Segura um órgão na mão principal para o meter";

pub struct Body {
    pub cavity: Cavity,
    pub normal: Scores,
    pub scores: Scores,
    /// What the organs keep count of between beats.
    pub progress: Progress,
    pub ticks: u64,
    /// The window id of the screen they have open.
    pub menu: Option<i32>,
    /// The last meal eaten and the tick it was; the food bar (level, saturation, exhaustion) as it was last tick.
    pub meal: Option<(Meal, u64)>,
    pub food: Option<(i32, f32, f32)>,
    /// What the lungs and the bones do, when they are not the normal ones'; the air the lungs could not count whole.
    pub lungs: Option<Lungs>,
    pub resist: Resist,
    pub air: f32,
    /// The tick (of `ticks`) each ability rests until.
    pub rests: HashMap<&'static str, u64>,
    /// Whether it has been let fly because of its organs, and the game mode that was in when it was.
    pub wings: bool,
    pub mode: i32,
}

impl Body {
    /// The body as saved, or a normal one; the attributes are only touched when it isn't normal.
    pub fn load(host: &mut dyn Host, uuid: u128) -> Body {
        let kind = player_kind();
        let saved = host.player_persistent_data(uuid).and_then(|d| match d.get(KEY) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        });
        let cavity = saved.map_or_else(|| Cavity::of(kind), |t| Cavity::from_text(&t));
        let body = Body::of(cavity);
        if body.cavity != Cavity::of(kind) {
            body.apply(host, uuid);
        }
        body
    }

    pub fn of(cavity: Cavity) -> Body {
        let mut body = Body {
            scores: Scores::new(),
            cavity,
            normal: Cavity::of(player_kind()).scores(player_kind()),
            progress: Progress::default(),
            ticks: 0,
            menu: None,
            meal: None,
            food: None,
            lungs: None,
            resist: Resist::default(),
            air: 0.0,
            rests: HashMap::new(),
            wings: false,
            mode: 0,
        };
        body.rescore();
        body
    }

    /// The scores, and what the hooks without a `Host` read, from the cavity.
    fn rescore(&mut self) {
        self.scores = self.cavity.scores(player_kind());
        self.lungs = Lungs::differing(&self.scores, &self.normal);
        self.resist = Resist::of(&self.scores, &self.normal);
    }

    /// A new body, as after a death.
    pub fn reset(&mut self, host: &mut dyn Host, uuid: u128) {
        self.cavity = Cavity::of(player_kind());
        self.changed(host, uuid);
    }

    fn apply(&self, host: &mut dyn Host, uuid: u128) {
        for (attribute, base) in effects::attribute_bases(&self.scores, &self.normal) {
            host.set_attribute_base(uuid, attribute, base);
        }
    }

    /// The cavity changed: new scores, attributes, saved state and screen.
    pub fn changed(&mut self, host: &mut dyn Host, uuid: u128) {
        self.rescore();
        self.apply(host, uuid);
        let mut data = host.player_persistent_data(uuid).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
        if let Tag::Compound(map) = &mut data {
            map.insert(KEY.into(), Tag::String(self.cavity.to_text().as_str().into()));
        }
        host.set_player_persistent_data(uuid, data);
        if self.menu.is_some() {
            host.set_menu_items(uuid, screen::items(&self.cavity));
        }
    }

    /// Opens the screen: the 27 slots as a chest. Click an organ to take it out, an organ of yours below to put it in.
    pub fn open(&mut self, host: &mut dyn Host, uuid: u128) -> bool {
        self.menu = host.open_menu(uuid, 3, Text::literal("Cavidade torácica"), screen::items(&self.cavity));
        self.menu.is_some()
    }

    pub fn summary(&self) -> String {
        effects::summary(&self.scores, &self.normal)
    }

    /// What a click on the screen did to the cavity: save it and tell the player, if there is anything to tell.
    fn settle(&mut self, host: &mut dyn Host, uuid: u128, click: Click) -> Option<String> {
        if click.changed {
            self.changed(host, uuid);
        }
        click.msg
    }

    /// A click on menu slot `slot`; what to tell the player, if anything.
    pub fn click(&mut self, host: &mut dyn Host, uuid: u128, slot: i32) -> Option<String> {
        let click = screen::click(host, uuid, &mut self.cavity, slot);
        self.settle(host, uuid, click)
    }

    /// Takes the organ in cavity slot `i` out into the player's inventory.
    pub fn take_out(&mut self, host: &mut dyn Host, uuid: u128, i: usize) -> Option<String> {
        let click = screen::take_out(host, uuid, &mut self.cavity, i);
        self.settle(host, uuid, click)
    }

    /// Puts `held`, an organ of the player's, in the first empty slot; `remove` takes it from where it is.
    pub fn put_in(&mut self, host: &mut dyn Host, uuid: u128, held: ItemStack, remove: impl FnOnce(&mut dyn Host, &ItemStack) -> bool) -> Option<String> {
        let click = screen::put_in(host, &mut self.cavity, held, remove);
        self.settle(host, uuid, click)
    }
    /// Puts the organ in the player's main hand in; what to tell them.
    pub fn put_held(&mut self, host: &mut dyn Host, uuid: u128) -> String {
        let Some(held) = host.held_stack(uuid) else { return NO_HELD.into() };
        self.put_in(host, uuid, held.clone(), |host, h| host.take_items(&h.item, h.count) == h.count)
            .unwrap_or_else(|| format!("Meteste {} na cavidade", organ(&held.item).map_or(held.item.as_ref(), |o| o.name)))
    }

    /// Once a second: what the organs do over time.
    pub fn beat(&mut self, host: &mut dyn Host, who: &Player, env: &Env, rng: &mut Rng) {
        let seconds = self.ticks / 20;
        self.mend(host, who);
        for action in effects::periodic(&self.scores, &self.normal, seconds, &mut self.progress, env) {
            match action {
                Action::Damage { amount, kind } => {
                    if let Ok(target) = u32::try_from(who.entity) {
                        host.hurt_entity(&who.dimension, target, amount, kind, None);
                    }
                }
                Action::Effect { id, ticks, amplifier } => {
                    let effect = NativeEffect { id: id.into(), amplifier: amplifier as u8, duration: ticks as i32, ambient: false, particles: false };
                    host.add_effect(who.uuid, &effect);
                }
                Action::Nourish { hunger, saturation, heal } => self.nourish(host, who, hunger, saturation, heal),
                Action::Teleport { range } => {
                    teleport::randomly(host, who, f64::from(range), rng);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
