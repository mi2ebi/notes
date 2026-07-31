use std::sync::LazyLock;

use fancy_regex::{Captures as FancyCaptures, Regex as FancyRegex};
use phf::phf_map;

pub static COMBINING: phf::Map<&str, char> = phf_map! {
    "=" | "bar" => '\u{0304}',
    "~" | "tilde" => '\u{0303}',
    "^" | "hat" => '\u{0302}',
    "." | "dot" => '\u{0307}',
    "ddot" => '\u{0308}',
    "'" | "acute" => '\u{0301}',
    "`" | "grave" => '\u{0300}',
    "u" | "breve" => '\u{0306}',
    "v" | "check" => '\u{030c}',
};

static ACCENT_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    let letter_alts = COMBINING
        .keys()
        .filter(|k| k.chars().all(|c| c.is_ascii_alphabetic()))
        .copied()
        .collect::<Vec<_>>()
        .join("|");
    let symbol_alts = COMBINING
        .keys()
        .filter(|k| k.len() == 1 && !k.chars().next().unwrap().is_ascii_alphabetic())
        .map(|s| regex::escape(s))
        .collect::<Vec<_>>()
        .join("|");
    FancyRegex::new(&format!(
        r"\\((?:{letter_alts})(?![a-zA-Z])|(?:{symbol_alts}))\s*(?:\{{([^}}])\}}|\{{[^}}]{{2,}}\}}|([^\\{{\s}}]))"
    )).unwrap()
});

pub fn replace(text: &str) -> String {
    ACCENT_RE
        .replace_all(text, |caps: &FancyCaptures<str>| {
            let cmd = caps.get(1).unwrap().as_str();
            let ch = caps.get(2).or_else(|| caps.get(3));
            ch.map_or_else(
                || caps.get(0).unwrap().as_str().to_owned(),
                |m| format!("{}{}", m.as_str(), COMBINING[cmd]),
            )
        })
        .into_owned()
}
