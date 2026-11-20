//! `/cc selftest [force]`: gives the player each body of `chestcavity_shared::selftest::cases` in turn, one every few of
//! their ticks, and checks what the game reports against the rules: the attribute bases, the saved cavity, the
//! rejection effect, the grown chest and the settings file. Each check is a line of `chestcavity-trace.log` that starts
//! with PASS, FAIL, INFO or INCONCLUSIVE (the game could not be read); the chat gets the counts.
//!
//! The body the player had is saved first in their persistent data (`SNAPSHOT`) and put back at the end, or at their
//! next login when the game stopped halfway; a death halfway ends the test and puts it back too. While it runs the
//! body has no beat (no bleeding without a heart, no rejection), and the other `/cc` commands wait.
use crate::body::{Body, REJECTION};
use crate::texts::{Msg, msg};
use crate::trace;
use chestcavity_shared::cavity::{Cavity, GROWN, SLOTS, Slot, player_kind};
use chestcavity_shared::config::Config;
use chestcavity_shared::layout;
use chestcavity_shared::selftest::{Case, cases, close, expected};
use horizon_client::mod_api::{Host, Tag};

/// The key of the body saved while a selftest runs.
const SNAPSHOT: &str = "chestcavity_selftest";
/// Ticks between giving a body and reading the game: the attributes settle in the meantime.
const WAIT: u8 = 2;
/// The organ the rejection check puts in, born in another body.
const FOREIGN: &str = "chestcavity:heart";

enum Stage {
    Case(usize),
    Rejection,
    Grow,
    Config,
}

pub struct Run {
    cases: Vec<Case>,
    stage: Stage,
    /// Whether the stage's body has been given, and the ticks left before reading the game.
    applied: bool,
    wait: u8,
    pass: u32,
    fail: u32,
    info: u32,
    snapshot: Cavity,
}

/// Starts a selftest for `uuid`, or says why not: a body that is not the normal one is refused unless `force`.
pub fn start(host: &mut dyn Host, uuid: u128, body: &mut Body, force: bool) -> Result<Run, Msg> {
    let altered = body.scores != body.normal || body.cavity.slots.len() != SLOTS || body.incompat > 0;
    if altered && !force {
        return Err(msg("chestcavity.selftest.refused"));
    }
    save(host, uuid, Some(&body.cavity));
    trace::line(&*host, format!("INFO selftest: start, {} bodies", cases(uuid).len()));
    Ok(Run { cases: cases(uuid), stage: Stage::Case(0), applied: false, wait: 0, pass: 0, fail: 0, info: 0, snapshot: body.cavity.clone() })
}

impl Run {
    fn check(&mut self, host: &dyn Host, ok: bool, what: String) {
        if ok {
            self.pass += 1;
        } else {
            self.fail += 1;
        }
        trace::line(host, format!("{} {what}", if ok { "PASS" } else { "FAIL" }));
    }

    fn info(&mut self, host: &dyn Host, what: String) {
        self.info += 1;
        trace::line(host, what);
    }

    /// Gives the body `cavity` and waits for the game to settle.
    fn give(&mut self, host: &mut dyn Host, uuid: u128, body: &mut Body, cavity: Cavity) {
        body.cavity = cavity;
        body.changed(host, uuid);
        self.applied = true;
        self.wait = WAIT;
    }

    fn next(&mut self, stage: Stage) {
        self.stage = stage;
        self.applied = false;
    }
}

/// One of the player's ticks: the next step. False when the selftest is over (the body is back and the chat told).
pub fn step(host: &mut dyn Host, uuid: u128, body: &mut Body, run: &mut Run, cfg: &Config) -> bool {
    if run.wait > 0 {
        run.wait -= 1;
        return true;
    }
    match run.stage {
        Stage::Case(i) if i >= run.cases.len() => run.next(Stage::Rejection),
        Stage::Case(i) if !run.applied => {
            let cavity = run.cases[i].cavity.clone();
            run.give(host, uuid, body, cavity);
        }
        Stage::Case(i) => {
            let name = run.cases[i].name.clone();
            for (attr, want) in expected(&body.cavity) {
                match host.attribute(uuid, attr) {
                    Some((base, _)) => run.check(&*host, close(base, want), format!("{name} {attr}: expected {want:.3}, got {base:.3}")),
                    None => run.info(&*host, format!("INCONCLUSIVE {name} {attr}: the game did not say")),
                }
            }
            let saved = saved(host, uuid, crate::body::KEY);
            run.check(&*host, saved.as_ref() == Some(&body.cavity), format!("{name} saved: the cavity read back is the one given"));
            run.next(Stage::Case(i + 1));
        }
        Stage::Rejection if !cfg.organ_rejection => {
            run.info(&*host, "INFO rejection: off in the settings".into());
            run.next(Stage::Grow);
        }
        Stage::Rejection if !run.applied => {
            let mut cavity = Cavity::of(player_kind()).born(uuid, || false);
            if let Some(i) = layout::place(&cavity, FOREIGN) {
                cavity.slots[i] = Some(Slot::new(FOREIGN, 1).owned_by(Some(!uuid)));
            }
            run.give(host, uuid, body, cavity);
            run.check(&*host, body.incompat == 1, format!("rejection: {} organ of another body counted", body.incompat));
            body.show_rejection(host, uuid, cfg.rejection_rate);
        }
        Stage::Rejection => {
            match host.effects(uuid) {
                Some(effects) => {
                    let has = effects.iter().any(|e| &*e.id == REJECTION);
                    run.check(&*host, has, format!("rejection: the {REJECTION} effect is on the player"));
                }
                None => run.info(&*host, "INCONCLUSIVE rejection: the game did not list the effects".into()),
            }
            host.remove_effect(uuid, REJECTION);
            body.rejection = 0;
            run.next(Stage::Grow);
        }
        Stage::Grow => {
            body.cavity = Cavity::of(player_kind()).born(uuid, || false);
            body.changed(host, uuid);
            let grew = body.grow(host, uuid);
            let size = saved(host, uuid, crate::body::KEY).map(|c| c.slots.len());
            run.check(&*host, grew && size == Some(GROWN), format!("grow: the chest grew to {GROWN} slots and was saved so ({size:?})"));
            run.next(Stage::Config);
        }
        Stage::Config => {
            match host.config_dir().ok().and_then(|d| std::fs::read_to_string(d.join("chestcavity.json")).ok()) {
                Some(text) => run.check(&*host, Config::parse(&text) == *cfg, "config: the settings in use are those of config/chestcavity.json".into()),
                None => run.info(&*host, "INFO config: no config/chestcavity.json to compare".into()),
            }
            finish(host, uuid, body, run);
            return false;
        }
    }
    true
}

/// The selftest is over: the body it started with is back, and the chat has the counts.
fn finish(host: &mut dyn Host, uuid: u128, body: &mut Body, run: &Run) {
    put_back(host, uuid, body, run.snapshot.clone());
    trace::line(&*host, format!("INFO selftest: end, {} PASS, {} FAIL, {} INFO", run.pass, run.fail, run.info));
    host.tell(uuid, msg("chestcavity.selftest.result").plain_arg(run.pass).plain_arg(run.fail).plain_arg(run.info).text());
}

/// A death halfway: the test ends as failed and the body it started with is back before the respawn sees it.
pub fn abort(host: &mut dyn Host, uuid: u128, body: &mut Body, run: Run) {
    trace::line(&*host, "FAIL selftest: stopped by the player's death");
    put_back(host, uuid, body, run.snapshot);
}

fn put_back(host: &mut dyn Host, uuid: u128, body: &mut Body, cavity: Cavity) {
    host.remove_effect(uuid, REJECTION);
    body.rejection = 0;
    body.cavity = cavity;
    body.changed(host, uuid);
    save(host, uuid, None);
}

/// The body a selftest that never ended left saved, taken away: the one to load instead of the test's.
pub fn left_over(host: &mut dyn Host, uuid: u128) -> Option<Cavity> {
    let cavity = saved(host, uuid, SNAPSHOT)?;
    save(host, uuid, None);
    trace::line(&*host, "INFO selftest: the body from before an unfinished selftest is back");
    Some(cavity)
}

/// The cavity saved under `key` in the player's persistent data.
fn saved(host: &dyn Host, uuid: u128, key: &str) -> Option<Cavity> {
    match host.player_persistent_data(uuid)?.get(key) {
        Some(Tag::String(s)) => Some(Cavity::from_text(&s.to_string_lossy())),
        _ => None,
    }
}

/// Saves the body to put back (`Some`) or forgets it (`None`).
fn save(host: &mut dyn Host, uuid: u128, cavity: Option<&Cavity>) {
    let mut data = host.player_persistent_data(uuid).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    let Tag::Compound(map) = &mut data else { return };
    match cavity {
        Some(c) => {
            map.insert(SNAPSHOT.into(), Tag::String(c.to_text().as_str().into()));
        }
        None => {
            if map.remove(&SNAPSHOT.into()).is_none() {
                return;
            }
        }
    }
    host.set_player_persistent_data(uuid, data);
}

impl crate::ChestCavity {
    /// `/cc selftest [force]` from `uuid`: starts it, or says why not.
    pub(crate) fn selftest_command(&self, host: &mut dyn Host, uuid: u128, force: bool) {
        let started = self.with(host, uuid, |body, host| start(host, uuid, body, force));
        let line = match started {
            Ok(run) => {
                crate::lock(&self.selftests).insert(uuid, run);
                msg("chestcavity.selftest.started")
            }
            Err(why) => why,
        };
        host.tell(uuid, line.text());
    }

    /// The player's tick while a selftest runs for them: its next step, and true (the body has no other tick then).
    pub(crate) fn selftest_tick(&self, host: &mut dyn Host, uuid: u128) -> bool {
        let Some(mut run) = crate::lock(&self.selftests).remove(&uuid) else { return false };
        let cfg = self.cfg();
        if self.with(host, uuid, |body, host| step(host, uuid, body, &mut run, &cfg)) {
            crate::lock(&self.selftests).insert(uuid, run);
        }
        true
    }
}

#[cfg(test)]
mod tests;
