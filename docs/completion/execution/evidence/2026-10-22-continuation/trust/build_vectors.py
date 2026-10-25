#!/usr/bin/env python3
"""Independent stdlib deterministic CBOR witness; no signature cryptography.

Published RFC 8032 example public keys are synthetic vector inputs, not custody
material. Malformed cases are typed-API refusals, not command-wire parser tests.
"""
import hashlib
import json
from pathlib import Path

def head(major, value):
    if value < 24:
        return bytes([(major << 5) | value])
    for width, tag in [(1, 24), (2, 25), (4, 26), (8, 27)]:
        if value < 1 << (width * 8):
            return bytes([(major << 5) | tag]) + value.to_bytes(width, 'big')
    raise ValueError('integer bound')

def cbor(value):
    if value is None:
        return b'\xf6'
    if isinstance(value, int):
        return head(0, value) if value >= 0 else head(1, -value - 1)
    if isinstance(value, bytes):
        return head(2, len(value)) + value
    if isinstance(value, str):
        raw = value.encode('utf-8')
        return head(3, len(raw)) + raw
    if isinstance(value, list):
        return head(4, len(value)) + b''.join(cbor(v) for v in value)
    if isinstance(value, dict):
        items = sorted(((cbor(k), cbor(v)) for k, v in value.items()), key=lambda x: (len(x[0]), x[0]))
        return head(5, len(items)) + b''.join(k + v for k, v in items)
    raise ValueError('unsupported fixture type')

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def vectors():
    namespace, admin, profile = 'synthetic-local', 'synthetic-admin', 'wdbx-complete-cose-sign1-prototype-v1'
    publics = [bytes.fromhex(s) for s in [
        'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
        '3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c']]
    ids = ['ed25519:' + digest(public) for public in publics]
    grants = [cbor({'tenant': 'synthetic-tenant', 'scope': [0, {'guild': 'a'}]})]
    registrations = [[ids[i], publics[i], 'writer', 0, profile, sorted(grants)] for i in range(2)]
    operations = [[0] + registrations[0], [1, ids[0]], [0] + registrations[1], [4, ids[0], ids[1]], [3, ids[0], 1, None]]
    command_ids = ['register-old', 'activate-old', 'register-new', 'rotate', 'revoke-old']
    keys, history, cases = {}, [], []
    for revision, (cid, operation) in enumerate(zip(command_ids, operations)):
        raw = cbor(['wdbx-trust-command-prototype-v1', namespace, cid, admin, revision, operation])
        history.append(bytes.fromhex(digest(raw)))
        next_revision = revision + 1
        if operation[0] == 0:
            keys[operation[1]] = [operation[1:], 0, next_revision, None]
        elif operation[0] == 1:
            keys[operation[1]][1:3] = [1, next_revision]
        elif operation[0] == 4:
            keys[operation[1]][1:3] = [2, next_revision]
            keys[operation[2]][1:3] = [1, next_revision]
        else:
            keys[operation[1]][1:] = [3, next_revision, [operation[2], operation[3]]]
        snapshot = cbor(['wdbx-p1-trust-snapshot-prototype-v1', namespace, next_revision,
                         [keys[k] for k in sorted(keys)], list(history)])
        cases.append({'command_id': cid, 'expected_revision': revision, 'command_hex': raw.hex(),
                      'command_sha256': digest(raw), 'snapshot_sha256': digest(snapshot)})
    zero = cbor(['wdbx-trust-command-prototype-v1', namespace, 'revoke-old', admin, 4, [3, ids[0], 1, 0]])
    return {'version': 1, 'scope': 'private typed API and deterministic encoding only; no wire parser or authentication qualification',
            'public_keys_hex': [p.hex() for p in publics], 'cases': cases,
            'claimed_zero_command_hex': zero.hex(),
            'typed_rejections': [{'case': c, 'error': e} for c, e in [
                ('duplicate-grants', 'DuplicateGrant'), ('weak-key', 'InvalidKeyRegistration'),
                ('wrong-purpose', 'WrongKeyPurpose'), ('wrong-profile', 'WrongKeyPurpose'),
                ('empty-command-id', 'InvalidIdentifier')]]}

if __name__ == '__main__':
    target = Path(__file__).with_name('commands.json')
    content = json.dumps(vectors(), indent=2) + '\n'
    import sys
    if '--write' in sys.argv:
        target.write_text(content)
    elif target.read_text() != content:
        raise SystemExit('trust vector mismatch')
    else:
        print('Trust command and snapshot vectors agree; encoding only.')
