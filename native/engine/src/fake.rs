//! The game as far as the engine asks of it, for the tests: a player's data, attributes, inventory,
//! commands, effects, the screen they have open, and the attachments of mobs.
use horizon_client::mod_api::{AttachmentError, Attachments, BlockStateId, Compound, Holder, Host, ItemStack, NativeEffect, Tag, Text};
use std::collections::HashMap;

pub struct Fake {
    pub data: Option<Tag>,
    pub bases: Vec<(String, f64)>,
    pub inventory: Vec<Option<ItemStack>>,
    pub commands: Vec<String>,
    /// (dimension, target, amount, damage type, attacker) of each hurt the engine asked for.
    pub hurts: Vec<(String, u32, f32, String, Option<u32>)>,
    pub effects: Vec<NativeEffect>,
    pub menu: Option<Vec<Option<ItemStack>>>,
    pub menu_title: Option<String>,
    /// Whether the game's server menus reach the player (not the window's own player).
    pub server_menus: bool,
    /// The mod screens opened, and what the player was told.
    pub screens: Vec<String>,
    pub said: Vec<String>,
    /// The mobs of the world, by entity id.
    pub mobs: HashMap<u32, Attachments>,
    /// The food bar of the player, and every status (health, food) the engine set.
    pub food: Option<(i32, f32, f32)>,
    pub statuses: Vec<(Option<f32>, Option<(i32, f32, f32)>)>,
    /// What was spawned: kind, position, data.
    pub summons: Vec<(String, [f64; 3], Compound)>,
    /// The blocks that are not air, by position.
    pub blocks: HashMap<[i32; 3], String>,
    /// Whether the engine let the player fly, each time it said so.
    pub flights: Vec<bool>,
    /// The player's potion effects, and how many times the engine replaced them.
    pub current: Vec<NativeEffect>,
    pub set_effects_calls: u32,
    /// The effects the engine took off the player.
    pub removed: Vec<String>,
    /// The blocks the engine set: dimension, position, state (0 is air).
    pub set_blocks: Vec<(String, [i32; 3], u32)>,
    /// The rows of the last menu the engine opened.
    pub menu_rows: Option<u8>,
}

impl Fake {
    pub fn new() -> Self {
        Fake {
            data: None,
            bases: Vec::new(),
            inventory: vec![None; 36],
            commands: Vec::new(),
            hurts: Vec::new(),
            effects: Vec::new(),
            menu: None,
            menu_title: None,
            server_menus: true,
            screens: Vec::new(),
            said: Vec::new(),
            mobs: HashMap::new(),
            food: None,
            statuses: Vec::new(),
            summons: Vec::new(),
            blocks: HashMap::new(),
            flights: Vec::new(),
            current: Vec::new(),
            set_effects_calls: 0,
            removed: Vec::new(),
            set_blocks: Vec::new(),
            menu_rows: None,
        }
    }

    pub fn base(&self, attribute: &str) -> f64 {
        self.bases.iter().rev().find(|(a, _)| a == attribute).map(|&(_, b)| b).expect(attribute)
    }
}

impl Host for Fake {
    fn attachments(&mut self, holder: Holder) -> Result<&mut Attachments, AttachmentError> {
        match holder {
            Holder::Entity(id) => self.mobs.get_mut(&id).ok_or(AttachmentError::MissingHolder),
            _ => Err(AttachmentError::MissingHolder),
        }
    }
    fn player_persistent_data(&self, _: u128) -> Option<Tag> {
        self.data.clone()
    }
    fn set_player_persistent_data(&mut self, _: u128, data: Tag) -> bool {
        self.data = Some(data);
        true
    }
    fn set_attribute_base(&mut self, _: u128, attribute: &str, base: f64) -> bool {
        self.bases.push((attribute.to_string(), base));
        true
    }
    fn give_stack(&mut self, _: u128, stack: ItemStack) -> Option<ItemStack> {
        match self.inventory.iter_mut().find(|s| s.is_none()) {
            Some(slot) => {
                *slot = Some(stack);
                None
            }
            None => Some(stack),
        }
    }
    fn take_stacks(&mut self, _: u128, slots: &[(usize, ItemStack)]) -> Result<Vec<ItemStack>, String> {
        if slots.iter().any(|(i, s)| self.inventory[*i].as_ref() != Some(s)) {
            return Err("that stack changed".into());
        }
        Ok(slots.iter().map(|(i, _)| self.inventory[*i].take().unwrap()).collect())
    }
    fn player_stacks(&self, _: u128) -> Option<Vec<Option<ItemStack>>> {
        Some(self.inventory.clone())
    }
    fn open_menu(&mut self, _: u128, rows: u8, title: Text, items: Vec<Option<ItemStack>>) -> Option<i32> {
        assert_eq!(rows as usize * 9, items.len(), "a menu of whole rows");
        if !self.server_menus {
            return None;
        }
        self.menu = Some(items);
        self.menu_rows = Some(rows);
        self.menu_title = Some(title.plain());
        Some(5)
    }
    fn set_menu_items(&mut self, _: u128, items: Vec<Option<ItemStack>>) {
        self.menu = Some(items);
    }
    fn held_stack(&self, _: u128) -> Option<ItemStack> {
        // The main hand of the double is hotbar slot 0.
        self.inventory[0].clone()
    }
    fn take_items(&mut self, item: &str, count: u32) -> u32 {
        let mut taken = 0;
        for slot in self.inventory.iter_mut() {
            let Some(s) = slot.as_mut().filter(|s| &*s.item == item) else { continue };
            let n = s.count.min(count - taken);
            s.count -= n;
            taken += n;
            if s.count == 0 {
                *slot = None;
            }
        }
        taken
    }
    fn mod_screen_open(&mut self, mod_id: &str) -> bool {
        self.screens.push(mod_id.to_string());
        true
    }
    fn tell(&mut self, _: u128, line: Text) -> bool {
        self.said.push(line.plain());
        true
    }
    fn action_bar(&mut self, _: u128, line: Text) -> bool {
        self.said.push(line.plain());
        true
    }
    fn add_effect(&mut self, _: u128, effect: &NativeEffect) -> bool {
        self.effects.push(effect.clone());
        true
    }
    fn remove_effect(&mut self, _: u128, id: &str) -> bool {
        self.removed.push(id.to_string());
        true
    }
    fn run_command(&mut self, line: &str) -> bool {
        self.commands.push(line.to_string());
        true
    }
    fn hurt_entity(&mut self, dimension: &str, target: u32, amount: f32, damage_type: &str, attacker: Option<u32>) -> bool {
        self.hurts.push((dimension.into(), target, amount, damage_type.into(), attacker));
        true
    }
    fn player_food(&self, _: u128) -> Option<(i32, f32, f32)> {
        self.food
    }
    fn set_player_status(&mut self, _: u128, health: Option<f32>, food: Option<(i32, f32, f32)>, _: Option<(i32, f32, i32)>) -> bool {
        self.statuses.push((health, food));
        if food.is_some() {
            self.food = food;
        }
        true
    }
    fn summon(&mut self, _: &str, kind: &str, pos: [f64; 3], nbt: Compound) -> Option<u32> {
        self.summons.push((kind.to_string(), pos, nbt));
        Some(self.summons.len() as u32)
    }
    fn default_state(&self, block: &str) -> Option<BlockStateId> {
        Some(BlockStateId(u32::from(block != "minecraft:air")))
    }
    fn set_block(&mut self, dimension: &str, pos: [i32; 3], state: BlockStateId, _: i32) -> bool {
        self.set_blocks.push((dimension.to_string(), pos, state.0));
        if state.0 == 0 {
            self.blocks.remove(&pos);
        }
        true
    }
    fn block(&self, pos: [i32; 3]) -> String {
        self.blocks.get(&pos).cloned().unwrap_or_else(|| "minecraft:air".to_string())
    }
    fn play_sound_to(&mut self, _: u128, _: &str, _: &str, _: Option<[f64; 3]>, _: f32, _: f32) -> bool {
        true
    }
    fn set_flight(&mut self, _: u128, allow: bool, _: bool) -> bool {
        self.flights.push(allow);
        true
    }
    fn entity(&self, _: u32) -> Option<horizon_client::mod_api::EntityRef> {
        None
    }
    fn effects(&self, _: u128) -> Option<Vec<NativeEffect>> {
        Some(self.current.clone())
    }
    fn set_effects(&mut self, _: u128, effects: &[NativeEffect]) -> bool {
        self.current = effects.to_vec();
        self.set_effects_calls += 1;
        true
    }
    fn config_dir(&self) -> Result<std::path::PathBuf, String> {
        Err("the tests have no config folder".into())
    }
}
