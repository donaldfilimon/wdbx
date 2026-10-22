"""Bounded independent profile reader; syntax witness only, never admission.

Request limits are unsigned integers up to 2**63-1, a portable policy ceiling.
All five omitted limits use defaults. Invalid requests return RequestError;
individual rejections do not change the JSON-lines command's zero exit status.
"""
from __future__ import annotations

import json
import sys

DEFAULTS = dict(max_input_bytes=1048576, max_value_count=65536,
                max_collection_entries=16384, max_string_bytes=1048576,
                max_parent_count=4096)


class DecodeError(ValueError):
    """Content-free category shared with the Rust reader."""


def fail(category):
    raise DecodeError(category)


class Reader:
    def __init__(self, data, limits, model, episode):
        self.data, self.limits, self.model, self.episode = data, limits, model, episode
        self.cursor = 0
        self.remaining = limits['max_value_count']

    def take(self, size):
        if size > (1 << 64) - 1 - self.cursor:
            fail('LengthOutOfRange')
        if size > len(self.data) - self.cursor:
            fail('UnexpectedEnd')
        start = self.cursor
        self.cursor += size
        return self.data[start:self.cursor]

    def argument(self, info):
        if info < 24:
            return info
        if info > 27:
            fail('InvalidAdditionalInformation')
        size = 1 << (info - 24)
        value = int.from_bytes(self.take(size), 'big')
        if value < (24, 256, 65536, 4294967296)[info - 24]:
            fail('NonMinimalArgument')
        return value

    def collection(self, size, cost):
        if size > self.limits['max_collection_entries']:
            fail('CollectionLimit')
        children = size * cost
        if children > (1 << 64) - 1:
            fail('LengthOutOfRange')
        if children > self.remaining:
            fail('ValueLimit')
        if children > len(self.data) - self.cursor:
            fail('UnexpectedEnd')

    def value(self, depth=0, parents=False):
        if depth > 32:
            fail('NestingLimit')
        if self.remaining == 0:
            fail('ValueLimit')
        self.remaining -= 1
        initial = self.take(1)[0]
        major, info = initial >> 5, initial & 31
        m = self.model
        if major == 7:
            if initial == 244:
                return m.Bool(False)
            if initial == 245:
                return m.Bool(True)
            if initial == 246:
                return m.NULL
            fail('UnsupportedType')
        if major == 6:
            fail('UnsupportedType')
        size = self.argument(info)
        if major == 0:
            return m.Unsigned(size)
        if major == 1:
            if size > (1 << 63) - 1:
                fail('IntegerOutOfRange')
            return m.Negative(-1 - size)
        if major in (2, 3):
            if size > self.limits['max_string_bytes']:
                fail('StringLimit')
            raw = self.take(size)
            if major == 2:
                return m.Bytes(raw)
            try:
                return m.Text(raw.decode('utf-8'))
            except UnicodeDecodeError:
                fail('InvalidUtf8')
        if major == 4:
            if parents and size > self.limits['max_parent_count']:
                fail('ParentLimit')
            self.collection(size, 1)
            return m.Array([self.value(depth + 1) for _ in range(size)])
        if major == 5:
            self.collection(size, 2)
            entries, previous = [], None
            for _ in range(size):
                start = self.cursor
                key = self.value(depth + 1)
                # Raw input slices keep compound keys faithful to the wire.
                raw = memoryview(self.data)[start:self.cursor]
                if previous is not None:
                    if len(previous) == len(raw) and previous == raw:
                        fail('DuplicateMapKey')
                    if len(previous) > len(raw) or (len(previous) == len(raw) and bytes(previous) > bytes(raw)):
                        fail('MapKeyOrder')
                previous = raw
                parent_array = self.episode and depth == 0 and isinstance(key, m.Unsigned) and key.value == 4
                entries.append((key, self.value(depth + 1, parent_array)))
            return m.Map(entries)
        fail('UnsupportedType')


def decode(data, limits, model, episode):
    if len(data) > limits['max_input_bytes']:
        fail('InputLimit')
    reader = Reader(data, limits, model, episode)
    root = reader.value()
    if reader.cursor != len(data):
        fail('TrailingBytes')
    if not episode:
        return model.encode_value(root)
    m = model
    if not isinstance(root, m.Map) or len(root.entries) != 5:
        fail('InvalidEnvelope')
    values = []
    # Sequential shape checks preserve static error categories.
    for expected, (key, value) in enumerate(root.entries):
        if not isinstance(key, m.Unsigned) or key.value != expected:
            fail('InvalidEnvelope')
        if expected == 0 and (not isinstance(value, m.Text) or value.value != m.PROFILE_NAME):
            fail('ProfileMismatch')
        if expected == 1:
            if not isinstance(value, m.Unsigned):
                fail('InvalidEnvelope')
            if value.value == 0:
                fail('ZeroSchemaVersion')
        if expected == 2 and not isinstance(value, m.Map):
            fail('HeaderMustBeMap')
        if expected == 3 and not isinstance(value, m.Map):
            fail('PayloadMustBeMap')
        values.append(value)
    if not isinstance(values[4], m.Array):
        fail('InvalidEnvelope')
    parents = []
    for parent in values[4].items:
        if not isinstance(parent, m.Bytes) or len(parent.value) != 32:
            fail('InvalidParentDigest')
        if parents and parents[-1] > parent.value:
            fail('ParentOrder')
        parents.append(parent.value)
    return m.envelope_bytes(values[1].value, values[2], values[3], parents)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError()
        result[key] = value
    return result


def reject_constant(_):
    raise ValueError()


def request(line, model):
    try:
        obj = json.loads(line, object_pairs_hook=unique_object, parse_constant=reject_constant)
        if not isinstance(obj, dict) or set(obj) not in ({'cbor_hex', 'kind'}, {'cbor_hex', 'kind', 'limits'}):
            raise ValueError()
        if obj['kind'] not in ('value', 'episode'):
            raise ValueError()
        text = obj['cbor_hex']
        if not isinstance(text, str) or len(text) % 2 or any(c not in '0123456789abcdefABCDEF' for c in text):
            raise ValueError()
        limits = DEFAULTS.copy()
        supplied = obj.get('limits', {})
        if not isinstance(supplied, dict) or any(key not in limits for key in supplied):
            raise ValueError()
        if any(type(value) is not int or not 0 <= value <= (1 << 63) - 1 for value in supplied.values()):
            raise ValueError()
        limits.update(supplied)
    except (ValueError, TypeError, RecursionError):
        return {'ok': False, 'error': 'RequestError'}
    try:
        if len(text) // 2 > limits['max_input_bytes']:
            fail('InputLimit')
        data = bytes.fromhex(text)
        encoded = decode(data, limits, model, obj['kind'] == 'episode')
        if encoded != data:
            raise RuntimeError('reader reencoding mismatch')
        return {'ok': True, 'cbor_hex': encoded.hex()}
    except DecodeError as error:
        return {'ok': False, 'error': str(error)}
    except MemoryError:
        return {'ok': False, 'error': 'AllocationFailure'}


def decode_corpus(model):
    for line in sys.stdin:
        sys.stdout.write(json.dumps(request(line, model), separators=(',', ':')) + '\n')
        sys.stdout.flush()
    return 0
