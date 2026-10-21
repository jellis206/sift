use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

use super::{Emit, Part, Prompt, Source};

/// A session line; only `message` lines with a user message matter.
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    timestamp: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    role: String,
    content: Vec<Part>,
}

/// Streams user prompts from Pi's sessions dir, resolved like Pi does:
/// `$PI_CODING_AGENT_SESSION_DIR`, else `$PI_CODING_AGENT_DIR/sessions`
/// (default `~/.pi/agent/sessions`).
// ponytail: ignores a `sessionDir` override in Pi's settings.json; read it
// here if anyone sets one.
pub fn scan(emit: Emit) -> Result<()> {
    let dir = std::env::var_os("PI_CODING_AGENT_SESSION_DIR")
        .map(PathBuf::from)
        .or_else(|| Some(super::agent_dir("PI_CODING_AGENT_DIR", ".pi/agent")?.join("sessions")));
    let Some(dir) = dir else {
        return Ok(());
    };

    for path in super::jsonl_files(&[dir]) {
        let id = super::session_id(&path);
        super::each_line(&path, &mut |line| {
            if !super::maybe_user_message(line) {
                return Ok(());
            }
            let Ok(Line {
                kind,
                timestamp,
                message: Some(message),
            }) = serde_json::from_slice(line)
            else {
                return Ok(());
            };
            if kind != "message" || message.role != "user" {
                return Ok(());
            }
            let text = super::join_text(message.content, "text");
            if text.trim().is_empty() || super::is_injected(&text) {
                return Ok(());
            }
            emit(Prompt {
                source: Source::Pi,
                timestamp: timestamp.as_deref().and_then(super::parse_time),
                session_id: id.map(str::to_string),
                text,
            })
        })?;
    }
    Ok(())
}
