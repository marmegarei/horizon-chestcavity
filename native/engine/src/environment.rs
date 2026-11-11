//! Where a player is, as their organs care: the light at their head, water on and around them, rain and End Crystals.
use chestcavity_shared::effects::Env;
use horizon_client::mod_api::{Host, NativeServerPlayer};

/// `CRYSTALSYNTHESIS_RANGE`, and Beyond's `CRYSTAL_EFFECT_SEARCH_RANGE`.
const CRYSTAL_RANGE: f64 = 32.0;
const CRYSTAL_CLOSE: f64 = 16.0;

fn water(block: &str) -> bool {
    block.starts_with("minecraft:water") || block.starts_with("minecraft:bubble_column")
}

/// The environment of `p`. The crystals are only looked for (every entity of the level is listed) when `crystals` says
/// the body has a use for them.
pub fn of(host: &mut dyn Host, p: &NativeServerPlayer, crystals: bool) -> Env {
    let [x, y, z] = p.position;
    let at = |dy: f64| [x.floor() as i32, (y + dy).floor() as i32, z.floor() as i32];
    let (feet, eyes) = (host.block(at(0.1)), host.block(at(1.62)));
    let (block, sky) = host.light(at(1.62));
    let submerged = water(&eyes);
    let rain = host.weather().0 > 0.2 && sky >= 15;
    let distance = |pos: &[f64; 3]| (0..3).map(|i| (pos[i] - p.position[i]).powi(2)).sum::<f64>();
    let nearest = if crystals {
        host.level_entities(&p.dimension).iter().filter(|(_, kind, _)| kind == "minecraft:end_crystal").map(|(_, _, pos)| distance(pos)).fold(f64::INFINITY, f64::min)
    } else {
        f64::INFINITY
    };
    Env {
        light: block.max(sky),
        wet: submerged || water(&feet) || rain,
        submerged,
        in_water: submerged || water(&feet),
        crystal: nearest <= CRYSTAL_RANGE * CRYSTAL_RANGE,
        crystal_close: nearest <= CRYSTAL_CLOSE * CRYSTAL_CLOSE,
    }
}
