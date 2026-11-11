//! A mob's chest cavity, kept on the mob itself (a server attachment, saved with it): its type's organs
//! until a player takes one out or puts one in with the Chest Opener.
use crate::screen::{self, Click};
use crate::texts::{Msg, item_name_of_mob, msg};
use chestcavity_shared::cavity::{Cavity, mob_kind};
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
        Err(e) => {
            crate::trace::line(&*host, format!("{mob} {entity}: the game keeps no cavity for it ({e:?})"));
            None
        }
    }
}

/// The cavity a player left on mob `entity`, if one ever changed it.
pub fn existing(host: &mut dyn Host, entity: u32) -> Option<Cavity> {
    match ORGANS.get_existing(&mut *host, Holder::Entity(entity)) {
        Ok(Some(text)) => Some(Cavity::from_text(&text)),
        _ => None,
    }
}

/// Keeps the cavity on the mob. It can't fail for a mob `load` just found: the codec always encodes.
fn save(host: &mut dyn Host, entity: u32, cavity: &Cavity) {
    let _ = ORGANS.set(&mut *host, Holder::Entity(entity), cavity.to_text());
}

/// "Cow's Chest Cavity": the mob's name in the player's language, its type's as a fallback.
pub fn title(mob: &str) -> Text {
    msg("chestcavity.title.mob").arg(item_name_of_mob(mob)).text()
}
/// Opens the mob's server menu for player `uuid`: its window id; none for the window's own player,
/// whom the game's menus don't reach.
pub fn open_menu(host: &mut dyn Host, uuid: u128, cavity: &Cavity, mob: &str) -> Option<i32> {
    host.open_menu(uuid, 3, title(mob), screen::items(cavity))
}

/// Changes the mob's cavity with `f`, keeping it when `f` says it changed: the cavity as it is now and
/// what to tell the player. None when the mob has no chest cavity or is gone.
pub fn edit(host: &mut dyn Host, entity: u32, mob: &str, f: impl FnOnce(&mut dyn Host, &mut Cavity) -> Click) -> Option<(Cavity, Option<Msg>)> {
    let mut cavity = load(host, entity, mob)?;
    let click = f(host, &mut cavity);
    if click.changed {
        save(host, entity, &cavity);
    }
    Some((cavity, click.msg))
}

/// A click on menu slot `slot` of the mob's screen; what to tell the player, if anything.
pub fn click(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str, slot: i32) -> Option<Msg> {
    let Some((cavity, msg)) = edit(host, entity, mob, |host, cavity| screen::click(host, uuid, cavity, slot)) else {
        return Some(msg("chestcavity.msg.mob_vanished").arg(item_name_of_mob(mob)));
    };
    host.set_menu_items(uuid, screen::items(&cavity));
    msg
}

#[cfg(test)]
mod tests;
