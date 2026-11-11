//! A mob's chest cavity, kept on the mob itself (a server attachment, saved with it): its type's organs
//! until a player takes one out or puts one in with the Chest Opener.
use crate::screen::{self, Click};
use crate::texts::{Msg, item_name_of_mob, msg};
use chestcavity_shared::cavity::{Cavity, Kind, mob_kind};
use chestcavity_shared::drops::Rng;
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

/// The body a mob's organs are born in: its uuid (its id, for a game that does not say it).
pub fn owner(host: &mut dyn Host, entity: u32) -> u128 {
    host.entity(entity).and_then(|e| host.entity_uuid(e)).unwrap_or(u128::from(entity))
}

/// The cavity of a mob nobody has opened: its type's organs, each born in the mob's body but for the share `donors` of them
/// that fit any body. The donors are picked from the mob's uuid, so they are the same ones every time it is looked at.
fn fresh(host: &mut dyn Host, entity: u32, kind: &Kind, donors: f32) -> Cavity {
    let owner = owner(host, entity);
    let mut rng = Rng(((owner as u64) ^ ((owner >> 64) as u64)).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    rng.next();
    Cavity::of(kind).born(owner, || rng.chance(donors))
}

/// The cavity of mob `entity` of type `mob` (`minecraft:cow`): its type's, or as a player left it. None when
/// the type has no chest cavity or the mob is gone. `donors` is the share of a mob's organs that fit any body.
pub fn load(host: &mut dyn Host, entity: u32, mob: &str, donors: f32) -> Option<Cavity> {
    let kind = mob_kind(mob)?;
    match ORGANS.get_existing(&mut *host, Holder::Entity(entity)) {
        Ok(Some(text)) => Some(restore(host, entity, &text).0),
        Ok(None) => Some(fresh(host, entity, kind, donors)),
        Err(e) => {
            crate::trace::line(&*host, format!("{mob} {entity}: the game keeps no cavity for it ({e:?})"));
            None
        }
    }
}

/// The cavity a player left on mob `entity`, if one ever changed it.
pub fn existing(host: &mut dyn Host, entity: u32) -> Option<Cavity> {
    existing_owned(host, entity).map(|(cavity, _)| cavity)
}

/// The first line of a saved cavity: the body its organs are its own in.
const OWNER_LINE: &str = "owner";

/// The body a saved cavity says is its own.
fn saved_owner(text: &str) -> Option<u128> {
    let mut words = text.lines().next()?.split(' ');
    (words.next()? == OWNER_LINE).then(|| words.next().and_then(|o| u128::from_str_radix(o, 16).ok())).flatten()
}

/// The cavity a player left on mob `entity` and the body it is its own in: the one it was saved with, since a dying mob
/// may not be there to ask (the game's answer, for a cavity saved without one).
pub fn existing_owned(host: &mut dyn Host, entity: u32) -> Option<(Cavity, u128)> {
    let Ok(Some(text)) = ORGANS.get_existing(&mut *host, Holder::Entity(entity)) else { return None };
    Some(restore(host, entity, &text))
}

/// A saved cavity as the mob lives by it, and the body it is own to: one saved before organs had owners (before version
/// 0.8) has every organ its own.
fn restore(host: &mut dyn Host, entity: u32, text: &str) -> (Cavity, u128) {
    match saved_owner(text) {
        Some(owner) => (Cavity::from_text(text), owner),
        None => {
            let owner = owner(host, entity);
            (Cavity::from_text(text).born(owner, || false), owner)
        }
    }
}

/// Keeps the cavity on the mob, with the body its organs are its own in. It can't fail for a mob `load` just found: the
/// codec always encodes.
fn save(host: &mut dyn Host, entity: u32, cavity: &Cavity) {
    let text = format!("{OWNER_LINE} {:032x}\n{}", owner(host, entity), cavity.to_text());
    let _ = ORGANS.set(&mut *host, Holder::Entity(entity), text);
}

/// "Cow's Chest Cavity": the mob's name in the player's language, its type's as a fallback.
pub fn title(mob: &str) -> Text {
    msg("chestcavity.title.mob").arg(item_name_of_mob(mob)).text()
}
/// Opens the mob's server menu for player `uuid`: its window id; none for the window's own player,
/// whom the game's menus don't reach.
pub fn open_menu(host: &mut dyn Host, uuid: u128, cavity: &Cavity, mob: &str) -> Option<i32> {
    host.open_menu(uuid, (cavity.slots.len() / 9) as u8, title(mob), screen::items(cavity))
}

/// Changes the mob's cavity with `f`, keeping it when `f` says it changed: the cavity as it is now and
/// what to tell the player. None when the mob has no chest cavity or is gone.
pub fn edit(host: &mut dyn Host, entity: u32, mob: &str, donors: f32, f: impl FnOnce(&mut dyn Host, &mut Cavity) -> Click) -> Option<(Cavity, Option<Msg>)> {
    let mut cavity = load(host, entity, mob, donors)?;
    let click = f(host, &mut cavity);
    if click.changed {
        save(host, entity, &cavity);
    }
    Some((cavity, click.msg))
}

/// A click on menu slot `slot` of the mob's screen; what to tell the player, if anything.
pub fn click(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str, donors: f32, slot: i32) -> Option<Msg> {
    let Some((cavity, msg)) = edit(host, entity, mob, donors, |host, cavity| screen::click(host, uuid, cavity, slot)) else {
        return Some(msg("chestcavity.msg.mob_vanished").arg(item_name_of_mob(mob)));
    };
    host.set_menu_items(uuid, screen::items(&cavity));
    msg
}

#[cfg(test)]
mod tests;
