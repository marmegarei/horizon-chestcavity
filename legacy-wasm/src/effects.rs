//! What organ scores do to the player, as vanilla commands. Chest Cavity
//! changes attributes and ticks its own effects; Horizon gives mods neither,
//! so the same numbers (its `CCConfig` defaults) go through `/attribute`,
//! `/effect` and `/damage`, run as the player.
use crate::cavity::{Scores, get};

/// (score, attribute, operation, per point of difference from normal).
/// A difference of `d` adds the modifier `chestcavity:<score>` worth `d * per`.
const ATTRIBUTES: &[(&str, &str, &str, f32)] = &[
    ("health", "minecraft:max_health", "add_value", 4.0), // HEART_HP
    ("strength", "minecraft:attack_damage", "add_multiplied_base", 1.0 / 8.0), // MUSCLE_STRENGTH per 8 stacks
    ("speed", "minecraft:movement_speed", "add_multiplied_base", 0.5 / 8.0), // MUSCLE_SPEED per 8 stacks
    ("nerves", "minecraft:attack_speed", "add_multiplied_base", 0.1), // NERVES_HASTE
    ("nerves", "minecraft:block_break_speed", "add_multiplied_base", 0.1),
    ("luck", "minecraft:luck", "add_value", 0.1), // APPENDIX_LUCK
    // ponytail: Chest Cavity cuts damage by 0.5^(d/4); armor is the nearest attribute.
    ("defense", "minecraft:armor", "add_value", 2.0),
    ("leaping", "minecraft:jump_strength", "add_multiplied_base", 0.25), // LEAPING_POWER
    ("swim_speed", "minecraft:water_movement_efficiency", "add_value", 1.0 / 8.0), // SWIMSPEED_FACTOR
    ("knockback_resistant", "minecraft:knockback_resistance", "add_value", 0.25),
    ("breath_capacity", "minecraft:oxygen_bonus", "add_value", 1.0),
];

/// The attribute modifiers for `now` against the body's `normal` scores:
/// each one removed, then added again when it isn't zero.
pub fn attributes(now: &Scores, normal: &Scores) -> Vec<String> {
    let mut lines = Vec::new();
    for &(score, attr, op, per) in ATTRIBUTES {
        let id = format!("chestcavity:{score}");
        lines.push(format!("attribute @s {attr} modifier remove {id}"));
        let value = (get(now, score) - get(normal, score)) * per;
        if value.abs() > 1e-4 {
            lines.push(format!("attribute @s {attr} modifier add {id} {value:.4} {op}"));
        }
    }
    // No spine: no moving at all.
    lines.push("attribute @s minecraft:movement_speed modifier remove chestcavity:paralysis".into());
    if get(now, "nerves") <= 0.0 {
        lines.push("attribute @s minecraft:movement_speed modifier add chestcavity:paralysis -1 add_multiplied_total".into());
    }
    lines
}

/// What the organs do over time, called every second (20 ticks) with the
/// seconds so far; `bleed` is how long the heart has been missing.
pub fn periodic(now: &Scores, normal: &Scores, seconds: u64, bleed: &mut u32) -> Vec<String> {
    let mut lines = Vec::new();
    // Heartbleed (HEARTBLEED_RATE 20 ticks): damage that grows each second.
    if get(now, "health") <= 0.0 {
        *bleed += 1;
        lines.push(format!("damage @s {} minecraft:generic", *bleed));
    } else {
        *bleed = 0;
    }
    // Kidneys (KIDNEY_RATE 60 ticks): less filtration than normal lets poison in.
    let filtration = get(now, "filtration");
    if seconds % 3 == 0 && filtration < get(normal, "filtration") {
        let level = if filtration <= 0.0 { 1 } else { 0 };
        lines.push(format!("effect give @s minecraft:poison 4 {level} true"));
    }
    // No lungs: air doesn't come back, so out of water the body drowns.
    if get(now, "breath_recovery") <= 0.0 && get(now, "water_breath") <= 0.0 {
        lines.push("execute anchored eyes positioned ^ ^ ^ unless block ~ ~ ~ minecraft:water run damage @s 2 minecraft:drown".into());
    }
    // ponytail: Chest Cavity applies these when eating; here they tick.
    if seconds % 3 == 0 && get(now, "digestion") < 0.0 {
        lines.push("effect give @s minecraft:nausea 6 0 true".into());
    }
    if seconds % 3 == 0 && get(now, "nutrition") < 0.0 {
        lines.push("effect give @s minecraft:hunger 4 0 true".into());
    }
    for (score, effect) in [
        ("water_breath", "water_breathing"),
        ("fire_resistant", "fire_resistance"),
        ("glowing", "glowing"),
    ] {
        if get(now, score) > get(normal, score) {
            lines.push(format!("effect give @s minecraft:{effect} 3 0 true"));
        }
    }
    lines
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

    #[test]
    fn organs_change_the_body() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        // A normal body: modifiers removed, none added, nothing ticking.
        let lines = attributes(&normal, &normal);
        assert!(lines.iter().all(|l| l.contains(" remove ")), "{lines:?}");
        assert!(periodic(&normal, &normal, 3, &mut 0).is_empty());

        // No heart: 4 less max health per heart, and heartbleed that grows.
        let no_heart = without("chestcavity:heart").scores(kind);
        let lines = attributes(&no_heart, &normal);
        assert!(lines.contains(&"attribute @s minecraft:max_health modifier add chestcavity:health -4.0000 add_value".to_string()), "{lines:?}");
        let mut bleed = 0;
        periodic(&no_heart, &normal, 1, &mut bleed);
        let lines = periodic(&no_heart, &normal, 2, &mut bleed);
        assert!(lines.contains(&"damage @s 2 minecraft:generic".to_string()), "{lines:?}");

        // No spine: no nerves, no moving.
        let no_spine = without("chestcavity:spine").scores(kind);
        assert!(attributes(&no_spine, &normal).iter().any(|l| l.contains("chestcavity:paralysis -1")));

        // Storage round trip.
        let c = without("chestcavity:liver");
        assert_eq!(Cavity::from_text(&c.to_text()), c);
    }
}
