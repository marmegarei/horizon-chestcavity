//! Organs from the mobs the player kills. Horizon's selectors never see a
//! dead mob and its `/loot` knows no mod tables, so each mob type gets a
//! `minecraft.killed:<mob>` objective; once a second the mod rolls the drops
//! for every type and hands them over where that score went up, then resets it.
//! ponytail: drops come at the player's feet, one roll per type per second.
use crate::cavity::Kind;
use crate::organs::{KINDS, MOBS};

/// Chest Cavity's base chance before the type's `dropRateMultiplier`.
const DROP_CHANCE: f32 = 0.1;

pub fn objective(i: usize) -> String {
    format!("cck{i}")
}

/// Creates the kill objectives, once a world.
pub fn setup() -> Vec<String> {
    MOBS.iter()
        .enumerate()
        // Stat criteria spell the id with a dot: minecraft.killed:minecraft.zombie.
        .map(|(i, (mob, _))| format!("scoreboard objectives add {} minecraft.killed:{}", objective(i), mob.replace(':', ".")))
        .collect()
}

/// A small xorshift: the sandbox has no clock or OS randomness.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { (self.next() % n as u64) as u32 }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        (self.next() % 1_000_000) as f32 / 1_000_000.0 < p
    }
}

/// `GeneratedChestCavityType.generateLootDrops` + `drawOrgansFromPile`:
/// (item, count) of the organs one death of `kind` drops.
pub fn roll(kind: &Kind, rng: &mut Rng) -> Vec<(&'static str, u32)> {
    let rolls = if kind.boss {
        3 + rng.below(2) + rng.below(2)
    } else if rng.chance(DROP_CHANCE * kind.drop_rate) {
        1 + rng.below(3) + rng.below(3)
    } else {
        return Vec::new();
    };
    let mut pile: Vec<_> = kind.slots.to_vec();
    let mut out = Vec::new();
    for _ in 0..rolls {
        if pile.is_empty() {
            break;
        }
        let (_, id, count) = pile.remove(rng.below(pile.len() as u32) as usize);
        let stack = crate::cavity::organ(id).map_or(64, |o| o.stack);
        let n = if count > 1 { 1 + rng.below(stack) } else { 1 };
        out.push((id, n));
    }
    out
}

/// The second's batch: drops where a kill was counted, then every score back to 0.
pub fn batch(rng: &mut Rng) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, &(_, kind)) in MOBS.iter().enumerate() {
        let obj = objective(i);
        for (id, n) in roll(&KINDS[kind], rng) {
            lines.push(format!(
                "execute if score @s {obj} matches 1.. run summon item ~ ~ ~ {{Item:{{id:\"{id}\",count:{n}}}}}"
            ));
        }
    }
    for i in 0..MOBS.len() {
        lines.push(format!("scoreboard players set @s {} 0", objective(i)));
    }
    lines
}
