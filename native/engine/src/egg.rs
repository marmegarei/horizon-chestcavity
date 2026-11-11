//! The dragon egg: the Chest Opener used on it grows the chest of whoever used it a fourth row, and the egg is spent. The
//! hook is asked before the egg's own use, which would teleport it, so this takes the click.
use crate::texts::msg;
use crate::{ChestCavity, lock, trace};
use horizon_client::mod_api::Host;

const EGG: &str = "minecraft:dragon_egg";

impl ChestCavity {
    /// The player `uuid`, in `dimension`, used the Chest Opener on the block at `pos`: whether that was the dragon egg, so
    /// that the click is taken. The first time their chest grows and the egg is set down for the server tick to remove; once
    /// it has grown the egg is left as it is.
    pub(crate) fn egg_used(&self, host: &mut dyn Host, uuid: u128, dimension: &str, pos: [i32; 3]) -> bool {
        if host.block(pos) != EGG {
            return false;
        }
        let grew = self.with(host, uuid, |body, host| body.grow(host, uuid));
        trace::line(&*host, format!("the opener on the dragon egg at {pos:?} by {uuid:x}: grew {grew}"));
        if grew {
            lock(&self.eggs).push((dimension.to_string(), pos));
            host.action_bar(uuid, msg("chestcavity.msg.grown").text());
            host.play_sound_to(uuid, "minecraft:entity.ender_dragon.growl", "player", None, 0.6, 1.2);
        } else {
            host.action_bar(uuid, msg("chestcavity.msg.already_grown").text());
        }
        true
    }

    /// Takes out of the world the eggs that were spent: the server tick, where a mod may change blocks.
    pub(crate) fn spend_eggs(&self, host: &mut dyn Host) {
        let eggs = std::mem::take(&mut *lock(&self.eggs));
        for (dimension, pos) in eggs {
            let Some(air) = host.default_state("minecraft:air") else { continue };
            let gone = host.set_block(&dimension, pos, air, 2);
            trace::line(&*host, format!("the dragon egg at {pos:?} taken away: {gone}"));
        }
    }
}

#[cfg(test)]
mod tests;
