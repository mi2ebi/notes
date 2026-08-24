use std::{fs, path::Path, sync::LazyLock};

use regex::Regex;

use crate::{
    colors::{RED, RESET, YELLOW},
    html::apply_edits,
};

static INCLUDE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)<!--\s*include\s+(\S+)\s*-->(\s*<!--\s*included\s*-->.*?<!--\s*/included\s*-->)?",
    )
    .unwrap()
});

static MAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<main\b[^>]*>(.*?)</main>").unwrap());

pub fn process(html: &str, path: &Path) -> String {
    if INCLUDE_RE.find(html).is_none() {
        return html.to_owned();
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let self_canonical = path.canonicalize().ok();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();

    for caps in INCLUDE_RE.captures_iter(html) {
        let m = caps.get(0).unwrap();
        let target_name = caps.get(1).unwrap().as_str();
        let target_path = base.join(target_name);

        let is_same_file = self_canonical.as_ref().map_or_else(
            || target_path.as_path() == path,
            |self_path| target_path.canonicalize().is_ok_and(|t| t == *self_path),
        );

        if is_same_file {
            println!("  {YELLOW}include skipped:{RESET} '{target_name}' refers to itself");
            continue;
        }

        let Ok(source) = fs::read_to_string(&target_path) else {
            println!("  {RED}include error:{RESET} couldn't read '{}'", target_path.display());
            continue;
        };
        let content = MAIN_RE.captures(&source).map_or_else(
            || {
                println!(
                    "  {YELLOW}include warning:{RESET} no <main> in '{target_name}', including \
                     the whole file"
                );
                source.trim().to_owned()
            },
            |c| c.get(1).unwrap().as_str().trim().to_owned(),
        );
        let replacement = format!(
            "<!-- include {target_name} -->\n<!-- included -->\n{content}\n<!-- /included -->"
        );
        edits.push((m.start(), m.end(), replacement));
    }
    apply_edits(html, edits)
}
