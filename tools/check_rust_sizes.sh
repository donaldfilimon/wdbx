#!/usr/bin/env bash
# Enforce the repository's Rust source-size limit (1000 lines per file) without
# walking generated build output. Git supplies a NUL-delimited list so unusual
# filenames remain one entry; untracked, non-ignored Rust sources are included
# so a new module is checked before its first commit. Ported from ABI's
# tools/check_rust_sizes.sh; WDBX has no per-file exceptions.
set -euo pipefail

cd "$(dirname "$0")/.."

limit=1000
failed=0
while IFS= read -r -d '' path; do
    [[ -f "${path}" ]] || continue

    lines=0
    while IFS= read -r line || [[ -n "${line}" ]]; do
        lines=$((lines + 1))
    done < "${path}"

    if ((lines > limit)); then
        printf 'error: Rust source exceeds %d-line limit: ' "${limit}" >&2
        printf '%q (%d lines)\n' "${path}" "${lines}" >&2
        failed=1
    fi
done < <(git ls-files --cached --others --exclude-standard -z -- '*.rs')

if ((failed != 0)); then
    exit 1
fi

printf 'Rust source sizes: all within limits\n'
