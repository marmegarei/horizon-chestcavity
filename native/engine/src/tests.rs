//! The engine as the game drives it: a click with the Chest Opener only asks for a screen, the server tick
//! opens it and charges for it, and the screen's clicks move the organs.
use super::*;
use crate::fake::Fake;
use chestcavity_shared::cavity::{Cavity, mob_kind, player_kind};
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;
/// The window the double gives every menu it opens.
const WINDOW: i32 = 5;
const OVERWORLD: &str = "minecraft:overworld";

fn me() -> Player {
    Player { uuid: ME, dimension: OVERWORLD.into(), entity: 1, health: 20.0, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 }
}

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
    cc.ask_own(me());
    cc.ask_own(me());
    assert!(game.menu.is_none(), "nothing opens while the click is being handled");

    cc.open_queued(&mut game);
    assert_eq!(game.menu.as_ref().map(Vec::len), Some(27));
    assert_eq!(game.menu_title.as_deref(), Some("Chest Cavity"));
    assert_eq!(game.hurts, [(OVERWORLD.into(), 1, 4.0, "minecraft:generic".into(), None)], "opening a chest hurts, once");
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
    cc.ask_own(me());
    cc.open_queued(&mut game);
    assert!(game.menu.is_some());
}

#[test]
fn a_mobs_cavity_opens_at_the_tick_and_its_organs_come_out() {
    let (cc, mut game) = world();
    let &(slot, id, _) = mob_kind("minecraft:zombie").unwrap().slots.first().unwrap();
    cc.ask_mob(me(), ZOMBIE, "minecraft:zombie");
    cc.open_queued(&mut game);
    assert_eq!(game.menu_title.as_deref(), Some("Zombie's Chest Cavity"));
    assert_eq!(game.hurts, [(OVERWORLD.into(), ZOMBIE, 4.0, "minecraft:player_attack".into(), Some(1))], "the zombie pays for it");
    assert_eq!(game.menu.as_ref().unwrap()[slot as usize].as_ref().unwrap().item.as_ref(), id);

    // A shift-click takes the organ out of the zombie; the zombie keeps it that way.
    cc.clicked(&mut game, ME, WINDOW, slot as i32, 1);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some(id));
    assert!(mob::load(&mut game, ZOMBIE, "minecraft:zombie", 0.1).unwrap().slots[slot as usize].is_none());

    // Closing the screen forgets it: a later click on another organ is nobody's.
    let &(other, _, _) = mob_kind("minecraft:zombie").unwrap().slots.get(1).unwrap();
    cc.closed(ME, WINDOW);
    game.inventory[0] = None;
    cc.clicked(&mut game, ME, WINDOW, other as i32, 1);
    assert_eq!(game.inventory[0], None);
    assert!(mob::load(&mut game, ZOMBIE, "minecraft:zombie", 0.1).unwrap().slots[other as usize].is_some());
}

#[test]
fn a_mob_without_a_body_and_a_game_without_menus_both_tell_the_player() {
    let (cc, mut game) = world();
    cc.ask_mob(me(), ZOMBIE, "minecraft:villager_of_nowhere");
    cc.open_queued(&mut game);
    assert_eq!(game.said, ["Villager Of Nowhere has no chest cavity"]);

    game.server_menus = false;
    cc.ticks.fetch_add(10, Ordering::Relaxed);
    cc.ask_own(me());
    cc.open_queued(&mut game);
    assert_eq!(game.said.last().map(String::as_str), Some("Could not open the screen: use /cc list, /cc take <slot> and /cc put"));
    assert!(game.hurts.is_empty(), "nothing opened, nothing paid");
}

/// One click on a mob raises the entity's hook and then the item's own use (the client sends both): the second
/// must not swap the mob's screen for your own chest, whichever comes first and wherever the tick falls.
#[test]
fn a_click_on_a_mob_is_not_taken_for_your_own_chest() {
    for order in 0..3 {
        let (cc, mut game) = world();
        match order {
            0 => {
                cc.ask_mob(me(), ZOMBIE, "minecraft:zombie");
                cc.ask_own(me());
            }
            1 => {
                cc.ask_own(me());
                cc.ask_mob(me(), ZOMBIE, "minecraft:zombie");
            }
            _ => {
                cc.ask_mob(me(), ZOMBIE, "minecraft:zombie");
                cc.open_queued(&mut game);
                game.menu_title = None;
                cc.ticks.fetch_add(1, Ordering::Relaxed);
                cc.ask_own(me());
            }
        }
        cc.open_queued(&mut game);
        if order == 2 {
            assert_eq!(game.menu_title, None, "the item's use a tick later asks for nothing");
        } else {
            assert_eq!(game.menu_title.as_deref(), Some("Zombie's Chest Cavity"), "order {order}");
        }
        assert!(game.hurts.iter().all(|h| h.1 == ZOMBIE), "only the zombie paid: {:?}", game.hurts);
    }

    // A while later your own chest can be asked for again.
    let (cc, mut game) = world();
    cc.ask_mob(me(), ZOMBIE, "minecraft:zombie");
    cc.ticks.fetch_add(10, Ordering::Relaxed);
    cc.ask_own(me());
    cc.open_queued(&mut game);
    assert_eq!(game.menu_title.as_deref(), Some("Chest Cavity"));
}

#[test]
fn an_organ_tells_what_it_is_good_at_in_its_tooltip_and_other_items_say_nothing() {
    let heart = ChestCavity::tooltip_lines(&crate::screen::stack("chestcavity:heart", 1));
    assert!(heart.iter().any(|l| l.plain().starts_with("Max health: +")), "{:?}", heart.iter().map(Text::plain).collect::<Vec<_>>());
    assert!(heart.iter().any(|l| l.plain().contains("without a heart you bleed")));
    assert!(ChestCavity::tooltip_lines(&crate::screen::stack("minecraft:stick", 1)).is_empty());
    // An organ says whether it fits any body or only the one it came from.
    let said = |lines: Vec<Text>| lines.last().map(Text::plain);
    assert_eq!(said(heart), Some("Safe to use".to_string()));
    let bound = ChestCavity::tooltip_lines(&crate::screen::organ_stack("chestcavity:heart", 1, Some(7)));
    assert_eq!(said(bound), Some("Only fits the body it was taken from".to_string()));
}
