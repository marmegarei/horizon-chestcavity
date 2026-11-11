//! Astro Adds' organs that work on the other organs (`AstralCavityUtil`): regrowth fills the empty slots, autophagy eats the
//! organs that are food, tumors grow where something was eaten, hunters clear the harmful ones and the sculk infection turns
//! food into sculk tumors. Rules on a `Cavity`; the engine times them and feeds the body.
use crate::cavity::{Cavity, Kind, SLOTS, Slot, organ};
use crate::drops::Rng;

/// Ticks between each of them (`CCAAConfig`: `REGROWTH_COOLDOWN`, `AUTOPHAGY_COOLDOWN`, `TUMOR_HUNTING_COOLDOWN`,
/// `SCULK_INFECTION_COOLDOWN`).
pub const REGROWTH: u64 = 400;
pub const AUTOPHAGY: u64 = 450;
pub const HUNTING: u64 = 250;
pub const INFECTION: u64 = 500;

const BENIGN: &str = "chestcavity:benign_tumor";
const AUTOPHAGY_TUMOR: &str = "chestcavity:autophagy_tumor";
const SCULK_TUMOR: &str = "chestcavity:sculk_tumor";

/// The eight parts a body's organs come in.
const PARTS: [&str; 8] = ["appendix", "heart", "intestine", "kidney", "liver", "lung", "spleen", "stomach"];

/// What eating one of an organ gives (`CCFoodComponents`): its food points and saturation modifier (a point of food with a
/// modifier of 0.6 gives 1.2 of saturation). None for what is not food.
pub fn food_of(id: &str) -> Option<(i32, f32)> {
    match id {
        "minecraft:beef" => return Some((3, 0.3)),
        "minecraft:mutton" => return Some((2, 0.3)),
        "minecraft:rotten_flesh" => return Some((4, 0.1)),
        _ => {}
    }
    let name = id.strip_prefix("chestcavity:")?;
    let part_of = |prefixes: &[&str]| prefixes.iter().any(|p| name.strip_prefix(p).is_some_and(|rest| PARTS.contains(&rest)));
    Some(match name {
        "muscle" => (2, 0.4),
        "rotten_muscle" => (1, 0.1),
        "animal_muscle" | "aquatic_muscle" | "brutish_muscle" | "fireproof_muscle" | "fish_muscle" | "springy_muscle" | "swift_muscle" | "ender_muscle" | "insect_muscle" | "dragon_muscle" | "dragon_heart" => (1, 0.4),
        "rabbit_heart" | "small_animal_muscle" | "small_aquatic_muscle" | "small_fish_muscle" | "small_springy_muscle" | "small_gills" => (1, 0.2),
        "small_carnivore_intestine" | "small_carnivore_stomach" | "small_herbivore_intestine" | "small_herbivore_stomach" => (1, 0.2),
        "carnivore_intestine" | "carnivore_stomach" | "herbivore_intestine" | "herbivore_rumen" | "herbivore_stomach" | "gills" | "llama_lung" => (2, 0.6),
        "insect_caeca" | "insect_heart" | "insect_intestine" | "insect_lung" | "insect_stomach" | "silk_gland" => (2, 0.6),
        "dragon_appendix" | "dragon_kidney" | "dragon_liver" | "dragon_lung" | "dragon_spleen" | "mana_reactor" => (2, 0.6),
        _ if PARTS.contains(&name) => (3, 0.6),
        _ if part_of(&["rotten_"]) => (4, 0.1),
        _ if part_of(&["ender_"]) => (2, 0.6),
        _ if part_of(&["animal_", "fireproof_"]) => (2, 0.6),
        _ if part_of(&["small_animal_"]) => (1, 0.2),
        _ => return None,
    })
}

fn tumor(rng: &mut Rng) -> Slot {
    Slot::new(if rng.chance(0.5) { BENIGN } else { AUTOPHAGY_TUMOR }, 1)
}

/// Regrowth (`growBackOrgans`): the first `n` empty slots get the organ their type has there, full; where the type has none,
/// or one time in four, or where its organ is dirt, a tumor. Returns what it costs a player in exhaustion: 0.25, and for a
/// food organ 0.25 for each of the stack and each point of food and saturation it would give. `owner` is whose the new organs
/// are (None for a body that tags none). Only the slots of the type's own cavity grow back: a fourth row is the player's
/// to keep as they like.
pub fn grow_back(cavity: &mut Cavity, kind: &Kind, n: u32, rng: &mut Rng, owner: Option<u128>) -> f32 {
    let (mut left, mut tired) = (n, 0.0);
    for i in 0..cavity.slots.len().min(SLOTS) {
        if left == 0 {
            break;
        }
        if cavity.slots[i].is_some() {
            continue;
        }
        let own = kind.slots.iter().find(|s| s.0 as usize == i).map(|s| s.1).filter(|id| *id != "minecraft:dirt");
        let slot = match own {
            Some(id) if !rng.chance(0.25) => Slot::new(id, organ(id).map_or(64, |o| o.stack)),
            _ => tumor(rng),
        };
        tired += match food_of(&slot.id) {
            Some((food, modifier)) => 0.25 * slot.count as f32 * (food as f32 + modifier),
            None => 0.25,
        };
        cavity.slots[i] = Some(slot.owned_by(owner));
        left -= 1;
    }
    tired
}

/// What became of a unit of food taken from the cavity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Used {
    /// It fed the body.
    Fed,
    /// The body was full, and it mended 1 point of health.
    Healed,
    /// The body needed neither: the unit stays.
    Spared,
}

/// Autophagy (`eatOrgans`): the body eats up to `n` units of the food organs in its cavity, first slot first; `eat` is told
/// each unit's item and says what became of it. A stack that is used up leaves a tumor when the eater is a tumor (`tumorous`)
/// and one time in two otherwise, else the slot is left empty. The loop reads the stack's count again at each turn, so a stack
/// of 16 gives about 8 units in one go.
pub fn eat_organs(cavity: &mut Cavity, n: u32, tumorous: bool, rng: &mut Rng, mut eat: impl FnMut(&str) -> Used) {
    let mut left = n;
    for i in 0..cavity.slots.len() {
        if left == 0 {
            break;
        }
        let Some(mut stack) = cavity.slots[i].clone().filter(|s| food_of(&s.id).is_some()) else { continue };
        let mut turn = 0;
        while turn < stack.count && left > 0 {
            if eat(&stack.id) != Used::Spared {
                stack.count -= 1;
                left -= 1;
                if stack.count == 0 && (tumorous || rng.chance(0.5)) {
                    // A tumor is not food: the slot is done.
                    cavity.slots[i] = Some(tumor(rng).owned_by(stack.owner));
                    break;
                }
                cavity.slots[i] = (stack.count > 0).then(|| stack.clone());
            }
            turn += 1;
        }
    }
}

/// Tumor hunting (`huntTumors`): the first `n` harmful tumors (autophagy and sculk) leave the cavity. A benign one stays.
pub fn hunt_tumors(cavity: &mut Cavity, n: u32) {
    let mut left = n;
    for slot in cavity.slots.iter_mut() {
        if left > 0 && slot.as_ref().is_some_and(|s| s.id == AUTOPHAGY_TUMOR || s.id == SCULK_TUMOR) {
            *slot = None;
            left -= 1;
        }
    }
}

/// Sculk infection (`infectOrgans`): the first `n` slots that hold food or a benign or autophagy tumor become one sculk tumor
/// each, whatever was in them.
pub fn infect(cavity: &mut Cavity, n: u32) {
    let mut left = n;
    for slot in cavity.slots.iter_mut() {
        if left > 0 && slot.as_ref().is_some_and(|s| food_of(&s.id).is_some() || s.id == BENIGN || s.id == AUTOPHAGY_TUMOR) {
            *slot = Some(Slot::new(SCULK_TUMOR, 1).owned_by(slot.as_ref().and_then(|s| s.owner)));
            left -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{SLOTS, player_kind};
    use crate::organs::ORGANS;

    fn at(cavity: &Cavity, i: usize) -> Option<&str> {
        cavity.slots[i].as_ref().map(|s| s.id.as_str())
    }

    #[test]
    fn ninety_organs_are_food_and_the_rest_are_not() {
        let food = ORGANS.iter().filter(|o| food_of(o.id).is_some()).count();
        assert_eq!(food, 90 + 3, "Astro's list: 90 organs and the pseudo-organs beef, mutton and rotten flesh");
        assert_eq!(food_of("chestcavity:heart"), Some((3, 0.6)));
        assert_eq!(food_of("chestcavity:rotten_heart"), Some((4, 0.1)));
        assert_eq!(food_of("chestcavity:small_animal_muscle"), Some((1, 0.2)));
        assert_eq!(food_of("chestcavity:dragon_heart"), Some((1, 0.4)));
        for not in ["chestcavity:rib", "chestcavity:benign_tumor", "chestcavity:spine", "minecraft:bone", "chestcavity:dragon_heart_of_nothing"] {
            assert_eq!(food_of(not), None, "{not}");
        }
    }

    #[test]
    fn regrowth_fills_the_empty_slots_in_order_with_what_the_type_has_there() {
        let kind = player_kind();
        let mut cavity = Cavity::of(kind);
        let (heart, muscle) = (4, 0);
        cavity.slots[muscle] = None;
        cavity.slots[heart] = None;
        let first_free = (0..SLOTS).find(|&i| cavity.slots[i].is_none() && !kind.slots.iter().any(|s| s.0 as usize == i)).expect("a slot the type does not use");
        let mut rng = Rng(7);
        // One at a time, from the first slot: a muscle, full (16), costing 0.25 × 16 × (2 + 0.4).
        let tired = grow_back(&mut cavity, kind, 1, &mut rng, None);
        let slot = cavity.slots[muscle].as_ref().expect("regrown");
        if slot.id == "chestcavity:muscle" {
            assert_eq!(slot.count, 16, "a stack grows full");
            assert!((tired - 9.6).abs() < 1e-4, "{tired}");
        } else {
            assert!(slot.id.ends_with("_tumor") && tired == 0.25, "one time in four it is a tumor instead");
        }
        assert!(cavity.slots[heart].is_none(), "one slot for each point");
        // Enough points fill them all, the slots the type does not use with tumors.
        let mut full = cavity.clone();
        grow_back(&mut full, kind, 100, &mut rng, Some(5));
        assert!(full.slots.iter().all(Option::is_some));
        assert!(at(&full, first_free).is_some_and(|id| id.ends_with("_tumor")));
        assert!(full.slots[heart].as_ref().is_some_and(|s| s.owner == Some(5)), "they are whose the engine says");
        // A fourth row is left as the player has it.
        let mut grown = Cavity::of(kind).grown();
        grow_back(&mut grown, kind, 100, &mut rng, None);
        assert!(grown.slots[SLOTS..].iter().all(Option::is_none));
    }

    #[test]
    fn autophagy_eats_food_organs_a_unit_at_a_time_and_leaves_tumors_behind() {
        let kind = player_kind();
        let first_food = |c: &Cavity| c.slots.iter().position(|s| s.as_ref().is_some_and(|s| food_of(&s.id).is_some())).expect("some food");
        let whole = Cavity::of(kind);
        let muscle = first_food(&whole);
        assert_eq!(at(&whole, muscle), Some("chestcavity:muscle"));
        let count = whole.slots[muscle].as_ref().expect("a muscle").count;

        // Three units come from the first food slot.
        let (mut cavity, mut eaten) = (whole.clone(), Vec::new());
        eat_organs(&mut cavity, 3, false, &mut Rng(3), |id| {
            eaten.push(id.to_string());
            Used::Fed
        });
        assert_eq!(eaten, ["chestcavity:muscle"; 3]);
        assert_eq!(cavity.slots[muscle].as_ref().map(|s| s.count), Some(count - 3));
        // With units to spare it reads the count again at each turn and stops halfway: 16 gives 8.
        let mut many = whole.clone();
        eat_organs(&mut many, 1000, false, &mut Rng(3), |_| Used::Fed);
        assert_eq!(many.slots[muscle].as_ref().map(|s| s.count), Some(count / 2));
        // A body that needs nothing eats nothing, and a body that mends takes the same units.
        let mut still = whole.clone();
        eat_organs(&mut still, 5, false, &mut Rng(3), |_| Used::Spared);
        assert_eq!(still, whole);
        let mut healed = whole.clone();
        eat_organs(&mut healed, 2, false, &mut Rng(3), |_| Used::Healed);
        assert_eq!(healed.slots[muscle].as_ref().map(|s| s.count), Some(count - 2));

        // A tumor eats a stack up and leaves a tumor in its place; a stomach, one time in two.
        let mut small = whole.clone();
        small.slots[muscle] = Some(Slot::new("chestcavity:heart", 1));
        eat_organs(&mut small, 1, true, &mut Rng(3), |_| Used::Healed);
        assert!(at(&small, muscle).is_some_and(|id| id.ends_with("_tumor")), "{:?}", at(&small, muscle));
        let mut rng = Rng(11);
        let left_tumor = (0..400)
            .filter(|_| {
                let mut c = whole.clone();
                c.slots[muscle] = Some(Slot::new("chestcavity:heart", 1));
                eat_organs(&mut c, 1, false, &mut rng, |_| Used::Fed);
                c.slots[muscle].is_some()
            })
            .count();
        assert!((140..260).contains(&left_tumor), "about half leave a tumor: {left_tumor} of 400");
    }

    #[test]
    fn hunters_clear_the_harmful_tumors_and_the_infection_turns_food_into_sculk_tumors() {
        let mut cavity = Cavity::of(player_kind());
        cavity.slots[0] = Some(Slot::new("chestcavity:autophagy_tumor", 1));
        cavity.slots[1] = Some(Slot::new("chestcavity:benign_tumor", 1));
        cavity.slots[2] = Some(Slot::new("chestcavity:sculk_tumor", 1));
        let mut hunted = cavity.clone();
        hunt_tumors(&mut hunted, 1);
        assert_eq!((at(&hunted, 0), at(&hunted, 1), at(&hunted, 2)), (None, Some("chestcavity:benign_tumor"), Some("chestcavity:sculk_tumor")), "one at a time, the first harmful");
        hunt_tumors(&mut hunted, 5);
        assert_eq!((at(&hunted, 1), at(&hunted, 2)), (Some("chestcavity:benign_tumor"), None), "a benign tumor is left alone");

        let mut infected = cavity.clone();
        infect(&mut infected, 2);
        assert_eq!(at(&infected, 0), Some("chestcavity:sculk_tumor"));
        assert_eq!(at(&infected, 1), Some("chestcavity:sculk_tumor"));
        let rib = infected.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:rib")).expect("a rib");
        infect(&mut infected, 100);
        assert_eq!(at(&infected, rib), Some("chestcavity:rib"), "what is not food is immune");
        assert!(infected.slots.iter().flatten().all(|s| food_of(&s.id).is_none()), "all the food is infected");
    }
}
