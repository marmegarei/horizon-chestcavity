//! Organs from the mobs a player kills: `GeneratedChestCavityType.generateLootDrops` and `drawOrgansFromPile`, and what a
//! cleaver does to them (`processButchering`). The engine calls `for_mob` from the loot hook.
use crate::cavity::{Kind, mob_kind, organ};
use crate::organs::SALVAGE;

/// One of Chest Cavity's salvage recipes (`chestcavity:crafting_salvage`): `required` organs of any of `inputs` are taken
/// apart into `count` of `output`.
pub struct Salvage {
    pub inputs: &'static [&'static str],
    pub required: u32,
    pub output: &'static str,
    pub count: u32,
}

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

/// What decides how often a death drops organs and how many: the killer's Looting, the base chance (`UNIVERSAL_DONOR_RATE`
/// in Chest Cavity's config) and what each level of Looting adds to it (`ORGAN_BUNDLE_LOOTING_BOOST`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Luck {
    pub looting: u32,
    pub base: f32,
    pub boost: f32,
}

impl Default for Luck {
    fn default() -> Luck {
        Luck { looting: 0, base: 0.1, boost: 0.04 }
    }
}

/// (item, count) of the organs one death of `kind` drops.
pub fn roll(kind: &Kind, rng: &mut Rng, luck: Luck) -> Vec<(&'static str, u32)> {
    let rolls = if kind.boss {
        3 + rng.below(2 + luck.looting) + rng.below(2 + luck.looting)
    } else if rng.chance((luck.base + luck.boost * luck.looting as f32) * kind.drop_rate) {
        1 + rng.below(3) + rng.below(3)
    } else {
        return Vec::new();
    };
    let mut pile: Vec<_> = kind.slots.iter().filter(|s| s.1.starts_with("chestcavity:")).copied().collect();
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
pub fn for_mob(mob: &str, rng: &mut Rng, luck: Luck) -> Vec<(&'static str, u32)> {
    mob_kind(mob).map_or_else(Vec::new, |kind| roll(kind, rng, luck))
}

/// What a cleaver makes of the loot: the organs that have a salvage recipe are taken apart (`processButchering`), the
/// first recipe that takes an organ getting it, and a recipe gives `count` for each `required` organs it was given (the
/// rest is lost). The loot that is not salvageable stays as it is.
pub fn butcher(loot: Vec<(String, u32)>) -> Vec<(String, u32)> {
    let mut given = vec![0u32; SALVAGE.len()];
    let mut out = Vec::new();
    for (id, count) in loot {
        match SALVAGE.iter().position(|r| r.inputs.contains(&id.as_str())) {
            Some(r) => given[r] += count,
            None => out.push((id, count)),
        }
    }
    for (recipe, given) in SALVAGE.iter().zip(given) {
        let made = given / recipe.required.max(1) * recipe.count;
        if made > 0 {
            out.push((recipe.output.to_string(), made));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organs::{KINDS, MOBS};

    #[test]
    fn mobs_drop_their_own_organs() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        assert!(for_mob("minecraft:not_a_mob", &mut rng, Luck::default()).is_empty());
        // Over many deaths a zombie drops organs of its body, with sane counts.
        let &(mob, kind) = MOBS.iter().find(|(m, _)| *m == "minecraft:zombie").expect("zombie");
        let own: Vec<_> = KINDS[kind].slots.iter().map(|s| s.1).collect();
        let mut seen = 0;
        for _ in 0..2000 {
            for (id, n) in for_mob(mob, &mut rng, Luck::default()) {
                assert!(own.contains(&id) && id.starts_with("chestcavity:"), "{id} is not a zombie organ");
                assert!(n >= 1);
                seen += 1;
            }
        }
        assert!(seen > 0, "a zombie never dropped an organ in 2000 deaths");
    }

    #[test]
    fn looting_makes_drops_likelier_and_bosses_always_drop() {
        let &(mob, _) = MOBS.iter().find(|(m, _)| *m == "minecraft:zombie").expect("zombie");
        let deaths_with = |looting| {
            let mut rng = Rng(0x2545_f491_4f6c_dd1d);
            (0..20_000).filter(|_| !for_mob(mob, &mut rng, Luck { looting, ..Luck::default() }).is_empty()).count()
        };
        let (plain, looted, cleaved) = (deaths_with(0), deaths_with(3), deaths_with(30));
        assert!(looted > plain * 3 / 2, "Looting III: {looted} drops against {plain}");
        assert!(cleaved > looted * 2, "a cleaver's x10 Looting: {cleaved} drops against {looted}");
        assert!(cleaved < 20_000, "a zombie's chance stays under 1 (its multiplier is under 1)");
        // A boss drops at least three organs, with any luck; a body that has fewer gives all it has.
        let dragon = MOBS.iter().find(|(m, _)| *m == "minecraft:ender_dragon").map(|&(_, k)| &KINDS[k]).expect("dragon");
        let mut rng = Rng(7);
        for _ in 0..200 {
            assert!(roll(dragon, &mut rng, Luck::default()).len() >= 3);
        }
    }

    #[test]
    fn a_cleaver_takes_organs_apart_by_recipe_and_leaves_the_rest() {
        let loot = vec![
            ("chestcavity:muscle".to_string(), 9),
            ("chestcavity:heart".to_string(), 1),
            ("chestcavity:rotten_muscle".to_string(), 5),
            ("minecraft:string".to_string(), 2),
        ];
        let out = butcher(loot);
        // Nine muscles make two portions of meat (4 each) and the odd one is lost; a heart is organ meat.
        assert!(out.contains(&("chestcavity:raw_man_meat".to_string(), 2)), "{out:?}");
        assert!(out.iter().any(|(id, n)| id == "chestcavity:raw_human_organ_meat" && *n == 1), "{out:?}");
        // What no recipe takes (rotten muscle is only for the crafting table) and what is not an organ is left alone.
        assert!(out.contains(&("chestcavity:rotten_muscle".to_string(), 5)));
        assert!(out.contains(&("minecraft:string".to_string(), 2)));
        assert!(!out.iter().any(|(id, _)| id == "chestcavity:muscle" || id == "chestcavity:heart"));
        // Fewer organs than a recipe requires give nothing, and the organs are used up all the same.
        let few = butcher(vec![("chestcavity:muscle".to_string(), 3)]);
        assert!(few.is_empty(), "{few:?}");
    }
}
