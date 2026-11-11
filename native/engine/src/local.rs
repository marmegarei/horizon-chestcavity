//! The screen of the player whose window this is (single player, or the host of a LAN game). The game's
//! server menus only reach other players, so this one is drawn by the mod over the world (a mod's own
//! screen): the 27 slots as a list of icons, click an organ to take it out, a button to put in the organ
//! in your hand. Drawing and clicks run on the client with a read-only `Host`, so a click only queues an
//! `Action`; the next `player_tick`, which has the player's state swapped in, carries it out.
use crate::screen::stack;
use chestcavity_shared::cavity::{Cavity, SLOTS, organ};
use horizon_client::mod_api::{HudBatch, Host, NativeScreenView, Text};

pub const MOD_ID: &str = "chestcavity";

const W: f32 = 176.0;
const H: f32 = 140.0;
const SLOT: f32 = 18.0;
const COLS: usize = 9;

/// Whose cavity the screen shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Own,
    Mob { entity: u32, mob: String },
}

/// What a click asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Take(usize),
    PutHeld,
}

pub struct Local {
    pub uuid: u128,
    pub target: Target,
    pub title: String,
    pub cavity: Cavity,
    pub summary: Option<String>,
    /// The last thing worth telling the player.
    pub note: Option<String>,
    pub pending: Vec<Action>,
}

/// What a point of the screen is.
#[derive(Debug, PartialEq)]
pub enum Hit {
    Slot(usize),
    Put,
}

/// Where the panel's top left is: centred in the screen's rectangle.
fn origin(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + (rect[2] - W) / 2.0).floor(), (rect[1] + (rect[3] - H) / 2.0).floor()]
}

fn slot_rect(o: [f32; 2], i: usize) -> [f32; 4] {
    [o[0] + 8.0 + (i % COLS) as f32 * SLOT, o[1] + 16.0 + (i / COLS) as f32 * SLOT, SLOT, SLOT]
}

fn put_rect(o: [f32; 2]) -> [f32; 4] {
    [o[0] + 8.0, o[1] + H - 22.0, 112.0, 16.0]
}

fn inside(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

/// What is under `mouse` on a screen whose rectangle is `rect`.
pub fn hit(rect: [f32; 4], mouse: [f32; 2]) -> Option<Hit> {
    let o = origin(rect);
    if inside(put_rect(o), mouse) {
        return Some(Hit::Put);
    }
    (0..SLOTS).find(|&i| inside(slot_rect(o, i), mouse)).map(Hit::Slot)
}

/// `text` broken into lines of at most `max` wide at its commas, by `width`.
fn wrap(text: &str, max: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for part in text.split(", ") {
        match lines.last_mut() {
            Some(line) if width(&format!("{line}, {part}")) <= max => {
                line.push_str(", ");
                line.push_str(part);
            }
            _ => lines.push(part.to_string()),
        }
    }
    lines
}

pub fn draw(host: &dyn Host, batch: &mut HudBatch, screen: &NativeScreenView<'_>, local: &Local) {
    let o = origin(screen.rect);
    host.fill(batch, [o[0], o[1], W, H], 0xF0101018);
    host.text(batch, &local.title, [o[0] + 8.0, o[1] + 5.0], 0xFFFFFF, true);
    let over = hit(screen.rect, screen.mouse);
    for i in 0..SLOTS {
        let r = slot_rect(o, i);
        let color = if over == Some(Hit::Slot(i)) { 0xFF4B4B6B } else { 0xFF2B2B33 };
        host.fill(batch, [r[0] + 1.0, r[1] + 1.0, r[2] - 2.0, r[3] - 2.0], color);
        if let Some(s) = &local.cavity.slots[i] {
            host.gui_draw_item(batch, &stack(&s.id, s.count), [r[0] + 1.0, r[1] + 1.0]);
        }
    }
    let mut y = o[1] + 76.0;
    if let Some(summary) = &local.summary {
        for line in wrap(summary, W - 16.0, |s| host.text_width(s)).iter().take(3) {
            host.text(batch, line, [o[0] + 8.0, y], 0xA0A0B0, false);
            y += 10.0;
        }
    }
    if let Some(note) = &local.note {
        host.text(batch, note, [o[0] + 8.0, o[1] + H - 34.0], 0xFFD070, false);
    }
    let p = put_rect(o);
    host.fill(batch, p, if over == Some(Hit::Put) { 0xFF50507A } else { 0xFF30304A });
    host.text(batch, "Put held organ in", [p[0] + 6.0, p[1] + 4.0], 0xFFFFFF, true);
    if let Some(Hit::Slot(i)) = over {
        if let Some(s) = &local.cavity.slots[i] {
            let name = organ(&s.id).map_or(s.id.as_str(), |o| o.name);
            let line = if s.count > 1 { format!("{name} x{}", s.count) } else { name.to_string() };
            host.gui_tooltip(batch, &[Text::literal(line)], screen.mouse);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_sits_in_the_middle_and_clicks_find_its_parts() {
        let rect = [0.0, 0.0, 400.0, 300.0];
        let o = origin(rect);
        assert_eq!(o, [112.0, 80.0]);
        // The first slot's middle, the last's, the button; the title and the world are nothing.
        assert_eq!(hit(rect, [o[0] + 17.0, o[1] + 25.0]), Some(Hit::Slot(0)));
        assert_eq!(hit(rect, [o[0] + 8.0 + 8.0 * 18.0 + 9.0, o[1] + 16.0 + 2.0 * 18.0 + 9.0]), Some(Hit::Slot(26)));
        assert_eq!(hit(rect, [o[0] + 20.0, o[1] + H - 14.0]), Some(Hit::Put));
        assert_eq!(hit(rect, [o[0] + 20.0, o[1] + 6.0]), None);
        assert_eq!(hit(rect, [5.0, 5.0]), None);
    }

    #[test]
    fn a_summary_wraps_at_its_commas() {
        let lines = wrap("Health 1.0, Strength 1.0, Speed 1.0", 22.0, |s| s.len() as f32);
        assert_eq!(lines, ["Health 1.0", "Strength 1.0", "Speed 1.0"]);
        assert_eq!(wrap("a, b, c", 99.0, |s| s.len() as f32), ["a, b, c"]);
    }
}
