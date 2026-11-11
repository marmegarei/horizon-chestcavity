use super::*;
use crate::fake::Fake;
use crate::screen::{EMPTY_SLOT, inventory_slot, stack};
use chestcavity_shared::cavity::SLOTS;

const ME: u128 = 7;

fn heart_slot(body: &Body) -> usize {
    body.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart")
}

#[test]
fn menu_slots_map_to_the_inventory() {
    assert_eq!(inventory_slot(27, 10), None); // the cavity's own
    assert_eq!(inventory_slot(27, 27), Some(9)); // first main row
    assert_eq!(inventory_slot(27, 53), Some(35));
    assert_eq!(inventory_slot(27, 54), Some(0)); // hotbar
    assert_eq!(inventory_slot(27, 62), Some(8));
    assert_eq!(inventory_slot(27, 63), None);
}

#[test]
fn an_organ_out_and_back_in() {
    let mut game = Fake::new();
    let mut body = Body::load(&mut game, ME);
    assert!(game.bases.is_empty(), "a normal body leaves the attributes alone");
    assert!(body.open(&mut game, ME));
    let i = heart_slot(&body);

    // Take the heart out: it is in the inventory, the body is a heart short and it is saved.
    assert_eq!(body.click(&mut game, ME, i as i32), None);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some("chestcavity:heart"));
    assert_eq!(game.base("minecraft:max_health"), 16.0);
    assert_eq!(game.menu.as_ref().unwrap()[i].as_ref().unwrap().item.as_ref(), EMPTY_SLOT);
    assert_eq!(Body::load(&mut game, ME).cavity, body.cavity);

    // Click it in the hotbar (menu slot 27 + 27 + 0): back in, the body is whole again.
    assert_eq!(body.click(&mut game, ME, (SLOTS + 27) as i32), None);
    assert_eq!(game.inventory[0], None);
    assert_eq!(game.base("minecraft:max_health"), 20.0);
    assert_eq!(body.cavity, Cavity::of(player_kind()));
}

#[test]
fn only_organs_go_in_and_a_full_inventory_keeps_them_out() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    game.inventory[2] = Some(stack("chestcavity:chest_opener", 1));
    let msg = body.click(&mut game, ME, (SLOTS + 27 + 2) as i32);
    assert!(msg.is_some_and(|m| m.contains("not an organ")));
    assert!(game.inventory[2].is_some());

    game.inventory = vec![Some(stack("minecraft:stone", 1)); 36];
    let i = heart_slot(&body);
    assert_eq!(body.click(&mut game, ME, i as i32).as_deref(), Some("No room in your inventory"));
    assert!(body.cavity.slots[i].is_some(), "the heart stays when it has nowhere to go");
}

#[test]
fn no_heart_bleeds_and_kidneys_poison() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    let i = heart_slot(&body);
    body.take_out(&mut game, ME, i);
    body.ticks = 20;
    body.beat(&mut game, ME, "Steve", "minecraft:air");
    body.ticks = 40;
    body.beat(&mut game, ME, "Steve", "minecraft:air");
    assert_eq!(game.commands, ["damage Steve 1 minecraft:generic", "damage Steve 2 minecraft:generic"]);
    assert!(game.effects.is_empty());
}
