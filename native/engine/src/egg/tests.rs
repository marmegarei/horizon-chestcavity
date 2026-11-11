use super::*;
use crate::fake::Fake;
use horizon_client::mod_api::Tag;

const ME: u128 = 7;
const OVERWORLD: &str = "minecraft:overworld";
const EGG_AT: [i32; 3] = [4, 70, 4];

fn world() -> (ChestCavity, Fake) {
    let mut game = Fake::new();
    game.blocks.insert(EGG_AT, "minecraft:dragon_egg".into());
    (ChestCavity::new(1), game)
}

fn saved(game: &Fake) -> String {
    match game.data.as_ref().and_then(|d| d.get("chestcavity")) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        other => panic!("the body was saved as {other:?}"),
    }
}

#[test]
fn the_opener_on_the_dragon_egg_grows_the_chest_once_and_spends_the_egg() {
    let (cc, mut game) = world();
    assert!(!cc.egg_used(&mut game, ME, OVERWORLD, [0, 64, 0]), "no egg there: the click is not ours");
    assert!(game.said.is_empty());

    assert!(cc.egg_used(&mut game, ME, OVERWORLD, EGG_AT));
    assert_eq!(cc.with(&mut game, ME, |body, _| body.cavity.slots.len()), 36);
    assert_eq!(game.said.last().map(String::as_str), Some("Your chest cavity grew a fourth row"));
    assert!(saved(&game).contains("size 36"), "{}", saved(&game));
    // The egg stays until the server tick, where a mod may change blocks.
    assert_eq!(game.block(EGG_AT), "minecraft:dragon_egg");
    cc.spend_eggs(&mut game);
    assert_eq!(game.set_blocks, [(OVERWORLD.to_string(), EGG_AT, 0)]);
    assert_eq!(game.block(EGG_AT), "minecraft:air");
    cc.spend_eggs(&mut game);
    assert_eq!(game.set_blocks.len(), 1, "an egg is spent once");

    // Another egg, with the row grown already: the player is told and the egg is left alone.
    game.blocks.insert(EGG_AT, "minecraft:dragon_egg".into());
    assert!(cc.egg_used(&mut game, ME, OVERWORLD, EGG_AT), "the click is still taken, or the egg would teleport");
    assert_eq!(game.said.last().map(String::as_str), Some("Your chest cavity already has its fourth row"));
    cc.spend_eggs(&mut game);
    assert_eq!(game.set_blocks.len(), 1);
    assert_eq!(game.block(EGG_AT), "minecraft:dragon_egg");
}
