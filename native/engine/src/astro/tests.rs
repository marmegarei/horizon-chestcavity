use super::*;
use crate::fake::Fake;
use chestcavity_shared::cavity::{Cavity, Slot};

const ME: u128 = 7;

fn who(health: f32) -> Player {
    Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 }
}

/// A body whose first free slot holds `organ`, standing at `ticks`.
fn with(organ: &str, ticks: u64) -> Body {
    let mut cavity = Cavity::of(player_kind());
    let free = cavity.slots.iter().position(Option::is_none).expect("a free slot");
    cavity.slots[free] = Some(Slot::new(organ, 1));
    let mut body = Body::of(cavity);
    body.uuid = ME;
    body.food = Some((20, 5.0, 0.0));
    body.ticks = ticks;
    body
}

fn first_food(body: &Body) -> usize {
    body.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| chestcavity_shared::astro::food_of(&s.id).is_some())).expect("some food")
}

#[test]
fn regrowth_fills_what_is_missing_every_400_ticks_and_costs_food() {
    let mut game = Fake::new();
    let mut body = with("chestcavity:stem_cell_canister", 399);
    let heart = body.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:heart")).expect("a heart");
    body.cavity.slots[heart] = None;
    body.rescore();
    body.astro(&mut game, &who(20.0), &mut Rng(1));
    assert!(body.cavity.slots[heart].is_none(), "not yet: it is every 400 ticks");
    body.ticks = 400;
    body.astro(&mut game, &who(20.0), &mut Rng(1));
    assert!(body.cavity.slots[heart].is_some(), "an organ or a tumor grew back");
    assert_eq!(body.cavity.slots[heart].as_ref().map(|s| s.owner), Some(None), "a body tags none of what grows");
    let (_, food) = game.statuses.last().cloned().expect("it told the game");
    assert!(food.is_some_and(|f| f.2 >= 0.25), "growing is tiring: {food:?}");
    assert!(game.data.is_some(), "and the cavity was saved");
}

#[test]
fn autophagy_feeds_a_hungry_body_with_its_food_organs_and_mends_a_full_one() {
    let mut game = Fake::new();
    let mut body = with("chestcavity:autophagy_stomach", 449);
    let at = first_food(&body);
    let count = body.cavity.slots[at].as_ref().expect("food").count;
    body.food = Some((10, 0.0, 0.0));
    body.astro(&mut game, &who(10.0), &mut Rng(1));
    assert_eq!(body.cavity.slots[at].as_ref().map(|s| s.count), Some(count), "not yet: it is every 450 ticks");

    body.ticks = 450;
    body.astro(&mut game, &who(10.0), &mut Rng(1));
    assert_eq!(body.cavity.slots[at].as_ref().map(|s| s.count), Some(count - 1), "a unit eaten");
    assert!(body.food.is_some_and(|f| f.0 > 10), "and it fed the body: {:?}", body.food);

    // A body that is full and hurt mends a point of health for it instead.
    body.food = Some((20, 20.0, 0.0));
    body.ticks = 900;
    let before = game.statuses.len();
    body.astro(&mut game, &who(10.0), &mut Rng(1));
    assert_eq!(body.cavity.slots[at].as_ref().map(|s| s.count), Some(count - 2));
    assert!(game.statuses[before..].iter().any(|(health, _)| *health == Some(11.0)), "{:?}", game.statuses);
    // Full and unhurt: nothing to do, nothing eaten.
    body.ticks = 1350;
    body.astro(&mut game, &who(20.0), &mut Rng(1));
    assert_eq!(body.cavity.slots[at].as_ref().map(|s| s.count), Some(count - 2));
}

#[test]
fn hunters_and_infections_work_at_their_own_pace() {
    let mut game = Fake::new();
    let mut hunter = with("chestcavity:nanobot_canister", 249);
    let tumor = hunter.cavity.slots.iter().position(|s| s.as_ref().is_some_and(|s| s.id == "chestcavity:rib")).expect("a rib");
    hunter.cavity.slots[tumor] = Some(Slot::new("chestcavity:autophagy_tumor", 1));
    hunter.rescore();
    hunter.astro(&mut game, &who(20.0), &mut Rng(1));
    assert!(hunter.cavity.slots[tumor].is_some());
    hunter.ticks = 250;
    hunter.astro(&mut game, &who(20.0), &mut Rng(1));
    assert!(hunter.cavity.slots[tumor].is_none(), "the hunters cleared it");

    let mut sick = with("chestcavity:sculk_canister", 499);
    let food = first_food(&sick);
    sick.astro(&mut game, &who(20.0), &mut Rng(1));
    assert_ne!(sick.cavity.slots[food].as_ref().map(|s| s.id.as_str()), Some("chestcavity:sculk_tumor"));
    sick.ticks = 500;
    sick.astro(&mut game, &who(20.0), &mut Rng(1));
    assert_eq!(sick.cavity.slots[food].as_ref().map(|s| s.id.as_str()), Some("chestcavity:sculk_tumor"), "the infection took a food organ");

    // A body with none of these organs is left alone at every pace.
    let mut plain = Body::of(Cavity::of(player_kind()));
    plain.food = Some((20, 5.0, 0.0));
    let before = plain.cavity.clone();
    for t in [250, 400, 450, 500, 1800] {
        plain.ticks = t;
        plain.astro(&mut game, &who(20.0), &mut Rng(1));
    }
    assert_eq!(plain.cavity, before);
}
