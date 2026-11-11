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
    ("leaping", "minecraft:jump_strength", 0.42, true, 0.25), // LEAPING_POWER
    ("swim_speed", "minecraft:water_movement_efficiency", 0.0, false, 1.0 / 8.0), // SWIMSPEED_FACTOR
    ("knockback_resistant", "minecraft:knockback_resistance", 0.0, false, 0.1), // KNOCKBACK_RESISTANCE per point
    ("knockback_resistance", "minecraft:knockback_resistance", 0.0, false, 1.0), // Beyond: added as it is
    ("block_interaction_range", "minecraft:block_interaction_range", 4.5, false, 1.0),
    ("entity_interaction_range", "minecraft:entity_interaction_range", 3.0, false, 1.0),
];

/// How far one point of `score` moves its attribute (the table above), for describing an organ.
pub fn rate(score: &str) -> Option<f64> {
    ATTRIBUTES.iter().find(|a| a.0 == score).map(|a| a.4)
}

/// `LIGHTWIEGHT_FACTOR`: how much one point of lightness takes off gravity (and one of heaviness adds).
const LIGHTWEIGHT: f64 = 0.25;

/// The base value of every attribute the organs touch for `now` against the
/// body's `normal` scores, in the order to set them. Scores that move the same attribute add up.
/// A normal body gets vanilla's bases back.
pub fn attribute_bases(now: &Scores, normal: &Scores) -> Vec<(&'static str, f64)> {
    // (attribute, vanilla base, multiplies the base, what the scores add up to)
    let mut sums: Vec<(&'static str, f64, bool, f64)> = Vec::new();
    for &(score, attr, vanilla, multiplies, per) in ATTRIBUTES {
        let d = f64::from(get(now, score) - get(normal, score)) * per;
        match sums.iter_mut().find(|e| e.0 == attr) {
            Some(e) => e.3 += d,
            None => sums.push((attr, vanilla, multiplies, d)),
        }
    }
    let mut out: Vec<_> = sums.into_iter().map(|(attr, vanilla, multiplies, d)| (attr, if multiplies { vanilla * (1.0 + d) } else { vanilla + d })).collect();
    // Light organs: less gravity; heavy ones: more (`applyLightweightToGravity`).
    let light = f64::from(get(now, "lightweight") - get(normal, "lightweight"));
    out.push(("minecraft:gravity", if light > 0.0 { 0.08 / (1.0 + light * LIGHTWEIGHT) } else { 0.08 * (1.0 - light * LIGHTWEIGHT) }));
    // No spine: no moving at all.
    if get(now, "nerves") <= 0.0 {
        out.push(("minecraft:movement_speed", 0.0));
    }
    out
}

/// What a mob's organs add to its own attributes, whatever their vanilla bases: (attribute, amount, operation) of a
/// command's `attribute <mob> <attribute> modifier add`. Without a spine it can't move at all.
pub fn modifiers(now: &Scores, normal: &Scores) -> Vec<(&'static str, f64, &'static str)> {
    const MOBS: &[&str] = &["minecraft:max_health", "minecraft:attack_damage", "minecraft:movement_speed", "minecraft:knockback_resistance"];
    let mut sums: Vec<(&'static str, bool, f64)> = Vec::new();
    for &(score, attr, _, multiplies, per) in ATTRIBUTES.iter().filter(|a| MOBS.contains(&a.1)) {
        let d = f64::from(get(now, score) - get(normal, score)) * per;
        match sums.iter_mut().find(|e| e.0 == attr) {
            Some(e) => e.2 += d,
            None => sums.push((attr, multiplies, d)),
        }
    }
    let paralysed = get(normal, "nerves") > 0.0 && get(now, "nerves") <= 0.0;
    let mut out: Vec<_> = sums
        .into_iter()
        .filter(|&(attr, _, d)| d != 0.0 && !(paralysed && attr == "minecraft:movement_speed"))
        .map(|(attr, multiplies, d)| (attr, d, if multiplies { "add_multiplied_base" } else { "add_value" }))
        .collect();
    if paralysed {
        out.push(("minecraft:movement_speed", -1.0, "add_multiplied_total"));
    }
    out
}

/// Something an organ does to the body on the second's beat.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `kind` is a damage type id (`minecraft:generic`, `minecraft:magic`).
    Damage { amount: f32, kind: &'static str },
    Effect { id: &'static str, ticks: u32, amplifier: u32 },
    /// Food if the body isn't full, else saturation, else health.
    Nourish { hunger: i32, saturation: f32, heal: f32 },
    /// Somewhere else at random, at most `range` blocks away.
    Teleport { range: f32 },
}

/// Where the body is, as far as its organs care.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Env {
    /// The light at its head, 0 to 15 (the brighter of the block's and the sky's).
    pub light: u8,
    /// Its body is in water, or rain falls on it.
    pub wet: bool,
    /// Its head is under water.
    pub submerged: bool,
    /// An End Crystal is within reach of the body.
    pub crystal: bool,
}

/// What the organs keep count of between beats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    /// Seconds the heart has been missing.
    pub bleed: u32,
    /// How much light the leaves have taken in since the last meal of it.
    pub light: i32,
    /// Ticks until water hurts again.
    pub dry: i32,
}

/// `PHOTOSYNTHESIS_FREQUENCY` (50) ticks of 8 leaves in full light win one step.
const PHOTOSYNTHESIS_STEP: i32 = 50 * 8 * 15;
/// How long water spares a body after it hurt (`WATER_VULNERABILITY`, 260 ticks at 1).
const WATER_SPARE: f32 = 260.0;

/// What the organs do over time, called every second with the seconds so far. (Lungs, stomach and intestines work
/// when breathing and eating, not here.)
pub fn periodic(now: &Scores, normal: &Scores, seconds: u64, progress: &mut Progress, env: &Env) -> Vec<Action> {
    let mut out = Vec::new();
    // Heartbleed (HEARTBLEED_RATE 20 ticks): damage that grows each second, for a body that needs a heart.
    if get(now, "health") <= 0.0 && get(normal, "health") > 0.0 {
        progress.bleed += 1;
        out.push(Action::Damage { amount: progress.bleed as f32, kind: "minecraft:generic" });
    } else {
        progress.bleed = 0;
    }
    // Kidneys (KIDNEY_RATE 60 ticks): filtering less than normal lets poison in, for longer the less it filters.
    let wanted = get(normal, "filtration");
    if seconds % 3 == 0 && wanted > 0.0 {
        let ratio = get(now, "filtration") / wanted;
        if ratio < 1.0 {
            out.push(Action::Effect { id: "minecraft:poison", ticks: ((48.0 * (1.0 - ratio)) as u32).max(1), amplifier: 0 });
        }
    }
    // Glowing organs (TickGlowing): the body shines through walls.
    if get(now, "glowing") > 0.0 {
        out.push(Action::Effect { id: "minecraft:glowing", ticks: 200, amplifier: 0 });
    }
    // Leaves (TickPhotosynthesis): light wins food, then saturation, then health.
    let leaves = get(now, "photosynthesis") - get(normal, "photosynthesis");
    if leaves > 0.0 {
        progress.light += (leaves * f32::from(env.light)) as i32 * 20;
        if progress.light > PHOTOSYNTHESIS_STEP {
            progress.light = 0;
            out.push(Action::Nourish { hunger: 1, saturation: 1.0, heal: 1.0 });
        }
    }
    // Crystals (TickCrystalsynthesis): an End Crystal in reach feeds the body twice a second, then heals it.
    let crystal = get(now, "crystalsynthesis");
    if crystal > 0.0 && env.crystal {
        out.push(Action::Nourish { hunger: if crystal >= 5.0 { 2 } else { 1 }, saturation: crystal / 5.0, heal: crystal * 2.0 / 5.0 });
    }
    // Water allergy (TickHydroallergenic): a splash hurts a little, a dive a lot, then the body is spared a while.
    let allergy = get(now, "hydroallergenic");
    progress.dry = (progress.dry - 20).max(0);
    if allergy > 0.0 && progress.dry == 0 && (env.submerged || env.wet) {
        out.push(Action::Damage { amount: if env.submerged { 10.0 } else { 1.0 }, kind: "minecraft:magic" });
        progress.dry = (WATER_SPARE / allergy) as i32;
    }
    // Water fear (TickHydrophobia): bodies that don't fear it by nature are thrown across the land.
    let fear = get(now, "hydrophobia");
    if fear > 0.0 && get(normal, "hydrophobia") == 0.0 && env.wet {
        out.push(Action::Teleport { range: fear * 32.0 });
    }
    out
}

/// The scores the summary shows.
pub const SHOWN: &[&str] = &["health", "strength", "speed", "defense", "nerves", "filtration", "breath_recovery", "digestion", "nutrition", "luck"];

/// Each shown score, with its value and how far it is from normal.
pub fn summary(now: &Scores, normal: &Scores) -> Vec<(&'static str, f32, f32)> {
    SHOWN.iter().map(|&k| (k, get(now, k), get(now, k) - get(normal, k))).collect()
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

    fn no_spine_scores(normal: &Scores) -> Scores {
        let mut s = normal.clone();
        s.insert("nerves", 0.0);
        s.insert("speed", get(normal, "speed") + 1.0);
        s
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
        assert!(periodic(&normal, &normal, 3, &mut Progress::default(), &Env::default()).is_empty());

        // No heart: 4 less max health per heart, and heartbleed that grows.
        let no_heart = without("chestcavity:heart").scores(kind);
        assert_eq!(base(&attribute_bases(&no_heart, &normal), "minecraft:max_health"), 16.0);
        let mut progress = Progress::default();
        periodic(&no_heart, &normal, 1, &mut progress, &Env::default());
        let actions = periodic(&no_heart, &normal, 2, &mut progress, &Env::default());
        assert!(actions.contains(&Action::Damage { amount: 2.0, kind: "minecraft:generic" }), "{actions:?}");

        // Lighter organs fall slower, and two scores that move one attribute add up.
        let mut light = normal.clone();
        light.insert("lightweight", 1.0);
        light.insert("knockback_resistant", 1.0);
        light.insert("knockback_resistance", 0.5);
        let bases = attribute_bases(&light, &normal);
        assert!((base(&bases, "minecraft:gravity") - 0.064).abs() < 1e-9);
        assert!((base(&bases, "minecraft:knockback_resistance") - 0.6).abs() < 1e-9);
        assert_eq!(base(&attribute_bases(&normal, &normal), "minecraft:gravity"), 0.08);

        // A mob's attributes come as modifiers: a heart short is 4 health short, no spine is no moving.
        let mob = modifiers(&no_heart, &normal);
        assert!(mob.contains(&("minecraft:max_health", -4.0, "add_value")), "{mob:?}");
        assert!(modifiers(&normal, &normal).is_empty());
        let still = modifiers(&no_spine_scores(&normal), &normal);
        assert_eq!(still.iter().filter(|m| m.0 == "minecraft:movement_speed").count(), 1, "{still:?}");
        assert!(still.contains(&("minecraft:movement_speed", -1.0, "add_multiplied_total")));

        // No spine: no nerves, no moving.
        let no_spine = without("chestcavity:spine").scores(kind);
        assert_eq!(base(&attribute_bases(&no_spine, &normal), "minecraft:movement_speed"), 0.0);

        // Leaves in the sun feed, an allergic body hurts in water and then is spared, a crystal feeds.
        let mut leafy = normal.clone();
        leafy.insert("photosynthesis", 8.0);
        let sun = Env { light: 15, ..Env::default() };
        let mut progress = Progress::default();
        let steps = (0..4).filter(|&t| periodic(&leafy, &normal, t, &mut progress, &sun).iter().any(|a| matches!(a, Action::Nourish { .. }))).count();
        assert_eq!(steps, 1, "8 leaves in full light take 2.5 seconds for a step");
        assert!(periodic(&leafy, &normal, 1, &mut Progress::default(), &Env::default()).is_empty(), "no light, no step");

        let mut allergic = normal.clone();
        allergic.insert("hydroallergenic", 1.0);
        let dive = Env { wet: true, submerged: true, ..Env::default() };
        let mut progress = Progress::default();
        let hit = |p: &mut Progress, env: &Env| periodic(&allergic, &normal, 1, p, env).into_iter().find_map(|a| match a { Action::Damage { amount, kind: "minecraft:magic" } => Some(amount), _ => None });
        assert_eq!(hit(&mut progress, &dive), Some(10.0));
        assert_eq!(hit(&mut progress, &dive), None, "spared for a while after");
        let mut progress = Progress::default();
        assert_eq!(hit(&mut progress, &Env { wet: true, ..Env::default() }), Some(1.0), "rain only stings");
        assert_eq!(hit(&mut Progress::default(), &Env::default()), None, "dry land is safe");

        let mut crystalline = normal.clone();
        crystalline.insert("crystalsynthesis", 2.0);
        assert!(periodic(&crystalline, &normal, 1, &mut Progress::default(), &Env { crystal: true, ..Env::default() }).iter().any(|a| matches!(a, Action::Nourish { .. })));
        assert!(periodic(&crystalline, &normal, 1, &mut Progress::default(), &Env::default()).is_empty());

        let mut afraid = normal.clone();
        afraid.insert("hydrophobia", 1.0);
        assert!(periodic(&afraid, &normal, 1, &mut Progress::default(), &Env { wet: true, ..Env::default() }).contains(&Action::Teleport { range: 32.0 }));

        // Storage round trip.
        let c = without("chestcavity:liver");
        assert_eq!(Cavity::from_text(&c.to_text()), c);
    }
}
