//! A chest cavity: 27 slots of organs and the scores they add up to, as
//! Chest Cavity's `ChestCavityUtil.evaluateChestCavity` counts them.
use crate::organs::{KINDS, MOBS, ORGANS, PLAYER};
use std::collections::BTreeMap;

pub const SLOTS: usize = 27;

/// An item with organ scores: one of the mods' organs, or a vanilla
/// pseudo-organ (dirt, bone, ...).
pub struct Organ {
    pub id: &'static str,
    pub name: &'static str,
    /// Its max stack size: a slot scores `count / stack` of its values.
    pub stack: u32,
    pub scores: &'static [(&'static str, f32)],
}

/// A chest cavity type: the organs a body starts with.
pub struct Kind {
    /// (slot, item, count).
    pub slots: &'static [(u8, &'static str, u32)],
    /// Scores the body has whatever its organs.
    pub base: &'static [(&'static str, f32)],
    pub drop_rate: f32,
    /// Always drops organs, and more of them.
    pub boss: bool,
}

pub type Scores = BTreeMap<&'static str, f32>;

/// Every door and trapdoor: Chest Cavity registers them as organs by tag (`#minecraft:doors`, `#minecraft:trapdoors`), each
/// worth a full point of ease of access (a score of one oak door's stack size, 64, shared by its 64).
static DOOR: Organ = Organ { id: "minecraft:door", name: "Door", stack: 64, scores: &[("ease_of_access", 64.0)] };

pub fn organ(id: &str) -> Option<&'static Organ> {
    ORGANS.binary_search_by(|o| o.id.cmp(id)).ok().map(|i| &ORGANS[i]).or_else(|| {
        let name = id.strip_prefix("minecraft:")?;
        (name.ends_with("_door") || name.ends_with("_trapdoor")).then_some(&DOOR)
    })
}

pub fn player_kind() -> &'static Kind {
    &KINDS[PLAYER]
}

/// The chest cavity type of a mob (`minecraft:cow`); none for a mob Chest Cavity has no body for.
pub fn mob_kind(mob: &str) -> Option<&'static Kind> {
    MOBS.iter().find(|(m, _)| *m == mob).map(|&(_, k)| &KINDS[k])
}

/// "Cave Spider" for `minecraft:cave_spider`.
pub fn mob_name(mob: &str) -> String {
    let path = mob.rsplit(':').next().unwrap_or(mob);
    path.split('_')
        .map(|w| w.chars().next().map_or_else(String::new, |c| c.to_uppercase().chain(w.chars().skip(1)).collect()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub id: String,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cavity {
    pub slots: Vec<Option<Slot>>,
}

impl Cavity {
    pub fn of(kind: &Kind) -> Self {
        let mut slots = vec![None; SLOTS];
        for &(pos, id, count) in kind.slots {
            slots[pos as usize] = Some(Slot { id: id.into(), count });
        }
        Cavity { slots }
    }

    pub fn scores(&self, kind: &Kind) -> Scores {
        let mut scores: Scores = kind.base.iter().copied().collect();
        for slot in self.slots.iter().flatten() {
            let Some(o) = organ(&slot.id) else { continue };
            let share = (slot.count as f32 / o.stack as f32).min(1.0);
            for &(k, v) in o.scores {
                *scores.entry(k).or_default() += v * share;
            }
        }
        scores
    }

    /// One line a slot, `<slot> <id> <count>`, for the mod's storage.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.slots.iter().enumerate() {
            if let Some(s) = s {
                out += &format!("{i} {} {}\n", s.id, s.count);
            }
        }
        out
    }

    pub fn from_text(text: &str) -> Self {
        let mut slots = vec![None; SLOTS];
        for line in text.lines() {
            let mut w = line.split(' ');
            let (Some(i), Some(id), Some(n)) = (w.next(), w.next(), w.next()) else { continue };
            if let (Ok(i), Ok(count)) = (i.parse::<usize>(), n.parse()) {
                if i < SLOTS {
                    slots[i] = Some(Slot { id: id.into(), count });
                }
            }
        }
        Cavity { slots }
    }
}

/// A score, 0 when absent.
pub fn get(scores: &Scores, key: &str) -> f32 {
    scores.get(key).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobs_have_their_own_bodies() {
        assert_eq!(mob_name("minecraft:cave_spider"), "Cave Spider");
        assert!(mob_kind("minecraft:zombie").is_some_and(|k| !k.slots.is_empty()));
        assert!(mob_kind("minecraft:villager_of_nowhere").is_none());
        // A door is an organ too: one of them makes a chest easy to open.
        let mut cavity = Cavity::of(player_kind());
        cavity.slots[6] = Some(Slot { id: "minecraft:oak_door".into(), count: 1 });
        assert_eq!(get(&cavity.scores(player_kind()), "ease_of_access"), 1.0);
        assert!(organ("minecraft:iron_trapdoor").is_some() && organ("minecraft:stone").is_none());
        // Every kind's own cavity fits its 27 slots.
        for (_, k) in MOBS {
            assert!(KINDS[*k].slots.iter().all(|&(pos, ..)| (pos as usize) < SLOTS));
        }
    }
}
