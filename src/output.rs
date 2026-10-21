use std::io::{self, Write};

use chrono::Local;

use crate::search::Query;
use crate::sources::Prompt;

const MARKER: &str = ">>>> ";
const INDENT: &str = "     ";
/// Longer prompts (usually pasted logs) show only matching lines.
const MAX_LINES: usize = 8;
const MAX_LINE_CHARS: usize = 300;

pub enum Format {
    Text { color: bool },
    Json,
}

pub fn print(
    out: &mut impl Write,
    prompt: &Prompt,
    query: &Query,
    format: &Format,
) -> io::Result<()> {
    match *format {
        Format::Text { color } => print_text(out, prompt, query, color),
        Format::Json => {
            serde_json::to_writer(&mut *out, prompt)?;
            writeln!(out)
        }
    }
}

/// Writes a header line, the prompt with continuation lines indented under
/// the `>>>>` marker, then a blank separator line. Prompts over `MAX_LINES`
/// show their first line and matching lines, with `…` marking skipped ones.
fn print_text(out: &mut impl Write, prompt: &Prompt, query: &Query, color: bool) -> io::Result<()> {
    let (head, dim, reset) = if color {
        ("\x1b[1;36m", "\x1b[2m", "\x1b[0m")
    } else {
        ("", "", "")
    };
    let when = prompt.timestamp.map_or_else(
        || "unknown time".to_string(),
        |t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(),
    );
    writeln!(out, "{head}{} · {when}{reset}", prompt.source.label())?;

    let lines: Vec<&str> = prompt.text.lines().collect();
    let shown: Vec<usize> = if lines.len() <= MAX_LINES {
        (0..lines.len()).collect()
    } else {
        std::iter::once(0)
            .chain((1..lines.len()).filter(|&i| query.hits(lines[i])))
            .take(MAX_LINES)
            .collect()
    };
    let mut next = 0;
    for &i in &shown {
        if i > next {
            writeln!(out, "{dim}{INDENT}…{reset}")?;
        }
        let lead = if i == 0 { MARKER } else { INDENT };
        writeln!(out, "{dim}{lead}{reset}{}", clip(lines[i]))?;
        next = i + 1;
    }
    if shown.len() < lines.len() {
        let hidden = lines.len() - shown.len();
        writeln!(
            out,
            "{dim}{INDENT}… {hidden} more lines (--json shows all){reset}"
        )?;
    }
    writeln!(out)
}

fn clip(line: &str) -> String {
    match line.char_indices().nth(MAX_LINE_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Source;

    #[test]
    fn long_prompts_show_first_and_matching_lines() {
        let mut text = vec!["cd somewhere".to_string()];
        text.extend((1..=20).map(|i| format!("noise {i}")));
        text[5] = "error: review-pr/SKILL.md".into();
        text.push("x".repeat(400));
        let prompt = Prompt {
            source: Source::Pi,
            timestamp: None,
            session_id: None,
            text: text.join("\n"),
        };
        let query = Query::new(&["review".into()], &[], false);
        let mut out = Vec::new();
        print_text(&mut out, &prompt, &query, false).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "PI · unknown time\n\
             >>>> cd somewhere\n\
             \x20    …\n\
             \x20    error: review-pr/SKILL.md\n\
             \x20    … 20 more lines (--json shows all)\n\n"
        );
        assert_eq!(clip(&"é".repeat(301)).chars().count(), 301);
    }
}
