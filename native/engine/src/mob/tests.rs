use super::*;
use chestcavity_shared::cavity::Slot;
use crate::fake::Fake;
use crate::screen::EMPTY_SLOT;
use chestcavity_shared::cavity::SLOTS;
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;
/// No organ of a mob fits every body.
const NO_DONORS: f32 = 0.0;

/// A world with one zombie in it, and the first organ of a zombie's body: (slot, item).
fn zombie_world() -> (Fake, usize, &'static str) {
    // ponytail: a zombie has several ribs, so counts below compare against the start, not 1.
    let mut game = Fake::new();
    game.mobs.insert(ZOMBIE, Attachments::default());
    let &(slot, id, _) = mob_kind("minecraft:zombie").expect("zombie").slots.first().expect("an organ");
    (game, slot as usize, id)
}

#[test]
fn a_mobs_organs_come_out_and_go_back_in() {
    let (mut game, slot, id) = zombie_world();
    let count = |game: &mut Fake| load(game, ZOMBIE, "minecraft:zombie", NO_DONORS).unwrap().slots.iter().flatten().filter(|s| s.id == id).count();
    let start = count(&mut game);

    // Its screen shows its type's organs under its name.
    let cavity = load(&mut game, ZOMBIE, "minecraft:zombie", NO_DONORS).unwrap();
    assert_eq!(open_menu(&mut game, ME, &cavity, "minecraft:zombie"), Some(5));
    assert_eq!(game.menu_title.as_deref(), Some("Zombie's Chest Cavity"));
    assert_eq!(game.menu.as_ref().unwrap()[slot].as_ref().unwrap().item.as_ref(), id);

    // Take the organ out: it is in the player's inventory, the screen shows a pane and the mob keeps it that way.
    assert_eq!(click(&mut game, ME, ZOMBIE, "minecraft:zombie", NO_DONORS, slot as i32), None);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some(id));
    assert_eq!(game.menu.as_ref().unwrap()[slot].as_ref().unwrap().item.as_ref(), EMPTY_SLOT);
    assert!(load(&mut game, ZOMBIE, "minecraft:zombie", NO_DONORS).unwrap().slots[slot].is_none());
    assert_eq!(count(&mut game), start - 1);

    // Put it back from the hotbar (menu slot 27 + 27 + 0): it goes in the first empty slot.
    assert_eq!(click(&mut game, ME, ZOMBIE, "minecraft:zombie", NO_DONORS, (SLOTS + 27) as i32), None);
    assert_eq!(game.inventory[0], None);
    assert_eq!(count(&mut game), start, "the organ is in the mob again");
}

#[test]
fn a_mob_without_a_body_or_gone_gives_nothing() {
    let (mut game, slot, _) = zombie_world();
    assert!(load(&mut game, ZOMBIE, "minecraft:villager_of_nowhere", NO_DONORS).is_none(), "no body, no cavity");

    // The zombie is gone by the time of the click: no organ is handed out.
    game.mobs.clear();
    let said = click(&mut game, ME, ZOMBIE, "minecraft:zombie", NO_DONORS, slot as i32);
    assert_eq!(said.map(|m| m.plain()).as_deref(), Some("Zombie disappeared"));
    assert!(game.inventory.iter().all(Option::is_none));
}

#[test]
fn the_organs_of_a_mob_are_born_in_its_body_and_a_share_of_them_fit_any_body() {
    let (mut game, _, _) = zombie_world();
    let own = |cavity: &Cavity| cavity.slots.iter().flatten().filter(|s| s.owner.is_some()).count();
    let whole = load(&mut game, ZOMBIE, "minecraft:zombie", NO_DONORS).unwrap();
    let total = whole.slots.iter().flatten().count();
    assert_eq!(own(&whole), total, "with no donors every organ is the zombie's");
    assert!(whole.slots.iter().flatten().all(|s| s.owner == Some(u128::from(ZOMBIE))), "and the game's uuid of the mob says whose");
    // Anyone's body rejects them (the player's own is another body).
    assert_eq!(whole.incompatibility(ME), total);

    let donors = load(&mut game, ZOMBIE, "minecraft:zombie", 1.0).unwrap();
    assert_eq!(own(&donors), 0, "with every organ a donor, none is bound");
    assert_eq!(donors.incompatibility(ME), 0);

    // Which organs are donors does not change from one look to the next.
    let a = load(&mut game, ZOMBIE, "minecraft:zombie", 0.5).unwrap();
    let b = load(&mut game, ZOMBIE, "minecraft:zombie", 0.5).unwrap();
    assert_eq!(a, b);
    assert!(own(&a) > 0 && own(&a) < total, "about half: {} of {total}", own(&a));
}

#[test]
fn a_saved_cavity_remembers_whose_organs_are_its_own_even_if_the_mob_is_not_there_to_ask() {
    let (mut game, slot, _) = zombie_world();
    assert_eq!(existing_owned(&mut game, ZOMBIE), None, "nobody opened it");
    click(&mut game, ME, ZOMBIE, "minecraft:zombie", NO_DONORS, slot as i32);
    let (cavity, owner) = existing_owned(&mut game, ZOMBIE).expect("it was opened");
    assert_eq!(owner, u128::from(ZOMBIE), "the game's uuid of it (here its id)");
    assert!(cavity.slots[slot].is_none() && cavity.slots.iter().flatten().all(|s| s.owner == Some(owner)));
    assert_eq!(saved_owner("owner 0000000000000000000000000000000a\n3 chestcavity:heart 1\n"), Some(10));
    assert_eq!(saved_owner("3 chestcavity:heart 1\n"), None, "a cavity saved before the line existed");
    assert_eq!(Cavity::from_text("owner 0000000000000000000000000000000a\n3 chestcavity:heart 1\n").slots[3], Some(Slot::new("chestcavity:heart", 1)));
}

#[test]
fn a_cavity_saved_before_organs_had_owners_has_them_all_its_own() {
    let (mut game, _, _) = zombie_world();
    ORGANS.set(&mut game as &mut dyn Host, Holder::Entity(ZOMBIE), "3 chestcavity:rotten_heart 1\n5 chestcavity:rotten_lung 1\n".to_string()).expect("saved");
    let (cavity, owner) = existing_owned(&mut game, ZOMBIE).expect("opened once");
    assert!(cavity.slots.iter().flatten().all(|s| s.owner == Some(owner)), "{cavity:?}");
    assert_eq!(cavity.slots.iter().flatten().count(), 2);
    let loaded = load(&mut game, ZOMBIE, "minecraft:zombie", 1.0).unwrap();
    assert!(loaded.slots.iter().flatten().all(|s| s.owner == Some(owner)), "donors are picked for cavities nobody opened, not for these");
}
