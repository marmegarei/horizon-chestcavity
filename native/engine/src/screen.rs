//! The chest screen, for any cavity (a player's or a mob's): its slots as a chest, 27 or 36. Click an organ to take it
//! out into the player's inventory, an organ of the player's below to put it in. The functions change a
//! `Cavity` and the player's inventory only; whoever owns the cavity saves it when `changed` says so.
use crate::texts::{Msg, item_name, msg};
use chestcavity_shared::cavity::{Cavity, Slot, organ};
use chestcavity_shared::layout;
use horizon_client::mod_api::{Host, ItemStack, Tag, Text};

pub(crate) const EMPTY_SLOT: &str = "minecraft:gray_stained_glass_pane";

/// What a click did: whether the cavity changed, and what to tell the player.
#[derive(Debug, Default, PartialEq)]
pub struct Click {
    pub changed: bool,
    pub msg: Option<Msg>,
}

impl Click {
    fn said(msg: Msg) -> Click {
        Click { changed: false, msg: Some(msg) }
    }
    fn done() -> Click {
        Click { changed: true, msg: None }
    }
}

pub fn stack(id: &str, count: u32) -> ItemStack {
    ItemStack { item: id.into(), count, damage: 0, extra: None }
}

/// The key of an organ's owner in its `minecraft:custom_data`.
const OWNER: &str = "chestcavity:owner";

/// An organ as an item: with the body it was born in written on it, when it has one.
pub fn organ_stack(id: &str, count: u32, owner: Option<u128>) -> ItemStack {
    let mut s = stack(id, count);
    if let Some(owner) = owner {
        s.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([(OWNER, Tag::String(format!("{owner:032x}").as_str().into()))])));
    }
    s
}

/// The body an organ item was born in, when it says.
pub fn owner_of(item: &ItemStack) -> Option<u128> {
    match item.custom_data()?.get(&OWNER.into())? {
        Tag::String(s) => u128::from_str_radix(&s.to_string_lossy(), 16).ok(),
        _ => None,
    }
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
    let size = cavity.slots.len();
    if slot < size {
        return take_out(host, uuid, cavity, slot);
    }
    let held = inventory_slot(size, slot)
        .and_then(|from| host.player_stacks(uuid)?.get(from).cloned().flatten().map(|held| (from, held)));
    let Some((from, held)) = held else { return Click::default() };
    put_in(host, cavity, held, |host, held| host.take_stacks(uuid, &[(from, held.clone())]).is_ok())
}

/// Takes the organ in cavity slot `i` out into the player's inventory; an empty slot does nothing.
pub fn take_out(host: &mut dyn Host, uuid: u128, cavity: &mut Cavity, i: usize) -> Click {
    let Some(s) = cavity.slots.get(i).cloned().flatten() else { return Click::default() };
    let kept = host.give_stack(uuid, organ_stack(&s.id, s.count, s.owner)).map_or(0, |left| left.count);
    if kept >= s.count {
        return Click::said(msg("chestcavity.msg.no_space"));
    }
    cavity.slots[i] = (kept > 0).then(|| Slot::new(s.id, kept).owned_by(s.owner));
    Click::done()
}

/// Puts `held`, an organ of the player's, in its place (`layout::place`); `remove` takes it from where it is.
pub fn put_in(host: &mut dyn Host, cavity: &mut Cavity, held: ItemStack, remove: impl FnOnce(&mut dyn Host, &ItemStack) -> bool) -> Click {
    let Some(o) = organ(&held.item) else { return Click::said(msg("chestcavity.msg.not_organ").plain_arg(&held.item)) };
    if held.count > o.stack {
        return Click::said(msg("chestcavity.msg.max_stack").plain_arg(o.stack).arg(item_name(&o.id, o.name)));
    }
    let Some(i) = layout::place(cavity, &held.item) else {
        return Click::said(msg("chestcavity.msg.full"));
    };
    if !remove(host, &held) {
        return Click::said(msg("chestcavity.msg.stale"));
    }
    cavity.slots[i] = Some(Slot::new(held.item.to_string(), held.count).owned_by(owner_of(&held)));
    Click::done()
}

/// One line a slot: " 3. Heart", " 4. (empty)".
pub fn list(cavity: &Cavity) -> Vec<Text> {
    cavity
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let head = Text::literal(format!("{:>2}. ", i + 1));
            match s {
                Some(s) => {
                    let name = organ(&s.id).map_or_else(|| Text::literal(s.id.clone()), |o| item_name(&o.id, o.name));
                    if s.count > 1 { head.append(name).append(Text::literal(format!(" x{}", s.count))) } else { head.append(name) }
                }
                None => head.append(msg("chestcavity.list.empty").text()),
            }
        })
        .collect()
}
