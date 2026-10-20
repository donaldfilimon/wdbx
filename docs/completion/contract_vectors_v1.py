"""Executable C0 contract vectors, not a product runtime or efficacy experiment.

No files are written on import. The checked-in worked vectors are reference data.
Numerical comparisons use declared tolerance; MAC bytes are exact.
"""
import hashlib
import hmac
import math
import struct
from collections import Counter
from fractions import Fraction
from itertools import permutations


def f32(x):
    return struct.unpack('!f', struct.pack('!f', x))[0]


def random_step(state):
    mask = (1 << 64) - 1
    state ^= (state << 13) & mask
    state ^= state >> 7
    state ^= (state << 17) & mask
    state &= mask
    if state == 0:
        state = 104729
    return state, (state >> 11) / float(1 << 53)


def network():
    """Existing default CSR topology/seed, with explicit IEEE f32 weight bytes."""
    seed = 104729
    layers = []
    for inputs, outputs in [(128, 64), (64, 32)]:
        columns, weights = [], []
        for row in range(outputs):
            for k in range(8):
                columns.append((row * 7 + k * 13) % inputs)
                seed, value = random_step(seed)
                weights.append(f32((value - 0.5) * 0.7))
        layers.append({'inputs': inputs, 'outputs': outputs,
                       'offsets': list(range(0, outputs * 8 + 1, 8)),
                       'columns': columns, 'weights': weights,
                       'biases': [f32(0.01)] * outputs})
    return {'version': 1, 'initial_seed': 104729, 'layers': layers}


def encode(case):
    # Caller passes the frozen normalized role/value tokens and explicit scalars.
    values = [0.0] * 128
    tokens = case['tokens']
    for role, token in tokens:
        hashed = hashlib.sha256((role + '\x00' + token).encode('utf-8')).digest()
        bucket = int.from_bytes(hashed[:4], 'big') % 96
        sign = 1 if hashed[4] & 1 == 0 else -1
        values[bucket] = f32(values[bucket] + f32(sign / max(1, len(tokens))))
    for offset, value in enumerate(case['context'] + case['atp_features'], 96):
        values[offset] = f32(value)
    return values


def run_network(artifact, encoded, seed, familiar):
    values = encoded
    activations = []
    for layer in artifact['layers']:
        seed, draw = random_step(seed)
        sigmoid = draw < (0.75 if familiar else 0.25)
        next_values = []
        for row in range(layer['outputs']):
            total = f32(0)
            for k in range(layer['offsets'][row], layer['offsets'][row + 1]):
                total = f32(total + f32(layer['weights'][k] * values[layer['columns'][k]]))
            total = f32(total + layer['biases'][row])
            value = f32(1 / (1 + math.exp(-min(30, max(-30, total))))) if sigmoid else f32(min(16, max(0, total)))
            next_values.append(value)
        values = next_values
        activations.append('sigmoid' if sigmoid else 'relu')
    return {'output': values, 'activations': activations}


def operation_vectors():
    # Fixed one-hot basis. It is a testable arbitrary choice, not learned evidence.
    return [{'id': name, 'vector': [1 if j == i else 0 for j in range(32)]}
            for i, name in enumerate(['select', 'concatenate', 'equivalent', 'arithmetic', 'join', 'shape'])]


def worked_synthesis():
    case = {'tokens': [['subject','box'], ['predicate','contains'], ['object','2'], ['subject','box'], ['predicate','added'], ['object','3']],
            'context': [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            'atp_features': [0] * 16, 'seed': 104729, 'familiar': False,
            'ingredients': [{'ref':'f0','kind':'fact','fact':['box','contains',2]},
                            {'ref':'f1','kind':'fact','fact':['box','added',3]}],
            'query': {'kind':'count_after_add','subject':'box','base':2,'added':3}}
    encoded = encode(case)
    result = run_network(network(), encoded, case['seed'], case['familiar'])
    ranked = sorted([{'operation': op['id'], 'score': result['output'][i]} for i, op in enumerate(operation_vectors())], key=lambda x:(-x['score'],x['operation']))
    # Query contract admits arithmetic only; higher-scoring inadmissible operations cannot win.
    return {'input':case,'encoded':encoded,'activations':result['activations'],
            'network_output':result['output'],'raw_ranking':ranked,
            'admissible_operations':['arithmetic'],
            'decoded_ast':['arithmetic','+',2,3],
            'output_text':'5','source_refs':['f0','f1']}


def frame(items):
    return b''.join(len(item).to_bytes(4, 'big') + item for item in items)


def pair_bytes(pattern_class, action_class, inhibition, mode='public-class', version='v1'):
    """Already validated canonical class IDs, no arbitrary phrase normalization."""
    return frame([b'wdbx/inhibited-pair/v1', mode.encode('ascii'), version.encode('ascii'),
                  pattern_class.encode('utf-8'), action_class.encode('utf-8')] +
                 [s.encode('ascii') for s in sorted(set(inhibition))])


def commitment(tenant_key_hex, tenant, scope, pair):
    key = bytes.fromhex(tenant_key_hex)
    derived = hmac.new(key, frame([b'wdbx/inhibition-scope/v1', tenant.encode('ascii'), scope.encode('ascii')]), hashlib.sha256).digest()
    return hmac.new(derived, pair, hashlib.sha256).hexdigest()


def inhibition_vectors():
    key0, key1 = bytes(range(32)).hex(), bytes(range(32,64)).hex()
    pair = pair_bytes('pattern:add-two:v1','action:sum-two:v1',['deny:network'])
    base = {'tenant':'tenant-A','scope':'specimen-1','pattern_class':'pattern:add-two:v1','action_class':'action:sum-two:v1','inhibition':['deny:network']}
    old = commitment(key0, 'tenant-A','specimen-1',pair)
    new = commitment(key1, 'tenant-A','specimen-1',pair)
    return {'synthetic_keys_only':True,'key_epoch_0_hex':key0,'key_epoch_1_hex':key1,
            'canonical_input':base,'preimage_hex':pair.hex(),
            'epoch_0_tag':old,'epoch_1_tag':new,
            'other_tenant_tag':commitment(key0,'tenant-B','specimen-1',pair),
            'other_scope_tag':commitment(key0,'tenant-A','specimen-2',pair),
            'equivalent_attempts':['two plus two -> four','2 + 2 -> 4'],
            'after_payload_key_destruction':{'match_epoch':0,'rejected':True,'tag':old},
            'after_mac_rotation':{'query_epochs':[0,1],'stored_epoch':0,'rejected':True},
            'non_equivalent_tag':commitment(key0,'tenant-A','specimen-1',pair_bytes('pattern:add-two:v1','action:sum-three:v1',['deny:network']))}


def synthetic_pair(pattern, action):
    """Finite public test equivalences, never a private phrase registry."""
    patterns = {'two plus two': 'pattern:add-two:v1', '2 + 2': 'pattern:add-two:v1'}
    actions = {'four': 'action:sum-two:v1', '4': 'action:sum-two:v1'}
    return pair_bytes(patterns[pattern], actions[action], ['deny:network'])


def slot_similarity(left, right):
    def jaccard(a, b):
        a, b = Counter(a), Counter(b)
        union = sum((a | b).values())
        return Fraction(sum((a & b).values()), union) if union else Fraction(1)
    a, b = left.split(), right.split()
    return 100 * (2 * jaccard(a, b) + jaccard(list(zip(a, a[1:])), list(zip(b, b[1:])))) / 3


def small_alignment(recipients, donors):
    """Exhaustive <=8-slot branch; stable ref ties, exact rational weights."""
    recipients = sorted(recipients, key=lambda s: s['ref'])
    donors = sorted(donors, key=lambda s: s['ref'])
    count = min(len(recipients), len(donors))
    best = None
    for assignment in permutations(donors, count):
        scores = [slot_similarity(r['text'], d['text']) for r, d in zip(recipients, assignment)]
        if any(score < 20 for score in scores):
            continue
        candidate = (-sum(scores), tuple(d['ref'] for d in assignment))
        if best is None or candidate < best:
            best = candidate
    return None if best is None else list(best[1])
