use std::fmt::Write as _;

use phf::phf_map;

pub static ENV_SHORTHANDS: phf::Map<&str, &str> = phf_map! {
    "bmat" => "bmatrix",
    "pmat" => "pmatrix",
    "vmat" => "vmatrix",
    "Vmat" => "Vmatrix",
    "amat" => "align*",
    "gmat" => "gather*",
    "arr" => "array",
};

fn matching_delim(chars: &[char], open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut depth = 0;
    for (k, &c) in chars.iter().enumerate().skip(open) {
        if c == open_ch {
            depth += 1;
        } else if c == close_ch {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

pub fn replace(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            let name_start = i + 1;
            let mut name_end = name_start;
            while name_end < chars.len() && chars[name_end].is_ascii_alphabetic() {
                name_end += 1;
            }
            let name: String = chars[name_start .. name_end].iter().collect();
            if let Some(&env) = ENV_SHORTHANDS.get(name.as_str()) {
                let mut cursor = name_end;
                let mut spec = None;
                if chars.get(cursor) == Some(&'[')
                    && let Some(close) = matching_delim(&chars, cursor, '[', ']')
                {
                    spec = Some(chars[cursor + 1 .. close].iter().collect::<String>());
                    cursor = close + 1;
                }
                if chars.get(cursor) == Some(&'{')
                    && let Some(close) = matching_delim(&chars, cursor, '{', '}')
                {
                    let body: String = chars[cursor + 1 .. close].iter().collect();
                    let spec_part = spec.map_or_else(String::new, |s| format!("{{{s}}}"));
                    let _ = write!(out, "\\begin{{{env}}}{spec_part}{body}\\end{{{env}}}");
                    i = close + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
