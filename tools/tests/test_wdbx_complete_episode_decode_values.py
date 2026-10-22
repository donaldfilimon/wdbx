"""Staged mapping tests: these do not establish complete validation."""
import copy
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import abbey_cbor_episode_v1 as c
from abbey_cbor_reader_v1 import Reader, DEFAULTS
import wdbx_complete_episode_draft as encoder
import wdbx_complete_episode_decode_values as v
from wdbx_complete_episode_decode_values import ValueDecodeError, map_complete

FIXTURES = Path(__file__).resolve().parents[2] / 'crates/abi-wdbx/src/v3/complete_episode/tests'


def tree(value):
    if type(value) is int:
        return c.Unsigned(value) if value >= 0 else c.Negative(value)
    if type(value) is str:
        return c.Text(value)
    if type(value) is bytes:
        return c.Bytes(value)
    if type(value) is list:
        return c.Array([tree(x) for x in value])
    if type(value) is dict:
        return c.Map([(tree(k), tree(x)) for k, x in value.items()])
    raise AssertionError('test tree type')


class ValuesTests(unittest.TestCase):
    def reject(self, code, function, *args):
        with self.assertRaises(ValueDecodeError) as caught:
            function(*args)
        self.assertEqual(caught.exception.code, code)
        self.assertEqual(str(caught.exception), code)

    def test_minimal_and_populated_pinned_projections(self):
        for name in ('minimal', 'populated'):
            with self.subTest(name=name):
                expected = json.loads((FIXTURES / f'provisional-{name}.json').read_text())
                data = bytes.fromhex((FIXTURES / f'provisional-{name}.hex').read_text())
                reader = Reader(data, DEFAULTS.copy(), c, False)
                root = reader.value()
                self.assertEqual(reader.cursor, len(data))
                actual = map_complete(root)
                # Constructor input deliberately has reversed posterior order.
                # Pin the two received wire entries explicitly, independently
                # of both forward mapping and the inverse under test.
                if name == 'populated':
                    expected['context']['regime_posterior']['known'] = [['a', 1_000_000], ['z', 0]]
                self.assertEqual(actual, expected)
                self.assertEqual(encoder.cbor(encoder.map_fixture(actual)), data)

    def test_redaction_and_category_whitelist(self):
        for value in ('secret-sensitive', '', 'InvalidValue:secret'):
            with self.assertRaisesRegex(ValueError, '^invalid value error category$'):
                ValueDecodeError(value)
        self.reject('InvalidShape', map_complete, c.Text('secret-sensitive'))

    def setUp(self):
        self.binding = {'tenant': 'tenant', 'scope': [0, {'guild': 'guild'}]}
        self.version = {'component': 'component', 'revision': 'v1', 'digest': bytes(32)}
        self.local = {'binding': self.binding, 'domain': [0], 'digest': bytes(32)}
        self.artifact = {'binding': self.binding, 'digest': bytes(32), 'byte_length': 0,
                         'kind': [8], 'format': self.version}
        self.foreign = {'origin_organization': 'org', 'origin_binding': self.binding,
                        'domain': [1], 'digest': bytes(32), 'import_receipt': self.local}
        self.action = {'capability': self.version, 'action': 'act',
                       'parameter_classes': [], 'phase': [0]}

    def root(self):
        data = bytes.fromhex((FIXTURES / 'provisional-populated.hex').read_text())
        return Reader(data, DEFAULTS.copy(), c, False).value()

    def test_root_dispatch_and_exact_fields(self):
        root = self.root()
        for entries in (root.entries[:-1], root.entries + [(c.Unsigned(12), c.NULL)],
                        root.entries[:-1] + [root.entries[0]],
                        [(c.Text('0'), root.entries[0][1])] + root.entries[1:]):
            self.reject('InvalidShape', map_complete, c.Map(entries))
        for index, value, code in (
            (0, c.Text('private-other'), 'UnsupportedDomain'),
            (0, c.Unsigned(1), 'InvalidShape'),
            (1, c.Unsigned(2), 'UnsupportedVersion'),
            (1, c.Bool(True), 'InvalidShape'),
        ):
            altered = copy.deepcopy(root)
            altered.entries[index] = (c.Unsigned(index), value)
            self.reject(code, map_complete, altered)
        self.assertEqual(map_complete(c.Map(list(reversed(root.entries)))), map_complete(root))

    def test_every_populated_map_field_shape(self):
        # Walk every populated group/reference, including optional maps. Each
        # mutation is single-fault; duplicate map keys must survive as pairs.
        root = self.root()
        paths = []
        def visit(node, path=()):
            if type(node) is c.Map:
                paths.append(path)
                for i, (_, child) in enumerate(node.entries):
                    visit(child, path + (('m', i),))
            elif type(node) is c.Array:
                for i, child in enumerate(node.items):
                    visit(child, path + (('a', i),))
        visit(root)
        def at(node, path):
            for kind, index in path:
                node = node.entries[index][1] if kind == 'm' else node.items[index]
            return node
        for path in paths:
            node = at(root, path)
            # A removed optional field may be legal; a null present value is not.
            for i in range(len(node.entries)):
                altered = copy.deepcopy(root)
                target = at(altered, path)
                target.entries[i] = (target.entries[i][0], c.NULL)
                with self.subTest(path=path, field=i, mutation='null'):
                    self.reject('InvalidShape', map_complete, altered)
                for operation in ('duplicate', 'wrongkey'):
                    altered = copy.deepcopy(root)
                    target = at(altered, path)
                    if operation == 'duplicate':
                        target.entries.append(copy.deepcopy(target.entries[i]))
                    else:
                        key = c.Text('wrong') if not path else c.Unsigned(0)
                        target.entries[i] = (key, target.entries[i][1])
                    with self.subTest(path=path, field=i, mutation=operation):
                        self.reject('InvalidShape', map_complete, altered)
            altered = copy.deepcopy(root)
            target = at(altered, path)
            key = c.Unsigned(20) if not path else c.Text('extra-sensitive')
            target.entries.append((key, c.NULL))
            self.reject('InvalidShape', map_complete, altered)
        self.assertGreater(len(paths), 50)

    def test_every_map_helper_missing_fields(self):
        examples = [
            (v._binding, self.binding), (v._version, self.version), (v._local, self.local),
            (v._artifact, self.artifact), (v._foreign, self.foreign),
            (v._actor, {'principal': 'p', 'kind': 'service'}),
            (v._resource, {'resource': [0], 'amount': 0}), (v._action, self.action),
            (v._consent, {'binding': self.binding, 'epoch': 0, 'policy': self.version,
                          'attestation': self.local}),
            (v._deletion_key, {'binding': self.binding, 'key_id': 'key'}),
            (v._edge, {'relation': [0], 'target': self.local, 'basis': self.local}),
            (v._measurement, {'unit': [0], 'value': 0}),
            (v._monotonic, {'clock': 'clock', 'start': 0, 'end': 0}),
            (v._dimensions, {key: [0] for key in ('cryptographic', 'source', 'calibration',
                'outcome', 'compatibility', 'regime', 'lifecycle', 'confidence')}),
        ]
        root = self.root()
        examples.extend((mapper, root.entries[index][1]) for index, mapper in enumerate(
            (v._identity, v._time, v._origin, v._context, v._decision, v._outcome,
             v._authority, v._evidence, v._lifecycle, v._provenance), 2))
        for mapper, data in examples:
            node = data if type(data) is c.Map else tree(data)
            for i, (key, _) in enumerate(node.entries):
                if key.value in ('monotonic', 'summary', 'reservoir', 'consent', 'deletion_key'):
                    continue
                with self.subTest(mapper=mapper.__name__, field=key.value):
                    self.reject('InvalidShape', mapper, c.Map(node.entries[:i] + node.entries[i+1:]))

    def test_scalars_exact_types_widths_and_redaction(self):
        for mapper in (v._text, v._unsigned, v._signed, v._token, v._signer,
                       v._digest, v._uuid, v._ppm):
            for value in (c.Bool(True), c.NULL, c.Array([]), c.Map([])):
                self.reject('InvalidShape', mapper, value)
        for mapper, value in ((v._text, c.Text(1)), (v._unsigned, c.Unsigned(True)),
                              (v._signed, c.Negative(0)), (v._signed, c.Unsigned(1 << 63)),
                              (v._digest, c.Bytes(bytes(31))), (v._uuid, c.Bytes(bytes(15)))):
            self.reject('InvalidShape', mapper, value)
        for amount in (-1, 1 << 64, True):
            broken = c.Unsigned(0)
            broken.value = amount
            self.reject('InvalidShape', v._unsigned, broken)
        broken = c.Negative(-1)
        broken.value = -(1 << 63) - 1
        self.reject('InvalidShape', v._signed, broken)
        for spelling in ('', 'x' * 129, 'secret sensitive', 'é', 'a\n', 'a/b'):
            self.reject('InvalidValue:InvalidIdentifier', v._token, c.Text(spelling))
        for spelling in ('x' * 128, 'A-z_9.:'):
            self.assertEqual(v._token(c.Text(spelling)), spelling)
        for spelling in ('secret-sensitive', 'ed25519:' + 'A' * 64, 'ed25519:' + 'a' * 63):
            self.reject('InvalidValue:InvalidIdentifier', v._signer, c.Text(spelling))
        self.assertEqual(v._digest(c.Bytes(bytes(32))), '00' * 32)
        self.assertEqual(v._unsigned(c.Unsigned((1 << 64) - 1)), (1 << 64) - 1)
        self.assertEqual(v._signed(c.Negative(-(1 << 63))), -(1 << 63))
        self.assertEqual(v._signed(c.Unsigned((1 << 63) - 1)), (1 << 63) - 1)
        self.reject('InvalidShape', v._ppm, c.Unsigned(1 << 32))
        self.reject('InvalidValue:InvalidRange', v._ppm, c.Unsigned(1_000_001))
        self.assertEqual(v._ppm(c.Unsigned(0)), 0)
        self.assertEqual(v._ppm(c.Unsigned(1_000_000)), 1_000_000)

    def test_uuid_versions_variants(self):
        for version in (4, 7):
            raw = bytearray(16)
            raw[6], raw[8] = version << 4, 128
            self.assertEqual(v._uuid(c.Bytes(raw)), bytes(raw).hex())
        for raw in (bytes(16), bytes.fromhex('00000000000060008000000000000000'),
                    bytes.fromhex('0000000000004000c000000000000000')):
            self.reject('InvalidValue:InvalidUuid', v._uuid, c.Bytes(raw))

    def test_all_unit_vocabulary_and_bad_tuples(self):
        tables = (
            (v._DOMAIN, ('CompleteV1', 'LegacyEpisodeV1')),
            (v._ROUTE, ('NoModel', 'LocalModel', 'CloudModel', 'Specialist', 'Deterministic')),
            (v._PHASE, ('Proposed', 'Performed')),
            (v._UNIT, ('Unitless', 'Count', 'Milliseconds', 'Bytes', 'Microjoules')),
            (v._RESOURCE, ('Tokens', 'CpuMicros', 'MemoryBytes', 'NetworkBytes', 'EnergyMicrojoules')),
            (v._ARTIFACT, ('StateSummary', 'Prediction', 'Outcome', 'Observation', 'Model',
                          'Calibration', 'Provenance', 'ReservoirEmbedding', 'Evidence', 'Authority')),
            (v._SAFETY, ('Unknown', 'NoViolationObserved', 'InterlockTriggered', 'ViolationObserved')),
            (v._ROLLBACK, ('NotApplicable', 'NotAttempted', 'Attempted', 'ExactRestoreObserved',
                          'Failed', 'Unknown')),
            (v._RELATION, ('Supersedes', 'Contradicts', 'Revokes', 'Quarantines', 'Resolves',
                          'Redacts', 'Forgets')),
        )
        for mapper, names in tables:
            for index, name in enumerate(names):
                self.assertEqual(mapper(tree([index])), name)
            self.reject('UnknownVariant', mapper, tree([len(names)]))
            for malformed in (c.NULL, tree([]), tree([0, {}]), tree(['0']), c.Array([c.Bool(True)])):
                self.reject('InvalidShape', mapper, malformed)
        for label, name in (('human_subject', 'HumanSubject'), ('organization_owner', 'OrganizationOwner'),
                            ('guild_owner', 'GuildOwner'), ('guild_administrator', 'GuildAdministrator'),
                            ('guild_manager', 'GuildManager'), ('service', 'Service')):
            self.assertEqual(v._actor_kind(c.Text(label)), name)
        self.reject('UnknownVariant', v._actor_kind, c.Text('secret-sensitive'))
        for index in range(8):
            self.assertEqual(v._maturity(c.Text('C' + str(index))), 'C' + str(index))
        self.reject('UnknownVariant', v._maturity, c.Text('C8'))
        self.reject('InvalidShape', v._actor_kind, c.Unsigned(0))
        self.reject('InvalidShape', v._maturity, c.Unsigned(0))

    def test_scope_and_consulted_variants(self):
        for index, (name, fields) in enumerate((('Guild', {'guild': 'g'}),
            ('GuildMember', {'guild': 'g', 'member': 'm'}), ('DirectMessage', {'user': 'u'}),
            ('Organization', {'organization': 'o'}))):
            self.assertEqual(v._scope(tree([index, fields])), {'variant': name, **fields})
            for key in fields:
                missing = dict(fields)
                del missing[key]
                self.reject('InvalidShape', v._scope, tree([index, missing]))
        for index, name, reference in ((0, 'Local', self.local), (1, 'Foreign', self.foreign)):
            result = v._consulted(tree([index, {'reference': reference}]))
            self.assertEqual(result['variant'], name)
            self.assertEqual(result['reference']['digest'], '00' * 32)
            self.reject('InvalidShape', v._consulted, tree([index, {}]))
        for mapper in (v._scope, v._consulted):
            self.reject('UnknownVariant', mapper, tree([99, {}]))
            for malformed in ([], [0], [0, {}, {}], [99, 0], ['0', {}]):
                self.reject('InvalidShape', mapper, tree(malformed))

    def test_retention_and_estimate_arity(self):
        for tag, name, fields in ((1, 'Session', {'session': 's', 'expires_at': 0}),
            (2, 'Operational', {'expires_at': 0}), (3, 'Durable', {'privacy_basis': self.local}),
            (4, 'MandatoryIncident', {'obligation': self.local})):
            result = v._retention(tree([tag, fields]))
            self.assertEqual(result['variant'], name)
            self.reject('InvalidShape', v._retention, tree([tag]))
            for key in fields:
                missing = dict(fields)
                del missing[key]
                self.reject('InvalidShape', v._retention, tree([tag, missing]))
        self.assertEqual(v._retention(tree([0])), {'variant': 'Ephemeral'})
        for malformed in ([], [0, {}], [0, {}, {}], [1, 0], [99, 0]):
            self.reject('InvalidShape', v._retention, tree(malformed))
        self.reject('UnknownVariant', v._retention, tree([99]))
        self.reject('UnknownVariant', v._retention, tree([99, {}]))
        self.assertEqual(v._estimate(tree([0]), v._ppm), {'unknown': True})
        self.assertEqual(v._estimate(tree([1, 0]), v._ppm), {'known': 0})
        for malformed in ([], [0, 0], [1], [2], [2, 0], [1, 0, 0], ['0']):
            self.reject('InvalidShape', v._estimate, tree(malformed), v._ppm)
        self.reject('InvalidShape', v._estimate, c.Array([c.Unsigned(1), c.NULL]), v._ppm)

    def test_posterior_constructor_and_preconversion_cap(self):
        self.assertEqual(v._posterior(tree([['z', 0], ['a', 1_000_000]])), [['z', 0], ['a', 1_000_000]])
        self.assertEqual(len(v._posterior(tree([[f'r{i}', 31_250] for i in range(32)]))), 32)
        for malformed in ([], [['a', 1]], [['a', 500_000], ['a', 500_000]]):
            self.reject('InvalidValue:InvalidPosterior', v._posterior, tree(malformed))
        for malformed in ([['a']], [['a', 0, 0]], [[0, 1_000_000]]):
            self.reject('InvalidShape', v._posterior, tree(malformed))
        self.reject('InvalidValue:InvalidRange', v._posterior, tree([['a', 1_000_001]]))
        self.reject('InvalidShape', v._posterior, tree([['a', 1 << 32]]))
        with patch.object(v, '_posterior_entry', side_effect=AssertionError('child conversion')) as child:
            self.reject('InvalidValue:InvalidPosterior', v._posterior, c.Array([c.NULL] * 33))
            child.assert_not_called()

    def test_monotonic_and_measurement_constructor_rules(self):
        self.reject('InvalidValue:InvalidRange', v._monotonic, tree({'clock': 'c', 'start': 2, 'end': 1}))
        self.assertEqual(v._monotonic(tree({'clock': 'c', 'start': 1, 'end': 1}))['end'], 1)
        for unit in (1, 3):
            for amount in (-1_000_000, 1):
                self.reject('InvalidValue:InvalidRange', v._measurement, tree({'unit': [unit], 'value': amount}))
            for amount in (0, 1_000_000):
                self.assertEqual(v._measurement(tree({'unit': [unit], 'value': amount}))['value'], amount)
        for unit in (0, 2, 4):
            for amount in (-(1 << 63), (1 << 63) - 1):
                self.assertEqual(v._measurement(tree({'unit': [unit], 'value': amount}))['value'], amount)

    def test_all_typed_caps_and_no_child_conversion(self):
        # Repeated values at cap are mapping-only boundaries, not valid sets.
        cases = (
            (v._action, 'parameter_classes', 32, 'p', self.action),
            (v._context, 'resources', 16, {'resource': [0], 'amount': 0}, None),
            (v._decision, 'actions', 128, self.action, None),
            (v._outcome, 'costs', 16, {'resource': [0], 'amount': 0}, None),
            *((v._authority, key, 32, self.version, None) for key in ('capabilities', 'models', 'calibrations')),
            (v._authority, 'approvals', 64, self.local, None),
            *((v._evidence, key, 256, self.artifact, None) for key in ('source_refs', 'observation_refs')),
            (v._lifecycle, 'holds', 64, self.local, None),
            (v._lifecycle, 'edges', 64, {'relation': [0], 'target': self.local, 'basis': self.local}, None),
            (v._provenance, 'causal_parents', 64, self.local, None),
            (v._provenance, 'consulted_parents', 256, [0, {'reference': self.local}], None),
            (v._provenance, 'records', 256, self.artifact, None),
        )
        roots = dict(zip((v._identity, v._time, v._origin, v._context, v._decision, v._outcome,
                         v._authority, v._evidence, v._lifecycle, v._provenance),
                        (child for _, child in self.root().entries[2:])))
        for mapper, key, cap, item, data in cases:
            for length in (cap, cap + 1):
                node = tree(data) if data else copy.deepcopy(roots[mapper])
                for i, (field, _) in enumerate(node.entries):
                    if field.value == key:
                        node.entries[i] = (field, c.Array([tree(item) for _ in range(length)]))
                with self.subTest(mapper=mapper.__name__, key=key, length=length):
                    if length == cap:
                        self.assertEqual(len(mapper(node)[key]), cap)
                    else:
                        self.reject('InvalidValue:CollectionLimit', mapper, node)
                        for i, (field, _) in enumerate(node.entries):
                            if field.value == key:
                                node.entries[i] = (field, c.Array([c.NULL] * length))
                        self.reject('InvalidValue:CollectionLimit', mapper, node)
            with patch.object(v, '_token', side_effect=AssertionError('unrelated seam')):
                # The fixed _list seam proves no child is reached before its cap.
                calls = []
                self.reject('InvalidValue:CollectionLimit', v._list,
                            c.Array([c.NULL] * (cap + 1)), cap, lambda x: calls.append(x))
                self.assertEqual(calls, [])

    def test_stage_only_retention_of_invalid_relations_duplicates_and_order(self):
        root = self.root()
        def replace(group, field, value):
            node = root.entries[group][1]
            for i, (key, _) in enumerate(node.entries):
                if key.value == field:
                    node.entries[i] = (key, tree(value))
                    return
            raise AssertionError('fixture field absent')
        first = dict(self.version, component='z')
        second = dict(self.version, component='a')
        replace(8, 'capabilities', [first, second, first])
        replace(5, 'regime_posterior', [1, [['z', 0], ['a', 1_000_000]]])
        bad_artifact = copy.deepcopy(self.artifact)
        bad_artifact['byte_length'] = (1 << 40) + 1
        bad_artifact['binding']['tenant'] = 'foreign-tenant'
        replace(11, 'records', [bad_artifact, bad_artifact])
        replace(11, 'consulted_parents', [[1, {'reference': self.foreign}]])
        repeated = dict(self.action, parameter_classes=['z', 'a', 'z'])
        replace(6, 'actions', [repeated, repeated])
        result = map_complete(root)
        self.assertEqual([x['component'] for x in result['authority']['capabilities']], ['z', 'a', 'z'])
        self.assertEqual(result['context']['regime_posterior']['known'], [['z', 0], ['a', 1_000_000]])
        self.assertEqual(result['provenance']['records'][0]['byte_length'], (1 << 40) + 1)
        self.assertEqual(result['provenance']['records'][0]['binding']['tenant'], 'foreign-tenant')
        self.assertEqual(len(result['provenance']['records']), 2)
        self.assertEqual(result['decision']['actions'][0]['parameter_classes'], ['z', 'a', 'z'])
        self.assertEqual(len(result['decision']['actions']), 2)
        self.assertEqual(result['provenance']['consulted_parents'][0]['reference']['origin_binding']['scope']['variant'], 'Guild')

    def test_no_forward_encoder_input_mutation_or_shared_mutable_output(self):
        root = self.root()
        before = copy.deepcopy(root)
        before_bytes = c.encode_value(root)
        with patch.object(encoder, 'map_fixture', side_effect=AssertionError('forward mapper called')):
            first, second = map_complete(root), map_complete(root)
        self.assertEqual(c.encode_value(root), before_bytes)
        self.assertEqual(map_complete(root), map_complete(before))
        first['identity']['binding']['scope']['guild'] = 'changed'
        first['authority']['capabilities'].clear()
        self.assertNotEqual(first, second)
        self.assertEqual(map_complete(root), second)
        self.assertFalse(hasattr(v, 'decode_complete'))
        self.assertFalse(hasattr(v, 'main'))


    def test_optional_absence_and_present_null(self):
        root = self.root()
        optional = ((3, 'monotonic'), (5, 'summary'), (5, 'reservoir'),
                    (8, 'consent'), (10, 'deletion_key'))
        for group, field in optional:
            altered = copy.deepcopy(root)
            node = altered.entries[group][1]
            self.assertTrue(any(key.value == field for key, _ in node.entries))
            node.entries = [(key, child) for key, child in node.entries if key.value != field]
            output = map_complete(altered)
            name = ('identity', 'time', 'origin', 'context', 'decision', 'outcome',
                    'authority', 'evidence', 'lifecycle', 'provenance')[group - 2]
            self.assertNotIn(field, output[name])
            node.entries.append((c.Text(field), c.NULL))
            self.reject('InvalidShape', map_complete, altered)

    def test_exact_model_classes_and_builtin_payloads(self):
        class SubUnsigned(c.Unsigned):
            pass
        class SubText(c.Text):
            pass
        class SubBytes(c.Bytes):
            pass
        class SubArray(c.Array):
            pass
        class SubMap(c.Map):
            pass
        self.reject('InvalidShape', v._unsigned, SubUnsigned(1))
        self.reject('InvalidShape', v._text, SubText('token'))
        self.reject('InvalidShape', v._digest, SubBytes(bytes(32)))
        self.reject('InvalidShape', v._array, SubArray([]))
        self.reject('InvalidShape', v._binding, SubMap(tree(self.binding).entries))
        broken = c.Bytes(bytes(32))
        broken.value = bytearray(32)
        self.reject('InvalidShape', v._digest, broken)
        broken_array = c.Array([])
        broken_array.items = ()
        self.reject('InvalidShape', v._array, broken_array)
        broken_map = c.Map([])
        broken_map.entries = ()
        self.reject('InvalidShape', v._binding, broken_map)
        for code in (None, [], True, 42):
            with self.assertRaisesRegex(ValueError, '^invalid value error category$'):
                ValueDecodeError(code)

    def test_alternate_variant_maps_exact_fields(self):
        cases = [(v._scope, [tag, fields]) for tag, fields in (
            (0, {'guild': 'g'}), (1, {'guild': 'g', 'member': 'm'}),
            (2, {'user': 'u'}), (3, {'organization': 'o'}))]
        cases += [(v._retention, [tag, fields]) for tag, fields in (
            (1, {'session': 's', 'expires_at': 0}), (2, {'expires_at': 0}),
            (3, {'privacy_basis': self.local}), (4, {'obligation': self.local}))]
        cases += [(v._consulted, [tag, {'reference': ref}])
                  for tag, ref in ((0, self.local), (1, self.foreign))]
        for mapper, raw in cases:
            for operation in ('duplicate', 'extra', 'wrongkey', 'null', 'bool'):
                node = tree(raw)
                body = node.items[1]
                key, child = body.entries[0]
                if operation == 'duplicate':
                    body.entries.append((key, child))
                elif operation == 'extra':
                    body.entries.append((c.Text('extra'), c.NULL))
                elif operation == 'wrongkey':
                    body.entries[0] = (c.Unsigned(0), child)
                elif operation == 'null':
                    body.entries[0] = (key, c.NULL)
                else:
                    body.entries[0] = (key, c.Bool(False))
                with self.subTest(mapper=mapper.__name__, tag=raw[0], operation=operation):
                    self.reject('InvalidShape', mapper, node)


if __name__ == '__main__':
    unittest.main()
