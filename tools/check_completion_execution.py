#!/usr/bin/env python3
"""Check execution inventory and hash observations, never promote C0 claims."""
import argparse
from collections import Counter
import hashlib
import json
import re
from pathlib import Path, PurePosixPath
import sys

REGISTER = 'docs/completion/execution/register-v1.json'
CONTRACT_PINS = {'docs/completion/requirements.json': '571e55910ca882db6141c616f1a0de036baec7fe211c371ca1ddfeb04691fa65', 'docs/completion/plans.json': 'ca85e660fd548f26bdfffccc16a16e246caa13e54006543c63ee785107910e7d'}
AUDIT_MANIFESTS = ['docs/completion/execution-status.json', 'docs/completion/complete-episode-decoder-v1-source.json', 'docs/completion/complete-episode-python-source.json']

CANDIDATE_PATHS = 'docs/completion/execution/candidate-paths-v1.json'
CANDIDATE_PATHS_SHA256 = '1e5b1946385d6914cb32ea3e45148fa855dcc1cd756ae53b679640664aed3502'

# Exact per-ID artifact and review path identities. The pin is set only after an
# independent reviewer verifies that the catalog reproduces the live register.
REVIEW_ARTIFACT_PATHS = 'docs/completion/execution/review-artifact-paths-v1.json'
REVIEW_ARTIFACT_PATHS_SHA256 = '0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf'

# README count statements. Bullet integers are compared exactly, and the
# "Local gaps:"/"External gaps:" lists are compared as ID sets after expanding
# en-dash ranges such as P05.01–P05.04 and OS01–OS06. Review pointers are
# counted as unique catalog paths: "historical" are those outside the 2026-10-23
# scoring packet, "scoring-packet" are those inside it, plus their total.
README = 'docs/completion/execution/README.md'
SCORING_REVIEW_PREFIX = 'docs/completion/execution/evidence/2026-10-23-scoring/'
README_BULLETS = {
    'candidate': re.compile(r'^- \*\*(\d+) candidate mappings\*\*', re.M),
    'local_gap': re.compile(r'^- \*\*(\d+) local gaps\*\*', re.M),
    'external_gap': re.compile(r'^- \*\*(\d+) external gaps\*\*', re.M),
    'historical_review': re.compile(r'^- \*\*(\d+) historical review/receipt pointers\*\*', re.M),
    'scoring_review': re.compile(r'^- \*\*(\d+) scoring-packet review pointers\*\*', re.M),
    'total_review': re.compile(r'\b(\d+) unique review paths\b'),
}
README_LISTS = {
    'local_gap': re.compile(r'^Local gaps: (.*?)\.\s', re.M | re.S),
    'external_gap': re.compile(r'^External gaps: (.*?)\.\s', re.M | re.S),
}


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


def read_review_artifact_catalog(root):
    """Return (sha256, catalog) for the review/artifact path catalog."""
    path = safe_file(root, REVIEW_ARTIFACT_PATHS)
    return sha(path), json.loads(path.read_text())


def expand_ids(text):
    """Expand a README ID list ("A, B; P05.01–P05.04") into a list of IDs."""
    ids = []
    for token in re.split(r'[,;]', ' '.join(text.split())):
        token = token.strip()
        if '–' not in token:
            ids.append(token)
            continue
        first, last = (part.strip() for part in token.split('–'))
        head_a, num_a = re.fullmatch(r'(.*?)(\d+)', first).groups()
        head_b, num_b = re.fullmatch(r'(.*?)(\d+)', last).groups()
        if head_a != head_b or len(num_a) != len(num_b) or int(num_a) > int(num_b):
            raise ValueError('invalid README ID range: ' + token)
        ids.extend(head_a + str(n).zfill(len(num_a)) for n in range(int(num_a), int(num_b) + 1))
    return ids


def readme_errors(text, candidates, review_catalog):
    """Compare README count statements with the two pinned catalogs."""
    errors = []
    by_disposition = {kind: sorted(i for i, entry in candidates.items()
                                   if entry.get('disposition') == kind)
                      for kind in ('candidate', 'local_gap', 'external_gap')}
    reviews = {path for entry in review_catalog.values() for path in entry['review']}
    scoring = {path for path in reviews if path.startswith(SCORING_REVIEW_PREFIX)}
    expected = {kind: len(ids) for kind, ids in by_disposition.items()}
    expected.update(historical_review=len(reviews - scoring), scoring_review=len(scoring),
                    total_review=len(reviews))
    for kind, pattern in README_BULLETS.items():
        found = [int(v) for v in pattern.findall(text)]
        if found != [expected[kind]]:
            errors.append('README count drift: %s %r != %d' % (kind, found, expected[kind]))
    for kind, pattern in README_LISTS.items():
        found = pattern.findall(text)
        try:
            ids = expand_ids(found[0]) if len(found) == 1 else None
        except (ValueError, AttributeError):
            ids = None
        if ids is None or len(ids) != len(set(ids)) or sorted(ids) != by_disposition[kind]:
            errors.append('README count drift: %s list' % kind)
    return errors


def validate(root, data=None, readme=None):
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
        if REVIEW_ARTIFACT_PATHS_SHA256 is None:
            errors.append('review/artifact catalog pin pending review')
        review_sha, review_catalog = read_review_artifact_catalog(root)
        if REVIEW_ARTIFACT_PATHS_SHA256 is not None and review_sha != REVIEW_ARTIFACT_PATHS_SHA256:
            errors.append('review/artifact catalog drift')
        if set(review_catalog) != {r['id'] for r in expected}:
            errors.append('review/artifact catalog requirement closure')
        for entry in review_catalog.values():
            if (not isinstance(entry, dict) or set(entry) != {'artifacts', 'review'}
                    or not all(isinstance(entry[k], list)
                               and all(isinstance(p, str) for p in entry[k])
                               for k in entry)):
                raise ValueError('invalid review/artifact catalog entry')
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

            if not is_candidate and references['review']:
                errors.append('gap cannot carry review: ' + row['id'])
            identity = review_catalog.get(row['id'], {})
            if Counter(ref['path'] for ref in references['review']) != Counter(identity.get('review', [])):
                errors.append('review identity drift: ' + row['id'])
            if Counter(ref['path'] for ref in references['artifacts']) != Counter(identity.get('artifacts', [])):
                errors.append('artifact identity drift: ' + row['id'])
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
        readme = safe_file(root, README).read_text() if readme is None else readme
        errors.extend(readme_errors(readme, candidates, review_catalog))
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
