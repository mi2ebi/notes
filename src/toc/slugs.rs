use std::collections::HashSet;

use super::Entry;
use crate::colors::{RESET, YELLOW};

pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                slug.push(lc);
            }
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('-');
            last_was_sep = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

pub fn dedup_slug(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_owned()) {
        return base.to_owned();
    }
    let mut n = 2_u32;
    loop {
        let candidate = format!("{base}-{n}");
        if used.insert(candidate.clone()) {
            eprintln!("{YELLOW}duplicate slug:{RESET} '{base}', using '{candidate}'");
            return candidate;
        }
        n += 1;
    }
}

pub fn assign_slugs(entries: &mut [Entry]) {
    let mut scope: [Option<String>; 3] = [None, None, None];
    let mut used = HashSet::new();
    for e in entries.iter_mut() {
        let level = e.effective_level as usize;
        if level == 0 || level > 3 {
            continue;
        }
        if e.fake && !e.scoped {
            continue;
        }
        for s in scope.iter_mut().skip(level) {
            *s = None;
        }
        if e.fake && e.slug_segment.is_empty() {
            continue;
        }
        let parent = (level > 1)
            .then(|| scope[..level - 1].iter().rev().find_map(|s| s.as_deref()))
            .flatten();
        let base = match parent {
            Some(p) => format!("{p}--{}", e.slug_segment),
            None => e.slug_segment.clone(),
        };
        let slug = dedup_slug(&base, &mut used);
        scope[level - 1] = Some(slug.clone());
        e.full_slug = slug;
    }
}
