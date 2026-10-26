#!/usr/bin/env python3
"""Read-only verification of the recovered WDBX v5 source, Python 3.9+."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys

SOURCE_SHA256 = 'f98d61f6f80503a2d00687c8cb6f38ed654cf9ad46cf55407ba5e06765354822'
SOURCE_BYTES = 344347
SOURCE_PATH = 'docs/source-records/wdbx-master-v5.md'
MANIFEST_PATH = 'docs/source-records/custody-v1.json'
EXPECTED_MANIFEST = {
    'schema': 'wdbx.source-custody.v1',
    'recorded_on': '2026-10-25',
    'scope': 'source-custody-only',
    'source': {
        'id': 'AUDITED-SOURCE-02',
        'title': 'WDBX-3D-Neural-Specimen-Master-Plan-v5.md',
        'path': SOURCE_PATH,
        'sha256': SOURCE_SHA256,
        'byte_length': SOURCE_BYTES,
    },
    'origin': {
        'repository': 'LYLX',
        'manifest': 'docs/requirements/historical-sources/manifest.json',
        'object': 'docs/requirements/historical-sources/objects/' + SOURCE_SHA256,
        'recovery': 'read-only Time Machine snapshot 2026-10-20-181425',
        'receipt': 'artifacts/historical-source-recovery-20261022-001/missing-search/time-machine/findings.json',
        'recovered_artifact': 'artifacts/historical-source-recovery-20261022-001/missing-search/time-machine/recovered/AUDITED-SOURCE-02/WDBX-3D-Neural-Specimen-Master-Plan-v5.md',
    },
}


def read_regular(root, relative, limit):
    """Bound reads and reject symlinks throughout the path, including ancestors.

    Directory descriptors and O_NOFOLLOW prevent component replacement from
    redirecting a read. Nonblocking open also refuses FIFOs without hanging.
    """
    if not isinstance(relative, str) or '\\' in relative:
        raise ValueError()
    parts = PurePosixPath(relative).parts
    if (not parts or PurePosixPath(relative).is_absolute()
            or '/'.join(parts) != relative or any(p in ('.', '..') for p in parts)):
        raise ValueError()
    root = Path(root).absolute()
    if any(p in ('.', '..') for p in root.parts):
        raise ValueError()
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_DIRECTORY
    directory = os.open(root.anchor, flags)
    try:
        for part in (*root.parts[1:], *parts[:-1]):
            child = os.open(part, flags, dir_fd=directory)
            os.close(directory)
            directory = child
        descriptor = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                             dir_fd=directory)
        with os.fdopen(descriptor, 'rb') as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
                raise ValueError()
            data = stream.read(limit + 1)
        if len(data) > limit:
            raise ValueError()
        return data
    finally:
        os.close(directory)


def unique_object(pairs):
    """Refuse duplicate keys rather than silently choose a competing record."""
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError()
        value[key] = item
    return value


def verify(root):
    """Check the closed manifest and bytes against independent fixed identities."""
    manifest = json.loads(read_regular(root, MANIFEST_PATH, 16384),
                          object_pairs_hook=unique_object)
    if manifest != EXPECTED_MANIFEST:
        raise ValueError()
    # JSON integer identity is stricter than Python's int/float equality.
    if type(manifest['source']['byte_length']) is not int:
        raise ValueError()
    data = read_regular(root, manifest['source']['path'], SOURCE_BYTES)
    if len(data) != SOURCE_BYTES or hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise ValueError()


class ContentFreeParser(argparse.ArgumentParser):
    """Argument failures must never echo argv, paths, or argparse details."""
    def error(self, message):
        self.exit(2, 'source custody: invalid arguments\n')


def main():
    parser = ContentFreeParser(prog='check_source_custody.py', description=__doc__,
                               allow_abbrev=False)
    parser.add_argument('--repo-root', type=Path,
                        default=Path(__file__).absolute().parents[1])
    args = parser.parse_args()
    try:
        verify(args.repo_root)
    except (OSError, ValueError, TypeError, RecursionError):
        print('source custody: verification failed', file=sys.stderr)
        return 1
    print('source custody: verified AUDITED-SOURCE-02 (custody only)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
