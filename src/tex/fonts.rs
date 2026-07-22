use std::{collections::HashMap, sync::LazyLock};

use fancy_regex::{Captures as FancyCaptures, Regex as FancyRegex};
use phf::phf_map;

static GREEK_SMALL: phf::Map<&str, char> = phf_map! {
    "ALPHA" => 'α',
    "BETA" => 'β',
    "GAMMA" => 'γ',
    "DELTA" => 'δ',
    "EPSILON" => 'ε', // \varepsilon
    "ZETA" => 'ζ',
    "ETA" => 'η',
    "THETA" => 'θ',
    "IOTA" => 'ι',
    "KAPPA" => 'κ',
    "LAMDA" => 'λ', // this is how the character name spells it!
    "MU" => 'μ',
    "NU" => 'ν',
    "XI" => 'ξ',
    "OMICRON" => 'ο',
    "PI" => 'π',
    "RHO" => 'ρ',
    "SIGMA" => 'σ',
    "TAU" => 'τ',
    "UPSILON" => 'υ',
    "PHI" => 'φ',
    "CHI" => 'χ',
    "PSI" => 'ψ',
    "OMEGA" => 'ω',
    "DIGAMMA" => 'ϝ',
};

static GREEK_CAPITAL: phf::Map<&str, char> = phf_map! {
    "ALPHA" => 'Α',
    "BETA" => 'Β',
    "GAMMA" => 'Γ',
    "DELTA" => 'Δ',
    "EPSILON" => 'Ε',
    "ZETA" => 'Ζ',
    "ETA" => 'Η',
    "THETA" => 'Θ',
    "IOTA" => 'Ι',
    "KAPPA" => 'Κ',
    "LAMDA" => 'Λ',
    "MU" => 'Μ',
    "NU" => 'Ν',
    "XI" => 'Ξ',
    "OMICRON" => 'Ο',
    "PI" => 'Π',
    "RHO" => 'Ρ',
    "SIGMA" => 'Σ',
    "TAU" => 'Τ',
    "UPSILON" => 'Υ',
    "PHI" => 'Φ',
    "CHI" => 'Χ',
    "PSI" => 'Ψ',
    "OMEGA" => 'Ω',
    "DIGAMMA" => 'Ϝ',
};

static GREEK_SYMBOL_SMALL: phf::Map<&str, char> = phf_map! {
    "EPSILON" => 'ϵ', // \epsilon
    "THETA" => 'ϑ',
    "KAPPA" => 'ϰ',
    "PHI" => 'ϕ',
    "RHO" => 'ϱ',
    "PI" => 'ϖ',
};

static GREEK_SYMBOL_CAPITAL: phf::Map<&str, char> = phf_map! {
    "THETA" => 'ϴ',
};

static DIGIT: phf::Map<&str, char> = phf_map! {
    "ZERO" => '0',
    "ONE" => '1',
    "TWO" => '2',
    "THREE" => '3',
    "FOUR" => '4',
    "FIVE" => '5',
    "SIX" => '6',
    "SEVEN" => '7',
    "EIGHT" => '8',
    "NINE" => '9',
};

type Pred = fn(&str) -> bool;
pub const FONT_PREDICATES: &[(&str, Pred)] = &[
    ("mathbfit", |n| n.contains("MATHEMATICAL BOLD ITALIC") && !n.contains("SANS-SERIF")),
    ("mathbb", |n| n.contains("DOUBLE-STRUCK") && !n.contains("ITALIC")),
    ("mathfrak", |n| {
        (n.contains("MATHEMATICAL FRAKTUR") || n.contains("BLACK-LETTER")) && !n.contains("BOLD")
    }),
    ("mathcal", |n| n.contains("SCRIPT") && !n.contains("BOLD") && n != "SCRIPT CAPITAL P"),
    ("mathsfit", |n| n.contains("MATHEMATICAL SANS-SERIF ITALIC") && !n.contains("BOLD")),
    ("mathsf", |n| {
        n.contains("MATHEMATICAL SANS-SERIF") && !n.contains("BOLD") && !n.contains("ITALIC")
    }),
    ("mathtt", |n| n.contains("MATHEMATICAL MONOSPACE")),
    ("mathbf", |n| {
        n.contains("MATHEMATICAL BOLD")
            && !n.contains("ITALIC")
            && !n.contains("FRAKTUR")
            && !n.contains("SANS-SERIF")
            && !n.contains("SCRIPT")
    }),
    ("mathit", |n| {
        n.contains("MATHEMATICAL ITALIC") && !n.contains("BOLD") && !n.contains("SANS-SERIF")
    }),
];

pub static FONT_ALIASES: phf::Map<&str, &str> = phf_map! {
    "Bbb" => "mathbb",
    "bm" => "mathbfit",
    "bold" => "mathbf",
    "frak" => "mathfrak",
    "textbf" => "mathbf",
    "textit" => "mathit",
    "textsf" => "mathsf",
    "texttt" => "mathtt",
};

fn extract_base_char(name: &str) -> Option<char> {
    let words: Vec<&str> = name.split_whitespace().collect();
    let last = *words.last()?;
    let is_capital = words.contains(&"CAPITAL");
    if let Some(&c) = DIGIT.get(last) {
        return Some(c);
    }
    if last.len() == 1 && last.chars().next()?.is_ascii_alphabetic() {
        return if is_capital { last.to_string() } else { last.to_lowercase() }.chars().next();
    }
    if last == "SYMBOL" {
        let gname = words.len().checked_sub(2).and_then(|i| words.get(i))?;
        return if is_capital {
            GREEK_SYMBOL_CAPITAL.get(gname).copied()
        } else {
            GREEK_SYMBOL_SMALL.get(gname).copied()
        };
    }
    if last == "NABLA" {
        return Some('∇');
    }
    if last == "DIFFERENTIAL" {
        return Some('∂');
    }
    if is_capital { GREEK_CAPITAL.get(last).copied() } else { GREEK_SMALL.get(last).copied() }
}

pub type FontMaps = HashMap<&'static str, HashMap<char, char>>;

pub fn build(letters: &HashMap<char, String>) -> FontMaps {
    let mut maps =
        FONT_PREDICATES.iter().map(|(cmd, _)| (*cmd, HashMap::new())).collect::<FontMaps>();
    for (&styled, name) in letters {
        for (cmd, pred) in FONT_PREDICATES {
            if pred(name) {
                if let Some(base) = extract_base_char(name) {
                    maps.get_mut(cmd).unwrap().entry(base).or_insert(styled);
                }
                break;
            }
        }
    }
    maps.get_mut("mathit").unwrap().entry('h').or_insert('ℎ');
    maps
}

static MATH_FONT_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    let mut cmds = FONT_PREDICATES.iter().map(|(c, _)| *c).collect::<Vec<_>>();
    cmds.extend(FONT_ALIASES.keys().copied());
    cmds.sort_by_key(|s| -s.len().cast_signed());
    let alts = cmds.iter().map(|c| regex::escape(c)).collect::<Vec<_>>().join("|");
    let char_class = r"[A-Za-z0-9\u{0391}-\u{03ff}\u{2202}\u{2207}]";
    FancyRegex::new(&format!(r"\\({alts})\s*(?:\{{({char_class}+)\}}|({char_class})(?![A-Za-z]))"))
        .unwrap()
});

pub fn replace(text: &str, maps: &FontMaps) -> String {
    MATH_FONT_RE
        .replace_all(text, |caps: &FancyCaptures| {
            let cmd = caps.get(1).unwrap().as_str();
            let canonical = FONT_ALIASES.get(cmd).copied().unwrap_or(cmd);
            let whole = || caps.get(0).unwrap().as_str().to_owned();
            let Some(map) = maps.get(canonical) else { return whole() };
            caps.get(2).map_or_else(
                || {
                    let ch = caps.get(3).unwrap().as_str().chars().next().unwrap();
                    map.get(&ch).map_or_else(whole, |&s| s.to_string())
                },
                |braced| {
                    let converted: Option<String> =
                        braced.as_str().chars().map(|c| map.get(&c).copied()).collect();
                    converted.unwrap_or_else(whole)
                },
            )
        })
        .into_owned()
}
