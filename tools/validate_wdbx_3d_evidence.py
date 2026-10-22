"""Validate the source inventory before treating it as implementation-ready."""
REQUIRED_AUTHORITIES = (
    'north-star', 'multiway', 'agent-architecture', 'canonical-episodes',
    'completion-design', 'member-erasure', 'claims', 'readme', 'constitution',
    'original-specimen', 'runtime-profile', 'acceptance-ledger',
    'specification-v6', 'master-plan-v5', 'requirements-v6',
    'command-contracts', 'gateway-schema', 'toolchain', 'lockfile', 'current-tests',
)

import argparse
import json
from pathlib import Path, PurePosixPath
import re


def _text(value):
    return isinstance(value, str) and bool(value.strip())


def _digest(value, width):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{%d}' % width, value) is not None


def _relative_file(value):
    if not _text(value) or value != value.strip():
        return False
    return (not any(ord(c) < 32 for c in value)
            and not any(c in value for c in ('\\', ':'))
            and not PurePosixPath(value).is_absolute()
            and all(part not in ('', '.', '..') for part in value.split('/')))


def _valid_ranges(ranges, count):
    if type(count) is not int or count < 1 or not isinstance(ranges, list):
        return False
    end = 0
    for span in ranges:
        if (not isinstance(span, list) or len(span) != 2
                or any(type(n) is not int for n in span)):
            return False
        first, last = span
        if first <= end or last < first or last > count:
            return False
        end = last
    return True


def _complete_ranges(ranges, count):
    return (_valid_ranges(ranges, count) and bool(ranges)
            and ranges[0][0] == 1 and ranges[-1][1] == count
            and all(right[0] == left[1] + 1
                    for left, right in zip(ranges, ranges[1:])))


def validate_source_manifest(document):
    """Check inventory admission; this does not verify source bytes or test logs.

    Required authorities cover complete documents except explicitly scoped plan
    and lockfile inventories. Every row has typed, bounded read ranges; ancillary
    source rows may be partially read. Reported evidence cannot
    carry an execution result. A test result needs a positive observed count.
    Paths are repository-relative, keeping operator paths out of public exports.
    """
    if not isinstance(document, dict):
        return ['manifest must be an object']
    errors = []
    if document.get('schema') != 'wdbx-3d-source-manifest-v1':
        errors.append('unsupported manifest schema')
    rows = document.get('sources')
    if not isinstance(rows, list):
        return errors + ['sources must be an array']
    found = set()
    for i, row in enumerate(rows):
        prefix = f'sources[{i}]'
        if not isinstance(row, dict):
            errors.append(prefix + ' must be an object')
            continue
        authority = row.get('authority')
        if _text(authority):
            if authority in REQUIRED_AUTHORITIES:
                if authority in found:
                    errors.append(prefix + ': duplicate required authority')
                found.add(authority)
        else:
            errors.append(prefix + ': authority required')
        for field in ('repository', 'ref', 'owner', 'role'):
            if not _text(row.get(field)):
                errors.append(prefix + ': ' + field + ' required')
        if not _relative_file(row.get('path')):
            errors.append(prefix + ': path must be a normalized repository-relative file')
        if not _digest(row.get('sha256'), 64):
            errors.append(prefix + ': immutable sha256 required')
        if row.get('identity_kind', 'git') == 'external-artifact':
            if row.get('commit') is not None or not _text(row.get('provenance')):
                errors.append(prefix + ': external artifact needs honest non-git provenance')
        elif row.get('identity_kind', 'git') != 'git' or not _digest(row.get('commit'), 40):
            errors.append(prefix + ': immutable git commit required')
        if row.get('identity_kind', 'git') == 'git':
            if not _text(row.get('branch')) or type(row.get('clean')) is not bool:
                errors.append(prefix + ': git branch and boolean clean state required')
        ranges_valid = _valid_ranges(row.get('read_ranges'), row.get('line_count'))
        if not ranges_valid:
            errors.append(prefix + ': valid typed read ranges and line_count required')
        if authority == 'master-plan-v5' and row.get('read_scope') == 'first-packet':
            ranges = row.get('read_ranges')
            count = row.get('line_count')
            valid = ranges_valid and count >= 2804
            required = ((1, 624), (2511, 2610), (2645, 2725), (2766, 2804))
            if not valid or any(not any(a <= first and b >= last for a, b in ranges)
                                for first, last in required):
                errors.append(prefix + ': first-packet plan read ranges incomplete')
        elif authority == 'lockfile' and row.get('read_scope') == 'dependency-inventory':
            if not ranges_valid or not row.get('read_ranges'):
                errors.append(prefix + ': nonempty dependency inventory read ranges required')
        elif authority in REQUIRED_AUTHORITIES and not _complete_ranges(
                row.get('read_ranges'), row.get('line_count')):
            errors.append(prefix + ': complete read ranges required')
        kind = row.get('evidence_kind')
        if kind not in ('source', 'reported', 'executed'):
            errors.append(prefix + ': invalid evidence_kind')
        execution = row.get('execution')
        if kind != 'executed':
            if execution is not None:
                errors.append(prefix + ': nonexecuted source cannot carry execution evidence')
        elif not isinstance(execution, dict):
            errors.append(prefix + ': execution object required')
        else:
            if any(not _text(execution.get(f)) for f in ('command', 'artifact')):
                errors.append(prefix + ': command and artifact required')
            if not _relative_file(execution.get('artifact')):
                errors.append(prefix + ': artifact must be a normalized relative file')
            if (not _digest(execution.get('source_sha256'), 64)
                    or execution.get('source_sha256') != row.get('sha256')):
                errors.append(prefix + ': execution must bind source_sha256')
            if not _digest(execution.get('artifact_sha256'), 64):
                errors.append(prefix + ': immutable execution artifact_sha256 required')
            if type(execution.get('exit_status')) is not int:
                errors.append(prefix + ': integer exit_status required')
            if execution.get('kind') not in ('check', 'test'):
                errors.append(prefix + ': execution kind must be check or test')
            count = execution.get('test_count')
            if execution.get('kind') == 'test':
                if type(count) is not int or count <= 0:
                    errors.append(prefix + ': nonzero executed test count required')
            elif count is not None:
                errors.append(prefix + ': check is not test evidence')
    errors.extend('missing authority: ' + name for name in REQUIRED_AUTHORITIES if name not in found)
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest', type=Path)
    args = parser.parse_args()
    try:
        errors = validate_source_manifest(json.loads(args.manifest.read_text()))
    except (OSError, ValueError) as error:
        errors = [str(error)]
    for error in errors:
        print(error)
    if not errors:
        print('Source manifest inventory accepted (content/execution not independently verified)')
    return bool(errors)


if __name__ == '__main__':
    raise SystemExit(main())
