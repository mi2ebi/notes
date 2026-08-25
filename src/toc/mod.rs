mod attrs;
mod slugs;

use std::{fmt::Write as _, sync::LazyLock};

use attrs::{TocAttrs, parse_toc_comment};
use regex::Regex;
use slugs::{assign_slugs, slugify};

use crate::html::{ID_ATTR_RE, apply_edits, strip_tags};

static TOC_HERE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<!--\s*toc\s+here\s*-->\n?(?:\s*<nav class="toc">.*?</nav>\n?)?"#).unwrap()
});

static HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?s)<h([1-3])([^>]*)>(.*?)</h[1-3]>").unwrap());

static TOC_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--\s*toc\b(.*?)-->").unwrap());

#[allow(clippy::struct_excessive_bools, reason = "flags")]
pub struct Entry {
    pub effective_level: u8,
    pub display_text: String,
    pub slug_segment: String,
    pub full_slug: String,
    pub fake: bool,
    pub skip: bool,
    pub nolink: bool,
    pub scoped: bool,
    pub id_insert_pos: Option<usize>,
    pub existing_id: Option<(usize, usize)>,
}

enum HtmlItem {
    TocComment { start: usize, attrs: TocAttrs },
    Heading { start: usize, level: u8, attrs_start: usize, attrs_end: usize, content: String },
}

fn collect_entries(html: &str, toc_range: &std::ops::Range<usize>) -> Vec<Entry> {
    let mut items: Vec<HtmlItem> = Vec::new();
    for caps in TOC_COMMENT_RE.captures_iter(html) {
        let m = caps.get(0).unwrap();
        if toc_range.contains(&m.start()) {
            continue;
        }
        if let Some(attrs) = parse_toc_comment(m.as_str()) {
            items.push(HtmlItem::TocComment { start: m.start(), attrs });
        }
    }
    for caps in HEADING_RE.captures_iter(html) {
        let m = caps.get(0).unwrap();
        if toc_range.contains(&m.start()) {
            continue;
        }
        let level: u8 = caps.get(1).unwrap().as_str().parse().unwrap();
        let attrs_m = caps.get(2).unwrap();
        items.push(HtmlItem::Heading {
            start: m.start(),
            level,
            attrs_start: attrs_m.start(),
            attrs_end: attrs_m.end(),
            content: caps.get(3).unwrap().as_str().to_owned(),
        });
    }
    items.sort_by_key(|item| match item {
        HtmlItem::TocComment { start, .. } | HtmlItem::Heading { start, .. } => *start,
    });
    let mut entries = Vec::new();
    let mut pending: Option<TocAttrs> = None;
    for item in items {
        match item {
            HtmlItem::TocComment { attrs, .. } => {
                if attrs.fake {
                    entries.push(Entry {
                        effective_level: attrs.level.unwrap_or(1),
                        display_text: attrs.label.unwrap_or_default(),
                        slug_segment: attrs.id_segment.unwrap_or_default(),
                        full_slug: String::new(),
                        fake: true,
                        skip: attrs.skip,
                        nolink: true,
                        scoped: attrs.scoped,
                        id_insert_pos: None,
                        existing_id: None,
                    });
                    pending = None;
                } else {
                    pending = Some(attrs);
                }
            }
            HtmlItem::Heading { level, attrs_start, attrs_end, content, .. } => {
                let d = pending.take().unwrap_or_default();
                if d.skip {
                    continue;
                }
                let text = strip_tags(&content);
                let display_text = d.label.unwrap_or_else(|| text.clone());
                let slug_segment = d.id_segment.unwrap_or_else(|| slugify(&text));
                let effective_level = d.level.unwrap_or(level);
                let existing_id = ID_ATTR_RE
                    .find(&html[attrs_start .. attrs_end])
                    .map(|id_m| (attrs_start + id_m.start(), attrs_start + id_m.end()));
                entries.push(Entry {
                    effective_level,
                    display_text,
                    slug_segment,
                    full_slug: String::new(),
                    fake: false,
                    skip: false,
                    nolink: d.nolink,
                    scoped: true,
                    id_insert_pos: Some(attrs_start),
                    existing_id,
                });
            }
        }
    }
    entries
}

fn build_nav(entries: &[Entry]) -> String {
    let visible: Vec<_> = entries.iter().filter(|e| !e.skip).collect();
    if visible.is_empty() {
        return String::new();
    }
    let mut out = String::from("<nav class=\"toc\">\n");
    let mut depth: u8 = 0;
    let mut prev_had_children = false;
    for (i, e) in visible.iter().enumerate() {
        let next_level = visible.get(i + 1).map(|n| n.effective_level);
        let has_children = next_level > Some(e.effective_level);
        if e.effective_level > depth {
            while depth < e.effective_level {
                depth += 1;
                let list_indent = "  ".repeat((2 * depth as usize).saturating_sub(1));
                out.push_str(&list_indent);
                out.push_str("<ul class=\"tight\">\n");
            }
        } else {
            if prev_had_children {
                let item_indent = "  ".repeat(2 * depth as usize);
                out.push_str(&item_indent);
                out.push_str("</li>\n");
            }
            while depth > e.effective_level {
                let list_indent = "  ".repeat((2 * depth as usize).saturating_sub(1));
                out.push_str(&list_indent);
                out.push_str("</ul>\n");
                depth -= 1;
                if depth > 0 {
                    let item_indent = "  ".repeat(2 * depth as usize);
                    out.push_str(&item_indent);
                    out.push_str("</li>\n");
                }
            }
        }
        let item_indent = "  ".repeat(2 * depth as usize);
        out.push_str(&item_indent);
        if e.nolink {
            let _ = write!(out, "<li>{}", e.display_text);
        } else {
            let _ = write!(out, "<li><a href=\"#{}\">{}</a>", e.full_slug, e.display_text);
        }
        if has_children {
            out.push('\n');
        } else {
            out.push_str("</li>\n");
        }
        prev_had_children = has_children;
    }
    if prev_had_children {
        let item_indent = "  ".repeat(2 * depth as usize);
        out.push_str(&item_indent);
        out.push_str("</li>\n");
    }
    while depth > 0 {
        let list_indent = "  ".repeat((2 * depth as usize).saturating_sub(1));
        out.push_str(&list_indent);
        out.push_str("</ul>\n");
        depth -= 1;
        if depth > 0 {
            let item_indent = "  ".repeat(2 * depth as usize);
            out.push_str(&item_indent);
            out.push_str("</li>\n");
        }
    }
    out.push_str("</nav>\n");
    out
}

fn line_indent(html: &str, pos: usize) -> &str {
    let line_start = html[.. pos].rfind('\n').map_or(0, |i| i + 1);
    let prefix = &html[line_start .. pos];
    if prefix.chars().all(|c| c == ' ') { prefix } else { "" }
}

fn indent_lines(text: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len() + prefix.len() * (text.lines().count() + 1));
    for line in text.lines() {
        if !line.is_empty() {
            out.push_str(prefix);
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn process(html: &str) -> String {
    if TOC_HERE_RE.find(html).is_none() {
        return html.to_owned();
    }
    let toc_m = TOC_HERE_RE.find(html).unwrap();
    let toc_range = toc_m.start() .. toc_m.end();
    let base_indent = line_indent(html, toc_m.start());
    let mut entries = collect_entries(html, &toc_range);
    assign_slugs(&mut entries);
    let nav = indent_lines(&build_nav(&entries), base_indent);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let replacement = if nav.is_empty() {
        "<!-- toc here -->".to_string()
    } else {
        format!("<!-- toc here -->\n{nav}")
    };
    edits.push((toc_m.start(), toc_m.end(), replacement));
    for e in &entries {
        if e.fake || e.nolink {
            continue;
        }
        let id_attr = format!(r#" id="{}""#, e.full_slug);
        if let Some((start, end)) = e.existing_id {
            edits.push((start, end, id_attr));
        } else if let Some(pos) = e.id_insert_pos {
            edits.push((pos, pos, id_attr));
        }
    }
    apply_edits(html, edits)
}
