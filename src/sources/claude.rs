use anyhow::Result;
use chrono::DateTime;
use serde::Deserialize;

use super::{Emit, Prompt, Source};

/// One line of `history.jsonl`; other fields are ignored.
#[derive(Deserialize)]
struct Entry {
    display: String,
    timestamp: Option<i64>,
}

/// Streams prompts from `$CLAUDE_CONFIG_DIR/history.jsonl` (default
/// `~/.claude`). Malformed records and empty prompts are skipped.
pub fn scan(emit: Emit) -> Result<()> {
    let Some(dir) = super::agent_dir("CLAUDE_CONFIG_DIR", ".claude") else {
        return Ok(());
    };
    super::each_line(&dir.join("history.jsonl"), &mut |line| {
        let Ok(entry) = serde_json::from_slice::<Entry>(line) else {
            return Ok(());
        };
        if entry.display.trim().is_empty() {
            return Ok(());
        }
        emit(Prompt {
            source: Source::Claude,
            text: entry.display,
            timestamp: entry.timestamp.and_then(DateTime::from_timestamp_millis),
        })
    })
}
