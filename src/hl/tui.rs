use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{self, Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::hl::{
    classes::{self, CLASSES},
    selector::parse_selector,
    session::{Session, SpanKind},
};

pub enum TuiResult {
    Done,
    Discard,
    SkipAll,
}

pub fn run(session: &mut Session, status: &str) -> io::Result<TuiResult> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let result = run_inner(session, &mut stdout, status);
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    result
}

enum Action {
    Advance,
    Commit(SpanKind),
    Undo,
    Tag,
    SaveBlock,
    DiscardBlock,
    QuitFile,
}

fn cursor_line(session: &Session) -> usize {
    session.chars[..session.cursor()].iter().filter(|&&c| c == '\n').count()
}

fn run_inner(
    session: &mut Session,
    stdout: &mut impl Write,
    status: &str,
) -> io::Result<TuiResult> {
    let mut scroll_offset: usize = 0;
    let mut prev_cells: Vec<Cell> = Vec::new();
    loop {
        let (_, height) = terminal::size()?;
        let content_height = height.saturating_sub(2) as usize;
        let cl = cursor_line(session);
        if cl < scroll_offset {
            scroll_offset = cl;
        } else if cl >= scroll_offset + content_height {
            scroll_offset = cl - content_height + 1;
        }
        render(session, stdout, scroll_offset, status, &mut prev_cells)?;
        match read_event()? {
            Action::Advance => session.advance(),
            Action::Commit(kind) => {
                session.commit(kind);
                if session.at_end() {
                    break;
                }
            }
            Action::Undo => session.undo(),
            Action::Tag => {
                let input = prompt(stdout, "\\")?;
                prev_cells.clear();
                if let Some((open, tag_name)) = input.as_deref().and_then(parse_selector) {
                    session.commit(SpanKind::Tag(open, tag_name));
                }
            }
            Action::SaveBlock => break,
            Action::DiscardBlock => {
                return Ok(TuiResult::Discard);
            }
            Action::QuitFile => return Ok(TuiResult::SkipAll),
        }
    }
    Ok(TuiResult::Done)
}

fn read_event() -> io::Result<Action> {
    loop {
        let Event::Key(KeyEvent { code, modifiers, kind: KeyEventKind::Press, .. }) =
            event::read()?
        else {
            continue;
        };
        let action = match (code, modifiers) {
            (KeyCode::Right, _) => Action::Advance,
            (KeyCode::Char('u'), KeyModifiers::NONE) => Action::Undo,
            (KeyCode::Char('\\'), KeyModifiers::NONE) => Action::Tag,
            (KeyCode::Enter, _) => Action::SaveBlock,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::DiscardBlock,
            (KeyCode::Char(' '), KeyModifiers::NONE) => Action::Commit(SpanKind::Skip),
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if let Some(info) = classes::by_key(ch) {
                    Action::Commit(SpanKind::Styled(info))
                } else {
                    continue;
                }
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Action::QuitFile,
            _ => continue,
        };
        return Ok(action);
    }
}

#[derive(Clone, PartialEq)]
enum CharStyle {
    Plain,
    Open,
    Bg(u8, u8, u8),
    Tag(String),
    Cursor,
}

const TRAILING_C: (u8, u8, u8) = (0x98, 0x98, 0x98);
const SELECTED_C: (u8, u8, u8) = (0x55, 0x5a, 0x66);
const TAG_C: (u8, u8, u8) = (0x55, 0x2f, 0x5f);
const CURSOR_C: (u8, u8, u8) = (0xff, 0x66, 0xff);
const CURSOR_WS_C: (u8, u8, u8) = (0xff, 0x59, 0x59);
const STATUS_C: (u8, u8, u8) = (0x83, 0xd7, 0xac);
const KEYBINDS_C: (u8, u8, u8) = (0xb6, 0xa0, 0xff);
const PROMPT_C: (u8, u8, u8) = (0x4a, 0xe2, 0xf0);

#[derive(Clone, PartialEq)]
struct Cell {
    ch: char,
    fg: Color,
    bg: Color,
}

impl Default for Cell {
    fn default() -> Self { Self { ch: ' ', fg: Color::Reset, bg: Color::Reset } }
}

const fn style_bg(style: &CharStyle) -> Color {
    match style {
        CharStyle::Open => Color::Rgb { r: SELECTED_C.0, g: SELECTED_C.1, b: SELECTED_C.2 },
        CharStyle::Bg(r, g, b) => Color::Rgb { r: *r, g: *g, b: *b },
        CharStyle::Tag(_) => Color::Rgb { r: TAG_C.0, g: TAG_C.1, b: TAG_C.2 },
        _ => Color::Reset,
    }
}

const fn char_to_cell(ch: char, style: &CharStyle, trailing: bool) -> Cell {
    let is_cursor = matches!(style, CharStyle::Cursor);
    match ch {
        '\n' => Cell {
            ch: '\u{21b5}',
            fg: if is_cursor {
                Color::Rgb { r: CURSOR_WS_C.0, g: CURSOR_WS_C.1, b: CURSOR_WS_C.2 }
            } else {
                Color::Rgb { r: TRAILING_C.0, g: TRAILING_C.1, b: TRAILING_C.2 }
            },
            bg: style_bg(style),
        },
        ' ' if trailing || is_cursor => Cell {
            ch: '\u{00b7}',
            fg: if is_cursor {
                Color::Rgb { r: CURSOR_WS_C.0, g: CURSOR_WS_C.1, b: CURSOR_WS_C.2 }
            } else {
                Color::Rgb { r: TRAILING_C.0, g: TRAILING_C.1, b: TRAILING_C.2 }
            },
            bg: Color::Reset,
        },
        _ => Cell {
            ch,
            fg: match style {
                CharStyle::Bg(_, _, _) => Color::Black,
                CharStyle::Tag(_) => Color::White,
                CharStyle::Cursor => Color::Rgb { r: CURSOR_C.0, g: CURSOR_C.1, b: CURSOR_C.2 },
                _ => Color::Reset,
            },
            bg: style_bg(style),
        },
    }
}

fn build_frame(
    session: &Session,
    char_styles: &[CharStyle],
    trailing: &[bool],
    scroll_offset: usize,
    width: u16,
    height: u16,
    status: &str,
) -> Vec<Cell> {
    let status_lines = 2_u16;
    let content_height = height.saturating_sub(status_lines);
    let total = width as usize * height as usize;
    let mut cells = vec![Cell::default(); total];
    let mut col = 0_u16;
    let mut row = 0_u16;
    let mut current_line = 0_usize;
    for (i, &ch) in session.chars.iter().enumerate() {
        if ch == '\n' {
            if current_line >= scroll_offset && row < content_height && col < width {
                cells[row as usize * width as usize + col as usize] =
                    char_to_cell(ch, &char_styles[i], false);
            }
            current_line += 1;
            if current_line <= scroll_offset {
                continue;
            }
            col = 0;
            row += 1;
            if row >= content_height {
                break;
            }
            continue;
        }
        if current_line < scroll_offset {
            continue;
        }
        if row >= content_height {
            break;
        }
        cells[row as usize * width as usize + col as usize] =
            char_to_cell(ch, &char_styles[i], trailing[i]);
        col += 1;
        if col >= width {
            col = 0;
            row += 1;
        }
    }
    let keys = CLASSES.iter().map(|c| c.key).collect::<String>();
    let bar =
        format!("[{keys}]  [spc]r.skip [\\]tag  [u]undo  [ret]b.save  [^c]b.discard [^q]f.quit");
    let bar_row = content_height as usize;
    for (col, ch) in bar.chars().take(width as usize).enumerate() {
        cells[bar_row * width as usize + col] = Cell {
            ch,
            fg: Color::Rgb { r: KEYBINDS_C.0, g: KEYBINDS_C.1, b: KEYBINDS_C.2 },
            bg: Color::Reset,
        };
    }
    let status_row = (height - 1) as usize;
    for (col, ch) in status.chars().take(width as usize).enumerate() {
        cells[status_row * width as usize + col] = Cell {
            ch,
            fg: Color::Rgb { r: STATUS_C.0, g: STATUS_C.1, b: STATUS_C.2 },
            bg: Color::Reset,
        };
    }
    cells
}

#[allow(clippy::similar_names)]
fn emit_diff(
    stdout: &mut impl Write,
    new_cells: &[Cell],
    prev_cells: &mut Vec<Cell>,
    width: u16,
) -> io::Result<()> {
    if prev_cells.len() != new_cells.len() {
        queue!(stdout, terminal::Clear(ClearType::All))?;
        *prev_cells =
            vec![Cell { ch: '\x00', fg: Color::Reset, bg: Color::Reset }; new_cells.len()];
    }
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    let mut last_pos: Option<(u16, u16)> = None;
    for (idx, (new, prev)) in new_cells.iter().zip(prev_cells.iter()).enumerate() {
        if new == prev {
            continue;
        }
        let row = (idx / width as usize) as u16;
        let col = (idx % width as usize) as u16;
        let need_move = last_pos.is_none_or(|(lr, lc)| lr != row || lc.saturating_add(1) != col);
        if need_move {
            queue!(stdout, cursor::MoveTo(col, row))?;
        }
        if new.bg != cur_bg {
            queue!(stdout, SetBackgroundColor(new.bg))?;
            cur_bg = new.bg;
        }
        if new.fg != cur_fg {
            queue!(stdout, SetForegroundColor(new.fg))?;
            cur_fg = new.fg;
        }
        queue!(stdout, Print(new.ch))?;
        last_pos = Some((row, col));
    }
    if cur_fg != Color::Reset || cur_bg != Color::Reset {
        queue!(stdout, SetForegroundColor(Color::Reset), SetBackgroundColor(Color::Reset))?;
    }
    prev_cells.clone_from_slice(new_cells);
    Ok(())
}

fn render(
    session: &Session,
    stdout: &mut impl Write,
    scroll_offset: usize,
    status: &str,
    prev_cells: &mut Vec<Cell>,
) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let mut char_styles = vec![CharStyle::Plain; session.chars.len()];
    for span in &session.spans[..session.current] {
        let style = match &span.kind {
            Some(SpanKind::Styled(c)) => CharStyle::Bg(c.bg.0, c.bg.1, c.bg.2),
            Some(SpanKind::Tag(_, tag_name)) => CharStyle::Tag(tag_name.clone()),
            Some(SpanKind::Skip) | None => continue,
        };
        for cs in char_styles.iter_mut().take(span.end).skip(span.start) {
            *cs = style.clone();
        }
    }
    let open_span = &session.spans[session.current];
    if open_span.start == open_span.end {
        if let Some(cs) = char_styles.get_mut(open_span.start) {
            *cs = CharStyle::Cursor;
        }
    } else {
        for cs in char_styles.iter_mut().take(open_span.end).skip(open_span.start) {
            *cs = CharStyle::Open;
        }
    }
    let mut trailing = vec![false; session.chars.len()];
    let mut space_run_start = None;
    for (i, &ch) in session.chars.iter().enumerate() {
        match ch {
            ' ' => space_run_start = space_run_start.or(Some(i)),
            '\n' => {
                if let Some(start) = space_run_start {
                    for t in trailing.iter_mut().take(i).skip(start) {
                        *t = true;
                    }
                }
                space_run_start = None;
            }
            _ => space_run_start = None,
        }
    }
    if let Some(start) = space_run_start {
        for t in trailing.iter_mut().skip(start) {
            *t = true;
        }
    }
    let new_cells =
        build_frame(session, &char_styles, &trailing, scroll_offset, width, height, status);
    emit_diff(stdout, &new_cells, prev_cells, width)?;
    stdout.flush()
}

fn prompt(stdout: &mut impl Write, prefix: &str) -> io::Result<Option<String>> {
    let (_, height) = terminal::size()?;
    let row = height - 1;
    let mut input = String::new();
    loop {
        queue!(stdout, cursor::MoveTo(0, row), Clear(ClearType::CurrentLine))?;
        queue!(
            stdout,
            SetForegroundColor(Color::Rgb { r: PROMPT_C.0, g: PROMPT_C.1, b: PROMPT_C.2 }),
            style::Print(prefix),
            SetForegroundColor(Color::Reset),
            style::Print(&input),
        )?;
        queue!(stdout, cursor::Show)?;
        stdout.flush()?;
        let Event::Key(KeyEvent { code, .. }) = event::read()? else {
            continue;
        };
        match code {
            KeyCode::Enter => {
                queue!(stdout, cursor::Hide)?;
                return Ok(Some(input));
            }
            KeyCode::Esc => {
                queue!(stdout, cursor::Hide)?;
                return Ok(None);
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => input.push(c),
            _ => {}
        }
    }
}
