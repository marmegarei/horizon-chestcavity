//! A player's body: its cavity, what the organs do to them, and the screen
//! that edits it. Everything goes through `Host` with the player's UUID, so a
//! test double can stand in for the game.
use crate::screen::{self, Click};
use chestcavity_shared::cavity::{Cavity, Scores, player_kind};
use chestcavity_shared::effects::{self, Action};
use horizon_client::mod_api::{Host, ItemStack, NativeEffect, Tag, Text};

/// The key of the cavity in the player's persistent data (saved with the player).
const KEY: &str = "chestcavity";

pub struct Body {
    pub cavity: Cavity,
    pub normal: Scores,
    pub scores: Scores,
    /// How long the heart has been missing, in seconds.
    pub bleed: u32,
    pub ticks: u64,
    /// The window id of the screen they have open.
    pub menu: Option<i32>,
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
        let kind = player_kind();
        Body { scores: cavity.scores(kind), cavity, normal: Cavity::of(kind).scores(kind), bleed: 0, ticks: 0, menu: None }
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
        self.scores = self.cavity.scores(player_kind());
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
        self.menu = host.open_menu(uuid, 3, Text::literal("Chest Cavity".to_string()), screen::items(&self.cavity));
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
    /// Once a second: what the organs do over time. `eyes` is the block at the player's eyes.
    pub fn beat(&mut self, host: &mut dyn Host, uuid: u128, name: &str, eyes: &str) {
        let seconds = self.ticks / 20;
        let in_water = eyes.starts_with("minecraft:water");
        for action in effects::periodic(&self.scores, &self.normal, seconds, &mut self.bleed, in_water) {
            match action {
                Action::Damage { amount, kind } => {
                    host.run_command(&format!("damage {name} {amount} {kind}"));
                }
                Action::Effect { id, seconds, amplifier } => {
                    let effect = NativeEffect { id: id.into(), amplifier: amplifier as u8, duration: seconds as i32 * 20, ambient: false, particles: false };
                    host.add_effect(uuid, &effect);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
