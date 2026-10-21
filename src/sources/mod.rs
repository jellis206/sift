pub mod claude;

use chrono::{DateTime, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Claude,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Claude => "CLAUDE",
        }
    }
}

/// A user-authored prompt, normalized across agents.
pub struct Prompt {
    pub source: Source,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
}
