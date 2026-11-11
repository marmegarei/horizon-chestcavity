//! Where a random teleport lands (`OrganUtil.teleportRandomly`): a few tries at a column within range, looking down
//! it for a place to stand. The engine says what can be stood on; this only chooses.
use crate::drops::Rng;

/// `MAX_TELEPORT_ATTEMPTS`.
const ATTEMPTS: u32 = 5;
/// How far down a column to look for ground.
const DEPTH: u32 = 32;

/// A block centre within `range` blocks of `from` where `standable` (ground under a block with room for a body) is
/// true, after a few random tries; none when they all fail.
pub fn find(rng: &mut Rng, from: [f64; 3], range: f64, mut standable: impl FnMut([i32; 3]) -> bool) -> Option<[f64; 3]> {
    let offset = |rng: &mut Rng| (f64::from(rng.below(2001)) / 1000.0 - 1.0) * range;
    for _ in 0..ATTEMPTS {
        let (x, z, top) = (from[0] + offset(rng), from[2] + offset(rng), from[1] + offset(rng) / 2.0);
        for dy in 0..DEPTH {
            let pos = [x.floor() as i32, (top - f64::from(dy)).floor() as i32, z.floor() as i32];
            if standable(pos) {
                return Some([f64::from(pos[0]) + 0.5, f64::from(pos[1]), f64::from(pos[2]) + 0.5]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lands_on_ground_within_range_or_nowhere() {
        let mut rng = Rng(0x1234_5678_9abc_def1);
        // A flat world whose ground is at y = 64: a block is standable at 65.
        let spot = find(&mut rng, [0.0, 65.0, 0.0], 32.0, |[_, y, _]| y == 65).expect("flat ground");
        assert_eq!(spot[1], 65.0);
        assert!(spot[0].abs() <= 33.0 && spot[2].abs() <= 33.0, "{spot:?}");
        // The hunt stops where the ground never is.
        assert_eq!(find(&mut rng, [0.0, 65.0, 0.0], 32.0, |_| false), None);
    }
}
