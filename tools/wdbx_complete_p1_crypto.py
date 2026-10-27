"""Private independent full-payload P1 witness, not admission or qualification.

The explicitly selected maintained pure-message backend is PyCA or libsodium.
PyCA's known weak-key acceptance disagreement stays visible. It performs no point/scalar arithmetic and
never reads key files or stores. `sign` accepts synthetic test seed bytes only;
callers must never supply production material to this experimental protocol.
Input and selected buffers are bounded, not total process memory.
"""
from __future__ import annotations
import hashlib
from importlib.metadata import PackageNotFoundError, version
import json
import os
import re
import sys

from wdbx_complete_episode_decode import CompleteDecodeError, decode_complete

PROFILE = 'wdbx-complete-cose-sign1-prototype-v1'
MAX_REQUEST = 3 * 1024 * 1024
MAX_PAYLOAD = 1_048_576
MAX_CONTAINER = 256
MAX_PROTECTED = 128
MAX_STRUCTURE = 1_049_088
_PINS = {'cryptography': '50.0.2', 'cffi': '2.1.1', 'pycparser': '3.11'}
_ERRORS = frozenset(('RequestError', 'ResourceLimit', 'MalformedCbor',
    'NonCanonicalEncoding', 'InvalidHeaderSet', 'UnsupportedAlgorithm',
    'InvalidLength', 'InvalidPayload', 'KeyIdentityMismatch',
    'DependencyUnavailable', 'DependencyVersionMismatch', 'BackendUnavailable'))


class WitnessError(ValueError):
    """Static failure; no source bytes or parser diagnostics are retained."""
    def __init__(self, code):
        if code not in _ERRORS:
            raise ValueError('invalid witness error category')
        self.code = code
        super().__init__(code)


def selected_backend():
    selected = os.environ.get('WDBX_CRYPTO_BACKEND', 'pyca')
    if selected not in ('pyca', 'libsodium'):
        raise WitnessError('RequestError')
    return selected


def _crypto():
    selected = selected_backend()
    pins = (_PINS if selected == 'pyca' else
            {'PyNaCl': '1.6.2', 'cffi': '2.1.1', 'pycparser': '3.11'})
    try:
        if any(version(name) != expected for name, expected in pins.items()):
            raise WitnessError('DependencyVersionMismatch')
        if selected == 'libsodium':
            from nacl.signing import SigningKey, VerifyKey
            from nacl.exceptions import BadSignatureError
            return {'selected': selected, 'pins': pins, 'private': SigningKey,
                    'public': VerifyKey, 'invalid': BadSignatureError,
                    'backend': 'libsodium bundled by PyNaCl 1.6.2 (wheel provenance pinned separately)'}
        from cryptography.exceptions import InvalidSignature, UnsupportedAlgorithm
        from cryptography.hazmat.backends.openssl.backend import backend
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey
        from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat
    except (ImportError, PackageNotFoundError):
        raise WitnessError('DependencyUnavailable') from None
    if backend.openssl_version_text() != 'OpenSSL 4.0.3 29 Sep 2026':
        raise WitnessError('DependencyVersionMismatch')
    return {'selected': selected, 'pins': pins, 'private': Ed25519PrivateKey,
            'public': Ed25519PublicKey, 'invalid': InvalidSignature,
            'unsupported': UnsupportedAlgorithm, 'encoding': Encoding,
            'public_format': PublicFormat, 'backend': backend.openssl_version_text()}


def backend_identity():
    backend = _crypto()
    return {'python': sys.version.split()[0], 'distributions': backend['pins'],
            'selected_backend': backend['selected'], 'backend': backend['backend'],
            'profile': PROFILE}


def synthetic_public(seed):
    backend = _crypto()
    if backend['selected'] == 'libsodium':
        return bytes(backend['private'](seed).verify_key)
    try:
        return backend['private'].from_private_bytes(seed).public_key().public_bytes(
            backend['encoding'].Raw, backend['public_format'].Raw)
    except (ValueError, backend['unsupported']):
        raise WitnessError('BackendUnavailable') from None


def _sign(seed, message):
    backend = _crypto()
    if backend['selected'] == 'libsodium':
        return backend['private'](seed).sign(message).signature
    try:
        return backend['private'].from_private_bytes(seed).sign(message)
    except backend['unsupported']:
        raise WitnessError('BackendUnavailable') from None


def _verify(public, signature, message):
    backend = _crypto()
    try:
        if backend['selected'] == 'libsodium':
            backend['public'](public).verify(message, signature)
        else:
            backend['public'].from_public_bytes(public).verify(signature, message)
    except (backend['invalid'], ValueError):
        return False
    return True


def key_id(public):
    return 'ed25519:' + hashlib.sha256(public).hexdigest()


class _Reader:
    def __init__(self, data):
        self.data, self.cursor = data, 0

    def byte(self):
        if self.cursor == len(self.data):
            raise WitnessError('MalformedCbor')
        value = self.data[self.cursor]
        self.cursor += 1
        return value

    def head(self):
        first = self.byte()
        major, info = first >> 5, first & 31
        if info < 24:
            return major, info
        if info not in (24, 25, 26, 27):
            raise WitnessError('MalformedCbor')
        count, minimum = {24: (1, 24), 25: (2, 256), 26: (4, 65536), 27: (8, 2**32)}[info]
        value = 0
        for _ in range(count):
            value = (value << 8) | self.byte()
        if value < minimum:
            raise WitnessError('NonCanonicalEncoding')
        return major, value

    def expect(self, major, value, code='MalformedCbor'):
        if self.head() != (major, value):
            raise WitnessError(code)

    def bytes(self, cap):
        major, count = self.head()
        if major != 2:
            raise WitnessError('MalformedCbor')
        if count > cap:
            raise WitnessError('ResourceLimit')
        end = self.cursor + count
        if end > len(self.data):
            raise WitnessError('MalformedCbor')
        value = self.data[self.cursor:end]
        self.cursor = end
        return value

    def finish(self):
        if self.cursor != len(self.data):
            raise WitnessError('MalformedCbor')


def _bstr(value):
    size = len(value)
    if size < 24:
        prefix = bytes([0x40 + size])
    elif size <= 255:
        prefix = bytes([0x58, size])
    elif size <= 65535:
        prefix = b'\x59' + size.to_bytes(2, 'big')
    else:
        prefix = b'\x5a' + size.to_bytes(4, 'big')
    return prefix + value


def _payload_origin(payload):
    if len(payload) > MAX_PAYLOAD:
        raise WitnessError('ResourceLimit')
    try:
        return decode_complete(payload)['fixture']['origin']['signer']
    except CompleteDecodeError as error:
        resources = ('InputLimit', 'LengthOutOfRange', 'CollectionLimit',
                     'ValueLimit', 'NestingLimit', 'StringLimit', 'ParentLimit', 'AllocationFailure')
        if error.code.split(':')[-1] in resources:
            raise WitnessError('ResourceLimit') from None
        raise WitnessError('InvalidPayload') from None


def _structure(protected, payload):
    # Check a conservative upper bound before allocating the signed message.
    if 12 + 5 + len(protected) + 5 + len(PROFILE) + 5 + len(payload) > MAX_STRUCTURE:
        raise WitnessError('ResourceLimit')
    return b'\x84\x6aSignature1' + _bstr(protected) + _bstr(PROFILE.encode('ascii')) + _bstr(payload)


def parse_p1(container, payload):
    if len(container) > MAX_CONTAINER or len(payload) > MAX_PAYLOAD or len(container) + len(payload) > 1_048_832:
        raise WitnessError('ResourceLimit')
    reader = _Reader(container)
    reader.expect(6, 18)
    reader.expect(4, 4)
    protected = reader.bytes(MAX_PROTECTED)
    reader.expect(5, 0)
    if reader.byte() != 0xf6:
        raise WitnessError('MalformedCbor')
    signature = reader.bytes(64)
    if len(signature) != 64:
        raise WitnessError('InvalidLength')
    reader.finish()
    header = _Reader(protected)
    header.expect(5, 2, 'InvalidHeaderSet')
    header.expect(0, 1, 'InvalidHeaderSet')
    if header.head() != (1, 18):
        raise WitnessError('UnsupportedAlgorithm')
    header.expect(0, 4, 'InvalidHeaderSet')
    kid = header.bytes(72)
    if len(kid) != 72:
        raise WitnessError('InvalidLength')
    header.finish()
    try:
        kid = kid.decode('ascii')
    except UnicodeDecodeError:
        raise WitnessError('InvalidHeaderSet') from None
    if not re.fullmatch(r'ed25519:[0-9a-f]{64}', kid):
        raise WitnessError('InvalidHeaderSet')
    origin = _payload_origin(payload)
    return {'protected': protected, 'signature': signature, 'kid': kid,
            'origin': origin, 'sig_structure': _structure(protected, payload)}


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise WitnessError('RequestError')
        result[key] = value
    return result


def _hex(value, cap, exact=None):
    if type(value) is not str:
        raise WitnessError('RequestError')
    if len(value) > 2 * cap:
        raise WitnessError('ResourceLimit')
    if len(value) % 2 or re.fullmatch(r'[0-9a-f]*', value) is None:
        raise WitnessError('RequestError')
    if exact is not None and len(value) != exact * 2:
        raise WitnessError('InvalidLength')
    return bytes.fromhex(value)


def request(raw):
    try:
        if len(raw) > MAX_REQUEST:
            raise WitnessError('ResourceLimit')
        try:
            value = json.loads(raw, object_pairs_hook=_pairs)
        except (ValueError, UnicodeError, RecursionError):
            raise WitnessError('RequestError') from None
        if type(value) is not dict:
            raise WitnessError('RequestError')
        op = value.get('op')
        fields = {'sign': {'op', 'payload_hex', 'synthetic_seed_hex'},
                  'verify': {'op', 'container_hex', 'payload_hex', 'public_key_hex'}}
        if type(op) is not str or op not in fields or set(value) != fields[op]:
            raise WitnessError('RequestError')
        payload = _hex(value['payload_hex'], MAX_PAYLOAD)
        _crypto()  # Explicit selected dependency must be available; no fallback.
        if op == 'sign':
            seed = _hex(value['synthetic_seed_hex'], 32, 32)
            public = synthetic_public(seed)
            kid = key_id(public)
            if _payload_origin(payload) != kid:
                raise WitnessError('KeyIdentityMismatch')
            protected = b'\xa2\x01\x32\x04' + _bstr(kid.encode('ascii'))
            structure = _structure(protected, payload)
            signature = _sign(seed, structure)
            container = b'\xd2\x84' + _bstr(protected) + b'\xa0\xf6' + _bstr(signature)
            return {'ok': True, 'container_hex': container.hex(), 'public_key_hex': public.hex(),
                    'sig_structure_hex': structure.hex()}
        public = _hex(value['public_key_hex'], 32, 32)
        parsed = parse_p1(_hex(value['container_hex'], MAX_CONTAINER), payload)
        if parsed['kid'] != parsed['origin'] or parsed['kid'] != key_id(public):
            return {'ok': True, 'valid': False, 'reason': 'KeyIdentityMismatch'}
        if not _verify(public, parsed['signature'], parsed['sig_structure']):
            return {'ok': True, 'valid': False, 'reason': 'InvalidSignature'}
        return {'ok': True, 'valid': True, 'reason': None}
    except WitnessError as error:
        return {'ok': False, 'error': error.code}
    except MemoryError:
        return {'ok': False, 'error': 'ResourceLimit'}
    except Exception:
        # An unexpected import/native failure is not a signature refusal.
        # Preserve a hard failure without leaking backend diagnostic content.
        return {'ok': False, 'error': 'BackendUnavailable'}


def backend_probe():
    try:
        accepted = _verify(b'\x01' + bytes(31), b'\x01' + bytes(63), b'probe')
        return {**backend_identity(), 'raw_weak_key_accepted': accepted,
                'strict_parity_qualified': False,
                'qualification_scope': 'single known disagreement probe only; no full qualification'}
    except WitnessError:
        raise
    except Exception:
        raise WitnessError('BackendUnavailable') from None


def run_request(source, sink):
    raw = source.read(MAX_REQUEST + 1)
    response = request(raw)
    sink.write((json.dumps(response, separators=(',', ':')) + '\n').encode())
    return 2 if response.get('error') in ('DependencyUnavailable', 'DependencyVersionMismatch', 'BackendUnavailable') else 0


def main(argv):
    if argv == ['request']:
        return run_request(sys.stdin.buffer, sys.stdout.buffer)
    if argv in (['backend-probe'], ['backend-probe', '--qualify']):
        try:
            probe = backend_probe()
        except WitnessError as error:
            print(json.dumps({'ok': False, 'error': error.code}))
            return 2
        print(json.dumps(probe, sort_keys=True))
        return int('--qualify' in argv and not probe['strict_parity_qualified'])
    return 2


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
