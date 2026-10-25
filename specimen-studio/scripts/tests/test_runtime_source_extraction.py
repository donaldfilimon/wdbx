"""Independent hostile archive fixtures; no downloads, SDKs or live data."""
import gzip
import importlib.util
import io
import os
import shutil
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

    def make_extended_archive(self, kind, metadata, repeats=1):
        if (self.work / self.top).exists():
            shutil.rmtree(self.work / self.top)
        # Build physical extension records explicitly; TarFile hides them on read.
        extension = tarfile.TarInfo('././@LongLink')
        extension.type = kind
        extension.size = len(metadata)
        member = tarfile.TarInfo(self.top + '/file')
        member.size = 1
        record = (extension.tobuf(format=tarfile.USTAR_FORMAT) + metadata
                  + b'\x00' * (-len(metadata) % 512))
        raw = record * repeats + member.tobuf() + b'x' + b'\x00' * 511 + b'\x00' * 1024
        self.archive.write_bytes(gzip.compress(raw, mtime=0))

    @staticmethod
    def pax_record(key, value):
        body = (' ' + key + '=' + value + '\n').encode()
        size = len(body) + 1
        while len(str(size)) + len(body) != size:
            size = len(str(size)) + len(body)
        return str(size).encode() + body

    def test_extension_payloads_are_bounded_before_the_tar_parser_reads_them(self):
        for kind in [tarfile.XHDTYPE, tarfile.XGLTYPE,
                     tarfile.GNUTYPE_LONGNAME, tarfile.GNUTYPE_LONGLINK]:
            with self.subTest(kind=kind):
                metadata = (self.pax_record('comment', 'x' * 1024 * 1024)
                            if kind in [tarfile.XHDTYPE, tarfile.XGLTYPE]
                            else b'x' * (1024 * 1024) + b'\x00')
                self.make_extended_archive(kind, metadata)
                with patch.object(builder, 'SOURCE_MAX_FILE_BYTES', 16), \
                        patch.object(builder.tarfile, 'open', wraps=tarfile.open) as parser:
                    with self.assertRaisesRegex(ValueError, 'metadata|member'):
                        self.extract()
                    parser.assert_not_called()
                self.assertEqual(list(self.work.iterdir()), [])

    def test_extension_headers_count_toward_member_and_payload_limits(self):
        for kind in [tarfile.XHDTYPE, tarfile.XGLTYPE,
                     tarfile.GNUTYPE_LONGNAME, tarfile.GNUTYPE_LONGLINK]:
            metadata = (self.pax_record('comment', 'small')
                        if kind in [tarfile.XHDTYPE, tarfile.XGLTYPE]
                        else (self.top + '/file').encode() + b'\x00')
            for bound, limit in [('SOURCE_MAX_MEMBERS', 1), ('SOURCE_MAX_BYTES', 1)]:
                with self.subTest(kind=kind, bound=bound):
                    self.make_extended_archive(kind, metadata)
                    with patch.object(builder, bound, limit):
                        with self.assertRaises(ValueError):
                            self.extract()
                    self.assertEqual(list(self.work.iterdir()), [])

    def test_compressed_trailing_stream_is_bounded_before_tar_parser(self):
        self.make_archive([(self.top + '/file', tarfile.REGTYPE, b'x')])
        with self.archive.open('ab') as output:
            output.write(gzip.compress(b'\x00' * 1024 * 1024, mtime=0))
        with patch.object(builder, 'SOURCE_MAX_TAR_BYTES', 16384, create=True), \
                patch.object(builder.tarfile, 'open', wraps=tarfile.open) as parser:
            with self.assertRaisesRegex(ValueError, 'decoded'):
                self.extract()
            parser.assert_not_called()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_small_global_and_local_pax_and_gnu_longname_are_compatible(self):
        for kind in [tarfile.XHDTYPE, tarfile.XGLTYPE, tarfile.GNUTYPE_LONGNAME]:
            with self.subTest(kind=kind):
                metadata = (self.pax_record('comment', 'pinned-source')
                            if kind in [tarfile.XHDTYPE, tarfile.XGLTYPE]
                            else (self.top + '/file').encode() + b'\x00')
                self.make_extended_archive(kind, metadata)
                extracted = self.extract()
                self.assertEqual((extracted / 'file').read_bytes(), b'x')
                shutil.rmtree(extracted)

    def test_metadata_limits_and_sparse_headers_fail_before_the_tar_parser(self):
        metadata = self.pax_record('comment', 'x' * 200)
        for bound, limit, repeats in [
                ('SOURCE_MAX_METADATA_BYTES', len(metadata) - 1, 1),
                ('SOURCE_MAX_METADATA_TOTAL', len(metadata) * 2 - 1, 2),
                ('SOURCE_MAX_METADATA_HEADERS', 2, 3)]:
            with self.subTest(bound=bound):
                self.make_extended_archive(tarfile.XGLTYPE, metadata, repeats)
                with patch.object(builder, bound, limit, create=True), \
                        patch.object(builder.tarfile, 'open', wraps=tarfile.open) as parser:
                    with self.assertRaisesRegex(ValueError, 'metadata'):
                        self.extract()
                    parser.assert_not_called()
                self.assertEqual(list(self.work.iterdir()), [])
        self.make_extended_archive(tarfile.XHDTYPE, self.pax_record('GNU.sparse.map', '0,1'))
        with patch.object(builder.tarfile, 'open', wraps=tarfile.open) as parser:
            with self.assertRaisesRegex(ValueError, 'sparse'):
                self.extract()
            parser.assert_not_called()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_metadata_and_materialized_aliases_share_the_expansion_quota(self):
        metadata_size = len(self.pax_record('comment', 'small'))
        with tarfile.open(self.archive, 'w:gz', pax_headers={'comment': 'small'}) as contents:
            target = tarfile.TarInfo(self.top + '/target')
            target.size = 5
            contents.addfile(target, io.BytesIO(b'bytes'))
            alias = tarfile.TarInfo(self.top + '/alias')
            alias.type = tarfile.SYMTYPE
            alias.linkname = 'target'
            contents.addfile(alias)
        with patch.object(builder, 'SOURCE_MAX_BYTES', metadata_size + 9):
            with self.assertRaisesRegex(ValueError, 'expanded|expansion'):
                self.extract()
        self.assertEqual(list(self.work.iterdir()), [])

    def test_exact_decoded_and_metadata_boundaries_are_accepted(self):
        metadata = self.pax_record('comment', 'bounded')
        self.make_extended_archive(tarfile.XGLTYPE, metadata)
        decoded_size = len(gzip.decompress(self.archive.read_bytes()))
        with patch.object(builder, 'SOURCE_MAX_TAR_BYTES', decoded_size, create=True), \
                patch.object(builder, 'SOURCE_MAX_METADATA_BYTES', len(metadata), create=True), \
                patch.object(builder, 'SOURCE_MAX_METADATA_TOTAL', len(metadata), create=True), \
                patch.object(builder, 'SOURCE_MAX_METADATA_HEADERS', 1, create=True):
            extracted = self.extract()
        self.assertEqual((extracted / 'file').read_bytes(), b'x')

    def test_metadata_header_is_rejected_without_reading_its_declared_payload(self):
        self.make_extended_archive(tarfile.XGLTYPE, self.pax_record('comment', 'x' * 100))
        reads = []
        original = gzip.GzipFile.read

        def record_read(source, size=-1):
            reads.append(size)
            return original(source, size)

        with patch.object(gzip.GzipFile, 'read', record_read), \
                patch.object(builder, 'SOURCE_MAX_FILE_BYTES', 16):
            with self.assertRaisesRegex(ValueError, 'member'):
                self.extract()
        self.assertEqual(reads, [512])
        self.assertEqual(list(self.work.iterdir()), [])

    def test_preflight_cancellation_closes_spool_and_preserves_existing_work(self):
        self.make_archive([(self.top + '/file', tarfile.REGTYPE, b'contents')])
        keep = self.work / 'keep'
        keep.write_bytes(b'original')
        temporary_files = []
        original_temp = tempfile.TemporaryFile
        original_read = gzip.GzipFile.read
        reads = []

        def record_temp(*args, **kwargs):
            result = original_temp(*args, **kwargs)
            temporary_files.append(result)
            return result

        def cancel_read(source, size=-1):
            reads.append(size)
            if len(reads) == 2:
                raise KeyboardInterrupt
            return original_read(source, size)

        with patch.object(builder.tempfile, 'TemporaryFile', record_temp), \
                patch.object(gzip.GzipFile, 'read', cancel_read):
            with self.assertRaises(KeyboardInterrupt):
                self.extract()
        self.assertEqual(len(temporary_files), 1)
        self.assertTrue(temporary_files[0].closed)
        self.assertEqual(list(self.work.iterdir()), [keep])
        self.assertEqual(keep.read_bytes(), b'original')

    def test_gnu_longlink_keeps_internal_file_alias_compatibility(self):
        target_name = self.top + '/' + 'long-' * 30
        link = tarfile.TarInfo(self.top + '/alias')
        link.type = tarfile.SYMTYPE
        link.linkname = target_name.split('/', 1)[1]
        target = tarfile.TarInfo(target_name)
        target.size = 5
        with tarfile.open(self.archive, 'w:gz', format=tarfile.GNU_FORMAT) as contents:
            contents.addfile(link)
            contents.addfile(target, io.BytesIO(b'bytes'))
        extracted = self.extract()
        self.assertEqual((extracted / 'alias').read_bytes(), b'bytes')
        self.assertFalse((extracted / 'alias').is_symlink())

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
