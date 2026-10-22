"""Finite pipeline failure, semantic order and real CLI probes."""
import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_complete_episode_decode as d
import wdbx_complete_episode_decode_ingress as ingress
import wdbx_complete_episode_draft as encoder
from abbey_cbor_reader_v1 import Reader, DEFAULTS
import abbey_cbor_episode_v1 as c
from test_wdbx_complete_episode_decode import FIXTURES


def raw(name='minimal'):
    return bytes.fromhex((FIXTURES / f'provisional-{name}.hex').read_text())


def line(data=None, **extra):
    return json.dumps({'cbor_hex': (raw() if data is None else data).hex(), **extra}).encode()


class ProtocolTests(unittest.TestCase):
    def reject(self, data, code, limits=None):
        with self.assertRaises(d.CompleteDecodeError) as caught:
            d.decode_complete(data, limits)
        self.assertEqual((caught.exception.code, str(caught.exception)), (code, code))

    def test_actual_digest_reaches_real_validator(self):
        with patch.object(d, 'validate_fixture', wraps=d.validate_fixture) as spy:
            d.decode_complete(raw('populated'))
        self.assertEqual(spy.call_args.args[1], hashlib.sha256(raw('populated')).digest())

    def test_static_errors_and_injected_stage_categories(self):
        for unknown in ('secret-sensitive', '', 4, None):
            with self.assertRaisesRegex(ValueError, '^invalid complete error category$'):
                d.CompleteDecodeError(unknown)
        for target, error in (
            ('prepare_raw', d.IngressError('IngressLimit')),
            ('map_complete', d.ValueDecodeError('UnknownVariant')),
            ('validate_fixture', d.RelationalDecodeError('InvalidValue:SelfReference')),
        ):
            with self.subTest(target=target), patch.object(d, target, side_effect=error):
                self.reject(raw(), error.code)
        with patch.object(d, 'Reader', side_effect=d.DecodeError('UnexpectedEnd')):
            self.reject(raw(), 'Canonical:UnexpectedEnd')

    def test_memory_failures_at_every_stage_and_request(self):
        for target in ('prepare_raw', 'Reader', 'map_complete', 'validate_fixture'):
            with self.subTest(target=target), patch.object(d, target, side_effect=MemoryError('secret')):
                self.reject(raw(), 'AllocationFailure')
        with patch.object(d.encoder, 'map_fixture', side_effect=MemoryError('secret')):
            self.reject(raw(), 'AllocationFailure')
        with patch.object(d.encoder, 'cbor', side_effect=MemoryError('secret')):
            self.reject(raw(), 'AllocationFailure')
        with patch.object(d, 'parse_request', side_effect=MemoryError('secret')):
            self.assertEqual(d.request(line()), {'ok': False, 'error': 'AllocationFailure'})

    def test_invariant_and_io_errors_propagate(self):
        with patch.object(d.encoder, 'map_fixture', side_effect=encoder.FixtureError('secret')):
            with self.assertRaisesRegex(RuntimeError, '^validated fixture encoding invariant failed$') as caught:
                d.decode_complete(raw())
            self.assertTrue(caught.exception.__suppress_context__)
        with patch.object(d, 'map_complete', side_effect=RuntimeError('programmer')):
            with self.assertRaises(RuntimeError):
                d.request(line())
        class Broken(io.BytesIO):
            def write(self, _value):
                raise OSError('sink failure')
        with self.assertRaises(OSError):
            d.run_json_lines(io.BytesIO(line()), Broken())

    def test_serialization_allocation_has_no_partial_output(self):
        sink = io.BytesIO()
        source = io.BytesIO(line())
        with patch.object(d.json, 'dumps', side_effect=MemoryError('secret')):
            self.assertEqual(d.run_json_lines(source, sink), 0)
        self.assertEqual(sink.getvalue(), b'{"ok":false,"error":"AllocationFailure"}\n')

    def test_raw_and_policy_categories(self):
        for data, code in ((b'', 'Canonical:UnexpectedEnd'),
                           (b'\x18\x00', 'Canonical:NonMinimalArgument'),
                           (b'\x1f', 'Canonical:InvalidAdditionalInformation'),
                           (b'\xc0\x00', 'Canonical:UnsupportedType'),
                           (raw()+b'\x00', 'Canonical:TrailingBytes')):
            self.reject(data, code)
        for key, code in (('max_input_bytes', 'InputLimit'),
                          ('max_value_count', 'ValueLimit'),
                          ('max_collection_entries', 'CollectionLimit'),
                          ('max_string_bytes', 'StringLimit')):
            self.reject(raw(), 'Canonical:'+code, {key: 0})
        self.assertEqual(d.decode_complete(raw(), {'max_parent_count': 0})['bytes'], raw().hex())
        self.reject(bytes(1048577), 'Canonical:InputLimit', {'max_input_bytes': 2**63-1})
        for position in range(len(raw())):
            with self.subTest(position=position):
                with self.assertRaises(d.CompleteDecodeError):
                    d.decode_complete(raw()[:position])

    def test_request_rejections_and_exact_shapes(self):
        for invalid in (b'\xff', b'', b'{}', b'{"cbor_hex":"","cbor_hex":""}',
                        b'{"cbor_hex":"00","limits":{"max_value_count":1.0}}',
                        b'{"cbor_hex":"00","x":1}',
                        b'{"cbor_hex":"00","limits":NaN}'):
            self.assertEqual(d.request(invalid), {'ok': False, 'error': 'RequestError'})
        response = d.request(line())
        self.assertEqual(set(response), {'ok', 'fixture', 'bytes', 'digest'})
        self.assertIs(response['ok'], True)

    def test_logical_lines_oversize_blank_invalid_and_final_no_lf(self):
        source = io.BytesIO(b'x'*(ingress.MAX_LINE_BYTES+100)+b'\n\n{}\n'+line()+b'\n'+line())
        sink = io.BytesIO()
        self.assertEqual(d.run_json_lines(source, sink), 0)
        responses = [json.loads(value) for value in sink.getvalue().splitlines()]
        self.assertEqual(len(responses), 5)
        self.assertEqual([value.get('error') for value in responses[:3]],
                         ['IngressLimit', 'RequestError', 'RequestError'])
        self.assertTrue(all(value['ok'] for value in responses[3:]))
        self.assertTrue(sink.getvalue().endswith(b'\n'))

    def test_real_finite_cli(self):
        script = Path(d.__file__).resolve()
        completed = subprocess.run([sys.executable, str(script), 'decode-json-lines'],
                                   input=b'{}\n'+line()+b'\n'+line(),
                                   capture_output=True, timeout=30)
        self.assertEqual((completed.returncode, completed.stderr), (0, b''))
        responses = [json.loads(value) for value in completed.stdout.splitlines()]
        self.assertEqual(len(responses), 3)
        self.assertEqual(responses[0], {'ok': False, 'error': 'RequestError'})
        self.assertEqual(responses[1], responses[2])
        wrong = subprocess.run([sys.executable, str(script), 'unknown'], input=b'',
                               capture_output=True, timeout=30)
        self.assertEqual((wrong.returncode, wrong.stdout, wrong.stderr), (2, b'', b''))

    def test_semantic_order_and_actions(self):
        # Map encoding remains canonical while semantic set order is reversed.
        source = json.loads((FIXTURES/'provisional-populated.json').read_text())
        for group_name, field in (('authority', 'models'), ('authority', 'calibrations'), ('lifecycle', 'holds'), ('evidence', 'observation_refs'), ('provenance', 'records')):
            extra = copy.deepcopy(source[group_name][field][0])
            if 'component' in extra:
                extra['component'] = 'second'
            else:
                extra['digest'] = 'ef' * 32
            source[group_name][field].append(extra)
        canonical = encoder.cbor(encoder.map_fixture(source))
        root = Reader(canonical, DEFAULTS.copy(), c, False).value()
        def group(name):
            index = ('identity', 'time', 'origin', 'context', 'decision', 'outcome', 'authority', 'evidence', 'lifecycle', 'provenance').index(name) + 2
            return next(value for key, value in root.entries if key.value == index)
        exercised = 0
        for group_name, fields in (
            ('context', ('resources',)), ('outcome', ('costs',)),
            ('authority', ('capabilities', 'models', 'calibrations', 'approvals')),
            ('lifecycle', ('holds', 'edges')),
            ('provenance', ('causal_parents', 'consulted_parents', 'records')),
            ('evidence', ('source_refs', 'observation_refs'))):
            for field in fields:
                values = next(value for key, value in group(group_name).entries if key.value == field)
                if len(values.items) > 1:
                    values.items.reverse()
                    with self.subTest(field=field):
                        self.reject(c.encode_value(root), 'NonCanonicalEncoding')
                    values.items.reverse()
                    exercised += 1
        self.assertEqual(exercised, 13)
        posterior = next(value for key, value in group('context').entries if key.value == 'regime_posterior').items[1]
        posterior.items.reverse()
        self.reject(c.encode_value(root), 'NonCanonicalEncoding')
        posterior.items.reverse()
        action_tree = next(value for key, value in group('decision').entries if key.value == 'actions').items[0]
        params = next(value for key, value in action_tree.entries if key.value == 'parameter_classes')
        params.items.reverse()
        self.reject(c.encode_value(root), 'NonCanonicalEncoding')
        params.items.reverse()
        actions = source['decision']['actions']
        other_action = copy.deepcopy(actions[0])
        other_action['action'] = 'other-action'
        actions.append(other_action)
        actions.append(copy.deepcopy(actions[0]))
        encoded = encoder.cbor(encoder.map_fixture(source))
        self.assertEqual(len(d.decode_complete(encoded)['fixture']['decision']['actions']), len(actions))
        if len(actions) > 1:
            actions.reverse()
            other = encoder.cbor(encoder.map_fixture(source))
            self.assertEqual(d.decode_complete(other)['bytes'], other.hex())
            # Reverse three palindromic actions would be unchanged; rotate instead.
            actions.append(actions.pop(0))
            changed = encoder.cbor(encoder.map_fixture(source))
            self.assertNotEqual(changed, encoded)
            self.assertEqual(d.decode_complete(changed)['bytes'], changed.hex())

if __name__ == '__main__':
    unittest.main()
