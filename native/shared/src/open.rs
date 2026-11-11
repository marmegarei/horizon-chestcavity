//! When a chest can be opened (`isOpenable` of Chest Cavity's chest cavity types): the target's chest must not be covered,
//! and it must be easy to open or the target weak enough.

/// Health at or below which a target is weak enough (`CHEST_OPENER_ABSOLUTE_HEALTH_THRESHOLD`), and the share of its
/// maximum health at or below which it is (`CHEST_OPENER_FRACTIONAL_HEALTH_THRESHOLD`).
pub const ABSOLUTE_HEALTH: f32 = 20.0;
pub const FRACTIONAL_HEALTH: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Open,
    /// Something is worn over the chest.
    Obstructed,
    TooHealthy,
}

/// Whether the chest of a target with `health` out of `max` (when known) can be opened; `armoured` says it wears a
/// chestplate, `easy` that its organs give ease of access.
pub fn verdict(health: f32, max: Option<f32>, armoured: bool, easy: bool, absolute: f32, fraction: f32) -> Verdict {
    if armoured {
        return Verdict::Obstructed;
    }
    let weak = health <= absolute || max.is_some_and(|m| health <= m * fraction);
    if easy || weak { Verdict::Open } else { Verdict::TooHealthy }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weak_uncovered_chest_opens_and_a_covered_or_healthy_one_does_not() {
        let v = |health, max, armoured, easy| verdict(health, max, armoured, easy, ABSOLUTE_HEALTH, FRACTIONAL_HEALTH);
        assert_eq!(v(10.0, Some(10.0), false, false), Verdict::Open, "a cow has 20 or less");
        assert_eq!(v(100.0, Some(100.0), false, false), Verdict::TooHealthy, "a full iron golem");
        assert_eq!(v(50.0, Some(100.0), false, false), Verdict::Open, "half of its health");
        assert_eq!(v(100.0, Some(100.0), false, true), Verdict::Open, "easy access skips the health");
        assert_eq!(v(5.0, Some(20.0), true, true), Verdict::Obstructed, "armour obstructs whatever else");
        assert_eq!(v(60.0, None, false, false), Verdict::TooHealthy, "no maximum known: only the absolute number counts");
    }
}
