mod output;
mod search;
mod sources;

use std::io::{self, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

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
    let color =
        !cli.no_color && std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal();
    let mut out = BufWriter::new(io::stdout().lock());
    let mut found = false;

    let mut emit = |prompt: Prompt| -> Result<()> {
        if query.matches(&prompt.text) {
            found = true;
            output::print(&mut out, &prompt, color)?;
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

    out.flush()?;
    Ok(found)
}

fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<io::Error>())
        .any(|io| io.kind() == io::ErrorKind::BrokenPipe)
}
