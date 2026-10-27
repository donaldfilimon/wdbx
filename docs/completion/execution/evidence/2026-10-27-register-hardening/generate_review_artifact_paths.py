#!/usr/bin/env python3
"""Regenerate review-artifact-paths-v1.json from the live execution register.

Usage: python3 generate_review_artifact_paths.py [--root ROOT] [--out PATH]
Writes the catalog to --out (default: stdout). The catalog lists, per requirement ID
in register row order, the exact artifact and review reference paths in register
order. It records path identity only; sha256 pins stay in the register.
"""
import argparse
import json
from pathlib import Path
import sys

REGISTER = 'docs/completion/execution/register-v1.json'


def build(root):
    register = json.loads((root / REGISTER).read_text())
    catalog = {}
    for row in register['requirements']:
        catalog[row['id']] = {
            'artifacts': [ref['path'] for ref in row['artifacts']['references']],
            'review': [ref['path'] for ref in row['review']['references']],
        }
    return json.dumps(catalog, indent=2) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[5])
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    text = build(args.root)
    if args.out:
        args.out.write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == '__main__':
    sys.exit(main())
