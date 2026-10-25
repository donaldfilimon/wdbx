# WDBX: legacy migration source

AGENTS.md is canonical. Read the relevant sections of
[docs/agent-guide.md](docs/agent-guide.md) before changing a subsystem.

- WDBX owns durable episodic trust, supersession, contradiction, quarantine and deletion semantics. Public source does not make live stores public. Never use the user's live store in tests.
- Retain the five-crate dependency layering. This workspace has no runtime dependency back into ABI/Abbey; consumers use it as the adjacent ../wdbx checkout. Do not mix Git and path identities of shared crates.
- Use pinned stable Rust 1.99.1, edition 2024. Put the rustup toolchain and /usr/bin before Homebrew/Swiftly compiler shims. The gate is bash tools/check.sh; it checks instructions, sizes, formatting, Clippy and workspace tests. Keep stdin at EOF for tests.
- Read the detailed guide's Gates, Persistence Boundaries and Contract Corpus before storage changes. Preserve format versions, independent cross-language oracles, exact golden-regeneration opt-ins, locks, recovery and evidence limits.
- Default gates do not qualify optional FHE features or production/distributed/accelerator capabilities. Use the detailed Claims and Evidence / Completion Contracts sections before making claims.
- Company consolidation is staged. Source availability, relocation and successful builds do not qualify LYLX replacement. Preserve dirty work, history, aliases and remotes until retirement conditions pass.

<!-- machine-git-policy -->
## Git workflow (machine policy, 2026-08-27)

Work on the default branch in this canonical checkout. Do not create
branches or worktrees by default; they are for tasks that genuinely need
isolation, or when Donald asks. Any worktree or topic branch created here
must be merged back into this checkout's default branch, the worktree
removed, and the branch deleted, before pushing and before the task is
called done. Full policy: `~/.claude/CLAUDE.md` (*Git discipline*).
<!-- /machine-git-policy -->
