#!/usr/bin/env python3
"""Reproduce one synthetic parser vector; no cryptographic verification."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
FIXTURE = 'crates/abi-wdbx/src/v3/complete_episode/tests/provisional-minimal.hex'


def bstr(value):
    length = len(value)
    if length < 24:
        prefix = bytes([0x40 + length])
    elif length < 256:
        prefix = b'\x58' + length.to_bytes(1, 'big')
    elif length < 65536:
        prefix = b'\x59' + length.to_bytes(2, 'big')
    else:
        prefix = b'\x5a' + length.to_bytes(4, 'big')
    return prefix + value


if __name__ == '__main__':
    fixture = (ROOT / FIXTURE).read_bytes()
    payload = bytes.fromhex(fixture.decode().strip())
    kid = b'ed25519:' + b'0' * 64
    protected = b'\xa2\x01\x32\x04' + bstr(kid)
    aad = b'wdbx-complete-cose-sign1-prototype-v1'
    container = b'\xd2\x84' + bstr(protected) + b'\xa0\xf6' + bstr(bytes([7]) * 64)
    structure = b'\x84\x6aSignature1' + bstr(protected) + bstr(aad) + bstr(payload)
    print(json.dumps({
        'qualification': 'synthetic parser vector only; signature not verified',
        'fixture': FIXTURE,
        'fixture_sha256': hashlib.sha256(fixture).hexdigest(),
        'protected_hex': protected.hex(),
        'container_hex': container.hex(),
        'external_aad_hex': aad.hex(),
        'sig_structure_hex': structure.hex(),
        'sig_structure_length': len(structure),
        'sig_structure_sha256': hashlib.sha256(structure).hexdigest(),
    }, indent=2))
