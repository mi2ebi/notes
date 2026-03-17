pub mod classes;
pub mod elements;
pub mod selector;
pub mod session;
pub mod tui;

use std::{fs, io, path::Path};

use elements::{apply_spans, find};
use session::Session;
use tui::TuiResult;

use crate::colors::{GREEN, RESET};

pub fn process_file(path: &Path, html: &str) -> io::Result<bool> {
    let elements = find(html);
    if elements.is_empty() {
        return Ok(false);
    }
    let mut result = html.to_owned();
    for (i, element) in elements.iter().rev().enumerate() {
        let mut session = Session::new(&element.stripped_content);
        let status = format!("{}  block {}/{}", path.display(), i + 1, elements.len());
        match tui::run(&mut session, &status)? {
            TuiResult::SkipAll => break,
            TuiResult::Discard => continue,
            TuiResult::Done => {}
        }
        let spans = session.finish();
        let replacement = apply_spans(element, &spans);
        result.replace_range(element.start..element.end, &replacement);
    }
    if result == html {
        return Ok(false);
    }
    fs::write(path, &result)?;
    println!("  {GREEN}done{RESET}");
    Ok(true)
}
