mod output;
mod rank;
mod search;
mod sources;

use std::io::{self, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};
use clap::Parser;

use output::Format;
use rank::{Order, TopK};
use search::Query;
use sources::{Prompt, claude, codex, pi};

/// Search what you've asked your coding agents.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Terms to search for: case-insensitive literals, quote multi-word terms
    #[arg(required = true)]
    terms: Vec<String>,

    /// Require all terms (default: any term)
    #[arg(short = 'a', long = "and")]
    all: bool,

    /// Search Claude Code (combine source flags; default: all sources)
    #[arg(short, long)]
    claude: bool,

    /// Search Codex
    #[arg(short = 'x', long)]
    codex: bool,

    /// Search Pi
    #[arg(short, long)]
    pi: bool,

    /// Exclude prompts containing TERM (repeatable)
    #[arg(long = "not", value_name = "TERM")]
    exclude: Vec<String>,

    /// Show at most N results (sorts newest first unless --sort is given)
    #[arg(short = 'n', long, value_name = "N")]
    limit: Option<usize>,

    /// Only prompts from the last DURATION, e.g. 30m, 12h, 7d, 2w
    #[arg(long, value_name = "DURATION", value_parser = parse_duration)]
    since: Option<TimeDelta>,

    /// Sort all results by time (default: stream in scan order, grouped by source)
    #[arg(long, value_enum, value_name = "ORDER")]
    sort: Option<Order>,

    /// Print one JSON object per line
    #[arg(long)]
    json: bool,

    /// Disable ANSI colors (also off when piped or NO_COLOR is set)
    #[arg(long)]
    no_color: bool,
}

/// grep-style exit codes: 0 matched, 1 no match, 2 error.
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        // `sift … | head` closing the pipe is a normal way to stop.
        Err(e) if is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sift: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<bool> {
    let query = Query::new(&cli.terms, &cli.exclude, cli.all);
    let format = if cli.json {
        Format::Json
    } else {
        let color =
            !cli.no_color && std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal();
        Format::Text { color }
    };
    // Untimed prompts can't be shown to be recent, so --since drops them.
    let cutoff = cli.since.map(|age| {
        Utc::now()
            .checked_sub_signed(age)
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
    });
    // Sorting has to see every match first; otherwise results stream as found.
    let mut ranked = cli
        .sort
        .or(cli.limit.map(|_| Order::Newest))
        .map(|order| TopK::new(order, cli.limit));
    let mut out = BufWriter::new(io::stdout().lock());
    let mut found = false;

    let mut emit = |prompt: Prompt| -> Result<()> {
        let too_old = cutoff.is_some_and(|c| prompt.timestamp.is_none_or(|t| t < c));
        if too_old || !query.matches(&prompt.text) {
            return Ok(());
        }
        found = true;
        match &mut ranked {
            Some(top) => top.push(prompt),
            None => output::print(&mut out, &prompt, &format)?,
        }
        Ok(())
    };
    // Missing histories are skipped: not every machine has every agent.
    let every = !(cli.claude || cli.codex || cli.pi);
    if every || cli.claude {
        claude::scan(&mut emit)?;
    }
    if every || cli.codex {
        codex::scan(&mut emit)?;
    }
    if every || cli.pi {
        pi::scan(&mut emit)?;
    }

    for prompt in ranked.map(TopK::into_sorted).unwrap_or_default() {
        output::print(&mut out, &prompt, &format)?;
    }
    out.flush()?;
    Ok(found)
}

/// `<n><unit>` with unit s, m, h, d or w.
fn parse_duration(s: &str) -> Result<TimeDelta, String> {
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (n, unit) = s.split_at(split);
    let n: i64 = n
        .parse()
        .map_err(|_| format!("expected e.g. 7d, got {s:?}"))?;
    let delta = match unit {
        "s" => TimeDelta::try_seconds(n),
        "m" => TimeDelta::try_minutes(n),
        "h" => TimeDelta::try_hours(n),
        "d" => TimeDelta::try_days(n),
        "w" => TimeDelta::try_weeks(n),
        _ => return Err(format!("unit must be s, m, h, d or w, got {s:?}")),
    };
    delta.ok_or_else(|| format!("{s:?} is too large"))
}

fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<io::Error>())
        .any(|io| io.kind() == io::ErrorKind::BrokenPipe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("30m"), Ok(TimeDelta::minutes(30)));
        assert_eq!(parse_duration("7d"), Ok(TimeDelta::days(7)));
        assert_eq!(parse_duration("2w"), Ok(TimeDelta::weeks(2)));
        for bad in ["", "7", "d", "7y", "-1d", "1.5h", "99999999999999999w"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
    }
}
