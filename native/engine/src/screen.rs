//! The chest screen, for any cavity (a player's or a mob's): 27 slots as a chest. Click an organ to take it
//! out into the player's inventory, an organ of the player's below to put it in. The functions change a
//! `Cavity` and the player's inventory only; whoever owns the cavity saves it when `changed` says so.
use chestcavity_shared::cavity::{Cavity, SLOTS, Slot, organ};
use horizon_client::mod_api::{Host, ItemStack};

pub(crate) const EMPTY_SLOT: &str = "minecraft:gray_stained_glass_pane";

/// What a click did: whether the cavity changed, and what to tell the player.
#[derive(Debug, Default, PartialEq)]
pub struct Click {
    pub changed: bool,
    pub msg: Option<String>,
}

impl Click {
    fn said(msg: impl Into<String>) -> Click {
        Click { changed: false, msg: Some(msg.into()) }
    }
    fn done() -> Click {
        Click { changed: true, msg: None }
    }
}

pub fn stack(id: &str, count: u32) -> ItemStack {
    ItemStack { item: id.into(), count, damage: 0, extra: None }
}

/// The player's inventory slot (0..9 the hotbar) of a menu slot, for a menu of
/// `size` own slots: the player's inventory comes below them, main rows first.
pub fn inventory_slot(size: usize, slot: usize) -> Option<usize> {
    match slot.checked_sub(size)? {
        n @ 0..27 => Some(n + 9),
        n @ 27..36 => Some(n - 27),
        _ => None,
    }
}

/// What the screen shows: each organ, a pane for an empty slot.
pub fn items(cavity: &Cavity) -> Vec<Option<ItemStack>> {
    cavity.slots.iter().map(|s| Some(s.as_ref().map_or_else(|| stack(EMPTY_SLOT, 1), |s| stack(&s.id, s.count)))).collect()
}

/// A click on menu slot `slot` by player `uuid`.
pub fn click(host: &mut dyn Host, uuid: u128, cavity: &mut Cavity, slot: i32) -> Click {
    let Ok(slot) = usize::try_from(slot) else { return Click::default() };
    if slot < SLOTS {
        return take_out(host, uuid, cavity, slot);
    }
    let held = inventory_slot(SLOTS, slot)
        .and_then(|from| host.player_stacks(uuid)?.get(from).cloned().flatten().map(|held| (from, held)));
    let Some((from, held)) = held else { return Click::default() };
    put_in(host, cavity, held, |host, held| host.take_stacks(uuid, &[(from, held.clone())]).is_ok())
}

/// Takes the organ in cavity slot `i` out into the player's inventory; an empty slot does nothing.
pub fn take_out(host: &mut dyn Host, uuid: u128, cavity: &mut Cavity, i: usize) -> Click {
    let Some(s) = cavity.slots.get(i).cloned().flatten() else { return Click::default() };
    let kept = host.give_stack(uuid, stack(&s.id, s.count)).map_or(0, |left| left.count);
    if kept >= s.count {
        return Click::said("No room in your inventory");
    }
    cavity.slots[i] = (kept > 0).then(|| Slot { id: s.id, count: kept });
    Click::done()
}

/// Puts `held`, an organ of the player's, in the first empty slot; `remove` takes it from where it is.
pub fn put_in(host: &mut dyn Host, cavity: &mut Cavity, held: ItemStack, remove: impl FnOnce(&mut dyn Host, &ItemStack) -> bool) -> Click {
    let Some(o) = organ(&held.item) else { return Click::said(format!("{} is not an organ", held.item)) };
    if held.count > o.stack {
        return Click::said(format!("Put at most {} {} in at a time", o.stack, o.name));
    }
    let Some(i) = cavity.slots.iter().position(Option::is_none) else {
        return Click::said("Every slot is taken: take an organ out first");
    };
    if !remove(host, &held) {
        return Click::said("That stack changed");
    }
    cavity.slots[i] = Some(Slot { id: held.item.to_string(), count: held.count });
    Click::done()
}

/// One line a slot: " 3. Heart", " 4. (empty)".
pub fn list(cavity: &Cavity) -> Vec<String> {
    cavity
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| match s {
            Some(s) => {
                let name = organ(&s.id).map_or(s.id.as_str(), |o| o.name);
                if s.count > 1 { format!("{:>2}. {name} x{}", i + 1, s.count) } else { format!("{:>2}. {name}", i + 1) }
            }
            None => format!("{:>2}. (empty)", i + 1),
        })
        .collect()
}
