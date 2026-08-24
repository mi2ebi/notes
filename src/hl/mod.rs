use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    sync::LazyLock,
};

mod classes;
mod elements;
mod session;
mod tui;

use regex::{Captures, Regex};

use crate::hl::{
    classes::ClassInfo,
    elements::{apply_spans, find},
    session::Session,
    tui::TuiResult,
};

static EXISTING_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="(hl-[0-9a-f]{16}(?:-\d+)?)""#).unwrap());

static BATCHED_SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?-s)[ \t]*<script>[ \t]*\n((?:[ \t]*color\("hl-[0-9a-f]{16}(?:-\d+)?", [^\n]*\);?[ \t]*\n)*)[ \t]*</script>[ \t]*\n?"#
    )
    .unwrap()
});

static DATA_LANG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\sdata-lang="([^"]*)""#).unwrap());

fn extract_call_id(call: &str) -> Option<&str> {
    let rest = call.strip_prefix("color(\"")?;
    let end = rest.find('"')?;
    Some(&rest[.. end])
}

fn find_id_pos(html: &str, id: &str) -> Option<usize> {
    let pattern = format!(r#"id="{id}""#);
    html.find(&pattern)
}

pub fn process_file(path: &Path, html: &str) -> io::Result<String> {
    let mut prev_calls: Vec<String> = Vec::new();
    let cleaned = BATCHED_SCRIPT_RE
        .replace_all(html, |caps: &Captures| {
            let inner = caps.get(1).unwrap().as_str();
            for line in inner.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("color(") {
                    prev_calls.push(trimmed.to_owned());
                }
            }
            ""
        })
        .into_owned();
    let elements = find(&cleaned);
    if elements.is_empty() && prev_calls.is_empty() {
        return Ok(html.to_string());
    }
    let mut used_ids: HashSet<String> =
        EXISTING_ID_RE.captures_iter(&cleaned).map(|c| c[1].to_owned()).collect();
    let preassigned_ids: Vec<String> = elements
        .iter()
        .map(|e| {
            let hash_hex = elements::content_hash(&e.stripped_content);
            elements::dedup_id(&hash_hex, &mut used_ids)
        })
        .collect();
    let mut suggestions: HashMap<Vec<char>, &'static ClassInfo> = HashMap::new();
    let mut result = cleaned;
    let mut new_calls: Vec<(usize, String)> = vec![];
    for (i, element) in elements.iter().enumerate().rev() {
        let spans = {
            let mut session = Session::new(&element.stripped_content, &mut suggestions);
            let lang = DATA_LANG_RE.captures(&element.attrs).map(|c| c.get(1).unwrap().as_str());
            let status = lang.map_or_else(
                || format!("{}  block {}/{}", path.display(), i + 1, elements.len()),
                |l| format!("{}  block {}/{}  [{}]", path.display(), i + 1, elements.len(), l),
            );
            match tui::run(&mut session, &status)? {
                TuiResult::SkipAll => break,
                TuiResult::Discard => continue,
                TuiResult::Done => {}
            }
            session.finish()
        };
        let id = &preassigned_ids[i];
        let (tag, js) = apply_spans(element, &spans, id);
        result.replace_range(element.start .. element.end, &tag);
        new_calls.push((element.start, js));
    }
    let final_ids: HashSet<String> =
        EXISTING_ID_RE.captures_iter(&result).map(|c| c[1].to_owned()).collect();
    prev_calls.retain(|call| {
        let Some(id) = extract_call_id(call) else { return false };
        final_ids.contains(id)
    });
    let mut all_calls: Vec<(usize, String)> = prev_calls
        .into_iter()
        .filter_map(|call| {
            let id = extract_call_id(&call)?;
            let pos = find_id_pos(&result, id).unwrap_or(0);
            Some((pos, call))
        })
        .collect();
    all_calls.extend(new_calls);
    all_calls.sort_by_key(|(pos, _)| *pos);
    if !all_calls.is_empty() {
        let calls: Vec<String> = all_calls.into_iter().map(|(_, c)| c).collect();
        if let Some(pos) = result.rfind("</body>") {
            let script = format!("  <script>\n     {}\n    </script>\n  ", calls.join("\n     "));
            result.insert_str(pos, &script);
        }
    }
    Ok(result)
}
