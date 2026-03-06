use std::{collections::HashMap, sync::LazyLock};

use fancy_regex::{Captures as FancyCaptures, Regex as FancyRegex};

static BRACED_SUP_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<!\\)\^\{([^}]*)\}").unwrap());
static BRACED_SUB_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<!\\)_\{([^}]*)\}").unwrap());
static BARE_SUP_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<!\\)\^([0-9a-zA-Z+\-=()])").unwrap());
static BARE_SUB_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<!\\)_([0-9a-zA-Z+\-=()])").unwrap());

pub fn replace(
    text: &str,
    superscripts: &HashMap<char, char>,
    subscripts: &HashMap<char, char>,
) -> String {
    let text = replace_braced(&BRACED_SUP_RE, text, superscripts);
    let text = replace_braced(&BRACED_SUB_RE, &text, subscripts);
    let text = replace_bare(&BARE_SUP_RE, &text, superscripts);
    replace_bare(&BARE_SUB_RE, &text, subscripts)
}

fn replace_braced(re: &FancyRegex, text: &str, map: &HashMap<char, char>) -> String {
    re.replace_all(text, |caps: &FancyCaptures| {
        let inner = caps.get(1).unwrap().as_str();
        if inner.contains('\\') {
            return caps.get(0).unwrap().as_str().to_owned();
        }
        try_convert(inner.replace(' ', "").chars(), map)
            .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_owned())
    })
    .into_owned()
}

fn replace_bare(re: &FancyRegex, text: &str, map: &HashMap<char, char>) -> String {
    re.replace_all(text, |caps: &FancyCaptures| {
        let ch = caps.get(1).unwrap().as_str().chars().next().unwrap();
        map.get(&ch).map_or_else(|| caps.get(0).unwrap().as_str().to_owned(), |&s| s.to_string())
    })
    .into_owned()
}

fn try_convert(chars: impl Iterator<Item = char>, map: &HashMap<char, char>) -> Option<String> {
    chars.map(|c| map.get(&c).copied()).collect()
}
