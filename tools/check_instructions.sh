#!/usr/bin/env bash
# AGENTS.md is the canonical agent-instruction file. CLAUDE.md must be exactly
# its pointer form: title, the canonical pointer line, and a byte-identical
# copy of AGENTS.md's machine-git-policy block. Any other content in CLAUDE.md
# is drift: move the rule into AGENTS.md instead.
set -euo pipefail

cd "$(dirname "$0")/.."

open='<!-- machine-git-policy -->'
close='<!-- /machine-git-policy -->'

for file in AGENTS.md CLAUDE.md; do
    [[ -f "${file}" ]] || { printf 'error: %s is missing\n' "${file}" >&2; exit 1; }
    count=$(grep -cxF -- "${open}" "${file}" || true)
    if [[ "${count}" != 1 ]]; then
        printf 'error: %s must hold exactly one machine-git-policy block (found %s)\n' \
            "${file}" "${count}" >&2
        exit 1
    fi
done

block=$(awk -v start="${open}" -v end="${close}" '
    $0 == start { inside = 1 }
    inside { print }
    $0 == end { inside = 0 }
' AGENTS.md)

expected=$(printf '# CLAUDE.md\n\nSee [AGENTS.md](AGENTS.md) — it is canonical for this repository.\n\n%s\n' "${block}")

if ! diff -u <(printf '%s\n' "${expected}") CLAUDE.md; then
    printf 'error: CLAUDE.md drifted from its pointer form generated from AGENTS.md\n' >&2
    exit 1
fi

printf 'Agent instructions: CLAUDE.md matches its AGENTS.md pointer form\n'
