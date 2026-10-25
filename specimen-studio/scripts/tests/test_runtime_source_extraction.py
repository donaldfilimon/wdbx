"""Independent hostile archive fixtures; no downloads, SDKs or live data."""
import importlib.util
import io
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

HELPER = Path(__file__).resolve().parents[1] / 'build-runtimes.py'
spec = importlib.util.spec_from_file_location('runtime_archive_builder', HELPER)
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class RuntimeArchiveExtractionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.work = self.root / 'work'
        self.work.mkdir()
        self.archive = self.root / 'source.tar.gz'
        self.top = 'source-' + 'a' * 40

    def make_archive(self, entries):
        with tarfile.open(self.archive, 'w:gz') as contents:
            for name, kind, payload in entries:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.mode = 0o6755
                if kind == tarfile.REGTYPE:
                    member.size = len(payload)
                    contents.addfile(member, io.BytesIO(payload))
                else:
                    member.linkname = payload
                    contents.addfile(member)

    def extract(self):
        return builder.extract_source_archive(self.archive, self.work, self.top)

    def test_regular_archive_preserves_bytes_and_strips_privileged_modes(self):
        self.make_archive([(self.top + '/CMakeLists.txt', tarfile.REGTYPE, b'source\n'),
                           (self.top + '/nested/tool', tarfile.REGTYPE, b'#!/bin/sh\n')])
        extracted = self.extract()
        self.assertEqual(extracted, self.work / self.top)
        self.assertEqual((extracted / 'nested/tool').read_bytes(), b'#!/bin/sh\n')
        self.assertEqual((extracted / 'nested/tool').stat().st_mode & 0o7777, 0o755)

    def test_implicit_directories_never_keep_permissive_umask_modes(self):
        self.make_archive([(self.top + '/nested/deep/tool', tarfile.REGTYPE, b'bytes')])
        previous = os.umask(0)
        try:
            extracted = self.extract()
        finally:
            os.umask(previous)
        for directory in [extracted, extracted / 'nested', extracted / 'nested/deep']:
            self.assertEqual(directory.stat().st_mode & 0o7777, 0o755)

    def test_copy_cancellation_discards_only_its_partial_staging_tree(self):
        self.make_archive([(self.top + '/file', tarfile.REGTYPE, b'contents')])
        with patch.object(builder.shutil, 'copyfileobj', side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.extract()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_traversal_absolute_drive_and_wrong_root_fail_before_any_write(self):
        bad_names = ['../escape', '/absolute', self.top + '/../escape',
                     self.top + '/nested/../../escape', self.top + '/C:/escape',
                     self.top + '/nested\\escape', self.top + '/CON.txt',
                     self.top + '/COM1', self.top + '/CONIN$', 'another-root/file']
        for name in bad_names:
            with self.subTest(name=name):
                self.make_archive([(self.top + '/valid', tarfile.REGTYPE, b'valid'),
                                   (name, tarfile.REGTYPE, b'hostile')])
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertEqual(list(self.work.iterdir()), [])
                self.assertFalse((self.root / 'escape').exists())

    def test_internal_regular_file_links_are_materialized_portably(self):
        self.make_archive([(self.top + '/target', tarfile.REGTYPE, b'original bytes'),
                           (self.top + '/alias', tarfile.SYMTYPE, 'target'),
                           (self.top + '/hard', tarfile.LNKTYPE, self.top + '/target')])
        extracted = self.extract()
        for name in ['alias', 'hard']:
            self.assertFalse((extracted / name).is_symlink())
            self.assertEqual((extracted / name).read_bytes(), b'original bytes')

    def test_forward_file_links_are_independent_of_archive_order(self):
        self.make_archive([(self.top + '/alias', tarfile.SYMTYPE, 'target'),
                           (self.top + '/hard', tarfile.LNKTYPE, self.top + '/target'),
                           (self.top + '/target', tarfile.REGTYPE, b'forward bytes')])
        extracted = self.extract()
        self.assertEqual((extracted / 'alias').read_bytes(), b'forward bytes')
        self.assertEqual((extracted / 'hard').read_bytes(), b'forward bytes')

    def test_links_cannot_hide_expansion_or_become_file_ancestors(self):
        self.make_archive([(self.top + '/target', tarfile.REGTYPE, b'12345'),
                           (self.top + '/alias', tarfile.SYMTYPE, 'target')])
        with patch.object(builder, 'SOURCE_MAX_BYTES', 9), self.assertRaises(ValueError):
            self.extract()
        self.assertEqual(list(self.work.iterdir()), [])
        self.make_archive([(self.top + '/alias/child', tarfile.REGTYPE, b'child'),
                           (self.top + '/alias', tarfile.SYMTYPE, 'target'),
                           (self.top + '/target', tarfile.REGTYPE, b'target')])
        with self.assertRaises(ValueError):
            self.extract()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_links_devices_and_fifo_are_rejected_before_regular_file_writes(self):
        for kind in [tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.CHRTYPE, tarfile.BLKTYPE, tarfile.FIFOTYPE, tarfile.CONTTYPE]:
            with self.subTest(kind=kind):
                self.make_archive([(self.top + '/valid', tarfile.REGTYPE, b'valid'),
                                   (self.top + '/link', kind, '../../escape')])
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertEqual(list(self.work.iterdir()), [])

    def test_duplicate_and_file_ancestor_paths_are_rejected(self):
        for second in [self.top + '/file', self.top + '/file/child']:
            with self.subTest(second=second):
                self.make_archive([(self.top + '/file', tarfile.REGTYPE, b'first'),
                                   (second, tarfile.REGTYPE, b'second')])
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertEqual(list(self.work.iterdir()), [])

    def test_parent_file_collision_is_rejected_when_child_comes_first(self):
        self.make_archive([(self.top + '/file/child', tarfile.REGTYPE, b'child'),
                           (self.top + '/file', tarfile.REGTYPE, b'parent')])
        with self.assertRaises(ValueError):
            self.extract()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_existing_tree_and_symlink_work_directory_are_preserved(self):
        self.make_archive([(self.top + '/file', tarfile.REGTYPE, b'new')])
        existing = self.work / self.top
        existing.mkdir()
        (existing / 'keep').write_bytes(b'original')
        with self.assertRaises(ValueError):
            self.extract()
        self.assertEqual((existing / 'keep').read_bytes(), b'original')
        real = self.root / 'real'
        real.mkdir()
        symlink = self.root / 'symlink'
        symlink.symlink_to(real, target_is_directory=True)
        with self.assertRaises(ValueError):
            builder.extract_source_archive(self.archive, symlink, self.top)
        self.assertEqual(list(real.iterdir()), [])

    def test_member_expansion_and_compressed_input_bounds_fail_closed(self):
        self.make_archive([(self.top + '/one', tarfile.REGTYPE, b'12345'),
                           (self.top + '/two', tarfile.REGTYPE, b'67890')])
        for bound, limit in [('SOURCE_MAX_MEMBERS', 1), ('SOURCE_MAX_BYTES', 9),
                             ('SOURCE_MAX_FILE_BYTES', 4), ('SOURCE_MAX_ARCHIVE_BYTES', 1)]:
            with self.subTest(bound=bound), patch.object(builder, bound, limit):
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertEqual(list(self.work.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
