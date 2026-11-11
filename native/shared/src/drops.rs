//! Organs from the mobs a player kills: `GeneratedChestCavityType.generateLootDrops`
//! and `drawOrgansFromPile`. The engine calls `for_mob` from the loot hook.
use crate::cavity::{Kind, organ};
use crate::organs::{KINDS, MOBS};

/// Chest Cavity's base chance before the type's `dropRateMultiplier`.
const DROP_CHANCE: f32 = 0.1;

/// A small xorshift, seeded by the engine from the game's clock and positions.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { (self.next() % u64::from(n)) as u32 }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        (self.next() % 1_000_000) as f32 / 1_000_000.0 < p
    }
}

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
        let stack = organ(id).map_or(64, |o| o.stack);
        let n = if count > 1 { 1 + rng.below(stack) } else { 1 };
        out.push((id, n));
    }
    out
}

/// The organs a dead `mob` (`minecraft:zombie`) drops; none for a mob Chest Cavity has no body for.
pub fn for_mob(mob: &str, rng: &mut Rng) -> Vec<(&'static str, u32)> {
    MOBS.iter().find(|(m, _)| *m == mob).map_or_else(Vec::new, |&(_, kind)| roll(&KINDS[kind], rng))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobs_drop_their_own_organs() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        assert!(for_mob("minecraft:not_a_mob", &mut rng).is_empty());
        // Over many deaths a zombie drops organs of its body, with sane counts.
        let &(mob, kind) = MOBS.iter().find(|(m, _)| *m == "minecraft:zombie").expect("zombie");
        let own: Vec<_> = KINDS[kind].slots.iter().map(|s| s.1).collect();
        let mut seen = 0;
        for _ in 0..2000 {
            for (id, n) in for_mob(mob, &mut rng) {
                assert!(own.contains(&id), "{id} is not a zombie organ");
                assert!(n >= 1);
                seen += 1;
            }
        }
        assert!(seen > 0, "a zombie never dropped an organ in 2000 deaths");
    }
}
