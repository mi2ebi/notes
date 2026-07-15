pub mod classes;
pub mod elements;
pub mod session;
pub mod tui;

use std::{collections::HashSet, io, path::Path, sync::LazyLock};

use elements::{apply_spans, find};
use regex::Regex;
use session::Session;
use tui::TuiResult;

static EXISTING_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="(hl-[0-9a-f]{16}(?:-\d+)?)""#).unwrap());

pub fn process_file(path: &Path, html: &str) -> io::Result<String> {
    let elements = find(html);
    if elements.is_empty() {
        return Ok(html.to_string());
    }
    let mut used_ids: HashSet<String> =
        EXISTING_ID_RE.captures_iter(html).map(|c| c[1].to_owned()).collect();
    let mut result = html.to_owned();
    for (i, element) in elements.iter().enumerate().rev() {
        let spans = if let Some(legacy_spans) = &element.legacy_spans {
            legacy_spans.clone()
        } else {
            let mut session = Session::new(&element.stripped_content);
            let status = format!("{}  block {}/{}", path.display(), i + 1, elements.len());
            match tui::run(&mut session, &status)? {
                TuiResult::SkipAll => break,
                TuiResult::Discard => continue,
                TuiResult::Done => {}
            }
            session.finish()
        };
        let replacement = apply_spans(element, &spans, &mut used_ids);
        result.replace_range(element.start .. element.full_end, &replacement);
    }
    Ok(result)
}
