"""Independent native-package contract tests; no golden regeneration."""
import hashlib
import json
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_model_reference as ref

FIXTURES = Path(__file__).resolve().parents[2] / 'crates/abi-wdbx/tests/fixtures/model-v1'


class NativeModelContract(unittest.TestCase):
    def test_frozen_vectors(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        for row in manifest['fixtures']:
            with self.subTest(row['name']):
                raw = bytes.fromhex((FIXTURES / (row['name'] + '.hex')).read_text())
                self.assertEqual(hashlib.sha256(raw).hexdigest(), row['artifact_sha256'])
                self.assertEqual((FIXTURES / (row['name'] + '.sha256')).read_text().strip(), row['artifact_sha256'])
                if row['expected'] == 'ValidatedModelState':
                    actual = ref.validate(raw)
                    for key in ('artifact_sha256', 'package_digest', 'state_root', 'generation_id'):
                        self.assertEqual(actual[key], row[key])
                else:
                    if row['stage'] == 'semantic':
                        self.assertEqual(ref.frame(raw)['stage'], 'FramedPackage')
                    with self.assertRaises(ref.ContractError) as error:
                        ref.validate(raw)
                    self.assertEqual(error.exception.code, row['expected'])

    def test_auxiliary_pins_and_complete_inventory(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        for row in manifest['auxiliary']:
            self.assertEqual(hashlib.sha256((FIXTURES / row['name']).read_bytes()).hexdigest(), row['sha256'])
        names = {row['name'] for row in manifest['fixtures']}
        self.assertEqual({p.stem for p in FIXTURES.glob('*.hex')}, names)
        self.assertEqual({p.stem for p in FIXTURES.glob('*.sha256')}, names)

    def test_signed_zero_and_optional_bytes_are_preserved(self):
        raw = bytes.fromhex((FIXTURES / 'signed-zero.hex').read_text())
        framed = ref.frame(raw)
        offsets = json.loads((FIXTURES / 'field-table.json').read_text())['inference']
        pos = offsets['tensor.1.data']
        self.assertEqual(framed['entries'][3]['bytes'][pos:pos + 4], bytes.fromhex('00000080'))
        self.assertEqual(ref.validate(raw)['stage'], 'ValidatedModelState')
        annotation = ref.frame(bytes.fromhex((FIXTURES / 'annotation.hex').read_text()))
        self.assertEqual(annotation['entries'][99]['version'], [5, 9])
        self.assertEqual(annotation['entries'][99]['bytes'], b'opaque annotation')

    def test_framing_failure_precedes_semantics(self):
        raw = bytearray(bytes.fromhex((FIXTURES / 'missing-input.hex').read_text()))
        raw[-4] ^= 1
        with self.assertRaises(ref.ContractError) as error:
            ref.validate(bytes(raw))
        self.assertEqual(error.exception.code, 'IntegrityMismatch')

    def test_inference_is_never_resumable(self):
        model = ref.validate(bytes.fromhex((FIXTURES / 'inference.hex').read_text()))
        with self.assertRaises(ref.ContractError) as error:
            ref.check_resume(model, model['resume_context'], authorized=True)
        self.assertEqual(error.exception.code, 'NotResumable')

    def test_annotation_and_partition_identities(self):
        def load(name):
            return ref.validate(bytes.fromhex((FIXTURES / (name + '.hex')).read_text()))
        base, annotation, partition = map(load, ['inference', 'annotation', 'partition'])
        self.assertEqual(base['state_root'], annotation['state_root'])
        self.assertEqual(base['generation_id'], annotation['generation_id'])
        self.assertNotEqual(base['package_digest'], annotation['package_digest'])
        self.assertEqual(base['state_root'], partition['state_root'])
        self.assertNotEqual(base['generation_id'], partition['generation_id'])

    def test_literal_rng_vectors(self):
        vectors = json.loads((FIXTURES / 'rng-vectors.json').read_text())
        for row in vectors:
            state = ref.seed_stream(row['seed'], row['stream'])
            self.assertEqual(state, row['initial'])
            outputs = []
            for _ in row['outputs']:
                value, state = ref.xoshiro(state)
                outputs.append(value)
            self.assertEqual(outputs, row['outputs'])
            self.assertEqual(state, row['final'])

    def test_bpe_literal_ids(self):
        raw = bytes.fromhex((FIXTURES / 'bpe.hex').read_text())
        model = ref.validate(raw)
        self.assertEqual(ref.encode(b'abab', model['tokenizer']), [260, 260])
        self.assertEqual(ref.encode(b'<BOS>', model['tokenizer']), [64, 70, 83, 87, 66])
        self.assertEqual(ref.encode(b'', model['tokenizer']), [])

    def test_corrupt_expected_digest_is_detected(self):
        raw = bytes.fromhex((FIXTURES / 'inference.hex').read_text())
        actual = ref.validate(raw)
        with self.assertRaises(AssertionError):
            self.assertEqual(actual['artifact_sha256'], '00' * 32)

    def test_context_is_external_and_resume_is_exact(self):
        raw = bytes.fromhex((FIXTURES / 'resumable.hex').read_text())
        model = ref.validate(raw)
        ctx = dict(model['resume_context'])
        with self.assertRaises(ref.ContractError) as error:
            ref.check_resume(model, ctx, authorized=False)
        self.assertEqual(error.exception.code, 'ResumeMismatch')
        self.assertEqual(ref.check_resume(model, ctx, authorized=True), 'ResumableState')
        ctx['preparation_digest'] = '00' * 32
        with self.assertRaises(ref.ContractError) as error:
            ref.check_resume(model, ctx, authorized=True)
        self.assertEqual(error.exception.code, 'ResumeMismatch')


if __name__ == '__main__':
    unittest.main()
