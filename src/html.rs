use std::sync::LazyLock;

use regex::Regex;

use crate::colors::{RESET, YELLOW};

pub static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?s)<!--.*?-->|<[a-zA-Z0-9/][^>]*>").unwrap());

pub static CLASS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\sclass="([^"]*)""#).unwrap());

pub static ID_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\sid="([^"]*)""#).unwrap());

static IMG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<img\b[^>]*>").unwrap());

static SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<script\b[^>]*>.*?</script>|<style\b[^>]*>.*?</style>").unwrap()
});

pub fn strip_tags(html: &str) -> String {
    let no_scripts = SCRIPT_RE.replace_all(html, "");
    TAG_RE.replace_all(&no_scripts, "").into_owned()
}

pub fn warn_missing_alt(html: &str) -> bool {
    let missing = IMG_RE.find_iter(html).filter(|m| !m.as_str().contains("alt=")).count();
    if missing > 0 {
        let plural = if missing == 1 { "" } else { "s" };
        println!(
            "  {YELLOW}missing alt text:{RESET} {missing} image{plural} with no alt attribute"
        );
        true
    } else {
        false
    }
}

pub fn apply_edits(html: &str, mut edits: Vec<(usize, usize, String)>) -> String {
    if edits.is_empty() {
        return html.to_owned();
    }
    edits.sort_by_key(|e| e.0);
    let mut result = html.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        result.replace_range(start .. end, &replacement);
    }
    result
}
