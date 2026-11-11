//! Showing a chest cavity. The Chest Opener only asks: the game opens a menu for a player between ticks, not
//! while it handles the click. The server tick shows the screen and then charges what opening a chest costs.
use crate::texts::{item_name_of_mob, msg};
use crate::{ChestCavity, MobScreen, lock, mob, trace};
use chestcavity_shared::cavity::{get, mob_kind};
use chestcavity_shared::open::{self, Verdict};
use horizon_client::mod_api::{EquipmentSlot, Host, NativeServerPlayer, Tag};
use std::sync::atomic::Ordering;

/// What opening a chest cavity costs whoever it is opened on, unless it is easy to get into.
const COST: f32 = 4.0;
/// How many ticks after a click on a mob the item's own use of that click still is part of it.
const SAME_CLICK: u64 = 3;

/// The player who asked, as the game names them.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub uuid: u128,
    pub dimension: String,
    /// Their entity id this session: what the game's hurt and its attacker go by.
    pub entity: i32,
    pub health: f32,
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl From<&NativeServerPlayer> for Player {
    fn from(p: &NativeServerPlayer) -> Self {
        Player { uuid: p.uuid, dimension: p.dimension.clone(), entity: p.entity_id, health: p.health, position: p.position, yaw: p.yaw, pitch: p.pitch }
    }
}

/// Whose chest cavity a player asked to see.
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    Own,
    Mob { entity: u32, kind: String },
}

/// A screen asked for and not yet shown.
pub struct Ask {
    pub open: Open,
    pub by: Player,
}

impl ChestCavity {
    /// A click with the opener on mob `entity` of type `kind`: its screen at the next server tick.
    pub(crate) fn ask_mob(&self, by: Player, entity: u32, kind: &str) {
        lock(&self.mob_clicks).insert(by.uuid, self.ticks.load(Ordering::Relaxed));
        lock(&self.opens).insert(by.uuid, Ask { open: Open::Mob { entity, kind: kind.into() }, by });
    }

    /// The opener used in the air or on a block: your own chest cavity. A click on a mob raises the entity's hook
    /// and then the item's own: that second use must not swap the mob's screen for yours.
    pub(crate) fn ask_own(&self, by: Player) {
        let ticks = self.ticks.load(Ordering::Relaxed);
        if lock(&self.mob_clicks).get(&by.uuid).is_some_and(|&at| ticks <= at + SAME_CLICK) {
            return;
        }
        lock(&self.opens).insert(by.uuid, Ask { open: Open::Own, by });
    }

    /// Shows the screens asked for (the server tick: no player's state is swapped in, so menus open), then
    /// charges what opening cost: the one opened is hurt, unless their chest is easy to get into.
    pub(crate) fn open_queued(&self, host: &mut dyn Host) {
        if lock(&self.opens).is_empty() {
            return;
        }
        let asks: Vec<Ask> = lock(&self.opens).drain().map(|(_, ask)| ask).collect();
        for Ask { open, by } in asks {
            let easy = match &open {
                Open::Own => self.open_own(host, by.uuid),
                Open::Mob { entity, kind } => self.open_mob(host, by.uuid, &by.dimension, *entity, kind),
            };
            trace::line(&*host, format!("open {open:?} for {} ({}): {easy:?}", by.uuid, by.entity));
            if easy != Some(false) {
                continue;
            }
            let (target, attacker, damage) = match open {
                Open::Own => (by.entity, None, "minecraft:generic"),
                Open::Mob { entity, .. } => (entity as i32, Some(by.entity), "minecraft:player_attack"),
            };
            let (Ok(target), attacker) = (u32::try_from(target), attacker.and_then(|a| u32::try_from(a).ok()).filter(|&a| a > 0)) else {
                continue;
            };
            let hurt = host.hurt_entity(&by.dimension, target, COST, damage, attacker);
            trace::line(&*host, format!("hurt {target} by {attacker:?}: {hurt}"));
        }
    }

    /// The player's own cavity: None when the screen did not open, else whether it is easy to get into.
    fn open_own(&self, host: &mut dyn Host, uuid: u128) -> Option<bool> {
        let easy = self.with(host, uuid, |body, host| body.open(host, uuid).then(|| get(&body.scores, "ease_of_access") > 0.0));
        if easy.is_none() {
            Self::say(host, uuid, msg("chestcavity.msg.no_screen"));
        }
        easy
    }

    /// The cavity of mob `entity` of type `kind`, as `open_own`.
    fn open_mob(&self, host: &mut dyn Host, uuid: u128, dimension: &str, entity: u32, kind: &str) -> Option<bool> {
        let Some(body) = mob_kind(kind) else {
            Self::say(host, uuid, msg("chestcavity.msg.mob_none").arg(item_name_of_mob(kind)));
            return None;
        };
        let cfg = self.cfg();
        let Some(cavity) = mob::load(host, entity, kind, cfg.universal_donor_rate) else {
            Self::say(host, uuid, msg("chestcavity.msg.mob_gone").arg(item_name_of_mob(kind)));
            return None;
        };
        let easy = get(&cavity.scores(body), "ease_of_access") > 0.0;
        if let Some((health, max, armoured)) = condition(host, entity) {
            match open::verdict(health, max, armoured, easy, cfg.opener_absolute_health, cfg.opener_fractional_health) {
                Verdict::Open => {}
                Verdict::Obstructed => {
                    host.action_bar(uuid, msg("chestcavity.msg.obstructed").text());
                    return None;
                }
                Verdict::TooHealthy => {
                    host.action_bar(uuid, msg("chestcavity.msg.too_healthy").text());
                    return None;
                }
            }
        }
        let Some(window) = mob::open_menu(host, uuid, &cavity, kind) else {
            Self::say(host, uuid, msg("chestcavity.msg.no_screen"));
            return None;
        };
        lock(&self.mob_screens).insert(uuid, MobScreen { window, entity, mob: kind.to_string(), dimension: dimension.to_string() });
        Some(easy)
    }
}

/// A mob's health and its maximum (when its data says), and whether it wears something over the chest; none when the game
/// does not say (then nothing stops the chest from opening).
fn condition(host: &mut dyn Host, entity: u32) -> Option<(f32, Option<f32>, bool)> {
    let target = host.entity(entity)?;
    let armoured = host.living(target).and_then(|l| l.equipment(&*host, EquipmentSlot::Chest)).is_some();
    let nbt = target.nbt(host)?;
    let health = match nbt.get(&"Health".into())? {
        Tag::Float(bits) => f32::from_bits(*bits),
        Tag::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    };
    let max = match nbt.get(&"attributes".into()) {
        Some(Tag::List(list)) => list.iter().find_map(|a| match a {
            Tag::Compound(c) if matches!(c.get(&"id".into()), Some(Tag::String(s)) if s.to_string_lossy().ends_with("max_health")) => match c.get(&"base".into()) {
                Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
                _ => None,
            },
            _ => None,
        }),
        _ => None,
    };
    Some((health, max, armoured))
}
