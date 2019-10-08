use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Span {
    start: Instant,
    end: Option<Instant>,
}

impl Span {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            end: None,
        }
    }

    pub fn evaluate(&self) -> Duration {
        self.end.expect("evaluation failed: unclosed span") - self.start
    }
}

pub struct TimeSpans {
    spans: HashMap<String, Span>,
}

impl TimeSpans {
    pub fn new() -> Self {
        TimeSpans {
            spans: HashMap::new(),
        }
    }

    pub fn begin(&mut self, name: String) {
        self.spans.insert(name, Span::new());
    }

    pub fn end(&mut self, name: String) {
        self.spans
            .get_mut(&name)
            .expect(&format!("you forgot begin {}", name))
            .end = Some(Instant::now());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Span)> + '_ {
        self.spans.iter()
    }
}
