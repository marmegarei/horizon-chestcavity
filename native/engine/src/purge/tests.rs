use super::*;
use crate::fake::Fake;
use chestcavity_shared::cavity::{Cavity, player_kind};
use chestcavity_shared::food::Meal;
use horizon_client::mod_api::NativeEffect;

const ME: u128 = 7;

fn effect(id: &str, duration: i32) -> NativeEffect {
    NativeEffect { id: id.into(), amplifier: 0, duration, ambient: false, particles: true }
}

/// A body without the organs that give `score`.
fn without(score: &str) -> Body {
    let kind = player_kind();
    let mut cavity = Cavity::of(kind);
    for slot in cavity.slots.iter_mut() {
        if slot.as_ref().and_then(|s| chestcavity_shared::cavity::organ(&s.id)).is_some_and(|o| o.scores.iter().any(|s| s.0 == score)) {
            *slot = None;
        }
    }
    Body::of(cavity)
}

/// One tick of the game: what the body does, then every duration runs down.
fn tick(body: &mut Body, game: &mut Fake) {
    body.ticks += 1;
    body.purge(game, ME);
    for e in game.current.iter_mut().filter(|e| e.duration > 0) {
        e.duration -= 1;
    }
}

#[test]
fn a_body_that_filters_less_keeps_a_new_harmful_effect_longer_once() {
    let mut game = Fake::new();
    let mut body = without("detoxification");
    game.current = vec![effect("minecraft:poison", 100), effect("minecraft:speed", 100)];
    tick(&mut body, &mut game);
    assert_eq!(game.current[0].duration, 199, "poison doubled, and a tick gone");
    assert_eq!(game.current[1].duration, 99, "what helps is as it was");
    for _ in 0..5 {
        tick(&mut body, &mut game);
    }
    assert_eq!(game.current[0].duration, 194, "not doubled again: it only runs down");
    // Poisoned again, longer than what is left: a new one.
    game.current[0].duration = 300;
    tick(&mut body, &mut game);
    assert_eq!(game.current[0].duration, 599);
}

#[test]
fn a_normal_body_is_not_looked_at() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    game.current = vec![effect("minecraft:poison", 100)];
    tick(&mut body, &mut game);
    assert_eq!(game.current[0].duration, 99);
    assert!(game.set_effects_calls == 0);
}

#[test]
fn an_iron_gut_drops_the_harm_of_a_meal_and_leaves_other_harm() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    body.scores.insert("iron_gut", 1.0);
    body.last_meal = Some((Meal::of("minecraft:rotten_flesh"), 0));
    game.current = vec![effect("minecraft:hunger", 600)];
    tick(&mut body, &mut game);
    assert!(game.current.is_empty(), "the hunger of the rotten flesh is gone");
    // Long after the meal, harm is harm.
    body.ticks += 100;
    game.current = vec![effect("minecraft:poison", 100)];
    tick(&mut body, &mut game);
    assert_eq!(game.current.len(), 1);
}
