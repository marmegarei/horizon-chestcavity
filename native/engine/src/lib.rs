//! Chest Cavity for Horizon, a native trait mod (docs/native-mods.md, "Trait mods"): the launcher
//! links this crate into the game and calls `hooks()`. Organs in the player's chest, opened with the
//! Chest Opener and swapped from a chest screen, what having (or missing) them does, what each organ
//! says about itself in its tooltip, and organs from the mobs the player kills. The rules are in
//! `chestcavity-shared`; `body` ties them to `Host`.
mod abilities;
mod body;
mod combat;
mod environment;
#[cfg(test)]
mod fake;
mod mob;
mod nutrition;
mod opening;
mod screen;
mod teleport;
#[cfg(test)]
mod tests;
mod trace;

use body::Body;
use chestcavity_shared::cavity::{SLOTS, organ};
use chestcavity_shared::describe::{Tone, describe};
use chestcavity_shared::drops::{self, Rng};
use chestcavity_shared::food::Meal;
use horizon_client::mod_api::{
    AttackEntityEvent, Flow, Hooks, Host, InteractEntityEvent, ItemStack, KeyEvent, LivingBreatheEvent, LivingEntityUseItemFinishEvent,
    LivingIncomingDamageV3Event, LootDropsEvent, MenuClickedEvent, MenuClosedEvent, NativeCommand, NetFromClientEvent,
    PerformCommandEvent, PlayerLoggedOutEvent, PlayerRespawnedEvent, PlayerTickEvent, RegisterCommandsEvent, ServerTickEvent,
    ServerTickPhase, Text, TooltipEvent, UseItemEvent, UseItemOnEvent,
};
use combat::Hit;
use opening::{Ask, Player};
use screen::stack;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const OPENER: &str = "chestcavity:chest_opener";
/// The message a client sends when an ability key goes down: its key group.
const ABILITY_CHANNEL: &str = "chestcavity:ability";
const NO_SCREEN: &str = "Não foi possível abrir o ecrã: usa /cc list, /cc take <ranhura> e /cc put";

pub fn hooks() -> Box<dyn Hooks> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    Box::new(ChestCavity::new(seed))
}

struct ChestCavity {
    bodies: Mutex<HashMap<u128, Body>>,
    /// The players' entity ids this session, for the hooks the game tells by id alone.
    entities: Mutex<HashMap<u32, u128>>,
    /// The screens asked for and not yet opened (see `opening`).
    opens: Mutex<HashMap<u128, Ask>>,
    /// The abilities asked for and not yet set off (see `abilities`): player, key group or ability.
    abilities: Mutex<Vec<(u128, String)>>,
    /// The blows to answer and the bodies to send away at the next server tick (see `combat`).
    hits: Mutex<Vec<Hit>>,
    dodges: Mutex<Vec<(u128, f32)>>,
    /// When each player last clicked a mob with the opener, in server ticks.
    mob_clicks: Mutex<HashMap<u128, u64>>,
    ticks: AtomicU64,
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
    fn new(seed: u64) -> Self {
        ChestCavity {
            bodies: Mutex::new(HashMap::new()),
            entities: Mutex::new(HashMap::new()),
            opens: Mutex::new(HashMap::new()),
            abilities: Mutex::new(Vec::new()),
            hits: Mutex::new(Vec::new()),
            dodges: Mutex::new(Vec::new()),
            mob_clicks: Mutex::new(HashMap::new()),
            ticks: AtomicU64::new(0),
            mob_screens: Mutex::new(HashMap::new()),
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

    /// A click on slot `slot` of the screen `window` of player `uuid`, as the game tells it.
    fn clicked(&self, host: &mut dyn Host, uuid: u128, window: i32, slot: i32, mode: i32) {
        // Plain clicks only: a throw or a drag must not move an organ.
        if !matches!(mode, 0 | 1) {
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

    /// The lines an organ adds to its tooltip, coloured: green what it improves, red what it costs, purple its
    /// abilities, gold what happens without it. None for an item that is not an organ.
    fn tooltip_lines(item: &str) -> Vec<Text> {
        let Some(organ) = organ(item) else { return Vec::new() };
        describe(organ)
            .into_iter()
            .map(|line| {
                let color = match line.tone {
                    Tone::Better => "§a",
                    Tone::Worse => "§c",
                    Tone::Ability => "§d",
                    Tone::Note => "§6",
                };
                let idle = if line.idle { " §8(sem efeito ainda)" } else { "" };
                Text::legacy(&format!("{color}{}{idle}", line.text))
            })
            .collect()
    }

    /// The Chest Opener used by the acting player: their own chest cavity, at the next server tick.
    fn opener_used(&self, host: &mut dyn Host, held: &ItemStack) -> Flow {
        if &*held.item != OPENER {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        trace::line(&*host, format!("the opener in use by {}", p.name));
        self.ask_own(Player::from(&p));
        Flow::Cancel
    }

    /// The screen `window` of `uuid` closed.
    fn closed(&self, uuid: u128, window: i32) {
        if let Some(body) = lock(&self.bodies).get_mut(&uuid).filter(|b| b.menu == Some(window)) {
            body.menu = None;
        }
        let mut screens = lock(&self.mob_screens);
        if screens.get(&uuid).is_some_and(|s| s.window == window) {
            screens.remove(&uuid);
        }
    }
}

impl Hooks for ChestCavity {
    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        for usage in ["cc", "cc show", "cc list", "cc take <slot:integer>", "cc put", "cc ability <name:word>"] {
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
            self.ask_own(Player::from(event.player));
            return Flow::Cancel;
        }
        if sub == Some("ability") {
            self.ask_ability(uuid, arg.unwrap_or(""));
            return Flow::Cancel;
        }
        let lines = self.with(host, uuid, |body, host| match sub {
            None | Some("show") => vec![body.summary()],
            Some("list") => screen::list(&body.cavity),
            Some("reset") => {
                body.reset(host, uuid);
                vec!["Cavidade torácica de volta ao normal".into()]
            }
            Some("take") => match arg.and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=SLOTS).contains(n)) {
                Some(n) if body.cavity.slots[n - 1].is_none() => vec![format!("A ranhura {n} está vazia")],
                Some(n) => vec![body.take_out(host, uuid, n - 1).unwrap_or_else(|| format!("Tiraste a ranhura {n}"))],
                None => vec![format!("/cc take 1-{SLOTS}")],
            },
            Some("put") => vec![body.put_held(host, uuid)],
            Some(other) => vec![format!("Desconhecido: /cc {other} (show, list, take <ranhura>, put, ability <nome>)")],
        });
        for line in lines {
            Self::say(host, uuid, line);
        }
        Flow::Cancel
    }

    /// The Chest Opener in the air: your own chest cavity.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        self.opener_used(host, event.stack)
    }

    /// The same with the crosshair on a block that let the click through.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        self.opener_used(host, event.stack)
    }

    /// The Chest Opener on a mob (right click): its chest cavity, to take organs out of or put others in.
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if !event.stack.is_some_and(|s| &*s.item == OPENER) {
            return Flow::Continue;
        }
        let Some(p) = host.acting_player() else { return Flow::Continue };
        trace::line(&*host, format!("the opener on {} {} by {}", event.kind, event.entity, p.name));
        self.ask_mob(Player::from(&p), event.entity as u32, event.kind);
        Flow::Cancel
    }

    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            self.ticks.fetch_add(1, Ordering::Relaxed);
        }
        self.open_queued(host);
        self.run_abilities(host);
        self.run_combat(host);
    }

    /// A player's blow: what their body adds to it is done at the server tick.
    fn attack_entity(&self, host: &mut dyn Host, event: &mut AttackEntityEvent<'_>) -> Flow {
        if let (Some(p), Ok(target)) = (host.acting_player(), u32::try_from(event.entity)) {
            self.attacked(host, &Player::from(&p), target);
        }
        Flow::Continue
    }

    /// The ability keys, on the window's side: V sets off the utility abilities, R the attacks (as in Chest Cavity).
    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        let which = match event.key {
            "key.keyboard.v" => "utility",
            "key.keyboard.r" => "attack",
            _ => return,
        };
        if event.pressed && host.playing() {
            let _ = host.net_send_to_server(ABILITY_CHANNEL, which.as_bytes());
        }
    }

    /// A player's ability key, from their window.
    fn net_from_client(&self, _host: &mut dyn Host, event: &mut NetFromClientEvent<'_>) {
        if let (ABILITY_CHANNEL, Ok(which)) = (event.channel, std::str::from_utf8(event.data)) {
            self.ask_ability(event.player, which);
        }
    }

    /// A click in a server menu: an organ in the cavity comes out, an organ of yours below goes in.
    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        self.clicked(host, event.uuid, event.window, event.slot, event.mode);
    }
    fn menu_closed(&self, _host: &mut dyn Host, event: &mut MenuClosedEvent) {
        self.closed(event.uuid, event.window);
    }
    /// Once a tick for each player: what they ate and how tired they got; the body's beat comes every second.
    fn player_tick(&self, host: &mut dyn Host, _event: &mut PlayerTickEvent) {
        let Some(p) = host.acting_player() else { return };
        if let Ok(entity) = u32::try_from(p.entity_id) {
            lock(&self.entities).insert(entity, p.uuid);
        }
        self.with(host, p.uuid, |body, host| {
            body.ticks += 1;
            if p.game_mode == 3 {
                return;
            }
            body.feed(host, p.uuid);
            body.fly(host, p.uuid, p.game_mode);
            if body.ticks % 20 == 0 {
                let env = environment::of(host, &p, chestcavity_shared::cavity::get(&body.scores, "crystalsynthesis") > 0.0);
                body.beat(host, &Player::from(&p), &env, &mut lock(&self.rng));
            }
        });
    }

    /// A meal eaten: the stomach and intestines take it from here, at the player's next tick.
    fn living_entity_use_item_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        let (None, Some(p)) = (event.entity, host.acting_player()) else { return };
        let meal = Meal::of(&event.stack.item);
        self.with(host, p.uuid, |body, _| body.ate(meal));
    }

    /// What bones, fireproof and impact resistant organs take off a hit, and the arrows a body steps away from.
    fn living_incoming_damage_v3(&self, event: &mut LivingIncomingDamageV3Event<'_>) -> Flow {
        let Some(uuid) = event.victim_uuid else { return Flow::Continue };
        match self.incoming(uuid, event.damage_type, event.amount) {
            Some(amount) => event.amount = amount,
            None => {
                event.amount = 0.0;
                return Flow::Cancel;
            }
        }
        Flow::Continue
    }

    /// The air a body's lungs lose or win this tick.
    fn living_breathe(&self, event: &mut LivingBreatheEvent<'_>) {
        if event.kind != "minecraft:player" {
            return;
        }
        let Some(uuid) = lock(&self.entities).get(&event.entity).copied() else { return };
        let mut bodies = lock(&self.bodies);
        let Some(body) = bodies.get_mut(&uuid) else { return };
        let Some(lungs) = body.lungs else { return };
        let air = lungs.tick(event.can_breathe, event.consume, event.refill, &mut body.air);
        event.can_breathe = air >= 0;
        event.refill = air.max(0);
        event.consume = (-air).max(0);
    }

    /// A new body, as after a death.
    fn player_respawned(&self, host: &mut dyn Host, event: &mut PlayerRespawnedEvent<'_>) {
        let uuid = event.player;
        self.with(host, uuid, |body, host| body.reset(host, uuid));
    }

    fn player_logged_out(&self, _host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        lock(&self.bodies).remove(&event.player);
        lock(&self.entities).retain(|_, uuid| *uuid != event.player);
        lock(&self.mob_screens).remove(&event.player);
        lock(&self.opens).remove(&event.player);
    }

    /// Organs from the mobs a player kills.
    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let ctx = event.context;
        let (Some((_, mob)), Some(_), false) = (ctx.entity, ctx.player, ctx.used) else { return };
        for (id, n) in drops::for_mob(mob, &mut lock(&self.rng)) {
            event.drops.push(stack(id, n));
        }
    }

    /// What an organ is good and bad at, under its name wherever the game shows its tooltip.
    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        for line in Self::tooltip_lines(&event.stack.item) {
            event.lines.push((line, u32::MAX));
        }
    }
}