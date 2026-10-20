"""Finite relational tests; direct digest seams are not byte fixed points."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import abbey_cbor_episode_v1 as c
from abbey_cbor_reader_v1 import Reader, DEFAULTS
from wdbx_complete_episode_decode_values import map_complete
import wdbx_complete_episode_decode_validate as v

FIXTURES = Path(__file__).resolve().parents[2] / 'crates/abi-wdbx/src/v3/complete_episode/tests'
DIGEST = bytes.fromhex('fe' * 32)
DIMS = ('cryptographic', 'source', 'calibration', 'outcome', 'compatibility',
        'regime', 'lifecycle', 'confidence')
KINDS = ('StateSummary', 'Prediction', 'Outcome', 'Observation', 'Model',
         'Calibration', 'Provenance', 'ReservoirEmbedding', 'Evidence', 'Authority')
LOCAL = (
    ('authority', 'approvals', 0), ('lifecycle', 'holds', 0),
    ('provenance', 'causal_parents', 0), ('authority', 'consent', 'attestation'),
    ('lifecycle', 'edges', 0, 'target'), ('lifecycle', 'edges', 0, 'basis'),
    ('lifecycle', 'retention', 'privacy_basis'), ('lifecycle', 'retention', 'obligation'),
    ('provenance', 'consulted_parents', 0, 'reference'),
    ('provenance', 'consulted_parents', 1, 'reference', 'import_receipt'),
)
ARTIFACT = (
    (('context', 'summary'), 'StateSummary'),
    (('context', 'reservoir'), 'ReservoirEmbedding'),
    (('decision', 'prediction', 'known'), 'Prediction'),
    (('outcome', 'observed', 'known'), 'Outcome'),
    *((('evidence', 'dimensions', key, 'known'), 'Evidence') for key in DIMS),
    (('evidence', 'source_refs', 0), None),
    (('evidence', 'observation_refs', 0), 'Observation'),
    (('provenance', 'records', 0), 'Provenance'),
)
LISTS = (
    (('decision', 'actions'), 128),
    (('decision', 'actions', 0, 'parameter_classes'), 32),
    (('context', 'resources'), 16), (('outcome', 'costs'), 16),
    *((('authority', key), 32) for key in ('capabilities', 'models', 'calibrations')),
    (('authority', 'approvals'), 64), (('lifecycle', 'holds'), 64),
    (('provenance', 'causal_parents'), 64), (('lifecycle', 'edges'), 64),
    (('provenance', 'consulted_parents'), 256),
    (('evidence', 'source_refs'), 256), (('evidence', 'observation_refs'), 256),
    (('provenance', 'records'), 256),
)


def fixture():
    return json.loads((FIXTURES / 'provisional-populated.json').read_text())


def at(value, path):
    for key in path:
        value = value[key]
    return value


def put(value, path, replacement):
    at(value, path[:-1])[path[-1]] = replacement


def for_local(path):
    result = fixture()
    if path[-1] in ('privacy_basis', 'obligation'):
        ref = copy.deepcopy(result['authority']['approvals'][0])
        result['lifecycle']['retention'] = {
            'variant': 'Durable' if path[-1] == 'privacy_basis' else 'MandatoryIncident',
            path[-1]: ref,
        }
    return result


def tree_at(node, path):
    for key in path:
        if isinstance(key, int) and type(node) is c.Array:
            node = node.items[key]
        else:
            node = next(child for item, child in node.entries if item.value == key)
    return node


def replace_tree_field(node, name, replacement):
    for index, (key, value) in enumerate(node.entries):
        if key.value == name:
            node.entries[index] = (key, replacement)
            return
    raise AssertionError('missing test field')


def byte_pipeline(data):
    reader = Reader(data, DEFAULTS.copy(), c, False)
    root = reader.value()
    if reader.cursor != len(data):
        raise AssertionError('test pipeline trailing input')
    result = map_complete(root)
    v.validate_fixture(result, hashlib.sha256(data).digest())
    return result


class RelationalTests(unittest.TestCase):
    def reject(self, suffix, value, digest=DIGEST):
        before = copy.deepcopy(value)
        with self.assertRaises(v.RelationalDecodeError) as caught:
            v.validate_fixture(value, digest)
        code = 'InvalidValue:' + suffix
        self.assertEqual(caught.exception.code, code)
        self.assertEqual(str(caught.exception), code)
        self.assertEqual(value, before)

    def accept(self, value, digest=DIGEST):
        before = copy.deepcopy(value)
        self.assertIsNone(v.validate_fixture(value, digest))
        self.assertEqual(value, before)

    def test_base_digest_arguments_and_error_redaction(self):
        self.accept(fixture())
        for digest in ('private-sensitive', bytearray(32), b'', bytes(31), bytes(33), None):
            with self.subTest(digest_type=type(digest).__name__):
                with self.assertRaisesRegex(ValueError, '^invalid digest argument$'):
                    v.validate_fixture(fixture(), digest)
        for code in ('private-sensitive', '', 'InvalidValue:bad', None, True):
            with self.assertRaisesRegex(ValueError, '^invalid relational error category$'):
                v.RelationalDecodeError(code)
        for suffix in ('CollectionLimit', 'InvalidPosterior', 'InvalidRange',
                       'DuplicateReference', 'ScopeMismatch', 'InvalidArtifactKind', 'SelfReference'):
            error = v.RelationalDecodeError('InvalidValue:' + suffix)
            self.assertEqual(str(error), error.code)

    def test_every_local_slot_scope_and_self_reference(self):
        scopes = ({'variant': 'Guild', 'guild': 'g'},
                  {'variant': 'GuildMember', 'guild': 'g', 'member': 'm'},
                  {'variant': 'DirectMessage', 'user': 'u'},
                  {'variant': 'Organization', 'organization': 'o'})
        for path in LOCAL:
            for scope in scopes:
                with self.subTest(path=path, scope=scope['variant']):
                    base = for_local(path)
                    binding = {'tenant': 'local', 'scope': scope}
                    base['identity']['binding'] = copy.deepcopy(binding)
                    v._walk(base, lambda ref: ref.update(binding=copy.deepcopy(binding)),
                            lambda ref, kind: ref.update(binding=copy.deepcopy(binding)))
                    base['authority']['consent']['binding'] = copy.deepcopy(binding)
                    base['lifecycle']['deletion_key']['binding'] = copy.deepcopy(binding)
                    self.accept(base)
                    for field in ('tenant', 'scope'):
                        bad = copy.deepcopy(base)
                        ref = at(bad, path)
                        ref['binding'][field] = ('other' if field == 'tenant' else
                                                {'variant': 'DirectMessage', 'user': 'different'})
                        self.reject('ScopeMismatch', bad)
                    bad = copy.deepcopy(base)
                    at(bad, path)['digest'] = DIGEST.hex()
                    self.reject('SelfReference', bad)
                    at(bad, path)['domain'] = 'LegacyEpisodeV1'
                    self.accept(bad)

    def test_every_artifact_slot_matrix(self):
        for path, expected in ARTIFACT:
            with self.subTest(path=path):
                base = fixture()
                self.accept(base)
                for field in ('tenant', 'scope'):
                    bad = copy.deepcopy(base)
                    at(bad, path)['binding'][field] = ('other' if field == 'tenant' else
                                                      {'variant': 'Organization', 'organization': 'other'})
                    self.reject('ScopeMismatch', bad)
                exact = copy.deepcopy(base)
                at(exact, path)['byte_length'] = 1 << 40
                self.accept(exact)
                at(exact, path)['byte_length'] += 1
                self.reject('InvalidRange', exact)
                bad = copy.deepcopy(base)
                at(bad, path)['digest'] = DIGEST.hex()
                self.reject('SelfReference', bad)
                if expected is not None:
                    bad = copy.deepcopy(base)
                    at(bad, path)['kind'] = 'Model'
                    self.reject('InvalidArtifactKind', bad)
                else:
                    for kind in KINDS:
                        any_kind = copy.deepcopy(base)
                        at(any_kind, path)['kind'] = kind
                        self.accept(any_kind)
                        at(any_kind, path)['digest'] = DIGEST.hex()
                        self.reject('SelfReference', any_kind)

    def test_walk_exact_inventory_and_received_order(self):
        base = fixture()
        expected_local = [at(base, path) for path in LOCAL
                          if path[-1] not in ('privacy_basis', 'obligation')]
        expected_artifact = [(at(base, path), kind) for path, kind in ARTIFACT]
        # Several list fields contain two entries; explicitly include the second.
        expected_local.insert(1, base['authority']['approvals'][1])
        expected_local.insert(4, base['provenance']['causal_parents'][1])
        expected_local.insert(8, base['lifecycle']['edges'][1]['target'])
        expected_local.insert(9, base['lifecycle']['edges'][1]['basis'])
        expected_artifact.insert(13, (base['evidence']['source_refs'][1], None))
        local, artifacts = [], []
        v._walk(base, local.append, lambda ref, kind: artifacts.append((ref, kind)))
        self.assertEqual(local, expected_local)
        self.assertEqual(artifacts, expected_artifact)
        for retention_path in LOCAL[6:8]:
            scoped = for_local(retention_path)
            found = []
            v._walk(scoped, found.append, lambda ref, kind: None)
            self.assertIn(at(scoped, retention_path), found)
        empty = json.loads((FIXTURES / 'provisional-minimal.json').read_text())
        v._walk(empty, lambda ref: self.fail('unexpected local callback'),
                lambda ref, kind: self.fail('unexpected artifact callback'))

    def test_binding_foreign_and_nonreference_exceptions(self):
        for path in (('authority', 'consent', 'binding'), ('lifecycle', 'deletion_key', 'binding')):
            for field in ('tenant', 'scope'):
                bad = fixture()
                at(bad, path)[field] = ('other' if field == 'tenant' else
                                        {'variant': 'DirectMessage', 'user': 'other'})
                self.reject('ScopeMismatch', bad)
        base = fixture()
        foreign = base['provenance']['consulted_parents'][1]['reference']
        self.accept(base)
        foreign['origin_binding']['scope']['organization'] = 'wrong'
        self.reject('ScopeMismatch', base)
        for scope in ({'variant': 'Guild', 'guild': 'elsewhere'},
                      {'variant': 'GuildMember', 'guild': 'elsewhere', 'member': 'm'},
                      {'variant': 'DirectMessage', 'user': 'elsewhere'}):
            foreign['origin_binding']['scope'] = scope
            foreign['digest'] = DIGEST.hex()
            foreign['domain'] = 'CompleteV1'
            self.accept(base)
        # These are digests/identities, not walked episode/artifact references.
        base = fixture()
        base['identity']['contract']['digest'] = DIGEST.hex()
        base['authority']['consent']['policy']['digest'] = DIGEST.hex()
        base['authority']['policy']['digest'] = DIGEST.hex()
        base['context']['summary']['format']['digest'] = DIGEST.hex()
        self.accept(base)

    def test_each_duplicate_family_and_distinct_controls(self):
        paths = [path for path, cap in LISTS if path != ('decision', 'actions')]
        for path in paths:
            with self.subTest(path=path):
                base = fixture()
                item = copy.deepcopy(at(base, path)[0])
                put(base, path, [copy.deepcopy(item), copy.deepcopy(item)])
                self.reject('DuplicateReference', base)
                second = at(base, path)[1]
                if path[-1] == 'parameter_classes':
                    at(base, path)[1] = 'different'
                elif path[-1] in ('resources', 'costs'):
                    second['amount'] += 1
                    self.reject('DuplicateReference', base)
                    second['resource'] = 'MemoryBytes'
                elif path[-1] in ('capabilities', 'models', 'calibrations'):
                    second['revision'] = 'other'
                    second['digest'] = 'aa' * 32
                    self.reject('DuplicateReference', base)
                    second['component'] = 'different'
                elif path[-1] in ('approvals', 'holds', 'causal_parents'):
                    second['domain'] = 'LegacyEpisodeV1'
                elif path[-1] == 'edges':
                    second['relation'] = 'Forgets'
                elif path[-1] == 'consulted_parents':
                    second['reference']['domain'] = 'LegacyEpisodeV1'
                else:
                    second['format']['revision'] = 'different'
                    self.accept(base)
                    second['format']['revision'] = item['format']['revision']
                    second['byte_length'] += 1
                self.accept(base)
        base = fixture()
        base['decision']['actions'] *= 2
        base['outcome']['costs'] = copy.deepcopy(base['context']['resources'])
        base['lifecycle']['holds'] = copy.deepcopy(base['authority']['approvals'])
        base['provenance']['causal_parents'] = copy.deepcopy(base['authority']['approvals'])
        base['provenance']['consulted_parents'][0]['reference'] = copy.deepcopy(base['authority']['approvals'][0])
        self.accept(base)
        # Dict insertion order does not defeat structural duplicate equality.
        base = fixture()
        first = base['authority']['approvals'][0]
        base['authority']['approvals'] = [first, dict(reversed(list(first.items()))) ]
        self.reject('DuplicateReference', base)

    def test_every_collection_cap_precedes_walk_and_duplicates(self):
        for path, cap in LISTS:
            with self.subTest(path=path, cap=cap):
                base = fixture()
                put(base, path, [None] * (cap + 1))
                with patch.object(v, '_walk', side_effect=AssertionError('walk before cap')):
                    self.reject('CollectionLimit', base)
                base = fixture()
                original = copy.deepcopy(at(base, path)[0])
                put(base, path, [copy.deepcopy(original) for _ in range(cap)])
                if path == ('decision', 'actions'):
                    self.accept(base)
                else:
                    self.reject('DuplicateReference', base)

    def test_unique_collection_boundaries(self):
        resources = ('Tokens', 'CpuMicros', 'MemoryBytes', 'NetworkBytes', 'EnergyMicrojoules')
        for path, cap in LISTS:
            with self.subTest(path=path):
                base = fixture()
                original = at(base, path)[0]
                values = []
                # Resource cap16 is not attainable with unique kind keys: the
                # vocabulary has only five. Do not claim repeated16 is valid.
                count = 5 if path[-1] in ('resources', 'costs') else cap
                for index in range(count):
                    item = copy.deepcopy(original)
                    if path[-1] == 'parameter_classes':
                        item = 'parameter-' + str(index)
                    elif path[-1] in ('resources', 'costs'):
                        item['resource'] = resources[index]
                    elif path[-1] in ('capabilities', 'models', 'calibrations'):
                        item['component'] = 'component-' + str(index)
                    elif path[-1] in ('approvals', 'holds', 'causal_parents'):
                        item['digest'] = index.to_bytes(32, 'big').hex()
                    elif path[-1] == 'edges':
                        item['target']['digest'] = index.to_bytes(32, 'big').hex()
                    elif path[-1] == 'consulted_parents':
                        item['reference']['digest'] = index.to_bytes(32, 'big').hex()
                    elif path != ('decision', 'actions'):
                        item['byte_length'] = index
                    values.append(item)
                put(base, path, values)
                self.accept(base)

    def test_artifact_check_sequence_before_self_reference(self):
        base = fixture()
        artifact = base['context']['summary']
        artifact.update(byte_length=(1 << 40) + 1, kind='Model', digest=DIGEST.hex())
        artifact['binding']['tenant'] = 'other'
        self.reject('ScopeMismatch', base)
        artifact['binding']['tenant'] = base['identity']['binding']['tenant']
        self.reject('InvalidRange', base)
        artifact['byte_length'] = 0
        self.reject('InvalidArtifactKind', base)
        artifact['kind'] = 'StateSummary'
        self.reject('SelfReference', base)

    def test_posterior_monotonic_and_utility_defense(self):
        for entries in ([], [['x', 1_000_000]] * 33, [['x', 500_000], ['x', 500_000]],
                        [['x', 999_999]]):
            base = fixture()
            base['context']['regime_posterior'] = {'known': entries}
            self.reject('InvalidPosterior', base)
        base = fixture()
        base['context']['regime_posterior'] = {'known': [[str(i), 1_000_000 if i == 31 else 0] for i in range(32)]}
        self.accept(base)
        base['time']['monotonic']['start'] = 11
        self.reject('InvalidRange', base)
        base['time']['monotonic']['end'] = 11
        self.accept(base)
        for unit in ('Count', 'Bytes', 'Unitless', 'Milliseconds', 'Microjoules'):
            for amount in (-1_000_000, -1, 0, 1, 1_000_000):
                with self.subTest(unit=unit, amount=amount):
                    base = fixture()
                    base['outcome']['utility'] = {'known': {'unit': unit, 'value': amount}}
                    if unit in ('Count', 'Bytes') and (amount < 0 or amount % 1_000_000):
                        self.reject('InvalidRange', base)
                    else:
                        self.accept(base)

    def test_retention_expiration_boundary_and_no_clock_policy(self):
        for variant in ('Session', 'Operational'):
            for delta in (-1, 0, 1):
                with self.subTest(variant=variant, delta=delta):
                    base = fixture()
                    retention = {'variant': variant, 'expires_at': base['time']['received_at'] + delta}
                    if variant == 'Session':
                        retention['session'] = 'synthetic-session'
                    base['lifecycle']['retention'] = retention
                    if delta <= 0:
                        self.reject('InvalidRange', base)
                    else:
                        self.accept(base)
        for path in LOCAL[6:8]:
            base = for_local(path)
            base['time']['received_at'] = (1 << 64) - 1
            self.accept(base)
        base = fixture()
        base['lifecycle']['retention'] = {'variant': 'Ephemeral'}
        base['time']['acquired_at'] = (1 << 64) - 1
        self.accept(base)

    def test_unsorted_sets_remain_stage_accepted(self):
        base = fixture()
        for path, cap in LISTS:
            at(base, path).reverse()
        base['context']['regime_posterior']['known'].reverse()
        self.accept(base)

    def test_canonical_byte_relational_failures_and_digest_forwarding(self):
        # Each mutation passes independent bounded syntax + reviewed typed mapping.
        for code, mutation in (
            ('DuplicateReference', lambda root: tree_at(root, (8, 'approvals')).items.append(
                copy.deepcopy(tree_at(root, (8, 'approvals')).items[0]))),
            ('ScopeMismatch', lambda root: replace_tree_field(
                tree_at(root, (5, 'summary', 'binding')), 'tenant', c.Text('other'))),
            ('InvalidArtifactKind', lambda root: replace_tree_field(
                tree_at(root, (5, 'summary')), 'kind', c.Array([c.Unsigned(4)]))),
            ('InvalidRange', lambda root: replace_tree_field(
                tree_at(root, (10, 'retention')).items[1], 'expires_at', c.Unsigned(99))),
            ('InvalidRange', lambda root: replace_tree_field(
                tree_at(root, (5, 'summary')), 'byte_length', c.Unsigned((1 << 40) + 1))),
        ):
            with self.subTest(code=code):
                valid = bytes.fromhex((FIXTURES / 'provisional-populated.hex').read_text())
                root = Reader(valid, DEFAULTS.copy(), c, False).value()
                mutation(root)
                data = c.encode_value(root)
                with self.assertRaises(v.RelationalDecodeError) as caught:
                    byte_pipeline(data)
                self.assertEqual(caught.exception.code, 'InvalidValue:' + code)
        data = bytes.fromhex((FIXTURES / 'provisional-minimal.hex').read_text())
        with patch.object(v, 'validate_fixture', wraps=v.validate_fixture) as observed:
            byte_pipeline(data)
        self.assertEqual(observed.call_count, 1)
        self.assertEqual(observed.call_args.args[1], hashlib.sha256(data).digest())


if __name__ == '__main__':
    unittest.main()
