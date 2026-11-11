//! Chest Cavity for Horizon: organs in the player's chest, opened with the
//! Chest Opener, swapped by hand, and what having (or missing) them does;
//! organs from the mobs the player kills. Content (items, textures, recipes)
//! comes from `tools/import.py`; this is the part that needs code. Everything
//! the game would do natively runs as vanilla commands as the player, so the
//! world needs cheats.
mod cavity;
mod drops;
mod effects;
mod organs;
mod screen;

use cavity::{Cavity, Scores, player_kind};
use horizon_mod_sdk::{Mod, Registration, chat, command, game, input, storage};

const OPENER: &str = "chestcavity:chest_opener";
/// The world's id lives in this score, so each world keeps its own cavity.
const WORLD_OBJECTIVE: &str = "cc_world";

struct ChestCavity {
    /// The world's id once read back from its scoreboard; None on the title
    /// screen and while it is being read.
    world: Option<u32>,
    /// Ticks since joining the world; the id is asked for on the first.
    in_world: u64,
    cavity: Cavity,
    scores: Scores,
    normal: Scores,
    dead: bool,
    bleed: u32,
    rng: drops::Rng,
    screen: screen::Screen,
}

impl ChestCavity {
    fn key(&self) -> Option<String> {
        self.world.map(|w| format!("w{w}"))
    }

    fn load(&mut self) {
        let stored = self.key().and_then(|k| storage::get(&k)).and_then(|v| String::from_utf8(v).ok());
        self.cavity = stored.map_or_else(|| Cavity::of(player_kind()), |t| Cavity::from_text(&t));
        self.changed();
    }

    /// The cavity changed: new scores, attributes and saved state.
    fn changed(&mut self) {
        self.scores = self.cavity.scores(player_kind());
        command::run_quiet(&effects::attributes(&self.scores, &self.normal));
        if let Some(k) = self.key() {
            let _ = storage::set(&k, self.cavity.to_text().as_bytes());
        }
        if self.screen.open {
            self.screen.show(&self.cavity, &self.scores, &self.normal);
        }
    }

    fn joined(&mut self, tick: u64) {
        let p = game::get_player();
        let seed = tick ^ p.as_ref().map_or(0, |p| (p.x * 1000.0) as i64 as u64 ^ ((p.z * 1000.0) as i64 as u64) << 20);
        self.rng = drops::Rng(seed | 1);
        let token = (self.rng.next() % 2_000_000_000) as u32 + 1;
        let mut lines = vec![
            format!("scoreboard objectives add {WORLD_OBJECTIVE} dummy"),
            format!("execute unless score #cc {WORLD_OBJECTIVE} matches 1.. run scoreboard players set #cc {WORLD_OBJECTIVE} {token}"),
        ];
        lines.extend(drops::setup());
        command::run_quiet(&lines);
    }

    fn take(&mut self) {
        let Some(i) = self.screen.selected else { return };
        let Some(slot) = self.cavity.slots[i].take() else { return };
        command::run_quiet(&[format!("give @s {} {}", slot.id, slot.count)]);
        self.changed();
    }

    fn put(&mut self) {
        let Some(i) = self.screen.selected else { return };
        if self.cavity.slots[i].is_some() {
            return chat::show("That slot is taken: take its organ out first");
        }
        let Some(held) = input::held() else {
            return chat::show("Hold an organ in your main hand to put it in");
        };
        let Some(organ) = cavity::organ(&held.id) else {
            return chat::show(&format!("{} is not an organ", held.id));
        };
        let count = held.count.min(organ.stack);
        command::run_quiet(&[format!("clear @s {} {count}", held.id)]);
        self.cavity.slots[i] = Some(cavity::Slot { id: held.id, count });
        self.changed();
    }
}

impl Mod for ChestCavity {
    fn init() -> Result<(Self, Registration), String> {
        input::claim_items(&[OPENER.to_string()]);
        let normal = Cavity::of(player_kind()).scores(player_kind());
        let cc = ChestCavity {
            world: None,
            in_world: 0,
            cavity: Cavity::of(player_kind()),
            scores: normal.clone(),
            normal,
            dead: false,
            bleed: 0,
            rng: drops::Rng(0x9e37_79b9_7f4a_7c15),
            screen: screen::Screen::default(),
        };
        Ok((cc, Registration::new().on_tick().on_chat().command("cc")))
    }

    fn tick(&mut self, tick: u64) {
        let Some(player) = game::get_player() else {
            // Title screen: the next world is another one.
            self.world = None;
            self.in_world = 0;
            return;
        };
        if self.in_world == 0 {
            self.joined(tick);
        }
        self.in_world += 1;
        for event in input::poll() {
            if let input::Event::UseStart(_) = event {
                // ChestOpener: opening a chest hurts (4) unless it is easy to get into.
                if cavity::get(&self.scores, "ease_of_access") <= 0.0 {
                    command::run_quiet(&["damage @s 4 minecraft:generic".to_string()]);
                }
                self.screen.open = true;
                self.screen.show(&self.cavity, &self.scores, &self.normal);
            }
        }
        // Death and respawn: the body comes back with a fresh set of organs.
        if player.health <= 0.0 {
            self.dead = true;
        } else if self.dead {
            self.dead = false;
            self.cavity = Cavity::of(player_kind());
            self.changed();
        }
        if self.in_world % 20 != 0 {
            return;
        }
        if self.world.is_none() {
            // Quiet commands run after this tick, so ask once a second until
            // the answer comes back through `chat` as "#cc has <id> [cc_world]".
            return command::run(&format!("scoreboard players get #cc {WORLD_OBJECTIVE}"));
        }
        if self.dead {
            return;
        }
        let seconds = self.in_world / 20;
        let mut lines = effects::periodic(&self.scores, &self.normal, seconds, &mut self.bleed);
        lines.extend(drops::batch(&mut self.rng));
        command::run_quiet(&lines);
    }

    fn chat(&mut self, text: &str) {
        if self.world.is_some() || !text.contains(WORLD_OBJECTIVE) || !text.contains("#cc") {
            return;
        }
        let id = text.split(|c: char| !c.is_ascii_digit()).find_map(|w| w.parse::<u32>().ok());
        if let Some(id) = id {
            self.world = Some(id);
            self.load();
        }
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let args = args.trim();
        // The Take button without the screen: tests, and a way out when it won't click.
        if let Some(n) = args.strip_prefix("take ") {
            let i = n.trim().parse::<usize>().ok().filter(|i| (1..=cavity::SLOTS).contains(i));
            let Some(i) = i else { return Err(format!("/cc take 1-{}", cavity::SLOTS)) };
            if self.cavity.slots[i - 1].is_none() {
                return Err(format!("Slot {i} is empty"));
            }
            self.screen.selected = Some(i - 1);
            self.take();
            return Ok(Some(format!("Took slot {i} out")));
        }
        match args {
            "reset" => {
                self.cavity = Cavity::of(player_kind());
                self.changed();
                Ok(Some("Chest cavity back to normal".into()))
            }
            "" | "show" => Ok(Some(screen::summary(&self.scores, &self.normal))),
            other => Err(format!("Unknown: /cc {other} (show, reset, take <slot>)")),
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            screen::LIST => self.screen.selected = Some(row as usize),
            screen::TAKE => return self.take(),
            screen::PUT => return self.put(),
            _ => return,
        }
        self.screen.show(&self.cavity, &self.scores, &self.normal);
    }

    fn ui_closed(&mut self) {
        self.screen.open = false;
    }
}

horizon_mod_sdk::export_mod!(ChestCavity);
