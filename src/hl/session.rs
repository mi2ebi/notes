use crate::hl::classes::ClassInfo;

#[derive(Debug, Clone)]
pub enum SpanKind {
    Styled(&'static ClassInfo),
    Tag(/* open: */ String, /* tag_name: */ String),
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
    pub open: String,
    pub close: String,
}

pub struct Session {
    pub content: String,
    pub chars: Vec<char>,
    pub spans: Vec<Span>,
    pub current: usize,
}

impl Session {
    pub fn new(content: &str) -> Self {
        let chars = content.chars().collect::<Vec<_>>();
        let len = chars.len();
        let spans = if len == 0 { vec![] } else { vec![Span::new(0)] };
        Self { content: content.to_string(), chars, spans, current: 0 }
    }
    pub fn cursor(&self) -> usize { self.spans[self.current].end }
    pub fn at_end(&self) -> bool { self.cursor() >= self.chars.len() }
    pub fn advance(&mut self) {
        if !self.at_end() {
            self.spans[self.current].end += 1;
        }
    }
    pub fn commit(&mut self, kind: SpanKind) {
        let span = &self.spans[self.current];
        if span.start == span.end {
            return;
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
        self.spans[..self.current]
            .iter()
            .filter_map(|s| match &s.kind {
                Some(SpanKind::Styled(c)) => Some(FinishedSpan {
                    start: s.start,
                    end: s.end,
                    open: format!(r#"<span class="{}">"#, c.name),
                    close: "</span>".to_owned(),
                }),
                Some(SpanKind::Tag(open, tag_name)) => Some(FinishedSpan {
                    start: s.start,
                    end: s.end,
                    open: open.clone(),
                    close: format!("</{tag_name}>"),
                }),
                Some(SpanKind::Skip) | None => None,
            })
            .collect()
    }
}
