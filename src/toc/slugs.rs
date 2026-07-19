use std::collections::{HashMap, HashSet};

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
        } else if c == '\'' || c == '’' {
            // drop
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
            println!("{YELLOW}duplicate slug:{RESET} '{base}', using '{candidate}'");
            return candidate;
        }
        n += 1;
    }
}

pub fn assign_slugs(entries: &mut [Entry]) {
    let mut flats: Vec<Option<String>> = Vec::with_capacity(entries.len());
    for e in entries.iter() {
        let level = e.effective_level as usize;
        if level == 0 || level > 3 || (e.fake && !e.scoped) {
            flats.push(None);
            continue;
        }
        if e.fake && e.slug_segment.is_empty() {
            flats.push(None);
            continue;
        }
        flats.push(Some(e.slug_segment.clone()));
    }
    let mut flat_counts: HashMap<String, usize> = HashMap::new();
    for flat in flats.iter().flatten() {
        *flat_counts.entry(flat.clone()).or_insert(0) += 1;
    }
    let mut used = HashSet::new();
    let mut scope: [Option<String>; 3] = [None, None, None];
    for (i, e) in entries.iter_mut().enumerate() {
        let level = e.effective_level as usize;
        if level == 0 || level > 3 || (e.fake && !e.scoped) {
            continue;
        }
        for s in scope.iter_mut().skip(level) {
            *s = None;
        }
        if e.fake && e.slug_segment.is_empty() {
            continue;
        }
        let flat = flats[i].as_ref().unwrap();
        let count = flat_counts.get(flat).copied().unwrap_or(0);
        let base = if count > 1 {
            let parent = (level > 1)
                .then(|| scope[.. level - 1].iter().rev().find_map(|s| s.as_deref()))
                .flatten();
            parent.map_or_else(|| flat.clone(), |p| format!("{p}--{flat}"))
        } else {
            flat.clone()
        };

        let slug = dedup_slug(&base, &mut used);
        e.full_slug.clone_from(&slug);
        scope[level - 1] = Some(slug);
    }
}
