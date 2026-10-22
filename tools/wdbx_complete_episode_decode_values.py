"""Private typed tree inverse only, not full validation or an admission API.

Input must already have passed the bounded canonical syntax reader. List order
and ordinary duplicates are retained for the later relational/order validator.
No encoder, digest, JSON ingress or command-line success protocol lives here.
"""
from __future__ import annotations

import re

from abbey_cbor_episode_v1 import Array, Bytes, Map, Negative, Text, Unsigned


class ValueDecodeError(ValueError):
    """A fixed, content-free mapping rejection."""

    def __init__(self, code: str):
        allowed = (
            'InvalidShape', 'UnsupportedDomain', 'UnsupportedVersion',
            'UnknownVariant', 'InvalidValue:InvalidIdentifier',
            'InvalidValue:InvalidUuid', 'InvalidValue:InvalidRange',
            'InvalidValue:CollectionLimit', 'InvalidValue:InvalidPosterior',
        )
        if type(code) is not str or code not in allowed:
            raise ValueError('invalid value error category')
        self.code = code
        super().__init__(code)


def _fail(code='InvalidShape'):
    raise ValueDecodeError(code)


def _text(value):
    if type(value) is not Text or type(value.value) is not str:
        _fail()
    return value.value


def _unsigned(value):
    if (type(value) is not Unsigned or type(value.value) is not int
            or not 0 <= value.value < 1 << 64):
        _fail()
    return value.value


def _signed(value):
    if type(value) is Unsigned:
        result = _unsigned(value)
        if result >= 1 << 63:
            _fail()
        return result
    if (type(value) is not Negative or type(value.value) is not int
            or not -(1 << 63) <= value.value < 0):
        _fail()
    return value.value


def _token(value):
    result = _text(value)
    if re.fullmatch(r'[A-Za-z0-9._:-]{1,128}', result) is None:
        _fail('InvalidValue:InvalidIdentifier')
    return result


def _signer(value):
    result = _text(value)
    if re.fullmatch(r'ed25519:[0-9a-f]{64}', result) is None:
        _fail('InvalidValue:InvalidIdentifier')
    return result


def _fixed(value, size):
    if (type(value) is not Bytes or type(value.value) is not bytes
            or len(value.value) != size):
        _fail()
    return value.value.hex()


def _digest(value):
    return _fixed(value, 32)


def _uuid(value):
    result = _fixed(value, 16)
    raw = value.value
    if not any(raw) or raw[8] & 192 != 128 or raw[6] >> 4 not in (4, 7):
        _fail('InvalidValue:InvalidUuid')
    return result


def _ppm(value):
    result = _unsigned(value)
    if result >= 1 << 32:
        _fail()
    if result > 1_000_000:
        _fail('InvalidValue:InvalidRange')
    return result


def _array(value):
    if type(value) is not Array or type(value.items) is not list:
        _fail()
    return value.items


def _list(value, cap, mapper):
    values = _array(value)
    if len(values) > cap:
        _fail('InvalidValue:CollectionLimit')
    return [mapper(child) for child in values]


def _sequence(cap, mapper):
    return lambda value: _list(value, cap, mapper)


def _fields(value, required, optional=(), root=False):
    if type(value) is not Map or type(value.entries) is not list:
        _fail()
    if not len(required) <= len(value.entries) <= len(required) + len(optional):
        _fail()
    fields = {}
    for entry in value.entries:
        if type(entry) not in (tuple, list) or len(entry) != 2:
            _fail()
        key, child = entry
        key = _unsigned(key) if root else _text(key)
        if key in fields or (key not in required and key not in optional):
            _fail()
        fields[key] = child
    if any(key not in fields for key in required):
        _fail()
    return fields


def _group(value, required, optional=None):
    optional = {} if optional is None else optional
    fields = _fields(value, required, optional)
    # Output owns all mutable containers; only immutable leaf payloads are shared.
    return {key: mapper(fields[key])
            for key, mapper in (*required.items(), *optional.items()) if key in fields}


def _unit(value, names):
    values = _array(value)
    if len(values) != 1:
        _fail()
    tag = _unsigned(values[0])
    if tag >= len(names):
        _fail('UnknownVariant')
    return names[tag]


def _enum(names):
    return lambda value: _unit(value, names)


_DOMAIN = _enum(('CompleteV1', 'LegacyEpisodeV1'))
_ROUTE = _enum(('NoModel', 'LocalModel', 'CloudModel', 'Specialist', 'Deterministic'))
_PHASE = _enum(('Proposed', 'Performed'))
_UNIT = _enum(('Unitless', 'Count', 'Milliseconds', 'Bytes', 'Microjoules'))
_RESOURCE = _enum(('Tokens', 'CpuMicros', 'MemoryBytes', 'NetworkBytes', 'EnergyMicrojoules'))
_ARTIFACT = _enum(('StateSummary', 'Prediction', 'Outcome', 'Observation', 'Model',
                   'Calibration', 'Provenance', 'ReservoirEmbedding', 'Evidence', 'Authority'))
_SAFETY = _enum(('Unknown', 'NoViolationObserved', 'InterlockTriggered', 'ViolationObserved'))
_ROLLBACK = _enum(('NotApplicable', 'NotAttempted', 'Attempted', 'ExactRestoreObserved',
                   'Failed', 'Unknown'))
_RELATION = _enum(('Supersedes', 'Contradicts', 'Revokes', 'Quarantines', 'Resolves',
                   'Redacts', 'Forgets'))
_DIMENSIONS = ('cryptographic', 'source', 'calibration', 'outcome', 'compatibility',
               'regime', 'lifecycle', 'confidence')


def _variant(value):
    values = _array(value)
    if len(values) != 2:
        _fail()
    tag = _unsigned(values[0])
    if type(values[1]) is not Map:
        _fail()
    return tag, values[1]


def _scope(value):
    tag, body = _variant(value)
    shapes = (('Guild', ('guild',)), ('GuildMember', ('guild', 'member')),
              ('DirectMessage', ('user',)), ('Organization', ('organization',)))
    if tag >= len(shapes):
        _fail('UnknownVariant')
    name, keys = shapes[tag]
    return {'variant': name, **_group(body, {key: _token for key in keys})}


def _binding(value):
    return _group(value, {'tenant': _token, 'scope': _scope})


def _version(value):
    return _group(value, {'component': _token, 'revision': _token, 'digest': _digest})


def _local(value):
    return _group(value, {'binding': _binding, 'domain': _DOMAIN, 'digest': _digest})


def _artifact(value):
    return _group(value, {'binding': _binding, 'digest': _digest,
                         'byte_length': _unsigned, 'kind': _ARTIFACT, 'format': _version})


def _foreign(value):
    return _group(value, {'origin_organization': _token, 'origin_binding': _binding,
                         'domain': _DOMAIN, 'digest': _digest, 'import_receipt': _local})


def _consulted(value):
    tag, body = _variant(value)
    if tag > 1:
        _fail('UnknownVariant')
    return {'variant': ('Local', 'Foreign')[tag],
            **_group(body, {'reference': (_local, _foreign)[tag]})}


def _estimate(value, mapper):
    values = _array(value)
    if not values:
        _fail()
    tag = _unsigned(values[0])
    if tag == 0 and len(values) == 1:
        return {'unknown': True}
    if tag == 1 and len(values) == 2:
        return {'known': mapper(values[1])}
    _fail()


def _estimated(mapper):
    return lambda value: _estimate(value, mapper)


def _posterior_entry(value):
    values = _array(value)
    if len(values) != 2:
        _fail()
    return [_token(values[0]), _ppm(values[1])]


def _posterior(value):
    values = _array(value)
    if not 1 <= len(values) <= 32:
        _fail('InvalidValue:InvalidPosterior')
    result = [_posterior_entry(child) for child in values]
    if len({pair[0] for pair in result}) != len(result) or sum(p[1] for p in result) != 1_000_000:
        _fail('InvalidValue:InvalidPosterior')
    return result


def _actor_kind(value):
    name = _text(value)
    names = {'human_subject': 'HumanSubject', 'organization_owner': 'OrganizationOwner',
             'guild_owner': 'GuildOwner', 'guild_administrator': 'GuildAdministrator',
             'guild_manager': 'GuildManager', 'service': 'Service'}
    if name not in names:
        _fail('UnknownVariant')
    return names[name]


def _actor(value):
    return _group(value, {'principal': _token, 'kind': _actor_kind})


def _resource(value):
    return _group(value, {'resource': _RESOURCE, 'amount': _unsigned})


def _action(value):
    return _group(value, {'capability': _version, 'action': _token,
                         'parameter_classes': _sequence(32, _token), 'phase': _PHASE})


def _measurement(value):
    result = _group(value, {'unit': _UNIT, 'value': _signed})
    if result['unit'] in ('Count', 'Bytes') and (result['value'] < 0 or result['value'] % 1_000_000):
        _fail('InvalidValue:InvalidRange')
    return result


def _monotonic(value):
    result = _group(value, {'clock': _token, 'start': _unsigned, 'end': _unsigned})
    if result['start'] > result['end']:
        _fail('InvalidValue:InvalidRange')
    return result


def _consent(value):
    return _group(value, {'binding': _binding, 'epoch': _unsigned,
                         'policy': _version, 'attestation': _local})


def _deletion_key(value):
    return _group(value, {'binding': _binding, 'key_id': _token})


def _edge(value):
    return _group(value, {'relation': _RELATION, 'target': _local, 'basis': _local})


def _retention(value):
    values = _array(value)
    if not 1 <= len(values) <= 2:
        _fail()
    tag = _unsigned(values[0])
    if len(values) == 2 and type(values[1]) is not Map:
        _fail()
    if tag > 4:
        _fail('UnknownVariant')
    if tag == 0:
        if len(values) != 1:
            _fail()
        return {'variant': 'Ephemeral'}
    if len(values) != 2:
        _fail()
    name, required = (
        ('Session', {'session': _token, 'expires_at': _unsigned}),
        ('Operational', {'expires_at': _unsigned}),
        ('Durable', {'privacy_basis': _local}),
        ('MandatoryIncident', {'obligation': _local}),
    )[tag - 1]
    return {'variant': name, **_group(values[1], required)}


def _maturity(value):
    result = _text(value)
    if result not in ('C0', 'C1', 'C2', 'C3', 'C4', 'C5', 'C6', 'C7'):
        _fail('UnknownVariant')
    return result


def _identity(value):
    return _group(value, {'episode_id': _uuid, 'request_id': _uuid,
                         'binding': _binding, 'contract': _version})


def _time(value):
    return _group(value, {'acquired_at': _unsigned, 'received_at': _unsigned},
                  {'monotonic': _monotonic})


def _origin(value):
    return _group(value, {'source_node': _token, 'actor': _actor, 'signer': _signer})


def _context(value):
    return _group(value, {'task_id': _uuid, 'regime': _token,
                         'regime_posterior': _estimated(_posterior),
                         'habituation': _estimated(_ppm), 'resources': _sequence(16, _resource)},
                  {'summary': _artifact, 'reservoir': _artifact})


def _decision(value):
    return _group(value, {'route': _ROUTE, 'prediction': _estimated(_artifact),
                         'actions': _sequence(128, _action), 'uncertainty': _estimated(_ppm),
                         'risk': _estimated(_ppm), 'novelty': _estimated(_ppm)})


def _outcome(value):
    return _group(value, {'observed': _estimated(_artifact), 'utility': _estimated(_measurement),
                         'safety_effect': _SAFETY, 'costs': _sequence(16, _resource),
                         'rollback': _ROLLBACK})


def _authority(value):
    return _group(value, {'policy': _version, 'safety_policy': _version,
                         'capabilities': _sequence(32, _version), 'models': _sequence(32, _version),
                         'calibrations': _sequence(32, _version), 'approvals': _sequence(64, _local)},
                  {'consent': _consent})


def _dimensions(value):
    return _group(value, {key: _estimated(_artifact) for key in _DIMENSIONS})


def _evidence(value):
    return _group(value, {'maturity': _maturity, 'source_refs': _sequence(256, _artifact),
                         'observation_refs': _sequence(256, _artifact), 'dimensions': _dimensions})


def _lifecycle(value):
    return _group(value, {'retention': _retention, 'holds': _sequence(64, _local),
                         'edges': _sequence(64, _edge)}, {'deletion_key': _deletion_key})


def _provenance(value):
    return _group(value, {'causal_parents': _sequence(64, _local),
                         'consulted_parents': _sequence(256, _consulted),
                         'records': _sequence(256, _artifact)})


def map_complete(root) -> dict:
    """Invert a bounded canonical value tree, retaining stage-only invalid values.

    Relational constraints, ordinary duplicate rejection and semantic set order
    are intentionally deferred. A successful return alone is not a valid episode.
    """
    fields = _fields(root, tuple(range(12)), root=True)
    if _text(fields[0]) != 'wdbx-complete-episode-v1':
        _fail('UnsupportedDomain')
    if _unsigned(fields[1]) != 1:
        _fail('UnsupportedVersion')
    groups = (('identity', _identity), ('time', _time), ('origin', _origin),
              ('context', _context), ('decision', _decision), ('outcome', _outcome),
              ('authority', _authority), ('evidence', _evidence),
              ('lifecycle', _lifecycle), ('provenance', _provenance))
    return {name: mapper(fields[index]) for index, (name, mapper) in enumerate(groups, 2)}
