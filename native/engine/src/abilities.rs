//! Setting off the abilities organs give. A key (or `/cc ability`) only asks; the server tick does it, where the game lets a
//! mod run commands, spawn entities and hand out items.
use crate::body::Body;
use crate::opening::Player;
use crate::screen::stack;
use crate::{ChestCavity, lock};
use chestcavity_shared::abilities::{self, Ability};
use chestcavity_shared::cavity::{get, organ};
use chestcavity_shared::drops::Rng;
use horizon_client::mod_api::{Compound, Host, Tag};

/// What a point of iron repair mends: a quarter of the body's health.
const IRON_REPAIR: f32 = 0.25;
/// What silk needs in food.
const SILK_FOOD: i32 = 6;

fn list(v: [f64; 3]) -> Tag {
    Tag::List(v.iter().map(|x| Tag::Double(x.to_bits())).collect())
}

impl ChestCavity {
    /// `uuid` pressed a key or asked by name for `which`: a key group (`utility`, `attack`) or an ability.
    pub(crate) fn ask_ability(&self, uuid: u128, which: &str) {
        lock(&self.abilities).push((uuid, which.to_string()));
    }

    /// Sets off what was asked, as each body allows; a named ability that can't be used says why.
    pub(crate) fn run_abilities(&self, host: &mut dyn Host) {
        let asked = std::mem::take(&mut *lock(&self.abilities));
        if asked.is_empty() {
            return;
        }
        let players = host.server_players();
        for (uuid, which) in asked {
            let Some(p) = players.iter().find(|p| p.uuid == uuid && p.game_mode != 3 && p.health > 0.0).map(Player::from) else { continue };
            for ability in abilities::picked(&which) {
                let said = self.with(host, uuid, |body, host| body.activate(host, &p, ability, &mut lock(&self.rng)));
                if let Err(why) = said {
                    if ability.name == which {
                        Self::say(host, uuid, why);
                    }
                }
            }
        }
    }
}

impl Body {
    /// The ability, if this body has it and it isn't resting.
    pub fn activate(&mut self, host: &mut dyn Host, p: &Player, ability: &Ability, rng: &mut Rng) -> Result<(), String> {
        let level = get(&self.scores, ability.score) - get(&self.normal, ability.score);
        if level <= 0.0 {
            return Err(format!("O teu corpo não tem {}", ability.name));
        }
        if let Some(&until) = self.rests.get(ability.name).filter(|&&until| until > self.ticks) {
            return Err(format!("{} ainda está a recuperar ({} s)", ability.name, (until - self.ticks).div_ceil(20)));
        }
        let rest = match ability.name {
            "iron_repair" => self.iron_repair(host, p, level)?,
            "creepy" => self.creepy(host, p)?,
            "grazing" => self.graze(host, p)?,
            "silk" => self.spin(host, p, level)?,
            _ => self.shoot(host, p, ability.name, level, rng)?,
        };
        self.rests.insert(ability.name, self.ticks + u64::from(rest.unwrap_or(ability.rest)));
        Ok(())
    }

    /// Exhaustion: what the ability costs a player in food.
    fn tire(&mut self, host: &mut dyn Host, uuid: u128, amount: f32) {
        if let Some((level, saturation, exhaustion)) = self.food {
            let food = (level, saturation, exhaustion + amount);
            host.set_player_status(uuid, None, Some(food), None);
            self.food = Some(food);
        }
    }

    pub fn max_health(&self) -> f32 {
        20.0 + 4.0 * (get(&self.scores, "health") - get(&self.normal, "health"))
    }

    /// An iron ingot mends a quarter of the body's health; the better the organ, the shorter the rest.
    fn iron_repair(&mut self, host: &mut dyn Host, p: &Player, level: f32) -> Result<Option<u32>, String> {
        if p.health >= self.max_health() {
            return Err("Já tens a vida cheia".into());
        }
        if !take_one(host, p.uuid, &["minecraft:iron_ingot", "chestcavity:iron_scrap"]) {
            return Err("Precisas de um lingote de ferro no inventário".into());
        }
        let health = (p.health + self.max_health() * IRON_REPAIR).min(self.max_health());
        host.set_player_status(p.uuid, Some(health), None, None);
        host.play_sound_to(p.uuid, "minecraft:entity.iron_golem.repair", "player", Some(p.position), 0.75, 1.0);
        Ok(Some((1200.0 / level) as u32))
    }

    /// Explodes the body with the strength of its explosive organs, which are used up.
    fn creepy(&mut self, host: &mut dyn Host, p: &Player) -> Result<Option<u32>, String> {
        let explosive = get(&self.scores, "explosive");
        for slot in self.cavity.slots.iter_mut() {
            if slot.as_ref().and_then(|s| organ(&s.id)).is_some_and(|o| o.scores.iter().any(|s| s.0 == "explosive")) {
                *slot = None;
            }
        }
        self.changed(host, p.uuid);
        let nbt: Compound = [
            ("Fuse".into(), Tag::Short(0)),
            ("ignited".into(), Tag::Byte(1)),
            ("ExplosionRadius".into(), Tag::Byte(explosive.sqrt().round().clamp(1.0, 100.0) as i8)),
        ]
        .into_iter()
        .collect();
        host.summon(&p.dimension, "minecraft:creeper", p.position, nbt).ok_or_else(|| "Não consegui explodir".to_string())?;
        Ok(None)
    }

    /// Eats the grass under the feet.
    fn graze(&mut self, host: &mut dyn Host, p: &Player) -> Result<Option<u32>, String> {
        let [x, y, z] = [p.position[0].floor() as i32, p.position[1].floor() as i32 - 1, p.position[2].floor() as i32];
        let left = match host.block([x, y, z]).as_str() {
            "minecraft:grass_block" | "minecraft:mycelium" => "minecraft:dirt",
            "minecraft:crimson_nylium" | "minecraft:warped_nylium" => "minecraft:netherrack",
            _ => return Err("Não há erva debaixo de ti".into()),
        };
        host.run_command(&format!("setblock {x} {y} {z} {left}"));
        // ponytail: Chest Cavity chews the grass for a while (the Ruminating effect); here it is eaten at once.
        self.nourish(host, p, 1, 0.5, 0.0);
        Ok(None)
    }

    /// Webs: wool or a cobweb behind the body, and string for what is left of the silk.
    fn spin(&mut self, host: &mut dyn Host, p: &Player, silk: f32) -> Result<Option<u32>, String> {
        if self.food.is_some_and(|f| f.0 < SILK_FOOD) {
            return Err("Tens pouca fome para tecer".into());
        }
        let (mut silk, mut cost) = (silk, 0.0);
        if silk >= 2.0 {
            // Behind: against where it faces, along the axis it faces most.
            let (sin, cos) = (f64::from(p.yaw).to_radians().sin(), f64::from(p.yaw).to_radians().cos());
            let back = if cos.abs() >= sin.abs() { [0, -(cos.signum() as i32)] } else { [sin.signum() as i32, 0] };
            let [x, y, z] = [p.position[0].floor() as i32 + back[0], p.position[1].floor() as i32, p.position[2].floor() as i32 + back[1]];
            if host.block([x, y, z]) == "minecraft:air" {
                let (block, used, tired) = if silk >= 3.0 { ("minecraft:white_wool", 3.0, 16.0) } else { ("minecraft:cobweb", 2.0, 8.0) };
                host.run_command(&format!("setblock {x} {y} {z} {block}"));
                silk -= used;
                cost += tired;
            }
        }
        while silk >= 1.0 {
            silk -= 1.0;
            cost += 4.0;
            host.give_stack(p.uuid, stack("minecraft:string", 1));
        }
        if cost == 0.0 {
            return Err("Não há onde tecer".into());
        }
        self.tire(host, p.uuid, cost);
        Ok(None)
    }

    /// Fireballs, bombs and spit: as many as the score says, along where the body looks.
    fn shoot(&mut self, host: &mut dyn Host, p: &Player, name: &str, level: f32, rng: &mut Rng) -> Result<Option<u32>, String> {
        let Some(shot) = abilities::shot(name) else { return Err(format!("{name} não se dispara")) };
        let shots = level as u32;
        if shots < 1 {
            return Err(format!("Precisas de mais {name} para isto"));
        }
        let aim = abilities::aim(p.yaw, p.pitch);
        let eyes = [p.position[0], p.position[1] + 1.4, p.position[2]];
        for _ in 0..shots {
            // Pyromancy sprays a little.
            let spray = |rng: &mut Rng| if name == "pyromancy" { (f64::from(rng.below(2001)) / 1000.0 - 1.0) * 0.1 } else { 0.0 };
            let dir = [aim[0] + spray(rng), aim[1], aim[2] + spray(rng)];
            let at = [eyes[0] + dir[0], eyes[1] + dir[1], eyes[2] + dir[2]];
            let v = dir.map(|d| d * shot.speed);
            let nbt: Compound = [("Motion".into(), list(v)), ("power".into(), list(v))].into_iter().collect();
            host.summon(&p.dimension, shot.kind, at, nbt);
        }
        self.tire(host, p.uuid, shot.tire * shots as f32);
        Ok(None)
    }
}

/// Takes one of the first item of `items` the player carries; whether there was one.
fn take_one(host: &mut dyn Host, uuid: u128, items: &[&str]) -> bool {
    let Some(inventory) = host.player_stacks(uuid) else { return false };
    let Some((slot, held)) = inventory.into_iter().enumerate().find_map(|(i, s)| s.filter(|s| items.contains(&&*s.item)).map(|s| (i, s))) else {
        return false;
    };
    if host.take_stacks(uuid, &[(slot, held.clone())]).is_err() {
        return false;
    }
    if held.count > 1 {
        host.give_stack(uuid, stack(&held.item, held.count - 1));
    }
    true
}

#[cfg(test)]
mod tests;
