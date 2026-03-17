pub mod accents;
pub mod fonts;
pub mod macros;
pub mod scripts;
pub mod structural;

use std::{collections::HashMap, sync::LazyLock};

use fonts::FontMaps;
use regex::Regex;
pub use structural::STRUCTURAL;

static NOT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\not\s*(\S)").unwrap());

fn replace_negations(text: &str, negations: &HashMap<char, char>) -> String {
    NOT_RE
        .replace_all(text, |caps: &regex::Captures| {
            let ch = caps.get(1).unwrap().as_str().chars().next().unwrap();
            negations
                .get(&ch)
                .map_or_else(|| caps.get(0).unwrap().as_str().to_owned(), |&n| n.to_string())
        })
        .into_owned()
}

pub fn process_region(
    content: &str,
    font_maps: &FontMaps,
    superscripts: &HashMap<char, char>,
    subscripts: &HashMap<char, char>,
    negations: &HashMap<char, char>,
) -> String {
    let content = macros::replace(content);
    let content = replace_negations(&content, negations);
    let content = fonts::replace(&content, font_maps);
    let content = accents::replace(&content);
    scripts::replace(&content, superscripts, subscripts)
}
