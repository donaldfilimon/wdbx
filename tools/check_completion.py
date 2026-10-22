#!/usr/bin/env python3
"""Validate C0 completion contracts. No product conformance or claim promotion.

Run from any directory. Reference snapshots work without sibling checkouts;
when live authoritative sources are present their hashes must also agree.
Updating pins is a reviewed contract-version change, never an automatic repair.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

PINS = {
    'program': 'bf0b774c5b66e8a164b72d636c064ca40274851da2f07265f2c48fd451a47e1e',
    'studio': '9064df9a034e381945a4fa11560609df9c3c9dbe7e3431aea7cd9fd19319f32c',
    'constitution': '7b3f932a08f55bb4bcf0dfe2b45ea356da84bc79d6103101f70900738b4c5bcc',
    'episodes': '8c0c6295d9956290b45f0180a13ddfd42546911ad5bf5bc83e57f06868681526',
}
SOURCE_PATHS = {
    'program': 'docs/superpowers/plans/2026-10-20-wdbx-full-completion.md',
    'studio': 'specimen-studio/public/WDBX-Specimen-Architecture-Specification.md',
    'constitution': '../abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md',
    'episodes': '../abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md',
}
CORPUS_PIN = '901afe708b7b4ffe50d1a0f89b9c2aad3425e0b1e77e8617987b6b19c216fec9'
PROTOCOL_PIN = 'c8a387eae542e047da64473c2f3a852e23caa6cf8b56f96d82f20e46d877ab9c'
OWNERS = {
    1: 'WDBX contracts', 2: 'WDBX', 3: 'WDBX', 4: 'WDBX',
    5: 'ABI with Abbey consumers', 6: 'ABI federation and WDBX',
    7: 'Studio sync and ABI', 8: 'Studio kernel/native',
    9: 'Studio and WDBX', 10: 'platform operators',
}
DEPENDENCIES = {
    1: [], 2: [1], 3: [2], 4: [2, 3], 5: [2, 3, 4],
    6: [2, 3, 5], 7: [3, 5], 8: [1], 9: [3, 5, 7, 8],
    10: [5, 6, 7, 9],
}
CONTRACT_PINS = {
    'experiments-v1.json': 'a5a1bde6c53e2efeb69ba61ac632092629704e985004474f50f186d8f85b3809',
    'synthetic-corpus-v1.jsonl': '044da91e58c90c067137a733b6eb86e49376b73bb05b9b7dfebbc1cb089d2232',
    'synthesis-network-v1.json': 'e667909e9acf8e75f01e876ae43513f955eee2b94bf681be545dc141ef19359c',
    'synthesis-operations-v1.json': 'a8317ba47e35b98bfa9eb15392ab65c49c1f16ae2bd5296bf3c817c2f0c76c63',
    'synthesis-worked-v1.json': '57c85c6d10be20ba9698598cb95e5a0b4680d34ec0e16275a5352ea2923c1adc',
    'synthesis-contract-v1.md': '45f6ef7f13f278dc537aec66354d0aa40ec174939b34a7edca13e0f948607c75',
    'inhibition-contract-v1.md': 'cd8511f32edd410a7fb083f8612c1e52b556f91991cc8ef5b2caea5e99578481',
    'inhibition-vectors-v1.json': 'e935e2b8f62d87cbd8c3c23ae4b4c6051aea6acbcf88ab14868a82d0a69cb2d4',
    'shared-types-v1.json': '5d51d33a6ce4adc8628f780bd913520f65e9d7f5ae6b875fe03027bd408d527b',
    'contract_vectors.py': '85f298d0e0a8ae24410c25e7d91ee98dae43cc805affa0d720122fd040842c3e',
    'contract_vectors_v1.py': 'c5379c39d21b3e7491383ecb3b19e7716838aafac4ea117d0a01165185e6a412',
    'experiments-v2.json': '02aaef494407922f7ed4234b9165bb261a59345586f65021dab7677c6b563efa',
    'synthesis-contract-v2.md': 'd66500bd4bac7d4602c69377b0d96a096bfd7984d88c498f9ac4e744b9af868f',
    'inhibition-contract-v2.md': '50905e7090b8d1453fdb9681520907001b6644ac13834f308bd4d4d04e902472',
    'synthesis-operations-v2.json': 'e17d7b752cb68730757f9e224cae835048333644f7b6fbe750894fcb781ceeb4',
    'synthesis-worked-v2.json': '6b547b8cb4ec86e369fc4f667a9069b1f15c4c145b6971b802d487292450f507',
    'synthesis-near-ties-v2.json': 'a23588bd4dd697532bc55c751f5b1966045ebc16d68a1a32e2076d0cb609a92c',
    'inhibition-vectors-v2.json': 'b88e1bcd751dae35518a3cf3ce90fe69a9c13f4236edb97e1e7fadaafdb44d42',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def metric_errors(protocol, records):
    """A frozen threshold without usable case inputs/oracles is not executable."""
    errors = []
    metrics = protocol['metric_contracts']
    if set(metrics) != set(protocol['thresholds']):
        errors.append('metric/threshold closure')
    for name, metric in metrics.items():
        if not metric.get('denominator') or not metric.get('pass'):
            errors.append(name + ': metric lacks denominator/pass rule')
        suites = metric['suite'].split(',')
        cases = [r for r in records if r['suite'] in suites or metric['suite'] == 'all']
        if not cases or not any(r['split'] == 'test' for r in cases):
            errors.append(name + ': no held-out cases')
        for case in cases:
            for section in ('input', 'oracle'):
                for field in metric[section]:
                    value = case[section]
                    for key in field.split('.'):
                        value = value.get(key) if isinstance(value, dict) else None
                    if value is None or value == [] or value == '':
                        errors.append(name + ': missing ' + section + '.' + field)
    # Variant labels and opaque IDs cannot turn replicated inputs into families.
    signatures = {}
    for record in records:
        if record['suite'] != 'retrieval':
            continue
        query = {k: v for k, v in record['input']['query'].items() if k != 'question'}
        candidates = [{k: v for k, v in c.items() if k != 'id'} for c in record['input']['candidates']]
        candidates.sort(key=lambda c: json.dumps(c, sort_keys=True))
        signature = json.dumps([query, candidates], sort_keys=True)
        family = record['family']
        if signature in signatures and signatures[signature] != family:
            errors.append('retrieval: replicated scenario assigned independent families')
        signatures[signature] = family
    return errors


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
            check(type(row['task']) is int, rid + ': task must be an integer')
            check(row.get('status') == 'proposed' and row.get('evidence_level') == 'C0' and row.get('evidence') == [], rid + ': unsupported claim promotion/evidence')
            check(all(nonempty(row.get(f)) for f in ('owner', 'plan', 'expected_evidence', 'completion_condition')), rid + ': missing owner/plan/condition/evidence expectation')
            if re.match(r'^(P\d\d\.\d\d|G\d\d)$', rid):
                check(row.get('completion_condition') == row.get('text'), rid + ': completion condition drifted from the source text')
            if rid in expected:
                check(tuple(row.get(k) for k in ('source', 'line', 'text', 'task')) == expected[rid], rid + ': source/task provenance mismatch')
            check(row['plan'] == 'task-%02d' % row['task'], rid + ': wrong plan routing')
            check(row['owner'] == OWNERS.get(row['task']), rid + ': wrong owner')
        platforms = {r.get('platform') for r in rows if r['id'].startswith('OS')}
        check(platforms == {'desktop:macos-arm64', 'desktop:macos-x86_64', 'desktop:windows-x86_64', 'desktop:linux-x86_64', 'service:linux-x86_64', 'service:macos-arm64'}, 'platform closure')
        plans = load('plans.json')['plans']
        check([p['id'] for p in plans] == ['task-%02d' % n for n in range(1, 11)], 'plan closure/order')
        check([p['task'] for p in plans] == list(range(1, 11)), 'plan numeric task closure/order')
        routed = [rid for plan in plans for rid in plan['requirement_ids']]
        check(Counter(routed) == Counter(ids), 'requirements must be routed exactly once')
        for position, plan in enumerate(plans, 1):
            n = plan['task']
            check(type(n) is int, plan['id'] + ': task must be an integer')
            check(n == position and plan['id'] == 'task-%02d' % n, 'task ID/number mismatch')
            check(plan.get('owner') == OWNERS.get(n), plan['id'] + ': wrong plan owner')
            check(set(DEPENDENCIES.get(n, [])) <= set(plan['depends_on']), plan['id'] + ': mandatory prerequisite missing')
            check(plan.get('schema_gate') == 'shared-types-v1.json#' + plan['id'], plan['id'] + ': missing schema gate')
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
        check(protocol['version'] == 3 and protocol['corpus'] == 'synthetic-corpus-v2.jsonl', 'active experiment version')
        corpus = (directory / 'synthetic-corpus-v2.jsonl').read_bytes()
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
        errors.extend(metric_errors(protocol, records))
        for filename, pin in CONTRACT_PINS.items():
            check(digest((directory / filename).read_bytes()) == pin, filename + ': contract artifact drift')
        shared = load('shared-types-v1.json')
        check(shared['status'] == 'proposed', 'shared type claim promotion')
        check([g['id'] for g in shared['gates']] == [p['id'] for p in plans], 'shared schema gate closure')
        for gate, plan in zip(shared['gates'], plans):
            check(gate['owner'] == plan['owner'] and gate['status'] == 'proposed', 'schema gate ownership/status')
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
