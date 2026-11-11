//! What dies drops: the organs a dead body gives up (`ChestCavityUtil.onDeath`) and the organs of the mobs a player kills
//! (`ChestCavityLootEntry`). The loot hook only unpacks what dropped them; the rules are here so the tests can run them.
use crate::screen::{organ_stack, stack};
use crate::{ChestCavity, lock, mob, trace};
use chestcavity_shared::cavity::{get, mob_kind};
use chestcavity_shared::drops::{self, Luck};
use horizon_client::mod_api::{Host, ItemStack};

/// Whether `item` is one of the cleavers (`#chestcavity:butchering_tool`).
fn is_cleaver(item: &str) -> bool {
    item.starts_with("chestcavity:") && item.ends_with("_cleaver")
}

impl ChestCavity {
    /// What the weapon in `killer`'s hand does to the chance of organs: their Looting, ten times over with a cleaver, and
    /// whether it is one. No weapon, no luck.
    fn luck(&self, host: &mut dyn Host, killer: Option<u128>) -> (Luck, bool) {
        let cfg = self.cfg();
        let held = killer.and_then(|k| host.held_stack(k));
        let cleaver = held.as_ref().is_some_and(|h| is_cleaver(&h.item));
        let looting = held.as_ref().map_or(0, |h| h.enchantment_level("minecraft:looting")) * if cleaver { 10 } else { 1 };
        (Luck { looting, base: cfg.universal_donor_rate, boost: cfg.looting_boost }, cleaver)
    }

    /// The player `uuid` died: the organs that are not their own fall with the rest of what they carried.
    pub(crate) fn player_dropped(&self, host: &mut dyn Host, uuid: u128, drops: &mut Vec<ItemStack>) {
        let keep = self.cfg().keep_chest_cavity;
        let gone = self.with(host, uuid, |body, host| body.die(host, uuid, keep));
        trace::line(&*host, format!("player {uuid:x} died (keeps its cavity: {keep}): {} organs leave the body", gone.len()));
        drops.extend(gone.iter().map(|s| organ_stack(&s.id, s.count, s.owner)));
    }

    /// Mob `entity` of type `mob` died, killed by the player `killer` when the game says one; `rolled` is whether its loot table
    /// rolled. `drops` is what it drops, and gets the organs.
    pub(crate) fn mob_dropped(&self, host: &mut dyn Host, entity: u32, mob: &str, killer: Option<u128>, rolled: bool, drops: &mut Vec<ItemStack>) {
        // A mob whose chest was opened dies by the cavity it has now: it gives up what is not its own, its loot table adds no
        // organs, and a wither without its star leaves none.
        if let Some((mut cavity, owner)) = mob::existing_owned(host, entity) {
            if let Some(kind) = mob_kind(mob).filter(|_| mob == "minecraft:wither") {
                if get(&cavity.scores(kind), "health") <= 0.0 {
                    drops.retain(|d| &*d.item != "minecraft:nether_star");
                }
            }
            let given = cavity.drop_unbound(owner);
            trace::line(&*host, format!("{mob} {entity} died with its chest opened: {} organs are not its own", given.len()));
            drops.extend(given.iter().map(|s| organ_stack(&s.id, s.count, s.owner)));
            return;
        }
        if !rolled {
            return;
        }
        let (luck, cleaver) = self.luck(host, killer);
        let mut loot: Vec<(String, u32)> = drops::for_mob(mob, &mut lock(&self.rng), luck).into_iter().map(|(id, n)| (id.to_string(), n)).collect();
        if cleaver {
            loot = drops::butcher(loot);
        }
        if !loot.is_empty() {
            trace::line(&*host, format!("{mob} {entity} died to {killer:?} (Looting {}, cleaver {cleaver}): {loot:?}", luck.looting));
        }
        drops.extend(loot.iter().map(|(id, n)| stack(id, *n)));
    }
}

#[cfg(test)]
mod tests;
