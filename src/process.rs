use std::{collections::HashMap, sync::LazyLock};

use fancy_regex::{Captures as FancyCaptures, Regex as FancyRegex};
use regex::Regex;

use crate::{
    colors::{RESET, YELLOW},
    entities,
    tex::{accents, fonts, fonts::FontMaps, macros, scripts},
};

static INLINE_MATH_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?s)(\\\()((?:(?!<[a-zA-Z/]).)*?)(\\\))").unwrap());
static DISPLAY_MATH_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?s)(\\\[)((?:(?!<[a-zA-Z/]).)*?)(\\\])").unwrap());

pub static STRUCTURAL: phf::Set<&str> = phf::phf_set! {
    "Big",
    "Bigg",
    "Biggl",
    "Biggr",
    "Bigl",
    "Bigr",
    "Bmatrix",
    "DeclareMathOperator",
    "LaTeX",
    "TeX",
    "Vert",
    "Vmatrix",
    "align",
    "aligned",
    "array",
    "atop",
    "begin",
    "big",
    "bigg",
    "bigl",
    "bigr",
    "binom",
    "cases",
    "cfrac",
    "choose",
    "class",
    "color",
    "colorbox",
    "cos",
    "dbinom",
    "def",
    "dfrac",
    "displaystyle",
    "end",
    "frac",
    "gather",
    "gathered",
    "gdef",
    "hline",
    "hphantom",
    "hskip",
    "kern",
    "label",
    "langle",
    "lceil",
    "lfloor",
    "left",
    "lim",
    "limits",
    "llap",
    "ln",
    "log",
    "lvert",
    "lVert",
    "mathbin",
    "mathcal",
    "mathclose",
    "mathop",
    "mathopen",
    "mathord",
    "mathpunct",
    "mathrel",
    "mathrm",
    "mathscr",
    "matrix",
    "max",
    "middle",
    "min",
    "mkern",
    "mskip",
    "newcommand",
    "nolimits",
    "nonumber",
    "not",
    "notag",
    "operatorname",
    "overbrace",
    "overbracket",
    "overline",
    "overset",
    "phantom",
    "pmatrix",
    "quad",
    "qquad",
    "rangle",
    "rceil",
    "renewcommand",
    "rfloor",
    "right",
    "rlap",
    "root",
    "rvert",
    "rVert",
    "scriptstyle",
    "scriptscriptstyle",
    "set",
    "sin",
    "smash",
    "sqrt",
    "stackrel",
    "substack",
    "tag",
    "tan",
    "tbinom",
    "text",
    "textrm",
    "textstyle",
    "tfrac",
    "underbrace",
    "underbracket",
    "underline",
    "underset",
    "vbmatrix",
    "vert",
    "vmatrix",
    "vphantom",
    "widehat",
    "widetilde",
    "xleftarrow",
    "xrightarrow",
    // defined by me
    "corr",
    "cov",
    "stdev",
    "var"
};

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

fn process_math_region(
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
            process_math_region(content, font_maps, superscripts, subscripts, negations);
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
    let text = entities::replace(text);
    apply_in_math_regions(&text, font_maps, superscripts, subscripts, negations)
}

static UNKNOWN_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("&([a-zA-Z]+);").unwrap());
static UNCONVERTED_MACRO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\([a-zA-Z]+)").unwrap());

pub fn warn_unknown(processed: &str, math_regions: &[String]) {
    for caps in UNKNOWN_ENTITY_RE.captures_iter(processed) {
        let entity = caps.get(0).unwrap().as_str();
        if !matches!(entity, "&lt;" | "&gt;" | "&amp;") {
            println!("  {YELLOW}unknown entity:{RESET} {entity}");
        }
    }
    let mut seen = std::collections::HashSet::new();
    for content in math_regions {
        for caps in UNCONVERTED_MACRO_RE.captures_iter(content) {
            let name = caps.get(1).unwrap().as_str();
            if !STRUCTURAL.contains(name) && seen.insert(name.to_owned()) {
                println!("  {YELLOW}unknown macro:{RESET} \\{name}");
            }
        }
    }
}
