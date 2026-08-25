use std::{collections::HashSet, hash::Hasher as _, sync::LazyLock};

use regex::Regex;
use rustc_hash::FxHasher;

use crate::{entities, hl::session::FinishedSpan, html::ID_ATTR_RE};

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
    pub attrs: String,
    pub content: String,
    pub stripped_content: String,
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
        if needs_highlighting(attrs, &stripped_content) {
            elements.push(Element {
                kind: kind.clone(),
                start,
                end,
                attrs: attrs.to_owned(),
                content,
                stripped_content,
            });
        }
        pos = end;
    }
    elements
}

fn needs_highlighting(attrs: &str, stripped_content: &str) -> bool {
    if let Some(id) = ID_ATTR_RE.captures(attrs).map(|c| c.get(1).unwrap().as_str().to_owned())
        && id_matches_hash(&id, &content_hash(stripped_content))
    {
        return false;
    }
    true
}

pub fn content_hash(s: &str) -> String {
    let mut h = FxHasher::default();
    h.write(s.as_bytes());
    format!("{:016x}", h.finish())
}

fn id_matches_hash(id: &str, hash_hex: &str) -> bool {
    let expected = format!("hl-{hash_hex}");
    id == expected
        || id
            .strip_prefix(&format!("{expected}-"))
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

pub fn dedup_id(hash_hex: &str, used: &mut HashSet<String>) -> String {
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

pub fn apply_spans(element: &Element, spans: &[FinishedSpan], id: &str) -> (String, String) {
    let tag = match element.kind {
        ElementKind::Pre => "pre",
        ElementKind::Code => "code",
    };
    let attrs = set_id(&element.attrs, id);
    let tokens = build_tokens(&element.stripped_content, spans);
    let js = tokens_to_js(id, &tokens);
    let encoded = entities::encode_basic(&element.stripped_content);
    let body = if element.content.starts_with('\n') { format!("\n{encoded}") } else { encoded };
    (format!("<{tag}{attrs}>{body}</{tag}>"), js)
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

fn tokens_to_js(id: &str, tokens: &[(String, Option<&'static str>)]) -> String {
    let items: Vec<String> = tokens
        .iter()
        .map(|(text, class_name)| {
            let class_js = class_name.map_or_else(|| "null".to_owned(), |c| format!("\"{c}\""));
            format!("[{}, {class_js}]", js_string_literal(text))
        })
        .collect();
    format!("color({}, [{}]);", js_string_literal(id), items.join(", "))
}

fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '<' => out.push_str(r"\u003c"),
            '&' => out.push_str(r"\u0026"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
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
