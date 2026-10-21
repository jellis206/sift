use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::DateTime;
use serde::Deserialize;

use super::{Prompt, Source};

/// One line of `history.jsonl`; other fields are ignored.
#[derive(Deserialize)]
struct Entry {
    display: String,
    timestamp: Option<i64>,
}

/// `$CLAUDE_CONFIG_DIR/history.jsonl`, defaulting to `~/.claude/history.jsonl`.
pub fn history_path() -> Option<PathBuf> {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".claude")))?;
    Some(dir.join("history.jsonl"))
}

/// Streams prompts from `path` one record at a time. Malformed records and
/// empty prompts are skipped; an error from `emit` stops the scan.
pub fn scan(path: &Path, emit: &mut dyn FnMut(Prompt) -> Result<()>) -> Result<()> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        let Ok(entry) = serde_json::from_slice::<Entry>(&line) else {
            continue;
        };
        if entry.display.trim().is_empty() {
            continue;
        }
        emit(Prompt {
            source: Source::Claude,
            text: entry.display,
            timestamp: entry.timestamp.and_then(DateTime::from_timestamp_millis),
        })?;
    }
}
