//! A mob's chest cavity, kept on the mob itself (a server attachment, saved with it): its type's organs
//! until a player takes one out or puts one in with the Chest Opener.
use crate::body::NO_HELD;
use crate::screen::{self, Click};
use chestcavity_shared::cavity::{Cavity, mob_kind, mob_name};
use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Tag, Text};

static ORGANS: AttachmentType<String> = AttachmentType {
    namespace: "chestcavity",
    name: "organs",
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: |s| Ok([("organs".into(), Tag::String(s.as_str().into()))].into()),
        decode: |d: &Compound| match d.get(&"organs".into()) {
            Some(Tag::String(s)) => Ok(s.to_string_lossy()),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};

/// The cavity of mob `entity` of type `mob` (`minecraft:cow`): its type's, or as a player left it. None when
/// the type has no chest cavity or the mob is gone.
pub fn load(host: &mut dyn Host, entity: u32, mob: &str) -> Option<Cavity> {
    let kind = mob_kind(mob)?;
    match ORGANS.get_existing(&mut *host, Holder::Entity(entity)) {
        Ok(Some(text)) => Some(Cavity::from_text(&text)),
        Ok(None) => Some(Cavity::of(kind)),
        Err(_) => None,
    }
}

/// Keeps the cavity on the mob. It can't fail for a mob `load` just found: the codec always encodes.
fn save(host: &mut dyn Host, entity: u32, cavity: &Cavity) {
    let _ = ORGANS.set(&mut *host, Holder::Entity(entity), cavity.to_text());
}

/// "Cow's Chest Cavity".
pub fn title(mob: &str) -> String {
    format!("{}'s Chest Cavity", mob_name(mob))
}

/// Opens the mob's server menu for player `uuid`: its window id; none for the window's own player,
/// whom the game's menus don't reach.
pub fn open_menu(host: &mut dyn Host, uuid: u128, cavity: &Cavity, mob: &str) -> Option<i32> {
    host.open_menu(uuid, 3, Text::literal(title(mob)), screen::items(cavity))
}

/// Changes the mob's cavity with `f`, keeping it when `f` says it changed: the cavity as it is now and
/// what to tell the player. None when the mob has no chest cavity or is gone.
pub fn edit(host: &mut dyn Host, entity: u32, mob: &str, f: impl FnOnce(&mut dyn Host, &mut Cavity) -> Click) -> Option<(Cavity, Option<String>)> {
    let mut cavity = load(host, entity, mob)?;
    let click = f(host, &mut cavity);
    if click.changed {
        save(host, entity, &cavity);
    }
    Some((cavity, click.msg))
}

/// A click on menu slot `slot` of the mob's screen; what to tell the player, if anything.
pub fn click(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str, slot: i32) -> Option<String> {
    let Some((cavity, msg)) = edit(host, entity, mob, |host, cavity| screen::click(host, uuid, cavity, slot)) else {
        return Some(format!("{} is gone", mob_name(mob)));
    };
    host.set_menu_items(uuid, screen::items(&cavity));
    msg
}

/// Puts the organ in the player's main hand into the mob; what to tell the player.
pub fn put_held(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str) -> (Option<Cavity>, String) {
    let Some(held) = host.held_stack(uuid) else { return (load(host, entity, mob), NO_HELD.into()) };
    let edited = edit(host, entity, mob, |host, cavity| {
        screen::put_in(host, cavity, held.clone(), |host, h| host.take_items(&h.item, h.count) == h.count)
    });
    match edited {
        Some((cavity, msg)) => (Some(cavity), msg.unwrap_or_else(|| format!("Put {} in", held.item))),
        None => (None, format!("{} is gone", mob_name(mob))),
    }
}

#[cfg(test)]
mod tests;
