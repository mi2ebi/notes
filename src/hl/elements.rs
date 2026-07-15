use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    sync::LazyLock,
};

use regex::Regex;

use crate::{
    entities,
    hl::{classes, session::FinishedSpan},
    html::{CLASS_RE, ID_ATTR_RE},
};

const LEGACY_DONE_CLASS: &str = "highlit";

static LEGACY_SPAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<span class="([a-zA-Z0-9_-]+)">|</span>"#).unwrap());

#[derive(PartialEq, Eq, Clone)]
pub enum ElementKind {
    Pre,
    Code,
}

pub struct Element {
    pub kind: ElementKind,
    pub start: usize,
    pub end: usize,
    /// `end`, or the end of a trailing `<script>` block (from a previous
    /// run) that should be replaced along with the element itself.
    pub full_end: usize,
    pub attrs: String,
    pub content: String,
    pub stripped_content: String,
    /// `Some(spans)` for a block already fully annotated under the *old*
    /// inline-`<span>` scheme: it can be converted straight to the new
    /// id/`color()` form without going through the TUI. `None` means an
    /// interactive pass is needed (never processed, or its legacy markup
    /// didn't parse cleanly).
    pub legacy_spans: Option<Vec<FinishedSpan>>,
}

pub fn find(html: &str) -> Vec<Element> {
    let mut elements: Vec<Element> = find_elements_of_kind(html, &ElementKind::Pre)
        .into_iter()
        .chain(find_elements_of_kind(html, &ElementKind::Code))
        .collect();
    elements.sort_by_key(|e| e.start);
    let pre_ranges: Vec<_> =
        elements.iter().filter(|e| e.kind == ElementKind::Pre).map(|e| e.start .. e.end).collect();
    elements
        .retain(|e| e.kind == ElementKind::Pre || !pre_ranges.iter().any(|r| r.contains(&e.start)));
    elements
}

fn find_elements_of_kind(html: &str, kind: &ElementKind) -> Vec<Element> {
    let tag = match kind {
        ElementKind::Pre => "pre",
        ElementKind::Code => "code",
    };
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut elements = Vec::new();
    let mut pos = 0;
    while let Some(start) = html[pos ..].find(&open).map(|i| i + pos) {
        let after_tag = start + open.len();
        if !matches!(html.as_bytes().get(after_tag), Some(b'>' | b' ' | b'\n' | b'\t')) {
            pos = after_tag;
            continue;
        }
        let attrs_start = after_tag;
        let Some(tag_close) = html[attrs_start ..].find('>').map(|i| i + attrs_start) else {
            break;
        };
        let attrs = &html[attrs_start .. tag_close];
        let content_start = tag_close + 1;
        let Some(end_offset) = html[content_start ..].find(&close) else { break };
        let content_end = content_start + end_offset;
        let end = content_end + close.len();
        let content = html[content_start .. content_end].to_owned();
        let stripped_content = {
            let s = LEGACY_SPAN_RE.replace_all(&content, "");
            let s = entities::decode_basic_unconditional(&s);
            s.strip_prefix('\n').map_or_else(|| s.clone(), str::to_owned)
        };
        if let Some(legacy_spans) = classify(attrs, kind, &content, &stripped_content, html, end) {
            let full_end = trailing_script_end(html, end);
            elements.push(Element {
                kind: kind.clone(),
                start,
                end,
                full_end,
                attrs: attrs.to_owned(),
                content,
                stripped_content,
                legacy_spans,
            });
        }
        pos = end;
    }
    elements
}

/// Returns `None` if the block is already up to date (new scheme with a
/// matching hash and a companion script tag). Otherwise returns
/// `Some(legacy_spans)`: `Some(Some(spans))` for a clean migration,
/// `Some(None)` for a block that needs a fresh interactive pass.
#[allow(clippy::option_option, reason = "^that")]
fn classify(
    attrs: &str,
    kind: &ElementKind,
    content: &str,
    stripped_content: &str,
    html: &str,
    end: usize,
) -> Option<Option<Vec<FinishedSpan>>> {
    if let Some(id) = ID_ATTR_RE.captures(attrs).map(|c| c.get(1).unwrap().as_str().to_owned())
        && id_matches_hash(&id, &content_hash(stripped_content))
        && has_color_script(html, end, &id)
    {
        return None;
    }
    let classes_here: Vec<&str> = CLASS_RE
        .captures(attrs)
        .map(|c| c.get(1).unwrap().as_str().split_whitespace().collect())
        .unwrap_or_default();
    if *kind == ElementKind::Code && classes_here.iter().any(|cls| classes::by_name(cls).is_some())
    {
        return None; // old or new single-class inline-code fast path
    }
    if classes_here.contains(&LEGACY_DONE_CLASS)
        && let Some(spans) = parse_legacy_spans(content)
    {
        return Some(Some(spans));
    }
    Some(None)
}

fn has_color_script(html: &str, after: usize, id: &str) -> bool {
    let rest = &html[after ..];
    let skipped = rest.len() - rest.trim_start_matches([' ', '\t', '\n']).len();
    let tag_start = after + skipped;
    let body = &html[tag_start ..];
    let Some(inner) = body.strip_prefix("<script>") else { return false };
    let Some(close_rel) = inner.find("</script>") else { return false };
    let script_body = &inner[.. close_rel];
    let expected = format!(r#"color("{id}", ["#);
    let expected_empty = format!(r#"color("{id}", [])"#);
    script_body.starts_with(&expected) || script_body.starts_with(&expected_empty)
}

fn parse_legacy_spans(content: &str) -> Option<Vec<FinishedSpan>> {
    let content = content.strip_prefix('\n').unwrap_or(content);
    let mut spans = Vec::new();
    let mut open: Option<(usize, &'static str)> = None;
    let mut stripped_pos = 0_usize;
    let mut last_byte = 0_usize;
    for caps in LEGACY_SPAN_RE.captures_iter(content) {
        let m = caps.get(0).unwrap();
        stripped_pos +=
            entities::decode_basic_unconditional(&content[last_byte .. m.start()]).chars().count();
        last_byte = m.end();
        if let Some(class_name) = caps.get(1) {
            if open.is_some() {
                return None; // nested spans - not a shape we ever produced
            }
            let info = classes::by_name(class_name.as_str())?;
            open = Some((stripped_pos, info.name));
        } else {
            let (start, class_name) = open.take()?;
            spans.push(FinishedSpan { start, end: stripped_pos, class_name });
        }
    }
    if open.is_some() {
        return None; // unbalanced
    }
    let total = entities::decode_basic_unconditional(content).chars().count();
    if spans.iter().any(|s| s.end > total) {
        return None; // offsets don't add up - bail rather than feed build_tokens garbage
    }
    Some(spans)
}

fn trailing_script_end(html: &str, after: usize) -> usize {
    let rest = &html[after ..];
    let skipped = rest.len() - rest.trim_start_matches([' ', '\t', '\n']).len();
    let tag_start = after + skipped;
    let body = &html[tag_start ..];
    let Some(inner) = body.strip_prefix("<script>") else { return after };
    let Some(close_rel) = inner.find("</script>") else { return after };
    tag_start + "<script>".len() + close_rel + "</script>".len()
}

fn content_hash(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn id_matches_hash(id: &str, hash_hex: &str) -> bool {
    let expected = format!("hl-{hash_hex}");
    id == expected
        || id
            .strip_prefix(&format!("{expected}-"))
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

fn dedup_id(hash_hex: &str, used: &mut HashSet<String>) -> String {
    let base = format!("hl-{hash_hex}");
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2_u32;
    loop {
        let candidate = format!("{base}-{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

pub fn apply_spans(
    element: &Element,
    spans: &[FinishedSpan],
    used_ids: &mut HashSet<String>,
) -> String {
    let tag = match element.kind {
        ElementKind::Pre => "pre",
        ElementKind::Code => "code",
    };
    if element.kind == ElementKind::Code {
        let char_count = element.stripped_content.chars().count();
        if let [span] = spans
            && span.start == 0
            && span.end == char_count
        {
            let attrs = set_class(&element.attrs, span.class_name);
            return format!(
                "<{tag}{attrs}>{}</{tag}>",
                entities::encode_basic(&element.stripped_content)
            );
        }
    }
    let hash_hex = content_hash(&element.stripped_content);
    let id = dedup_id(&hash_hex, used_ids);
    let attrs = set_id(&element.attrs, &id);
    let tokens = build_tokens(&element.stripped_content, spans);
    let script = tokens_to_script(&id, &tokens);
    let encoded = entities::encode_basic(&element.stripped_content);
    let body = if element.content.starts_with('\n') { format!("\n{encoded}") } else { encoded };
    format!("<{tag}{attrs}>{body}</{tag}>\n{script}")
}

fn build_tokens(content: &str, spans: &[FinishedSpan]) -> Vec<(String, Option<&'static str>)> {
    let char_to_byte: Vec<usize> =
        content.char_indices().map(|(b, _)| b).chain(std::iter::once(content.len())).collect();
    let mut spans = spans.to_vec();
    spans.sort_by_key(|s| s.start);
    let mut tokens = Vec::new();
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor {
            continue;
        }
        if span.start > cursor {
            tokens
                .push((content[char_to_byte[cursor] .. char_to_byte[span.start]].to_owned(), None));
        }
        tokens.push((
            content[char_to_byte[span.start] .. char_to_byte[span.end]].to_owned(),
            Some(span.class_name),
        ));
        cursor = span.end;
    }
    let total = content.chars().count();
    if cursor < total {
        tokens.push((content[char_to_byte[cursor] ..].to_owned(), None));
    }
    tokens
}

fn tokens_to_script(id: &str, tokens: &[(String, Option<&'static str>)]) -> String {
    let items: Vec<String> = tokens
        .iter()
        .map(|(text, class_name)| {
            let class_js = class_name.map_or_else(|| "null".to_owned(), |c| format!("\"{c}\""));
            format!("[{}, {class_js}]", js_string_literal(text))
        })
        .collect();
    format!("<script>color({}, [{}]);</script>", js_string_literal(id), items.join(","))
}

fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn set_class(attrs: &str, class_name: &str) -> String {
    CLASS_RE.captures(attrs).map_or_else(
        || format!(r#"{attrs} class="{class_name}""#),
        |caps| {
            let existing = caps.get(1).unwrap().as_str();
            attrs.replacen(
                &format!(r#"class="{existing}""#),
                &format!(r#"class="{class_name}""#),
                1,
            )
        },
    )
}

fn set_id(attrs: &str, id: &str) -> String {
    ID_ATTR_RE.captures(attrs).map_or_else(
        || format!(r#"{attrs} id="{id}""#),
        |caps| {
            let existing = caps.get(1).unwrap().as_str();
            attrs.replacen(&format!(r#"id="{existing}""#), &format!(r#"id="{id}""#), 1)
        },
    )
}
