//! The engine as the game drives it: the window's own player has no server menu, so its screen is the
//! mod's own, and a click on it is queued and carried out in the player's next tick.
use super::*;
use crate::fake::Fake;
use crate::local::{Action, Target};
use crate::screen::stack;
use chestcavity_shared::cavity::{Cavity, mob_kind, player_kind};
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;

/// The window's own player, with a zombie nearby.
fn single_player() -> (ChestCavity, Fake) {
    let mut game = Fake::new();
    game.server_menus = false;
    game.mobs.insert(ZOMBIE, Attachments::default());
    (ChestCavity::new(1), game)
}

fn click(cc: &ChestCavity, action: Action) {
    lock(&cc.local).as_mut().expect("a screen is open").pending.push(action);
}

fn heart_slot(cavity: &Cavity) -> usize {
    cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart")
}

#[test]
fn the_window_players_own_cavity_opens_as_the_mods_screen_and_clicks_are_carried_out_in_their_tick() {
    let (cc, mut game) = single_player();
    assert!(cc.open_own(&mut game, ME));
    assert_eq!(game.screens, [local::MOD_ID]);
    let heart = {
        let guard = lock(&cc.local);
        let shown = guard.as_ref().unwrap();
        assert_eq!(shown.target, Target::Own);
        assert_eq!(shown.cavity, Cavity::of(player_kind()));
        heart_slot(&shown.cavity)
    };

    // A click on the heart: nothing happens until the player's tick.
    click(&cc, Action::Take(heart));
    assert_eq!(game.inventory[0], None);
    cc.drain_local(&mut game, ME);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some("chestcavity:heart"));
    assert_eq!(game.base("minecraft:max_health"), 16.0);
    assert!(lock(&cc.local).as_ref().unwrap().cavity.slots[heart].is_none(), "the screen shows it gone");

    // The Put button with the heart in the hand puts it back.
    click(&cc, Action::PutHeld);
    cc.drain_local(&mut game, ME);
    assert_eq!(game.inventory[0], None);
    assert_eq!(game.base("minecraft:max_health"), 20.0);
    let guard = lock(&cc.local);
    assert_eq!(guard.as_ref().unwrap().note.as_deref(), Some("Put chestcavity:heart in"));
    assert_eq!(guard.as_ref().unwrap().cavity, Cavity::of(player_kind()));
}

#[test]
fn a_mobs_cavity_opens_the_same_way_and_a_mob_without_a_body_says_so() {
    let (cc, mut game) = single_player();
    cc.open_mob(&mut game, ME, ZOMBIE, "minecraft:villager_of_nowhere");
    assert_eq!(game.said, ["Villager Of Nowhere has no chest cavity"]);
    assert!(game.screens.is_empty());

    cc.open_mob(&mut game, ME, ZOMBIE, "minecraft:zombie");
    assert_eq!(game.screens, [local::MOD_ID]);
    let &(slot, id, _) = mob_kind("minecraft:zombie").unwrap().slots.first().unwrap();
    {
        let guard = lock(&cc.local);
        let shown = guard.as_ref().unwrap();
        assert_eq!(shown.title, "Zombie's Chest Cavity");
        assert_eq!(shown.target, Target::Mob { entity: ZOMBIE, mob: "minecraft:zombie".into() });
    }
    click(&cc, Action::Take(slot as usize));
    cc.drain_local(&mut game, ME);
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some(id));
    assert!(lock(&cc.local).as_ref().unwrap().cavity.slots[slot as usize].is_none());
    assert!(mob::load(&mut game, ZOMBIE, "minecraft:zombie").unwrap().slots[slot as usize].is_none(), "the zombie keeps it that way");

    // An organ of yours goes in with the button, and a click that arrives for another player's screen is ignored.
    game.inventory[1] = Some(stack("chestcavity:heart", 1));
    game.inventory.swap(0, 1);
    click(&cc, Action::PutHeld);
    cc.drain_local(&mut game, ME + 1);
    assert!(game.inventory[0].is_some(), "not carried out for another player");
    cc.drain_local(&mut game, ME);
    assert_eq!(lock(&cc.local).as_ref().unwrap().note.as_deref(), Some("Put chestcavity:heart in"));
}
