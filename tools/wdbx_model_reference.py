#!/usr/bin/env python3
"""Independent standard-library witness for native package 1.0 (not a runtime).

Consumes frozen bytes, never rewrites fixtures. Python object allocation is not
N02 production loading qualification. No filesystem references are followed and
provenance never authorizes use. CLI: validate FILE.hex, frame FILE.hex.
"""
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

MASK = (1 << 64) - 1
SCHEMAS = tuple(range(1, 9))
# id: dtype, semantic axes, decay. Independent from Rust catalog.
CATALOG = [
    (1, (1, 5), False), (2, (1, 7), False), (1, (4, 5), False),
    (1, (5, 6), True), (1, (5, 6), True), (1, (3, 5, 6), True),
    (1, (5, 6), True), (1, (5,), False), (1, (2,), False),
    (1, (4, 6), True), (1, (4,), False), (1, (5, 6), True),
    (1, (5, 6), True), (1, (5, 6), True), (1, (5,), False),
    (1, (5,), False),
]


class ContractError(ValueError):
    """Bounded context, never source payload."""
    def __init__(self, code, field, offset=0):
        self.code = code
        super().__init__(f'{code}:{field}@{offset}')


def require(condition, code, field, offset=0):
    if not condition:
        raise ContractError(code, field, offset)


def digest(data):
    return hashlib.sha256(data).digest()


def bounded(value, low, high, code, field):
    require(low <= value <= high, code, field)
    return value


class Reader:
    def __init__(self, raw, kind):
        self.raw, self.kind, self.pos, self.metadata = raw, kind, 0, 0

    def take(self, count, numeric=False):
        require(0 <= count <= len(self.raw) - self.pos, 'Truncated', f'component{self.kind}', self.pos)
        start = self.pos
        self.pos += count
        if not numeric:
            self.metadata += count
        return self.raw[start:self.pos]

    def number(self, fmt):
        return struct.unpack('<' + fmt, self.take(struct.calcsize('<' + fmt)))[0]

    def u8(self): return self.number('B')
    def u16(self): return self.number('H')
    def u32(self): return self.number('I')
    def u64(self): return self.number('Q')

    def f64(self):
        value = self.number('d')
        require(math.isfinite(value), 'NonCanonical', 'finite-float', self.pos)
        return value

    def count(self, maximum):
        return bounded(self.u32(), 0, maximum, 'CapacityExceeded', 'count')

    def blob(self, maximum=65536, vocabulary=False):
        return self.take(self.count(maximum), numeric=vocabulary)

    def string(self, maximum=4096):
        raw = self.blob(maximum)
        try:
            return raw.decode('utf-8', 'strict')
        except UnicodeDecodeError:
            raise ContractError('NonCanonical', 'utf8', self.pos) from None

    def enum(self, size, values):
        value = self.u8() if size == 1 else self.u16()
        require(value in values, 'NonCanonical', 'enum', self.pos)
        return value

    def nonzero(self, size=32):
        value = self.take(size)
        require(any(value), 'InvalidProvenance', 'nonzero-reference', self.pos)
        return value

    def optional(self, size=32):
        return self.nonzero(size) if self.enum(1, (0, 1)) else None

    def end(self):
        require(self.pos == len(self.raw), 'NonCanonical', 'component-trailing', self.pos)


def frame(raw):
    """Hash-checked framing alone never implies model validity."""
    require(len(raw) >= 96, 'Truncated', 'header')
    require(raw[:8] == b'WDBXMOD1', 'BadMagic', 'magic')
    major, minor, size, total, count, flags = struct.unpack_from('<HHIQII', raw, 8)
    require((major, minor) == (1, 0), 'UnsupportedContainerVersion', 'version')
    require(size == 96 and flags == 0 and raw[64:96] == bytes(32), 'NonCanonical', 'header')
    bounded(total, 96, 64 * 1024**2, 'CapacityExceeded', 'artifact')
    bounded(count, 8, 32, 'CapacityExceeded', 'directory-count')
    require(len(raw) >= total, 'Truncated', 'artifact')
    require(len(raw) <= total, 'TrailingBytes', 'artifact')
    end = 96 + 80 * count
    require(end <= total, 'Truncated', 'directory')
    directory = raw[96:end]
    require(digest(directory) == raw[32:64], 'IntegrityMismatch', 'directory')
    entries, previous = {}, 0
    for index in range(count):
        entry = directory[index * 80:(index + 1) * 80]
        kind, major, minor, flags, reserved, offset, length = struct.unpack_from('<IHHIIQQ', entry)
        require(kind > previous and reserved == 0 and entry[64:] == bytes(16) and flags in (0, 1), 'NonCanonical', 'directory-entry')
        if kind in SCHEMAS:
            require(flags == 1, 'MissingComponent', 'required-flag')
            require((major, minor) == (1, 0), 'UnsupportedComponentVersion', 'component-version')
        else:
            require(flags == 0, 'UnsupportedRequiredComponent', 'kind')
        require(length > 0 and offset == end and length <= total - end, 'NonCanonical', 'payload-range')
        end += length
        entries[kind] = {'bytes': raw[offset:end], 'sha': entry[32:64], 'version': [major, minor]}
        previous = kind
    require(all(k in entries for k in SCHEMAS), 'MissingComponent', 'required-kinds')
    require(end == total, 'NonCanonical', 'payload-end')
    for kind, entry in entries.items():
        require(digest(entry['bytes']) == entry['sha'], 'IntegrityMismatch', f'component{kind}')
    return {'stage': 'FramedPackage', 'entries': entries, 'raw': raw}


def architecture(r):
    require(r.string() == 'wdbx-spatial-neural-v1', 'NonCanonical', 'architecture')
    require(r.string() == 'cpu-scalar-f32params-f64accum-v1', 'NonCanonical', 'numeric-profile')
    a = dict(purpose=r.enum(1, (0, 1)), learning=r.enum(1, (0, 1)))
    require(r.u16() == 1 and r.u64() == 3, 'NonCanonical', 'operators/features')
    names = ('nodes', 'edges', 'hidden', 'relations', 'vocab', 'steps', 'sequence', 'generated', 'batch', 'node_capacity', 'edge_capacity', 'degree', 'scalars')
    maxima = (128, 1024, 64, 8, 4096, 4, 128, 128, 4, 128, 1024, 16, 1000000)
    minima = (2, 1, 1, 1, 260, 1, 2, 1, 1, 2, 1, 1, 1)
    for name, minimum, maximum in zip(names, minima, maxima):
        a[name] = bounded(r.u32(), minimum, maximum, 'CapacityExceeded', name)
    for name, maximum in [('session', 256 * 1024**2), ('loaded_trainer', 512 * 1024**2), ('ops', 500000000)]:
        a[name] = bounded(r.u64(), 1, maximum, 'CapacityExceeded', name)
    a['scale'], a['radius'] = r.f64(), r.f64()
    require(a['scale'] > 0 and a['radius'] == 4 and a['nodes'] <= a['node_capacity'] and a['edges'] <= a['edge_capacity'], 'NonCanonical', 'architecture-bounds')
    return a


def topology(r):
    require(r.u16() == 1, 'InvalidTopology', 'coordinate-frame')
    node_high, edge_high = r.u64(), r.u64()
    relations = [(r.u32(), r.string(64)) for _ in range(r.count(8))]
    nodes = [(r.u64(), r.enum(1, range(4)), r.optional()) for _ in range(r.count(128))]
    edges = [(r.u64(), r.u64(), r.u64(), r.u32()) for _ in range(r.count(1024))]
    require([i for i, _ in relations] == list(range(len(relations))) and all(label for _, label in relations) and len(set(label for _, label in relations)) == len(relations), 'InvalidTopology', 'relations')
    for rows, high, name in [(nodes, node_high, 'node'), (edges, edge_high, 'edge')]:
        ids = [row[0] for row in rows]
        require(ids and ids == sorted(set(ids)) and 0 < ids[0] <= ids[-1] <= high < MASK, 'InvalidTopology', name + '-ids')
    require(sum(n[1] == 0 for n in nodes) == 1 and sum(n[1] == 3 for n in nodes) == 1, 'InvalidTopology', 'roles')
    require(all(n[2] is None or n[1] == 2 for n in nodes), 'InvalidTopology', 'source-role')
    require(len(set(e[1:] for e in edges)) == len(edges), 'InvalidTopology', 'edge-triples')
    return dict(relations=relations, nodes=nodes, edges=edges, node_high=node_high, edge_high=edge_high)


def tensors(r):
    require(r.count(16) == 16, 'InvalidTensor', 'membership')
    result, total = [], 0
    for expected in range(1, 17):
        ident, dtype, rank = r.u16(), r.enum(1, (1, 2)), r.u8()
        require(ident == expected and 1 <= rank <= 3, 'InvalidTensor', 'id/rank')
        axes, count = [], 1
        for _ in range(rank):
            kind = r.enum(1, range(1, 8))
            length = bounded(r.u32(), 1, 4096, 'InvalidTensor', 'axis-length')
            ids = [r.u64() for _ in range(length)] if kind in (1, 2) else []
            count *= length
            require(count <= 1000000, 'CapacityExceeded', 'tensor-scalars')
            axes.append((kind, length, ids))
        total += count
        require(total <= 1000000, 'CapacityExceeded', 'total-scalars')
        length = r.u64()
        require(length == count * (4 if dtype == 1 else 8), 'InvalidTensor', 'data-length')
        values = [v[0] for v in struct.iter_unpack('<f' if dtype == 1 else '<d', r.take(length, numeric=True))]
        require(all(math.isfinite(v) for v in values), 'InvalidTensor', 'finite')
        require(dtype == CATALOG[expected - 1][0] and tuple(a[0] for a in axes) == CATALOG[expected - 1][1], 'InvalidTensor', 'semantic-axes')
        require(expected != 2 or all(-4 <= v <= 4 for v in values), 'InvalidTensor', 'coordinate-radius')
        result.append(dict(id=ident, dtype=dtype, axes=axes, count=count, values=values))
    return result


def tokenizer(r):
    require(len(r.raw) <= 1024**2, 'CapacityExceeded', 'tokenizer-component')
    require(r.string() == 'wdbx-byte-bpe-v1' and r.u8() == 0 and r.u8() == 1 and r.u16() == 0, 'InvalidTokenizer', 'profile')
    require([r.u32() for _ in range(4)] == [0, 1, 2, 3], 'InvalidTokenizer', 'special-ids')
    corpus, split = r.nonzero(), r.nonzero()
    require(r.u32() == 2, 'InvalidTokenizer', 'pair-frequency')
    requested = bounded(r.u32(), 260, 4096, 'InvalidTokenizer', 'requested-vocab')
    vocab = [r.blob(vocabulary=True) for _ in range(r.count(4096))]
    merges = [(r.u32(), r.u32(), r.u32()) for _ in range(r.count(3836))]
    require(260 <= len(vocab) <= requested and len(merges) == len(vocab) - 260, 'InvalidTokenizer', 'vocab-count')
    require(vocab[:4] == [b''] * 4 and vocab[4:260] == [bytes([i]) for i in range(256)], 'InvalidTokenizer', 'base-vocabulary')
    seen = set()
    for rank, (left, right, result) in enumerate(merges):
        require(result == rank + 260 and 4 <= left < result and 4 <= right < result and (left, right) not in seen, 'InvalidTokenizer', 'merge-order')
        require(len(vocab[left]) + len(vocab[right]) <= 65536 and vocab[result] == vocab[left] + vocab[right], 'InvalidTokenizer', 'merge-bytes')
        seen.add((left, right))
    return dict(vocab=vocab, merges=merges, corpus=corpus, split=split)


def objective(r):
    require([r.u16() for _ in range(4)] == [1] * 4, 'InvalidObjective', 'objective-profiles')
    text, graph, l2 = r.f64(), r.f64(), r.f64()
    require(text == 1 and graph >= 0 and l2 >= 0, 'InvalidObjective', 'weights')
    require(r.u16() == 1 and r.u16() == 1, 'InvalidObjective', 'policies')
    candidates = bounded(r.u32(), 1, 128, 'InvalidObjective', 'candidates')
    return dict(candidates=candidates, dataset=r.nonzero(), split=r.nonzero(), preparation=r.nonzero())


def optimizer(r):
    if r.enum(1, (0, 1)) == 0:
        return None
    require(r.u16() == 1 and r.u16() == 1, 'InvalidOptimizer', 'profiles')
    step = r.u64()
    lr, b1, b2, eps, decay, clip = [r.f64() for _ in range(6)]
    require(step < MASK and 0 < lr <= 1 and 0 <= b1 < 1 and 0 <= b2 < 1 and 0 < eps <= 1 and 0 <= decay <= 1 and 0 < clip <= 1000, 'InvalidOptimizer', 'hyperparameters')
    require(r.count(16) == 16, 'InvalidOptimizer', 'membership')
    counts, total = [], 0
    for expected in range(1, 17):
        ident, count = r.u16(), r.u64()
        total += count
        require(ident == expected and 1 <= count <= 1000000 and total <= 1000000, 'InvalidOptimizer', 'moment-id/count')
        first = [v[0] for v in struct.iter_unpack('<d', r.take(count * 8, numeric=True))]
        second = [v[0] for v in struct.iter_unpack('<d', r.take(count * 8, numeric=True))]
        require(all(math.isfinite(v) for v in first) and all(math.isfinite(v) and v >= 0 for v in second), 'InvalidOptimizer', 'moments')
        require(step != 0 or all(v == 0 for v in first + second), 'InvalidOptimizer', 'initial-moments')
        counts.append(count)
    return dict(step=step, counts=counts)


def rotl(value, count):
    return ((value << count) | (value >> (64 - count))) & MASK


def seed_stream(seed, stream):
    state, words = (seed + 0x9e3779b97f4a7c15 * stream) & MASK, []
    for _ in range(4):
        state = (state + 0x9e3779b97f4a7c15) & MASK
        z = state
        z = ((z ^ (z >> 30)) * 0xbf58476d1ce4e5b9) & MASK
        z = ((z ^ (z >> 27)) * 0x94d049bb133111eb) & MASK
        words.append(z ^ (z >> 31))
    return words


def xoshiro(words):
    s0, s1, s2, s3 = words
    output, t = (rotl((s1 * 5) & MASK, 7) * 9) & MASK, (s1 << 17) & MASK
    s2 ^= s0
    s3 ^= s1
    s1 ^= s2
    s0 ^= s3
    s2 ^= t
    s3 = rotl(s3, 45)
    return output, [s0, s1, s2, s3]


def rng_order(r):
    if r.enum(1, (0, 1)) == 0:
        return None
    require(r.u16() == 1 and r.u16() == 1, 'InvalidRngOrder', 'profiles')
    seed = r.u64()
    require(r.count(5) == 5, 'InvalidRngOrder', 'streams')
    for stream in range(1, 6):
        ident, state, consumed = r.u8(), [r.u64() for _ in range(4)], r.u64()
        require(ident == stream and any(state) and consumed < MASK, 'InvalidRngOrder', 'stream-state')
        require(consumed != 0 or state == seed_stream(seed, stream), 'InvalidRngOrder', 'initial-state')
    require([r.u16() for _ in range(3)] == [1, 1, 0], 'InvalidRngOrder', 'sampler/boundary')
    result = dict(dataset=r.nonzero(), split=r.nonzero(), preparation=r.nonzero())
    result['step'], epoch = r.u64(), r.u64()
    count, cursor, batch = r.u32(), r.u32(), r.u32()
    order = [r.u32() for _ in range(r.count(65536))]
    accepted, next_step = r.u64(), r.u64()
    require(1 <= count <= 65536 and len(order) == count and sorted(order) == list(range(count)) and cursor < count and 1 <= batch <= 4, 'InvalidRngOrder', 'permutation/cursor')
    require(result['step'] < MASK and (next_step == 0 or next_step > result['step']), 'InvalidRngOrder', 'counters')
    result.update(batch=batch, accepted=accepted, next_step=next_step)
    return result


def provenance(r):
    p = dict(model=r.nonzero(16), partition=r.nonzero(), origin=r.enum(1, (0, 1, 2)))
    p['parent'], parent_state, derivation, operation = r.optional(), r.optional(), r.optional(), r.optional(16)
    p['audience'], p['policy'], p['epoch'] = r.nonzero(), r.optional(), r.u64()
    cohorts = [(r.string(), r.string(), r.nonzero()) for _ in range(r.count(32))]
    roots = [(r.nonzero(), r.nonzero(), r.enum(1, range(5))) for _ in range(r.count(1024))]
    licenses = [r.nonzero() for _ in range(r.count(64))]
    report, restrictions = r.optional(), r.u32()
    require((p['parent'] is None) == (parent_state is None) and (p['parent'] is None or operation is not None), 'InvalidProvenance', 'lineage')
    names = [row[0].encode('utf-8') for row in cohorts]
    require(all(row[0] and row[1] for row in cohorts) and names == sorted(set(names)), 'InvalidProvenance', 'cohorts')
    keys = [(kind, content) for content, _, kind in roots]
    require(keys == sorted(set(keys)) and licenses == sorted(set(licenses)), 'InvalidProvenance', 'root/license-order')
    require(restrictions <= 3 and (p['origin'] != 0 or restrictions & 1) and (p['origin'] == 0 or licenses), 'InvalidProvenance', 'origin/restrictions')
    p['roots'] = set(keys)
    return p


PARSERS = (architecture, topology, tensors, tokenizer, objective, optimizer, rng_order, provenance)


def validate(raw):
    framed = frame(raw)
    components, metadata = [], 0
    for kind, parser in enumerate(PARSERS, 1):
        r = Reader(framed['entries'][kind]['bytes'], kind)
        components.append(parser(r))
        r.end()
        metadata += r.metadata
    metadata += sum(len(e['bytes']) for k, e in framed['entries'].items() if k > 8)
    require(metadata <= 2 * 1024**2, 'CapacityExceeded', 'metadata')
    a, top, ts, tok, obj, opt, rng, prov = components
    require(len(top['nodes']) == a['nodes'] and len(top['edges']) == a['edges'] and len(top['relations']) == a['relations'], 'InvalidTopology', 'counts')
    nodes, edges = [n[0] for n in top['nodes']], [e[0] for e in top['edges']]
    incoming = {}
    for _, source, destination, relation in top['edges']:
        require(source in nodes and destination in nodes and relation < a['relations'], 'InvalidTopology', 'endpoint/relation')
        incoming[destination] = incoming.get(destination, 0) + 1
    require(max(incoming.values()) <= a['degree'], 'InvalidTopology', 'degree')
    lengths = {1: a['nodes'], 2: a['edges'], 3: a['relations'], 4: a['vocab'], 5: a['hidden'], 6: a['hidden'], 7: 3}
    for tensor in ts:
        for kind, length, ids in tensor['axes']:
            require(length == lengths[kind] and (kind != 1 or ids == nodes) and (kind != 2 or ids == edges), 'InvalidTensor', 'axis-identity')
    require(sum(t['count'] for t in ts) <= a['scalars'], 'CapacityExceeded', 'declared-scalars')
    require(len(tok['vocab']) == a['vocab'], 'InvalidTokenizer', 'architecture-vocab')
    require(obj['candidates'] <= a['node_capacity'], 'InvalidObjective', 'candidate-capacity')
    require((opt is not None) == (a['purpose'] == 1), 'InvalidOptimizer', 'purpose')
    require((rng is not None) == (a['purpose'] == 1), 'InvalidRngOrder', 'purpose')
    if opt is not None:
        require(opt['counts'] == [t['count'] for t in ts], 'InvalidOptimizer', 'moment-counts')
        require(opt['step'] == rng['step'] and all(obj[k] == rng[k] for k in ('dataset', 'split', 'preparation')) and rng['batch'] <= a['batch'], 'InvalidRngOrder', 'state-crosslinks')
        require(a['learning'] != 0 or rng['accepted'] == rng['next_step'] == 0, 'InvalidRngOrder', 'offline-proposals')
    required = {(0, tok['corpus']), (2, tok['split']), (1, obj['dataset']), (2, obj['split']), (4, obj['preparation'])}
    required.update((3, n[2]) for n in top['nodes'] if n[2] is not None)
    require(required <= prov['roots'], 'InvalidProvenance', 'root-coverage')
    require((prov['policy'] is not None and prov['epoch'] > 0) if a['learning'] else (prov['policy'] is None and prov['epoch'] == 0), 'InvalidProvenance', 'policy-profile')
    state = digest(b'wdbx-model-state-v1\0' + b''.join(struct.pack('<IHH', k, 1, 0) + framed['entries'][k]['sha'] for k in range(1, 8)))
    parent = b'\0' if prov['parent'] is None else b'\1' + prov['parent']
    generation = digest(b'wdbx-model-generation-v1\0' + prov['model'] + prov['partition'] + parent + state + framed['entries'][8]['sha'])
    context = {k + '_digest': obj[k].hex() for k in ('dataset', 'split', 'preparation')}
    context.update(objective_digest=framed['entries'][5]['sha'].hex(), audience_digest=prov['audience'].hex(), policy_digest=prov['policy'].hex() if prov['policy'] else None, policy_epoch=prov['epoch'])
    return dict(stage='ValidatedModelState', artifact_sha256=digest(raw).hex(), package_digest=digest(b'wdbx-model-package-v1\0' + raw).hex(), state_root=state.hex(), generation_id=generation.hex(), purpose=a['purpose'], tokenizer=tok, resume_context=context)


def check_resume(model, context, authorized):
    require(model['purpose'] == 1, 'NotResumable', 'purpose')
    require(authorized and context == model['resume_context'], 'ResumeMismatch', 'external-context')
    return 'ResumableState'


def encode(raw, tok):
    """Slow diagnostic BPE reference; production tokenizer remains N02."""
    require(len(raw) <= 65536, 'CapacityExceeded', 'encode-input')
    tokens, visits = [byte + 4 for byte in raw], 0
    for left, right, result in tok['merges']:
        output, index = [], 0
        while index < len(tokens):
            visits += 1
            require(visits <= 8000000, 'CapacityExceeded', 'encode-work')
            if index + 1 < len(tokens) and tokens[index:index + 2] == [left, right]:
                output.append(result)
                index += 2
            else:
                output.append(tokens[index])
                index += 1
        tokens = output
    return tokens


def main():
    require(len(sys.argv) == 3 and sys.argv[1] in ('validate', 'frame'), 'NonCanonical', 'arguments')
    raw = bytes.fromhex(Path(sys.argv[2]).read_text())
    result = validate(raw) if sys.argv[1] == 'validate' else frame(raw)
    print(json.dumps({k: v for k, v in result.items() if k not in ('entries', 'raw', 'tokenizer')}))


if __name__ == '__main__':
    try:
        main()
    except ContractError as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
