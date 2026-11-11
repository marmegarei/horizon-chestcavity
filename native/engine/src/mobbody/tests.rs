use super::*;

const CREEPER: &str = "minecraft:creeper";

/// The creeper's cavity without the first organ that, by being gone, takes `score` down to nothing.
fn without_giving(score: &str) -> Option<Cavity> {
    let kind = mob_kind(CREEPER).expect("creeper");
    let whole = Cavity::of(kind);
    (0..whole.slots.len()).filter(|&i| whole.slots[i].is_some()).find_map(|i| {
        let mut cavity = whole.clone();
        cavity.slots[i] = None;
        (get(&cavity.scores(kind), score) <= 0.0 && get(&whole.scores(kind), score) > 0.0).then_some(cavity)
    })
}

fn hurts(acts: &[Act]) -> Vec<f32> {
    acts.iter().filter_map(|a| if let Act::Hurt { amount, .. } = a { Some(*amount) } else { None }).collect()
}

#[test]
fn a_mob_as_it_was_born_is_left_alone() {
    let kind = mob_kind(CREEPER).expect("creeper");
    let mut body = MobBody::new("minecraft:overworld", CREEPER, &Cavity::of(kind)).expect("a creeper has a body");
    // The first beat clears whatever modifiers there were; then there is nothing to do.
    assert_eq!(body.beat(1), [Act::Modifiers { remove: vec![], add: vec![] }]);
    assert!(body.is_idle());
    assert!(body.beat(2).is_empty());
    assert!(MobBody::new("minecraft:overworld", "minecraft:villager_of_nowhere", &Cavity::of(kind)).is_none());
}

#[test]
fn a_mob_without_a_heart_bleeds_more_each_second_until_it_dies() {
    // A creeper's heart is its trunk.
    let kind = mob_kind(CREEPER).expect("creeper");
    let mut body = MobBody::new("minecraft:overworld", CREEPER, &without_giving("health").expect("a creeper lives by an organ")).expect("body");
    body.beat(1);
    assert_eq!(hurts(&body.beat(2)), [2.0]);
    assert_eq!(hurts(&body.beat(3)), [3.0]);
    // Put it back and the bleeding stops and the modifiers come off.
    body.set(CREEPER, &Cavity::of(kind));
    let acts = body.beat(4);
    assert!(hurts(&acts).is_empty(), "{acts:?}");
    assert!(matches!(&acts[0], Act::Modifiers { remove, add } if !remove.is_empty() && add.is_empty()), "{acts:?}");
    assert!(body.beat(5).is_empty() && body.is_idle());
}

#[test]
fn a_mob_without_a_spine_cannot_move() {
    let mut body = MobBody::new("minecraft:overworld", CREEPER, &without_giving("nerves").expect("a creeper moves by an organ")).expect("body");
    let acts = body.beat(1);
    assert!(
        acts.iter().any(|a| matches!(a, Act::Modifiers { add, .. } if add.contains(&("minecraft:movement_speed", -1.0, "add_multiplied_total")))),
        "no spine, no moving: {acts:?}"
    );
}

#[test]
fn a_mob_without_lungs_drowns_on_land() {
    // The creeper has no lungs at all, so a mob that has them tells it: a zombie.
    let kind = mob_kind("minecraft:zombie").expect("zombie");
    let mut cavity = Cavity::of(kind);
    for slot in cavity.slots.iter_mut() {
        if slot.as_ref().is_some_and(|s| chestcavity_shared::cavity::organ(&s.id).is_some_and(|o| o.scores.iter().any(|s| s.0 == "breath_recovery" || s.0 == "water_breath"))) {
            *slot = None;
        }
    }
    let mut body = MobBody::new("minecraft:overworld", "minecraft:zombie", &cavity).expect("body");
    assert!(body.beat(1).contains(&Act::Hurt { amount: 2.0, kind: "minecraft:drown" }));
}

#[test]
fn modifier_ids_are_named_after_their_attribute() {
    assert_eq!(modifier_id("minecraft:max_health"), "chestcavity:cavity_max_health");
}

#[test]
fn a_creeper_without_its_appendix_cannot_explode_and_one_with_it_can_again() {
    let kind = mob_kind(CREEPER).expect("creeper");
    let mut body = MobBody::new("minecraft:overworld", CREEPER, &without_giving("creepy").expect("a creeper explodes by an organ")).expect("body");
    let fuse = |acts: &[Act]| acts.iter().find_map(|a| if let Act::Nbt(n) = a { Some(n.get(&"Fuse".into()).cloned()) } else { None });
    assert_eq!(fuse(&body.beat(1)), Some(Some(Tag::Short(i16::MAX))));
    assert!(!body.is_idle());
    body.set(CREEPER, &Cavity::of(kind));
    assert_eq!(fuse(&body.beat(2)), Some(Some(Tag::Short(30))));
    assert!(body.beat(3).is_empty() && body.is_idle());
}
