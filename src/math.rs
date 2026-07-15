use std::{collections::HashMap, sync::LazyLock};

use fancy_regex::{Captures as FancyCaptures, Regex as FancyRegex};
use regex::Regex;

use crate::{
    colors::{RESET, YELLOW},
    entities,
    tex::{self, fonts::FontMaps},
};

static INLINE_MATH_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?s)(\\\()((?:(?!<[a-zA-Z/]).)*?)(\\\))").unwrap());
static DISPLAY_MATH_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?s)(\\\[)((?:(?!<[a-zA-Z/]).)*?)(\\\])").unwrap());

fn apply_in_math_regions(
    text: &str,
    font_maps: &FontMaps,
    superscripts: &HashMap<char, char>,
    subscripts: &HashMap<char, char>,
    negations: &HashMap<char, char>,
) -> (String, Vec<String>) {
    let mut regions = Vec::new();
    let mut sub = |caps: &FancyCaptures| {
        let open = caps.get(1).unwrap().as_str();
        let content = caps.get(2).unwrap().as_str();
        let close = caps.get(3).unwrap().as_str();
        let processed =
            tex::process_region(content, font_maps, superscripts, subscripts, negations);
        regions.push(processed.clone());
        format!("{open}{processed}{close}")
    };
    let text = INLINE_MATH_RE.replace_all(text, &mut sub).into_owned();
    let text = DISPLAY_MATH_RE.replace_all(&text, &mut sub).into_owned();
    (text, regions)
}

pub fn process(
    text: &str,
    font_maps: &FontMaps,
    superscripts: &HashMap<char, char>,
    subscripts: &HashMap<char, char>,
    negations: &HashMap<char, char>,
) -> (String, Vec<String>) {
    apply_in_math_regions(text, font_maps, superscripts, subscripts, negations)
}

static UNKNOWN_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("&([a-zA-Z]+);").unwrap());
static UNCONVERTED_MACRO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\([a-zA-Z]+)").unwrap());
static DEFINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\(?:g?def|(?:re)?newcommand)\s*\{?\\([a-zA-Z]+)\}?").unwrap());

fn locally_defined(math_regions: &[String]) -> std::collections::HashSet<String> {
    let mut defined = std::collections::HashSet::new();
    for content in math_regions {
        for caps in DEFINE_RE.captures_iter(content) {
            defined.insert(caps.get(1).unwrap().as_str().to_owned());
        }
    }
    defined
}

pub fn warn_unknown(processed: &str, math_regions: &[String]) -> bool {
    let mut warned = false;
    for caps in UNKNOWN_ENTITY_RE.captures_iter(processed) {
        let entity = caps.get(1).unwrap().as_str();
        let full = format!("&{entity};");
        if !entities::ENTITIES.contains_key(&full) && !entities::STRUCTURAL.contains(&full) {
            println!("  {YELLOW}unknown entity:{RESET} {entity}");
            warned = true;
        }
    }
    let defined = locally_defined(math_regions);
    let mut seen = std::collections::HashSet::new();
    for content in math_regions {
        for caps in UNCONVERTED_MACRO_RE.captures_iter(content) {
            let name = caps.get(1).unwrap().as_str();
            if !tex::STRUCTURAL.contains(name)
                && !defined.contains(name)
                && seen.insert(name.to_owned())
            {
                println!("  {YELLOW}unknown macro:{RESET} \\{name}");
                warned = true;
            }
        }
    }
    warned
}
