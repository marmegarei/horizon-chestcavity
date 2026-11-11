//! A random teleport: `chestcavity_shared::teleport` chooses the spot, the game says what can be stood on.
use crate::opening::Player;
use chestcavity_shared::drops::Rng;
use chestcavity_shared::teleport;
use horizon_client::mod_api::Host;

/// Moves `who` to a random place within `range` blocks, if one with ground and room can be found.
pub fn randomly(host: &mut dyn Host, who: &Player, range: f64, rng: &mut Rng) -> bool {
    let Some(entity) = u32::try_from(who.entity).ok().and_then(|id| host.entity(id)) else { return false };
    let spot = {
        let game: &dyn Host = host;
        let solid = |pos: [i32; 3]| !game.block_collision(pos).is_empty();
        teleport::find(rng, who.position, range, |[x, y, z]| solid([x, y - 1, z]) && !solid([x, y, z]) && !solid([x, y + 1, z]))
    };
    spot.is_some_and(|at| host.entity_teleport_to(entity, at, None))
}
