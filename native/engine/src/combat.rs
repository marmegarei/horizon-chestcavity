//! What a body does when it fights. Its blows lift and poison what they hit (`OrganOnHitListeners`, the venom gland), it
//! steps out of the way of arrows (`attemptArrowDodging`), and the organs that give wings let it fly. The hooks only note
//! what happened; the server tick, where the game lets a mod run commands and move entities, does it.
use crate::body::Body;
use crate::opening::Player;
use crate::teleport;
use crate::{ChestCavity, lock};
use chestcavity_shared::cavity::get;
use horizon_client::mod_api::Host;

/// `LAUNCHING_POWER`: the upward speed a point of launching gives what it hits.
const LAUNCHING_POWER: f64 = 0.1;
/// `ARROW_DODGE_DISTANCE` and `ARROW_DODGE_COOLDOWN`, both divided by how well the body dodges.
const DODGE_DISTANCE: f32 = 32.0;
const DODGE_REST: f32 = 200.0;
/// `VENOM_COOLDOWN` in ticks, and how long the poison it gives lasts, in seconds.
const VENOM_REST: u64 = 40;
const VENOM_SECONDS: u32 = 10;

/// A blow of a player's that landed, to be answered at the server tick.
pub struct Hit {
    pub target: u32,
    pub launch: f32,
    pub venom: bool,
}

/// "123e4567-e89b-12d3-a456-426614174000": how a command names an entity by its uuid.
fn uuid_text(id: u128) -> String {
    format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", (id >> 96) as u32, (id >> 80) as u16, (id >> 64) as u16, (id >> 48) as u16, id as u64 & 0xffff_ffff_ffff)
}

impl ChestCavity {
    /// The acting player hit entity `target`: noted when their body has something to add to it.
    pub(crate) fn attacked(&self, host: &mut dyn Host, p: &Player, target: u32) {
        let barehanded = host.held_stack(p.uuid).is_none();
        let hit = self.with(host, p.uuid, |body, _| {
            let launch = get(&body.scores, "launching");
            let venom = barehanded && get(&body.scores, "venomous") > 0.0 && body.rests.get("venom").is_none_or(|&until| until <= body.ticks);
            if venom {
                body.rests.insert("venom", body.ticks + VENOM_REST);
            }
            (launch > 0.0 || venom).then_some(Hit { target, launch, venom })
        });
        lock(&self.hits).extend(hit);
    }

    /// What `uuid`'s body does to a hit it takes: the damage left, or none when it dodged (a body the engine has not met
    /// takes it as it comes). A dodged hit sends the body somewhere else at the next server tick.
    pub(crate) fn incoming(&self, uuid: u128, damage_type: &str, amount: f32) -> Option<f32> {
        let mut bodies = lock(&self.bodies);
        let Some(body) = bodies.get_mut(&uuid) else { return Some(amount) };
        let resist = body.resist;
        if resist.dodges(damage_type) && body.rests.get("arrow_dodging").is_none_or(|&until| until <= body.ticks) {
            body.rests.insert("arrow_dodging", body.ticks + (DODGE_REST / resist.dodging) as u64);
            lock(&self.dodges).push((uuid, DODGE_DISTANCE / resist.dodging));
            return None;
        }
        Some(if resist.is_normal() { amount } else { resist.apply(damage_type, amount) })
    }

    /// Lifts and poisons what players hit, and sends the bodies that dodged somewhere else.
    pub(crate) fn run_combat(&self, host: &mut dyn Host) {
        let hits = std::mem::take(&mut *lock(&self.hits));
        for Hit { target, launch, venom } in hits {
            let Some(entity) = host.entity(target) else { continue };
            if launch > 0.0 {
                if let Some(v) = entity.velocity(host) {
                    entity.set_velocity(host, [v[0], v[1] + LAUNCHING_POWER * f64::from(launch), v[2]]);
                }
            }
            if venom {
                if let Some(id) = entity.uuid(host) {
                    host.run_command(&format!("effect give {} minecraft:poison {VENOM_SECONDS} 0", uuid_text(id)));
                }
            }
        }
        let dodges = std::mem::take(&mut *lock(&self.dodges));
        if dodges.is_empty() {
            return;
        }
        for (uuid, range) in dodges {
            if let Some(p) = self.alive(uuid) {
                teleport::randomly(host, &p, f64::from(range), &mut lock(&self.rng));
            }
        }
    }
}

impl Body {
    /// Wings: a body that has them may fly as a creative player does; one that loses them lands.
    pub fn fly(&mut self, host: &mut dyn Host, uuid: u128, game_mode: i32) {
        // The game takes the allowance back whenever the game mode changes.
        if game_mode != self.mode {
            self.mode = game_mode;
            self.wings = false;
        }
        let wings = get(&self.scores, "flight") - get(&self.normal, "flight") > 0.0 && matches!(game_mode, 0 | 2);
        if wings != self.wings {
            host.set_flight(uuid, wings, false);
            self.wings = wings;
        }
    }
}

#[cfg(test)]
mod tests;
