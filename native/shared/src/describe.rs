//! What an organ is good and bad at, from its scores: what it improves, what it costs, what happens without
//! it and what ability it gives. Numbers for the scores that change an attribute come from `effects`, so the
//! text says what the game really does; what the port does not do yet (most abilities) is marked `idle`.
//! The words are not here: a line is a template (a lang key) and its arguments, and the game fills it in the
//! language of whoever reads it. The English given with each key is what shows when a language lacks it, and
//! what the tests read (`Arg::plain`).
use crate::cavity::Organ;
use crate::effects;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Better,
    Worse,
    Ability,
    Note,
}

/// Text to show: plain, or a lang key with its English and the arguments its template takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Plain(String),
    Lang { key: String, en: String, args: Vec<Arg> },
}

impl Arg {
    fn lang(key: impl Into<String>, en: impl Into<String>, args: Vec<Arg>) -> Arg {
        Arg::Lang { key: key.into(), en: en.into(), args }
    }

    /// The English reading, as the game would give it with no language to help.
    pub fn plain(&self) -> String {
        match self {
            Arg::Plain(s) => s.clone(),
            Arg::Lang { en, args, .. } => fill(en, &args.iter().map(Arg::plain).collect::<Vec<_>>()),
        }
    }
}

/// `%s`, `%1$s` and `%%` of a template, as Minecraft fills them.
pub fn fill(template: &str, args: &[String]) -> String {
    let (mut out, mut next) = (String::new(), 0);
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut digits = String::new();
        while let Some(d) = chars.peek().copied().filter(char::is_ascii_digit) {
            digits.push(d);
            chars.next();
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('$') => {
                chars.next(); // the `s`
                out.push_str(digits.parse::<usize>().ok().and_then(|n| args.get(n.wrapping_sub(1))).map_or("", String::as_str));
            }
            Some(_) => {
                out.push_str(args.get(next).map_or("", String::as_str));
                next += 1;
            }
            None => {}
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub tone: Tone,
    pub text: Arg,
    /// The port gives this no effect yet.
    pub idle: bool,
}

/// The templates of a line, by lang key, with their English.
pub const VALUE: (&str, &str) = ("chestcavity.fmt.value", "%1$s: %2$s");
pub const SAID: (&str, &str) = ("chestcavity.fmt.said", "%1$s %2$s: %3$s");
pub const ABILITY_LINE: (&str, &str) = ("chestcavity.fmt.ability", "%1$s: %2$s%3$s");
pub const LEVEL: (&str, &str) = ("chestcavity.fmt.level", " (level %s)");
pub const REDUCED: (&str, &str) = ("chestcavity.fmt.reduced", "%s reduced");
pub const CONSEQUENCE: (&str, &str) = ("chestcavity.fmt.consequence", "Consequence: %s");
pub const STACK: (&str, &str) = ("chestcavity.fmt.stack", "Full effect with %s in the stack; with fewer, proportional");
pub const IDLE: (&str, &str) = ("chestcavity.fmt.idle", " (no effect yet)");
/// Every template, for checking the lang files.
pub const TEMPLATES: [(&str, &str); 8] = [VALUE, SAID, ABILITY_LINE, LEVEL, REDUCED, CONSEQUENCE, STACK, IDLE];

fn tpl(t: (&str, &str), args: Vec<Arg>) -> Arg {
    Arg::lang(t.0, t.1, args)
}

/// How a score reads.
enum Fx {
    /// A plain amount of an attribute: value times the rate in `effects`.
    Add,
    /// A share of an attribute: value times the rate, as a percentage.
    Pct,
    /// A sentence for a higher value, and one for a lower (empty: "<name> reduzido").
    Said(&'static str, &'static str),
}
use Fx::{Add, Pct, Said};

/// The port applies it.
const LIVE: u8 = 1;
/// A drawback even when the value is positive.
const BAD: u8 = 2;
/// An ability, not a number.
const ABILITY: u8 = 4;
/// Its value is a size (an explosion's), not a level.
const BIG: u8 = 8;

struct Info {
    key: &'static str,
    name: &'static str,
    fx: Fx,
    flags: u8,
    /// What happens to a body left without it, when the port does that.
    missing: &'static str,
}

const fn i(key: &'static str, name: &'static str, fx: Fx, flags: u8, missing: &'static str) -> Info {
    Info { key, name, fx, flags, missing }
}

static INFO: &[Info] = &[
    // O corpo, o que o jogo aplica.
    i("health", "Max health", Add, LIVE, "without a heart you bleed (damage every second, growing)"),
    i("strength", "Melee damage", Pct, LIVE, ""),
    i("speed", "Movement speed", Pct, LIVE, ""),
    i("nerves", "Attack and mining speed", Pct, LIVE, "without nerves (a spine) you are paralysed"),
    i("luck", "Luck", Add, LIVE, ""),
    i("defense", "Bone defense", Said("less damage from blows armour would take (every 4 points: half)", "more damage from blows armour would take"), LIVE, ""),
    i("leaping", "Jump height", Pct, LIVE, ""),
    i("swim_speed", "Swimming efficiency", Pct, LIVE, ""),
    i("knockback_resistant", "Knockback resistance", Pct, LIVE, ""),
    i("knockback_resistance", "Knockback resistance", Pct, LIVE, ""),
    i("block_interaction_range", "Block reach", Add, LIVE, ""),
    i("entity_interaction_range", "Attack reach", Add, LIVE, ""),
    i("lightweight", "Lightness", Said("you fall slower (less gravity)", "you are heavier (you fall faster)"), LIVE, ""),
    i("ease_of_access", "Easy access", Said("opening your chest costs no damage and works even at full health", ""), LIVE | ABILITY, ""),
    i("breath_capacity", "Lung capacity", Said("your breath lasts longer (2 is normal)", "your breath lasts less"), LIVE, "with no capacity at all the air runs out 20 times faster"),
    i("filtration", "Filtration", Said("filters the blood", "filters the blood worse"), LIVE, "with less filtration than normal you get poisoned from time to time"),
    i("breath_recovery", "Breathing", Said("you recover air out of water (2 is normal)", "you recover air worse"), LIVE, "without lungs you drown out of water"),
    i("water_breath", "Water breathing", Said("you breathe under water (from 0.5 the air no longer drops)", "you breathe worse under water"), LIVE, ""),
    i("fire_resistant", "Fire resistance", Said("less fire and lava damage (every 4 points: -75%)", "more fire damage"), LIVE, ""),
    i("glowing", "Glowing", Said("you glow (visible through walls)", ""), LIVE | ABILITY, ""),
    // O corpo, o que o port ainda não aplica.
    i("endurance", "Endurance", Said("exercise makes you less hungry", "exercise makes you hungrier"), LIVE, ""),
    i("metabolism", "Metabolism", Said("you heal faster at the cost of food", "you heal slower"), LIVE, ""),
    i("digestion", "Digestion", Said("food restores more hunger (1 is normal)", "food restores less hunger"), LIVE, "without a stomach food makes you sick"),
    i("nutrition", "Nutrition", Said("food gives more saturation (4 is normal)", "food gives less saturation"), LIVE, "without intestines food leaves you hungry"),
    i("detoxification", "Detoxification", Said("harmful potion effects last less", "harmful effects last longer"), 0, ""),
    i("fire_resistance", "Fire protection", Said("less fire damage; immune to hot ground at 5, to fire at 10 and to lava at 15", "more fire damage"), LIVE, ""),
    i("frost_resistance", "Frost protection", Said("less freezing damage; immune at 5", "more freezing damage"), LIVE, ""),
    i("impact_resistant", "Impact resistance", Said("less damage from falls and wall crashes (every 4 points: -75%)", "more impact damage"), LIVE, ""),
    i("buoyant", "Buoyancy", Said("you float while you have air in your lungs (the Exhale key lets the air out)", "you sink faster"), 0, ""),
    i("neutral_water_buoyant", "Neutral buoyancy", Said("you float slowly in water and lava", ""), 0, ""),
    // Comer.
    i("herbivorous_digestion", "Plant digestion", Said("plants restore more hunger", "plants restore less hunger"), LIVE, ""),
    i("herbivorous_nutrition", "Plant nutrition", Said("plants give more saturation", "plants give less saturation"), LIVE, ""),
    i("carnivorous_digestion", "Meat digestion", Said("meat restores more hunger", "meat restores less hunger"), LIVE, ""),
    i("carnivorous_nutrition", "Meat nutrition", Said("meat gives more saturation", "meat gives less saturation"), LIVE, ""),
    i("scavenger_digestion", "Rot digestion", Said("rotten food restores more hunger (no poison or hunger)", ""), LIVE, ""),
    i("scavenger_nutrition", "Rot nutrition", Said("rotten food gives more saturation", ""), LIVE, ""),
    i("rot_digestion", "Rot digestion", Said("you make use of rotten food", ""), LIVE | ABILITY, ""),
    i("rotgut", "Rotgut", Said("rotten food nourishes you", ""), LIVE | ABILITY, ""),
    i("iron_gut", "Iron gut", Said("food applies no harmful effects to you", ""), ABILITY, ""),
    i("photosynthesis", "Photosynthesis", Said("in daylight you recover hunger and health", ""), LIVE | ABILITY, ""),
    i("crystalsynthesis", "Crystalsynthesis", Said("near End Crystals you recover hunger and health (if the crystal explodes, you suffer)", ""), LIVE | ABILITY, ""),
    i("crystallization", "Crystallization", Said("near End Crystals you regenerate health", ""), ABILITY, ""),
    // Habilidades ativas (com tecla).
    i("grazing", "Grazing", Said("you eat the grass under you (active, key V or /cc ability grazing)", ""), LIVE | ABILITY, ""),
    i("eat_the_rich", "Block grazing", Said("you chew blocks instead of grass (active)", ""), ABILITY, ""),
    i("iron_repair", "Iron repair", Said("you spend an iron ingot to recover a quarter of your health (active, key V or /cc ability iron_repair)", ""), LIVE | ABILITY, ""),
    i("furnace_powered", "Inner furnace", Said("you burn the fuel in your hand to gain energy (active)", ""), ABILITY, ""),
    i("silk", "Silk", Said("you spin webs and string behind you (active, key V or /cc ability silk)", ""), LIVE | ABILITY, ""),
    i("pyromancy", "Pyromancy", Said("you spit fireballs, one per point (active, key R or /cc ability pyromancy)", ""), LIVE | ABILITY, ""),
    i("ghastly", "Ghastly", Said("you fire ghast fireballs, one per point (active, key R or /cc ability ghastly)", ""), LIVE | ABILITY, ""),
    i("shulker_bullets", "Shulker bullets", Said("you fire homing bullets (active)", ""), ABILITY, ""),
    i("forceful_spit", "Forceful spit", Said("you spit projectiles with force, one per point (active, key R or /cc ability forceful_spit)", ""), LIVE | ABILITY, ""),
    i("dragon_breath", "Dragon breath", Said("you spit dragon breath (active)", ""), ABILITY, ""),
    i("dragon_bombs", "Dragon bombs", Said("you throw dragon fireballs, one per point (active, key R or /cc ability dragon_bombs)", ""), LIVE | ABILITY, ""),
    i("creepy", "Creepy", Said("you explode like a creeper, as hard as your explosive organs, which are used up (active, key R or /cc ability creepy)", ""), LIVE | ABILITY, ""),
    i("explosive", "Explosive", Said("adds to the power of the creeper appendix explosion", ""), LIVE | ABILITY | BIG, ""),
    i("flight", "Flight", Said("you can fly as in creative (double jump)", ""), LIVE | ABILITY, ""),
    // Ao atacar e ao ser atacado.
    i("venomous", "Venomous", Said("your bare-handed punch poisons the target", ""), LIVE | ABILITY, ""),
    i("launching", "Launching", Said("your blows throw the target into the air", ""), LIVE | ABILITY, ""),
    i("arrow_dodging", "Arrow dodging", Said("you teleport away from arrows and projectiles (rests 10 s)", ""), LIVE | ABILITY, ""),
    i("withered", "Wither resistance", Said("the Wither effect lasts less", ""), ABILITY, ""),
    i("panic", "Panic", Said("when you take damage you panic (temporary effect)", ""), ABILITY, ""),
    i("resonance", "Resonance", Said("when you are hit you release a resonant blast", ""), ABILITY, ""),
    i("shrieking", "Shrieking", Said("when you are hit you release a sculk shriek wave", ""), ABILITY, ""),
    i("cluster_explode", "Cluster explosion", Said("when you are hit you explode into cluster bombs", ""), ABILITY, ""),
    // Os órgãos que mexem nos órgãos.
    i("regrowth", "Organ regrowth", Said("lost organs grow back", ""), ABILITY, ""),
    i("autophagy", "Autophagy", Said("you consume your own organs to feed yourself", ""), ABILITY, ""),
    i("tumor_autophagy", "Tumor autophagy", Said("you consume tumors to feed yourself", ""), ABILITY, ""),
    i("tumor_hunting", "Tumor hunting", Said("you seek out and consume tumors", ""), ABILITY, ""),
    i("sculk_infection", "Sculk infection", Said("you infect organs with sculk", ""), ABILITY, ""),
    // O que custa.
    i("buff_purging", "Buff purging", Said("beneficial effects last less", ""), BAD, ""),
    i("hydroallergenic", "Hydroallergy", Said("you take damage touching water or rain (much more when submerged)", ""), LIVE | BAD, ""),
    i("water_allergy", "Water allergy", Said("you take damage in water or rain", ""), BAD, ""),
    i("water_weakness", "Water weakness", Said("less attack and mining speed in water", ""), BAD, ""),
    i("hydrophobia", "Hydrophobia", Said("in water or rain you are teleported at random", ""), LIVE | BAD, ""),
];

/// Whether the text knows the score `key`.
pub fn knows(key: &str) -> bool {
    INFO.iter().any(|i| i.key == key)
}

/// A number the way a player reads it: two decimals at most. (The decimal separator is the language's job: the
/// game's translation does not touch numbers, so a comma goes in `chestcavity.fmt.*` only if a language wants it.)
fn num(x: f64) -> String {
    format!("{}", (x * 100.0).round() / 100.0)
}

fn signed(x: f64) -> String {
    if x < 0.0 { format!("-{}", num(-x)) } else { format!("+{}", num(x)) }
}

/// The text a score says about itself: its lang key and its English.
fn say(key: &str, what: &str, en: &str) -> Arg {
    Arg::lang(format!("chestcavity.score.{key}.{what}"), en, Vec::new())
}

/// What `organ` is good and bad at, best first: what it improves, what it costs, its abilities, then what
/// happens without it and how its stack counts.
pub fn describe(organ: &Organ) -> Vec<Line> {
    let (mut better, mut worse, mut abilities, mut notes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for &(key, value) in organ.scores {
        let v = f64::from(value);
        let Some(info) = INFO.iter().find(|i| i.key == key) else {
            notes.push(Line { tone: Tone::Note, text: tpl(VALUE, vec![Arg::Plain(key.to_string()), Arg::Plain(num(v))]), idle: true });
            continue;
        };
        let live = info.flags & LIVE != 0;
        let name = say(key, "name", info.name);
        let text = match info.fx {
            Add => tpl(VALUE, vec![name, Arg::Plain(signed(v * effects::rate(key).unwrap_or(1.0)))]),
            Pct => tpl(VALUE, vec![name, Arg::Plain(format!("{}%", signed(v * effects::rate(key).unwrap_or(1.0) * 100.0)))]),
            Said(up, down) if info.flags & ABILITY != 0 => {
                let level = if v > 1.0 && info.flags & BIG == 0 { tpl(LEVEL, vec![Arg::Plain(num(v))]) } else { Arg::Plain(String::new()) };
                let said = if v >= 0.0 || down.is_empty() { say(key, "up", up) } else { say(key, "down", down) };
                tpl(ABILITY_LINE, vec![name, said, level])
            }
            Said(up, down) => {
                let said = if v >= 0.0 {
                    say(key, "up", up)
                } else if down.is_empty() {
                    tpl(REDUCED, vec![say(key, "name", info.name)])
                } else {
                    say(key, "down", down)
                };
                tpl(SAID, vec![name, Arg::Plain(signed(v)), said])
            }
        };
        let tone = if info.flags & BAD != 0 {
            Tone::Worse
        } else if info.flags & ABILITY != 0 {
            Tone::Ability
        } else if v >= 0.0 {
            Tone::Better
        } else {
            Tone::Worse
        };
        let line = Line { tone, text, idle: !live };
        match tone {
            Tone::Better => better.push(line),
            Tone::Worse => worse.push(line),
            _ => abilities.push(line),
        }
        if live && v > 0.0 && !info.missing.is_empty() {
            notes.push(Line { tone: Tone::Note, text: tpl(CONSEQUENCE, vec![say(key, "missing", info.missing)]), idle: false });
        }
    }
    if organ.stack > 1 {
        notes.push(Line { tone: Tone::Note, text: tpl(STACK, vec![Arg::Plain(organ.stack.to_string())]), idle: false });
    }
    notes.dedup();
    [better, worse, abilities, notes].concat()
}

/// The lang keys the scores' texts use, with their English: name, up, down and missing of each.
pub fn score_texts() -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for i in INFO {
        out.push((format!("chestcavity.score.{}.name", i.key), i.name));
        if let Said(up, down) = &i.fx {
            out.push((format!("chestcavity.score.{}.up", i.key), *up));
            if !down.is_empty() {
                out.push((format!("chestcavity.score.{}.down", i.key), *down));
            }
        }
        if !i.missing.is_empty() {
            out.push((format!("chestcavity.score.{}.missing", i.key), i.missing));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::organ;
    use crate::organs::ORGANS;

    fn text(lines: &[Line], tone: Tone) -> Vec<String> {
        lines.iter().filter(|l| l.tone == tone).map(|l| l.text.plain()).collect()
    }

    #[test]
    fn every_score_of_every_organ_is_described() {
        for o in ORGANS {
            for (key, _) in o.scores {
                assert!(knows(key), "{} has the score {key}, which describe.rs doesn't know", o.id);
            }
        }
    }

    #[test]
    fn the_scores_that_move_an_attribute_have_a_rate() {
        for info in INFO.iter().filter(|i| matches!(i.fx, Add | Pct)) {
            assert!(effects::rate(info.key).is_some(), "{} says a number but effects has no rate for it", info.key);
            assert!(info.flags & LIVE != 0, "{} moves an attribute, so the port applies it", info.key);
        }
    }

    #[test]
    fn numbers_come_from_the_game_and_the_rest_says_it_is_idle() {
        // A full stack of muscle: +100% damage from strength 8 (a share of 1/8 a point), the speed it gives.
        let muscle = organ("chestcavity:muscle").expect("muscle");
        let lines = describe(muscle);
        let better = text(&lines, Tone::Better);
        assert!(better.iter().any(|l| l.starts_with("Melee damage: +")), "{better:?}");
        assert!(better.iter().any(|l| l.starts_with("Movement speed: +")), "{better:?}");
        assert!(lines.iter().any(|l| l.text.plain().starts_with("Full effect with")), "a stack says how it counts");

        // The heart is what keeps you from bleeding; its text says so, and it is applied.
        let heart = describe(organ("chestcavity:heart").expect("heart"));
        assert!(text(&heart, Tone::Better).iter().any(|l| l.starts_with("Max health: +")));
        assert!(text(&heart, Tone::Note).iter().any(|l| l.contains("without a heart you bleed")));
        assert!(heart.iter().filter(|l| l.tone == Tone::Better).all(|l| !l.idle));

        // An ability the port does not give yet says so.
        let bullets = ORGANS.iter().find(|o| o.scores.iter().any(|s| s.0 == "shulker_bullets")).expect("a shulker organ");
        let lines = describe(bullets);
        assert!(lines.iter().any(|l| l.tone == Tone::Ability && l.idle && l.text.plain().starts_with("Shulker bullets:")), "{lines:?}");
        // And one it does give says nothing of the kind.
        let silk = ORGANS.iter().find(|o| o.scores.iter().any(|s| s.0 == "silk")).expect("a silk organ");
        assert!(describe(silk).iter().filter(|l| l.tone == Tone::Ability).all(|l| !l.idle));
    }

    #[test]
    fn what_costs_reads_as_worse_even_when_positive() {
        let kidney = ORGANS.iter().find(|o| o.id == "chestcavity:ender_kidney").expect("ender kidney");
        let lines = describe(kidney);
        let worse = text(&lines, Tone::Worse);
        assert!(worse.iter().any(|l| l.starts_with("Hydroallergy")), "{worse:?}");
        assert!(worse.iter().any(|l| l.starts_with("Hydrophobia")), "{worse:?}");
    }

    #[test]
    fn numbers_read_with_two_decimals_at_most() {
        assert_eq!(signed(12.5), "+12.5");
        assert_eq!(signed(-0.1), "-0.1");
        assert_eq!(signed(4.0), "+4");
        assert_eq!(signed(1.0 / 3.0), "+0.33");
    }

    #[test]
    fn templates_fill_like_minecrafts() {
        let args = ["a".to_string(), "b".to_string()];
        assert_eq!(fill("%1$s: %2$s", &args), "a: b");
        assert_eq!(fill("%s-%s 100%%", &args), "a-b 100%");
        assert_eq!(fill("%2$s %1$s", &args), "b a");
    }
}
