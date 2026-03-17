#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct TocAttrs {
    pub fake: bool,
    pub nolink: bool,
    pub skip: bool,
    pub scoped: bool,
    pub level: Option<u8>,
    pub label: Option<String>,
    pub id_segment: Option<String>,
}

pub fn parse_toc_comment(comment: &str) -> Option<TocAttrs> {
    let inner = comment.strip_prefix("<!--")?.strip_suffix("-->")?;
    let after_toc = inner.trim().strip_prefix("toc")?.trim_start();
    if after_toc == "here" || after_toc.starts_with("here ") || after_toc.starts_with("here\t") {
        return None;
    }
    Some(parse_attrs(after_toc))
}

pub fn parse_attrs(s: &str) -> TocAttrs {
    let mut attrs = TocAttrs::default();
    let mut rest = s.trim();
    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let key_end =
            rest.find(|c: char| c.is_ascii_whitespace() || c == ':').unwrap_or(rest.len());
        let key = &rest[..key_end];
        if key.is_empty() {
            break;
        }
        rest = &rest[key_end..];
        if rest.starts_with(':') {
            rest = &rest[1..];
            if rest.starts_with(' ') {
                rest = &rest[1..];
                let (val, remaining) = parse_multiword(rest);
                set_value_attr(&mut attrs, key, val);
                rest = remaining;
            } else {
                let val_end = rest.find(|c: char| c.is_ascii_whitespace()).unwrap_or(rest.len());
                set_value_attr(&mut attrs, key, &rest[..val_end]);
                rest = &rest[val_end..];
            }
        } else {
            match key {
                "fake" => attrs.fake = true,
                "nolink" => attrs.nolink = true,
                "skip" => attrs.skip = true,
                "scoped" => attrs.scoped = true,
                _ => {}
            }
        }
    }
    attrs
}

fn set_value_attr(attrs: &mut TocAttrs, key: &str, val: &str) {
    match key {
        "label" => attrs.label = Some(val.to_owned()),
        "id" => attrs.id_segment = Some(val.to_owned()),
        "level" => attrs.level = val.parse().ok(),
        _ => {}
    }
}

pub fn parse_multiword(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        if bytes[i] == b' ' && i + 1 < s.len() && bytes[i + 1] == b';' {
            let after = i + 2;
            if after >= s.len() || bytes[after] != b' ' {
                return (&s[..i], &s[after..]);
            }
        }
        i += 1;
    }
    (s.trim_end(), "")
}
