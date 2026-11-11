//! What organ scores do to a body, as plain data the engine applies with
//! Horizon's player helpers (no commands, so no cheats needed). Numbers are
//! Chest Cavity's `CCConfig` defaults.
use crate::cavity::{Scores, get};

/// (score, attribute, vanilla base, multiplies the base, per point of difference from normal).
/// A difference of `d` moves the attribute's base by `d * per`: added, or as a share of the vanilla base.
const ATTRIBUTES: &[(&str, &str, f64, bool, f64)] = &[
    ("health", "minecraft:max_health", 20.0, false, 4.0), // HEART_HP
    ("strength", "minecraft:attack_damage", 1.0, true, 1.0 / 8.0), // MUSCLE_STRENGTH per 8 stacks
    ("speed", "minecraft:movement_speed", 0.1, true, 0.5 / 8.0), // MUSCLE_SPEED per 8 stacks
    ("nerves", "minecraft:attack_speed", 4.0, true, 0.1), // NERVES_HASTE
    ("nerves", "minecraft:block_break_speed", 1.0, true, 0.1),
    ("luck", "minecraft:luck", 0.0, false, 0.1), // APPENDIX_LUCK
    // ponytail: Chest Cavity cuts damage by 0.5^(d/4); armor is the nearest attribute.
    ("defense", "minecraft:armor", 0.0, false, 2.0),
    ("leaping", "minecraft:jump_strength", 0.42, true, 0.25), // LEAPING_POWER
    ("swim_speed", "minecraft:water_movement_efficiency", 0.0, false, 1.0 / 8.0), // SWIMSPEED_FACTOR
    ("knockback_resistant", "minecraft:knockback_resistance", 0.0, false, 0.25),
    ("breath_capacity", "minecraft:oxygen_bonus", 0.0, false, 1.0),
];

/// The base value of every attribute the organs touch for `now` against the
/// body's `normal` scores, in the order to set them (a later entry for the
/// same attribute wins). A normal body gets vanilla's bases back.
pub fn attribute_bases(now: &Scores, normal: &Scores) -> Vec<(&'static str, f64)> {
    let mut out: Vec<(&'static str, f64)> = ATTRIBUTES
        .iter()
        .map(|&(score, attr, vanilla, multiplies, per)| {
            let d = f64::from(get(now, score) - get(normal, score)) * per;
            (attr, if multiplies { vanilla * (1.0 + d) } else { vanilla + d })
        })
        .collect();
    // No spine: no moving at all.
    if get(now, "nerves") <= 0.0 {
        out.push(("minecraft:movement_speed", 0.0));
    }
    out
}

/// Something an organ does to the body on the second's beat.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `kind` is a damage type id (`minecraft:generic`, `minecraft:drown`).
    Damage { amount: f32, kind: &'static str },
    Effect { id: &'static str, seconds: u32, amplifier: u32 },
}

/// What the organs do over time, called every second with the seconds so
/// far; `bleed` is how long the heart has been missing and `in_water` says
/// whether the head is in water.
pub fn periodic(now: &Scores, normal: &Scores, seconds: u64, bleed: &mut u32, in_water: bool) -> Vec<Action> {
    let mut out = Vec::new();
    // Heartbleed (HEARTBLEED_RATE 20 ticks): damage that grows each second.
    if get(now, "health") <= 0.0 {
        *bleed += 1;
        out.push(Action::Damage { amount: *bleed as f32, kind: "minecraft:generic" });
    } else {
        *bleed = 0;
    }
    // Kidneys (KIDNEY_RATE 60 ticks): less filtration than normal lets poison in.
    let filtration = get(now, "filtration");
    if seconds % 3 == 0 && filtration < get(normal, "filtration") {
        let amplifier = u32::from(filtration <= 0.0);
        out.push(Action::Effect { id: "minecraft:poison", seconds: 4, amplifier });
    }
    // No lungs: air doesn't come back, so out of water the body drowns.
    if get(now, "breath_recovery") <= 0.0 && get(now, "water_breath") <= 0.0 && !in_water {
        out.push(Action::Damage { amount: 2.0, kind: "minecraft:drown" });
    }
    // ponytail: Chest Cavity applies these when eating; here they tick.
    if seconds % 3 == 0 && get(now, "digestion") < 0.0 {
        out.push(Action::Effect { id: "minecraft:nausea", seconds: 6, amplifier: 0 });
    }
    if seconds % 3 == 0 && get(now, "nutrition") < 0.0 {
        out.push(Action::Effect { id: "minecraft:hunger", seconds: 4, amplifier: 0 });
    }
    for (score, id) in [
        ("water_breath", "minecraft:water_breathing"),
        ("fire_resistant", "minecraft:fire_resistance"),
        ("glowing", "minecraft:glowing"),
    ] {
        if get(now, score) > get(normal, score) {
            out.push(Action::Effect { id, seconds: 3, amplifier: 0 });
        }
    }
    out
}

/// The scores the summary shows, with their names.
const SHOWN: &[(&str, &str)] = &[
    ("health", "Health"), ("strength", "Strength"), ("speed", "Speed"), ("defense", "Defense"),
    ("nerves", "Nerves"), ("filtration", "Filtration"), ("breath_recovery", "Breath"),
    ("digestion", "Digestion"), ("nutrition", "Nutrition"), ("luck", "Luck"),
];

/// "Health 1.0 (-1.0)": each shown score, with how far it is from normal.
pub fn summary(now: &Scores, normal: &Scores) -> String {
    SHOWN
        .iter()
        .map(|&(k, name)| {
            let (v, d) = (get(now, k), get(now, k) - get(normal, k));
            if d.abs() < 0.01 { format!("{name} {v:.1}") } else { format!("{name} {v:.1} ({d:+.1})") }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Cavity, player_kind};

    fn without(item: &str) -> Cavity {
        let mut c = Cavity::of(player_kind());
        for s in c.slots.iter_mut() {
            if s.as_ref().is_some_and(|s| s.id == item) {
                *s = None;
            }
        }
        c
    }

    fn base(bases: &[(&'static str, f64)], attr: &str) -> f64 {
        bases.iter().rev().find(|(a, _)| *a == attr).map(|&(_, b)| b).unwrap()
    }

    #[test]
    fn organs_change_the_body() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        // A normal body: vanilla bases, nothing ticking.
        let bases = attribute_bases(&normal, &normal);
        assert_eq!(base(&bases, "minecraft:max_health"), 20.0);
        assert_eq!(base(&bases, "minecraft:movement_speed"), 0.1);
        assert!(periodic(&normal, &normal, 3, &mut 0, false).is_empty());

        // No heart: 4 less max health per heart, and heartbleed that grows.
        let no_heart = without("chestcavity:heart").scores(kind);
        assert_eq!(base(&attribute_bases(&no_heart, &normal), "minecraft:max_health"), 16.0);
        let mut bleed = 0;
        periodic(&no_heart, &normal, 1, &mut bleed, false);
        let actions = periodic(&no_heart, &normal, 2, &mut bleed, false);
        assert!(actions.contains(&Action::Damage { amount: 2.0, kind: "minecraft:generic" }), "{actions:?}");

        // No spine: no nerves, no moving.
        let no_spine = without("chestcavity:spine").scores(kind);
        assert_eq!(base(&attribute_bases(&no_spine, &normal), "minecraft:movement_speed"), 0.0);

        // No lungs drown out of water only.
        let no_lungs = without("chestcavity:lung").scores(kind);
        assert!(periodic(&no_lungs, &normal, 1, &mut 0, false).iter().any(|a| matches!(a, Action::Damage { kind: "minecraft:drown", .. })));
        assert!(periodic(&no_lungs, &normal, 1, &mut 0, true).is_empty());

        // Storage round trip.
        let c = without("chestcavity:liver");
        assert_eq!(Cavity::from_text(&c.to_text()), c);
    }
}
