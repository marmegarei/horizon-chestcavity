//! The chest cavity screen. Horizon's mod screens have no slots, so the 27
//! slots are a list: pick one, then take its organ out or put the held one in.
use crate::cavity::{Cavity, SLOTS, Scores, get, organ};
use horizon_mod_sdk::ui::{self, ButtonWidget, ItemWidget, ListWidget, TextWidget, Widget};

pub const LIST: u32 = 1;
pub const TAKE: u32 = 2;
pub const PUT: u32 = 3;
const SUMMARY: u32 = 4;
const ICON: u32 = 5;

/// The scores the summary shows, with their names.
const SHOWN: &[(&str, &str)] = &[
    ("health", "Health"), ("strength", "Strength"), ("speed", "Speed"), ("defense", "Defense"),
    ("nerves", "Nerves"), ("filtration", "Filtration"), ("breath_recovery", "Breath"),
    ("digestion", "Digestion"), ("nutrition", "Nutrition"), ("luck", "Luck"),
];

#[derive(Default)]
pub struct Screen {
    pub open: bool,
    pub selected: Option<usize>,
}

/// "Health 1.0 (-1.0)": each shown score, with how far it is from normal.
pub fn summary(now: &Scores, normal: &Scores) -> String {
    SHOWN
        .iter()
        .map(|&(k, name)| {
            let (v, d) = (get(now, k), get(now, k) - get(normal, k));
            if d.abs() < 0.01 { format!("{name} {v:.1}") } else { format!("{name} {v:.1} ({d:+.1})") }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl Screen {
    pub fn show(&self, cavity: &Cavity, now: &Scores, normal: &Scores) {
        let rows = (0..SLOTS)
            .map(|i| match &cavity.slots[i] {
                Some(s) => {
                    let name = organ(&s.id).map_or(s.id.as_str(), |o| o.name);
                    if s.count > 1 { format!("{:>2}. {name} x{}", i + 1, s.count) } else { format!("{:>2}. {name}", i + 1) }
                }
                None => format!("{:>2}. (empty)", i + 1),
            })
            .collect();
        let mut widgets = vec![Widget::Text(TextWidget { id: SUMMARY, text: summary(now, normal), color: 0 })];
        if let Some(s) = self.selected.and_then(|i| cavity.slots[i].as_ref()) {
            widgets.push(Widget::Item(ItemWidget { id: ICON, item: s.id.clone(), name: String::new() }));
        }
        widgets.push(Widget::List(ListWidget { id: LIST, rows, selected: self.selected.map(|i| i as u32) }));
        widgets.push(Widget::Button(ButtonWidget { id: TAKE, label: "Take organ out".into() }));
        widgets.push(Widget::Button(ButtonWidget { id: PUT, label: "Put held organ in".into() }));
        ui::open_screen(&ui::Screen { title: "Chest Cavity".into(), widgets });
    }
}
