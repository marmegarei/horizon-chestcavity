//! A mob's chest cavity, kept on the mob itself (a server attachment, saved with it): its type's organs
//! until a player takes one out or puts one in with the Chest Opener.
use crate::screen;
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

/// Opens the mob's screen for player `uuid`: its window id, or what to tell the player.
pub fn open(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str) -> Result<i32, String> {
    let Some(cavity) = load(host, entity, mob) else { return Err(format!("{} has no chest cavity", mob_name(mob))) };
    let title = Text::literal(format!("{}'s Chest Cavity", mob_name(mob)));
    host.open_menu(uuid, 3, title, screen::items(&cavity)).ok_or_else(|| "No screen here: the chest cavity needs a server player".to_string())
}

/// A click on menu slot `slot` of the mob's screen; what to tell the player, if anything.
pub fn click(host: &mut dyn Host, uuid: u128, entity: u32, mob: &str, slot: i32) -> Option<String> {
    let Some(mut cavity) = load(host, entity, mob) else { return Some(format!("{} is gone", mob_name(mob))) };
    let click = screen::click(host, uuid, &mut cavity, slot);
    if click.changed {
        save(host, entity, &cavity);
        host.set_menu_items(uuid, screen::items(&cavity));
    }
    click.msg
}

#[cfg(test)]
mod tests;
