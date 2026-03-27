use std::fmt::Write as _;

use crate::{
    hl::{classes, session::FinishedSpan},
    html::{CLASS_RE, strip_tags},
};

const DONE_CLASS: &str = "highlit";

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
        elements.iter().filter(|e| e.kind == ElementKind::Pre).map(|e| e.start..e.end).collect();
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
    while let Some(start) = html[pos..].find(&open).map(|i| i + pos) {
        let after_tag = start + open.len();
        if !matches!(html.as_bytes().get(after_tag), Some(b'>' | b' ' | b'\n' | b'\t')) {
            pos = after_tag;
            continue;
        }
        let attrs_start = after_tag;
        let Some(tag_close) = html[attrs_start..].find('>').map(|i| i + attrs_start) else { break };
        let attrs = &html[attrs_start..tag_close];
        let content_start = tag_close + 1;
        let Some(end_offset) = html[content_start..].find(&close) else { break };
        let content_end = content_start + end_offset;
        let end = content_end + close.len();
        if !has_done_class(attrs, kind) {
            let content = html[content_start..content_end].to_owned();
            let stripped_content = {
                let s = strip_tags(&content);
                s.strip_prefix('\n').unwrap_or(&s).to_owned()
            };
            eprintln!("{stripped_content:?}");
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

fn has_done_class(attrs: &str, kind: &ElementKind) -> bool {
    CLASS_RE.captures(attrs).is_some_and(|c| {
        c.get(1).unwrap().as_str().split_whitespace().any(|cls| {
            cls == DONE_CLASS || (*kind == ElementKind::Code && classes::by_name(cls).is_some())
        })
    })
}

pub fn apply_spans(element: &Element, spans: &[FinishedSpan]) -> String {
    let tag = match element.kind {
        ElementKind::Pre => "pre",
        ElementKind::Code => "code",
    };
    if element.kind == ElementKind::Code {
        let char_count = element.stripped_content.chars().count();
        if let [span] = spans
            && span.start == 0
            && span.end == char_count
            && span.close == "</span>"
        {
            let class_name =
                span.open.trim_start_matches(r#"<span class=""#).trim_end_matches(r#"">"#);
            let attrs = set_class(&element.attrs, class_name);
            let attrs = add_done_class(&attrs);
            return format!("<{tag}{attrs}>{}</{tag}>", element.stripped_content);
        }
    }
    let content = apply_spans_to_content(&element.stripped_content, spans);
    let content = if element.content.starts_with('\n') { format!("\n{content}") } else { content };
    let attrs = add_done_class(&element.attrs);
    format!("<{tag}{attrs}>{content}</{tag}>")
}

fn apply_spans_to_content(content: &str, spans: &[FinishedSpan]) -> String {
    let char_to_byte: Vec<usize> =
        content.char_indices().map(|(b, _)| b).chain(std::iter::once(content.len())).collect();
    let mut result = String::new();
    let mut cursor = 0;
    let mut spans = spans.to_vec();
    spans.sort_by_key(|s| s.start);
    for span in spans {
        if span.start < cursor {
            continue;
        }
        result.push_str(&content[char_to_byte[cursor]..char_to_byte[span.start]]);
        let _ = write!(
            result,
            "{}{}{}",
            span.open,
            &content[char_to_byte[span.start]..char_to_byte[span.end]],
            span.close,
        );
        cursor = span.end;
    }
    result.push_str(&content[char_to_byte[cursor]..]);
    result
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

fn add_done_class(attrs: &str) -> String {
    CLASS_RE.captures(attrs).map_or_else(
        || format!(r#"{attrs} class="{DONE_CLASS}""#),
        |caps| {
            let existing = caps.get(1).unwrap().as_str();
            let new_class = format!("{existing} {DONE_CLASS}");
            attrs.replacen(&format!(r#"class="{existing}""#), &format!(r#"class="{new_class}""#), 1)
        },
    )
}
