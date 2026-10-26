#!/usr/bin/env python3
"""Fail closed on unreviewed Studio preservation identity changes (Python 3.9)."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

TRANSITION_PATH = 'docs/completion/execution/evidence/2026-10-26-studio-preservation-transition.json'
TRANSITION_SHA256 = '12866d64310e4c251dcb2b3772f928a320a6a65a8cbd02784db28d616a879053'
BASELINE_PINS = {
    'language': '15d8d8fe9682da055b757536abaf0d856224b13596e2d0865d5069e987f8a4c0',
    'scoring': '16e8371d58abe5e44ba656a8f35efb89c142f70023c3116337264eae51b9a11a',
}
PREFIX = 'specimen-studio/'
PROFILES = {
    'language': {
        'baseline': 'docs/completion/execution/evidence/2026-10-22-language/legacy-baseline.json',
        'count': 23,
        'changed': {PREFIX + 'Cargo.lock', PREFIX + 'native/specimen-core/src/persistence.rs'},
    },
    'scoring': {
        'baseline': 'docs/completion/execution/evidence/2026-10-23-scoring/preservation-baseline.json',
        'count': 80,
        'changed': {PREFIX + 'AGENTS.md', PREFIX + 'native/specimen-core/Cargo.toml',
                    PREFIX + 'native/specimen-core/src/persistence.rs',
                    PREFIX + 'scripts/check-language-parity.sh'},
    },
}
ACCESSOR_PATH = PREFIX + 'native/specimen-kernel/src/identity/mod.rs'
ACCESSOR = b"    /// Borrow entries in insertion order in O(1), without copying any records.\n    /// Callers can reserve traversal/materialization work before inspecting them.\n    pub fn entries(&self) -> &[EntryRecord] {\n        &self.records.entries\n    }\n"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key: ' + key)
        result[key] = value
    return result


def pinned_json(root, path, pin):
    raw = (root / path).read_bytes()
    require(digest(raw) == pin, 'identity mismatch: ' + path)
    return json.loads(raw, object_pairs_hook=unique_object)


def valid_hash(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None


def check(root, profile, mode='current'):
    require(profile in PROFILES and mode in ('current', 'historical'), 'invalid preservation mode/profile')
    info = PROFILES[profile]
    baseline = pinned_json(root, info['baseline'], BASELINE_PINS[profile])
    rows = baseline['files']
    require(isinstance(rows, list) and len(rows) == info['count'], 'baseline membership count')
    expected = {}
    for row in rows:
        path = row['path']
        require(path not in expected and valid_hash(row['sha256']), 'invalid baseline row')
        expected[path] = row['sha256']
    if mode == 'current':
        manifest = pinned_json(root, TRANSITION_PATH, TRANSITION_SHA256)
        require(set(manifest) == {'version', 'profiles'} and manifest['version'] == 1,
                'invalid transition schema')
        require(set(manifest['profiles']) == set(PROFILES), 'transition profiles')
        # Validate every profile, including one not selected by this invocation.
        for name, transition in manifest['profiles'].items():
            require(set(transition) == {'baseline_sha256', 'changes'}, 'transition profile fields')
            require(transition['baseline_sha256'] == BASELINE_PINS[name], 'transition baseline digest')
            original = pinned_json(root, PROFILES[name]['baseline'], BASELINE_PINS[name])
            prior = {row['path']: row['sha256'] for row in original['files']}
            changes = transition['changes']
            require(isinstance(changes, list), 'transition changes must be a list')
            seen = set()
            for row in changes:
                require(set(row) == {'path', 'prior_sha256', 'current_sha256', 'provenance', 'rationale'},
                        'transition row fields')
                path = row['path']
                require(isinstance(path, str) and path in PROFILES[name]['changed'] and path not in seen,
                        'unexpected or duplicate transition path')
                seen.add(path)
                require(row['prior_sha256'] == prior.get(path), 'transition prior digest: ' + path)
                require(valid_hash(row['current_sha256']) and row['current_sha256'] != row['prior_sha256'],
                        'transition current digest: ' + path)
                require(isinstance(row['provenance'], list) and bool(row['provenance']) and
                        all(isinstance(commit, str) and re.fullmatch('[0-9a-f]{40}', commit)
                            for commit in row['provenance']), 'transition provenance')
                require(isinstance(row['rationale'], str) and bool(row['rationale'].strip()),
                        'transition rationale')
                if name == profile:
                    expected[path] = row['current_sha256']
            require(seen == PROFILES[name]['changed'], 'missing transition path')
    errors = []
    for path, pin in expected.items():
        try:
            data = (root / path).read_bytes()
            if profile == 'language' and path == ACCESSOR_PATH:
                require(data.count(ACCESSOR) == 1, 'exact immutable accessor required: ' + path)
                data = data.replace(ACCESSOR, b'', 1)
            require(digest(data) == pin, 'source identity mismatch: ' + path)
        except (OSError, ValueError) as error:
            errors.append(str(error))
    require(not errors, '\n'.join(errors))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', required=True, choices=PROFILES)
    parser.add_argument('--mode', default='current', choices=('current', 'historical'))
    args = parser.parse_args()
    try:
        check(Path(__file__).resolve().parents[2], args.profile, args.mode)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print('FAIL: ' + str(error), file=sys.stderr)
        return 1
    print('PASS: {} {} preservation identity; original baseline retained'.format(args.profile, args.mode))
    return 0


if __name__ == '__main__':
    sys.exit(main())
