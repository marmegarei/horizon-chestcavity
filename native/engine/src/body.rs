//! A player's body: its cavity, what the organs do to them, and the screen
//! that edits it. Everything goes through `Host` with the player's UUID, so a
//! test double can stand in for the game.
use crate::opening::Player;
use crate::screen::{self, Click};
use crate::texts::{Msg, item_name, msg};
use crate::teleport;
use chestcavity_shared::breath::Lungs;
use chestcavity_shared::config::Config;
use chestcavity_shared::cavity::{Cavity, GROWN, SLOTS, Scores, Slot, organ, player_kind};
use chestcavity_shared::layout;
use chestcavity_shared::damage::Resist;
use chestcavity_shared::drops::Rng;
use chestcavity_shared::effects::{self, Action, Env, Progress};
use chestcavity_shared::food::Meal;
use horizon_client::mod_api::{Host, ItemStack, NativeEffect, Tag, Text};
use std::collections::HashMap;

/// The key of the cavity in the player's persistent data (saved with the player).
const KEY: &str = "chestcavity";
/// The first line of a saved cavity since organs have owners. A cavity saved without it (before version 0.8) has none, and
/// every organ in it is its body's.
const OWNERS: &str = "owners";
/// The effect a body shows while it rejects an organ: its time is the time to the next rejection.
const REJECTION: &str = "chestcavity:organ_rejection";

pub struct Body {
    /// Whose body it is (their uuid): what organs fit it.
    pub uuid: u128,
    pub cavity: Cavity,
    /// How many of its organs it rejects, and how long it has been rejecting them (ticks).
    pub incompat: usize,
    pub rejection: i32,
    pub normal: Scores,
    pub scores: Scores,
    /// What the organs keep count of between beats.
    pub progress: Progress,
    pub ticks: u64,
    /// The window id of the screen they have open.
    pub menu: Option<i32>,
    /// The last meal eaten and the tick it was; the food bar (level, saturation, exhaustion) as it was last tick.
    pub meal: Option<(Meal, u64)>,
    /// The last meal and its tick, kept after it has been digested (what came with it is judged by it).
    pub last_meal: Option<(Meal, u64)>,
    /// The effects as they were last tick, by id: the duration each should have now if nothing new came.
    pub seen: HashMap<String, i32>,
    pub food: Option<(i32, f32, f32)>,
    /// What the lungs and the bones do, when they are not the normal ones'; the air the lungs could not count whole.
    pub lungs: Option<Lungs>,
    pub resist: Resist,
    pub air: f32,
    /// The tick (of `ticks`) each ability rests until.
    pub rests: HashMap<&'static str, u64>,
    /// How long abilities rest, against Chest Cavity's (`cooldown_scale` of the settings).
    pub scale: f32,
    /// Whether it was in water at its last beat.
    pub in_water: bool,
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
        // A body's own organs are born in it.
        let cavity = match saved {
            Some(t) if t.starts_with(OWNERS) => Cavity::from_text(&t),
            Some(t) => Cavity::from_text(&t).born(uuid, || false),
            None => Cavity::of(kind).born(uuid, || false),
        };
        let mut body = Body::of(cavity);
        body.uuid = uuid;
        body.rescore();
        if body.scores != body.normal {
            body.apply(host, uuid);
        }
        body
    }

    pub fn of(cavity: Cavity) -> Body {
        let mut body = Body {
            uuid: 0,
            incompat: 0,
            rejection: 0,
            scores: Scores::new(),
            cavity,
            normal: Cavity::of(player_kind()).scores(player_kind()),
            progress: Progress::default(),
            ticks: 0,
            menu: None,
            meal: None,
            last_meal: None,
            seen: HashMap::new(),
            food: None,
            lungs: None,
            resist: Resist::default(),
            air: 0.0,
            rests: HashMap::new(),
            scale: 1.0,
            in_water: false,
            wings: false,
            mode: 0,
        };
        body.rescore();
        body
    }

    /// The scores, and what the hooks without a `Host` read, from the cavity.
    pub(crate) fn rescore(&mut self) {
        self.scores = self.cavity.scores(player_kind());
        self.lungs = Lungs::differing(&self.scores, &self.normal);
        self.resist = Resist::of(&self.scores, &self.normal);
        self.incompat = self.cavity.incompatibility(self.uuid);
    }

    /// A new body, as after a death; it keeps the size its chest has.
    pub fn reset(&mut self, host: &mut dyn Host, uuid: u128) {
        self.cavity = Cavity::of(player_kind()).born(uuid, || false).resized(self.cavity.slots.len());
        self.changed(host, uuid);
    }

    /// The chest grows its fourth row (the dragon egg): whether it was not grown already.
    pub fn grow(&mut self, host: &mut dyn Host, uuid: u128) -> bool {
        if self.cavity.slots.len() >= GROWN {
            return false;
        }
        self.cavity = self.cavity.clone().grown();
        self.changed(host, uuid);
        true
    }

    /// The body dies: what is not its own leaves it (`dropUnboundOrgans`), and is returned to be dropped. A body that keeps its
    /// cavity through death (`keep`) gives nothing up.
    pub fn die(&mut self, host: &mut dyn Host, uuid: u128, keep: bool) -> Vec<Slot> {
        if keep {
            return Vec::new();
        }
        let gone = self.cavity.drop_unbound(uuid);
        if !gone.is_empty() {
            self.changed(host, uuid);
        }
        gone
    }

    /// The body after a death: a new one, or, when the settings keep the chest cavity (`keep`), the one it had with the rotten
    /// organs it lacked (`insertWelfareOrgans`). A new one keeps the fourth row of a grown chest when `keep_size` says so.
    /// What the dying did not drop (the game keeps the inventory, and the loot hook is not told) goes back into the
    /// player's hands rather than being lost.
    pub fn respawn(&mut self, host: &mut dyn Host, uuid: u128, keep: bool, keep_size: bool) {
        if keep {
            self.cavity.welfare(player_kind());
            self.changed(host, uuid);
            return;
        }
        for slot in self.cavity.drop_unbound(uuid) {
            host.give_stack(uuid, screen::organ_stack(&slot.id, slot.count, slot.owner));
        }
        if !keep_size {
            self.cavity = self.cavity.clone().resized(SLOTS);
        }
        self.reset(host, uuid);
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
            map.insert(KEY.into(), Tag::String(format!("{OWNERS}\n{}", self.cavity.to_text()).as_str().into()));
        }
        host.set_player_persistent_data(uuid, data);
        if self.menu.is_some() {
            host.set_menu_items(uuid, screen::items(&self.cavity));
        }
    }

    /// Opens the screen: the slots as a chest, with each organ in its place. Click an organ to take it out, an organ of
    /// yours below to put it in.
    pub fn open(&mut self, host: &mut dyn Host, uuid: u128) -> bool {
        if layout::organize(&mut self.cavity) {
            self.changed(host, uuid);
        }
        let rows = (self.cavity.slots.len() / 9) as u8;
        self.menu = host.open_menu(uuid, rows, msg("chestcavity.title.own").text(), screen::items(&self.cavity));
        self.menu.is_some()
    }

    /// The body's main scores, each with how far it is from normal, as one line.
    pub fn summary(&self) -> Text {
        let mut line = Text::empty();
        for (i, (key, value, diff)) in effects::summary(&self.scores, &self.normal).into_iter().enumerate() {
            if i > 0 {
                line = line.append(Text::literal(", "));
            }
            let name = Text::translate_with_fallback(&format!("chestcavity.score.{key}.name"), key, &[]);
            let entry = if diff.abs() < 0.01 {
                Text::translate_with_fallback("chestcavity.summary.entry", "%s %s", &[name, Text::literal(format!("{value:.1}"))])
            } else {
                Text::translate_with_fallback("chestcavity.summary.entry_diff", "%s %s (%s)", &[name, Text::literal(format!("{value:.1}")), Text::literal(format!("{diff:+.1}"))])
            };
            line = line.append(entry);
        }
        line
    }

    /// What a click on the screen did to the cavity: save it and tell the player, if there is anything to tell.
    fn settle(&mut self, host: &mut dyn Host, uuid: u128, click: Click) -> Option<Msg> {
        if click.changed {
            self.changed(host, uuid);
        }
        click.msg
    }

    /// A click on menu slot `slot`; what to tell the player, if anything.
    pub fn click(&mut self, host: &mut dyn Host, uuid: u128, slot: i32) -> Option<Msg> {
        let click = screen::click(host, uuid, &mut self.cavity, slot);
        self.settle(host, uuid, click)
    }

    /// Takes the organ in cavity slot `i` out into the player's inventory.
    pub fn take_out(&mut self, host: &mut dyn Host, uuid: u128, i: usize) -> Option<Msg> {
        let click = screen::take_out(host, uuid, &mut self.cavity, i);
        self.settle(host, uuid, click)
    }

    /// Puts `held`, an organ of the player's, in the first empty slot; `remove` takes it from where it is.
    pub fn put_in(&mut self, host: &mut dyn Host, uuid: u128, held: ItemStack, remove: impl FnOnce(&mut dyn Host, &ItemStack) -> bool) -> Option<Msg> {
        let click = screen::put_in(host, &mut self.cavity, held, remove);
        self.settle(host, uuid, click)
    }
    /// Puts the organ in the player's main hand in; what to tell them.
    pub fn put_held(&mut self, host: &mut dyn Host, uuid: u128) -> Msg {
        let Some(held) = host.held_stack(uuid) else { return msg("chestcavity.msg.no_held") };
        let name = organ(&held.item).map_or_else(|| Text::literal(held.item.to_string()), |o| item_name(o.id, o.name));
        self.put_in(host, uuid, held.clone(), |host, h| host.take_items(&h.item, h.count) == h.count)
            .unwrap_or_else(|| msg("chestcavity.msg.put").arg(name))
    }

    /// Once a second: what the organs do over time.
    pub fn beat(&mut self, host: &mut dyn Host, who: &Player, env: &Env, rng: &mut Rng, cfg: &Config) {
        let seconds = self.ticks / 20;
        // Organs of another body are rejected: the more of them, the sooner it hurts, and the effect shows how soon.
        if cfg.organ_rejection && self.incompat > 0 {
            let every = cfg.rejection_rate / self.incompat as f32;
            if self.rejection == 0 {
                let effect = NativeEffect { id: REJECTION.into(), amplifier: 0, duration: every as i32, ambient: false, particles: false };
                host.add_effect(who.uuid, &effect);
            }
            self.rejection += 20;
            if self.rejection as f32 >= every {
                self.rejection = 0;
                if let Ok(target) = u32::try_from(who.entity) {
                    host.hurt_entity(&who.dimension, target, cfg.rejection_damage, "minecraft:generic", None);
                }
            }
        } else {
            // Nothing left to reject: the effect goes with it.
            if self.rejection > 0 {
                host.remove_effect(who.uuid, REJECTION);
            }
            self.rejection = 0;
        }
        self.in_water = env.in_water;
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
                Action::Heal(amount) => {
                    let health = (who.health + amount).min(self.max_health());
                    if who.health > 0.0 && health > who.health {
                        host.set_player_status(who.uuid, Some(health), None, None);
                    }
                }
                Action::Teleport { range } => {
                    teleport::randomly(host, who, f64::from(range), rng);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
