"""Bounded ingress preparation only; does not decode or validate CBOR episodes.

The helpers deliberately have no command-line success protocol. Limits bound
individual parsing stages, not peak process memory or recovery from global OOM.
"""
from __future__ import annotations

import json
from collections.abc import Iterator

from abbey_cbor_reader_v1 import DEFAULTS

MAX_CBOR_BYTES = 1_048_576
MAX_LINE_BYTES = 2 * MAX_CBOR_BYTES + 4096
MAX_JSON_DEPTH = 64


class IngressError(ValueError):
    """A static, content-free ingress rejection category."""

    def __init__(self, code: str):
        if code not in ('RequestError', 'IngressLimit', 'Canonical:InputLimit',
                        'AllocationFailure'):
            raise ValueError('invalid ingress error category')
        self.code = code
        super().__init__(code)


def iter_lines(source) -> Iterator[bytes | IngressError]:
    """Yield bounded logical lines, draining each oversized line independently."""
    try:
        while True:
            chunk = source.readline(MAX_LINE_BYTES + 1)
            if not chunk:
                return
            if len(chunk) <= MAX_LINE_BYTES:
                yield chunk
                continue
            yield IngressError('IngressLimit')
            if not chunk.endswith(b'\n'):
                while True:
                    chunk = source.readline(4096)
                    if not chunk or chunk.endswith(b'\n'):
                        break
    except MemoryError:
        # Reading failed: the next logical-line boundary is unknown.
        yield IngressError('AllocationFailure')


def _effective_limits(limits):
    if limits is not None and type(limits) is not dict:
        raise IngressError('RequestError')
    effective = DEFAULTS.copy()
    if limits is not None:
        for key, value in limits.items():
            if key not in DEFAULTS or type(value) is not int or not 0 <= value <= 2**63 - 1:
                raise IngressError('RequestError')
            effective[key] = value
    effective['max_input_bytes'] = min(effective['max_input_bytes'], MAX_CBOR_BYTES)
    return effective


def prepare_raw(data: bytes, limits: dict | None = None) -> tuple[bytes, dict[str, int]]:
    """Validate raw input type and limits before any CBOR reader is invoked."""
    try:
        if type(data) is not bytes:
            raise IngressError('RequestError')
        effective = _effective_limits(limits)
        if len(data) > effective['max_input_bytes']:
            raise IngressError('Canonical:InputLimit')
        return data, effective
    except MemoryError:
        raise IngressError('AllocationFailure') from None


def _check_depth(text):
    depth = 0
    in_string = False
    escaped = False
    for char in text:
        if in_string:
            if escaped:
                escaped = False
            elif char == '\\':
                escaped = True
            elif char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char in '[{':
            depth += 1
            if depth > MAX_JSON_DEPTH:
                raise IngressError('RequestError')
        elif char in ']}':
            depth -= 1
    # Syntax, matching delimiters and complete strings belong to json.loads.


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise IngressError('RequestError')
        result[key] = value
    return result


def _reject_number(_token):
    raise IngressError('RequestError')


def _parse_int(token):
    digits = len(token) - (1 if token.startswith('-') else 0)
    if digits > 20:
        raise IngressError('RequestError')
    return int(token)


def _unhex(text):
    return bytes.fromhex(text)


def parse_request(line: bytes) -> tuple[bytes, dict[str, int]]:
    """Prepare a single exact-shape JSON request without parsing its CBOR."""
    try:
        if type(line) is not bytes:
            raise IngressError('RequestError')
        if len(line) > MAX_LINE_BYTES:
            raise IngressError('IngressLimit')
        text = line.decode('utf-8')
        _check_depth(text)
        request = json.loads(text, object_pairs_hook=_object,
                             parse_constant=_reject_number,
                             parse_float=_reject_number, parse_int=_parse_int)
        if type(request) is not dict or not {'cbor_hex'} <= request.keys() <= {'cbor_hex', 'limits'}:
            raise IngressError('RequestError')
        # An explicit null limits value is not an optional omitted object.
        if 'limits' in request and type(request['limits']) is not dict:
            raise IngressError('RequestError')
        limits = _effective_limits(request.get('limits'))
        encoded = request['cbor_hex']
        if type(encoded) is not str or len(encoded) % 2 or any(
                char not in '0123456789abcdefABCDEF' for char in encoded):
            raise IngressError('RequestError')
        if len(encoded) // 2 > limits['max_input_bytes']:
            raise IngressError('Canonical:InputLimit')
        return prepare_raw(_unhex(encoded), limits)
    except MemoryError:
        raise IngressError('AllocationFailure') from None
    except (UnicodeError, json.JSONDecodeError, RecursionError):
        raise IngressError('RequestError') from None
