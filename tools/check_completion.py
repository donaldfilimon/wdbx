#!/usr/bin/env python3
"""Validate C0 completion contracts. No product conformance or claim promotion.

Run from any directory. Reference snapshots work without sibling checkouts;
when live authoritative sources are present their hashes must also agree.
Updating pins is a reviewed contract-version change, never an automatic repair.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

PINS = {
    'program': 'af90b08cf0fa30e889c46afef0c361ba5e28d9d4ad53a441e286965ea09c058d',
    'studio': '82aa6347e264efb789b21e915eb802802d1b3771c31d1a63f1fb85ee2a0c4c88',
    'constitution': '7b3f932a08f55bb4bcf0dfe2b45ea356da84bc79d6103101f70900738b4c5bcc',
    'episodes': '8c0c6295d9956290b45f0180a13ddfd42546911ad5bf5bc83e57f06868681526',
}
SOURCE_PATHS = {
    'program': 'docs/superpowers/plans/2026-10-20-wdbx-full-completion.md',
    'studio': 'specimen-studio/public/WDBX-Specimen-Architecture-Specification.md',
    'constitution': '../abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md',
    'episodes': '../abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md',
}
CORPUS_PIN = '5b3f5bb45c51bbb65aeef28555cbc5469bf1951720cadb228947ec1b3855736a'
PROTOCOL_PIN = '4017e1c49c8b249c6b4998585c13cbd12e88455447f3d4ad0893e9ec40041697'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def source_rows(lines, section, header):
    active = False
    out = []
    for line, text in enumerate(lines, 1):
        if text.startswith('### '):
            active = text.startswith('### ' + section + ' ')
        if active and text.startswith('| ') and not text.startswith('| ' + header + ' |'):
            out.append((line, text))
    return out


def expected_requirements(sources):
    """Derive membership from separately pinned source, not editable inventory."""
    expected = {}
    task = count = constraint = 0
    for line, text in enumerate(sources['program'], 1):
        match = re.match(r'## Task (\d+):', text)
        if match:
            task, count = int(match[1]), 0
        if text.startswith('- [ ] '):
            count += 1
            expected['P%02d.%02d' % (task, count)] = ('program', line, text, task)
        if 15 <= line <= 27:
            constraint += 1
            expected['G%02d' % constraint] = ('program', line, text, 1)
    for line, text in enumerate(sources['studio'], 1):
        match = re.match(r'## (\d+)\. ', text)
        if match:
            expected['CH%02d' % int(match[1])] = ('studio', line, text, 8)
    for number, (line, text) in enumerate(source_rows(sources['studio'], '25.3', 'Case'), 1):
        expected['S%02d' % number] = ('studio', line, text, 8)
    count = 0
    for line in range(251, 322):
        text = sources['constitution'][line - 1]
        if text.startswith(('- ', '**')):
            count += 1
            expected['C%02d' % count] = ('constitution', line, text, 3 if line >= 296 else 2)
    dimensions = ('cryptographic', 'source', 'calibration', 'outcome', 'compatibility', 'regime', 'lifecycle', 'confidence')
    for number, dimension in enumerate(dimensions, 1):
        for line, text in enumerate(sources['episodes'], 1):
            if text.startswith(dimension + ' '):
                expected['E%02d' % number] = ('episodes', line, text, 4)
                break
    for number in range(1, 7):
        expected['OS%02d' % number] = ('program', 24, sources['program'][23], 10)
    return expected


def validate(root, live_sources=True):
    root = Path(root)
    directory = root / 'docs/completion'
    errors = []

    def check(condition, message):
        if not condition:
            errors.append(message)

    def load(name):
        return json.loads((directory / name).read_text(encoding='utf-8'))

    def nonempty(value):
        return isinstance(value, str) and bool(value.strip()) or isinstance(value, list) and bool(value) and all(nonempty(v) for v in value)

    try:
        catalog = load('sources.json')
        check(set(catalog) == set(PINS), 'source catalog closure')
        sources = {}
        for key, pin in PINS.items():
            meta = catalog[key]
            check(meta['path'] == SOURCE_PATHS[key], key + ': authoritative path changed')
            check(meta['snapshot'] == 'sources/' + key + '.md', key + ': snapshot path changed')
            data = (directory / ('sources/' + key + '.md')).read_bytes()
            check(digest(data) == pin == meta['sha256'], key + ': source digest mismatch')
            sources[key] = data.decode('utf-8').splitlines()
            live = root / SOURCE_PATHS[key]
            if live_sources and key in ('program', 'studio'):
                check(live.is_file(), key + ': required local source missing')
            if live_sources and live.exists():
                check(digest(live.read_bytes()) == pin, key + ': live source drift; reviewed refresh required')
        inventory = load('studio-acceptance-v1.json')
        check(inventory['version'] == 1, 'acceptance inventory version')
        check(inventory['source_sha256'] == PINS['studio'], 'acceptance source digest')
        for field, section, header, prefix, size in [('cases', '25.3', 'Case', 'S', 46), ('decisions', '25.1', 'Decision', 'D', 18)]:
            expected = [{'id': '%s%02d' % (prefix, n), 'line': line, 'row': row} for n, (line, row) in enumerate(source_rows(sources['studio'], section, header), 1)]
            check(len(expected) == size and inventory[field] == expected, field + ': missing or modified source row')
        manifest = load('requirements.json')
        expected = expected_requirements(sources)
        rows = manifest['requirements']
        ids = [r['id'] for r in rows]
        check(len(ids) == len(set(ids)), 'duplicate requirement ID')
        check(set(ids) == set(expected), 'required ID closure differs from pinned sources')
        check(manifest['version'] == 1 and len(expected) == 163, 'manifest version/cardinality')
        for row in rows:
            rid = row['id']
            check(row.get('status') == 'proposed' and row.get('evidence_level') == 'C0' and row.get('evidence') == [], rid + ': unsupported claim promotion/evidence')
            check(all(nonempty(row.get(f)) for f in ('owner', 'plan', 'expected_evidence', 'completion_condition')), rid + ': missing owner/plan/condition/evidence expectation')
            if rid in expected:
                check(tuple(row.get(k) for k in ('source', 'line', 'text', 'task')) == expected[rid], rid + ': source/task provenance mismatch')
            check(row['plan'] == 'task-%02d' % row['task'], rid + ': wrong plan routing')
        platforms = {r.get('platform') for r in rows if r['id'].startswith('OS')}
        check(platforms == {'desktop:macos-arm64', 'desktop:macos-x86_64', 'desktop:windows-x86_64', 'desktop:linux-x86_64', 'service:linux-x86_64', 'service:macos-arm64'}, 'platform closure')
        plans = load('plans.json')['plans']
        check([p['id'] for p in plans] == ['task-%02d' % n for n in range(1, 11)], 'plan closure/order')
        for plan in plans:
            n = plan['task']
            check(plan['status'] == 'proposed' and plan['evidence_level'] == 'C0', plan['id'] + ': plan promotion')
            check(all(nonempty(plan.get(f)) for f in ('interfaces', 'steps', 'acceptance', 'expected_evidence')), plan['id'] + ': incomplete execution contract')
            check(set(plan['requirement_ids']) == {r['id'] for r in rows if r['task'] == n}, plan['id'] + ': missing requirement routing')
            check(all(isinstance(d, int) and 1 <= d < n for d in plan['depends_on']), plan['id'] + ': invalid dependency')
        designs = load('designs.json')['designs']
        check({d['id'] for d in designs} == {'identity','language','scoring','synthesis','atp','vision','supervision','mutation','scheduler'}, 'design closure')
        decisions = [d for design in designs for d in design['decisions']]
        check(len(decisions) == 18 and set(decisions) == {'D%02d' % n for n in range(1, 19)}, 'design decision closure')
        for design in designs:
            check(design['status'] == 'proposed' and design['evidence_level'] == 'C0', design['id'] + ': design promotion')
            check(all(nonempty(design.get(f)) for f in ('language','interface','algorithm','limits','failure_behavior','acceptance')), design['id'] + ': incomplete design')
        protocol = load('experiments.json')
        check(digest((directory / 'experiments.json').read_bytes()) == PROTOCOL_PIN, 'frozen protocol changed; versioned review required')
        corpus = (directory / 'synthetic-corpus-v1.jsonl').read_bytes()
        check(digest(corpus) == CORPUS_PIN == protocol['corpus_sha256'], 'frozen corpus digest mismatch')
        records = [json.loads(line) for line in corpus.decode('utf-8').splitlines()]
        check(len(records) == protocol['records'] == 2400, 'corpus count')
        families = [f for split in protocol['splits'].values() for f in split]
        check(len(families) == len(set(families)) == 800, 'split overlap or dropped family')
        check(set(protocol['splits']) == {'train','validation','test'}, 'split roles')
        check(len({r['id'] for r in records}) == len(records), 'duplicate corpus case')
        check({r['family'] for r in records} == set(families), 'corpus family closure')
        for record in records:
            check(record['family'] in protocol['splits'][record['split']], 'corpus split mismatch')
        baseline = load('baseline.json')
        check(set(baseline['repositories']) == {'wdbx','abi','abbey'}, 'baseline repositories')
        for repo in baseline['repositories'].values():
            check(bool(re.fullmatch('[0-9a-f]{40}', repo['head'])), 'baseline revision')
        check(nonempty(baseline['limitations']) and len(baseline['gates']) >= 3 and len(baseline['observations']) >= 6, 'incomplete baseline')
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError) as error:
        errors.append('invalid contract structure: ' + str(error))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    errors = validate(args.root)
    if errors:
        for error in errors:
            print('FAIL: ' + error, file=sys.stderr)
        return 1
    print('Completion contracts valid: 163 C0 requirements, 46 acceptance cases; no implementation/efficacy claim.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
