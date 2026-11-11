//! The mod's lang files (`assets/chestcavity/lang`): a reader for their flat `"key": "value"` shape, so the tests can
//! check that every text the code uses is there in every language.

/// The pairs of a flat JSON object of strings, as `tools/import.py` writes it (one pair a line).
pub fn parse(json: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in json.lines() {
        let line = line.trim().trim_end_matches(',');
        let Some(rest) = line.strip_prefix('"') else { continue };
        let Some((key, value)) = rest.split_once("\": \"") else { continue };
        let Some(value) = value.strip_suffix('"') else { continue };
        out.push((unescape(key), unescape(value)));
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// How many arguments a template takes: its highest `%n$s`, or its count of `%s`.
pub fn arguments(template: &str) -> usize {
    let (mut plain, mut numbered) = (0, 0);
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        let mut digits = String::new();
        while let Some(d) = chars.peek().copied().filter(char::is_ascii_digit) {
            digits.push(d);
            chars.next();
        }
        match chars.next() {
            Some('$') => numbered = numbered.max(digits.parse().unwrap_or(0)),
            Some('s') => plain += 1,
            _ => {}
        }
    }
    plain.max(numbered)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const EN: &str = include_str!("../../../assets/chestcavity/lang/en_us.json");
    pub const PT: &str = include_str!("../../../assets/chestcavity/lang/pt_pt.json");
    pub const BR: &str = include_str!("../../../assets/chestcavity/lang/pt_br.json");

    #[test]
    fn it_reads_the_files_the_importer_writes() {
        let pairs = parse("{\n  \"a.b\": \"One \\\"two\\\"\",\n  \"c\": \"3 %s\"\n}\n");
        assert_eq!(pairs, [("a.b".to_string(), "One \"two\"".to_string()), ("c".to_string(), "3 %s".to_string())]);
        assert_eq!(arguments("%1$s: %2$s"), 2);
        assert_eq!(arguments("%s and %s 100%%"), 2);
        assert_eq!(arguments("none"), 0);
    }

    /// Every text of the describe table and its templates is in every language, with the arguments it needs.
    #[test]
    fn every_text_of_the_organs_is_in_every_language() {
        let (en, pt, br) = (parse(EN), parse(PT), parse(BR));
        let get = |l: &[(String, String)], k: &str| l.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        let wanted = crate::describe::score_texts().into_iter().chain(crate::describe::TEMPLATES.iter().map(|&(k, e)| (k.to_string(), e)));
        for (key, english) in wanted {
            assert_eq!(get(&en, &key).as_deref(), Some(english), "en_us has the English of {key}");
            let local = get(&pt, &key).unwrap_or_else(|| panic!("pt_pt lacks {key}"));
            assert!(!local.is_empty() && arguments(&local) == arguments(english), "{key}: {local:?} takes other arguments than {english:?}");
            assert_eq!(get(&br, &key), Some(local), "pt_br is pt_pt for {key}");
        }
    }

    #[test]
    fn every_item_is_named_in_every_language() {
        let (en, pt) = (parse(EN), parse(PT));
        let items: Vec<_> = en.iter().filter(|(k, _)| k.starts_with("item.")).collect();
        assert!(items.len() >= 291);
        for (key, _) in items {
            assert!(pt.iter().any(|(k, v)| k == key && !v.is_empty()), "pt_pt does not name {key}");
        }
    }
}
