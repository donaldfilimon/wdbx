"""Mutation tests for the non-promoting execution register."""
import copy
import hashlib
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


    # Review/artifact identity and README counts (tranche 2, Task 1).

    def test_review_reference_cannot_be_added_without_catalog(self):
        path = 'docs/completion/execution/evidence/2026-10-26-p1-review/README.md'
        digest = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        def add(data):
            row = next(r for r in data['requirements'] if r['id'] == 'G01')
            row['review']['references'].append({'path': path, 'sha256': digest})
        self.assertIn('review identity drift: G01', self.errors(add))

    def test_artifact_references_cannot_be_stripped(self):
        def strip(data):
            row = next(r for r in data['requirements'] if r['id'] == 'C14')
            refs = row['artifacts']['references']
            self.assertGreater(len(refs), 1)
            row['artifacts']['references'] = [
                ref for ref in refs if ref['path'] == 'docs/completion/plans.json']
        self.assertIn('artifact identity drift: C14', self.errors(strip))

    def test_gap_rows_cannot_gain_review_refs(self):
        # The catalog is patched to agree, so only the independent gap rule can fire.
        path = 'docs/completion/execution/evidence/2026-10-26-p1-review/README.md'
        digest = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        sha, catalog = self.module.read_review_artifact_catalog(ROOT)
        catalog = copy.deepcopy(catalog)
        catalog['G01']['review'].append(path)
        data = copy.deepcopy(self.register)
        row = next(r for r in data['requirements'] if r['id'] == 'G01')
        row['review']['references'].append({'path': path, 'sha256': digest})
        with patch.object(self.module, 'read_review_artifact_catalog',
                          return_value=(sha, catalog)):
            errors = self.module.validate(ROOT, data)
        self.assertIn('gap cannot carry review: G01', errors)
        self.assertNotIn('review identity drift: G01', errors)

    def test_review_artifact_catalog_requirement_closure(self):
        sha, catalog = self.module.read_review_artifact_catalog(ROOT)
        self.assertEqual(163, len(catalog))
        for change in ('drop', 'extra'):
            with self.subTest(change=change):
                altered = copy.deepcopy(catalog)
                if change == 'drop':
                    altered.pop('G01')
                else:
                    altered['ZZ99'] = {'artifacts': [], 'review': []}
                with patch.object(self.module, 'read_review_artifact_catalog',
                                  return_value=(sha, altered)):
                    errors = self.module.validate(ROOT, self.register)
                self.assertIn('review/artifact catalog requirement closure', errors)

    def test_review_artifact_catalog_hash_is_independently_pinned(self):
        original = self.module.sha
        def changed(path):
            return '0' * 64 if path.name == 'review-artifact-paths-v1.json' else original(path)
        with patch.object(self.module, 'sha', side_effect=changed):
            errors = self.module.validate(ROOT, self.register)
        self.assertIn('review/artifact catalog drift', errors)

    def test_readme_counts_match_catalog(self):
        readme = (ROOT / 'docs/completion/execution/README.md').read_text()
        self.assertEqual([], [e for e in self.module.validate(ROOT, self.register, readme=readme)
                              if e.startswith('README count drift')])
        mutations = [
            ('**131 candidate mappings**', '**132 candidate mappings**'),
            ('**16 local gaps**', '**15 local gaps**'),
            ('**16 external gaps**', '**17 external gaps**'),
            ('**14 historical review/receipt pointers**', '**21 historical review/receipt pointers**'),
            ('**7 scoring-packet review pointers**', '**8 scoring-packet review pointers**'),
            ('21 unique review paths', '20 unique review paths'),
            ('S28, S32, S36, S37, S41.', 'S28, S32, S36, S37.'),
            ('S28, S32, S36, S37, S41.', 'S28, S32, S36, S37, S41, C14.'),
            ('P05.01\u2013P05.04', 'P05.01\u2013P05.03'),
            ('OS01\u2013OS06', 'OS01\u2013OS07'),
        ]
        for old, new in mutations:
            with self.subTest(old=old, new=new):
                self.assertIn(old, readme)
                errors = self.module.validate(ROOT, self.register, readme=readme.replace(old, new, 1))
                self.assertTrue([e for e in errors if e.startswith('README count drift')], errors)

if __name__ == '__main__':
    unittest.main()
