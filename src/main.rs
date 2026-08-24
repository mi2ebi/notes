use std::path::PathBuf;

use glob::glob;
use jiff::{Timestamp, tz::TimeZone};
use notes::{
    aside, boilerplate,
    colors::{CYAN, GREEN, RED, RESET, YELLOW},
    entities, hl, html, include, math,
    tex::{
        self,
        accents::COMBINING,
        fonts::{self, FONT_ALIASES, FONT_PREDICATES, FontMaps},
        macros::MACROS,
    },
    toc, unicode,
    unicode::UnicodeData,
};

struct Pipeline {
    font_maps: FontMaps,
    unicode: UnicodeData,
    no_hl: bool,
}

struct RunResult {
    changed: bool,
    okay: bool,
    had_warnings: bool,
}

impl Pipeline {
    fn run(&self, path: &PathBuf, check: bool) -> RunResult {
        if !path.exists() {
            println!("  {YELLOW}skipping:{RESET} doesn't exist");
            return RunResult { changed: false, okay: false, had_warnings: true };
        }
        let original = match std::fs::read_to_string(path) {
            Ok(s) => s.replace("\r\n", "\n"),
            Err(e) => {
                println!("  {RED}reading error:{RESET} {e}");
                return RunResult { changed: false, okay: false, had_warnings: true };
            }
        };
        let mut converted = original;
        let mut changed = false;
        let mut okay = true;
        let mut had_warnings = false;

        let include_result = include::process(&converted, path);
        if include_result != converted {
            println!("  {GREEN}include done{RESET}");
            converted = include_result;
            changed = true;
        }
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
        had_warnings |= math::warn_unknown(&math_result, &math_regions);
        if math_result != converted {
            println!("  {GREEN}math done{RESET}");
            converted = math_result;
            changed = true;
        }
        if let Some(new_html) =
            boilerplate::ensure_temml(&converted, path, !math_regions.is_empty())
        {
            println!("  {GREEN}temml include added{RESET}");
            converted = new_html;
            changed = true;
        }
        let toc_result = toc::process(&converted);
        if toc_result != converted {
            println!("  {GREEN}toc done{RESET}");
            converted = toc_result;
            changed = true;
        }
        let aside_result = aside::process(&converted);
        if aside_result != converted {
            println!("  {GREEN}aside done{RESET}");
            converted = aside_result;
            changed = true;
        }
        had_warnings |= html::warn_missing_alt(&converted);
        if check {
            if changed {
                println!("  {CYAN}not writing the file due to `--check`{RESET}");
            }
            return RunResult { changed, okay, had_warnings };
        }
        if !self.no_hl {
            match hl::process_file(path, &converted) {
                Ok(hl_result) => {
                    if hl_result != converted {
                        converted = hl_result;
                        changed = true;
                    }
                    let needs_color = converted.contains("color(");
                    if let Some(new_html) =
                        boilerplate::ensure_highlight_js(&converted, path, needs_color)
                    {
                        println!("  {GREEN}highlight.js include added{RESET}");
                        converted = new_html;
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
        RunResult { changed, okay, had_warnings }
    }
}

fn is_build_artifact(path: &std::path::Path) -> bool {
    path.components().any(|c| match c {
        std::path::Component::Normal(s) => {
            s == "target" || s.to_str().is_some_and(|s| s.starts_with('.'))
        }
        _ => false,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let no_hl = args.iter().any(|a| a == "--no-hl");
    let paths: Vec<PathBuf> =
        args.iter().filter(|a| !a.starts_with("--")).map(PathBuf::from).collect();
    let files: Vec<PathBuf> = if paths.is_empty() {
        match glob("**/*.html") {
            Err(e) => {
                println!("{RED}glob error:{RESET} {e}");
                std::process::exit(1);
            }
            Ok(paths) => paths.filter_map(Result::ok).filter(|p| !is_build_artifact(p)).collect(),
        }
    } else {
        paths
    };
    if files.is_empty() {
        println!("{YELLOW}no html files{RESET}");
        std::process::exit(1);
    }
    let should_check_unicode = {
        let date = Timestamp::now().to_zoned(TimeZone::UTC).date();
        let month = date.month();
        month % 3 == 2 && date.day() == 1
    };
    let unicode = if should_check_unicode {
        println!("checking for unicode updates");
        match unicode::load() {
            Ok(u) => u,
            Err(e) => {
                println!("{RED}unicode error:{RESET} {e}");
                std::process::exit(1);
            }
        }
    } else {
        match unicode::load_local() {
            Ok(u) => u,
            Err(e) => {
                println!("{RED}unicode error:{RESET} {e}");
                std::process::exit(1);
            }
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
    for key in tex::envs::ENV_SHORTHANDS.keys() {
        if tex::STRUCTURAL.contains(key) {
            println!("{YELLOW}duplicate:{RESET} '{key}' is in both ENV_SHORTHANDS and STRUCTURAL");
        }
    }
    let pipeline = Pipeline { font_maps, unicode, no_hl };
    let mut any_changes = false;
    for path in &files {
        println!("{}", path.display());
        let result = pipeline.run(path, check);
        if result.changed && result.okay {
            any_changes = true;
        }
        if !result.changed && !result.had_warnings && result.okay {
            print!("\x1b[A\r\x1b[K");
        }
    }
    println!("{GREEN}done{RESET}");
    if check && any_changes {
        std::process::exit(1);
    }
}
