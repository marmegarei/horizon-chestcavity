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
    /// Items that are worth other scores in this type only (its creeper's trunk is its heart and spine): an item id,
    /// or `*_suffix` for a family of them, and the scores of a full stack. They replace the item's own.
    pub special: &'static [(&'static str, &'static [(&'static str, f32)])],
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
    /// The body the organ was born in (its uuid): it fits that body and is rejected by any other. None for an organ that fits
    /// any body: one a mob dropped, or a donor's.
    pub owner: Option<u128>,
}

impl Slot {
    pub fn new(id: impl Into<String>, count: u32) -> Slot {
        Slot { id: id.into(), count, owner: None }
    }

    pub fn owned_by(mut self, owner: Option<u128>) -> Slot {
        self.owner = owner;
        self
    }
}

/// How well an organ fits a body (`ChestCavityUtil.getCompatibilityLevel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// It is another body's: rejected.
    Rejected,
    /// It fits any body.
    Universal,
    /// It was born in this body.
    Own,
}

/// Whether `id` is one of the mod's own organs, which can be rejected (the stand-ins of vanilla items, bones and
/// gunpowder, are never).
pub fn is_true_organ(id: &str) -> bool {
    id.starts_with("chestcavity:")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cavity {
    pub slots: Vec<Option<Slot>>,
}

impl Cavity {
    pub fn of(kind: &Kind) -> Self {
        let mut slots = vec![None; SLOTS];
        for &(pos, id, count) in kind.slots {
            slots[pos as usize] = Some(Slot::new(id, count));
        }
        Cavity { slots }
    }

    /// The organs that have no owner are born in the body of `owner`, but for the ones `universal` says fit any body.
    pub fn born(mut self, owner: u128, mut universal: impl FnMut() -> bool) -> Self {
        for slot in self.slots.iter_mut().flatten().filter(|s| s.owner.is_none()) {
            if !universal() {
                slot.owner = Some(owner);
            }
        }
        self
    }

    /// How the organ in slot `i` fits the body of `body`.
    pub fn fit(&self, i: usize, body: u128) -> Option<Fit> {
        let slot = self.slots.get(i)?.as_ref()?;
        Some(match slot.owner {
            None => Fit::Universal,
            Some(o) if o == body => Fit::Own,
            Some(_) => Fit::Rejected,
        })
    }

    /// What a body gives up when it dies (`dropUnboundOrgans`): everything that is not bound to it, another body's and no
    /// one's, taken out of the cavity. What is its own stays, to be lost with the body.
    pub fn drop_unbound(&mut self, body: u128) -> Vec<Slot> {
        let mut out = Vec::new();
        for i in 0..self.slots.len() {
            if self.fit(i, body) != Some(Fit::Own) {
                out.extend(self.slots[i].take());
            }
        }
        out
    }

    /// The organs a body that keeps its cavity through death is given when it lacks what keeps one going
    /// (`insertWelfareOrgans`): a rotten heart, lung and spine and some rotten flesh. They fit any body. Each goes in the first
    /// free slot, or over the one Chest Cavity names when the cavity is full (ponytail: what it replaces is lost).
    pub fn welfare(&mut self, kind: &Kind) {
        const WELFARE: [(&str, &str, u32, usize); 4] = [
            ("health", "chestcavity:rotten_heart", 1, 4),
            ("breath_recovery", "chestcavity:rotten_lung", 1, 3),
            ("nerves", "chestcavity:rotten_spine", 1, 13),
            ("strength", "minecraft:rotten_flesh", 16, 0),
        ];
        for (score, id, count, slot) in WELFARE {
            if get(&self.scores(kind), score) <= 0.0 {
                let at = self.slots.iter().position(Option::is_none).unwrap_or(slot);
                self.slots[at] = Some(Slot::new(id, count));
            }
        }
    }

    /// How many of the mod's own organs here the body of `body` rejects (the `incompatibility` score).
    pub fn incompatibility(&self, body: u128) -> usize {
        (0..self.slots.len()).filter(|&i| self.fit(i, body) == Some(Fit::Rejected) && self.slots[i].as_ref().is_some_and(|s| is_true_organ(&s.id))).count()
    }

    pub fn scores(&self, kind: &Kind) -> Scores {
        let mut scores: Scores = kind.base.iter().copied().collect();
        for slot in self.slots.iter().flatten() {
            let special = kind.special.iter().find(|(pat, _)| pat.strip_prefix('*').map_or(*pat == slot.id, |end| slot.id.ends_with(end)));
            let (own, stack) = match (special, organ(&slot.id)) {
                (Some(&(_, own)), _) => (own, 64),
                (None, Some(o)) => (o.scores, o.stack),
                (None, None) => continue,
            };
            let share = (slot.count as f32 / stack as f32).min(1.0);
            for &(k, v) in own {
                *scores.entry(k).or_default() += v * share;
            }
        }
        scores
    }

    /// One line a slot, `<slot> <id> <count>` and the owner (32 hex digits) when it has one, for the mod's storage.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.slots.iter().enumerate() {
            if let Some(s) = s {
                out += &format!("{i} {} {}", s.id, s.count);
                if let Some(o) = s.owner {
                    out += &format!(" {o:032x}");
                }
                out.push('\n');
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
                    let owner = w.next().and_then(|o| u128::from_str_radix(o, 16).ok());
                    slots[i] = Some(Slot::new(id, count).owned_by(owner));
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
    fn a_body_that_dies_gives_up_what_is_not_its_own_and_a_cavity_with_no_heart_is_given_a_rotten_one() {
        let kind = player_kind();
        let mut cavity = Cavity::of(kind).born(7, || false);
        let heart = cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart");
        let liver = cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:liver")).expect("a liver");
        cavity.slots[heart] = Some(Slot::new("chestcavity:heart", 1)); // no one's
        cavity.slots[liver] = Some(Slot::new("chestcavity:liver", 1).owned_by(Some(9))); // another body's
        let gone = cavity.drop_unbound(7);
        assert_eq!(gone.len(), 2);
        assert!(gone.iter().any(|s| s.id == "chestcavity:heart" && s.owner.is_none()));
        assert!(gone.iter().any(|s| s.id == "chestcavity:liver" && s.owner == Some(9)), "an organ keeps the body it was born in");
        assert!(cavity.slots[heart].is_none() && cavity.slots[liver].is_none());
        assert!(cavity.slots.iter().flatten().all(|s| s.owner == Some(7)), "its own stay");
        // With no heart, lung or spine, the body is given the rotten ones; a body that has them is left alone.
        assert_eq!(get(&cavity.scores(kind), "health"), 0.0);
        let before = cavity.clone();
        cavity.welfare(kind);
        assert!(cavity.slots.iter().flatten().any(|s| s.id == "chestcavity:rotten_heart" && s.owner.is_none()));
        assert!(get(&cavity.scores(kind), "health") > 0.0);
        let mut whole = Cavity::of(kind);
        let same = whole.clone();
        whole.welfare(kind);
        assert_eq!(whole, same);
        assert_ne!(before, cavity);
    }

    #[test]
    fn mobs_have_their_own_bodies() {
        assert_eq!(mob_name("minecraft:cave_spider"), "Cave Spider");
        assert!(mob_kind("minecraft:zombie").is_some_and(|k| !k.slots.is_empty()));
        assert!(mob_kind("minecraft:villager_of_nowhere").is_none());
        // A creeper's trunk is its heart, and its spine: without it, it has neither.
        let creeper = mob_kind("minecraft:creeper").expect("creeper");
        let mut cavity = Cavity::of(creeper);
        let trunk = cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "minecraft:oak_log")).expect("a trunk");
        assert!(get(&cavity.scores(creeper), "health") > 0.0 && get(&cavity.scores(creeper), "nerves") > 0.0);
        cavity.slots[trunk] = None;
        assert_eq!(get(&cavity.scores(creeper), "health"), 0.0);
        assert_eq!(get(&cavity.scores(creeper), "nerves"), 0.0);
        // Organs are born in a body and fit only it; what has no owner fits any.
        let mine = Cavity::of(player_kind()).born(7, || false);
        assert!(mine.slots.iter().flatten().all(|s| s.owner == Some(7)));
        assert_eq!(mine.incompatibility(7), 0);
        assert_eq!(mine.incompatibility(8), mine.slots.iter().flatten().filter(|s| is_true_organ(&s.id)).count(), "to anyone else they are strangers");
        assert_eq!(Cavity::from_text(&mine.to_text()), mine, "the owner is kept in the storage");
        assert_eq!(Cavity::from_text("3 chestcavity:heart 1\n").slots[3], Some(Slot::new("chestcavity:heart", 1)), "storage from before owners");
        let donors = Cavity::of(player_kind()).born(7, || true);
        assert_eq!(donors.incompatibility(8), 0);
        assert_eq!(donors.fit(4, 8), Some(Fit::Universal));
        // A door is an organ too: one of them makes a chest easy to open.
        let mut cavity = Cavity::of(player_kind());
        cavity.slots[6] = Some(Slot::new("minecraft:oak_door", 1));
        assert_eq!(get(&cavity.scores(player_kind()), "ease_of_access"), 1.0);
        assert!(organ("minecraft:iron_trapdoor").is_some() && organ("minecraft:stone").is_none());
        // Every kind's own cavity fits its 27 slots.
        for (_, k) in MOBS {
            assert!(KINDS[*k].slots.iter().all(|&(pos, ..)| (pos as usize) < SLOTS));
        }
    }
}
