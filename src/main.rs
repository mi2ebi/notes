use std::path::PathBuf;

use glob::glob;
use notes::{
    colors::{CYAN, GREEN, RED, RESET, YELLOW},
    hl::highlight,
    process,
    tex::{
        accents::COMBINING,
        fonts::{self, FONT_ALIASES, FONT_PREDICATES},
        macros::MACROS,
    },
    unicode_data,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let no_hl = args.iter().any(|a| a == "--no-hl");
    let paths: Vec<PathBuf> =
        args.iter().filter(|a| !a.starts_with("--")).map(PathBuf::from).collect();
    let files: Vec<PathBuf> = if paths.is_empty() {
        match glob("**/*.html") {
            Err(e) => {
                eprintln!("{RED}glob error:{RESET} {e}");
                std::process::exit(1);
            }
            Ok(paths) => paths.filter_map(Result::ok).collect(),
        }
    } else {
        paths
    };
    if files.is_empty() {
        eprintln!("{YELLOW}no html files{RESET}");
        std::process::exit(1);
    }
    println!("{CYAN}checking for unicode updates{RESET}");
    let unicode = match unicode_data::load() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{RED}unicode error:{RESET} {e}");
            std::process::exit(1);
        }
    };
    let font_maps = fonts::build(&unicode.letters);
    let superscripts = &unicode.superscripts;
    let subscripts = &unicode.subscripts;
    let negations = &unicode.negations;
    for key in MACROS.keys() {
        if process::STRUCTURAL.contains(key) {
            eprintln!("{YELLOW}duplicate:{RESET} '{key}' is in both MACROS and STRUCTURAL");
        }
    }
    for key in COMBINING.keys() {
        if process::STRUCTURAL.contains(key) {
            eprintln!("{YELLOW}duplicate:{RESET} '{key}' is in both COMBINING and STRUCTURAL");
        }
    }
    for (font, _) in FONT_PREDICATES {
        if process::STRUCTURAL.contains(font) {
            eprintln!(
                "{YELLOW}duplicate:{RESET} '{font}' is in both FONT_PREDICATES and STRUCTURAL"
            );
        }
    }
    for (alias, _) in FONT_ALIASES.entries() {
        if process::STRUCTURAL.contains(alias) {
            eprintln!("{YELLOW}duplicate:{RESET} '{alias}' is in both FONT_ALIASES and STRUCTURAL");
        }
    }
    let mut any_changes = false;
    for path in &files {
        println!("{}", path.display());
        if !path.exists() {
            println!("  {YELLOW}skipping:{RESET} doesn't exist");
            continue;
        }
        let original = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {RED}reading error:{RESET} {e}");
                continue;
            }
        };
        let (converted, math_regions) =
            process::process(&original, &font_maps, superscripts, subscripts, negations);
        process::warn_unknown(&converted, &math_regions);
        if converted == original {
        } else if check {
            println!("  {CYAN}possible{RESET}");
            any_changes = true;
        } else {
            if std::fs::write(path, &converted).is_err() {
                eprintln!("  {RED}writing error{RESET}");
                continue;
            }
            println!("  {GREEN}done{RESET}");
        }
        if !check
            && !no_hl
            && let Err(e) = highlight::process_file(path, &converted)
        {
            eprintln!("  {RED}error:{RESET} {e}");
        }
    }
    if check && any_changes {
        std::process::exit(1);
    }
}
