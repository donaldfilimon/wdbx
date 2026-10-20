"""Mutation tests: contract closure must not be self-certified by its manifest."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('check_completion', ROOT / 'tools/check_completion.py')
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class CompletionContracts(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / 'docs/completion', self.root / 'docs/completion')
        self.data = self.root / 'docs/completion'

    def mutate(self, name, change):
        path = self.data / name
        value = json.loads(path.read_text())
        change(value)
        path.write_text(json.dumps(value))

    def rejected(self):
        self.assertTrue(checker.validate(self.root, live_sources=False))

    def test_complete_contract(self):
        self.assertEqual([], checker.validate(self.root, live_sources=False))

    def test_drop_acceptance_in_both_catalog_and_manifest(self):
        self.mutate('studio-acceptance-v1.json', lambda x: x['cases'].pop())
        self.mutate('requirements.json', lambda x: x['requirements'].__setitem__(slice(None), [r for r in x['requirements'] if r['id'] != 'S46']))
        self.rejected()

    def test_drop_chapter(self):
        self.mutate('requirements.json', lambda x: x['requirements'].__setitem__(slice(None), [r for r in x['requirements'] if r['id'] != 'CH26']))
        self.rejected()

    def test_drop_task_requirement(self):
        self.mutate('requirements.json', lambda x: x['requirements'].pop(0))
        self.rejected()

    def test_changed_acceptance_text(self):
        self.mutate('studio-acceptance-v1.json', lambda x: x['cases'][0].update(row='weaker assertion'))
        self.rejected()

    def test_false_promotion(self):
        self.mutate('requirements.json', lambda x: x['requirements'][0].update(status='implemented', evidence_level='C7'))
        self.rejected()

    def test_fabricated_evidence(self):
        self.mutate('requirements.json', lambda x: x['requirements'][0].update(evidence=['green']))
        self.rejected()

    def test_missing_evidence_expectation(self):
        self.mutate('requirements.json', lambda x: x['requirements'][0].update(expected_evidence=[]))
        self.rejected()

    def test_missing_plan(self):
        self.mutate('plans.json', lambda x: x['plans'].pop())
        self.rejected()

    def test_missing_interface(self):
        self.mutate('plans.json', lambda x: x['plans'][0].update(interfaces=[]))
        self.rejected()

    def test_missing_design_decision(self):
        self.mutate('designs.json', lambda x: x['designs'][0].update(decisions=[]))
        self.rejected()

    def test_missing_design_algorithm(self):
        self.mutate('designs.json', lambda x: x['designs'][0].update(algorithm=''))
        self.rejected()

    def test_dependency_cycle(self):
        self.mutate('plans.json', lambda x: x['plans'][0].update(depends_on=[10]))
        self.rejected()

    def test_missing_completion_condition(self):
        self.mutate('requirements.json', lambda x: x['requirements'][0].update(completion_condition=''))
        self.rejected()

    def test_live_source_drift(self):
        source = self.root / checker.SOURCE_PATHS['studio']
        source.parent.mkdir(parents=True)
        source.write_text('changed')
        self.assertTrue(checker.validate(self.root))

    def test_overlapping_holdout(self):
        self.mutate('experiments.json', lambda x: x['splits']['test'].append(x['splits']['train'][0]))
        self.rejected()

    def test_changed_corpus(self):
        with (self.data / 'synthetic-corpus-v1.jsonl').open('a') as f:
            f.write('{}\n')
        self.rejected()

    def test_reduced_efficacy_threshold(self):
        self.mutate('experiments.json', lambda x: x['thresholds'].update(retrieval_relative_failure_reduction=0))
        self.rejected()

    def test_changed_source_and_rehashed_catalog(self):
        path = self.data / 'sources/studio.md'
        path.write_text(path.read_text().replace('## 26. Source coverage map', 'Removed chapter'))
        self.mutate('sources.json', lambda x: x['studio'].update(sha256=checker.digest(path.read_bytes())))
        self.rejected()

    def test_duplicate_requirement(self):
        self.mutate('requirements.json', lambda x: x['requirements'].append(copy.deepcopy(x['requirements'][0])))
        self.rejected()

    def test_malformed_json_fails_cleanly(self):
        (self.data / 'plans.json').write_text('{')
        self.rejected()


if __name__ == '__main__':
    unittest.main()
