//! What organs do to the potion effects a body gets (`OrganAddStatusEffectListeners`): a liver filters the harmful ones
//! (`detoxification`), purging organs cut the helpful ones (`buff_purging`), withered bones the Wither (`withered`).
use crate::cavity::{Scores, get};

/// `BUFF_PURGING_DURATION_FACTOR` and `WITHERED_DURATION_FACTOR`.
const BUFF_PURGING: f32 = 0.5;
const WITHERED: f32 = 0.5;
/// How long a body lets a harmful effect last when it has no filtration at all.
const ENDLESS: i32 = 9999;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Harmful,
    Beneficial,
    Neutral,
}

/// `StatusEffectCategory` of a vanilla effect (`minecraft:poison`); anything else counts as neutral.
pub fn category(id: &str) -> Category {
    match id.strip_prefix("minecraft:").unwrap_or(id) {
        "slowness" | "mining_fatigue" | "instant_damage" | "nausea" | "blindness" | "hunger" | "weakness" | "poison" | "wither" | "unluck" | "darkness"
        | "wind_charged" | "weaving" | "oozing" | "infested" | "raid_omen" | "trial_omen" => Category::Harmful,
        "speed" | "haste" | "strength" | "instant_health" | "jump_boost" | "regeneration" | "resistance" | "fire_resistance" | "water_breathing"
        | "invisibility" | "night_vision" | "health_boost" | "absorption" | "saturation" | "luck" | "slow_falling" | "conduit_power" | "dolphins_grace"
        | "hero_of_the_village" => Category::Beneficial,
        _ => Category::Neutral,
    }
}

/// Whether any of this is worth looking at the body's effects for.
pub fn matters(now: &Scores, normal: &Scores) -> bool {
    let detox = get(normal, "detoxification") > 0.0 && get(now, "detoxification") != get(normal, "detoxification");
    detox || get(now, "buff_purging") > 0.0 || get(now, "withered") > 0.0
}

/// The duration, in ticks, a freshly applied effect `id` of `duration` ticks gets in a body with `now` scores (`normal`
/// being what the body is born with). Instant and endless effects (duration 0 or less) are left as they are.
pub fn adjust(id: &str, duration: i32, now: &Scores, normal: &Scores) -> i32 {
    if duration <= 0 {
        return duration;
    }
    let mut d = duration as f32;
    match category(id) {
        Category::Beneficial if get(now, "buff_purging") > 0.0 => d /= 1.0 + BUFF_PURGING * get(now, "buff_purging"),
        Category::Harmful => {
            let wanted = get(normal, "detoxification");
            if wanted > 0.0 && get(now, "detoxification") != wanted {
                let ratio = get(now, "detoxification") / wanted;
                d = if ratio > -1.0 { (d * 2.0 / (1.0 + ratio)).max(1.0) } else { ENDLESS as f32 };
            }
        }
        _ => {}
    }
    if id.ends_with("wither") && get(now, "withered") > 0.0 {
        d /= 1.0 + WITHERED * get(now, "withered");
    }
    d as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Cavity, player_kind};

    fn scores(f: impl Fn(&mut Scores)) -> (Scores, Scores) {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        let mut now = normal.clone();
        f(&mut now);
        (now, normal)
    }

    #[test]
    fn a_normal_body_changes_nothing() {
        let (now, normal) = scores(|_| {});
        assert!(!matters(&now, &normal));
        assert_eq!(adjust("minecraft:poison", 200, &now, &normal), 200);
    }

    #[test]
    fn a_body_without_a_liver_keeps_poison_twice_as_long_and_the_helpful_stay_as_they_are() {
        let (now, normal) = scores(|s| {
            s.insert("detoxification", 0.0);
        });
        assert!(matters(&now, &normal));
        assert_eq!(adjust("minecraft:poison", 100, &now, &normal), 200);
        assert_eq!(adjust("minecraft:speed", 100, &now, &normal), 100);
        assert_eq!(adjust("minecraft:glowing", 100, &now, &normal), 100);
        // Two livers filter more: 2 / (1 + 2) of the time.
        let (now, normal) = scores(|s| {
            let n = get(s, "detoxification");
            s.insert("detoxification", n * 2.0);
        });
        assert_eq!(adjust("minecraft:poison", 90, &now, &normal), 60);
        // A body that filters backwards is poisoned for good.
        let (now, normal) = scores(|s| {
            let n = get(s, "detoxification");
            s.insert("detoxification", -n * 2.0);
        });
        assert_eq!(adjust("minecraft:poison", 90, &now, &normal), 9999);
    }

    #[test]
    fn purging_organs_cut_what_helps_and_withered_bones_the_wither() {
        let (now, normal) = scores(|s| {
            s.insert("buff_purging", 2.0);
            s.insert("withered", 1.0);
        });
        assert_eq!(adjust("minecraft:regeneration", 100, &now, &normal), 50);
        assert_eq!(adjust("minecraft:wither", 90, &now, &normal), 60);
        assert_eq!(adjust("minecraft:poison", 90, &now, &normal), 90);
        assert_eq!(adjust("minecraft:regeneration", -1, &now, &normal), -1, "endless stays endless");
    }
}
