//! The mod's settings: the values of Chest Cavity's `CCConfig` that a player may want to change, read from
//! `config/chestcavity.json`. A line `"name": value` each; what is missing or unreadable keeps its default.

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Organs from another body are rejected (`!DISABLE_ORGAN_REJECTION`).
    pub organ_rejection: bool,
    /// Ticks between rejections at one point of incompatibility, and what each costs (`ORGAN_REJECTION_RATE`, `_DAMAGE`).
    pub rejection_rate: f32,
    pub rejection_damage: f32,
    /// The chest cavity survives death (`KEEP_CHEST_CAVITY`).
    pub keep_chest_cavity: bool,
    /// Health at or below which a mob's chest can be opened, absolutely and as a share of its maximum.
    pub opener_absolute_health: f32,
    pub opener_fractional_health: f32,
    /// The chance an organ of a mob's cavity is a universal donor's (`UNIVERSAL_DONOR_RATE`).
    pub universal_donor_rate: f32,
    /// What each level of Looting adds to the chance of organs (`ORGAN_BUNDLE_LOOTING_BOOST`).
    pub looting_boost: f32,
    /// Multiplies how long abilities rest (1: as in Chest Cavity).
    pub cooldown_scale: f32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            organ_rejection: true,
            rejection_rate: 600.0,
            rejection_damage: 2.0,
            keep_chest_cavity: false,
            opener_absolute_health: 20.0,
            opener_fractional_health: 0.5,
            universal_donor_rate: 0.1,
            looting_boost: 0.04,
            cooldown_scale: 1.0,
        }
    }
}

/// (name, what it says), in the order the file lists them.
pub const DOC: &[(&str, &str)] = &[
    ("organ_rejection", "organs taken from another body are rejected when put in yours (false: never)"),
    ("rejection_rate", "ticks between rejections for one point of incompatibility"),
    ("rejection_damage", "damage of each rejection"),
    ("keep_chest_cavity", "the chest cavity survives death"),
    ("opener_absolute_health", "a mob with this much health or less can be opened"),
    ("opener_fractional_health", "a mob at this share of its maximum health or less can be opened"),
    ("universal_donor_rate", "chance that an organ of a mob's cavity fits any body"),
    ("looting_boost", "what each level of Looting adds to the chance of organ drops"),
    ("cooldown_scale", "multiplies how long abilities rest"),
];

impl Config {
    /// The settings in `text`, each missing or unreadable one at its default.
    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let line = line.trim().trim_end_matches(',');
            let Some(rest) = line.strip_prefix('"') else { continue };
            let Some((name, value)) = rest.split_once("\":") else { continue };
            let value = value.trim();
            let number = value.parse::<f32>().ok().filter(|n| n.is_finite());
            let flag = value.parse::<bool>().ok();
            match name {
                "organ_rejection" => c.organ_rejection = flag.unwrap_or(c.organ_rejection),
                "keep_chest_cavity" => c.keep_chest_cavity = flag.unwrap_or(c.keep_chest_cavity),
                "rejection_rate" => c.rejection_rate = number.filter(|n| *n > 0.0).unwrap_or(c.rejection_rate),
                "rejection_damage" => c.rejection_damage = number.filter(|n| *n >= 0.0).unwrap_or(c.rejection_damage),
                "opener_absolute_health" => c.opener_absolute_health = number.unwrap_or(c.opener_absolute_health),
                "opener_fractional_health" => c.opener_fractional_health = number.unwrap_or(c.opener_fractional_health),
                "universal_donor_rate" => c.universal_donor_rate = number.map_or(c.universal_donor_rate, |n| n.clamp(0.0, 1.0)),
                "looting_boost" => c.looting_boost = number.filter(|n| *n >= 0.0).unwrap_or(c.looting_boost),
                "cooldown_scale" => c.cooldown_scale = number.filter(|n| *n >= 0.0).unwrap_or(c.cooldown_scale),
                _ => {}
            }
        }
        c
    }

    /// The file with these settings, each with a line of `_comment_<name>` beside it.
    pub fn render(&self) -> String {
        let value = |name: &str| match name {
            "organ_rejection" => self.organ_rejection.to_string(),
            "keep_chest_cavity" => self.keep_chest_cavity.to_string(),
            "rejection_rate" => self.rejection_rate.to_string(),
            "rejection_damage" => self.rejection_damage.to_string(),
            "opener_absolute_health" => self.opener_absolute_health.to_string(),
            "opener_fractional_health" => self.opener_fractional_health.to_string(),
            "universal_donor_rate" => self.universal_donor_rate.to_string(),
            "looting_boost" => self.looting_boost.to_string(),
            _ => self.cooldown_scale.to_string(),
        };
        let mut out = String::from("{\n");
        for (i, (name, doc)) in DOC.iter().enumerate() {
            out += &format!("  \"_comment_{name}\": \"{doc}\",\n");
            out += &format!("  \"{name}\": {}{}\n", value(name), if i + 1 < DOC.len() { "," } else { "" });
        }
        out + "}\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_it_writes_reads_back_and_a_bad_line_keeps_the_default() {
        let mut c = Config::default();
        assert_eq!(Config::parse(&c.render()), c);
        c.organ_rejection = false;
        c.rejection_rate = 300.0;
        c.cooldown_scale = 0.5;
        assert_eq!(Config::parse(&c.render()), c);
        let odd = Config::parse("{\n  \"organ_rejection\": maybe,\n  \"rejection_rate\": -5,\n  \"universal_donor_rate\": 7,\n  \"keep_chest_cavity\": true\n}");
        assert!(odd.organ_rejection && odd.keep_chest_cavity);
        assert_eq!(odd.rejection_rate, 600.0, "a negative rate is not one");
        assert_eq!(odd.universal_donor_rate, 1.0, "a chance is at most 1");
    }
}
