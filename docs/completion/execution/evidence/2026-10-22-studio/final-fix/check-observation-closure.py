"""One-shot standard-library observation closure check; no promotion verifier."""
import hashlib
import json
from pathlib import Path, PurePosixPath
from collections import Counter

ROOT = Path(__file__).resolve().parents[6]
ARTIFACT = 'docs/completion/execution/studio-research-v1.json'


def read(path):
    return json.loads((ROOT / path).read_text())


def safe_file(path):
    assert isinstance(path, str) and path and '\\' not in path
    relative = PurePosixPath(path)
    assert not relative.is_absolute() and all(p not in ('', '.', '..') for p in path.split('/'))
    current = ROOT
    for part in relative.parts:
        current = current / part
        assert not current.is_symlink(), path
    assert current.is_file() and current.resolve().is_relative_to(ROOT.resolve()), path
    return current


def sha(path):
    return hashlib.sha256(safe_file(path).read_bytes()).hexdigest()


data = read(ARTIFACT)
frozen = read('docs/completion/studio-acceptance-v1.json')
designs = read('docs/completion/designs.json')['designs']
assert data['version'] == 1 and data['claim_policy'] == 'C0-only; no promotion'
for section, prefix, count in [('cases', 'S', 46), ('decisions', 'D', 18)]:
    expected = [prefix + str(i).zfill(2) for i in range(1, count + 1)]
    assert [row['id'] for row in data[section]] == expected
    assert [row['id'] for row in frozen[section]] == expected
    assert [row['frozen_row'] for row in data[section]] == [row['row'] for row in frozen[section]]
    for row in data[section]:
        assert (row['status'], row['evidence_level'], row['test_result']) == ('open', 'C0', 'unknown')
        assert row['claim_policy'] == 'C0-only; no promotion'
        assert row['blockers'] and all(isinstance(x, str) and x.strip() for x in row['blockers'])
        assert row['source_references']
        if section == 'decisions':
            assert any(row['design_bundle'] == bundle['id'] and row['id'] in bundle['decisions'] for bundle in designs)
references = []


def walk(value):
    if isinstance(value, dict):
        if 'path' in value and 'sha256' in value:
            assert value['sha256'] == sha(value['path']), value['path']
            references.append(value['path'])
        for child in value.values():
            walk(child)
    elif isinstance(value, list):
        for child in value:
            walk(child)


walk(data)
assert 'docs/completion/execution/studio-research.md' not in references
assert not any(Path(p).name == 'receipt.json' for p in references)
for item in data['source_snapshot_observations']:
    observed = sha(item['path'])
    assert item['observed_sha256'] == observed
    assert item['state'] == ('matched' if observed == item['historical_expected_sha256'] else 'mismatched')
assert [row['case'] for row in data['local_mechanism_observations']] == ['S01', 'S02', 'S03', 'S04', 'S17', 'S40']
reg = read('docs/completion/execution/register-v1.json')
assert len(reg['requirements']) == 163
assert Counter(row['reconciliation']['disposition'] for row in reg['requirements']) == {'candidate': 118, 'local_gap': 29, 'external_gap': 16}
assert all((row['status'], row['evidence_level'], row['test_result']) == ('open', 'C0', 'unknown') for row in reg['requirements'])
print('PASS: ordered 46 S + 18 D IDs and verbatim frozen rows; 64 explicit blocker/C0/no-promotion records.')
print('PASS: all', len(references), 'artifact references hash-match safe existing repository files; design bundles match.')
print('PASS: 6 bounded local observations; 118 candidates / 29 local gaps / 16 external gaps; 163 open/C0/unknown.')
print('Artifact SHA256:', sha(ARTIFACT))
print('Closure is an inventory check only; no product, efficacy, hardware or whole-case qualification.')
