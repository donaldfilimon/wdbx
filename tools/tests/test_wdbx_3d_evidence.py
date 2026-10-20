"""Evidence admission must reject unsupported qualification claims."""
import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    'evidence', Path(__file__).parents[1] / 'validate_wdbx_3d_evidence.py')
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class SourceManifestTests(unittest.TestCase):
    def fixture(self):
        return {
            'schema': 'wdbx-3d-source-manifest-v1',
            'sources': [
                {'authority': name, 'repository': 'fixture', 'ref': 'main',
                 'branch': 'main', 'clean': False, 'commit': 'a' * 40, 'path': name + '.md', 'sha256': 'b' * 64,
                 'read_ranges': [[1, 10]], 'line_count': 10,
                 'owner': 'fixture maintainer', 'role': 'controlling',
                 'evidence_kind': 'source', 'execution': None}
                for name in module.REQUIRED_AUTHORITIES
            ],
        }

    def test_accepts_complete_pinned_inventory(self):
        self.assertEqual(module.validate_source_manifest(self.fixture()), [])

    def test_rejects_mutable_only_source(self):
        doc = self.fixture()
        doc['sources'][0].pop('commit')
        doc['sources'][0].pop('sha256')
        self.assertTrue(module.validate_source_manifest(doc))

    def test_distinguishes_reported_and_executed(self):
        doc = self.fixture()
        row = doc['sources'][0]
        row['evidence_kind'] = 'reported'
        self.assertEqual(module.validate_source_manifest(doc), [])
        row['execution'] = {'command': 'cargo check', 'exit_status': 0,
                            'test_count': 4, 'artifact': 'check.txt', 'kind': 'test'}
        self.assertTrue(module.validate_source_manifest(doc))

    def test_rejects_missing_original_authority(self):
        doc = self.fixture()
        doc['sources'] = [s for s in doc['sources'] if s['authority'] != 'original-specimen']
        self.assertTrue(module.validate_source_manifest(doc))

    def test_rejects_incomplete_read_ranges(self):
        doc = self.fixture()
        doc['sources'][0]['read_ranges'] = [[1, 3], [5, 10]]
        self.assertTrue(module.validate_source_manifest(doc))

    def test_zero_tests_do_not_qualify(self):
        doc = self.fixture()
        row = doc['sources'][0]
        row['evidence_kind'] = 'executed'
        row['execution'] = {'command': 'cargo test', 'exit_status': 0,
                            'test_count': 0, 'artifact': 'test.txt', 'kind': 'test',
                            'source_sha256': row['sha256'], 'artifact_sha256': 'c' * 64}
        self.assertTrue(module.validate_source_manifest(doc))
        row['execution']['test_count'] = 4
        self.assertEqual(module.validate_source_manifest(doc), [])

    def test_external_artifacts_do_not_invent_git_history(self):
        doc = self.fixture()
        row = doc['sources'][0]
        row.pop('branch')
        row.pop('clean')
        row.update(identity_kind='external-artifact', commit=None,
                   provenance='user-supplied artifact; no git provenance supplied')
        self.assertEqual(module.validate_source_manifest(doc), [])
        row.pop('provenance')
        self.assertTrue(module.validate_source_manifest(doc))

    def test_plan_can_qualify_only_the_first_packet_read_scope(self):
        doc = self.fixture()
        row = next(s for s in doc['sources'] if s['authority'] == 'master-plan-v5')
        row.update(line_count=3234, read_scope='first-packet',
                   read_ranges=[[1, 624], [2511, 2610], [2645, 2725], [2766, 2804]])
        self.assertEqual(module.validate_source_manifest(doc), [])
        row['read_ranges'][0] = [1, 600]
        self.assertTrue(module.validate_source_manifest(doc))

    def executed_fixture(self):
        doc = self.fixture()
        row = doc['sources'][0]
        row.update(evidence_kind='executed', execution={
            'command': 'cargo test', 'kind': 'test', 'test_count': 3,
            'exit_status': 1, 'artifact': 'evidence/test.txt',
            'source_sha256': row['sha256'], 'artifact_sha256': 'c' * 64})
        return doc

    def test_git_branch_and_clean_state_are_required(self):
        for field in ('branch', 'clean'):
            doc = self.fixture()
            doc['sources'][0].pop(field)
            self.assertTrue(module.validate_source_manifest(doc), field)
        for value in (None, 0, 'false'):
            doc = self.fixture()
            doc['sources'][0]['clean'] = value
            self.assertTrue(module.validate_source_manifest(doc), repr(value))
        doc = self.fixture()
        doc['sources'][0].update(branch='detached', clean=False)
        self.assertEqual(module.validate_source_manifest(doc), [])

    def test_required_inventory_categories_cannot_be_omitted(self):
        for authority in ('command-contracts', 'gateway-schema', 'toolchain',
                          'lockfile', 'current-tests'):
            doc = self.fixture()
            doc['sources'] = [r for r in doc['sources'] if r['authority'] != authority]
            self.assertTrue(module.validate_source_manifest(doc), authority)

    def test_partial_ancillary_ranges_must_still_be_well_typed(self):
        doc = self.fixture()
        row = dict(doc['sources'][0], authority='ancillary', read_ranges=[[2, 3], [6, 8]])
        doc['sources'].append(row)
        self.assertEqual(module.validate_source_manifest(doc), [])
        for ranges in (None, '2-3', [[True, 3]], [[0, 3]], [[2, 11]],
                       [[6, 8], [2, 3]], [[2, 4], [4, 6]]):
            row['read_ranges'] = ranges
            self.assertTrue(module.validate_source_manifest(doc), repr(ranges))
        row['read_ranges'] = [[2, 3]]
        row['line_count'] = '10'
        self.assertTrue(module.validate_source_manifest(doc))

    def test_file_and_execution_paths_are_normalized_relative_files(self):
        for path in ('.', '..', '/private/data', 'a/../b', 'a/./b',
                     'a//b', 'a/', 'a\\b', 'C:/data', 'https://site/a', 'a\x00b'):
            for field in ('path', 'artifact'):
                doc = self.executed_fixture()
                target = doc['sources'][0] if field == 'path' else doc['sources'][0]['execution']
                target[field] = path
                self.assertTrue(module.validate_source_manifest(doc), (field, repr(path)))

    def test_execution_requires_source_binding_and_immutable_artifact(self):
        for field in ('source_sha256', 'artifact_sha256'):
            doc = self.executed_fixture()
            doc['sources'][0]['execution'].pop(field)
            self.assertTrue(module.validate_source_manifest(doc), field)
        doc = self.executed_fixture()
        doc['sources'][0]['execution']['source_sha256'] = 'd' * 64
        self.assertTrue(module.validate_source_manifest(doc))
        doc['sources'][0]['execution']['source_sha256'] = doc['sources'][0]['sha256']
        doc['sources'][0]['execution']['artifact_sha256'] = 'invalid'
        self.assertTrue(module.validate_source_manifest(doc))

    def test_failed_execution_is_recordable_and_check_is_not_test_evidence(self):
        doc = self.executed_fixture()
        self.assertEqual(module.validate_source_manifest(doc), [])
        execution = doc['sources'][0]['execution']
        execution.update(command='cargo check', kind='check', test_count=None)
        self.assertEqual(module.validate_source_manifest(doc), [])
        execution['test_count'] = 3
        self.assertTrue(module.validate_source_manifest(doc))

    def test_lockfile_dependency_inventory_is_honestly_partial(self):
        doc = self.fixture()
        row = next(r for r in doc['sources'] if r['authority'] == 'lockfile')
        row.update(read_scope='dependency-inventory', line_count=100,
                   read_ranges=[[1, 10], [50, 60]])
        self.assertEqual(module.validate_source_manifest(doc), [])
        for ranges in ([], None, [[1, 101]], [[0, 10]]):
            row['read_ranges'] = ranges
            self.assertTrue(module.validate_source_manifest(doc), repr(ranges))

    def test_rejects_malformed_types_without_crashing(self):
        for doc in [None, [], {}, {'sources': [None]}, {'sources': 'invalid'}]:
            with self.subTest(doc=doc):
                self.assertTrue(module.validate_source_manifest(doc))
        for field, value in [('read_ranges', [[True, 10]]), ('line_count', -1),
                             ('sha256', 'not-a-digest'), ('path', '/private/data')]:
            doc = copy.deepcopy(self.fixture())
            doc['sources'][0][field] = value
            self.assertTrue(module.validate_source_manifest(doc))


if __name__ == '__main__':
    unittest.main()
