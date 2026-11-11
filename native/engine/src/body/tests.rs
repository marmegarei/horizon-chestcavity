use super::*;
use crate::fake::Fake;
use crate::screen::{EMPTY_SLOT, inventory_slot, stack};
use chestcavity_shared::cavity::SLOTS;
use chestcavity_shared::config::Config;
use chestcavity_shared::drops::Rng;
use chestcavity_shared::effects::Env;

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
    assert_eq!(body.cavity, Cavity::of(player_kind()).born(ME, || false));
}

#[test]
fn only_organs_go_in_and_a_full_inventory_keeps_them_out() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    game.inventory[2] = Some(stack("chestcavity:chest_opener", 1));
    let msg = body.click(&mut game, ME, (SLOTS + 27 + 2) as i32);
    assert!(msg.is_some_and(|m| m.plain().contains("is not an organ")));
    assert!(game.inventory[2].is_some());

    game.inventory = vec![Some(stack("minecraft:stone", 1)); 36];
    let i = heart_slot(&body);
    assert_eq!(body.click(&mut game, ME, i as i32).map(|m| m.plain()).as_deref(), Some("No room in the inventory"));
    assert!(body.cavity.slots[i].is_some(), "the heart stays when it has nowhere to go");
}

#[test]
fn no_heart_bleeds_and_kidneys_poison() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    let i = heart_slot(&body);
    body.take_out(&mut game, ME, i);
    let who = Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health: 20.0, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 };
    body.ticks = 20;
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &Config::default());
    body.ticks = 40;
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &Config::default());
    let hurt = |amount| ("minecraft:overworld".to_string(), 1, amount, "minecraft:generic".to_string(), None);
    assert_eq!(game.hurts, [hurt(1.0), hurt(2.0)]);
    assert!(game.effects.is_empty());
}

#[test]
fn an_organ_remembers_the_body_it_was_born_in_and_another_body_rejects_it() {
    let mut game = Fake::new();
    let mut mine = Body::load(&mut game, ME);
    let i = heart_slot(&mine);
    assert_eq!(mine.cavity.slots[i].as_ref().and_then(|s| s.owner), Some(ME), "the heart was born in this body");
    assert_eq!(mine.incompat, 0);

    // Out of the body it carries its owner, and back in it still fits.
    mine.take_out(&mut game, ME, i);
    let heart = game.inventory[0].clone().expect("the heart");
    assert_eq!(crate::screen::owner_of(&heart), Some(ME));
    mine.click(&mut game, ME, (SLOTS + 27) as i32);
    assert_eq!(mine.incompat, 0);

    // In another body it is a stranger.
    mine.take_out(&mut game, ME, i);
    let mut other = Body::of(Cavity::of(player_kind()).born(8, || false));
    other.uuid = 8;
    other.take_out(&mut game, 8, i);
    assert_eq!(other.incompat, 0);
    other.click(&mut game, 8, (SLOTS + 27 + 1) as i32);
    assert!(game.inventory.iter().flatten().any(|s| &*s.item == "chestcavity:heart"));
}

#[test]
fn a_body_rejects_the_organs_of_others_at_the_pace_the_settings_say() {
    let mut game = Fake::new();
    let mut cavity = Cavity::of(player_kind()).born(ME, || false);
    let i = cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart");
    cavity.slots[i].as_mut().expect("a heart").owner = Some(99);
    let mut body = Body::of(cavity);
    body.uuid = ME;
    body.changed(&mut game, ME);
    assert_eq!(body.incompat, 1);
    let who = Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health: 20.0, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 };
    let cfg = Config { rejection_rate: 100.0, ..Config::default() };
    let hits = |game: &Fake| game.hurts.iter().filter(|h| h.2 == 2.0).count();
    for _ in 0..4 {
        body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    }
    assert_eq!(hits(&game), 0, "100 ticks are 5 beats");
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    assert_eq!(hits(&game), 1);
    // The settings can turn it off.
    for _ in 0..20 {
        body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &Config { organ_rejection: false, ..cfg.clone() });
    }
    assert_eq!(hits(&game), 1);
    // Each wait shows an effect as long as it is, and it is not shown again while the wait goes on.
    assert_eq!(game.effects.len(), 1);
    assert_eq!((&*game.effects[0].id, game.effects[0].duration), ("chestcavity:organ_rejection", 100));
}

#[test]
fn the_rejection_effect_goes_when_the_organ_that_caused_it_does() {
    let mut game = Fake::new();
    let mut cavity = Cavity::of(player_kind()).born(ME, || false);
    let i = cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart");
    cavity.slots[i].as_mut().expect("a heart").owner = Some(99);
    let mut body = Body::of(cavity);
    body.uuid = ME;
    body.changed(&mut game, ME);
    let who = Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health: 20.0, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 };
    let cfg = Config::default();
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    assert_eq!((game.effects.len(), game.removed.len()), (1, 0));
    assert_eq!(game.effects[0].duration, 600, "a point of incompatibility: the 600 ticks of the settings");
    body.cavity.slots[i] = Some(Slot::new("chestcavity:heart", 1).owned_by(Some(ME)));
    body.changed(&mut game, ME);
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    assert_eq!(game.removed, ["chestcavity:organ_rejection"]);
    body.beat(&mut game, &who, &Env::default(), &mut Rng(1), &cfg);
    assert_eq!(game.removed.len(), 1, "it is taken off once");
}
