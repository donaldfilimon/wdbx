"""Private bounded complete-episode witness, not admission or signature checking.

Bounds apply to parsing stages, not total process memory. Allocation recovery is
best effort; this does not promise operation under global memory exhaustion.
"""
from __future__ import annotations
import hashlib
import json
import sys

import abbey_cbor_episode_v1 as model
from abbey_cbor_reader_v1 import DecodeError, Reader
import wdbx_complete_episode_draft as encoder
from wdbx_complete_episode_decode_ingress import IngressError, iter_lines, parse_request, prepare_raw
from wdbx_complete_episode_decode_values import ValueDecodeError, map_complete
from wdbx_complete_episode_decode_validate import RelationalDecodeError, validate_fixture

_CANONICAL = ('InputLimit', 'LengthOutOfRange', 'UnexpectedEnd',
              'InvalidAdditionalInformation', 'NonMinimalArgument', 'CollectionLimit',
              'ValueLimit', 'NestingLimit', 'UnsupportedType', 'IntegerOutOfRange',
              'StringLimit', 'InvalidUtf8', 'ParentLimit', 'DuplicateMapKey',
              'MapKeyOrder', 'TrailingBytes')
_ALLOWED = frozenset(('RequestError', 'IngressLimit', 'AllocationFailure',
                      'InvalidShape', 'UnsupportedDomain', 'UnsupportedVersion',
                      'UnknownVariant', 'NonCanonicalEncoding') +
                     tuple('Canonical:' + x for x in _CANONICAL) +
                     tuple('InvalidValue:' + x for x in (
                         'InvalidIdentifier', 'InvalidUuid', 'InvalidRange',
                         'CollectionLimit', 'InvalidPosterior', 'DuplicateReference',
                         'ScopeMismatch', 'InvalidArtifactKind', 'SelfReference')))
_ALLOCATION_RESPONSE = b'{"ok":false,"error":"AllocationFailure"}\n'


class CompleteDecodeError(ValueError):
    """Static rejection category; no source values are retained."""
    def __init__(self, code: str):
        if type(code) is not str or code not in _ALLOWED:
            raise ValueError('invalid complete error category')
        self.code = code
        super().__init__(code)


def decode_complete(data: bytes, limits: dict | None = None) -> dict:
    """Decode original canonical bytes to the private typed fixture projection."""
    try:
        data, effective = prepare_raw(data, limits)
        reader = Reader(data, effective, model, False)
        root = reader.value()
        if reader.cursor != len(data):
            raise CompleteDecodeError('Canonical:TrailingBytes')
        fixture = map_complete(root)
        digest = hashlib.sha256(data).digest()
        validate_fixture(fixture, digest)
        try:
            canonical = encoder.cbor(encoder.map_fixture(fixture))
        except encoder.FixtureError:
            raise RuntimeError('validated fixture encoding invariant failed') from None
        if canonical != data:
            raise CompleteDecodeError('NonCanonicalEncoding')
        return {'fixture': fixture, 'bytes': data.hex(), 'digest': digest.hex()}
    except (IngressError, ValueDecodeError, RelationalDecodeError) as error:
        raise CompleteDecodeError(error.code) from None
    except DecodeError as error:
        raise CompleteDecodeError('Canonical:' + str(error)) from None
    except MemoryError:
        raise CompleteDecodeError('AllocationFailure') from None


def request(line: bytes) -> dict:
    """Return one strict finite-request response with static expected failures."""
    try:
        data, limits = parse_request(line)
        return {'ok': True, **decode_complete(data, limits)}
    except (IngressError, CompleteDecodeError) as error:
        return {'ok': False, 'error': error.code}
    except MemoryError:
        return {'ok': False, 'error': 'AllocationFailure'}


def run_json_lines(source, sink) -> int:
    """Drain bounded logical lines and flush one fully serialized response each."""
    for line in iter_lines(source):
        try:
            response = ({'ok': False, 'error': line.code}
                        if isinstance(line, IngressError) else request(line))
            encoded = (json.dumps(response, separators=(',', ':')) + '\n').encode('utf-8')
        except MemoryError:
            encoded = _ALLOCATION_RESPONSE
        sink.write(encoded)
        sink.flush()
    return 0


def main(argv: list[str]) -> int:
    """Only the private JSON-lines decoding protocol is supported."""
    if argv != ['decode-json-lines']:
        return 2
    return run_json_lines(sys.stdin.buffer, sys.stdout.buffer)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
