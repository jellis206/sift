use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use serde::Deserialize;

use super::{Emit, Part, Prompt, Source};

/// A session (rollout) line; only user `response_item` messages matter.
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    timestamp: Option<String>,
    payload: Payload,
}

#[derive(Deserialize)]
struct Payload {
    role: Option<String>,
    #[serde(default)]
    content: Vec<Part>,
}

/// A `history.jsonl` line.
#[derive(Deserialize)]
struct HistoryEntry {
    session_id: Option<String>,
    ts: Option<i64>,
    text: String,
}

#[derive(Deserialize)]
struct Imports {
    records: Vec<Import>,
}

#[derive(Deserialize)]
struct Import {
    imported_thread_id: String,
    imported_at: i64,
}

/// Streams user prompts from `$CODEX_HOME` (default `~/.codex`): session
/// files first, then `history.jsonl` entries whose session file is gone.
pub fn scan(emit: Emit) -> Result<()> {
    let Some(home) = super::agent_dir("CODEX_HOME", ".codex") else {
        return Ok(());
    };
    let imports = imported_threads(&home);
    let mut sessions = HashSet::new();

    for path in super::jsonl_files(&[home.join("sessions"), home.join("archived_sessions")]) {
        let id = super::session_id(&path);
        // Threads imported from Claude Code replay their prompts up to the
        // import time; the Claude source already has those.
        let imported_at = id.and_then(|id| imports.get(id)).copied();
        sessions.extend(id.map(str::to_string));

        super::each_line(&path, &mut |line| {
            if !super::maybe_user_message(line) {
                return Ok(());
            }
            let Ok(line) = serde_json::from_slice::<Line>(line) else {
                return Ok(());
            };
            if line.kind != "response_item" || line.payload.role.as_deref() != Some("user") {
                return Ok(());
            }
            let timestamp = line.timestamp.as_deref().and_then(super::parse_time);
            if imported_at
                .is_some_and(|cutoff| timestamp.is_none_or(|t| whole_seconds(t) <= cutoff))
            {
                return Ok(());
            }
            let text = super::join_text(line.payload.content, "input_text");
            if text.trim().is_empty() || super::is_injected(&text) {
                return Ok(());
            }
            emit(Prompt {
                source: Source::Codex,
                text,
                timestamp,
            })
        })?;
    }

    // history.jsonl repeats prompts that are also in their session file.
    super::each_line(&home.join("history.jsonl"), &mut |line| {
        let Ok(entry) = serde_json::from_slice::<HistoryEntry>(line) else {
            return Ok(());
        };
        if entry.session_id.is_some_and(|id| sessions.contains(&id)) || entry.text.trim().is_empty()
        {
            return Ok(());
        }
        emit(Prompt {
            source: Source::Codex,
            text: entry.text,
            timestamp: entry.ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        })
    })
}

/// Thread id → import time, from `external_agent_session_imports.json`.
fn imported_threads(home: &Path) -> HashMap<String, DateTime<Utc>> {
    std::fs::read(home.join("external_agent_session_imports.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Imports>(&bytes).ok())
        .map(|imports| {
            imports
                .records
                .into_iter()
                .filter_map(|r| {
                    Some((
                        r.imported_thread_id,
                        DateTime::from_timestamp(r.imported_at, 0)?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Import times have whole-second precision; message times have millis.
fn whole_seconds(t: DateTime<Utc>) -> DateTime<Utc> {
    t.duration_trunc(TimeDelta::seconds(1)).unwrap_or(t)
}
