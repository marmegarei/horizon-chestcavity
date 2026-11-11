use super::*;
use crate::fake::Fake;
use crate::screen::stack;
use chestcavity_shared::cavity::{Cavity, Slot, player_kind};
use chestcavity_shared::organs::ORGANS;

const ME: u128 = 7;

fn me() -> Player {
    Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health: 20.0, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 }
}

/// A world where `ME`'s body has a full stack of the first organ that gives `score`.
fn with_organ(score: &str) -> ChestCavity {
    let organ = ORGANS.iter().find(|o| o.scores.iter().any(|s| s.0 == score)).expect(score);
    let mut cavity = Cavity::of(player_kind());
    let i = cavity.slots.iter().position(Option::is_none).expect("a free slot");
    cavity.slots[i] = Some(Slot { id: organ.id.into(), count: organ.stack });
    let cc = ChestCavity::new(1);
    lock(&cc.bodies).insert(ME, Body::of(cavity));
    cc
}

#[test]
fn blows_that_launch_are_noted_and_venom_needs_an_empty_hand_and_rests() {
    let mut game = Fake::new();
    let cc = with_organ("launching");
    cc.attacked(&mut game, &me(), 9);
    assert_eq!(lock(&cc.hits).len(), 1);
    assert!(lock(&cc.hits)[0].launch > 0.0 && !lock(&cc.hits)[0].venom);

    let cc = with_organ("venomous");
    game.inventory[0] = Some(stack("minecraft:stick", 1));
    cc.attacked(&mut game, &me(), 9);
    assert!(lock(&cc.hits).is_empty(), "a hand with something in it is no bite");
    game.inventory[0] = None;
    cc.attacked(&mut game, &me(), 9);
    assert!(lock(&cc.hits)[0].venom);
    cc.attacked(&mut game, &me(), 9);
    assert_eq!(lock(&cc.hits).len(), 1, "the venom rests for 40 ticks");
    lock(&cc.bodies).get_mut(&ME).expect("a body").ticks += 40;
    cc.attacked(&mut game, &me(), 9);
    assert_eq!(lock(&cc.hits).len(), 2);

    let plain = ChestCavity::new(1);
    plain.attacked(&mut game, &me(), 9);
    assert!(lock(&plain.hits).is_empty(), "an ordinary body adds nothing to a blow");
}

#[test]
fn an_arrow_is_dodged_once_and_the_body_rests() {
    let cc = with_organ("arrow_dodging");
    assert_eq!(cc.incoming(ME, "minecraft:fall", 4.0), Some(4.0), "a fall is not dodged");
    assert_eq!(cc.incoming(ME, "minecraft:arrow", 4.0), None);
    assert_eq!(lock(&cc.dodges).len(), 1, "the body is sent away");
    assert_eq!(cc.incoming(ME, "minecraft:arrow", 4.0), Some(4.0), "it rests after a dodge");
    lock(&cc.bodies).get_mut(&ME).expect("a body").ticks += 1000;
    assert_eq!(cc.incoming(ME, "minecraft:arrow", 4.0), None);
    assert_eq!(cc.incoming(99, "minecraft:arrow", 4.0), Some(4.0), "a body the engine has not met takes the hit as it comes");
}

#[test]
fn wings_let_a_survival_player_fly_and_losing_them_lands_them() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    body.fly(&mut game, ME, 0);
    assert!(game.flights.is_empty(), "a normal body asks nothing of the game");

    let organ = ORGANS.iter().find(|o| o.scores.iter().any(|s| s.0 == "flight")).expect("a flying organ");
    let i = body.cavity.slots.iter().position(Option::is_none).expect("a free slot");
    body.cavity.slots[i] = Some(Slot { id: organ.id.into(), count: organ.stack });
    body.changed(&mut game, ME);
    body.fly(&mut game, ME, 1);
    assert!(game.flights.is_empty(), "creative players fly anyway");
    body.fly(&mut game, ME, 0);
    body.fly(&mut game, ME, 0);
    assert_eq!(game.flights, [true], "asked once, not every tick");
    // A game mode change takes the allowance back: it is given again.
    body.fly(&mut game, ME, 2);
    assert_eq!(game.flights, [true, true]);
    body.cavity.slots[i] = None;
    body.changed(&mut game, ME);
    body.fly(&mut game, ME, 2);
    assert_eq!(game.flights, [true, true, false]);
}

#[test]
fn a_uuid_reads_as_a_command_wants_it() {
    assert_eq!(uuid_text(0x123e4567_e89b_12d3_a456_426614174000), "123e4567-e89b-12d3-a456-426614174000");
}
