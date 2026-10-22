"""Encoding-witness tests for the private `cose-sign1-p1` profile.

These tests cover the Python side only: the bounded container reader, the
bounded protected-header reader, the RFC 9052 `Sig_structure` rebuild, the
golden vectors, and the strict JSON-lines request protocol.

They deliberately do **not** cover signature verification. This environment has
no Python cryptography library, and this repository's design guidance forbids
hand-rolling cryptography in Python, so the witness never judges a signature.
Anything this suite proves is an *encoding* fact.
"""
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_cose_sign1_p1 as w

GOLDEN = Path(__file__).resolve().parents[2] / 'crates/abi-wdbx/tests/golden/cose-sign1-p1'


def golden_bytes(name):
    return bytes.fromhex((GOLDEN / name).read_text().strip())


def golden_container():
    return golden_bytes('container.hex')


def golden_kid():
    return golden_bytes('kid.hex')


def push_byte_string(out, data):
    """Append one definite-length byte string in its shortest form."""
    if len(data) <= 23:
        out += bytes([0x40 | len(data)])
    elif len(data) <= 0xFF:
        out += bytes([0x58, len(data)])
    else:
        out += bytes([0x59]) + len(data).to_bytes(2, 'big')
    out += data


def container(protected, payload, signature):
    """Frame a container in the profile's element order."""
    out = bytearray([0xd2, 0x84])
    push_byte_string(out, protected)
    out += b'\xa0'
    push_byte_string(out, payload)
    push_byte_string(out, signature)
    return bytes(out)


def canonical_header(entries):
    """Encode a protected header from `(label, encoded value)` pairs."""
    out = bytearray([0xa0 + len(entries)])
    for label, value in entries:
        out += bytes([label]) + value
    return bytes(out)


NEG_19 = bytes([0x32])
NEG_8 = bytes([0x27])


def kid_entry():
    kid = golden_kid()
    return bytes([0x58, len(kid)]) + kid


class ContainerTests(unittest.TestCase):
    def test_the_golden_container_parses(self):
        parsed = w.decode_container(golden_container())
        self.assertEqual(parsed['alg'], -19)
        self.assertEqual(parsed['kid'], golden_kid())
        self.assertEqual(parsed['key_id'], golden_kid().decode('ascii'))
        self.assertEqual(parsed['payload'], bytes([0x5a]) * 32)
        self.assertEqual(len(parsed['signature']), 64)
        self.assertEqual(parsed['protected'], golden_bytes('protected.hex'))

    def test_the_golden_sig_structure_is_rebuilt_byte_for_byte(self):
        parsed = w.decode_container(golden_container())
        structure = w.build_sig_structure(parsed['protected'], parsed['payload'])
        self.assertEqual(structure, golden_bytes('sig_structure.hex'))
        self.assertEqual(
            hashlib.sha256(structure).hexdigest(),
            (GOLDEN / 'sig_structure.sha256').read_text().strip(),
        )
        # The rebuild is exactly the RFC 9052 shape.
        self.assertTrue(structure.startswith(b'\x84\x6aSignature1'))
        self.assertIn(b'wdbx-cose-sign1-p1', structure)

    def test_the_rebuilt_protected_header_matches_the_carried_bytes(self):
        parsed = w.decode_container(golden_container())
        self.assertEqual(
            w.build_protected_header(parsed['alg'], parsed['kid']), parsed['protected']
        )

    def test_the_witness_never_claims_a_signature_verdict(self):
        response = w.inspect(golden_container())
        self.assertTrue(response['ok'])
        self.assertEqual(response['scope'], 'encoding-agreement-only')
        # The signature is reported as bytes, never as a verdict.
        self.assertEqual(len(response['signature_bytes_hex']), 128)
        self.assertNotIn('valid', json.dumps(response).lower())
        self.assertNotIn('verified', json.dumps(response).lower())

    def test_the_golden_vector_verifies_standalone(self):
        status = subprocess.run(
            [sys.executable, str(Path(w.__file__).resolve()), 'verify-goldens', str(GOLDEN)],
            capture_output=True,
            check=False,
        )
        self.assertEqual(status.returncode, 0, status.stdout.decode())
        self.assertIn(b'0 failure(s)', status.stdout)


class RefusalTests(unittest.TestCase):
    def refuse(self, data, expected):
        with self.assertRaises(w.CoseError) as caught:
            w.decode_container(data)
        self.assertEqual(caught.exception.name, expected)
        self.assertEqual(str(caught.exception), expected)

    def test_a_tag_other_than_eighteen_is_refused(self):
        for tag in (0xd1, 0xd3, 0xc2):
            with self.subTest(tag=tag):
                data = bytearray(golden_container())
                data[0] = tag
                self.refuse(bytes(data), 'UnexpectedTag')

    def test_a_wrong_element_count_is_refused(self):
        for count in (0x83, 0x85, 0x86):
            with self.subTest(count=count):
                data = bytearray(golden_container())
                data[1] = count
                self.refuse(bytes(data), 'UnexpectedElementCount')

    def test_a_non_empty_unprotected_map_is_refused(self):
        for head in (0xa1, 0xa2):
            with self.subTest(head=head):
                data = bytearray(golden_container())
                data[2 + 2 + len(golden_bytes('protected.hex'))] = head
                self.refuse(bytes(data), 'UnprotectedHeaderNotEmpty')

    def test_a_wrong_payload_length_is_refused(self):
        for length in (0x1f, 0x21):
            with self.subTest(length=length):
                data = bytearray(golden_container())
                offset = 2 + 2 + len(golden_bytes('protected.hex')) + 1
                data[offset + 1] = length
                self.refuse(bytes(data), 'InvalidPayloadLength')

    def test_a_wrong_signature_length_is_refused(self):
        for length in (0x3f, 0x41):
            with self.subTest(length=length):
                data = bytearray(golden_container())
                offset = 2 + 2 + len(golden_bytes('protected.hex')) + 1 + 2 + 32
                data[offset + 1] = length
                self.refuse(bytes(data), 'InvalidSignatureLength')

    def test_trailing_bytes_are_refused(self):
        self.refuse(golden_container() + b'\x00', 'TrailingBytes')

    def test_a_truncated_container_is_refused(self):
        data = golden_container()
        for cut in (0, 1, 5, 40, len(data) - 1):
            with self.subTest(cut=cut):
                self.refuse(data[:cut], 'Truncated')

    def test_an_oversized_container_is_refused(self):
        self.refuse(b'\x00' * 513, 'ContainerTooLarge')

    def test_an_unknown_protected_label_is_refused(self):
        header = canonical_header([(1, NEG_19), (4, kid_entry()), (5, bytes([0xf6]))])
        self.refuse(container(header, bytes(32), bytes(64)), 'UnknownProtectedLabel')

    def test_a_crit_label_is_refused(self):
        header = canonical_header([(1, NEG_19), (2, bytes([0x81, 0x01])), (4, kid_entry())])
        self.refuse(container(header, bytes(32), bytes(64)), 'CriticalHeaderUnsupported')

    def test_duplicate_protected_keys_are_refused(self):
        header = canonical_header([(1, NEG_19), (1, NEG_19), (4, kid_entry())])
        self.refuse(container(header, bytes(32), bytes(64)), 'MalformedProtectedHeader')

    def test_non_minimal_integers_are_refused(self):
        # -19 written in a two-byte argument instead of the one-byte form.
        header = canonical_header([(1, bytes([0x38, 0x12])), (4, kid_entry())])
        self.refuse(container(header, bytes(32), bytes(64)), 'MalformedProtectedHeader')

    def test_a_non_minimal_length_is_refused(self):
        # A one-byte string written with the two-byte length form.
        header = canonical_header([(1, NEG_19), (4, bytes([0x58, 0x01, 0x61]))])
        self.refuse(container(header, bytes(32), bytes(64)), 'MalformedProtectedHeader')

    def test_protected_keys_out_of_canonical_order_are_refused(self):
        header = canonical_header([(4, kid_entry()), (1, NEG_19)])
        self.refuse(container(header, bytes(32), bytes(64)), 'MalformedProtectedHeader')

    def test_an_unsupported_algorithm_is_refused(self):
        for value in (NEG_8, bytes([0x00]), bytes([0x01])):
            with self.subTest(value=value.hex()):
                header = canonical_header([(1, value), (4, kid_entry())])
                self.refuse(container(header, bytes(32), bytes(64)), 'UnsupportedAlgorithm')

    def test_a_malformed_key_identifier_is_refused(self):
        short = bytes([0x58, len(golden_kid()) - 1]) + golden_kid()[:-1]
        header = canonical_header([(1, NEG_19), (4, short)])
        self.refuse(container(header, bytes(32), bytes(64)), 'InvalidKeyId')

        text = bytes([0x78, 0x48]) + b'g' * 72
        header = canonical_header([(1, NEG_19), (4, text)])
        self.refuse(container(header, bytes(32), bytes(64)), 'InvalidKeyId')

    def test_an_oversized_protected_header_is_refused(self):
        header = b'\x00' * 129
        self.refuse(container(header, bytes(32), bytes(64)), 'ProtectedHeaderTooLarge')

    def test_a_protected_header_that_is_not_a_map_is_refused(self):
        self.refuse(container(b'\xa0', bytes(32), bytes(64)), 'MalformedProtectedHeader')


class ProtocolTests(unittest.TestCase):
    def request(self, container_hex):
        return json.dumps({'container_hex': container_hex}).encode()

    def test_a_valid_request_is_answered_with_the_encoding_facts(self):
        line = self.request(golden_container().hex())
        response = w.respond(line)
        self.assertTrue(response['ok'])
        self.assertEqual(response['scope'], 'encoding-agreement-only')
        self.assertEqual(response['protected_hex'], golden_bytes('protected.hex').hex())
        self.assertEqual(response['sig_structure_hex'], golden_bytes('sig_structure.hex').hex())
        self.assertEqual(response['payload_hex'], (bytes([0x5a]) * 32).hex())
        self.assertEqual(response['alg'], -19)
        self.assertEqual(response['kid'], golden_kid().decode('ascii'))

    def test_malformed_requests_are_refused(self):
        for line in (
            b'',
            b'null',
            b'[]',
            b'{}',
            b'{"container_hex": "00", "extra": 1}',
            b'{"container_hex": 0}',
            b'{"container_hex": "0"}',
            b'{"container_hex": "0g"}',
            b'{"container_hex": "00", "container_hex": "01"}',
            b'{"container_hex": "' + b'A' * 4 + b'"}',
            b'not json',
        ):
            with self.subTest(line=line):
                response = w.respond(line)
                self.assertFalse(response['ok'])
                self.assertEqual(response['error'], 'RequestError')
                self.assertEqual(response['scope'], 'encoding-agreement-only')

    def test_an_over_long_line_is_refused(self):
        source = io.BytesIO(b'x' * (w.MAX_LINE_BYTES + 1) + b'\n')
        self.assertEqual(list(w.iter_lines(source)), [None])

    def test_the_json_lines_protocol_answers_every_request(self):
        source = io.BytesIO(
            self.request(golden_container().hex()) + b'\n' + self.request('00') + b'\n'
        )
        sink = io.BytesIO()
        self.assertEqual(w.run_json_lines(source, sink), 0)
        answers = [json.loads(line) for line in sink.getvalue().splitlines()]
        self.assertEqual(len(answers), 2)
        self.assertTrue(answers[0]['ok'])
        self.assertFalse(answers[1]['ok'])
        self.assertEqual(answers[1]['error'], 'UnexpectedTag')

    def test_the_cli_refuses_an_unknown_command(self):
        status = subprocess.run(
            [sys.executable, str(Path(w.__file__).resolve()), 'not-a-command'],
            capture_output=True,
            check=False,
        )
        self.assertEqual(status.returncode, 1)


if __name__ == '__main__':
    unittest.main()
