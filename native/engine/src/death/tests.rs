use super::*;
use crate::body::Body;
use crate::fake::Fake;
use crate::screen::{Click, owner_of};
use chestcavity_shared::cavity::{Cavity, Slot, player_kind};
use chestcavity_shared::config::Config;
use horizon_client::mod_api::Attachments;

const ME: u128 = 7;
const ZOMBIE: u32 = 9;
const WITHER: u32 = 10;

fn world() -> (ChestCavity, Fake) {
    let mut game = Fake::new();
    game.mobs.insert(ZOMBIE, Attachments::default());
    game.mobs.insert(WITHER, Attachments::default());
    (ChestCavity::new(1), game)
}

fn keeping(cc: &ChestCavity) {
    *lock(&cc.config) = Config { keep_chest_cavity: true, ..Config::default() };
}

/// The player's liver slot, and a liver that is another body's put there.
fn with_a_strangers_liver(cc: &ChestCavity, game: &mut Fake) -> usize {
    cc.with(game, ME, |body, host| {
        let liver = body.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:liver")).expect("a liver");
        body.cavity.slots[liver] = Some(Slot::new("chestcavity:liver", 1).owned_by(Some(99)));
        body.changed(host, ME);
        liver
    })
}

#[test]
fn a_player_that_dies_drops_the_organs_that_are_not_theirs_and_only_those() {
    let (cc, mut game) = world();
    let mut drops = Vec::new();
    cc.player_dropped(&mut game, ME, &mut drops);
    assert!(drops.is_empty(), "a body with only its own organs drops none");

    let liver = with_a_strangers_liver(&cc, &mut game);
    cc.player_dropped(&mut game, ME, &mut drops);
    assert_eq!(drops.len(), 1);
    assert_eq!(&*drops[0].item, "chestcavity:liver");
    assert_eq!(owner_of(&drops[0]), Some(99), "the organ is still the other body's");
    assert!(cc.with(&mut game, ME, |body, _| body.cavity.slots[liver].is_none()), "and it left the cavity, so it is not dropped twice");
    // What is saved says so too, for a player that leaves at the death screen.
    let saved = Body::load(&mut game, ME);
    assert!(saved.cavity.slots[liver].is_none());
}

#[test]
fn a_body_that_keeps_its_cavity_keeps_it_and_gets_the_rotten_organs_it_lacks() {
    let (cc, mut game) = world();
    keeping(&cc);
    cc.with(&mut game, ME, |body, host| {
        let heart = body.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart");
        body.cavity.slots[heart] = None;
        body.changed(host, ME);
    });
    let liver = with_a_strangers_liver(&cc, &mut game);
    let mut drops = Vec::new();
    cc.player_dropped(&mut game, ME, &mut drops);
    assert!(drops.is_empty(), "nothing leaves a body that keeps its cavity");

    cc.with(&mut game, ME, |body, host| body.respawn(host, ME, true, true));
    let (rotten, still) = cc.with(&mut game, ME, |body, _| {
        (body.cavity.slots.iter().flatten().any(|s| s.id == "chestcavity:rotten_heart"), body.cavity.slots[liver].is_some())
    });
    assert!(rotten, "a body with no heart is given a rotten one");
    assert!(still, "and keeps the rest, the stranger's liver too");
}

#[test]
fn what_the_game_kept_of_a_dying_player_goes_back_to_them_and_the_body_starts_over() {
    // The game does not tell the loot hook of a player that keeps their inventory: the organs that were not theirs are
    // handed back at the respawn rather than lost.
    let (cc, mut game) = world();
    let liver = with_a_strangers_liver(&cc, &mut game);
    cc.with(&mut game, ME, |body, host| body.respawn(host, ME, false, true));
    assert_eq!(game.inventory[0].as_ref().map(|s| &*s.item), Some("chestcavity:liver"));
    assert_eq!(game.inventory[0].as_ref().and_then(owner_of), Some(99));
    let fresh = cc.with(&mut game, ME, |body, _| body.cavity.clone());
    assert_eq!(fresh, Cavity::of(player_kind()).born(ME, || false));
    assert!(fresh.slots[liver].as_ref().is_some_and(|s| s.owner == Some(ME)), "a body of their own again");
}

/// The deaths, out of `n`, of unopened zombies that dropped something, killed with `weapon` in hand.
fn drops_with(cc: &ChestCavity, game: &mut Fake, weapon: Option<ItemStack>, n: usize) -> (usize, Vec<ItemStack>) {
    game.inventory[0] = weapon;
    let mut all = Vec::new();
    let mut deaths = 0;
    for _ in 0..n {
        let mut drops = Vec::new();
        cc.mob_dropped(game, ZOMBIE, "minecraft:zombie", Some(ME), true, &mut drops);
        deaths += usize::from(!drops.is_empty());
        all.extend(drops);
    }
    (deaths, all)
}

#[test]
fn an_unopened_mob_drops_organs_by_the_killers_luck_and_a_cleaver_takes_them_apart() {
    let (cc, mut game) = world();
    let (plain, organs) = drops_with(&cc, &mut game, None, 4000);
    assert!(plain > 0 && organs.iter().all(|s| owner_of(s).is_none()), "organs from loot fit any body");

    let mut sword = stack("minecraft:diamond_sword", 1);
    sword.enchant("minecraft:looting", 3);
    let (looted, _) = drops_with(&cc, &mut game, Some(sword), 4000);
    assert!(looted > plain, "Looting III: {looted} against {plain}");

    // A cleaver with Looting III has ten times the luck and turns the organs into what they are made of.
    let mut cleaver = stack("chestcavity:iron_cleaver", 1);
    cleaver.enchant("minecraft:looting", 3);
    let (cleaved, pieces) = drops_with(&cc, &mut game, Some(cleaver), 4000);
    assert!(cleaved > looted * 2, "the cleaver's luck: {cleaved} against {looted}");
    assert!(pieces.iter().any(|s| s.item.contains("raw_") || &*s.item == "minecraft:bone_meal" || &*s.item == "minecraft:rotten_flesh"), "{:?}", pieces.iter().map(|s| s.item.to_string()).collect::<Vec<_>>());
    for gone in ["chestcavity:heart", "chestcavity:rib", "chestcavity:muscle", "chestcavity:liver"] {
        assert!(pieces.iter().all(|s| &*s.item != gone), "{gone} is taken apart by a cleaver");
    }

    // No loot table, no organs.
    let mut none = Vec::new();
    cc.mob_dropped(&mut game, ZOMBIE, "minecraft:zombie", Some(ME), false, &mut none);
    assert!(none.is_empty());
}

#[test]
fn a_mob_that_was_opened_gives_up_what_is_not_its_own_and_its_loot_table_adds_no_organs() {
    let (cc, mut game) = world();
    // A player put another body's liver in the zombie, in place of its first organ.
    crate::mob::edit(&mut game, ZOMBIE, "minecraft:zombie", 0.0, |_, cavity| {
        let first = cavity.slots.iter().position(Option::is_some).expect("an organ");
        cavity.slots[first] = Some(Slot::new("chestcavity:liver", 1).owned_by(Some(99)));
        Click { changed: true, msg: None }
    })
    .expect("a zombie");
    for _ in 0..500 {
        let mut drops = Vec::new();
        cc.mob_dropped(&mut game, ZOMBIE, "minecraft:zombie", Some(ME), true, &mut drops);
        assert_eq!(drops.len(), 1, "{:?}", drops.iter().map(|s| s.item.to_string()).collect::<Vec<_>>());
        assert_eq!((&*drops[0].item, owner_of(&drops[0])), ("chestcavity:liver", Some(99)));
    }
}

#[test]
fn a_wither_whose_star_was_taken_out_drops_none() {
    let (cc, mut game) = world();
    let star = || vec![stack("minecraft:nether_star", 1)];
    // Not opened: the star is as the game dropped it.
    let mut drops = star();
    cc.mob_dropped(&mut game, WITHER, "minecraft:wither", Some(ME), true, &mut drops);
    assert!(drops.iter().any(|d| &*d.item == "minecraft:nether_star"));
    // Opened and emptied: nothing keeps it alive, and nothing of it is a star.
    crate::mob::edit(&mut game, WITHER, "minecraft:wither", 0.0, |_, cavity| {
        cavity.slots.iter_mut().for_each(|s| *s = None);
        Click { changed: true, msg: None }
    })
    .expect("a wither");
    let mut drops = star();
    cc.mob_dropped(&mut game, WITHER, "minecraft:wither", Some(ME), true, &mut drops);
    assert!(drops.iter().all(|d| &*d.item != "minecraft:nether_star"), "{:?}", drops.iter().map(|s| s.item.to_string()).collect::<Vec<_>>());
}

#[test]
fn a_grown_chest_keeps_its_fourth_row_through_death_unless_the_settings_say_otherwise() {
    let (cc, mut game) = world();
    cc.with(&mut game, ME, |body, host| {
        body.grow(host, ME);
        body.cavity.slots[30] = Some(Slot::new("chestcavity:silk_gland", 1).owned_by(Some(ME)));
    });
    cc.with(&mut game, ME, |body, host| body.respawn(host, ME, false, true));
    let (len, organ) = cc.with(&mut game, ME, |body, _| (body.cavity.slots.len(), body.cavity.slots[30].clone()));
    assert_eq!(len, 36, "the row is an upgrade of the player's");
    assert!(organ.is_none(), "the organs that were in it are lost like the rest of the body's own");
    cc.with(&mut game, ME, |body, host| body.respawn(host, ME, false, false));
    assert_eq!(cc.with(&mut game, ME, |body, _| body.cavity.slots.len()), 27, "unless the settings take it back");
}
