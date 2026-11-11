//! The game as far as the engine asks of it, for the tests: a player's data, attributes, inventory,
//! commands, effects, the screen they have open, and the attachments of mobs.
use horizon_client::mod_api::{AttachmentError, Attachments, Holder, Host, ItemStack, NativeEffect, Tag, Text};
use std::collections::HashMap;

pub struct Fake {
    pub data: Option<Tag>,
    pub bases: Vec<(String, f64)>,
    pub inventory: Vec<Option<ItemStack>>,
    pub commands: Vec<String>,
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
}

impl Fake {
    pub fn new() -> Self {
        Fake {
            data: None,
            bases: Vec::new(),
            inventory: vec![None; 36],
            commands: Vec::new(),
            effects: Vec::new(),
            menu: None,
            menu_title: None,
            server_menus: true,
            screens: Vec::new(),
            said: Vec::new(),
            mobs: HashMap::new(),
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
        assert_eq!((rows, items.len()), (3, 27));
        if !self.server_menus {
            return None;
        }
        self.menu = Some(items);
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
    fn run_command(&mut self, line: &str) -> bool {
        self.commands.push(line.to_string());
        true
    }
}
