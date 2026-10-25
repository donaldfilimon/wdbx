"""Identify final current bytes; never promote requirement claims or attest runs."""
import hashlib
import json
from collections import Counter
from pathlib import Path, PurePosixPath

FOLDER = Path(__file__).resolve().parent
ROOT = FOLDER.parents[5]
PACKET = FOLDER.parent
OUTPUT = FOLDER / 'handoff-receipt.json'
assert not OUTPUT.exists(), 'never overwrite a historical receipt'


def safe(path):
    assert isinstance(path, str) and path and '\\' not in path
    relative = PurePosixPath(path)
    assert not relative.is_absolute() and all(x not in ('', '.', '..') for x in path.split('/'))
    current = ROOT
    for part in relative.parts:
        current = current / part
        assert not current.is_symlink(), path
    assert current.is_file() and current.resolve().is_relative_to(ROOT), path
    return current


def sha(path):
    return hashlib.sha256(safe(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(safe(path).read_text())


paths = set()
reference_count = 0


def walk(value):
    global reference_count
    if isinstance(value, dict):
        if 'path' in value and 'sha256' in value:
            assert sha(value['path']) == value['sha256'], value['path']
            paths.add(value['path'])
            reference_count += 1
        for child in value.values():
            walk(child)
    elif isinstance(value, list):
        for child in value:
            walk(child)


for name in ('register-v1.json', 'candidate-paths-v1.json', 'studio-research-v1.json'):
    path = 'docs/completion/execution/' + name
    paths.add(path)
    walk(read(path))
register = read('docs/completion/execution/register-v1.json')['requirements']
assert len(register) == 163
assert all((r['status'], r['evidence_level'], r['test_result']) == ('open', 'C0', 'unknown') for r in register)
counts = Counter(r['reconciliation']['disposition'] for r in register)
assert counts == {'candidate': 120, 'local_gap': 27, 'external_gap': 16}
freeze = json.loads((FOLDER / 'product-freeze.json').read_text())['files']
for record in freeze:
    assert sha(record['path']) == record['sha256'], record['path']
    paths.add(record['path'])
legacy = json.loads((PACKET / 'legacy-baseline.json').read_text())['files']
changed = [r for r in legacy if sha(r['path']) != r['sha256']]
assert len(legacy) == 23 and [r['path'] for r in changed] == ['specimen-studio/native/specimen-kernel/src/identity/mod.rs']
proof = json.loads((FOLDER / 'identity-accessor-extension.json').read_text())
before = (FOLDER / 'identity-mod.before.txt').read_bytes()
assert hashlib.sha256(before).hexdigest() == changed[0]['sha256'] == proof['original_sha256']
current = safe(proof['path']).read_bytes()
addition = proof['exact_added_utf8'].encode()
assert current.count(addition) == 1 and current.replace(addition, b'', 1) == before
summary = json.loads((FOLDER / 'gates/summary.json').read_text())
assert len(summary['gates']) == 4
for gate in summary['gates']:
    assert gate['exit_code'] == 0 and gate['source_unchanged_during_gate']
    full = read(gate['record']['path'])
    gate_sources = {r['path']: r['sha256'] for r in full['source_files']}
    for r in freeze:
        if r['path'].startswith('specimen-studio/'):
            assert gate_sources[r['path']] == r['sha256'], r['path']
review = FOLDER / 'language-final-fix-rereview.md'
assert review.is_file(), 'save the completed scoped review before producing receipt'
for file in PACKET.rglob('*'):
    if file.is_file() and file != OUTPUT:
        paths.add(file.relative_to(ROOT).as_posix())
for path in ('tools/check_completion_execution.py', 'docs/completion/execution/language-v3-profile.md',
             'docs/completion/execution/studio-research.md', 'docs/completion/execution/README.md'):
    paths.add(path)
records = [{'path': p, 'sha256': sha(p), 'bytes': safe(p).stat().st_size} for p in sorted(paths)]
receipt = {
    'kind': 'Unqualified current-byte handoff receipt; no claim promotion',
    'scope': 'Reviewed local language/text/scanner mechanisms and retained synthetic executions; parent full plan unfinished',
    'requirements': {'total': 163, 'status': 'open', 'evidence_level': 'C0', 'test_result': 'unknown', 'dispositions': dict(counts)},
    'legacy': {'original_baseline_files': 23, 'byte_identical': 22, 'explicit_accessor_only_extension': proof},
    'safe_current_reference_occurrences': reference_count,
    'files': records,
    'limitations': 'Hashes identify bytes, not execution authenticity, efficacy, ABI acceptance, production migration or operational/platform qualification.',
}
OUTPUT.write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({'receipt': str(OUTPUT), 'files': len(records), 'current_references': reference_count,
                  'legacy_identical': 22, 'exact_accessor_extension': 1}))
