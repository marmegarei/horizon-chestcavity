//! The in-game selftest's plan (`/cc selftest`): the bodies it gives a player one after another, and the attribute
//! bases the game should then report for each. The engine applies them, asks `Host::attribute` and compares; this
//! side has no game in it, so the cargo tests check the plan itself.
use crate::cavity::{Cavity, Slot, is_true_organ, player_kind};
use crate::effects::{attribute_bases, attribute_scores};
use crate::layout;
use crate::organs::ORGANS;

/// How far the game's value may be from the expected one and still pass.
pub const TOLERANCE: f64 = 0.01;

/// The range the game clamps each attribute the organs touch to (vanilla's `RangedAttribute`s, as the Horizon fdc87ae
/// exports them in `attributes.json`): a base outside it is reported clamped.
const RANGES: &[(&str, f64, f64)] = &[
    ("minecraft:max_health", 1.0, 1024.0),
    ("minecraft:attack_damage", 0.0, 2048.0),
    ("minecraft:movement_speed", 0.0, 1024.0),
    ("minecraft:attack_speed", 0.0, 1024.0),
    ("minecraft:block_break_speed", 0.0, 1024.0),
    ("minecraft:luck", -1024.0, 1024.0),
    ("minecraft:jump_strength", 0.0, 32.0),
    ("minecraft:water_movement_efficiency", 0.0, 1.0),
    ("minecraft:knockback_resistance", -2.0, 1.0),
    ("minecraft:block_interaction_range", 0.0, 64.0),
    ("minecraft:entity_interaction_range", 0.0, 64.0),
    ("minecraft:gravity", -1.0, 1.0),
];

pub fn clamp(attr: &str, v: f64) -> f64 {
    RANGES.iter().find(|(a, ..)| *a == attr).map_or(v, |&(_, lo, hi)| v.clamp(lo, hi))
}

pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE
}

/// One body the selftest gives the player: what it is called in the trace, and its cavity.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub name: String,
    pub cavity: Cavity,
}

/// The bodies, in order, for the player `body`: the normal one; the normal one without each of its kinds of organ (the
/// first slot of it); and the normal one with one more organ for each score that moves an attribute (and for
/// lightness), the first of the mod's organs that has it. Every organ is the player's own, so nothing is rejected.
pub fn cases(body: u128) -> Vec<Case> {
    let normal = Cavity::of(player_kind()).born(body, || false);
    let mut out = vec![Case { name: "normal".into(), cavity: normal.clone() }];
    let mut seen: Vec<&str> = Vec::new();
    for (i, slot) in normal.slots.iter().enumerate() {
        let Some(slot) = slot else { continue };
        if seen.contains(&slot.id.as_str()) {
            continue;
        }
        seen.push(&slot.id);
        let mut cavity = normal.clone();
        cavity.slots[i] = None;
        out.push(Case { name: format!("without {}", slot.id), cavity });
    }
    let mut scores = attribute_scores();
    scores.push("lightweight");
    for score in scores {
        let Some(organ) = ORGANS.iter().find(|o| is_true_organ(o.id) && o.scores.iter().any(|&(k, v)| k == score && v > 0.0)) else { continue };
        let Some(i) = layout::place(&normal, organ.id) else { continue };
        let mut cavity = normal.clone();
        cavity.slots[i] = Some(Slot::new(organ.id, organ.stack).owned_by(Some(body)));
        out.push(Case { name: format!("with {} ({score})", organ.id), cavity });
    }
    out
}

/// The base each attribute the organs touch should have with `cavity`, as the game reports it: the last value the
/// engine sets for it (a body without a spine gets its speed set twice), clamped to the attribute's range.
pub fn expected(cavity: &Cavity) -> Vec<(&'static str, f64)> {
    let kind = player_kind();
    let normal = Cavity::of(kind).scores(kind);
    let mut out: Vec<(&'static str, f64)> = Vec::new();
    for (attr, v) in attribute_bases(&cavity.scores(kind), &normal) {
        out.retain(|e| e.0 != attr);
        out.push((attr, clamp(attr, v)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: u128 = 7;

    fn base(case: &Case, attr: &str) -> f64 {
        expected(&case.cavity).into_iter().find(|e| e.0 == attr).map(|e| e.1).expect(attr)
    }

    #[test]
    fn the_normal_body_has_vanillas_bases() {
        let cases = cases(ME);
        assert_eq!(cases[0].name, "normal");
        assert!(close(base(&cases[0], "minecraft:max_health"), 20.0));
        assert!(close(base(&cases[0], "minecraft:movement_speed"), 0.1));
        assert!(close(base(&cases[0], "minecraft:gravity"), 0.08));
    }

    #[test]
    fn each_kind_of_organ_leaves_once_and_each_attribute_score_comes_in_once() {
        let cases = cases(ME);
        let without: Vec<_> = cases.iter().filter(|c| c.name.starts_with("without")).collect();
        let with: Vec<_> = cases.iter().filter(|c| c.name.starts_with("with ")).collect();
        let kinds: std::collections::BTreeSet<_> = player_kind().slots.iter().map(|s| s.1).collect();
        assert_eq!(without.len(), kinds.len());
        assert_eq!(with.len(), attribute_scores().len() + 1, "every score has an organ that adds to it");
        for case in &with {
            assert_eq!(case.cavity.incompatibility(ME), 0, "{}: all of it is the body's own", case.name);
            assert_eq!(case.cavity.slots.iter().flatten().count(), Cavity::of(player_kind()).slots.iter().flatten().count() + 1);
        }
    }

    #[test]
    fn missing_organs_move_their_attributes_and_the_clamp_holds() {
        let cases = cases(ME);
        let find = |name: &str| cases.iter().find(|c| c.name == name).expect(name);
        assert!(close(base(find("without chestcavity:heart"), "minecraft:max_health"), 16.0));
        // Without a spine the body cannot move: its speed is set to 0 after the muscles' share.
        assert_eq!(base(find("without chestcavity:spine"), "minecraft:movement_speed"), 0.0);
        assert_eq!(clamp("minecraft:knockback_resistance", 3.0), 1.0);
        assert_eq!(clamp("minecraft:max_health", -4.0), 1.0);
    }
}
