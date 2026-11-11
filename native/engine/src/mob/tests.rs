use super::*;
use crate::fake::Fake;
use crate::screen::EMPTY_SLOT;
use chestcavity_shared::cavity::SLOTS;
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;

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
    let count = |game: &mut Fake| load(game, ZOMBIE, "minecraft:zombie").unwrap().slots.iter().flatten().filter(|s| s.id == id).count();
    let start = count(&mut game);

    // Its screen shows its type's organs under its name.
    let cavity = load(&mut game, ZOMBIE, "minecraft:zombie").unwrap();
    assert_eq!(open_menu(&mut game, ME, &cavity, "minecraft:zombie"), Some(5));
    assert_eq!(game.menu_title.as_deref(), Some("Cavidade torácica de Zombie"));
    assert_eq!(game.menu.as_ref().unwrap()[slot].as_ref().unwrap().item.as_ref(), id);

    // Take the organ out: it is in the player's inventory, the screen shows a pane and the mob keeps it that way.
    assert_eq!(click(&mut game, ME, ZOMBIE, "minecraft:zombie", slot as i32), None);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some(id));
    assert_eq!(game.menu.as_ref().unwrap()[slot].as_ref().unwrap().item.as_ref(), EMPTY_SLOT);
    assert!(load(&mut game, ZOMBIE, "minecraft:zombie").unwrap().slots[slot].is_none());
    assert_eq!(count(&mut game), start - 1);

    // Put it back from the hotbar (menu slot 27 + 27 + 0): it goes in the first empty slot.
    assert_eq!(click(&mut game, ME, ZOMBIE, "minecraft:zombie", (SLOTS + 27) as i32), None);
    assert_eq!(game.inventory[0], None);
    assert_eq!(count(&mut game), start, "the organ is in the mob again");
}

#[test]
fn a_mob_without_a_body_or_gone_gives_nothing() {
    let (mut game, slot, _) = zombie_world();
    assert!(load(&mut game, ZOMBIE, "minecraft:villager_of_nowhere").is_none(), "no body, no cavity");

    // The zombie is gone by the time of the click: no organ is handed out.
    game.mobs.clear();
    let said = click(&mut game, ME, ZOMBIE, "minecraft:zombie", slot as i32);
    assert_eq!(said.as_deref(), Some("Zombie desapareceu"));
    assert!(game.inventory.iter().all(Option::is_none));
}
