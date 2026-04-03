use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    path::PathBuf,
};

use glob::glob;
use notes::{
    colors::{CYAN, GREEN, RED, RESET, YELLOW},
    entities, hl, math,
    tex::{
        self,
        accents::COMBINING,
        fonts::{self, FONT_ALIASES, FONT_PREDICATES, FontMaps},
        macros::MACROS,
    },
    toc, unicode,
    unicode::UnicodeData,
};
use notify_debouncer_full::{
    DebouncedEvent, new_debouncer, notify,
    notify::{EventKind, event::ModifyKind},
};

struct Pipeline {
    font_maps: FontMaps,
    unicode: UnicodeData,
    no_hl: bool,
}

impl Pipeline {
    fn run(&self, path: &PathBuf, check: bool) -> bool {
        if !path.exists() {
            println!("  {YELLOW}skipping:{RESET} doesn't exist");
            return false;
        }
        let original = match std::fs::read_to_string(path) {
            Ok(s) => s.replace("\r\n", "\n"),
            Err(e) => {
                println!("  {RED}reading error:{RESET} {e}");
                return false;
            }
        };
        let mut converted = original.clone();
        let mut changed = false;
        let mut okay = true;
        let entities_result = entities::replace(&converted);
        if entities_result != converted {
            println!("  {GREEN}entities done{RESET}");
            converted = entities_result;
            changed = true;
        }
        let (math_result, math_regions) = math::process(
            &converted,
            &self.font_maps,
            &self.unicode.superscripts,
            &self.unicode.subscripts,
            &self.unicode.negations,
        );
        math::warn_unknown(&math_result, &math_regions);
        if math_result != converted {
            println!("  {GREEN}math done{RESET}");
            converted = math_result;
            changed = true;
        }
        let toc_result = toc::process(&converted);
        if toc_result != converted {
            println!("  {GREEN}toc done{RESET}");
            converted = toc_result;
            changed = true;
        }
        if check {
            if changed {
                println!("  {CYAN}not writing the file due to `--check`{RESET}");
            }
            return changed;
        }
        if !self.no_hl {
            match hl::process_file(path, &converted) {
                Ok(hl_result) => {
                    if hl_result != converted {
                        converted = hl_result;
                        changed = true;
                    }
                }
                Err(e) => {
                    println!("  {RED}highlighting error:{RESET} {e}");
                    okay = false;
                }
            }
        }
        if changed {
            if std::fs::write(path, &converted).is_ok() {
                println!("  {GREEN}file written{RESET}");
            } else {
                println!("  {RED}writing error{RESET}");
                okay = false;
            }
        }
        changed && okay
    }
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let watch = args.iter().any(|a| a == "--watch");
    let no_hl = args.iter().any(|a| a == "--no-hl") || watch;
    let paths: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .collect();
    let files: Vec<PathBuf> = if paths.is_empty() {
        match glob("**/*.html") {
            Err(e) => {
                println!("{RED}glob error:{RESET} {e}");
                std::process::exit(1);
            }
            Ok(paths) => paths.filter_map(Result::ok).collect(),
        }
    } else {
        paths
    };
    if files.is_empty() {
        println!("{YELLOW}no html files{RESET}");
        std::process::exit(1);
    }
    println!("checking for unicode updates");
    let unicode = match unicode::load() {
        Ok(u) => u,
        Err(e) => {
            println!("{RED}unicode error:{RESET} {e}");
            std::process::exit(1);
        }
    };
    let font_maps = fonts::build(&unicode.letters);
    for key in MACROS.keys() {
        if tex::STRUCTURAL.contains(key) {
            println!("{YELLOW}duplicate:{RESET} '{key}' is in both MACROS and STRUCTURAL");
        }
    }
    for key in COMBINING.keys() {
        if tex::STRUCTURAL.contains(key) {
            println!("{YELLOW}duplicate:{RESET} '{key}' is in both COMBINING and STRUCTURAL");
        }
    }
    for (font, _) in FONT_PREDICATES {
        if tex::STRUCTURAL.contains(font) {
            println!(
                "{YELLOW}duplicate:{RESET} '{font}' is in both FONT_PREDICATES and STRUCTURAL"
            );
        }
    }
    for (alias, _) in FONT_ALIASES.entries() {
        if tex::STRUCTURAL.contains(alias) {
            println!("{YELLOW}duplicate:{RESET} '{alias}' is in both FONT_ALIASES and STRUCTURAL");
        }
    }
    let pipeline = Pipeline {
        font_maps,
        unicode,
        no_hl,
    };
    let mut any_changes = false;
    for path in &files {
        println!("{}", path.display());
        if pipeline.run(path, check) {
            any_changes = true;
        }
    }
    println!("{GREEN}done{RESET}");
    if check && any_changes {
        std::process::exit(1);
    }
    if watch {
        println!("watching...");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut last_seen: HashMap<PathBuf, u64> = HashMap::new();
        let mut debouncer = new_debouncer(std::time::Duration::from_secs(1), None, tx).unwrap();
        for path in &files {
            debouncer
                .watch(path, notify::RecursiveMode::NonRecursive)
                .unwrap();
        }
        for events in rx.into_iter().flatten() {
            for DebouncedEvent { event, .. } in events {
                if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                    for path in &event.paths {
                        let Ok(content) = std::fs::read(path) else {
                            continue;
                        };
                        let hash = hash_bytes(&content);
                        if last_seen.get(path) == Some(&hash) {
                            continue;
                        }
                        last_seen.insert(path.clone(), hash);
                        println!("{}", path.display());
                        pipeline.run(path, false);
                        if let Ok(new_content) = std::fs::read(path) {
                            last_seen.insert(path.clone(), hash_bytes(&new_content));
                        }
                    }
                }
            }
        }
    }
}
