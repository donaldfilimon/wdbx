#!/usr/bin/env python3
"""Check execution inventory and hash observations, never promote C0 claims."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys

REGISTER = 'docs/completion/execution/register-v1.json'
CONTRACT_PINS = {'docs/completion/requirements.json': '571e55910ca882db6141c616f1a0de036baec7fe211c371ca1ddfeb04691fa65', 'docs/completion/plans.json': 'ca85e660fd548f26bdfffccc16a16e246caa13e54006543c63ee785107910e7d'}
AUDIT_MANIFESTS = ['docs/completion/execution-status.json', 'docs/completion/complete-episode-decoder-v1-source.json', 'docs/completion/complete-episode-python-source.json']

CANDIDATE_PATHS = 'docs/completion/execution/candidate-paths-v1.json'
CANDIDATE_PATHS_SHA256 = 'da5bb9e1c51f9872fc6245ab2d9214885f30d288545bf10c9da76978d387d8e0'

def safe_file(root, path):
    if not isinstance(path, str) or not path or '\\' in path:
        raise ValueError('unsafe path')
    relative = PurePosixPath(path)
    if relative.is_absolute() or any(p in ('..', '.') for p in path.split('/')):
        raise ValueError('unsafe path: ' + path)
    target = root / path
    cursor = root
    for component in relative.parts:
        cursor = cursor / component
        if cursor.is_symlink():
            raise ValueError('symlink path: ' + path)
    if not target.resolve().is_relative_to(root.resolve()) or not target.is_file():
        raise ValueError('missing or escaping file: ' + path)
    return target


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_register(root):
    return json.loads((root / REGISTER).read_text())


def reference_list(section):
    """Reject malformed evidence references before path/hash processing."""
    if (not isinstance(section, dict) or set(section) != {'state', 'references'}
            or section['state'] != 'unqualified'
            or not isinstance(section['references'], list)):
        raise ValueError('invalid evidence reference section')
    for ref in section['references']:
        if (not isinstance(ref, dict) or set(ref) != {'path', 'sha256'}
                or not isinstance(ref['path'], str)
                or not isinstance(ref['sha256'], str)
                or len(ref['sha256']) != 64
                or any(c not in '0123456789abcdef' for c in ref['sha256'])):
            raise ValueError('invalid evidence reference')
    return section['references']


def validate(root, data=None):
    errors = []
    try:
        data = read_register(root) if data is None else data
        expected = json.loads((root / 'docs/completion/requirements.json').read_text())['requirements']
        if data['version'] != 1 or data['claim_policy'] != 'C0-only; no promotion':
            errors.append('unsupported schema or promotion policy')
        required_contracts = set(CONTRACT_PINS)
        if {r['path'] for r in data['contracts']} != required_contracts:
            errors.append('contract pins incomplete')
        for ref in data['contracts']:
            if (ref['sha256'] != CONTRACT_PINS.get(ref['path'])
                    or sha(safe_file(root, ref['path'])) != ref['sha256']):
                errors.append('contract drift: ' + ref['path'])
        catalog_file = safe_file(root, CANDIDATE_PATHS)
        if sha(catalog_file) != CANDIDATE_PATHS_SHA256:
            errors.append('candidate path catalog drift')
        candidates = json.loads(catalog_file.read_text())
        if set(candidates) != {r['id'] for r in expected}:
            errors.append('candidate catalog requirement closure')
        rows = data['requirements']
        if Counter(r['id'] for r in rows) != Counter(r['id'] for r in expected):
            errors.append('missing, extra or duplicate requirement IDs')
        by_id = {r['id']: r for r in expected}
        fields = {'id', 'owner', 'task', 'plan', 'status', 'evidence_level', 'implementation', 'tests', 'artifacts', 'review', 'blockers', 'test_result', 'reconciliation'}
        for row in rows:
            if set(row) != fields:
                errors.append('unexpected or missing requirement fields')
            original = by_id.get(row['id'], {})
            if any(row[k] != original.get(k) for k in ('owner', 'task', 'plan')):
                errors.append('routing drift: ' + row['id'])
            if row['status'] != 'open' or row['evidence_level'] != 'C0' or row['test_result'] != 'unknown':
                errors.append('unreviewed promotion: ' + row['id'])
            if not isinstance(row['blockers'], list) or not row['blockers'] or not all(isinstance(v, str) and v.strip() for v in row['blockers']):
                errors.append('missing blockers: ' + row['id'])
            references = {kind: reference_list(row[kind]) for kind in
                          ('implementation', 'tests', 'artifacts', 'review')}
            reconciliation = row['reconciliation']
            if (not isinstance(reconciliation, dict)
                    or set(reconciliation) != {'disposition', 'reason', 'qualification_plan', 'inspected'}
                    or reconciliation['disposition'] not in {'candidate', 'local_gap', 'external_gap'}
                    or not isinstance(reconciliation['reason'], str)
                    or not reconciliation['reason'].strip()):
                raise ValueError('invalid reconciliation')
            references['qualification_plan'] = reference_list(reconciliation['qualification_plan'])
            references['inspected'] = reference_list(reconciliation['inspected'])
            if not references['qualification_plan'] or not references['inspected']:
                errors.append('missing gap inspection or qualification plan: ' + row['id'])
            catalog = candidates.get(row['id'], {})
            if reconciliation['disposition'] != catalog.get('disposition'):
                errors.append('reconciliation disposition drift: ' + row['id'])
            is_candidate = reconciliation['disposition'] == 'candidate'
            if is_candidate and (not references['implementation'] or not references['tests']):
                errors.append('candidate requires implementation and tests: ' + row['id'])
            if not is_candidate and (references['implementation'] or references['tests']):
                errors.append('gap cannot imply candidate implementation: ' + row['id'])

            if not any(ref['path'] == 'docs/completion/plans.json' for ref in references['artifacts']):
                errors.append('missing required plan artifact: ' + row['id'])
            for kind in ('implementation', 'tests', 'qualification_plan', 'inspected'):
                if (Counter(ref['path'] for ref in references[kind])
                        != Counter(candidates.get(row['id'], {}).get(kind, []))):
                    errors.append('candidate identity drift: ' + row['id'])
            for refs in references.values():
                for ref in refs:
                    if sha(safe_file(root, ref['path'])) != ref['sha256']:
                        errors.append('reference drift: ' + ref['path'])
        if Counter(a['manifest']['path'] for a in data['audits']) != Counter(AUDIT_MANIFESTS):
            errors.append('missing or duplicate audit manifests')
        for audit in data['audits']:
            if sha(safe_file(root, audit['manifest']['path'])) != audit['manifest']['sha256']:
                errors.append('audit manifest drift')
            manifest = json.loads(safe_file(root, audit['manifest']['path']).read_text())
            source_files = manifest.get('files', manifest.get('reader', {}).get('source_sha256', []))
            if [(r['path'], r['sha256']) for r in source_files] != [(r['path'], r['expected_sha256']) for r in audit['files']]:
                errors.append('audit coverage drift')
            for item in audit['files']:
                observed = sha(safe_file(root, item['path']))
                state = 'matched' if observed == item['expected_sha256'] else 'mismatched'
                if observed != item['observed_sha256'] or state != item['state']:
                    errors.append('audit observation drift: ' + item['path'])
            if audit['qualification'] != 'historical_only; no current gate result':
                errors.append('audit cannot qualify gate results')
    except (ValueError, KeyError, TypeError, AttributeError, OSError):
        errors.append('invalid register structure or reference')
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    errors = validate(args.root)
    if errors:
        print('\n'.join(errors), file=sys.stderr)
        return 1
    print('Execution register valid: 163 reconciled open C0 requirements; no conformance or test-pass claim.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
