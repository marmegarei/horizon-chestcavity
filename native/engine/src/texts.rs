//! Everything the mod says that is not an organ's description (`chestcavity_shared::describe`): a lang key and what goes
//! into its template. The game shows it in the language of whoever reads it, from the content mod's lang files
//! (`assets/chestcavity/lang`); the English here is the fallback for a language without the key, and what the tests read.
use chestcavity_shared::describe::{Arg, fill};
use horizon_client::mod_api::Text;

/// Every message: lang key and English. `tests` checks that the lang files carry them all.
pub const ALL: &[(&str, &str)] = &[
    ("chestcavity.title.own", "Chest Cavity"),
    ("chestcavity.title.mob", "%s's Chest Cavity"),
    ("chestcavity.msg.no_screen", "Could not open the screen: use /cc list, /cc take <slot> and /cc put"),
    ("chestcavity.msg.reset", "Chest cavity back to normal"),
    ("chestcavity.msg.slot_empty", "Slot %s is empty"),
    ("chestcavity.msg.took", "You took out slot %s"),
    ("chestcavity.msg.take_usage", "/cc take 1-%s"),
    ("chestcavity.msg.unknown_sub", "Unknown: /cc %s (show, list, take <slot>, put, ability <name>)"),
    ("chestcavity.msg.put", "You put %s in the cavity"),
    ("chestcavity.msg.no_held", "Hold an organ in your main hand to put it in"),
    ("chestcavity.msg.no_space", "No room in the inventory"),
    ("chestcavity.msg.not_organ", "%s is not an organ"),
    ("chestcavity.msg.max_stack", "Put in at most %s %s at a time"),
    ("chestcavity.msg.full", "All slots are taken: take an organ out first"),
    ("chestcavity.msg.stale", "That stack changed"),
    ("chestcavity.msg.mob_none", "%s has no chest cavity"),
    ("chestcavity.msg.mob_gone", "%s is gone, or the game keeps no cavity for it"),
    ("chestcavity.msg.mob_vanished", "%s disappeared"),
    ("chestcavity.list.empty", "(empty)"),
    ("chestcavity.summary.entry", "%s %s"),
    ("chestcavity.summary.entry_diff", "%s %s (%s)"),
    ("chestcavity.ability.creepy", "Creeper blast"),
    ("chestcavity.ability.dragon_bombs", "Dragon bombs"),
    ("chestcavity.ability.forceful_spit", "Forceful spit"),
    ("chestcavity.ability.ghastly", "Ghastly"),
    ("chestcavity.ability.pyromancy", "Pyromancy"),
    ("chestcavity.ability.grazing", "Grazing"),
    ("chestcavity.ability.iron_repair", "Iron repair"),
    ("chestcavity.ability.silk", "Silk"),
    ("chestcavity.msg.no_ability", "Your body has no %s"),
    ("chestcavity.msg.resting", "%s is still recovering (%s s)"),
    ("chestcavity.msg.health_full", "Your health is already full"),
    ("chestcavity.msg.need_iron", "You need an iron ingot in your inventory"),
    ("chestcavity.msg.no_blast", "Could not explode"),
    ("chestcavity.msg.no_grass", "There is no grass under you"),
    ("chestcavity.msg.no_spin", "Nowhere to spin"),
    ("chestcavity.msg.hungry_silk", "You are too hungry to spin silk"),
    ("chestcavity.msg.not_shot", "%s cannot be fired"),
    ("chestcavity.msg.need_more", "You need more %s for that"),
];

fn english(key: &str) -> &'static str {
    ALL.iter().find(|(k, _)| *k == key).map_or("", |e| e.1)
}

/// A message: a key of `ALL` and its arguments.
#[derive(Clone, Debug)]
pub struct Msg {
    key: &'static str,
    args: Vec<Text>,
}

pub fn msg(key: &'static str) -> Msg {
    debug_assert!(!english(key).is_empty(), "{key} is not in texts::ALL");
    Msg { key, args: Vec::new() }
}

impl Msg {
    pub fn arg(mut self, text: impl Into<Text>) -> Msg {
        self.args.push(text.into());
        self
    }

    /// A number or a name that is the same in every language.
    pub fn plain_arg(self, text: impl std::fmt::Display) -> Msg {
        self.arg(Text::literal(text.to_string()))
    }

    pub fn text(&self) -> Text {
        Text::translate_with_fallback(self.key, english(self.key), &self.args)
    }

    /// The English reading.
    pub fn plain(&self) -> String {
        fill(english(self.key), &self.args.iter().map(Text::plain).collect::<Vec<_>>())
    }
}

impl PartialEq for Msg {
    fn eq(&self, other: &Msg) -> bool {
        self.key == other.key && self.plain() == other.plain()
    }
}

impl std::fmt::Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.plain())
    }
}

impl From<Msg> for Text {
    fn from(m: Msg) -> Text {
        m.text()
    }
}

/// A shared `Arg` as text: a lang key filled with its arguments, in the reader's language.
pub fn arg_text(arg: &Arg) -> Text {
    match arg {
        Arg::Plain(s) => Text::literal(s.clone()),
        Arg::Lang { key, en, args } => Text::translate_with_fallback(key, en, &args.iter().map(arg_text).collect::<Vec<_>>()),
    }
}

/// An organ's name in the language of the reader: the game's item name, the English as a fallback.
pub fn item_name(id: &str, english: &str) -> Text {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    Text::translate_with_fallback(&format!("item.{ns}.{path}"), english, &[])
}

/// An ability's name.
pub fn ability_name(name: &str) -> Text {
    let key = format!("chestcavity.ability.{name}");
    Text::translate_with_fallback(&key, english_of(&key).unwrap_or(name), &[])
}

fn english_of(key: &str) -> Option<&'static str> {
    ALL.iter().find(|(k, _)| *k == key).map(|e| e.1)
}

/// A mob's name in the language of the reader: the game's, and its type's as a fallback ("Cave Spider").
pub fn item_name_of_mob(mob: &str) -> Text {
    Text::translate_with_fallback(&format!("entity.{}", mob.replace(':', ".")), &chestcavity_shared::cavity::mob_name(mob), &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chestcavity_shared::lang::{arguments, parse};

    /// Every message is in every language, with the arguments it needs.
    #[test]
    fn every_message_is_in_every_language() {
        let (en, pt, br) = (
            parse(include_str!("../../../assets/chestcavity/lang/en_us.json")),
            parse(include_str!("../../../assets/chestcavity/lang/pt_pt.json")),
            parse(include_str!("../../../assets/chestcavity/lang/pt_br.json")),
        );
        let get = |l: &[(String, String)], k: &str| l.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        for &(key, english) in ALL {
            assert_eq!(get(&en, key).as_deref(), Some(english), "en_us has the English of {key}");
            let local = get(&pt, key).unwrap_or_else(|| panic!("pt_pt lacks {key}"));
            assert!(!local.is_empty() && arguments(&local) == arguments(english), "{key}: {local:?} takes other arguments than {english:?}");
            assert_eq!(get(&br, key), Some(local));
        }
    }

    #[test]
    fn a_message_reads_in_english_with_its_arguments() {
        assert_eq!(msg("chestcavity.msg.resting").arg(ability_name("silk")).plain_arg(3).plain(), "Silk is still recovering (3 s)");
        assert_eq!(msg("chestcavity.title.mob").arg(item_name_of_mob("minecraft:cave_spider")).plain(), "Cave Spider's Chest Cavity");
    }
}
