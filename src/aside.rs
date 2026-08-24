use std::sync::LazyLock;

use regex::Regex;

use crate::{
    colors::{RESET, YELLOW},
    html::apply_edits,
};

const COLORS: &[(&str, &str)] = &[
    ("ochre", "--bg-ochre"),
    ("sage", "--bg-sage"),
    ("lavender", "--bg-lavender"),
    ("green", "--bg-green-nuanced"),
    ("red", "--bg-red-nuanced"),
    ("magenta", "--bg-magenta-nuanced"),
    ("cyan", "--bg-cyan-subtle"),
];

fn var_for(name: &str) -> Option<&'static str> {
    COLORS.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<!--\s*aside\s*:\s*([a-zA-Z][a-zA-Z-]*)\s*-->\s*(<aside\b[^>]*>)").unwrap()
});

static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\sstyle="([^"]*)""#).unwrap());

fn set_background(style: &str, var: &str) -> String {
    let mut decls: Vec<String> = vec![format!("background:var({var})")];
    decls.extend(
        style
            .split(';')
            .map(str::trim)
            .filter(|d| !d.is_empty() && !d.starts_with("background"))
            .map(str::to_owned),
    );
    decls.join("; ")
}

pub fn process(html: &str) -> String {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for caps in DIRECTIVE_RE.captures_iter(html) {
        let name = caps.get(1).unwrap().as_str();
        let tag_m = caps.get(2).unwrap();
        let Some(var) = var_for(name) else {
            println!("  {YELLOW}unknown aside color:{RESET} {name}");
            continue;
        };
        let tag = tag_m.as_str();
        let inner = &tag[6 .. tag.len() - 1]; // strip leading "<aside" and trailing ">"
        let existing_style =
            STYLE_RE.captures(inner).map(|c| c.get(1).unwrap().as_str().to_owned());
        let new_style = set_background(existing_style.as_deref().unwrap_or(""), var);
        let new_inner = existing_style.as_ref().map_or_else(
            || format!(r#"{inner} style="{new_style}""#),
            |existing| {
                inner.replacen(
                    &format!(r#"style="{existing}""#),
                    &format!(r#"style="{new_style}""#),
                    1,
                )
            },
        );
        edits.push((tag_m.start(), tag_m.end(), format!("<aside{new_inner}>")));
    }
    apply_edits(html, edits)
}
