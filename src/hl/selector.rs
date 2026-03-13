use std::fmt::Write as _;

pub fn parse_selector(input: &str) -> Option<(String, String)> {
    let mut rest = input.trim();
    let tag_end = rest.find(|c| ".#[".contains(c)).unwrap_or(rest.len());
    let tag = &rest[..tag_end];
    if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    rest = &rest[tag_end..];
    let mut classes: Vec<&str> = Vec::new();
    let mut id: Option<&str> = None;
    let mut attrs: Vec<(&str, Option<&str>)> = Vec::new();
    while !rest.is_empty() {
        match rest.chars().next().unwrap() {
            '.' => {
                rest = &rest[1..];
                let end = rest.find(|c| ".#[".contains(c)).unwrap_or(rest.len());
                let cls = &rest[..end];
                if cls.is_empty() {
                    return None;
                }
                classes.push(cls);
                rest = &rest[end..];
            }
            '#' => {
                rest = &rest[1..];
                let end = rest.find(|c| ".#[".contains(c)).unwrap_or(rest.len());
                let val = &rest[..end];
                if val.is_empty() {
                    return None;
                }
                id = Some(val);
                rest = &rest[end..];
            }
            '[' => {
                rest = &rest[1..];
                let close = rest.find(']')?;
                let inside = &rest[..close];
                rest = &rest[close + 1..];
                if let Some(eq) = inside.find('=') {
                    let attr = &inside[..eq];
                    let val = &inside[eq + 1..];
                    let val = val.trim_matches(|c| c == '"' || c == '\'');
                    attrs.push((attr, Some(val)));
                } else {
                    attrs.push((inside, None));
                }
            }
            _ => return None,
        }
    }
    let mut open = format!("<{tag}");
    if !classes.is_empty() {
        let _ = write!(open, r#" class="{}""#, classes.join(" "));
    }
    if let Some(id_val) = id {
        let _ = write!(open, r#" id="{id_val}""#);
    }
    for (attr, val) in attrs {
        if let Some(v) = val {
            let _ = write!(open, r#" {attr}="{v}""#);
        } else {
            let _ = write!(open, " {attr}");
        }
    }
    open.push('>');
    Some((open, tag.to_owned()))
}
