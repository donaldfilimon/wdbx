"""Synthetic ingress tests; no CBOR or episode validity claim."""
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_complete_episode_decode_ingress as ingress
from abbey_cbor_reader_v1 import DEFAULTS


class GuardedStream:
    def __init__(self, data):
        self.stream = io.BytesIO(data)
        self.calls = []

    def readline(self, size):
        assert type(size) is int and size > 0
        assert size in (ingress.MAX_LINE_BYTES + 1, 4096)
        self.calls.append(size)
        return self.stream.readline(size)

    def read(self, *args):
        raise AssertionError('read forbidden')

    def __iter__(self):
        raise AssertionError('iteration forbidden')


class IngressTests(unittest.TestCase):
    def reject(self, callback, code='RequestError'):
        with self.assertRaises(ingress.IngressError) as caught:
            callback()
        self.assertEqual(caught.exception.code, code)
        self.assertEqual(str(caught.exception), code)

    def request(self, value):
        return ingress.parse_request(json.dumps(value).encode())

    def test_raw_defaults_identity_and_isolation(self):
        original = DEFAULTS.copy()
        data = b'abc'
        result, limits = ingress.prepare_raw(data)
        self.assertIs(result, data)
        self.assertEqual(limits, original)
        limits['max_value_count'] = 0
        self.assertEqual(DEFAULTS, original)
        self.assertEqual(ingress.prepare_raw(data)[1], original)

    def test_limits_zero_maximum_and_clamp(self):
        for key in DEFAULTS:
            with self.subTest(key=key):
                _, effective = ingress.prepare_raw(b'', {key: 0})
                self.assertEqual(effective[key], 0)
                _, effective = ingress.prepare_raw(b'', {key: 2**63 - 1})
                self.assertEqual(effective[key], ingress.MAX_CBOR_BYTES if key == 'max_input_bytes' else 2**63 - 1)
        overrides = {key: index + 7 for index, key in enumerate(DEFAULTS)}
        self.assertEqual(ingress.prepare_raw(b'', overrides)[1], overrides)
        self.assertIsNot(ingress.prepare_raw(b'', overrides)[1], overrides)

    def test_bad_limits_and_raw_types(self):
        for value in (True, 1.0, -1, 2**63, None, '1'):
            for key in DEFAULTS:
                self.reject(lambda: ingress.prepare_raw(b'', {key: value}))
        for limits in ([], 'bad', 3, True, {'unknown': 1}):
            self.reject(lambda: ingress.prepare_raw(b'', limits))
        for data in ('', bytearray(), memoryview(b''), None):
            self.reject(lambda: ingress.prepare_raw(data))
            self.reject(lambda: ingress.parse_request(data))

    def test_hex_forms_and_request_shape(self):
        for text in ('', '00', 'aAbB', 'ABCDEF'):
            self.assertEqual(self.request({'cbor_hex': text})[0], bytes.fromhex(text))
        for text in ('a', 'gg', '00 00', '\n00', '００', 'é0'):
            self.reject(lambda: self.request({'cbor_hex': text}))
        for value in ({}, [], None, 'secret-sentinel', {'cbor_hex': 1}, {'cbor_hex': '', 'extra': 1}):
            self.reject(lambda: self.request(value))

    def test_input_caps_before_unhex(self):
        self.assertEqual(len(ingress.prepare_raw(b'x' * ingress.MAX_CBOR_BYTES)[0]), ingress.MAX_CBOR_BYTES)
        self.reject(lambda: ingress.prepare_raw(b'x' * (ingress.MAX_CBOR_BYTES + 1)), 'Canonical:InputLimit')
        self.reject(lambda: ingress.prepare_raw(b'x', {'max_input_bytes': 0}), 'Canonical:InputLimit')
        with patch.object(ingress, '_unhex', side_effect=AssertionError('must not allocate')):
            self.reject(lambda: self.request({'cbor_hex': '00', 'limits': {'max_input_bytes': 0}}), 'Canonical:InputLimit')
            self.reject(lambda: self.request({'cbor_hex': '00' * (ingress.MAX_CBOR_BYTES + 1), 'limits': {'max_input_bytes': 2**63 - 1}}), 'Canonical:InputLimit')
        self.assertEqual(self.request({'cbor_hex': '00', 'limits': {'max_input_bytes': 1}})[0], b'\0')
        for bad in (None, [], True, 3, 'bad'):
            self.reject(lambda: self.request({'cbor_hex': '', 'limits': bad}))
        self.reject(lambda: self.request({'cbor_hex': '', 'limits': {'unknown': 1}}))

    def test_invalid_json_and_duplicate_keys(self):
        for line in (b'', b'\n', b'\xff', b'{', b'{"cbor_hex":"","cbor_hex":"00"}', b'{"cbor_hex":"","limits":{"max_value_count":1,"max_value_count":2}}', b'{"cbor_hex":"","limits":{"x":{"secret-sentinel":1,"secret-sentinel":2}}}', b'{"cbor_hex":"","limits":{"max_value_count":NaN}}', b'{"cbor_hex":"","limits":{"max_value_count":Infinity}}', b'{"cbor_hex":"","limits":{"max_value_count":-Infinity}}', b'{"cbor_hex":"","limits":{"max_value_count":1.0}}', b'{"cbor_hex":"","limits":{"max_value_count":1e0}}'):
            with self.subTest(line=line[:40]):
                self.reject(lambda: ingress.parse_request(line))

    def test_integer_token_bound_before_int(self):
        # Both are wrong request shapes; only the 20-digit token reaches int.
        with patch.object(ingress, '_parse_int', wraps=ingress._parse_int) as parser:
            self.reject(lambda: ingress.parse_request(b'12345678901234567890'))
            self.assertEqual(parser.call_count, 1)
        for token in (b'123456789012345678901', b'-123456789012345678901'):
            self.reject(lambda: ingress.parse_request(token))
        self.assertEqual(ingress._parse_int('-12345678901234567890'), -12345678901234567890)
        with patch.object(ingress, 'int', side_effect=AssertionError('integer conversion forbidden'), create=True):
            self.reject(lambda: ingress._parse_int('123456789012345678901'))
            self.reject(lambda: ingress._parse_int('-123456789012345678901'))

    def test_depth_and_escaped_strings(self):
        actual_loads = json.loads
        with patch.object(ingress.json, 'loads', wraps=actual_loads) as loads:
            self.reject(lambda: ingress.parse_request(b'[' * 64 + b'0' + b']' * 64))
            self.assertEqual(loads.call_count, 1)
            loads.reset_mock()
            self.reject(lambda: ingress.parse_request(b'[' * 65 + b'0' + b']' * 65))
            loads.assert_not_called()
        # Escaped quotes/backslashes/brackets inside strings do not add depth.
        value = {'cbor_hex': '', 'extra': '\\"' + '[' * 100 + ']' * 100}
        with patch.object(ingress.json, 'loads', wraps=actual_loads) as loads:
            self.reject(lambda: self.request(value))
            self.assertEqual(loads.call_count, 1)
        with patch.object(ingress.json, 'loads', side_effect=AssertionError('must not parse')):
            self.reject(lambda: ingress.parse_request(b'x' * (ingress.MAX_LINE_BYTES + 1)), 'IngressLimit')

    def test_framing_boundaries_and_recovery(self):
        cap = ingress.MAX_LINE_BYTES
        cases = [
            (b'', []), (b'\n', [b'\n']), (b'a\nb\n', [b'a\n', b'b\n']),
            (b'last', [b'last']), (b'x' * cap, [b'x' * cap]), (b'x' * (cap - 1) + b'\n', [b'x' * (cap - 1) + b'\n']),
            (b'x' * cap + b'\nnext\n', ['IngressLimit', b'next\n']),
            (b'x' * (cap + 4096) + b'\nnext\n', ['IngressLimit', b'next\n']),
            (b'x' * (cap + 8192), ['IngressLimit']),
        ]
        for data, expected in cases:
            stream = GuardedStream(data)
            result = list(ingress.iter_lines(stream))
            self.assertEqual([item.code if isinstance(item, ingress.IngressError) else item for item in result], expected)
            self.assertTrue(all(size > 0 for size in stream.calls))

    def test_memory_failure_sites_and_io_propagation(self):
        for name in ('loads',):
            with patch.object(ingress.json, name, side_effect=MemoryError):
                self.reject(lambda: ingress.parse_request(b'{"cbor_hex":""}'), 'AllocationFailure')
        with patch.object(ingress, '_unhex', side_effect=MemoryError):
            self.reject(lambda: ingress.parse_request(b'{"cbor_hex":""}'), 'AllocationFailure')
        with patch.object(ingress, '_effective_limits', side_effect=MemoryError):
            self.reject(lambda: ingress.prepare_raw(b''), 'AllocationFailure')
            self.reject(lambda: ingress.parse_request(b'{"cbor_hex":""}'), 'AllocationFailure')
        with patch.object(ingress.json, 'loads', side_effect=RecursionError):
            self.reject(lambda: ingress.parse_request(b'{"cbor_hex":""}'))
        stream = GuardedStream(b'')
        with patch.object(stream, 'readline', side_effect=MemoryError) as read:
            result = list(ingress.iter_lines(stream))
            self.assertEqual([item.code for item in result], ['AllocationFailure'])
            self.assertEqual(read.call_count, 1)
        stream = GuardedStream(b'')
        with patch.object(stream, 'readline', side_effect=[b'x' * (ingress.MAX_LINE_BYTES + 1), MemoryError]) as read:
            self.assertEqual([item.code for item in ingress.iter_lines(stream)], ['IngressLimit', 'AllocationFailure'])
            self.assertEqual(read.call_count, 2)
        stream = GuardedStream(b'')
        with patch.object(stream, 'readline', side_effect=OSError('system failure')):
            with self.assertRaises(OSError):
                list(ingress.iter_lines(stream))


if __name__ == '__main__':
    unittest.main()
