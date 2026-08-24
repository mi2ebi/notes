pub mod classes;
pub mod elements;
pub mod session;
pub mod tui;

use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    sync::LazyLock,
};

use elements::{apply_spans, find};
use regex::{Captures, Regex};
use session::Session;
use tui::TuiResult;

use crate::hl::classes::ClassInfo;

static EXISTING_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="(hl-[0-9a-f]{16}(?:-\d+)?)""#).unwrap());

static LEGACY_SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?-s)(</(?:pre|code)>)\s*<script>(color\("hl-[0-9a-f]{16}(?:-\d+)?", \[.*?\]\);?)</script>"#
    )
    .unwrap()
});

static BATCHED_SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?-s)[ \t]*<script>[ \t]*\n((?:[ \t]*color\("hl-[0-9a-f]{16}(?:-\d+)?", [^\n]*\);?[ \t]*\n)*)[ \t]*</script>[ \t]*\n?"#
    )
    .unwrap()
});

pub fn process_file(path: &Path, html: &str) -> io::Result<String> {
    let mut legacy_calls: Vec<String> = Vec::new();
    let cleaned = BATCHED_SCRIPT_RE
        .replace_all(html, |caps: &Captures| {
            let inner = caps.get(1).unwrap().as_str();
            for line in inner.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("color(") {
                    legacy_calls.push(trimmed.to_owned());
                }
            }
            ""
        })
        .into_owned();
    let cleaned = LEGACY_SCRIPT_RE
        .replace_all(&cleaned, |caps: &Captures| {
            legacy_calls.push(caps.get(2).unwrap().as_str().to_owned());
            caps.get(1).unwrap().as_str().to_owned()
        })
        .into_owned();
    let elements = find(&cleaned);
    if elements.is_empty() && legacy_calls.is_empty() {
        return Ok(html.to_string());
    }
    let mut used_ids: HashSet<String> =
        EXISTING_ID_RE.captures_iter(&cleaned).map(|c| c[1].to_owned()).collect();
    legacy_calls.retain(|call| {
        let Some(rest) = call.strip_prefix("color(\"") else { return false };
        let Some(end) = rest.find('"') else { return false };
        used_ids.contains(&rest[.. end])
    });
    let mut suggestions: HashMap<Vec<char>, &'static ClassInfo> = HashMap::new();
    let mut result = cleaned;
    let mut jses = vec![];
    for (i, element) in elements.iter().enumerate().rev() {
        let spans = {
            let mut session = Session::new(&element.stripped_content, &mut suggestions);
            let status = format!("{}  block {}/{}", path.display(), i + 1, elements.len());
            match tui::run(&mut session, &status)? {
                TuiResult::SkipAll => break,
                TuiResult::Discard => continue,
                TuiResult::Done => {}
            }
            session.finish()
        };
        let (tag, js) = apply_spans(element, &spans, &mut used_ids);
        result.replace_range(element.start .. element.end, &tag);
        jses.push(js);
    }
    if !legacy_calls.is_empty() || !jses.is_empty() {
        let mut all_calls = legacy_calls;
        all_calls.extend(jses.into_iter().rev());
        if let Some(pos) = result.rfind("</body>") {
            let script =
                format!("  <script>\n     {}\n    </script>\n  ", all_calls.join("\n     "));
            result.insert_str(pos, &script);
        }
    }
    Ok(result)
}
