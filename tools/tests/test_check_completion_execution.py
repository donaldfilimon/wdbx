"""Mutation tests for the non-promoting execution register."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('execution', ROOT / 'tools/check_completion_execution.py')


class ExecutionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(cls.module)
        cls.register = cls.module.read_register(ROOT)

    def errors(self, mutate):
        data = copy.deepcopy(self.register)
        mutate(data)
        return self.module.validate(ROOT, data)

    def test_current_register(self):
        self.assertEqual([], self.module.validate(ROOT, self.register))

    def test_missing_id(self):
        self.assertTrue(self.errors(lambda d: d['requirements'].pop()))

    def test_duplicate_id(self):
        self.assertTrue(self.errors(lambda d: d['requirements'].append(d['requirements'][0])))

    def test_routing_drift(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(owner='other')))

    def test_false_promotion(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(evidence_level='C1')))

    def test_completed_status(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(status='complete')))

    def test_source_drift(self):
        self.assertTrue(self.errors(lambda d: d['contracts'][0].update(sha256='0' * 64)))

    def test_unsafe_paths(self):
        for path in ['../AGENTS.md', '/etc/passwd', 'docs/../AGENTS.md', '']:
            with self.subTest(path=path):
                self.assertTrue(self.errors(lambda d: d['contracts'][0].update(path=path)))

    def test_symlink_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'escape').symlink_to(ROOT / 'AGENTS.md')
            with self.assertRaises(ValueError):
                self.module.safe_file(root, 'escape')

    def test_in_root_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source').write_text('bytes')
            (root / 'alias').symlink_to(root / 'source')
            with self.assertRaises(ValueError):
                self.module.safe_file(root, 'alias')

    def test_cannot_clear_artifact_references(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0]['artifacts'].update(references=[])))

    def test_cannot_clear_existing_mapping(self):
        def clear(data):
            row = next(r for r in data['requirements'] if r['implementation']['references'])
            row['implementation']['references'] = []
        self.assertTrue(self.errors(clear))

    def test_malformed_references_are_refused(self):
        for refs in [['bad'], [None], 'bad', [{'path': 9, 'sha256': '0' * 64}]]:
            with self.subTest(refs=refs):
                self.assertTrue(self.errors(lambda d: d['requirements'][0]['artifacts'].update(references=refs)))

    def test_valid_plan_cannot_replace_candidate_sources(self):
        def replace(data):
            row = next(r for r in data['requirements'] if r['implementation']['references'])
            for kind in ('implementation', 'tests'):
                row[kind]['references'] = [copy.deepcopy(data['contracts'][1])]
        self.assertTrue(self.errors(replace))

    def test_catalog_hash_is_independently_pinned(self):
        original = self.module.sha
        def changed(path):
            return '0' * 64 if path.name == 'candidate-paths-v1.json' else original(path)
        with patch.object(self.module, 'sha', side_effect=changed):
            errors = self.module.validate(ROOT, self.register)
        self.assertIn('candidate path catalog drift', errors)

    def test_every_requirement_has_typed_reconciliation(self):
        for row in self.register['requirements']:
            self.assertIn(row['reconciliation']['disposition'], {'candidate', 'local_gap', 'external_gap'})
            self.assertTrue(row['reconciliation']['qualification_plan']['references'])

    def test_reconciliation_cannot_be_deleted(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].pop('reconciliation')))

    def test_untyped_gap_rejected(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0]['reconciliation'].update(disposition='unknown')))

    def test_empty_gap_reason_rejected(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0]['reconciliation'].update(reason='')))

    def test_gap_cannot_be_relabelled_candidate(self):
        def change(data):
            row = next(r for r in data['requirements'] if r['reconciliation']['disposition'] != 'candidate')
            row['reconciliation']['disposition'] = 'candidate'
        self.assertTrue(self.errors(change))

    def test_qualification_plan_cannot_be_cleared(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0]['reconciliation']['qualification_plan'].update(references=[])))

    def test_gap_requires_inspection_refs(self):
        def change(data):
            row = next(r for r in data['requirements'] if r['reconciliation']['disposition'] != 'candidate')
            row['reconciliation']['inspected']['references'] = []
        self.assertTrue(self.errors(change))

    def test_blocker_requires_array(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(blockers='pending')))

    def test_observation_drift(self):
        self.assertTrue(self.errors(lambda d: d['audits'][0]['files'][0].update(observed_sha256='0' * 64)))

    def test_audit_cannot_be_removed(self):
        self.assertTrue(self.errors(lambda d: d['audits'].pop()))

    def test_invented_gate_result(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(test_result='passed')))

    def test_missing_blocker(self):
        self.assertTrue(self.errors(lambda d: d['requirements'][0].update(blockers=[])))


if __name__ == '__main__':
    unittest.main()
