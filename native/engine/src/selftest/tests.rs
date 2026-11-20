use super::*;
use crate::ChestCavity;
use crate::fake::Fake;
use chestcavity_shared::cavity::Cavity;

const ME: u128 = 7;

fn world() -> (ChestCavity, Fake) {
    let mut game = Fake::new();
    game.effects_listed = true;
    (ChestCavity::new(1), game)
}

fn normal() -> Cavity {
    Cavity::of(player_kind()).born(ME, || false)
}

/// Ticks the player until the selftest is over; how many ticks it took.
fn run_out(cc: &ChestCavity, game: &mut Fake) -> usize {
    for n in 0..5000 {
        if !cc.selftest_tick(game, ME) {
            return n;
        }
    }
    panic!("the selftest never ended");
}

fn body_of(cc: &ChestCavity, game: &mut Fake) -> Cavity {
    cc.with(game, ME, |body, _| body.cavity.clone())
}

#[test]
fn the_selftest_runs_every_body_with_no_fail_and_gives_the_body_back() {
    let (cc, mut game) = world();
    cc.selftest_command(&mut game, ME, false, 20.0);
    assert_eq!(game.said.last().map(String::as_str), Some("Selftest started: your chest cavity comes back when it ends"));
    run_out(&cc, &mut game);

    let checks: usize = cases(ME).iter().map(|c| expected(&c.cavity).len() + 1).sum::<usize>() + 3;
    // The double has no config folder: the settings check is an INFO.
    assert_eq!(game.said.last().unwrap(), &format!("Selftest: {checks} PASS, 0 FAIL, 1 INFO (details in chestcavity-trace.log)"));
    assert_eq!(body_of(&cc, &mut game), normal());
    assert!(saved(&game, ME, SNAPSHOT).is_none(), "nothing left to put back");
    assert!(game.removed.iter().any(|r| r == REJECTION), "the rejection effect went with the test");
    assert_eq!(game.statuses.last().map(|s| s.0), Some(Some(20.0)), "the health the heart test took is back");
}

#[test]
fn an_altered_body_is_refused_unless_forced_and_comes_back_as_it_was() {
    let (cc, mut game) = world();
    let mut altered = normal();
    altered.slots[4] = None; // the heart
    cc.with(&mut game, ME, |body, host| {
        body.cavity = altered.clone();
        body.changed(host, ME);
    });
    cc.selftest_command(&mut game, ME, false, 20.0);
    assert!(game.said.last().unwrap().contains("not the normal one"));
    assert!(!crate::lock(&cc.selftests).contains_key(&ME));

    cc.selftest_command(&mut game, ME, true, 20.0);
    run_out(&cc, &mut game);
    assert!(game.said.last().unwrap().contains(" 0 FAIL"));
    assert_eq!(body_of(&cc, &mut game), altered);
}

#[test]
fn a_selftest_cut_short_by_a_logout_puts_the_body_back_at_the_next_login() {
    let (cc, mut game) = world();
    let mut altered = normal();
    altered.slots[13] = None; // the spine
    cc.with(&mut game, ME, |body, host| {
        body.cavity = altered.clone();
        body.changed(host, ME);
    });
    cc.selftest_command(&mut game, ME, true, 20.0);
    for _ in 0..20 {
        assert!(cc.selftest_tick(&mut game, ME));
    }
    assert_ne!(body_of(&cc, &mut game), altered, "a test body is on");
    // The game stops: the mod forgets the body and the run, the player's data stays.
    crate::lock(&cc.bodies).remove(&ME);
    crate::lock(&cc.selftests).remove(&ME);

    let back = Body::load(&mut game, ME);
    assert_eq!(back.cavity, altered);
    assert!(saved(&game, ME, SNAPSHOT).is_none());
    assert_eq!(saved(&game, ME, crate::body::KEY), Some(altered), "and it is saved as the body");
}

#[test]
fn a_death_halfway_ends_the_test_and_puts_the_body_back() {
    let (cc, mut game) = world();
    cc.selftest_command(&mut game, ME, false, 20.0);
    for _ in 0..10 {
        cc.selftest_tick(&mut game, ME);
    }
    let run = crate::lock(&cc.selftests).remove(&ME).expect("still running");
    cc.with(&mut game, ME, |body, host| abort(host, ME, body, run));
    assert_eq!(body_of(&cc, &mut game), normal());
    assert!(saved(&game, ME, SNAPSHOT).is_none());
    assert!(!cc.selftest_tick(&mut game, ME), "nothing runs any more");
}

#[test]
fn the_players_own_screen_neither_opens_nor_moves_organs_while_a_selftest_runs() {
    let (cc, mut game) = world();
    assert!(cc.with(&mut game, ME, |body, host| body.open(host, ME)), "a screen open before the test");
    cc.selftest_command(&mut game, ME, false, 20.0);
    cc.selftest_tick(&mut game, ME);
    // A click on the heart's slot would hand out a test organ as a real item.
    cc.clicked(&mut game, ME, 5, 4, 0);
    assert!(game.inventory.iter().all(Option::is_none), "nothing came out");
    cc.ask_own(crate::opening::Player { uuid: ME, dimension: "minecraft:overworld".into(), entity: 1, health: 20.0, position: [0.0; 3], yaw: 0.0, pitch: 0.0 });
    assert!(crate::lock(&cc.opens).is_empty(), "no screen is asked for");
}
