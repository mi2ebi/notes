use std::path::PathBuf;

use glob::glob;
use notes::{
    accents,
    colors::{CYAN, GREEN, RED, RESET, YELLOW},
    fonts, macros, process, unicode_data,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let paths: Vec<PathBuf> =
        args.iter().filter(|a| !a.starts_with("--")).map(PathBuf::from).collect();

    let files: Vec<PathBuf> = if paths.is_empty() {
        match glob("**/*.html") {
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            Ok(paths) => paths.filter_map(Result::ok).collect(),
        }
    } else {
        paths
    };
    if files.is_empty() {
        eprintln!("no .html files found");
        std::process::exit(1);
    }
    let unicode = match unicode_data::load() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{RED}error:{RESET} glob: {e}");
            std::process::exit(1);
        }
    };
    let font_maps = fonts::build(&unicode.letters);
    let superscripts = &unicode.superscripts;
    let subscripts = &unicode.subscripts;
    let negations = &unicode.negations;
    for key in macros::MACROS.keys() {
        if process::STRUCTURAL.contains(key) {
            eprintln!("{YELLOW}duplicate:{RESET} '{key}' is in both MACROS and STRUCTURAL");
        }
    }
    for key in accents::COMBINING.keys() {
        if process::STRUCTURAL.contains(key) {
            eprintln!("{YELLOW}duplicate:{RESET} '{key}' is in both COMBINING and STRUCTURAL");
        }
    }
    for (font, _) in fonts::FONT_PREDICATES {
        if process::STRUCTURAL.contains(font) {
            eprintln!(
                "{YELLOW}duplicate:{RESET} '{font}' is in both FONT_PREDICATES and STRUCTURAL"
            );
        }
    }
    for (alias, _) in fonts::FONT_ALIASES.entries() {
        if process::STRUCTURAL.contains(alias) {
            eprintln!("{YELLOW}duplicate:{RESET} '{alias}' is in both FONT_ALIASES and STRUCTURAL");
        }
    }
    let mut any_changes = false;
    for path in &files {
        if !path.exists() {
            eprintln!("{CYAN}skipping:{RESET} couldn't find {}", path.display());
            continue;
        }
        let original = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{RED}error:{RESET} couldn't read {}: {e}", path.display());
                continue;
            }
        };
        let (converted, math_regions) =
            process::process(&original, &font_maps, superscripts, subscripts, negations);
        process::warn_unknown(&converted, &math_regions);
        if converted == original {
            println!("{CYAN}unchanged:{RESET} {}", path.display());
            continue;
        }
        if check {
            println!("{CYAN}would change:{RESET} {}", path.display());
            any_changes = true;
            continue;
        }
        if let Err(e) = std::fs::write(path, &converted) {
            eprintln!("{RED}error:{RESET} writing {}: {e}", path.display());
            continue;
        }
        let changed = original.lines().zip(converted.lines()).filter(|(a, b)| a != b).count();
        println!("{GREEN}modified:{RESET} {}: {changed} line(s) changed", path.display());
    }
    if check && any_changes {
        std::process::exit(1);
    }
}
