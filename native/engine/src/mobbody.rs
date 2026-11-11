//! A mob whose chest cavity a player has changed: its organs now say what it can still do. Take out its heart and it
//! bleeds to death, its spine and it can't move, its lungs and it drowns on land, its kidneys and it is poisoned, as for
//! a player. The mob's own state (`MobBody`) decides what to do each second; `run_mobs` does it through `Host`.
use crate::{ChestCavity, lock, mob};
use chestcavity_shared::cavity::{Cavity, Kind, Scores, get, mob_kind};
use chestcavity_shared::effects::{self, Action, Env, Progress};
use horizon_client::mod_api::Host;

/// What a mob has to be made to do.
#[derive(Debug, PartialEq)]
pub enum Act {
    Hurt { amount: f32, kind: &'static str },
    Effect { id: &'static str, ticks: u32, amplifier: u32 },
    /// Attribute modifiers: the ones to take off, then the ones to put on (attribute, amount, operation).
    Modifiers { remove: Vec<&'static str>, add: Vec<(&'static str, f64, &'static str)> },
}

pub struct MobBody {
    pub dimension: String,
    scores: Scores,
    normal: Scores,
    progress: Progress,
    /// The attributes it has modifiers on.
    applied: Vec<&'static str>,
    dirty: bool,
}

impl MobBody {
    /// The body of a mob of type `mob` with `cavity`; none for a type Chest Cavity has no body for.
    pub fn new(dimension: &str, mob: &str, cavity: &Cavity) -> Option<MobBody> {
        let kind: &'static Kind = mob_kind(mob)?;
        Some(MobBody {
            dimension: dimension.to_string(),
            scores: cavity.scores(kind),
            normal: Cavity::of(kind).scores(kind),
            progress: Progress::default(),
            applied: Vec::new(),
            dirty: true,
        })
    }

    /// Its cavity changed.
    pub fn set(&mut self, mob: &str, cavity: &Cavity) {
        if let Some(kind) = mob_kind(mob) {
            self.scores = cavity.scores(kind);
            self.dirty = true;
        }
    }

    /// Whether it is as it was born and has nothing left to take back.
    pub fn is_idle(&self) -> bool {
        !self.dirty && self.applied.is_empty() && self.scores == self.normal
    }

    /// What its organs do to it this second.
    pub fn beat(&mut self, seconds: u64) -> Vec<Act> {
        let mut acts = Vec::new();
        if self.dirty {
            self.dirty = false;
            let add = effects::modifiers(&self.scores, &self.normal);
            let remove = std::mem::replace(&mut self.applied, add.iter().map(|m| m.0).collect());
            acts.push(Act::Modifiers { remove, add });
        }
        for action in effects::periodic(&self.scores, &self.normal, seconds, &mut self.progress, &Env::default()) {
            match action {
                Action::Damage { amount, kind } => acts.push(Act::Hurt { amount, kind }),
                Action::Effect { id, ticks, amplifier } => acts.push(Act::Effect { id, ticks, amplifier }),
                _ => {}
            }
        }
        // No lungs: out of water the air runs out at once, and it drowns.
        if get(&self.normal, "breath_recovery") > 0.0 && get(&self.scores, "breath_recovery") <= 0.0 && get(&self.scores, "water_breath") <= 0.0 {
            acts.push(Act::Hurt { amount: 2.0, kind: "minecraft:drown" });
        }
        acts
    }
}

/// "chestcavity:cavity_movement_speed": the id of the modifier an attribute gets.
fn modifier_id(attribute: &str) -> String {
    format!("chestcavity:cavity_{}", attribute.rsplit(':').next().unwrap_or(attribute))
}

impl ChestCavity {
    /// Mob `entity` of type `mob` has `cavity` now: make it live by it.
    pub(crate) fn track_mob(&self, entity: u32, mob: &str, dimension: &str, cavity: &Cavity) {
        let mut mobs = lock(&self.mobs);
        match mobs.get_mut(&entity) {
            Some(body) => body.set(mob, cavity),
            None => {
                if let Some(body) = MobBody::new(dimension, mob, cavity) {
                    mobs.insert(entity, body);
                }
            }
        }
    }

    /// A mob loaded with a cavity a player once changed.
    pub(crate) fn mob_joined(&self, host: &mut dyn Host, entity: u32, mob: &str, dimension: &str) {
        if mob_kind(mob).is_some() {
            if let Some(cavity) = mob::existing(host, entity) {
                self.track_mob(entity, mob, dimension, &cavity);
            }
        }
    }

    /// Once a second: what the organs of every mob that has been opened do to it.
    pub(crate) fn run_mobs(&self, host: &mut dyn Host, seconds: u64) {
        let ids: Vec<u32> = lock(&self.mobs).keys().copied().collect();
        for id in ids {
            let Some(entity) = host.entity(id) else {
                lock(&self.mobs).remove(&id);
                continue;
            };
            let Some((acts, dimension)) = lock(&self.mobs).get_mut(&id).map(|b| (b.beat(seconds), b.dimension.clone())) else { continue };
            let uuid = entity.uuid(host).map(crate::combat::uuid_text);
            for act in acts {
                match (act, &uuid) {
                    (Act::Hurt { amount, kind }, _) => {
                        host.hurt_entity(&dimension, id, amount, kind, None);
                    }
                    (Act::Effect { id: effect, ticks, amplifier }, Some(uuid)) => {
                        host.run_command(&format!("effect give {uuid} {effect} {} {amplifier} true", (ticks / 20).max(1)));
                    }
                    (Act::Modifiers { remove, add }, Some(uuid)) => {
                        for attribute in remove {
                            host.run_command(&format!("attribute {uuid} {attribute} modifier remove {}", modifier_id(attribute)));
                        }
                        for (attribute, amount, operation) in add {
                            host.run_command(&format!("attribute {uuid} {attribute} modifier add {} {amount} {operation}", modifier_id(attribute)));
                        }
                    }
                    _ => {}
                }
            }
            let mut mobs = lock(&self.mobs);
            if mobs.get(&id).is_some_and(MobBody::is_idle) {
                mobs.remove(&id);
            }
        }
    }
}

#[cfg(test)]
mod tests;
