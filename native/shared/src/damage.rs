//! What the organs do to the damage a body takes, as Chest Cavity's `applyDefenses` does: ribs and bones cut the
//! blows armour would take, fireproof organs cut fire, impact resistant ones cut falls and wall crashes, and
//! springy ones soften a fall. The game tells the hook only the damage type and amount, so this is plain data.
use crate::cavity::{Scores, get};

/// Damage cut by 4 stacks of ribs (`BONE_DEFENSE`), by 4 fireproof organs and by 4 impact resistant ones.
const BONE_DEFENSE: f32 = 0.5;
const FIREPROOF_DEFENSE: f32 = 0.75;
const IMPACT_DEFENSE: f32 = 0.75;
/// Beyond's `FROST_IMMUNITY`: the frost resistance that no cold hurts through.
const FROST_IMMUNITY: f32 = 5.0;

/// Vanilla's `#minecraft:bypasses_armor`: what armour (and so the ribs) does not cut.
const BYPASSES_ARMOR: &[&str] = &[
    "on_fire", "in_wall", "cramming", "drown", "fly_into_wall", "generic", "wither", "dragon_breath", "starve", "fall", "ender_pearl",
    "freeze", "stalagmite", "magic", "indirect_magic", "out_of_world", "generic_kill", "sonic_boom", "outside_border", "dry_out",
];
/// `#minecraft:is_fire`, `#minecraft:is_fall` and `#minecraft:is_projectile`.
const IS_FIRE: &[&str] = &["in_fire", "campfire", "on_fire", "lava", "hot_floor", "unattributed_fireball", "fireball"];
const IS_FALL: &[&str] = &["fall", "ender_pearl", "stalagmite"];
const IS_PROJECTILE: &[&str] = &["arrow", "trident", "mob_projectile", "spit", "fireball", "unattributed_fireball", "wither_skull", "thrown", "wind_charge"];

fn plain(damage_type: &str) -> &str {
    damage_type.strip_prefix("minecraft:").unwrap_or(damage_type)
}

/// How a body differs from its normal one in what cuts damage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Resist {
    /// `defense` above (or below) normal: four points halve the blows armour would take.
    pub bone: f32,
    pub leaping: f32,
    pub impact: f32,
    pub fireproof: f32,
    /// Teleports away from projectiles when above 0 (`arrow_dodging`).
    pub dodging: f32,
    /// Beyond's `frost_resistance` and `fire_resistance`: immune at 5 to cold, at 5 to hot ground, at 10 to fire and at
    /// 15 to lava, and below that they cut the damage in proportion.
    pub frost: f32,
    pub fire: f32,
}

impl Resist {
    pub fn of(now: &Scores, normal: &Scores) -> Resist {
        Resist {
            bone: get(now, "defense") - get(normal, "defense"),
            leaping: get(now, "leaping") - get(normal, "leaping"),
            impact: get(now, "impact_resistant"),
            fireproof: get(now, "fire_resistant"),
            dodging: get(now, "arrow_dodging"),
            frost: get(now, "frost_resistance"),
            fire: get(now, "fire_resistance"),
        }
    }

    /// Whether this body is the normal one as far as damage goes.
    pub fn is_normal(&self) -> bool {
        *self == Resist::default()
    }

    /// Whether `damage_type` is a projectile's, which a dodging body steps away from.
    pub fn dodges(&self, damage_type: &str) -> bool {
        self.dodging > 0.0 && IS_PROJECTILE.contains(&plain(damage_type))
    }

    /// The damage of `amount` of `damage_type` (`minecraft:fall`) after the organs.
    pub fn apply(&self, damage_type: &str, amount: f32) -> f32 {
        let kind = plain(damage_type);
        let mut damage = amount;
        if !BYPASSES_ARMOR.contains(&kind) {
            damage *= (1.0 - BONE_DEFENSE).powf(self.bone / 4.0);
        }
        let fall = IS_FALL.contains(&kind);
        if fall && self.leaping > 0.0 {
            damage = (damage - self.leaping * self.leaping / 4.0).max(0.0);
        }
        if (fall || kind == "fly_into_wall") && self.impact > 0.0 {
            damage *= (1.0 - IMPACT_DEFENSE).powf(self.impact / 4.0);
        }
        if IS_FIRE.contains(&kind) && self.fireproof > 0.0 {
            damage *= (1.0 - FIREPROOF_DEFENSE).powf(self.fireproof / 4.0);
        }
        if kind == "freeze" && self.frost > 0.0 {
            damage *= (1.0 - self.frost / FROST_IMMUNITY).max(0.0);
        }
        if IS_FIRE.contains(&kind) && self.fire > 0.0 {
            // ponytail: Beyond says the hit is "attenuated" below the immunity; this cuts it in proportion.
            let immunity = match kind {
                "hot_floor" | "campfire" => 5.0,
                "lava" => 15.0,
                _ => 10.0,
            };
            damage *= (1.0 - self.fire / immunity).max(0.0);
        }
        damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::{Cavity, player_kind};

    #[test]
    fn a_normal_body_takes_what_it_is_dealt_and_the_organs_cut_what_they_cut() {
        let kind = player_kind();
        let normal = Cavity::of(kind).scores(kind);
        assert!(Resist::of(&normal, &normal).is_normal());
        assert_eq!(Resist::of(&normal, &normal).apply("minecraft:mob_attack", 7.0), 7.0);

        // Four more points of bone halve a mob's blow, not a fall.
        let bony = Resist { bone: 4.0, ..Resist::default() };
        assert_eq!(bony.apply("minecraft:mob_attack", 8.0), 4.0);
        assert_eq!(bony.apply("minecraft:fall", 8.0), 8.0);
        assert_eq!(bony.apply("minecraft:magic", 8.0), 8.0);
        // Fewer take more.
        assert!(Resist { bone: -4.0, ..Resist::default() }.apply("minecraft:arrow", 4.0) > 4.0);

        // Fire only from fireproof organs; falls from springy and impact resistant ones.
        let fireproof = Resist { fireproof: 4.0, ..Resist::default() };
        assert_eq!(fireproof.apply("minecraft:lava", 4.0), 1.0);
        assert_eq!(fireproof.apply("minecraft:fall", 4.0), 4.0);
        let springy = Resist { leaping: 2.0, impact: 4.0, ..Resist::default() };
        assert_eq!(springy.apply("minecraft:fall", 5.0), 1.0, "5 - 2*2/4 = 4, a quarter of that after the impact organs");
        assert_eq!(springy.apply("minecraft:fly_into_wall", 4.0), 1.0);
    }

    #[test]
    fn beyonds_frost_and_fire_resistance_grow_into_immunity() {
        let frost = Resist { frost: 5.0, ..Resist::default() };
        assert_eq!(frost.apply("minecraft:freeze", 3.0), 0.0);
        assert_eq!(Resist { frost: 2.5, ..Resist::default() }.apply("minecraft:freeze", 4.0), 2.0);
        assert_eq!(frost.apply("minecraft:on_fire", 3.0), 3.0, "frost is not fire resistance");
        let fire = Resist { fire: 10.0, ..Resist::default() };
        assert_eq!(fire.apply("minecraft:on_fire", 3.0), 0.0);
        assert_eq!(fire.apply("minecraft:hot_floor", 3.0), 0.0);
        assert!((fire.apply("minecraft:lava", 6.0) - 2.0).abs() < 1e-6, "lava needs 15");
        assert_eq!(fire.apply("minecraft:mob_attack", 3.0), 3.0);
    }

    #[test]
    fn only_a_dodging_body_dodges_and_only_projectiles() {
        let dodger = Resist { dodging: 1.0, ..Resist::default() };
        assert!(dodger.dodges("minecraft:arrow"));
        assert!(!dodger.dodges("minecraft:fall"));
        assert!(!Resist::default().dodges("minecraft:arrow"));
    }
}
