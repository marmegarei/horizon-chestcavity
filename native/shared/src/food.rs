//! What the organs do to a meal: Chest Cavity's `applyDigestion` (how much food a stomach gets out of it) and
//! `applyNutrition` (how much saturation the intestines get out of it), the kinds of food some organs digest
//! better, and how fast exercise makes you hungry (`endurance`). Plain numbers in, plain numbers out.
use crate::cavity::{Scores, get};

/// What a kind of food is, for the organs that digest some kinds better.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Meal {
    /// Meat and fish: what carnivorous organs digest; everything else is a plant's.
    pub meat: bool,
    /// Rotten flesh and the mod's rotten organs: what scavenging organs digest.
    pub rotten: bool,
}

/// Vanilla's meats and fish (`isMeat` and `#chestcavity:carnivore_food`).
const MEAT: &[&str] = &[
    "minecraft:beef", "minecraft:cooked_beef", "minecraft:porkchop", "minecraft:cooked_porkchop", "minecraft:mutton",
    "minecraft:cooked_mutton", "minecraft:chicken", "minecraft:cooked_chicken", "minecraft:rabbit", "minecraft:cooked_rabbit",
    "minecraft:rotten_flesh", "minecraft:cod", "minecraft:cooked_cod", "minecraft:salmon", "minecraft:cooked_salmon",
    "minecraft:pufferfish", "minecraft:tropical_fish",
];

impl Meal {
    /// The kind of food `item` (`minecraft:cooked_beef`) is. The mod's own foods are meat and sausages, and the
    /// rotten and toxic ones are the rotten kind.
    pub fn of(item: &str) -> Meal {
        let own = item.strip_prefix("chestcavity:");
        let meat = MEAT.contains(&item) || own.is_some_and(|n| (n.contains("meat") || n.contains("sausage")) && !n.contains("skin"));
        let rotten = item == "minecraft:rotten_flesh" || own.is_some_and(|n| n.starts_with("rotten") || n.contains("toxic"));
        Meal { meat, rotten }
    }
}

/// The digestion and nutrition a body brings to `meal`: its stomach and intestines, and the organs for the kind of food.
pub fn scores(now: &Scores, meal: Meal) -> (f32, f32) {
    let (mut digestion, mut nutrition) = (get(now, "digestion"), get(now, "nutrition"));
    let (kind_digestion, kind_nutrition) = if meal.meat { ("carnivorous_digestion", "carnivorous_nutrition") } else { ("herbivorous_digestion", "herbivorous_nutrition") };
    digestion += get(now, kind_digestion);
    nutrition += get(now, kind_nutrition);
    if meal.rotten {
        digestion += get(now, "rot_digestion") + get(now, "scavenger_digestion");
        nutrition += get(now, "rotgut") + get(now, "scavenger_nutrition");
    }
    (digestion, nutrition)
}

/// What `hunger` points of food become in a stomach that digests at `digestion` (1 is normal): the points, and the
/// ticks of nausea. A stomach that can't digest (below 0) gets one point and makes you sick; one that digests
/// little still gets one.
pub fn digest(digestion: f32, hunger: i32) -> (i32, i32) {
    if hunger <= 0 || (digestion - 1.0).abs() < 1e-6 {
        return (hunger, 0);
    }
    if digestion < 0.0 {
        return (1, (hunger as f32 * -digestion * 400.0) as i32);
    }
    (((hunger as f32 * digestion) as i32).max(1), 0)
}

/// What `saturation` becomes with intestines that nourish at `nutrition` (4 is normal): the saturation, and the
/// ticks of hunger. Intestines that can't nourish (below 0) give none and make you hungry.
pub fn nourish(nutrition: f32, saturation: f32) -> (f32, i32) {
    if saturation <= 0.0 || (nutrition - 4.0).abs() < 1e-6 {
        return (saturation, 0);
    }
    if nutrition < 0.0 {
        return (0.0, (saturation * -nutrition * 800.0) as i32);
    }
    (saturation * nutrition / 4.0, 0)
}

/// What a body's meal gave: food points, saturation, and the ticks of nausea and of hunger it caused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eaten {
    pub food: i32,
    pub saturation: f32,
    pub nausea: i32,
    pub hunger: i32,
}

/// The meal `meal` that gave `food` points and `saturation` in vanilla, as `now` digests it.
pub fn eat(now: &Scores, meal: Meal, food: i32, saturation: f32) -> Eaten {
    let (digestion, nutrition) = scores(now, meal);
    let (food, nausea) = digest(digestion, food);
    let (saturation, hunger) = nourish(nutrition, saturation);
    Eaten { food, saturation, nausea, hunger }
}

/// How much of the exhaustion exercise adds (`endurance` above normal tires you less, below it more).
pub fn endurance_factor(now: &Scores, normal: &Scores) -> f32 {
    let d = get(now, "endurance") - get(normal, "endurance");
    if d > 0.0 { 1.0 / (1.0 + d / 2.0) } else { 1.0 - d / 2.0 }
}

/// How many times faster than normal the body heals from a full stomach (`metabolism`): 1 is normal.
pub fn metabolism_rate(now: &Scores, normal: &Scores) -> f32 {
    let d = get(now, "metabolism") - get(normal, "metabolism");
    if d > 0.0 { 1.0 + d } else { 1.0 / (1.0 - d) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Cavity, player_kind};

    #[test]
    fn a_normal_body_changes_nothing_and_the_others_change_the_meal() {
        assert_eq!(digest(1.0, 6), (6, 0));
        assert_eq!(digest(0.5, 6), (3, 0));
        assert_eq!(digest(0.05, 6), (1, 0), "a bad stomach still gets a point");
        assert_eq!(digest(-0.5, 6), (1, 1200), "and one that cannot digest makes you sick");
        assert_eq!(nourish(4.0, 5.0), (5.0, 0));
        assert_eq!(nourish(2.0, 5.0), (2.5, 0));
        assert_eq!(nourish(-1.0, 5.0), (0.0, 4000));
        assert_eq!(digest(2.0, 0), (0, 0));
    }

    #[test]
    fn foods_have_a_kind() {
        assert_eq!(Meal::of("minecraft:cooked_beef"), Meal { meat: true, rotten: false });
        assert_eq!(Meal::of("minecraft:rotten_flesh"), Meal { meat: true, rotten: true });
        assert_eq!(Meal::of("minecraft:bread"), Meal { meat: false, rotten: false });
        assert!(Meal::of("chestcavity:cooked_organ_meat").meat);
        assert!(Meal::of("chestcavity:toxic_sausage").rotten);
        assert!(!Meal::of("chestcavity:sausage_skin").meat);
    }

    #[test]
    fn organs_for_a_kind_of_food_only_count_for_that_food() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        assert_eq!(scores(&normal, Meal::of("minecraft:bread")), (1.0, 4.0));
        let mut herbivore = normal.clone();
        herbivore.insert("herbivorous_digestion", 1.0);
        assert_eq!(scores(&herbivore, Meal::of("minecraft:bread")).0, 2.0);
        assert_eq!(scores(&herbivore, Meal::of("minecraft:beef")).0, 1.0);
        let eaten = eat(&herbivore, Meal::of("minecraft:bread"), 5, 6.0);
        assert_eq!(eaten.food, 10, "twice the digestion, twice the food");
    }

    #[test]
    fn endurance_and_metabolism_are_ratios_around_normal() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        assert_eq!(endurance_factor(&normal, &normal), 1.0);
        assert_eq!(metabolism_rate(&normal, &normal), 1.0);
        let mut tough = normal.clone();
        *tough.entry("endurance").or_default() += 2.0;
        assert_eq!(endurance_factor(&tough, &normal), 0.5);
        let mut fast = normal.clone();
        *fast.entry("metabolism").or_default() += 1.0;
        assert_eq!(metabolism_rate(&fast, &normal), 2.0);
        let mut slow = normal.clone();
        *slow.entry("metabolism").or_default() -= 1.0;
        assert_eq!(metabolism_rate(&slow, &normal), 0.5);
    }
}
