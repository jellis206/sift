# sift

A fast, local-first CLI for searching your AI coding-agent prompt history
(Claude Code, Codex, Pi). Read-only, offline, no index.

Work in progress.

## Install

```sh
make install                     # -> ~/.local/bin/sift
make install PREFIX=/usr/local   # or anywhere
```

On a machine set up from the dotfiles, `.init.sh` clones this repo to `~/dev/sift`
and installs it. Rebuild it after pulling with `devtools sift`.
