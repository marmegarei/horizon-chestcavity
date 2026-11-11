//! The abilities organs give, set off with a key (V for the utility ones, R for the attacks, as in Chest Cavity) or with
//! `/cc ability <name>`: the score behind each, its key group, how long it rests afterwards, and what the shots are.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Utility,
    Attack,
}

pub struct Ability {
    pub name: &'static str,
    /// The organ score that gives it (and how many shots it throws, as a whole number).
    pub score: &'static str,
    pub group: Group,
    /// Ticks it rests after it is used (the original's `*_COOLDOWN`).
    pub rest: u32,
}

use Group::{Attack, Utility};

pub const ABILITIES: &[Ability] = &[
    Ability { name: "creepy", score: "creepy", group: Attack, rest: 200 },
    Ability { name: "dragon_bombs", score: "dragon_bombs", group: Attack, rest: 200 },
    Ability { name: "forceful_spit", score: "forceful_spit", group: Attack, rest: 20 },
    Ability { name: "ghastly", score: "ghastly", group: Attack, rest: 60 },
    Ability { name: "pyromancy", score: "pyromancy", group: Attack, rest: 78 },
    Ability { name: "grazing", score: "grazing", group: Utility, rest: 0 },
    Ability { name: "iron_repair", score: "iron_repair", group: Utility, rest: 1200 },
    Ability { name: "silk", score: "silk", group: Utility, rest: 20 },
];

/// The abilities a key group (`utility`, `attack`) sets off, or the one called `which`.
pub fn picked(which: &str) -> Vec<&'static Ability> {
    let group = match which {
        "utility" => Some(Utility),
        "attack" => Some(Attack),
        _ => None,
    };
    ABILITIES.iter().filter(|a| group.map_or(a.name == which, |g| a.group == g)).collect()
}

/// What a shooting ability throws: the entity, how fast, and what each shot costs a player in exhaustion.
pub struct Shot {
    pub kind: &'static str,
    pub speed: f64,
    pub tire: f32,
}

pub fn shot(name: &str) -> Option<Shot> {
    Some(match name {
        "pyromancy" => Shot { kind: "minecraft:small_fireball", speed: 0.1, tire: 0.1 },
        "ghastly" => Shot { kind: "minecraft:fireball", speed: 0.1, tire: 0.3 },
        "dragon_bombs" => Shot { kind: "minecraft:dragon_fireball", speed: 0.1, tire: 0.6 },
        "forceful_spit" => Shot { kind: "minecraft:llama_spit", speed: 2.0, tire: 0.1 },
        _ => return None,
    })
}

/// The unit vector a body looks along, from its yaw and pitch in degrees (`Entity.getRotationVector`).
pub fn aim(yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::get;
    use crate::organs::ORGANS;

    #[test]
    fn keys_pick_their_groups_and_names_pick_one() {
        assert!(picked("utility").iter().all(|a| a.group == Utility) && !picked("utility").is_empty());
        assert!(picked("attack").iter().any(|a| a.name == "pyromancy"));
        assert_eq!(picked("silk").len(), 1);
        assert!(picked("nothing").is_empty());
        assert!(ABILITIES.iter().all(|a| a.group == Utility || shot(a.name).is_some() || a.name == "creepy"));
    }

    #[test]
    fn every_ability_has_an_organ_that_gives_it() {
        for a in ABILITIES {
            assert!(ORGANS.iter().any(|o| o.scores.iter().any(|s| s.0 == a.score)), "no organ gives {}", a.name);
        }
        let _ = get;
    }

    #[test]
    fn the_aim_follows_yaw_and_pitch() {
        let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(near(aim(0.0, 0.0), [0.0, 0.0, 1.0]), "yaw 0 faces south");
        assert!(near(aim(90.0, 0.0), [-1.0, 0.0, 0.0]), "yaw 90 faces west");
        assert!(near(aim(0.0, -90.0), [0.0, 1.0, 0.0]), "pitch -90 looks up");
    }
}
