//! A player's body: its cavity, what the organs do to them, and the screen
//! that edits it. Everything goes through `Host` with the player's UUID, so a
//! test double can stand in for the game.
use chestcavity_shared::cavity::{Cavity, SLOTS, Scores, Slot, organ, player_kind};
use chestcavity_shared::effects::{self, Action};
use horizon_client::mod_api::{Host, ItemStack, NativeEffect, Tag, Text};

/// The key of the cavity in the player's persistent data (saved with the player).
const KEY: &str = "chestcavity";
const EMPTY_SLOT: &str = "minecraft:gray_stained_glass_pane";

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

pub fn stack(id: &str, count: u32) -> ItemStack {
    ItemStack { item: id.into(), count, damage: 0, extra: None }
}

/// The player's inventory slot (0..9 the hotbar) of a menu slot, for a menu of
/// `size` own slots: the player's inventory comes below them, main rows first.
pub fn inventory_slot(size: usize, slot: usize) -> Option<usize> {
    match slot.checked_sub(size)? {
        n @ 0..27 => Some(n + 9),
        n @ 27..36 => Some(n - 27),
        _ => None,
    }
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
            host.set_menu_items(uuid, self.menu_items());
        }
    }

    fn menu_items(&self) -> Vec<Option<ItemStack>> {
        self.cavity.slots.iter().map(|s| Some(s.as_ref().map_or_else(|| stack(EMPTY_SLOT, 1), |s| stack(&s.id, s.count)))).collect()
    }

    /// Opens the screen: the 27 slots as a chest. Click an organ to take it out, an organ of yours below to put it in.
    pub fn open(&mut self, host: &mut dyn Host, uuid: u128) -> bool {
        self.menu = host.open_menu(uuid, 3, Text::literal("Chest Cavity".to_string()), self.menu_items());
        self.menu.is_some()
    }

    pub fn summary(&self) -> String {
        effects::summary(&self.scores, &self.normal)
    }

    /// One line a slot: " 3. Heart", " 4. (empty)".
    pub fn list(&self) -> Vec<String> {
        self.cavity
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| match s {
                Some(s) => {
                    let name = organ(&s.id).map_or(s.id.as_str(), |o| o.name);
                    if s.count > 1 { format!("{:>2}. {name} x{}", i + 1, s.count) } else { format!("{:>2}. {name}", i + 1) }
                }
                None => format!("{:>2}. (empty)", i + 1),
            })
            .collect()
    }

    /// A click on menu slot `slot`; what to tell the player, if anything.
    pub fn click(&mut self, host: &mut dyn Host, uuid: u128, slot: i32) -> Option<String> {
        let slot = usize::try_from(slot).ok()?;
        if slot < SLOTS {
            return self.take_out(host, uuid, slot);
        }
        let from = inventory_slot(SLOTS, slot)?;
        let held = host.player_stacks(uuid)?.get(from)?.clone()?;
        self.put_in(host, uuid, held, |host, held| host.take_stacks(uuid, &[(from, held.clone())]).is_ok())
    }

    /// Takes the organ in cavity slot `i` out into the player's inventory.
    pub fn take_out(&mut self, host: &mut dyn Host, uuid: u128, i: usize) -> Option<String> {
        let s = self.cavity.slots.get(i)?.clone()?;
        let kept = host.give_stack(uuid, stack(&s.id, s.count)).map_or(0, |left| left.count);
        if kept >= s.count {
            return Some("No room in your inventory".into());
        }
        self.cavity.slots[i] = (kept > 0).then(|| Slot { id: s.id, count: kept });
        self.changed(host, uuid);
        None
    }

    /// Puts `held`, an organ of the player's, in the first empty slot; `remove` takes it from where it is.
    pub fn put_in(&mut self, host: &mut dyn Host, uuid: u128, held: ItemStack, remove: impl FnOnce(&mut dyn Host, &ItemStack) -> bool) -> Option<String> {
        let Some(o) = organ(&held.item) else { return Some(format!("{} is not an organ", held.item)) };
        if held.count > o.stack {
            return Some(format!("Put at most {} {} in at a time", o.stack, o.name));
        }
        let Some(i) = self.cavity.slots.iter().position(Option::is_none) else {
            return Some("Every slot is taken: take an organ out first".into());
        };
        if !remove(host, &held) {
            return Some("That stack changed".into());
        }
        self.cavity.slots[i] = Some(Slot { id: held.item.to_string(), count: held.count });
        self.changed(host, uuid);
        None
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
