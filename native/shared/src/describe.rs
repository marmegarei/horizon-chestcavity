//! What an organ is good and bad at, in words (Portuguese), from its scores: what it improves, what it
//! costs, what happens without it and what ability it gives. Numbers for the scores that change an
//! attribute come from `effects`, so the text says what the game really does; what the port does not do
//! yet (most abilities) is marked `idle`.
use crate::cavity::Organ;
use crate::effects;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Better,
    Worse,
    Ability,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub tone: Tone,
    pub text: String,
    /// The port gives this no effect yet.
    pub idle: bool,
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
    i("health", "Vida máxima", Add, LIVE, "sem coração sangras (dano por segundo, cada vez maior)"),
    i("strength", "Dano corpo a corpo", Pct, LIVE, ""),
    i("speed", "Velocidade de movimento", Pct, LIVE, ""),
    i("nerves", "Velocidade de ataque e de mineração", Pct, LIVE, "sem nervos (espinha) ficas paralisado"),
    i("luck", "Sorte", Add, LIVE, ""),
    i("defense", "Armadura", Add, LIVE, ""),
    i("leaping", "Altura do salto", Pct, LIVE, ""),
    i("swim_speed", "Eficiência a nadar", Pct, LIVE, ""),
    i("knockback_resistant", "Resistência a recuo", Pct, LIVE, ""),
    i("breath_capacity", "Capacidade de ar", Add, LIVE, ""),
    i("filtration", "Filtração", Said("filtra o sangue", "filtra pior o sangue"), LIVE, "com menos filtração do que o normal envenenas-te de tempos a tempos"),
    i("breath_recovery", "Respiração", Said("respira ar normalmente", "recupera pior o ar"), LIVE, "sem pulmões (nem respiração aquática) afogas fora de água"),
    i("water_breath", "Respiração aquática", Said("respira debaixo de água", "respira pior debaixo de água"), LIVE, ""),
    i("fire_resistant", "Resistência ao fogo", Said("ficas resistente ao fogo", "ficas menos resistente ao fogo"), LIVE, ""),
    i("glowing", "Brilho", Said("brilhas (visível através das paredes)", ""), LIVE | ABILITY, ""),
    // O corpo, o que o port ainda não aplica.
    i("endurance", "Resistência", Said("gastas a fome mais devagar", "gastas a fome mais depressa"), 0, ""),
    i("metabolism", "Metabolismo", Said("curas mais depressa à custa da fome", "curas mais devagar"), 0, ""),
    i("digestion", "Digestão", Said("a comida restaura mais fome", "a comida restaura menos fome"), 0, ""),
    i("nutrition", "Nutrição", Said("a comida dá mais saciedade", "a comida dá menos saciedade"), 0, ""),
    i("detoxification", "Desintoxicação", Said("os efeitos de poção nocivos duram menos", "os efeitos nocivos duram mais"), 0, ""),
    i("lightweight", "Leveza", Said("caís mais devagar (menos gravidade)", "és mais pesado (caís mais depressa)"), 0, ""),
    i("fire_resistance", "Proteção contra fogo", Said("menos dano de fogo; com valor alto, imune a blocos quentes, fogo e lava", "mais dano de fogo"), 0, ""),
    i("frost_resistance", "Proteção contra frio", Said("menos dano de congelação", "mais dano de congelação"), 0, ""),
    i("impact_resistant", "Resistência a impactos", Said("menos dano de impacto (quedas e colisões)", "mais dano de impacto"), 0, ""),
    i("knockback_resistance", "Resistência a recuo", Said("menos recuo ao seres atingido", "mais recuo"), 0, ""),
    i("buoyant", "Flutuação", Said("flutuas enquanto tiveres ar nos pulmões (a tecla Exalar liberta o ar)", "afundas mais depressa"), 0, ""),
    i("neutral_water_buoyant", "Flutuação neutra", Said("flutuas devagar na água e na lava", ""), 0, ""),
    i("block_interaction_range", "Alcance de blocos", Said("alcanças blocos mais longe", "alcanças blocos menos longe"), 0, ""),
    i("entity_interaction_range", "Alcance de ataque", Said("alcanças criaturas mais longe", "alcanças criaturas menos longe"), 0, ""),
    // Comer.
    i("herbivorous_digestion", "Digestão de plantas", Said("as plantas restauram mais fome", "as plantas restauram menos fome"), 0, ""),
    i("herbivorous_nutrition", "Nutrição de plantas", Said("as plantas dão mais saciedade", "as plantas dão menos saciedade"), 0, ""),
    i("carnivorous_digestion", "Digestão de carne", Said("a carne restaura mais fome", "a carne restaura menos fome"), 0, ""),
    i("carnivorous_nutrition", "Nutrição de carne", Said("a carne dá mais saciedade", "a carne dá menos saciedade"), 0, ""),
    i("scavenger_digestion", "Digestão de podre", Said("a comida podre restaura mais fome (sem veneno nem fome)", ""), 0, ""),
    i("scavenger_nutrition", "Nutrição de podre", Said("a comida podre dá mais saciedade", ""), 0, ""),
    i("rot_digestion", "Digestão de podre", Said("aproveitas comida podre", ""), ABILITY, ""),
    i("rotgut", "Intestino de podre", Said("a comida podre alimenta-te", ""), ABILITY, ""),
    i("iron_gut", "Estômago de ferro", Said("a comida não te aplica efeitos nocivos", ""), ABILITY, ""),
    i("photosynthesis", "Fotossíntese", Said("à luz do dia recuperas fome e vida", ""), ABILITY, ""),
    i("crystalsynthesis", "Cristalossíntese", Said("perto de cristais do End recuperas fome e vida (se o cristal explodir, sofres)", ""), ABILITY, ""),
    i("crystallization", "Cristalização", Said("perto de cristais do End regeneras vida", ""), ABILITY, ""),
    // Habilidades ativas (com tecla).
    i("grazing", "Pastar", Said("comes a relva debaixo de ti e ruminas (ativa)", ""), ABILITY, ""),
    i("eat_the_rich", "Ruminar blocos", Said("ruminas blocos em vez de relva (ativa)", ""), ABILITY, ""),
    i("iron_repair", "Reparação com ferro", Said("gastas lingotes de ferro para te curares (ativa)", ""), ABILITY, ""),
    i("furnace_powered", "Fornalha interior", Said("queimas o combustível da mão para ganhar energia (ativa)", ""), ABILITY, ""),
    i("silk", "Seda", Said("teces teias (ativa)", ""), ABILITY, ""),
    i("pyromancy", "Piromancia", Said("cospes bolas de fogo (ativa)", ""), ABILITY, ""),
    i("ghastly", "Ghastly", Said("disparas bolas de fogo de ghast (ativa)", ""), ABILITY, ""),
    i("shulker_bullets", "Balas de shulker", Said("disparas balas teleguiadas (ativa)", ""), ABILITY, ""),
    i("forceful_spit", "Cuspo forte", Said("cospes projéteis com força (ativa)", ""), ABILITY, ""),
    i("dragon_breath", "Sopro de dragão", Said("cospes sopro de dragão (ativa)", ""), ABILITY, ""),
    i("dragon_bombs", "Bombas de dragão", Said("lanças bombas de dragão (ativa)", ""), ABILITY, ""),
    i("creepy", "Creepy", Said("explodes como um creeper, com a explosividade dos órgãos (ativa)", ""), ABILITY, ""),
    i("explosive", "Explosivo", Said("aumenta a potência da explosão do apêndice de creeper", ""), ABILITY | BIG, ""),
    i("flight", "Voo", Said("podes voar (alterna com a tecla)", ""), ABILITY, ""),
    // Ao atacar e ao ser atacado.
    i("venomous", "Venenoso", Said("o teu golpe envenena o alvo", ""), ABILITY, ""),
    i("launching", "Lançamento", Said("o teu golpe lança o alvo para o ar", ""), ABILITY, ""),
    i("arrow_dodging", "Esquiva de flechas", Said("teletransportas-te para esquivar flechas e projéteis", ""), ABILITY, ""),
    i("withered", "Resistência a Wither", Said("o efeito Wither dura menos", ""), ABILITY, ""),
    i("panic", "Pânico", Said("ao sofreres dano entras em pânico (efeito temporário)", ""), ABILITY, ""),
    i("resonance", "Ressonância", Said("ao seres atingido, soltas uma explosão ressonante", ""), ABILITY, ""),
    i("shrieking", "Grito", Said("ao seres atingido, soltas uma onda de grito sculk", ""), ABILITY, ""),
    i("cluster_explode", "Explosão em cacho", Said("ao seres atingido, explodes em bombas de fragmentação", ""), ABILITY, ""),
    // Os órgãos que mexem nos órgãos.
    i("regrowth", "Regeneração de órgãos", Said("os órgãos perdidos voltam a crescer", ""), ABILITY, ""),
    i("autophagy", "Autofagia", Said("consomes os teus órgãos para te alimentares", ""), ABILITY, ""),
    i("tumor_autophagy", "Autofagia de tumores", Said("consomes tumores para te alimentares", ""), ABILITY, ""),
    i("tumor_hunting", "Caça-tumores", Said("procuras e consomes tumores", ""), ABILITY, ""),
    i("sculk_infection", "Infeção sculk", Said("infetas os órgãos com sculk", ""), ABILITY, ""),
    // O que custa.
    i("buff_purging", "Purga de efeitos", Said("os efeitos positivos duram menos", ""), BAD, ""),
    i("hydroallergenic", "Hidroalergia", Said("levas dano ao tocar em água ou chuva (muito mais submerso)", ""), BAD, ""),
    i("water_allergy", "Alergia à água", Said("levas dano em água ou chuva", ""), BAD, ""),
    i("water_weakness", "Fraqueza na água", Said("menos ataque e mineração dentro de água", ""), BAD, ""),
    i("hydrophobia", "Hidrofobia", Said("na água ou chuva és teletransportado ao acaso", ""), BAD, ""),
];

/// Whether the text knows the score `key`.
pub fn knows(key: &str) -> bool {
    INFO.iter().any(|i| i.key == key)
}

/// A number the way a Portuguese player reads it: two decimals at most, a comma.
fn num(x: f64) -> String {
    format!("{}", (x * 100.0).round() / 100.0).replace('.', ",")
}

fn signed(x: f64) -> String {
    if x < 0.0 { format!("-{}", num(-x)) } else { format!("+{}", num(x)) }
}

/// What `organ` is good and bad at, best first: what it improves, what it costs, its abilities, then what
/// happens without it and how its stack counts.
pub fn describe(organ: &Organ) -> Vec<Line> {
    let (mut better, mut worse, mut abilities, mut notes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for &(key, value) in organ.scores {
        let v = f64::from(value);
        let Some(info) = INFO.iter().find(|i| i.key == key) else {
            notes.push(Line { tone: Tone::Note, text: format!("{key}: {}", num(v)), idle: true });
            continue;
        };
        let live = info.flags & LIVE != 0;
        let text = match info.fx {
            Add => format!("{}: {}", info.name, signed(v * effects::rate(key).unwrap_or(1.0))),
            Pct => format!("{}: {}%", info.name, signed(v * effects::rate(key).unwrap_or(1.0) * 100.0)),
            Said(up, down) if info.flags & ABILITY != 0 => {
                let level = if v > 1.0 && info.flags & BIG == 0 { format!(" (nível {})", num(v)) } else { String::new() };
                format!("{}: {}{level}", info.name, if v >= 0.0 || down.is_empty() { up } else { down })
            }
            Said(up, down) => {
                let said = if v >= 0.0 { up.to_string() } else if down.is_empty() { format!("{} reduzido", info.name) } else { down.to_string() };
                format!("{} {}: {said}", info.name, signed(v))
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
            notes.push(Line { tone: Tone::Note, text: format!("Consequência: {}", info.missing), idle: false });
        }
    }
    if organ.stack > 1 {
        notes.push(Line { tone: Tone::Note, text: format!("Efeito completo com {} na pilha; com menos, proporcional", organ.stack), idle: false });
    }
    notes.dedup();
    [better, worse, abilities, notes].concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::organ;
    use crate::organs::ORGANS;

    fn text(lines: &[Line], tone: Tone) -> Vec<&str> {
        lines.iter().filter(|l| l.tone == tone).map(|l| l.text.as_str()).collect()
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
        assert!(better.iter().any(|l| l.starts_with("Dano corpo a corpo: +")), "{better:?}");
        assert!(better.iter().any(|l| l.starts_with("Velocidade de movimento: +")), "{better:?}");
        assert!(lines.iter().any(|l| l.text.starts_with("Efeito completo com")), "a stack says how it counts");

        // The heart is what keeps you from bleeding; its text says so, and it is applied.
        let heart = describe(organ("chestcavity:heart").expect("heart"));
        assert!(text(&heart, Tone::Better).iter().any(|l| l.starts_with("Vida máxima: +")));
        assert!(text(&heart, Tone::Note).iter().any(|l| l.contains("sem coração sangras")));
        assert!(heart.iter().filter(|l| l.tone == Tone::Better).all(|l| !l.idle));

        // An ability the port does not give yet says so.
        let silk = ORGANS.iter().find(|o| o.scores.iter().any(|s| s.0 == "silk")).expect("a silk organ");
        let lines = describe(silk);
        assert!(lines.iter().any(|l| l.tone == Tone::Ability && l.idle && l.text.starts_with("Seda:")), "{lines:?}");
    }

    #[test]
    fn what_costs_reads_as_worse_even_when_positive() {
        let kidney = ORGANS.iter().find(|o| o.id == "chestcavity:ender_kidney").expect("ender kidney");
        let lines = describe(kidney);
        let worse = text(&lines, Tone::Worse);
        assert!(worse.iter().any(|l| l.starts_with("Hidroalergia")), "{worse:?}");
        assert!(worse.iter().any(|l| l.starts_with("Hidrofobia")), "{worse:?}");
    }

    #[test]
    fn numbers_read_as_a_portuguese_player_reads_them() {
        assert_eq!(signed(12.5), "+12,5");
        assert_eq!(signed(-0.1), "-0,1");
        assert_eq!(signed(4.0), "+4");
    }
}
