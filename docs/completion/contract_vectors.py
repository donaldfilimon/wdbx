"""Executable C0 contract vectors, not a product runtime or efficacy experiment.

No files are written on import. The checked-in worked vectors are reference data.
Numerical comparisons use declared tolerance; MAC bytes are exact.
"""
import hashlib
import hmac
import math
import struct
import json
import re
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
    return [{'id': name, 'vector': [1 if j == i else 0 for j in range(32)]}
            for i, name in enumerate(['select', 'concatenate', 'equivalent', 'arithmetic', 'join', 'shape', 'report'])]


def score_bucket(score):
    """Exact floor(score / (1/250000)) for the supplied binary64 value."""
    if not math.isfinite(score):
        raise ValueError('nonfinite score')
    numerator, denominator = float(score).as_integer_ratio()
    return (numerator * 250000) // denominator


def rank_scores(items):
    return sorted(items, key=lambda x: (-score_bucket(x['score']), x['name'].encode('utf-8')))


def composition_graph(public_input, seed=104729):
    """Only the corpus public input is accepted: no oracle, labels or split."""
    if set(public_input) != {'facts', 'query'}:
        raise ValueError('composition public input must contain facts/query only')
    if type(seed) is not int or not 0 <= seed < (1 << 64):
        raise ValueError('invalid seed')
    if not isinstance(public_input['query'], str) or len(public_input['query']) > 8000:
        raise ValueError('invalid query bound')
    match = re.fullmatch(r'what does ([A-Za-z0-9_]+) own and how many items after adding\?', public_input['query'])
    if not match:
        raise ValueError('unsupported public query grammar')
    facts = public_input['facts']
    if not isinstance(facts, list) or len(facts) > 32:
        raise ValueError('invalid fact bound')
    ingredients, tokens = [], []
    for index, fact in enumerate(facts):
        if not isinstance(fact, list) or len(fact) != 3:
            raise ValueError('invalid triple')
        subject, predicate, obj = fact
        if not isinstance(subject, str) or predicate not in ('owns', 'contains', 'added'):
            raise ValueError('invalid relation')
        if (predicate == 'owns' and not isinstance(obj, str)) or (predicate != 'owns' and type(obj) is not int):
            raise ValueError('invalid object type')
        if type(obj) is int and not -(1 << 63) <= obj < (1 << 63):
            raise ValueError('integer bound')
        for value in (subject, predicate, str(obj)):
            if len(value.encode('utf-8')) > 256 or '\x00' in value:
                raise ValueError('invalid token')
        ingredients.append({'ref': 'input/facts/' + str(index), 'kind': 'fact', 'fact': fact[:]})
    ingredients.sort(key=lambda x: x['ref'].encode('utf-8'))
    for ingredient in ingredients:
        for role, value in zip(('subject', 'predicate', 'object'), ingredient['fact']):
            tokens.append([role, str(value)])
    return {'tokens': tokens, 'context': [1]+[0]*15, 'atp_features': [0]*16,
            'seed':seed, 'familiar':False, 'ingredients':ingredients,
            'query': {'kind':'owner_count_after_add','subject':match.group(1)}}


def decode_composition(graph, outputs):
    """Execute bounded typed report decoding; no expected labels are read."""
    if len(outputs) != 32 or not all(math.isfinite(x) for x in outputs):
        raise ValueError('invalid network output')
    if graph['query']['kind'] != 'owner_count_after_add':
        return {'status':'abstain','reason':'unsupported root'}
    facts = graph['ingredients']
    candidates = []
    attempted = 0
    for owner in facts:
        if owner['fact'][:2] != [graph['query']['subject'], 'owns']:
            continue
        for contains in facts:
            if contains['fact'][1] != 'contains':
                continue
            for added in facts:
                if added['fact'][1] != 'added':
                    continue
                attempted += 1
                if attempted > 256:
                    break
                obj = owner['fact'][2]
                if contains['fact'][0] != obj or added['fact'][0] != obj:
                    continue
                count = contains['fact'][2] + added['fact'][2]
                if not -(1 << 63) <= count < (1 << 63):
                    continue
                ast = ['report',owner['ref'],['arithmetic','+',contains['ref'],added['ref']]]
                name = json.dumps(ast, ensure_ascii=False, separators=(',', ':'))
                score = (outputs[6] + outputs[3]) / 2 - .01
                candidates.append({'name':name,'score':score,'ast':ast,
                    'fields':{'owner':owner['fact'][0],'object':obj,'count':count},
                    'source_refs':sorted([owner['ref'],contains['ref'],added['ref']]),
                    'output_text':owner['fact'][0]+' owns '+obj+'; '+obj+' contains '+str(count)+' after adding.'})
            if attempted > 256:
                break
        if attempted > 256:
            break
    if not candidates:
        return {'status':'abstain','reason':'no complete source-backed report','attempted':min(attempted,256)}
    selected = rank_scores(candidates)[:8][0]
    if len(selected['output_text'].split()) > 512:
        return {'status':'abstain','reason':'output token bound'}
    return dict(selected, status='complete', attempted=min(attempted,256), bucket=score_bucket(selected['score']))


def worked_synthesis(public_input=None):
    if public_input is None:
        public_input = {'facts':[['train_entity_0','owns','train_entity_0_box'],
                               ['train_entity_0_box','contains',2],['train_entity_0_box','added',3]],
                        'query':'what does train_entity_0 own and how many items after adding?'}
    graph = composition_graph(public_input)
    encoded = encode(graph)
    result = run_network(network(), encoded, graph['seed'], graph['familiar'])
    ranking = rank_scores([{'name':op['id'],'score':result['output'][i]} for i,op in enumerate(operation_vectors())])
    return {'public_input':public_input, 'graph':graph, 'encoded':encoded,
            'activations':result['activations'], 'network_output':result['output'],
            'raw_ranking':ranking, 'decoded':decode_composition(graph,result['output'])}


def near_tie_vector():
    items = [{'name':'a','score':0.0},{'name':'b','score':3e-6},{'name':'c','score':6e-6},
             {'name':'d','score':-1e-6},{'name':'e','score':8e-6}]
    return {'input':items,'buckets':{x['name']:score_bucket(x['score']) for x in items},
            'ordered_names':[x['name'] for x in rank_scores(items)]}


def frame(items):
    return b''.join(len(item).to_bytes(4, 'big') + item for item in items)


def pair_bytes(pattern_class, action_class, inhibition, mode='public-class', version='v1'):
    """Pair-wide identity: policy codes never affect the permanent ban tag."""
    return frame([b'wdbx/inhibited-pair/v2', mode.encode('ascii'), version.encode('ascii'),
                  pattern_class.encode('utf-8'), action_class.encode('utf-8')])


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
            'policy_variations':[{'codes':codes, 'epoch_0_tag':commitment(key0,'tenant-A','specimen-1',pair_bytes('pattern:add-two:v1','action:sum-two:v1',codes)), 'epoch_1_tag':commitment(key1,'tenant-A','specimen-1',pair_bytes('pattern:add-two:v1','action:sum-two:v1',codes))} for codes in [['deny:network'],[],['deny:write']]],
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
