# sift

A fast, local-first CLI for searching your AI coding-agent prompt history
(Claude Code, Codex, Pi). Read-only, offline, no index.

## Usage

```sh
sift review pr                  # any term (case-insensitive literals)
sift -a review pr               # every term, any order
sift review --not draft         # exclude; repeatable, phrases work: --not "already merged"
sift -c review                  # Claude Code only; -x Codex, -p Pi, combinable (default: all)
sift review --no-color          # color is also off when piped or NO_COLOR is set

sift review -n 20               # 20 newest matches, newest first
sift review --sort oldest       # sort everything by time (newest | oldest)
sift review --since 7d          # only the last 7 days (s, m, h, d, w)
sift review --json              # one JSON object per line: source, timestamp, session_id, text
```

Without `--sort` or `-n`, results stream as they're found, grouped by source,
which gives the fastest first result. With them, sources are merged by time.
Prompts with no timestamp count as oldest, and `--since` drops them. `-n`
keeps only N results in memory no matter how many prompts match.

Exit status is grep-style: 0 matched, 1 no match, 2 error.

## Sources

| Agent  | Reads                                                         | Override                                               |
| ------ | ------------------------------------------------------------- | ------------------------------------------------------ |
| Claude | `~/.claude/history.jsonl`                                     | `CLAUDE_CONFIG_DIR`                                    |
| Codex  | `~/.codex/{sessions,archived_sessions}/**/*.jsonl`, `history.jsonl` | `CODEX_HOME`                                     |
| Pi     | `~/.pi/agent/sessions/**/*.jsonl`                             | `PI_CODING_AGENT_SESSION_DIR`, `PI_CODING_AGENT_DIR`   |

Only prompts you typed are shown. Agent-injected context such as
`<environment_context>`, `<skill>` expansions, and AGENTS.md is skipped. So are
Codex threads imported from Claude Code, since those prompts already come from
Claude, and Codex `history.jsonl` entries that duplicate a session file.

## Privacy

sift only reads local history files. It never writes to them and never sends
anything over the network. Unreadable files are reported by path only, never
with their contents.

## Install

Requires a recent stable Rust toolchain.

```sh
cargo install --git https://github.com/jellis206/sift   # -> ~/.cargo/bin/sift

# or from a checkout
make install                     # -> ~/.local/bin/sift
make install PREFIX=/usr/local   # or anywhere
cargo install --path .           # -> ~/.cargo/bin/sift
```

Prebuilt binaries for macOS (Apple Silicon) and Linux (x86_64, arm64) are
attached to each [release](https://github.com/jellis206/sift/releases).

On a machine set up from the dotfiles, `.init.sh` clones this repo to `~/dev/sift`
and installs it. Rebuild it after pulling with `devtools sift`.

## Development

```sh
cargo test                 # unit + CLI tests; fixtures pin every history location
cargo clippy --all-targets
```

CI runs fmt, clippy and tests on macOS and Linux. Pushing a `v*` tag publishes a
release with binaries.
