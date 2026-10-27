"""Private P1 cryptographic witness tests; invoke only with explicit dependency."""
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wdbx_complete_episode_draft as encoder
import wdbx_complete_p1_crypto as p1

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'crates/abi-wdbx/src/v3/complete_episode/tests/provisional-minimal.json'


def payload(seed=1):
    fixture = json.loads(FIXTURE.read_text())
    public = p1.synthetic_public(bytes([seed]) * 32)
    fixture['origin']['signer'] = p1.key_id(public)
    return encoder.cbor(encoder.map_fixture(fixture))


def request(value):
    return p1.request(json.dumps(value).encode())


class P1Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Explicit failure, never a dependency skip.
        p1.backend_identity()

    def sign(self):
        body = payload()
        signed = request({'op': 'sign', 'payload_hex': body.hex(),
                          'synthetic_seed_hex': (bytes([1])*32).hex()})
        self.assertTrue(signed['ok'], signed)
        return body, signed

    def test_sign_verify_and_original_structure(self):
        body, signed = self.sign()
        self.assertEqual(set(signed), {'ok', 'container_hex', 'public_key_hex', 'sig_structure_hex'})
        result = request({'op': 'verify', 'container_hex': signed['container_hex'],
                          'payload_hex': body.hex(), 'public_key_hex': signed['public_key_hex']})
        self.assertEqual(result, {'ok': True, 'valid': True, 'reason': None})
        parsed = p1.parse_p1(bytes.fromhex(signed['container_hex']), body)
        self.assertEqual(parsed['sig_structure'].hex(), signed['sig_structure_hex'])
        self.assertTrue(parsed['sig_structure'].endswith(body))
        self.assertIn(p1.PROFILE.encode(), parsed['sig_structure'])

    def test_tamper_and_wrong_key_binding(self):
        body, signed = self.sign()
        container = bytearray.fromhex(signed['container_hex']); container[-1] ^= 1
        self.assertEqual(request({'op': 'verify', 'container_hex': container.hex(),
                                  'payload_hex': body.hex(), 'public_key_hex': signed['public_key_hex']}),
                         {'ok': True, 'valid': False, 'reason': 'InvalidSignature'})
        wrong = p1.synthetic_public(bytes([2])*32)
        self.assertEqual(request({'op': 'verify', 'container_hex': signed['container_hex'],
                                  'payload_hex': body.hex(), 'public_key_hex': wrong.hex()}),
                         {'ok': True, 'valid': False, 'reason': 'KeyIdentityMismatch'})
        self.assertEqual(request({'op': 'sign', 'payload_hex': body.hex(),
                                  'synthetic_seed_hex': (bytes([2])*32).hex()}),
                         {'ok': False, 'error': 'KeyIdentityMismatch'})

    def test_substituted_valid_payload_is_not_reencoded_to_original(self):
        body, signed = self.sign()
        fixture=json.loads(FIXTURE.read_text()); fixture['origin']['signer']=p1.key_id(bytes.fromhex(signed['public_key_hex']))
        fixture['origin']['source_node']='changed-source'
        changed=encoder.cbor(encoder.map_fixture(fixture))
        self.assertNotEqual(changed,body)
        self.assertEqual(request({'op':'verify','container_hex':signed['container_hex'],'payload_hex':changed.hex(),'public_key_hex':signed['public_key_hex']}), {'ok':True,'valid':False,'reason':'InvalidSignature'})

    def test_closed_shape_and_all_truncations(self):
        body,signed=self.sign(); container=bytes.fromhex(signed['container_hex'])
        for end in range(len(container)):
            with self.assertRaises(p1.WitnessError):p1.parse_p1(container[:end],body)
        with self.assertRaises(p1.WitnessError):p1.parse_p1(container+b'\0',body)
        for index,byte in [(0,0xd3),(1,0x83),(2,0x5f),(82,0xa1),(83,0x40)]:
            changed=bytearray(container);changed[index]=byte
            with self.assertRaises(p1.WitnessError):p1.parse_p1(changed,body)
        changed=bytearray(container);changed[6]=0x27
        with self.assertRaises(p1.WitnessError) as raised:p1.parse_p1(changed,body)
        self.assertEqual(raised.exception.code,'UnsupportedAlgorithm')

    def test_json_ingress_is_strict_and_bounded(self):
        for value in [b'{"op":"sign","op":"verify"}',b'[]',b'{}',b'{"op":"other"}',b'{"op":"sign","payload_hex":"AB","synthetic_seed_hex":"00"}',b'{"op":"sign","payload_hex":"00 ","synthetic_seed_hex":"00"}',b'{"op":"verify","container_hex":"","payload_hex":"","public_key_hex":"","extra":1}',b'{"op":"sign","payload_hex":0,"synthetic_seed_hex":"00"}']:
            response=p1.request(value);self.assertFalse(response['ok']);self.assertNotIn('AB',response['error'])
        self.assertEqual(p1.request(b' '*(p1.MAX_REQUEST+1)), {'ok':False,'error':'ResourceLimit'})
        source=io.BytesIO(b' '*(p1.MAX_REQUEST+100));sink=io.BytesIO()
        self.assertEqual(p1.run_request(source,sink),0)
        self.assertEqual(source.tell(),p1.MAX_REQUEST+1)
        self.assertEqual(json.loads(sink.getvalue()),{'ok':False,'error':'ResourceLimit'})

    def test_payload_and_declared_length_bounds(self):
        body,signed=self.sign();container=bytes.fromhex(signed['container_hex'])
        for bad in [b'',b'\xa0',b'\x81'*33+b'\0',body+b'\0',bytes(1_048_577)]:
            with self.assertRaises(p1.WitnessError):p1.parse_p1(container,bad)
        for bad in [bytes(257),b'\xd2\x84\x5b'+bytes([255])*8]:
            with self.assertRaises(p1.WitnessError) as raised:p1.parse_p1(bad,body)
            self.assertEqual(raised.exception.code,'ResourceLimit')

    def test_known_backend_disagreement_is_characterized_not_qualified(self):
        probe=p1.backend_probe()
        self.assertEqual(probe['raw_weak_key_accepted'],p1.selected_backend()=='pyca')
        self.assertFalse(probe['strict_parity_qualified'])
        result=subprocess.run([sys.executable,str(ROOT/'tools/wdbx_complete_p1_crypto.py'),'backend-probe','--qualify'],capture_output=True,check=False)
        self.assertEqual(result.returncode,1)
        self.assertFalse(json.loads(result.stdout)['strict_parity_qualified'])

    def test_native_ph_signature_is_not_pure_message_ed25519(self):
        path=ROOT/'docs/completion/execution/evidence/2026-10-22-continuation/libsodium-dependencies/ph-vector.json'
        vector=json.loads(path.read_text())
        self.assertEqual(request({'op':'verify','container_hex':vector['container_hex'],'payload_hex':vector['payload_hex'],'public_key_hex':vector['public_key_hex']}), {'ok':True,'valid':False,'reason':'InvalidSignature'})
        if p1.selected_backend()=='libsodium':
            # Produce Ed25519ph through maintained native APIs, not by signing
            # a digest with pure Ed25519. Exact deterministic fixture must agree.
            from nacl.bindings import crypto_sign_seed_keypair,crypto_sign_ed25519ph_state,crypto_sign_ed25519ph_update,crypto_sign_ed25519ph_final_create,crypto_sign_ed25519ph_final_verify
            public,secret=crypto_sign_seed_keypair(bytes.fromhex(vector['synthetic_seed_hex']))
            state=crypto_sign_ed25519ph_state()
            crypto_sign_ed25519ph_update(state,bytes.fromhex(vector['sig_structure_hex']))
            signature=crypto_sign_ed25519ph_final_create(state,secret)
            self.assertEqual(signature.hex(),vector['signature_hex'])
            state=crypto_sign_ed25519ph_state()
            crypto_sign_ed25519ph_update(state,bytes.fromhex(vector['sig_structure_hex']))
            self.assertTrue(crypto_sign_ed25519ph_final_verify(state,signature,public))

    def test_malformed_point_and_scalar_matrix_uses_selected_native_backend(self):
        body,signed=self.sign();container=bytes.fromhex(signed['container_hex'])
        original=container[-64:]
        order=2**252+27742317777372353535851937790883648493
        # Construct malformed test bytes only; all signature judgments remain
        # in the maintained backend, never application point arithmetic.
        signatures=[bytes([255])*32+original[32:],b'\x01'+bytes(31)+original[32:],original[:32]+(int.from_bytes(original[32:],'little')+order).to_bytes(32,'little')]
        for signature in signatures:
            self.assertEqual(request({'op':'verify','container_hex':(container[:-64]+signature).hex(),'payload_hex':body.hex(),'public_key_hex':signed['public_key_hex']}),{'ok':True,'valid':False,'reason':'InvalidSignature'})

    def test_missing_dependency_fails_explicit_cli_invocation(self):
        result=subprocess.run([sys.executable,'-S',str(ROOT/'tools/wdbx_complete_p1_crypto.py'),'backend-probe'],capture_output=True,check=False)
        self.assertEqual(result.returncode,2)
        self.assertEqual(json.loads(result.stdout),{'ok':False,'error':'DependencyUnavailable'})

    def test_unexpected_backend_failure_is_static_and_fails_request(self):
        raw=json.dumps({'op':'sign','payload_hex':payload().hex(),'synthetic_seed_hex':(bytes([1])*32).hex()}).encode()
        for failure in [RuntimeError('private/backend/detail'),OSError('/private/backend/path')]:
            with patch.object(p1,'_crypto',side_effect=failure):
                sink=io.BytesIO()
                self.assertEqual(p1.run_request(io.BytesIO(raw),sink),2)
                self.assertEqual(json.loads(sink.getvalue()),{'ok':False,'error':'BackendUnavailable'})

    def test_unexpected_backend_failure_is_static_and_fails_probe_cli(self):
        for failure in [RuntimeError('private/backend/detail'),OSError('/private/backend/path')]:
            with patch.object(p1,'_verify',side_effect=failure),patch.object(sys,'stdout',io.StringIO()) as output:
                self.assertEqual(p1.main(['backend-probe']),2)
                self.assertEqual(json.loads(output.getvalue()),{'ok':False,'error':'BackendUnavailable'})


if __name__ == '__main__':unittest.main()
