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

# Historical evidence bytes remain immutable; only the reviewed product changes
# named below differ from the old source snapshot. This is an explicit scope,
# not permission to repin or silently expand the legacy exception.
E = 'docs/completion/execution/evidence/2026-10-22-language/'
F = E+'final-fix/'
K = 'specimen-studio/native/specimen-kernel/'
W = 'specimen-studio/native/specimen-wasm/'
changed_sources = {
 K+'src/identity/mod.rs', K+'src/language_v3/model.rs',
 K+'src/language_v3/prepare.rs', K+'src/language_v3/script.rs',
 K+'src/language_v3/tables.rs', K+'src/scanner_v3/routes/matching.rs',
 K+'src/scanner_v3/routes/mod.rs', K+'src/text_v3/mod.rs',
 W+'src/language_probe.rs', W+'tests/language_probe.rs',
 'specimen-studio/scripts/check-language-parity.sh',
 'specimen-studio/tests/language-parity.mjs',
}
new_sources = {K+'src/scanner_v3/routes/work.rs',K+'tests/language_final_fix.rs',K+'tests/routes_final_fix.rs',W+'src/language_probe/final_fix.rs'}
for inventory in [F+'historical-evidence-baseline.json',E+'register/prior-evidence-identities.json']:
 for record in read(inventory)['files']:
  assert sha(record['path']) == record['sha256'], record['path']
source_freeze = {r['path']:r for r in read(F+'product-freeze.json')['files']}
for record in source_freeze.values(): assert sha(record['path']) == record['sha256'], record['path']
changed = set()
for record in read(E+'register/frozen-before.json')['files']:
 path = record['path']
 if sha(path) != record['sha256']:
  changed.add(path)
  assert path in changed_sources and sha(F+'before/'+path) == record['sha256'], path
  assert sha(path) == source_freeze[path]['sha256']
assert changed == changed_sources, changed
assert new_sources <= set(source_freeze)
legacy = read(E+'legacy-baseline.json')['files']
assert len(legacy) == 23
extension = ("    /// Borrow entries in insertion order in O(1), without copying any records.\n"
 "    /// Callers can reserve traversal/materialization work before inspecting them.\n"
 "    pub fn entries(&self) -> &[EntryRecord] {\n"
 "        &self.records.entries\n"
 "    }\n").encode()
for record in legacy:
 body = safe_file(record['path']).read_bytes()
 if record['path'] == K+'src/identity/mod.rs':
  assert body.count(extension) == 1
  body = body.replace(extension,b'',1)
  assert body == safe_file(F+'identity-mod.before.txt').read_bytes()
 assert hashlib.sha256(body).hexdigest() == record['sha256'], record['path']
proof = read(F+'identity-accessor-extension.json')
assert proof['exact_added_utf8'].encode() == extension
assert proof['current_sha256'] == sha(K+'src/identity/mod.rs')
# Same claim/routing/disposition and bounded observation identities as immediately
# before this integrated wave; no duplicate observations or silent C0 promotions.
before = read(F+'before/docs/completion/execution/register-v1.json')['requirements']
for row,old in zip(reg['requirements'],before):
 assert [row[k] for k in ['id','owner','task','plan','status','evidence_level','test_result']] == [old[k] for k in ['id','owner','task','plan','status','evidence_level','test_result']]
 assert row['reconciliation']['disposition'] == old['reconciliation']['disposition']
prior_studio = read(F+'before/docs/completion/execution/studio-research-v1.json')
assert [x['id'] for x in data['local_mechanism_observations']] == [x['id'] for x in prior_studio['local_mechanism_observations']]
manifest = read(F+'current-source-manifest.json')['files']
mapping = read(F+'requirement-mapping.json')
mapped = {p for g in mapping['groups'] for p in g['sources']+g['tests']}
assert mapped == {r['path'] for r in manifest} - {K+'src/lib.rs'}
assert new_sources <= mapped and changed_sources <= mapped
for record in manifest:
 assert record['sha256'] == sha(record['path'])
 assert record['mapped_ids'] and record['mapped_decisions']
known_ids = {r['id'] for r in reg['requirements']}
known_decisions = {d['id'] for d in data['decisions']}
for group in mapping['groups']:
 assert len(group['ids']) == len(set(group['ids'])) and set(group['ids']) <= known_ids
 assert len(group['decisions']) == len(set(group['decisions'])) and set(group['decisions']) <= known_decisions
 assert group['boundary']
for observation in data['local_mechanism_observations']:
 assert 'fn '+observation['direct_test']['name']+'(' in safe_file(observation['direct_test']['path']).read_text()
summary = read(F+'gates/summary.json')
assert len(summary['gates']) == 4 and all(g['exit_code'] == 0 for g in summary['gates'])
for gate in summary['gates']:
 assert sha(F+'gates/'+gate['gate']+'.log') == gate['log_sha256']
 assert read(F+'gates/'+gate['gate']+'.json')['source_unchanged_during_gate']
assert len(safe_file(F+'language-runtime/native.json').read_bytes()) == 151812
assert sha(F+'language-runtime/native.json') == sha(F+'language-runtime/wasm.json') == '743d07418aadbe60c704d0d934c2bffb237e62e1e39c162ad731aba7d4a8e4d9'
assert len(safe_file(F+'identity-runtime/native.json').read_bytes()) == 4205
assert sha(F+'identity-runtime/native.json') == sha(F+'identity-runtime/wasm.json') == '60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580'
assert 'ordered scanner component names/count' in safe_file(F+'language-runtime/negative-label.log').read_text()
assert 'full32 digest' in safe_file(F+'language-runtime/negative.log').read_text()
print('PASS: historical raw/frozen bytes preserved; exactly 12 reviewed existing product edits and 4 added files pinned at absolute freeze.')
print('PASS: original 23-file legacy baseline retained: 22 unchanged + one exact immutable accessor-only extension; identity trace unchanged.')
print('PASS: current source/test mappings, 15 unchanged observation IDs, actual 151812-byte language trace and root four-gate source continuity.')
print('Final scoped independent rereview pending; no whole-case, scientific, host, hardware or acceptance qualification.')
