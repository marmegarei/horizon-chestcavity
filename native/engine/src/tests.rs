//! The engine as the game drives it: a click with the Chest Opener only asks for a screen, the server tick
//! opens it, and the screen's clicks move the organs.
use super::*;
use crate::fake::Fake;
use chestcavity_shared::cavity::{Cavity, mob_kind, player_kind};
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;
/// The window the double gives every menu it opens.
const WINDOW: i32 = 5;

fn world() -> (ChestCavity, Fake) {
    let mut game = Fake::new();
    game.mobs.insert(ZOMBIE, Attachments::default());
    (ChestCavity::new(1), game)
}

fn heart_slot(cavity: &Cavity) -> usize {
    cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart")
}

#[test]
fn the_opener_asks_and_the_next_server_tick_opens_the_screen() {
    let (cc, mut game) = world();
    assert!(cc.queue(ME, Open::Own));
    assert!(!cc.queue(ME, Open::Own), "a click held down asks again: nothing more");
    assert!(game.menu.is_none(), "nothing opens while the click is being handled");

    cc.open_queued(&mut game);
    assert_eq!(game.menu.as_ref().map(Vec::len), Some(27));
    assert_eq!(game.menu_title.as_deref(), Some("Cavidade torácica"));
    game.menu = None;
    cc.open_queued(&mut game);
    assert!(game.menu.is_none(), "it opens once");

    // A click on the heart takes it out; a throw (mode 4) moves nothing.
    let heart = heart_slot(&Cavity::of(player_kind()));
    cc.clicked(&mut game, ME, WINDOW, heart as i32, 4);
    assert_eq!(game.inventory[0], None);
    cc.clicked(&mut game, ME, WINDOW, heart as i32, 0);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some("chestcavity:heart"));
    assert_eq!(game.base("minecraft:max_health"), 16.0);
    // Another window is not this screen.
    cc.clicked(&mut game, ME, WINDOW + 1, 0, 0);

    // Once the screen has been used it can be asked for again.
    assert!(cc.queue(ME, Open::Own));
}

#[test]
fn a_mobs_cavity_opens_at_the_tick_and_its_organs_come_out() {
    let (cc, mut game) = world();
    let &(slot, id, _) = mob_kind("minecraft:zombie").unwrap().slots.first().unwrap();
    cc.queue(ME, Open::Mob { entity: ZOMBIE, kind: "minecraft:zombie".into() });
    cc.open_queued(&mut game);
    assert_eq!(game.menu_title.as_deref(), Some("Cavidade torácica de Zombie"));
    assert_eq!(game.menu.as_ref().unwrap()[slot as usize].as_ref().unwrap().item.as_ref(), id);

    // A shift-click takes the organ out of the zombie; the zombie keeps it that way.
    cc.clicked(&mut game, ME, WINDOW, slot as i32, 1);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some(id));
    assert!(mob::load(&mut game, ZOMBIE, "minecraft:zombie").unwrap().slots[slot as usize].is_none());

    // Closing the screen forgets it: a later click on another organ is nobody's.
    let &(other, _, _) = mob_kind("minecraft:zombie").unwrap().slots.get(1).unwrap();
    cc.closed(ME, WINDOW);
    game.inventory[0] = None;
    cc.clicked(&mut game, ME, WINDOW, other as i32, 1);
    assert_eq!(game.inventory[0], None);
    assert!(mob::load(&mut game, ZOMBIE, "minecraft:zombie").unwrap().slots[other as usize].is_some());
}

#[test]
fn a_mob_without_a_body_and_a_game_without_menus_both_tell_the_player() {
    let (cc, mut game) = world();
    cc.queue(ME, Open::Mob { entity: ZOMBIE, kind: "minecraft:villager_of_nowhere".into() });
    cc.open_queued(&mut game);
    assert_eq!(game.said, ["Villager Of Nowhere não tem cavidade torácica"]);

    game.server_menus = false;
    cc.queue(ME, Open::Own);
    cc.open_queued(&mut game);
    assert_eq!(game.said.last().map(String::as_str), Some(NO_SCREEN));
}

#[test]
fn an_organ_tells_what_it_is_good_at_in_its_tooltip_and_other_items_say_nothing() {
    let heart = ChestCavity::tooltip_lines("chestcavity:heart");
    assert!(heart.iter().any(|l| l.plain().starts_with("Vida máxima: +")), "{:?}", heart.iter().map(Text::plain).collect::<Vec<_>>());
    assert!(heart.iter().any(|l| l.plain().contains("sem coração sangras")));
    assert!(ChestCavity::tooltip_lines("minecraft:stick").is_empty());
}
