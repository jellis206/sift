use std::io::{self, Write};

use chrono::Local;

use crate::sources::Prompt;

const MARKER: &str = ">>>> ";
const INDENT: &str = "     ";

pub enum Format {
    Text { color: bool },
    Json,
}

pub fn print(out: &mut impl Write, prompt: &Prompt, format: &Format) -> io::Result<()> {
    match *format {
        Format::Text { color } => print_text(out, prompt, color),
        Format::Json => {
            serde_json::to_writer(&mut *out, prompt)?;
            writeln!(out)
        }
    }
}

/// Writes a header line, the prompt with continuation lines indented under
/// the `>>>>` marker, then a blank separator line.
fn print_text(out: &mut impl Write, prompt: &Prompt, color: bool) -> io::Result<()> {
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
    for (i, line) in prompt.text.lines().enumerate() {
        let lead = if i == 0 { MARKER } else { INDENT };
        writeln!(out, "{dim}{lead}{reset}{line}")?;
    }
    writeln!(out)
}
