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
VECTOR_SPEC = importlib.util.spec_from_file_location('contract_vectors', ROOT / 'docs/completion/contract_vectors.py')
vectors = importlib.util.module_from_spec(VECTOR_SPEC)
VECTOR_SPEC.loader.exec_module(vectors)
LEGACY_SPEC = importlib.util.spec_from_file_location('contract_vectors_v1', ROOT / 'docs/completion/contract_vectors_v1.py')
legacy_vectors = importlib.util.module_from_spec(LEGACY_SPEC)
LEGACY_SPEC.loader.exec_module(legacy_vectors)


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

    def test_task_number_retargeted_with_matching_requirements(self):
        def retarget(x):
            x['plans'][0]['task'] = 2
            x['plans'][0]['requirement_ids'] = x['plans'][1]['requirement_ids'][:]
        self.mutate('plans.json', retarget)
        self.rejected()

    def test_required_dependency_removed(self):
        self.mutate('plans.json', lambda x: x['plans'][9].update(depends_on=[]))
        self.rejected()

    def test_requirement_ownership_changed(self):
        self.mutate('requirements.json', lambda x: x['requirements'][0].update(owner='unrelated owner'))
        self.rejected()

    def test_duplicate_requirement_routing(self):
        self.mutate('plans.json', lambda x: x['plans'][0]['requirement_ids'].append(x['plans'][0]['requirement_ids'][0]))
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
        with (self.data / 'synthetic-corpus-v2.jsonl').open('a') as f:
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

    def metric_inputs(self):
        protocol = json.loads((self.data / 'experiments.json').read_text())
        records = [json.loads(s) for s in (self.data / protocol['corpus']).read_text().splitlines()]
        return protocol, records

    def test_metric_without_tone_text_rejected_without_relying_on_hash(self):
        protocol, records = self.metric_inputs()
        for row in records:
            if row['suite'] == 'atp':
                del row['input']['source_text']
        self.assertTrue(checker.metric_errors(protocol, records))

    def test_metric_without_mutation_text_rejected_without_relying_on_hash(self):
        protocol, records = self.metric_inputs()
        for row in records:
            if row['suite'] == 'mutation':
                del row['input']['recipient_slots']
        self.assertTrue(checker.metric_errors(protocol, records))

    def test_relabelled_retrieval_copies_rejected(self):
        protocol, records = self.metric_inputs()
        retrieval = [r for r in records if r['suite'] == 'retrieval']
        retrieval[-1]['input'] = copy.deepcopy(retrieval[0]['input'])
        self.assertTrue(checker.metric_errors(protocol, records))

    def test_missing_query_embedding_rejected(self):
        protocol, records = self.metric_inputs()
        del next(r for r in records if r['suite'] == 'retrieval')['input']['query']['vector']
        self.assertTrue(checker.metric_errors(protocol, records))

    def test_threshold_without_metric_mapping_rejected(self):
        protocol, records = self.metric_inputs()
        del protocol['metric_contracts']['tone_semantic_preservation']
        self.assertTrue(checker.metric_errors(protocol, records))

    def test_network_and_worked_vector_reproduce(self):
        network = json.loads((self.data / 'synthesis-network-v1.json').read_text())
        worked = json.loads((self.data / 'synthesis-worked-v1.json').read_text())
        self.assertEqual(network, vectors.network())
        actual = legacy_vectors.worked_synthesis()
        self.assertEqual(worked['encoded'], actual['encoded'])
        self.assertEqual(worked['decoded_ast'], actual['decoded_ast'])
        for expected, output in zip(worked['network_output'], actual['network_output']):
            self.assertAlmostEqual(expected, output, delta=2e-6)
        self.assertEqual(worked['output_text'], '5')

    def test_alignment_exact_score_and_ref_ties(self):
        self.assertEqual(vectors.slot_similarity('a b c', 'a b c'), 100)
        self.assertEqual(vectors.slot_similarity('a b c', 'x y z'), 0)
        r = [{'ref': 'r1', 'text': 'a b c'}, {'ref': 'r0', 'text': 'a b c'}]
        d = [{'ref': 'd1', 'text': 'a b c'}, {'ref': 'd0', 'text': 'a b c'}]
        self.assertEqual(vectors.small_alignment(r, d), ['d0', 'd1'])
        self.assertEqual(vectors.small_alignment(r[::-1], d[::-1]), ['d0', 'd1'])

    def test_inhibition_erasure_rotation_equivalence_and_scope(self):
        frozen = json.loads((self.data / 'inhibition-vectors-v2.json').read_text())
        self.assertEqual(frozen, vectors.inhibition_vectors())
        a = vectors.synthetic_pair('two plus two', 'four')
        b = vectors.synthetic_pair('2 + 2', '4')
        self.assertEqual(a, b)
        # Matching API takes inhibition keys only; erased object key is absent.
        stored = frozen['epoch_0_tag']
        epochs = [frozen['key_epoch_0_hex'], frozen['key_epoch_1_hex']]
        self.assertIn(stored, [vectors.commitment(k, 'tenant-A', 'specimen-1', b) for k in epochs])
        self.assertNotEqual(stored, vectors.commitment(epochs[0], 'tenant-B', 'specimen-1', b))
        self.assertNotEqual(stored, vectors.commitment(epochs[0], 'tenant-A', 'specimen-2', b))
        self.assertNotEqual(stored, frozen['non_equivalent_tag'])

    def test_score_bucket_total_order_near_ties(self):
        items = [{'name': 'a', 'score': 0.0}, {'name': 'b', 'score': 3e-6},
                 {'name': 'c', 'score': 6e-6}]
        from itertools import permutations
        for sequence in permutations(items):
            self.assertEqual([x['name'] for x in vectors.rank_scores(sequence)], ['c', 'a', 'b'])
        frozen = json.loads((self.data / 'synthesis-near-ties-v2.json').read_text())
        self.assertEqual(frozen, vectors.near_tie_vector())

    def test_corpus_composition_executes_decoder_without_oracle(self):
        _, rows = self.metric_inputs()
        source = next(r for r in rows if r['suite'] == 'composition')
        graph = vectors.composition_graph(source['input'])
        result = vectors.decode_composition(graph, [0.0] * 32)
        self.assertEqual(result['fields'], {'owner': source['input']['facts'][0][0],
                                          'object': source['input']['facts'][0][2],
                                          'count': source['input']['facts'][1][2] + source['input']['facts'][2][2]})
        self.assertEqual(result['source_refs'], ['input/facts/0', 'input/facts/1', 'input/facts/2'])
        missing = copy.deepcopy(source['input'])
        missing['facts'].pop()
        self.assertEqual(vectors.decode_composition(vectors.composition_graph(missing), [0.0]*32)['status'], 'abstain')
        changed = copy.deepcopy(source['input'])
        changed['facts'][2][2] += 7
        self.assertEqual(vectors.decode_composition(vectors.composition_graph(changed), [0.0]*32)['fields']['count'], result['fields']['count'] + 7)
        with self.assertRaises(ValueError):
            vectors.composition_graph(source)
        frozen = json.loads((self.data / 'synthesis-worked-v2.json').read_text())
        self.assertEqual(frozen, vectors.worked_synthesis(source['input']))

    def test_pair_inhibition_ignores_changed_and_empty_policy_codes(self):
        self.assertEqual(vectors.pair_bytes('p', 'a', ['deny:network']), vectors.pair_bytes('p', 'a', []))
        self.assertEqual(vectors.pair_bytes('p', 'a', ['deny:network']), vectors.pair_bytes('p', 'a', ['deny:write']))
        data = vectors.inhibition_vectors()
        for variant in data['policy_variations']:
            self.assertEqual(variant['epoch_0_tag'], data['epoch_0_tag'])
            self.assertEqual(variant['epoch_1_tag'], data['epoch_1_tag'])
            candidate_tags = [variant['epoch_0_tag'], variant['epoch_1_tag']]
            self.assertIn(data['after_payload_key_destruction']['tag'], candidate_tags)


if __name__ == '__main__':
    unittest.main()
