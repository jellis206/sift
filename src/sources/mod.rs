pub mod claude;
pub mod codex;
pub mod pi;

use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Claude,
    Codex,
    Pi,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Claude => "CLAUDE",
            Source::Codex => "CODEX",
            Source::Pi => "PI",
        }
    }
}

/// A user-authored prompt, normalized across agents. Serializes as one
/// `--json` output line.
#[derive(Serialize)]
pub struct Prompt {
    pub source: Source,
    pub timestamp: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub text: String,
}

/// Receives each prompt a source finds; an error stops the scan.
pub type Emit<'a> = &'a mut dyn FnMut(Prompt) -> Result<()>;

/// `$var` if set, else `~/default`.
fn agent_dir(var: &str, default: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(default)))
}

/// Calls `on_line` with each raw line of `path`, reusing one buffer. A missing
/// file is silently empty; an unreadable one warns and is skipped. Errors from
/// `on_line` propagate.
fn each_line(path: &Path, on_line: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let skip = |e: std::io::Error| {
        eprintln!("sift: skipping {}: {e}", path.display());
        Ok(())
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return skip(e),
    };
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => return Ok(()),
            Ok(_) => on_line(&line)?,
            Err(e) => return skip(e),
        }
    }
}

/// `.jsonl` files under `dirs` (recursively), ordered by file name. Both
/// Codex and Pi start session file names with a timestamp, so this keeps
/// output roughly chronological.
fn jsonl_files(dirs: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|e| e.path()) {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "jsonl") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    for dir in dirs {
        walk(dir, &mut files);
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    files
}

/// Session files end in the session's UUID: `…-<uuid>.jsonl` / `…_<uuid>.jsonl`.
fn session_id(path: &Path) -> Option<&str> {
    let stem = path.file_stem()?.to_str()?;
    stem.get(stem.len().checked_sub(36)?..)
}

/// Only lines containing this can hold a user message, so everything else
/// (assistant turns, tool output — most of a session) skips JSON parsing.
fn maybe_user_message(line: &[u8]) -> bool {
    memchr::memmem::find(line, br#""role":"user""#).is_some()
}

fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|t| t.to_utc())
}

/// A content part of a session message.
#[derive(Deserialize)]
struct Part {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

/// Joins the text of parts of type `kind`, ignoring images, files, etc.
fn join_text(parts: Vec<Part>, kind: &str) -> String {
    let texts: Vec<String> = parts
        .into_iter()
        .filter(|p| p.kind == kind)
        .filter_map(|p| p.text)
        .collect();
    texts.join("\n")
}

/// Context the agent injects as a user message (environment blocks, skill
/// expansions, AGENTS.md) rather than something the user typed.
// ponytail: prefix heuristic, so a typed prompt starting with `<tag>` is
// skipped too; key off per-agent markers if that ever bites.
fn is_injected(text: &str) -> bool {
    let text = text.trim_start();
    if text.starts_with("# AGENTS.md instructions") {
        return true;
    }
    let Some(rest) = text.strip_prefix('<') else {
        return false;
    };
    let tag_len = rest
        .find(|c: char| !(c.is_ascii_lowercase() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    rest.starts_with(|c: char| c.is_ascii_lowercase())
        && rest[tag_len..].starts_with(['>', ' ', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_context_detection() {
        assert!(is_injected("<environment_context>\n<cwd>/p</cwd>"));
        assert!(is_injected("<skill name=\"x\">body</skill>"));
        assert!(is_injected("# AGENTS.md instructions for /p"));
        assert!(!is_injected("review <b>this</b>"));
        assert!(!is_injected("<- arrow first"));
        assert!(!is_injected("<3 thanks"));
    }

    #[test]
    fn session_id_from_file_name() {
        let uuid = "01a15cbd-af5f-735d-a177-9ad4691a7156";
        let pi = PathBuf::from(format!("2026-10-21T04-34-57-504Z_{uuid}.jsonl"));
        let codex = PathBuf::from(format!("rollout-2026-10-21T04-34-57-{uuid}.jsonl"));
        assert_eq!(session_id(&pi), Some(uuid));
        assert_eq!(session_id(&codex), Some(uuid));
        assert_eq!(session_id(Path::new("short.jsonl")), None);
    }
}
