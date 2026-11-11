//! Chest Cavity for Horizon, a native trait mod (docs/native-mods.md, "Trait mods"): the launcher
//! links this crate into the game and calls `hooks()`. Organs in the player's chest, opened with the
//! Chest Opener and swapped from a chest screen, what having (or missing) them does, and organs from
//! the mobs the player kills. The rules are in `chestcavity-shared`; `body` ties them to `Host`.
mod body;
#[cfg(test)]
mod fake;
mod local;
mod mob;
mod screen;

use body::Body;
use chestcavity_shared::cavity::{SLOTS, get, mob_name};
use chestcavity_shared::drops::{self, Rng};
use horizon_client::mod_api::{
    Flow, Hooks, Host, InteractEntityEvent, LootDropsEvent, MenuClickedEvent, MenuClosedEvent, NativeCommand, PerformCommandEvent,
    PlayerLoggedOutEvent, PlayerRespawnedEvent, PlayerTickEvent, RegisterCommandsEvent, ScreenMouseButtonPressedPreEvent,
    ScreenRenderPostEvent, Text, UseItemEvent,
};
use local::{Action, Hit, Local, Target};
use screen::stack;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const OPENER: &str = "chestcavity:chest_opener";
const NO_SCREEN: &str = "No screen here: use /cc list, /cc take <slot> and /cc put";

pub fn hooks() -> Box<dyn Hooks> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    Box::new(ChestCavity::new(seed))
}

struct ChestCavity {
    bodies: Mutex<HashMap<u128, Body>>,
    /// The server menus of mobs that players have open.
    mob_screens: Mutex<HashMap<u128, MobScreen>>,
    /// The screen of the player whose window this is, drawn by the mod.
    local: Mutex<Option<Local>>,
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
    fn new(seed: u64) -> Self {
        ChestCavity {
            bodies: Mutex::new(HashMap::new()),
            mob_screens: Mutex::new(HashMap::new()),
            local: Mutex::new(None),
            rng: Mutex::new(Rng(seed | 1)),
        }
    }

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

    /// Shows `local` as the mod's own screen over the world; false where this isn't the window's player.
    fn open_local(&self, host: &mut dyn Host, local: Local) -> bool {
        *lock(&self.local) = Some(local);
        let opened = host.mod_screen_open(local::MOD_ID);
        if !opened {
            *lock(&self.local) = None;
        }
        opened
    }

    /// The player's own cavity: a server menu, or the mod's own screen for the window's player.
    fn open_own(&self, host: &mut dyn Host, uuid: u128) -> bool {
        self.with(host, uuid, |body, host| {
            body.open(host, uuid) || {
                let own = Local {
                    uuid,
                    target: Target::Own,
                    title: "Chest Cavity".into(),
                    cavity: body.cavity.clone(),
                    summary: Some(body.summary()),
                    note: None,
                    pending: Vec::new(),
                };
                self.open_local(host, own)
            }
        })
    }

    /// The cavity of mob `entity` of type `kind`: a server menu, or the mod's own screen.
    fn open_mob(&self, host: &mut dyn Host, uuid: u128, entity: u32, kind: &str) {
        let Some(cavity) = mob::load(host, entity, kind) else {
            return Self::say(host, uuid, format!("{} has no chest cavity", mob_name(kind)));
        };
        if let Some(window) = mob::open_menu(host, uuid, &cavity, kind) {
            lock(&self.mob_screens).insert(uuid, MobScreen { window, entity, mob: kind.to_string() });
            return;
        }
        let screen = Local {
            uuid,
            target: Target::Mob { entity, mob: kind.to_string() },
            title: mob::title(kind),
            cavity,
            summary: None,
            note: None,
            pending: Vec::new(),
        };
        if !self.open_local(host, screen) {
            Self::say(host, uuid, NO_SCREEN.into());
        }
    }

    /// Carries out what clicks on the mod's own screen asked for, in the player's turn.
    fn drain_local(&self, host: &mut dyn Host, uuid: u128) {
        let work = {
            let mut guard = lock(&self.local);
            guard.as_mut().filter(|l| l.uuid == uuid && !l.pending.is_empty()).map(|l| (l.target.clone(), std::mem::take(&mut l.pending)))
        };
        let Some((target, actions)) = work else { return };
        let mut note = None;
        let (cavity, summary) = match &target {
            Target::Own => self.with(host, uuid, |body, host| {
                for action in actions {
                    note = match action {
                        Action::Take(i) => body.take_out(host, uuid, i),
                        Action::PutHeld => Some(body.put_held(host, uuid)),
                    };
                }
                (Some(body.cavity.clone()), Some(body.summary()))
            }),
            Target::Mob { entity, mob } => {
                let mut cavity = None;
                for action in actions {
                    let (now, said) = match action {
                        Action::Take(i) => {
                            let edited = mob::edit(host, *entity, mob, |host, cavity| screen::take_out(host, uuid, cavity, i));
                            (edited.as_ref().map(|e| e.0.clone()), edited.and_then(|e| e.1))
                        }
                        Action::PutHeld => {
                            let (now, said) = mob::put_held(host, uuid, *entity, mob);
                            (now, Some(said))
                        }
                    };
                    note = said;
                    cavity = now;
                }
                (cavity, None)
            }
        };
        if let Some(l) = lock(&self.local).as_mut().filter(|l| l.uuid == uuid && l.target == target) {
            match cavity {
                Some(cavity) => {
                    l.cavity = cavity;
                    l.note = note;
                }
                None => l.note = Some("It is gone".into()),
            }
            if summary.is_some() {
                l.summary = summary;
            }
        }
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
        if sub == Some("open") {
            if !self.open_own(host, uuid) {
                Self::say(host, uuid, NO_SCREEN.into());
            }
            return Flow::Cancel;
        }
        let lines = self.with(host, uuid, |body, host| match sub {
            None | Some("show") => vec![body.summary()],
            Some("list") => screen::list(&body.cavity),
            Some("reset") => {
                body.reset(host, uuid);
                vec!["Chest cavity back to normal".into()]
            }
            Some("take") => match arg.and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=SLOTS).contains(n)) {
                Some(n) if body.cavity.slots[n - 1].is_none() => vec![format!("Slot {n} is empty")],
                Some(n) => vec![body.take_out(host, uuid, n - 1).unwrap_or_else(|| format!("Took slot {n} out"))],
                None => vec![format!("/cc take 1-{SLOTS}")],
            },
            Some("put") => vec![body.put_held(host, uuid)],
            Some(other) => vec![format!("Unknown: /cc {other} (show, list, take <slot>, put, open)")],
        });
        for line in lines {
            Self::say(host, uuid, line);
        }
        Flow::Cancel
    }

    /// The Chest Opener in the air: your own chest cavity. Opening a chest hurts (4) unless it is easy to get into.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if &*event.stack.item != OPENER {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        let easy = self.with(host, p.uuid, |body, _| get(&body.scores, "ease_of_access") > 0.0);
        if !easy {
            host.run_command(&format!("damage {} 4 minecraft:generic", p.name));
        }
        if !self.open_own(host, p.uuid) {
            Self::say(host, p.uuid, NO_SCREEN.into());
        }
        Flow::Cancel
    }

    /// The Chest Opener on a mob (right click): its chest cavity, to take organs out of or put others in.
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if !event.stack.is_some_and(|s| &*s.item == OPENER) {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        self.open_mob(host, p.uuid, event.entity as u32, event.kind);
        Flow::Cancel
    }

    /// A click in a server menu: an organ in the cavity comes out, an organ of yours below goes in.
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

    /// The mod's own screen (the window's player), drawn over the world.
    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        if host.mod_screen() != Some(local::MOD_ID) {
            return;
        }
        if let Some(local) = lock(&self.local).as_ref() {
            local::draw(host, event.batch, event.screen, local);
        }
    }

    /// A click on the mod's own screen: queued, the player's next tick carries it out.
    fn screen_mouse_button_pressed_pre(&self, host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        if host.mod_screen() != Some(local::MOD_ID) {
            return Flow::Continue;
        }
        let action = match local::hit(event.screen.rect, event.screen.mouse) {
            Some(Hit::Slot(i)) if event.button == 0 => Some(Action::Take(i)),
            Some(Hit::Put) if event.button == 0 => Some(Action::PutHeld),
            _ => None,
        };
        if let (Some(action), Some(local)) = (action, lock(&self.local).as_mut()) {
            local.pending.push(action);
        }
        Flow::Cancel
    }

    /// Once a tick for each player: the body's beat comes every second.
    fn player_tick(&self, host: &mut dyn Host, _event: &mut PlayerTickEvent) {
        let Some(p) = host.acting_player() else { return };
        self.drain_local(host, p.uuid);
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

#[cfg(test)]
mod tests;
