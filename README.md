# sift

A fast, local-first CLI for searching your AI coding-agent prompt history
(Claude Code, Codex, Pi). Read-only, offline, no index.

Work in progress: Claude Code is supported today; Codex and Pi are next.

## Usage

```sh
sift review pr                  # any term (case-insensitive literals)
sift -a review pr               # every term, any order
sift review --not draft         # exclude; repeatable, phrases work: --not "already merged"
sift -c review                  # Claude Code only
sift review --no-color          # color is also off when piped or NO_COLOR is set
```

Exit status is grep-style: 0 matched, 1 no match, 2 error.

History is read from `$CLAUDE_CONFIG_DIR/history.jsonl` (default `~/.claude`).

## Privacy

sift only reads local history files. It never writes to them and never sends anything over the network.

## Install

```sh
make install                     # -> ~/.local/bin/sift
make install PREFIX=/usr/local   # or anywhere
```

On a machine set up from the dotfiles, `.init.sh` clones this repo to `~/dev/sift`
and installs it. Rebuild it after pulling with `devtools sift`.
