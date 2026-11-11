use super::*;
use crate::fake::Fake;
use chestcavity_shared::cavity::{Cavity, Slot, player_kind};
use chestcavity_shared::organs::ORGANS;

const ME: u128 = 7;

fn player(health: f32) -> Player {
    Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health, position: [0.0, 64.0, 0.0], yaw: 0.0, pitch: 0.0 }
}

/// A normal body with `count` of `organ` in the first free slot.
fn body_with(organ: &str, count: u32) -> Body {
    let mut cavity = Cavity::of(player_kind());
    let i = cavity.slots.iter().position(Option::is_none).expect("a free slot");
    cavity.slots[i] = Some(Slot { id: organ.into(), count });
    let mut body = Body::of(cavity);
    body.food = Some((20, 5.0, 0.0));
    body
}

fn ability(name: &str) -> &'static Ability {
    abilities::picked(name).into_iter().next().expect(name)
}

#[test]
fn a_body_without_the_organ_has_no_ability_and_a_used_one_rests() {
    let mut game = Fake::new();
    let mut body = Body::of(Cavity::of(player_kind()));
    let err = body.activate(&mut game, &player(20.0), ability("pyromancy"), &mut Rng(1)).unwrap_err();
    assert!(err.contains("não tem"), "{err}");

    let mut body = body_with("chestcavity:active_blaze_rod", 3);
    body.activate(&mut game, &player(20.0), ability("pyromancy"), &mut Rng(1)).unwrap();
    let err = body.activate(&mut game, &player(20.0), ability("pyromancy"), &mut Rng(1)).unwrap_err();
    assert!(err.contains("a recuperar"), "{err}");
    body.ticks += 78;
    assert!(body.activate(&mut game, &player(20.0), ability("pyromancy"), &mut Rng(1)).is_ok(), "it is ready again after its rest");
}

#[test]
fn pyromancy_throws_a_fireball_for_each_point_along_where_the_body_looks() {
    let mut game = Fake::new();
    let mut body = body_with("chestcavity:active_blaze_rod", 3);
    body.activate(&mut game, &player(20.0), ability("pyromancy"), &mut Rng(1)).unwrap();
    assert_eq!(game.summons.len(), 3);
    for (kind, at, nbt) in &game.summons {
        assert_eq!(kind, "minecraft:small_fireball");
        assert!(at[2] > 0.5 && at[1] > 65.0, "in front of the eyes, which look south: {at:?}");
        assert!(nbt.contains_key(&"Motion".into()) && nbt.contains_key(&"power".into()));
    }
    assert!(game.statuses.last().is_some_and(|s| s.1.is_some_and(|f| f.2 > 0.0)), "it tires a player");
}

#[test]
fn iron_mends_a_quarter_of_the_health_and_one_ingot_is_used() {
    let mut game = Fake::new();
    let plating = ORGANS.iter().find(|o| o.id == "chestcavity:golem_plating").expect("golem plating");
    let mut body = body_with(plating.id, plating.stack);
    // Nothing to mend with.
    assert!(body.activate(&mut game, &player(10.0), ability("iron_repair"), &mut Rng(1)).unwrap_err().contains("lingote"));
    game.inventory[3] = Some(stack("minecraft:iron_ingot", 5));
    // Already healthy.
    assert!(body.activate(&mut game, &player(20.0), ability("iron_repair"), &mut Rng(1)).unwrap_err().contains("cheia"));
    body.activate(&mut game, &player(10.0), ability("iron_repair"), &mut Rng(1)).unwrap();
    assert_eq!(game.statuses.last().and_then(|s| s.0), Some(15.0));
    let ingots: u32 = game.inventory.iter().flatten().filter(|s| &*s.item == "minecraft:iron_ingot").map(|s| s.count).sum();
    assert_eq!(ingots, 4, "one ingot of five");
}

#[test]
fn a_creeper_appendix_explodes_the_body_and_is_used_up() {
    let mut game = Fake::new();
    let mut body = body_with("chestcavity:creeper_appendix", 1);
    body.activate(&mut game, &player(20.0), ability("creepy"), &mut Rng(1)).unwrap();
    let (kind, _, nbt) = &game.summons[0];
    assert_eq!(kind, "minecraft:creeper");
    assert_eq!(nbt.get(&"Fuse".into()), Some(&Tag::Short(0)));
    assert!(body.cavity.slots.iter().flatten().all(|s| s.id != "chestcavity:creeper_appendix"), "the organ is spent");
}

#[test]
fn grazing_eats_the_grass_under_the_feet_and_nothing_else() {
    let mut game = Fake::new();
    let mut body = body_with("chestcavity:herbivore_rumen", 1);
    body.food = Some((10, 0.0, 0.0));
    assert!(body.activate(&mut game, &player(20.0), ability("grazing"), &mut Rng(1)).unwrap_err().contains("erva"));
    game.blocks.insert([0, 63, 0], "minecraft:grass_block".into());
    body.activate(&mut game, &player(20.0), ability("grazing"), &mut Rng(1)).unwrap();
    assert_eq!(game.commands, ["setblock 0 63 0 minecraft:dirt"]);
    assert_eq!(body.food.map(|f| f.0), Some(11));
}

#[test]
fn a_key_asks_and_the_server_tick_sets_the_ability_off_for_a_player_the_engine_has_seen() {
    let mut game = Fake::new();
    game.blocks.insert([0, 63, 0], "minecraft:grass_block".into());
    let cc = ChestCavity::new(1);
    lock(&cc.bodies).insert(ME, body_with("chestcavity:herbivore_rumen", 1));

    // Nobody has been seen on a tick yet: nothing to act for.
    cc.ask_ability(ME, "utility");
    cc.run_abilities(&mut game);
    assert!(game.commands.is_empty());

    lock(&cc.players).insert(ME, (player(20.0), 3));
    cc.ask_ability(ME, "utility");
    cc.run_abilities(&mut game);
    assert!(game.commands.is_empty(), "a spectator does nothing");

    lock(&cc.players).insert(ME, (player(20.0), 0));
    cc.ask_ability(ME, "utility");
    cc.run_abilities(&mut game);
    assert_eq!(game.commands, ["setblock 0 63 0 minecraft:dirt"], "the key group sets off its abilities");

    // A named ability that can't be used says why; a group stays quiet.
    cc.ask_ability(ME, "pyromancy");
    cc.run_abilities(&mut game);
    assert!(game.said.last().is_some_and(|l| l.contains("não tem")), "{:?}", game.said);
    let said = game.said.len();
    cc.ask_ability(ME, "attack");
    cc.run_abilities(&mut game);
    assert_eq!(game.said.len(), said);
}
