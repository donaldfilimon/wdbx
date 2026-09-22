#!/bin/sh
# Fails if CLAUDE.md drifts from its pointer form: AGENTS.md is the only
# canonical agent-instruction file, so rules added to CLAUDE.md would be
# invisible to every other agent.
set -eu
cd "$(dirname "$0")/.."
expected='# CLAUDE.md

See AGENTS.md — canonical for both the browser and native editions. Machine-wide layout
and toolchain traps live in `~/CLAUDE.md`.'
if [ "$(cat CLAUDE.md)" != "$expected" ]; then
  echo 'check-instructions: CLAUDE.md is not the AGENTS.md pointer; move its rules into AGENTS.md.' >&2
  diff -u /dev/stdin CLAUDE.md <<PTR >&2 || true
$expected
PTR
  exit 1
fi
[ -f AGENTS.md ] || { echo 'check-instructions: AGENTS.md missing' >&2; exit 1; }
echo 'check-instructions: ok'
