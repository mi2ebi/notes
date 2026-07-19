use std::{collections::HashMap, fs, io, path::Path};

use reqwest::blocking::get;

use crate::colors::{RED, RESET, YELLOW};

const UNICODE_DATA_URL: &str = "https://unicode.org/Public/UCD/latest/ucd/UnicodeData.txt";
const LOCAL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/UnicodeData.txt");

const COMBINING_SOLIDUS: u32 = 0x0338;

pub struct UnicodeData {
    pub superscripts: HashMap<char, char>,
    pub subscripts: HashMap<char, char>,
    pub negations: HashMap<char, char>,
    pub letters: HashMap<char, String>,
}

pub fn load() -> io::Result<UnicodeData> {
    let raw = fetch_raw()?;
    let ud = parse(&raw);
    Ok(ud)
}

pub fn load_local() -> io::Result<UnicodeData> {
    let path = Path::new(LOCAL_PATH);
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        Ok(parse(&raw))
    } else {
        println!("  {YELLOW}no local unicode data, falling back to fetch{RESET}");
        load()
    }
}

pub fn fetch_raw() -> io::Result<String> {
    get(UNICODE_DATA_URL).and_then(reqwest::blocking::Response::text).map_or_else(
        |_| {
            println!("  {YELLOW}fetch error:{RESET} trying local backup");
            let path = Path::new(LOCAL_PATH);
            if path.exists() {
                fs::read_to_string(path)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "  {RED}error:{RESET} fetch failed and no backup exists.\n  download \
                         manually from:\n    {UNICODE_DATA_URL}"
                    ),
                ))
            }
        },
        |new_text| {
            let path = Path::new(LOCAL_PATH);
            match fs::read_to_string(path) {
                Ok(old_text) if old_text != new_text => {
                    let old = parse(&old_text);
                    let new = parse(&new_text);
                    diff_maps("superscripts", &old.superscripts, &new.superscripts);
                    diff_maps("subscripts", &old.subscripts, &new.subscripts);
                    diff_maps("negations", &old.negations, &new.negations);
                }
                _ => {}
            }
            if fs::write(LOCAL_PATH, &new_text).is_err() {
                println!("  {RED}writing error{RESET}");
            }
            Ok(new_text)
        },
    )
}

fn diff_maps(name: &str, old: &HashMap<char, char>, new: &HashMap<char, char>) {
    let mut new = new.iter().collect::<Vec<_>>();
    new.sort();
    for (&base, &new_val) in new {
        if !old.contains_key(&base) {
            println!("  {YELLOW}{name} added:{RESET} {base} -> {new_val}");
        }
    }
}

fn parse(raw: &str) -> UnicodeData {
    let mut superscripts = HashMap::new();
    let mut subscripts = HashMap::new();
    let mut negations = HashMap::new();
    let mut letters = HashMap::new();
    for line in raw.lines() {
        let fields = line.splitn(7, ';').collect::<Vec<_>>();
        if fields.len() < 6 {
            continue;
        }
        let Some(cp) = u32::from_str_radix(fields[0], 16).ok().and_then(char::from_u32) else {
            continue;
        };
        let name = fields[1];
        let decomp = fields[5];
        if let Some(base) = parse_tagged(decomp, "<super>") {
            if !"ªº".contains(cp) && !name.contains("IDEOGRAPHIC ANNOTATION") {
                superscripts.entry(base).or_insert(cp);
            }
        } else if let Some(base) = parse_tagged(decomp, "<sub>") {
            subscripts.entry(base).or_insert(cp);
        }
        if let Some(base) = parse_negation(decomp) {
            negations.entry(base).or_insert(cp);
        }
        if name.starts_with("MATHEMATICAL")
            || name.contains("DOUBLE-STRUCK")
            || name.contains("BLACK-LETTER")
        {
            letters.entry(cp).or_insert_with(|| name.into());
        }
    }
    superscripts.insert('-', '⁻');
    subscripts.insert('-', '₋');
    superscripts.insert('ɜ', 'ᶟ');
    superscripts.insert('ᴈ', 'ᵌ');
    UnicodeData { superscripts, subscripts, negations, letters }
}

fn parse_tagged(decomp: &str, tag: &str) -> Option<char> {
    let rest = decomp.strip_prefix(tag)?.trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 1 {
        return None;
    }
    u32::from_str_radix(parts[0], 16).ok().and_then(char::from_u32)
}

fn parse_negation(decomp: &str) -> Option<char> {
    if decomp.is_empty() || decomp.starts_with('<') {
        return None;
    }
    let parts: Vec<&str> = decomp.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    let second = u32::from_str_radix(parts[1], 16).ok()?;
    if second != COMBINING_SOLIDUS {
        return None;
    }
    u32::from_str_radix(parts[0], 16).ok().and_then(char::from_u32)
}
