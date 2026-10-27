"""Custody controls exercise real files and CLI; no product claims are tested."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / 'tools/check_source_custody.py'
DIRECTORY = Path('docs/source-records')
SOURCE = DIRECTORY / 'wdbx-master-v5.md'
MANIFEST = DIRECTORY / 'custody-v1.json'
PIN = 'f98d61f6f80503a2d00687c8cb6f38ed654cf9ad46cf55407ba5e06765354822'
MARKER = 'PRIVATE-CONTENT-MUST-NOT-APPEAR'


class SourceCustody(unittest.TestCase):
    def setUp(self):
        self.assertTrue(CHECKER.is_file(), 'custody verifier is not implemented')
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve() / 'repo'
        shutil.copytree(ROOT / DIRECTORY, self.root / DIRECTORY)
        self.cwd = Path(self.tmp.name).resolve() / 'elsewhere'
        self.cwd.mkdir()

    def run_checker(self, *args):
        return subprocess.run(
            [sys.executable, str(CHECKER), *map(str, args)], cwd=self.cwd,
            capture_output=True, text=True, timeout=10,
            env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'},
        )

    def rejected(self, *args):
        result = self.run_checker('--repo-root', self.root, *args)
        self.assertNotEqual(0, result.returncode)
        output = result.stdout + result.stderr
        self.assertLessEqual(len(output), 200)
        self.assertNotIn(MARKER, output)
        self.assertNotIn(str(self.root), output)
        self.assertNotIn('Traceback', output)

    def mutate(self, change):
        path = self.root / MANIFEST
        value = json.loads(path.read_text())
        change(value)
        path.write_text(json.dumps(value))

    def test_valid_unchanged_bytes_from_arbitrary_cwd(self):
        source = self.root / SOURCE
        self.assertEqual(344347, source.stat().st_size)
        self.assertEqual(PIN, hashlib.sha256(source.read_bytes()).hexdigest())
        before = {p.relative_to(self.root): (p.read_bytes(), p.stat().st_mtime_ns)
                  for p in self.root.rglob('*') if p.is_file()}
        result = self.run_checker('--repo-root', self.root)
        self.assertEqual(0, result.returncode, result.stderr)
        after = {p.relative_to(self.root): (p.read_bytes(), p.stat().st_mtime_ns)
                 for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before, after)

    def test_default_root_is_checker_location(self):
        (self.root / 'tools').mkdir()
        shutil.copyfile(CHECKER, self.root / 'tools/check_source_custody.py')
        result = subprocess.run(
            [sys.executable, str(self.root / 'tools/check_source_custody.py')],
            cwd=self.cwd, capture_output=True, text=True, timeout=10,
        )
        self.assertEqual(0, result.returncode, result.stderr)

    def test_missing_source(self):
        (self.root / SOURCE).unlink()
        self.rejected()

    def test_truncated_source(self):
        p = self.root / SOURCE
        p.write_bytes(p.read_bytes()[:-1])
        self.rejected()

    def test_modified_same_length_source(self):
        p = self.root / SOURCE
        data = p.read_bytes()
        p.write_bytes(bytes([data[0] ^ 1]) + data[1:])
        self.rejected()

    def test_coordinated_source_and_manifest_hash_edit(self):
        data = b'x' * 344347
        (self.root / SOURCE).write_bytes(data)
        self.mutate(lambda v: v['source'].update(sha256=hashlib.sha256(data).hexdigest()))
        self.rejected()

    def test_edited_hash_size_and_identity(self):
        for field, value in [('sha256', '0' * 64), ('byte_length', 1),
                             ('byte_length', True), ('byte_length', 344347.0),
                             ('id', MARKER),
                             ('title', MARKER)]:
            with self.subTest(field=field, value=value):
                original = (self.root / MANIFEST).read_bytes()
                self.mutate(lambda v: v['source'].update({field: value}))
                self.rejected()
                (self.root / MANIFEST).write_bytes(original)

    def test_path_escape_and_noncanonical_paths(self):
        for path in ['../' + MARKER, '/' + MARKER, 'docs/../' + MARKER,
                     'docs\\' + MARKER, 'docs//source-records/wdbx-master-v5.md',
                     './docs/source-records/wdbx-master-v5.md', '', 7]:
            with self.subTest(path=path):
                self.mutate(lambda v: v['source'].update(path=path))
                self.rejected()

    def test_symlinked_source_even_within_repository(self):
        p = self.root / SOURCE
        target = self.root / 'original.md'
        p.rename(target)
        p.symlink_to(target)
        self.rejected()

    def test_symlinked_manifest(self):
        p = self.root / MANIFEST
        target = self.root / 'original.json'
        p.rename(target)
        p.symlink_to(target)
        self.rejected()

    def test_symlinked_directory(self):
        p = self.root / DIRECTORY
        target = self.root / 'original'
        p.rename(target)
        p.symlink_to(target, target_is_directory=True)
        self.rejected()

    def test_symlinked_root_and_ancestor(self):
        alias = Path(self.tmp.name) / 'alias'
        alias.symlink_to(self.root.parent, target_is_directory=True)
        root_alias = Path(self.tmp.name) / 'root-alias'
        root_alias.symlink_to(self.root, target_is_directory=True)
        for path in [root_alias, alias / 'repo', alias / 'repo' / 'docs' / '..']:
            with self.subTest(path=path):
                self.assertNotEqual(0, self.run_checker('--repo-root', path).returncode)

    def test_nonregular_source_directory(self):
        p = self.root / SOURCE
        p.unlink()
        p.mkdir()
        self.rejected()

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'requires POSIX FIFO')
    def test_nonregular_source_fifo_does_not_block(self):
        p = self.root / SOURCE
        p.unlink()
        os.mkfifo(p)
        self.rejected()

    def test_missing_manifest(self):
        (self.root / MANIFEST).unlink()
        self.rejected()

    def test_malformed_oversized_and_duplicate_json(self):
        for data in [MARKER, '{"source":{},"source":{}}', '[' * 1500,
                     ' ' * 16385, 'null', '[]']:
            with self.subTest(length=len(data)):
                (self.root / MANIFEST).write_text(data)
                self.rejected()

    def test_missing_unknown_and_changed_origin_fields(self):
        original = (self.root / MANIFEST).read_bytes()
        for change in [lambda v: v.pop('source'),
                       lambda v: v.update(extra=MARKER),
                       lambda v: v['origin'].update(repository=MARKER),
                       lambda v: v.update(schema=MARKER)]:
            self.mutate(change)
            self.rejected()
            (self.root / MANIFEST).write_bytes(original)

    def test_oversized_source(self):
        (self.root / SOURCE).write_bytes(b'x' * 344348)
        self.rejected()

    def test_argument_errors_are_bounded_and_content_free(self):
        for args in [('--' + MARKER * 100,), ('--repo-root',),
                     (MARKER * 100,), ('--repo-root', MARKER + '\n')]:
            with self.subTest(args=args[:1]):
                result = self.run_checker(*args)
                self.assertNotEqual(0, result.returncode)
                output = result.stdout + result.stderr
                self.assertLessEqual(len(output), 200)
                self.assertNotIn(MARKER, output)
                self.assertNotIn('Traceback', output)


if __name__ == '__main__':
    unittest.main()
