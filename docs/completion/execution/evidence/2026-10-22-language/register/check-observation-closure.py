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
assert [row['case'] for row in data['local_mechanism_observations']] == ['S01', 'S02', 'S03', 'S04', 'S17', 'S40', 'S01', 'S04', 'S05', 'S06', 'S19', 'S20', 'S21', 'S25', 'S46']
assert len({row['id'] for row in data['local_mechanism_observations']}) == 15
reg = read('docs/completion/execution/register-v1.json')
assert len(reg['requirements']) == 163
assert Counter(row['reconciliation']['disposition'] for row in reg['requirements']) == {'candidate': 120, 'local_gap': 27, 'external_gap': 16}
assert all((row['status'], row['evidence_level'], row['test_result']) == ('open', 'C0', 'unknown') for row in reg['requirements'])
print('PASS: ordered 46 S + 18 D IDs and verbatim frozen rows; 64 explicit blocker/C0/no-promotion records.')
print('PASS: all', len(references), 'artifact references hash-match safe existing repository files; design bundles match.')
print('PASS: 15 bounded local observations; 120 candidates / 27 local gaps / 16 external gaps; 163 open/C0/unknown.')
print('Artifact SHA256:', sha(ARTIFACT))
print('Closure is an inventory check only; no product, efficacy, hardware or whole-case qualification.')

# Additional current packet closure; every retained baseline/evidence byte is checked.
E = 'docs/completion/execution/evidence/2026-10-22-language/'
for inventory in ['prior-evidence-identities.json', 'frozen-before.json']:
    for record in read(E+'register/'+inventory)['files']:
        assert sha(record['path']) == record['sha256'], record['path']
legacy = read(E+'legacy-baseline.json')['files']
assert len(legacy) == 23
for record in legacy:
    assert sha(record['path']) == record['sha256'], record['path']
before = read(E+'register/before-register-v1.json')['requirements']
assert [(r['id'],r['owner'],r['task'],r['plan'],r['status'],r['evidence_level'],r['test_result']) for r in reg['requirements']] == [(r['id'],r['owner'],r['task'],r['plan'],r['status'],r['evidence_level'],r['test_result']) for r in before]
changed = [r['id'] for r,b in zip(reg['requirements'],before) if r['reconciliation']['disposition'] != b['reconciliation']['disposition']]
assert changed == ['S05','S06'], changed
manifest = read(E+'register/current-source-manifest.json')['files']
mapping = read(E+'register/requirement-mapping.json')
mapped = {p for g in mapping['groups'] for p in g['sources']+g['tests']}
assert mapped == {r['path'] for r in manifest} - {'specimen-studio/native/specimen-kernel/src/lib.rs'}
for record in manifest:
    assert record['sha256'] == sha(record['path'])
    assert record['mapped_ids'] and record['mapped_decisions']
known_ids = {r['id'] for r in reg['requirements']}
known_decisions = {d['id'] for d in data['decisions']}
for g in mapping['groups']:
    assert len(g['ids']) == len(set(g['ids'])) and set(g['ids']) <= known_ids
    assert len(g['decisions']) == len(set(g['decisions'])) and set(g['decisions']) <= known_decisions
    assert g['boundary']
for m in data['local_mechanism_observations']:
    assert 'fn '+m['direct_test']['name']+'(' in safe_file(m['direct_test']['path']).read_text()
summary = read(E+'gates/summary.json')
assert len(summary['gates']) == 4 and all(g['exit_code'] == 0 for g in summary['gates'])
for gate in summary['gates']:
    assert sha(E+'gates/'+gate['gate']+'.log') == gate['log_sha256']
assert len(safe_file(E+'parity/native.json').read_bytes()) == 133421
assert sha(E+'parity/native.json') == sha(E+'parity/wasm.json') == '0874bb27c8a593db459c47f06faa46e62cf2c917f31e7eaa78845a5df7475bae'
print('PASS: 23 legacy baselines, prior evidence and product/frozen input bytes preserved; only S05/S06 disposition changes.')
print('PASS: new source/test routing closure, named assertions, retained four gate logs and actual 133421-byte feature-WASM/native trace identity.')
print('No product gates rerun. Final whole-tranche review remains pending.')
