use std::sync::LazyLock;

use regex::Regex;

pub static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?s)<!--.*?-->|<[a-zA-Z0-9/][^>]*>").unwrap());

pub static CLASS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\sclass="([^"]*)""#).unwrap());

pub static ID_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\sid="([^"]*)""#).unwrap());

pub fn strip_tags(html: &str) -> String { TAG_RE.replace_all(html, "").into_owned() }
