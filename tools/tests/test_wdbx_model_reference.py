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



class BpeTrainerReferenceTests(unittest.TestCase):
    def test_independent_trainer_has_literal_ties_and_overlapping_counts(self):
        result = ref.train_bpe(['aaaa', 'aaaa'], bytes([7]) * 32, 262)
        self.assertEqual(result['merges'], [(101, 101, 260), (260, 260, 261)])
        self.assertEqual(ref.encode(b'aaa', result), [260, 101])

    def test_n02_training_vectors_are_independently_pinned(self):
        raw = (FIXTURES / 'bpe-trainer-vectors-v1.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), '04eadb1cd0cc65d43afb764dbe6a622646fb142dd1b93bfc516a0456d7406fbd')
        for row in json.loads(raw)['vectors']:
            t = ref.train_bpe(row['documents'], bytes.fromhex(row['split']), row['requested'])
            self.assertEqual(t['corpus'].hex(), row['corpus'])
            self.assertEqual([list(m) for m in t['merges']], row['merges'])
            self.assertEqual(t['visits'], row['visits'])
            for sample in row['samples']:
                self.assertEqual(ref.encode(sample['text'].encode(), t), sample['tokens'])

    def test_unknown_tensor_identity_is_semantic_membership_failure(self):
        raw = bytearray(bytes.fromhex((FIXTURES / 'inference.hex').read_text()))
        entry = 96 + 2 * 80
        offset, length = struct.unpack_from('<QQ', raw, entry + 16)
        struct.pack_into('<H', raw, offset + 4, 17)
        raw[entry + 32:entry + 64] = hashlib.sha256(raw[offset:offset + length]).digest()
        count = struct.unpack_from('<I', raw, 24)[0]
        raw[32:64] = hashlib.sha256(raw[96:96 + 80 * count]).digest()
        self.assertEqual(ref.frame(bytes(raw))['stage'], 'FramedPackage')
        with self.assertRaises(ref.ContractError) as error:
            ref.validate(bytes(raw))
        self.assertEqual(error.exception.code, 'InvalidTensor')

    def test_tokenizer_profile_bytes_have_tokenizer_family(self):
        path = FIXTURES / 'tokenizer-profile-negatives-v1.json'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), 'd6feb7ebeb2a8446008f1bcf7dbbc8ed65345dec21acd614d886c580049e8a4a')
        doc = json.loads(path.read_bytes())
        self.assertEqual(doc['format'], 'native-model-tokenizer-profile-negatives-v1')
        self.assertEqual({row['field'] for row in doc['vectors']}, {'normalization', 'special_policy'})
        for row in doc['vectors']:
            with self.subTest(row['name']):
                raw = bytes.fromhex(row['hex'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(), row['artifact_sha256'])
                base = bytes.fromhex((FIXTURES / (row['base'] + '.hex')).read_text())
                self.assertEqual(hashlib.sha256(base).hexdigest(), row['base_artifact_sha256'])
                entry = 96 + 3 * 80
                self.assertEqual(struct.unpack_from('<H', raw, entry)[0], row['component_kind'])
                offset = struct.unpack_from('<Q', raw, entry + 16)[0]
                changed = {i for i in range(len(raw)) if raw[i] != base[i]}
                allowed = set(range(32, 64)) | set(range(entry + 32, entry + 64))
                self.assertEqual(changed - allowed, {offset + row['component_offset']})
                self.assertEqual((base[offset + row['component_offset']], raw[offset + row['component_offset']]), (row['original'], row['value']))
                self.assertEqual(ref.frame(raw)['stage'], 'FramedPackage')
                with self.assertRaises(ref.ContractError) as error:
                    ref.validate(raw)
                self.assertEqual(error.exception.code, row['expected'])

    def test_n02_matrix_negatives_have_the_same_family_as_rust(self):
        path = FIXTURES / 'n02-matrix-negatives-v1.json'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), '1a724aa256436a1c8c2f4b95edc25513ac4b7bcff19de338ac401e238f38c966')
        doc = json.loads(path.read_bytes())
        self.assertEqual(doc['format'], 'native-model-n02-matrix-negatives-v1')
        self.assertEqual(len(doc['vectors']), 8)
        for row in doc['vectors']:
            with self.subTest(row['name']):
                raw = bytes.fromhex(row['hex'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(), row['artifact_sha256'])
                base = bytes.fromhex((FIXTURES / (row['base'] + '.hex')).read_text())
                self.assertEqual(hashlib.sha256(base).hexdigest(), row['base_artifact_sha256'])
                entry = 96 + (row['component_kind'] - 1) * 80
                self.assertEqual(struct.unpack_from('<H', raw, entry)[0], row['component_kind'])
                start = struct.unpack_from('<Q', raw, entry + 16)[0] + row['component_offset']
                old, new = bytes.fromhex(row['original_hex']), bytes.fromhex(row['value_hex'])
                self.assertEqual((base[start:start + len(old)], raw[start:start + len(new)]), (old, new))
                changed = {i for i in range(len(raw)) if raw[i] != base[i]}
                allowed = set(range(32, 64)) | set(range(entry + 32, entry + 64)) | set(range(start, start + len(new)))
                self.assertLessEqual(changed, allowed)
                self.assertEqual(ref.frame(raw)['stage'], 'FramedPackage')
                with self.assertRaises(ref.ContractError) as error:
                    ref.validate(raw)
                self.assertEqual(error.exception.code, row['expected'])

    def test_versioned_offset_erratum_preserves_v1_and_marks_absent_payloads(self):
        path = FIXTURES / 'field-table-errata-v2.json'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), '0b5064e396dcbaa365bd038fa611acb88ef9869d01edd6aa434dcbb60facd678')
        corrected = json.loads(path.read_bytes())
        self.assertEqual(corrected['format'], 'native-model-field-table-errata-v2')
        original = (FIXTURES / corrected['source_table']).read_bytes()
        self.assertEqual(hashlib.sha256(original).hexdigest(), corrected['source_table_sha256'])
        table = json.loads(original)
        for row in corrected['fixtures']:
            name = row['name']
            raw = bytes.fromhex((FIXTURES / (name + '.hex')).read_text())
            self.assertEqual(hashlib.sha256(raw).hexdigest(), row['artifact_sha256'])
            framed = ref.frame(raw)
            for kind, parser in [(6, ref.optimizer), (7, ref.rng_order)]:
                payload = framed['entries'][kind]['bytes']
                state = parser(ref.Reader(payload, kind))
                record = row['component_states'][str(kind)]
                self.assertEqual(record['encoded_bytes'], len(payload))
                self.assertEqual(record['state_tag_offset'], 0)
                self.assertEqual(record['state'], 'Absent' if state is None else 'Present')
                if state is None:
                    self.assertEqual(payload, b'\x00')
            if name in ('inference', 'bpe'):
                self.assertTrue(row['removed_v1_offsets'])
                self.assertFalse(any(k.startswith(('optimizer.', 'rng.')) for k in row['field_offsets']))
            else:
                self.assertFalse(row['removed_v1_offsets'])
            self.assertEqual(dict(row['field_offsets'], **row['removed_v1_offsets']), table[name])


if __name__ == '__main__':
    unittest.main()
