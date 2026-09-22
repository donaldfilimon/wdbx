# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**`AGENTS.md` in this directory is canonical — read it first**, for both the browser and the
native edition. On any conflict, it wins.

Three points worth carrying in before you open it:

- **This project has two halves and therefore two gates.** The JS side is
  `bun run check`; the Rust/Tauri side is `cargo test -p specimen-core`
  (`bun run check:all` adds Clippy, not the tests). Running only one half and calling it
  green is the standing mistake here.
- **⚠️ It has two remotes and the default one is dead.** `origin` points at
  `git.chatgpt-team.site` and is unreachable from this machine, *and* it is the tracking
  upstream — so `git branch -vv` never mentions the reachable `github` remote
  (`donaldfilimon/wdbx-specimen-studio`) that actually holds `main`. **Use `git remote -v`,
  not `git branch -vv`, before concluding anything about where this repo pushes.**
- **It has been found running** (`bun dev` + `vite` + `workerd`). Check for live processes
  before touching `node_modules`, `.wrangler`, or `.next`.

Machine-wide layout and toolchain traps live in `~/CLAUDE.md` and are not restated here.
