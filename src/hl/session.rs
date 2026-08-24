use std::collections::HashMap;

use crate::hl::classes::ClassInfo;

#[derive(Debug, Clone)]
pub enum SpanKind {
    Styled(&'static ClassInfo),
    Skip,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: Option<SpanKind>,
}

impl Span {
    const fn new(start: usize) -> Self { Self { start, end: start, kind: None } }
}

#[derive(Clone)]
pub struct FinishedSpan {
    pub start: usize,
    pub end: usize,
    pub class_name: &'static str,
}

pub struct Session<'a> {
    pub chars: Vec<char>,
    pub spans: Vec<Span>,
    pub current: usize,
    suggestions: &'a mut HashMap<Vec<char>, &'static ClassInfo>,
}

impl<'a> Session<'a> {
    pub fn new(content: &str, suggestions: &'a mut HashMap<Vec<char>, &'static ClassInfo>) -> Self {
        let chars = content.chars().collect::<Vec<_>>();
        let spans = if chars.is_empty() { vec![] } else { vec![Span::new(0)] };
        Self { chars, spans, current: 0, suggestions }
    }

    pub fn cursor(&self) -> usize { self.spans[self.current].end }

    pub fn at_end(&self) -> bool { self.cursor() >= self.chars.len() }

    pub fn advance(&mut self, n: isize) {
        if !self.at_end() {
            let span = &mut self.spans[self.current];
            let new_end = span.end.saturating_add_signed(n).clamp(span.start, self.chars.len());
            span.end = new_end;
        }
    }

    pub fn commit(&mut self, kind: SpanKind) {
        let span = &self.spans[self.current];
        if span.start == span.end {
            return;
        }
        if let SpanKind::Styled(info) = &kind {
            let text: Vec<char> = self.chars[span.start .. span.end].to_vec();
            self.suggestions.insert(text, info);
        }
        self.spans[self.current].kind = Some(kind);
        let next_start = self.cursor();
        self.spans.push(Span::new(next_start));
        self.current += 1;
    }

    pub fn undo(&mut self) {
        if self.current == 0 {
            return;
        }
        let restart = self.spans[self.current - 1].start;
        self.spans.truncate(self.current);
        self.current -= 1;
        self.spans[self.current] = Span::new(restart);
    }

    pub fn finish(&self) -> Vec<FinishedSpan> {
        self.spans[.. self.current]
            .iter()
            .filter_map(|s| match &s.kind {
                Some(SpanKind::Styled(c)) => {
                    Some(FinishedSpan { start: s.start, end: s.end, class_name: c.name })
                }
                Some(SpanKind::Skip) | None => None,
            })
            .collect()
    }

    pub fn suggestion(&self) -> Option<(&'static ClassInfo, usize)> {
        let start = self.spans[self.current].start;
        self.suggestions
            .iter()
            .filter(|(key, _)| self.chars[start ..].starts_with(key))
            .map(|(key, info)| (*info, key.len()))
            .max_by_key(|&(_, len)| len)
    }

    pub fn accept_suggestion(&mut self) -> bool {
        let Some((info, len)) = self.suggestion() else { return false };
        let start = self.spans[self.current].start;
        self.spans[self.current].end = start + len;
        self.commit(SpanKind::Styled(info));
        true
    }
}
