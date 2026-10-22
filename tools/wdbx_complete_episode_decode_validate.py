"""Private structural relations over reviewed typed synthetic fixture values.

This stage neither parses arbitrary fixtures nor resolves references, policies,
consent, signatures or evidence truth. It preserves input values and their order.
"""
from __future__ import annotations


class RelationalDecodeError(ValueError):
    """Fixed content-free relational rejection, without retained episode data."""

    def __init__(self, code: str):
        allowed = ('CollectionLimit', 'InvalidPosterior', 'InvalidRange',
                   'DuplicateReference', 'ScopeMismatch', 'InvalidArtifactKind',
                   'SelfReference')
        if type(code) is not str or code not in tuple('InvalidValue:' + x for x in allowed):
            raise ValueError('invalid relational error category')
        self.code = code
        super().__init__(code)


def _fail(category):
    raise RelationalDecodeError('InvalidValue:' + category)


def _known(estimate):
    return estimate.get('known')


def _collection_limits(fixture):
    decision = fixture['decision']
    if len(decision['actions']) > 128:
        _fail('CollectionLimit')
    for action in decision['actions']:
        if len(action['parameter_classes']) > 32:
            _fail('CollectionLimit')
    groups = (
        (fixture['context'], ('resources',), 16),
        (fixture['outcome'], ('costs',), 16),
        (fixture['authority'], ('capabilities', 'models', 'calibrations'), 32),
        (fixture['authority'], ('approvals',), 64),
        (fixture['lifecycle'], ('holds', 'edges'), 64),
        (fixture['provenance'], ('causal_parents',), 64),
        (fixture['provenance'], ('consulted_parents', 'records'), 256),
        (fixture['evidence'], ('source_refs', 'observation_refs'), 256),
    )
    for group, names, maximum in groups:
        for name in names:
            if len(group[name]) > maximum:
                _fail('CollectionLimit')


def _unique(values, key=lambda value: value):
    # Structural equality preserves complete typed values and ignores dict key
    # insertion order. Bounds were checked before this quadratic comparison.
    for index, value in enumerate(values):
        if any(key(value) == key(previous) for previous in values[:index]):
            _fail('DuplicateReference')


def _same(binding, expected):
    if binding != expected:
        _fail('ScopeMismatch')


def _walk(fixture, local, artifact):
    """Visit only the declared reference slots in received order; no resolution."""
    for value, kind in (
        (fixture['context'].get('summary'), 'StateSummary'),
        (fixture['context'].get('reservoir'), 'ReservoirEmbedding'),
        (_known(fixture['decision']['prediction']), 'Prediction'),
        (_known(fixture['outcome']['observed']), 'Outcome'),
    ):
        if value is not None:
            artifact(value, kind)
    for name in ('cryptographic', 'source', 'calibration', 'outcome',
                 'compatibility', 'regime', 'lifecycle', 'confidence'):
        value = _known(fixture['evidence']['dimensions'][name])
        if value is not None:
            artifact(value, 'Evidence')
    for group, name, kind in (
        ('evidence', 'source_refs', None),
        ('evidence', 'observation_refs', 'Observation'),
        ('provenance', 'records', 'Provenance'),
    ):
        for value in fixture[group][name]:
            artifact(value, kind)
    for group, name in (('authority', 'approvals'), ('lifecycle', 'holds'),
                        ('provenance', 'causal_parents')):
        for value in fixture[group][name]:
            local(value)
    consent = fixture['authority'].get('consent')
    if consent is not None:
        local(consent['attestation'])
    for edge in fixture['lifecycle']['edges']:
        local(edge['target'])
        local(edge['basis'])
    retention = fixture['lifecycle']['retention']
    if retention['variant'] == 'Durable':
        local(retention['privacy_basis'])
    elif retention['variant'] == 'MandatoryIncident':
        local(retention['obligation'])
    for consulted in fixture['provenance']['consulted_parents']:
        reference = consulted['reference']
        local(reference if consulted['variant'] == 'Local' else reference['import_receipt'])


def _reject_self_reference(fixture, episode_digest):
    digest = episode_digest.hex()

    def local(reference):
        if reference['domain'] == 'CompleteV1' and reference['digest'] == digest:
            _fail('SelfReference')

    def artifact(reference, expected_kind):
        if reference['digest'] == digest:
            _fail('SelfReference')

    _walk(fixture, local, artifact)


def validate_fixture(fixture: dict, episode_digest: bytes) -> None:
    """Check relations on map_complete output against the actual input digest.

    Exact primitive types/shape and leaf constructors are the mapper's contract.
    Direct arbitrary dictionaries are not supported as a second parsing API.
    """
    if type(episode_digest) is not bytes or len(episode_digest) != 32:
        raise ValueError('invalid digest argument')
    _collection_limits(fixture)
    posterior = _known(fixture['context']['regime_posterior'])
    if posterior is not None:
        if (not 1 <= len(posterior) <= 32
                or len({entry[0] for entry in posterior}) != len(posterior)
                or sum(entry[1] for entry in posterior) != 1_000_000):
            _fail('InvalidPosterior')
    monotonic = fixture['time'].get('monotonic')
    if monotonic is not None and monotonic['start'] > monotonic['end']:
        _fail('InvalidRange')
    utility = _known(fixture['outcome']['utility'])
    if (utility is not None and utility['unit'] in ('Count', 'Bytes')
            and (utility['value'] < 0 or utility['value'] % 1_000_000)):
        _fail('InvalidRange')
    retention = fixture['lifecycle']['retention']
    if (retention['variant'] in ('Session', 'Operational')
            and retention['expires_at'] <= fixture['time']['received_at']):
        _fail('InvalidRange')
    for action in fixture['decision']['actions']:
        _unique(action['parameter_classes'])
    for readings in (fixture['context']['resources'], fixture['outcome']['costs']):
        _unique(readings, lambda reading: reading['resource'])
    for name in ('capabilities', 'models', 'calibrations'):
        _unique(fixture['authority'][name], lambda version: version['component'])
    for group, names in (
        ('authority', ('approvals',)),
        ('lifecycle', ('holds', 'edges')),
        ('provenance', ('causal_parents', 'consulted_parents', 'records')),
        ('evidence', ('source_refs', 'observation_refs')),
    ):
        for name in names:
            _unique(fixture[group][name])
    binding = fixture['identity']['binding']
    consent = fixture['authority'].get('consent')
    if consent is not None:
        _same(consent['binding'], binding)
    deletion_key = fixture['lifecycle'].get('deletion_key')
    if deletion_key is not None:
        _same(deletion_key['binding'], binding)
    for consulted in fixture['provenance']['consulted_parents']:
        if consulted['variant'] == 'Foreign':
            reference = consulted['reference']
            scope = reference['origin_binding']['scope']
            if (scope['variant'] == 'Organization'
                    and scope['organization'] != reference['origin_organization']):
                _fail('ScopeMismatch')

    def artifact(reference, expected_kind):
        _same(reference['binding'], binding)
        if reference['byte_length'] > 1 << 40:
            _fail('InvalidRange')
        if expected_kind is not None and reference['kind'] != expected_kind:
            _fail('InvalidArtifactKind')

    _walk(fixture, lambda reference: _same(reference['binding'], binding), artifact)
    _reject_self_reference(fixture, episode_digest)
