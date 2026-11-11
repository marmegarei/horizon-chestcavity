//! Chest Cavity for Horizon, a native trait mod (docs/native-mods.md, "Trait mods"): the launcher
//! links this crate into the game and calls `hooks()`. Organs in the player's chest, opened with the
//! Chest Opener and swapped from a chest screen, what having (or missing) them does, and organs from
//! the mobs the player kills. The rules are in `chestcavity-shared`; `body` ties them to `Host`.
mod body;
#[cfg(test)]
mod fake;
mod mob;
mod screen;

use body::Body;
use chestcavity_shared::cavity::{SLOTS, get};
use screen::stack;
use chestcavity_shared::drops::{self, Rng};
use horizon_client::mod_api::{
    Flow, Hooks, Host, InteractEntityEvent, LootDropsEvent, MenuClickedEvent, MenuClosedEvent, NativeCommand, PerformCommandEvent,
    PlayerLoggedOutEvent, PlayerRespawnedEvent, PlayerTickEvent, RegisterCommandsEvent, Text, UseItemEvent,
};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const OPENER: &str = "chestcavity:chest_opener";

pub fn hooks() -> Box<dyn Hooks> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    Box::new(ChestCavity { bodies: Mutex::new(HashMap::new()), mob_screens: Mutex::new(HashMap::new()), rng: Mutex::new(Rng(seed | 1)) })
}

struct ChestCavity {
    bodies: Mutex<HashMap<u128, Body>>,
    /// The mob screens players have open.
    mob_screens: Mutex<HashMap<u128, MobScreen>>,
    rng: Mutex<Rng>,
}

struct MobScreen {
    window: i32,
    entity: u32,
    mob: String,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl ChestCavity {
    /// Runs `f` on `uuid`'s body, loading it first when it isn't here yet. The body is out of the
    /// map meanwhile, so a hook the game fires from inside a helper (a menu closing) never waits on it.
    fn with<R>(&self, host: &mut dyn Host, uuid: u128, f: impl FnOnce(&mut Body, &mut dyn Host) -> R) -> R {
        let taken = lock(&self.bodies).remove(&uuid);
        let mut body = taken.unwrap_or_else(|| Body::load(host, uuid));
        let out = f(&mut body, host);
        lock(&self.bodies).insert(uuid, body);
        out
    }

    fn say(host: &mut dyn Host, uuid: u128, line: String) {
        host.tell(uuid, Text::literal(line));
    }
}

impl Hooks for ChestCavity {
    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        for usage in ["cc", "cc show", "cc list", "cc take <slot:integer>", "cc put"] {
            event.commands.push(NativeCommand::new(usage));
        }
        for usage in ["cc open", "cc reset"] {
            event.commands.push(NativeCommand::op(usage, 2));
        }
    }

    /// `/cc` shows the body, `/cc take <slot>` and `/cc put` swap organs without the screen.
    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let mut words = event.command.split_whitespace();
        if words.next() != Some("cc") {
            return Flow::Continue;
        }
        let uuid = event.player.uuid;
        let (sub, arg) = (words.next(), words.next());
        let lines = self.with(host, uuid, |body, host| match sub {
            None | Some("show") => vec![body.summary()],
            Some("list") => screen::list(&body.cavity),
            Some("open") => vec![if body.open(host, uuid) { "Opened".into() } else { "The screen needs a server player".into() }],
            Some("reset") => {
                body.reset(host, uuid);
                vec!["Chest cavity back to normal".into()]
            }
            Some("take") => match arg.and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=SLOTS).contains(n)) {
                Some(n) if body.cavity.slots[n - 1].is_none() => vec![format!("Slot {n} is empty")],
                Some(n) => vec![body.take_out(host, uuid, n - 1).unwrap_or_else(|| format!("Took slot {n} out"))],
                None => vec![format!("/cc take 1-{SLOTS}")],
            },
            Some("put") => match host.held_stack(uuid) {
                None => vec!["Hold an organ in your main hand to put it in".into()],
                Some(held) => {
                    let said = body.put_in(host, uuid, held.clone(), |host, h| host.take_items(&h.item, h.count) == h.count);
                    vec![said.unwrap_or_else(|| format!("Put {} in", held.item))]
                }
            },
            Some(other) => vec![format!("Unknown: /cc {other} (show, list, take <slot>, put)")],
        });
        for line in lines {
            Self::say(host, uuid, line);
        }
        Flow::Cancel
    }

    /// The Chest Opener: opening a chest hurts (4) unless it is easy to get into.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if &*event.stack.item != OPENER {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        self.with(host, p.uuid, |body, host| {
            if get(&body.scores, "ease_of_access") <= 0.0 {
                host.run_command(&format!("damage {} 4 minecraft:generic", p.name));
            }
            if !body.open(host, p.uuid) {
                Self::say(host, p.uuid, "No screen here: use /cc list, /cc take <slot> and /cc put".into());
            }
        });
        Flow::Cancel
    }

    /// The Chest Opener on a mob (right click): its chest cavity, to take organs out of or put others in.
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if !event.stack.is_some_and(|s| &*s.item == OPENER) {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        let entity = event.entity as u32;
        match mob::open(host, p.uuid, entity, event.kind) {
            Ok(window) => {
                lock(&self.mob_screens).insert(p.uuid, MobScreen { window, entity, mob: event.kind.to_string() });
            }
            Err(line) => Self::say(host, p.uuid, line),
        }
        Flow::Cancel
    }

    /// A click in a screen: an organ in the cavity comes out, an organ of yours below goes in.
    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        let (uuid, window, slot) = (event.uuid, event.window, event.slot);
        // Plain clicks only: a throw or a drag must not move an organ.
        if !matches!(event.mode, 0 | 1) {
            return;
        }
        let mob = lock(&self.mob_screens).get(&uuid).filter(|s| s.window == window).map(|s| (s.entity, s.mob.clone()));
        let line = if let Some((entity, mob)) = mob {
            mob::click(host, uuid, entity, &mob, slot)
        } else if lock(&self.bodies).get(&uuid).is_some_and(|b| b.menu == Some(window)) {
            self.with(host, uuid, |body, host| body.click(host, uuid, slot))
        } else {
            return;
        };
        if let Some(line) = line {
            host.action_bar(uuid, Text::literal(line));
        }
    }

    fn menu_closed(&self, _host: &mut dyn Host, event: &mut MenuClosedEvent) {
        if let Some(body) = lock(&self.bodies).get_mut(&event.uuid).filter(|b| b.menu == Some(event.window)) {
            body.menu = None;
        }
        let mut screens = lock(&self.mob_screens);
        if screens.get(&event.uuid).is_some_and(|s| s.window == event.window) {
            screens.remove(&event.uuid);
        }
    }

    /// Once a tick for each player: the body's beat comes every second.
    fn player_tick(&self, host: &mut dyn Host, _event: &mut PlayerTickEvent) {
        let Some(p) = host.acting_player() else { return };
        self.with(host, p.uuid, |body, host| {
            body.ticks += 1;
            if body.ticks % 20 != 0 || p.game_mode == 3 {
                return;
            }
            let [x, y, z] = p.position;
            let eyes = host.block([x.floor() as i32, (y + 1.62).floor() as i32, z.floor() as i32]);
            body.beat(host, p.uuid, &p.name, &eyes);
        });
    }

    /// A new body, as after a death.
    fn player_respawned(&self, host: &mut dyn Host, event: &mut PlayerRespawnedEvent<'_>) {
        let uuid = event.player;
        self.with(host, uuid, |body, host| body.reset(host, uuid));
    }

    fn player_logged_out(&self, _host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        lock(&self.bodies).remove(&event.player);
        lock(&self.mob_screens).remove(&event.player);
    }

    /// Organs from the mobs a player kills.
    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let ctx = event.context;
        let (Some((_, mob)), Some(_), false) = (ctx.entity, ctx.player, ctx.used) else { return };
        for (id, n) in drops::for_mob(mob, &mut lock(&self.rng)) {
            event.drops.push(stack(id, n));
        }
    }
}
