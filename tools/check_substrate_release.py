#!/usr/bin/env python3
"""Validate the version-1 substrate release inventory. Never promote claims.

The independently pinned catalog freezes membership, source identities and scope
routing. A catalog change requires explicit checker-pin review, never auto-repair.
Python 3.9 standard library only. Reads are bounded and reject symlink components.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys

MAX_BYTES = 8 * 1024 * 1024
CATALOG_PIN = 'e65441f2a36c099a1c752735ee9d3f109fe0b30c57e71158585e77408b8b955f'
DEPENDENCIES = {1: [], 2: [1], 3: [1], 4: [2, 3], 5: [4], 6: [2, 4, 5], 7: [3], 8: [7], 9: [4, 8], 10: [9], 11: [2, 3], 12: [3, 9], 13: [2, 3], 14: [13], 15: [13], 16: [2, 4], 17: [4, 16], 18: [4, 5], 19: [2, 4, 9, 18], 20: [4, 5, 6, 9, 11, 12, 14, 15, 16, 17, 18, 19], 21: [3, 19, 20], 22: [20, 21]}
PROTOCOL_PIN = '46fcc50ae82ca122645b75ceeda4be013d19e1f950ce0c90b16bec919f38219d'
BASELINE_PIN = 'c63443d76942595ba64b0515feca7fe180c61cf9edb1ab35ecaa2d5a734d1b76'
CLAIM_POLICY = 'C0 only: no implementation, qualification, production completion or approval.'
SCOPE_REVIEW_PIN = '95a1dea4d4bac0694eb2fbf7ccd796ff1bda4d43687a022baa2160e667510e1c'
DIRECTORY = 'docs/completion/substrate-release'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(root, relative):
    """Only bounded regular files below root; reject even in-root symlinks."""
    if not isinstance(relative, str) or '\\' in relative:
        raise ValueError('invalid relative path')
    parts = PurePosixPath(relative).parts
    if not parts or relative.startswith('/') or any(p in ('.', '..') for p in parts):
        raise ValueError('path escape')
    current = Path(root)
    if current.is_symlink():
        raise ValueError('symlink root')
    for part in parts:
        current = current / part
        if current.is_symlink():
            raise ValueError('symlink component: ' + relative)
    if not current.is_file():
        raise ValueError('missing regular artifact: ' + relative)
    with current.open('rb') as stream:
        data = stream.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError('oversized artifact: ' + relative)
    return data


def unique_object(pairs):
    """A duplicate key must never conceal a competing claim or routing table."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key: ' + key)
        result[key] = value
    return result


def load_json(data):
    return json.loads(data, object_pairs_hook=unique_object)


def validate(root, live=True, report=None):
    errors = []
    drifted = []
    complete = False
    live_cache = {}

    def finish():
        if report is not None:
            report.update(version=1, mode='live' if live else 'snapshot-only',
                          valid=not errors, complete=complete, errors=errors,
                          drifted_paths=drifted)
        return errors

    def live_identity(path, expected_hash, message):
        actual, read_error = live_cache[path]
        if read_error:
            errors.append(message + ': ' + read_error)
        elif actual != expected_hash:
            errors.append(message)

    def check(condition, message):
        if not condition:
            errors.append(message)

    def artifact(path):
        return read(root, DIRECTORY + '/' + path)

    def nonempty(value):
        return isinstance(value, str) and bool(value.strip())

    try:
        raw = artifact('catalog-v1.json')
        check(digest(raw) == CATALOG_PIN, 'catalog changed; independently reviewed version/pin migration required')
        if digest(raw) != CATALOG_PIN:
            return finish()
        catalog = load_json(raw)
        if live:
            identities = {}
            def identity(path, expected_hash, rid, kind):
                if not isinstance(expected_hash, str) or not re.fullmatch('[0-9a-f]{64}', expected_hash):
                    raise ValueError('invalid live source hash: ' + str(rid))
                entry = identities.setdefault(path, dict(path=path, expected_sha256=expected_hash,
                    actual_sha256=None, declaration_ids=[], source_ids=[], read_error=None))
                if entry['expected_sha256'] != expected_hash:
                    raise ValueError('conflicting expected hashes: ' + str(path))
                entry[kind].append(rid)
            for source in catalog['sources']:
                if source['live_path']:
                    identity(source['live_path'], source['sha256'], source['id'], 'source_ids')
            for kind in ('modules', 'features', 'formats'):
                for row in catalog[kind]:
                    identity(row['path'], row['sha256'], row['id'], 'declaration_ids')
            for path, entry in identities.items():
                try:
                    entry['actual_sha256'] = digest(read(root, path))
                except (OSError, ValueError) as error:
                    entry['read_error'] = str(error)
                live_cache[path] = (entry['actual_sha256'], entry['read_error'])
                if entry['read_error'] or entry['actual_sha256'] != entry['expected_sha256']:
                    drifted.append(entry)

        check(type(catalog['version']) is int and catalog['version'] == 1, 'catalog version')
        source_lines = {}
        for source in catalog['sources']:
            data = artifact(source['path'])
            check(digest(data) == source['sha256'], source['id'] + ': altered source snapshot')
            source_lines[source['id']] = data.decode('utf-8').splitlines()
            if live and source['live_path']:
                live_identity(source['live_path'], source['sha256'], source['id'] + ': live source drift')
        expected = {r['id']: r for r in catalog['items']}
        check(len(expected) == len(catalog['items']), 'duplicate catalog IDs')
        historical = load_json(artifact('sources/historical-requirements.json'))['requirements']
        check(len(historical) == 163, 'historical source cardinality')
        check({i for i in expected if i.startswith('H-')} == {'H-' + r['id'] for r in historical}, 'historical source closure')
        check({i for i in expected if i.startswith('TASK-')} == {'TASK-%02d' % t for t in range(1, 23)}, '22-task closure')
        for kind in ('modules', 'features', 'formats'):
            check(all(r['id'] not in expected for r in catalog[kind]), kind + ': declaration cannot be a behavior obligation')
            for row in catalog[kind]:
                if live:
                    live_identity(row['path'], row['sha256'], row['id'] + ': source identity drift')
                check(bool(re.fullmatch('[0-9a-f]{64}', row['sha256'])), row['id'] + ': source hash')
                check(row['classification'] == 'declaration-coverage-only', row['id'] + ': invalid declaration classification')
                check(all(link in expected and expected[link]['route'] == 'included' for link in row['behavior_links']), row['id'] + ': missing linked behavior')
                blocker = row['missing_behavior_blocker']
                check(blocker['status'] == 'open' and nonempty(blocker['condition']) and blocker['owner'] == row['owner'] and blocker['task'] == row['task'], row['id'] + ': missing behavior blocker')
        review_bytes = artifact('scope-review-v1.json')
        check(digest(review_bytes) == SCOPE_REVIEW_PIN, 'scope review changed; reviewed pin migration required')
        review = load_json(review_bytes)
        check(type(review['version']) is int and review['version'] == 1, 'scope review version')
        protocol_bytes = artifact('protocols-v1.json')
        check(digest(protocol_bytes) == PROTOCOL_PIN, 'test/evidence protocol changed; reviewed pin migration required')
        protocols = load_json(protocol_bytes)['protocols']
        check(set(protocols) == {'task-%02d' % t for t in range(1, 23)}, 'protocol task closure')
        inventory = load_json(artifact('inventory-v1.json'))
        check(set(inventory) == {'version', 'claim_policy', 'obligations'}, 'unknown release fields')
        check(inventory['claim_policy'] == CLAIM_POLICY, 'C0 claim policy drift')
        check(type(inventory['version']) is int and inventory['version'] == 1, 'release version')
        rows = inventory['obligations']
        check(Counter(r['id'] for r in rows) == Counter(expected.keys()), 'obligation closure/duplicate routing')
        historical_ids = {r['id'] for r in historical}
        for row in rows:
            rid = row['id']
            check(set(row) == (set(expected[rid]) | {'owner', 'depends_on', 'assertion', 'proposed_test', 'expected_evidence', 'closure', 'status', 'evidence_level', 'evidence'}) if rid in expected else False, rid + ': unknown/missing obligation fields')
            check(rid in expected, rid + ': unknown obligation')
            if rid not in expected:
                continue
            for key, value in expected[rid].items():
                check(row.get(key) == value, rid + ': changed source/task/scope/reason ' + key)
            check(type(row['task']) is int and 1 <= row['task'] <= 22, rid + ': invalid task')
            check(row.get('status') == 'proposed' and row.get('evidence_level') == 'C0' and row.get('evidence') == [], rid + ': fabricated success/promotion')
            check(all(nonempty(row.get(k)) for k in ('owner', 'reason', 'assertion')), rid + ': incomplete acceptance contract')
            protocol_id = 'task-%02d' % row['task']
            protocol = protocols[protocol_id]
            check(row['owner'] == protocol['owner'], rid + ': owner mismatch')
            check(row.get('expected_evidence') == {'protocol': protocol_id, 'required': True}, rid + ': missing evidence requirements')
            check(row.get('proposed_test') == {'protocol': protocol_id, 'oracle': row['source_ref'], 'mode': 'direct-assertion' if row['route'] == 'included' else 'scope-audit-only'}, rid + ': missing direct test/oracle')
            check(row.get('closure') == {'assertion': row['source_ref'], 'protocol': protocol_id, 'behavior_remains_open': row['route'] != 'included'}, rid + ': falsified closure')
            check(isinstance(row.get('depends_on'), list) and row['depends_on'] == ['TASK-%02d' % d for d in DEPENDENCIES[row['task']]], rid + ': dependency closure')
            ref = row['source_ref']
            if ref.startswith('historical#'):
                check(ref.split('#', 1)[1] in historical_ids, rid + ': missing historical reference')
                check(row['assertion'] == next(r['completion_condition'] for r in historical if r['id'] == ref.split('#', 1)[1]), rid + ': historical oracle drift')
            elif ref.rsplit(':', 1)[0] in source_lines:
                key, line = ref.rsplit(':', 1)
                check(1 <= int(line) <= len(source_lines[key]) and bool(source_lines[key][int(line)-1].strip()), rid + ': missing source line')
                check(row['assertion'] == source_lines[key][int(line)-1], rid + ': source oracle drift')
            else:
                check(False, rid + ': unhandled source reference kind')
        baseline_bytes = artifact('baseline-v1.json')
        check(digest(baseline_bytes) == BASELINE_PIN, 'baseline identity snapshot drift')
        baseline = load_json(baseline_bytes)
        check(type(baseline['version']) is int and baseline['version'] == 1 and bool(re.fullmatch('[0-9a-f]{40}', baseline['head'])), 'baseline identity/version')
        check(bool(baseline['limitations']) and bool(baseline['toolchains']) and bool(baseline['existing_receipts']), 'incomplete baseline')
        for row in baseline['toolchains']:
            check(digest(row['contents'].encode()) == row['sha256'], 'baseline toolchain hash')
        for row in baseline['dirty_files'] + baseline['existing_receipts']:
            check(bool(re.fullmatch('[0-9a-f]{64}', row['sha256'])) and not row['path'].startswith('/') and '..' not in PurePosixPath(row['path']).parts, 'invalid baseline public identity')
        check(all(r['status'] == 'unavailable-original' and nonempty(r['impact']) for r in catalog['missing_sources']), 'missing original cannot become evidence')
        complete = True
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError) as error:
        errors.append('invalid release contract: ' + str(error))
    return finish()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--drift-report', action='store_true', help='emit live drift diagnostics as JSON; retain refusal exit status')
    modes.add_argument('--snapshot-only', action='store_true', help='validate frozen contract only; do not qualify live source identities')
    args = parser.parse_args()
    report = {} if args.drift_report else None
    errors = validate(args.root, live=not args.snapshot_only, report=report)
    if errors:
        for error in errors:
            print('FAIL: ' + error, file=sys.stderr)
    if args.drift_report:
        print(json.dumps(report, indent=2))
        return 1 if errors else 0
    if errors:
        return 1
    print('Substrate release C0 contract valid; no implementation, approval or release qualification claim.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
