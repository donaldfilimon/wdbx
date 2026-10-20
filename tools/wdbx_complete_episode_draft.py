#!/usr/bin/env python3
"""Independent synthetic private-draft mapping evidence, not an ABI protocol."""
import hashlib
import json
import re
import sys

class FixtureError(Exception):
    pass

def fail():
    raise FixtureError()

def obj(value, required, optional=()):
    if type(value) is not dict or set(value) - set(required) - set(optional) or set(required) - set(value):
        fail()
    return value

def integer(value, lo=0, hi=(1 << 64)-1):
    if type(value) is not int or not lo <= value <= hi:
        fail()
    return value

def token(value):
    if type(value) is not str or not re.fullmatch(r'[A-Za-z0-9._:-]{1,128}', value):
        fail()
    return value

def binary(value, size):
    if type(value) is not str or not re.fullmatch('[0-9a-f]{%d}' % (size*2), value):
        fail()
    return bytes.fromhex(value)

def uuid(value):
    value = binary(value, 16)
    if not any(value) or value[8] & 192 != 128 or value[6] >> 4 not in (4, 7):
        fail()
    return value

def tag(value, names):
    if type(value) is not str or value not in names:
        fail()
    return [names.index(value)]

DOMAINS = ('CompleteV1', 'LegacyEpisodeV1')
ARTIFACTS = ('StateSummary', 'Prediction', 'Outcome', 'Observation', 'Model', 'Calibration', 'Provenance', 'ReservoirEmbedding', 'Evidence', 'Authority')
RESOURCES = ('Tokens', 'CpuMicros', 'MemoryBytes', 'NetworkBytes', 'EnergyMicrojoules')
ACTORS = {'HumanSubject':'human_subject', 'OrganizationOwner':'organization_owner', 'GuildOwner':'guild_owner', 'GuildAdministrator':'guild_administrator', 'GuildManager':'guild_manager', 'Service':'service'}
DIMENSIONS = ('cryptographic', 'source', 'calibration', 'outcome', 'compatibility', 'regime', 'lifecycle', 'confidence')

def head(major, n):
    if n < 24:
        return bytes([major*32+n])
    for marker, width in ((24,1),(25,2),(26,4),(27,8)):
        if n < 1 << (width*8):
            return bytes([major*32+marker]) + n.to_bytes(width, 'big')
    fail()

def cbor(value):
    if type(value) is int:
        return head(0, value) if value >= 0 else head(1, -1-value)
    if type(value) is str:
        data = value.encode('utf-8')
        return head(3, len(data)) + data
    if type(value) is bytes:
        return head(2, len(value)) + value
    if type(value) is list:
        return head(4, len(value)) + b''.join(map(cbor, value))
    if type(value) is dict:
        entries = [(cbor(k), cbor(v)) for k,v in value.items()]
        entries.sort(key=lambda pair:(len(pair[0]), pair[0]))
        return head(5, len(entries)) + b''.join(k+v for k,v in entries)
    fail()

def collection(value, mapper, cap, order=None):
    if type(value) is not list or len(value) > cap:
        fail()
    mapped = [mapper(v) for v in value]
    encoded = [cbor(v) for v in mapped]
    if len(set(encoded)) != len(encoded):
        fail()
    return sorted(mapped, key=order or cbor)

def binding(v):
    obj(v, ('tenant','scope'))
    s = obj(v['scope'], ('variant',), ('guild','member','user','organization'))
    variant = s['variant']
    fields = {'Guild':('guild',), 'GuildMember':('guild','member'), 'DirectMessage':('user',), 'Organization':('organization',)}
    if variant not in fields:
        fail()
    obj(s, ('variant',)+fields[variant])
    return {'tenant':token(v['tenant']), 'scope':[('Guild','GuildMember','DirectMessage','Organization').index(variant), {k:token(s[k]) for k in fields[variant]}]}

def version(v):
    obj(v, ('component','revision','digest'))
    return {'component':token(v['component']), 'revision':token(v['revision']), 'digest':binary(v['digest'],32)}

def local(v):
    obj(v, ('binding','domain','digest'))
    return {'binding':binding(v['binding']), 'domain':tag(v['domain'],DOMAINS), 'digest':binary(v['digest'],32)}

def artifact(v):
    obj(v, ('binding','digest','byte_length','kind','format'))
    return {'binding':binding(v['binding']), 'digest':binary(v['digest'],32), 'byte_length':integer(v['byte_length'],hi=1<<40), 'kind':tag(v['kind'],ARTIFACTS), 'format':version(v['format'])}

def foreign(v):
    obj(v, ('origin_organization','origin_binding','domain','digest','import_receipt'))
    return {'origin_organization':token(v['origin_organization']), 'origin_binding':binding(v['origin_binding']), 'domain':tag(v['domain'],DOMAINS), 'digest':binary(v['digest'],32), 'import_receipt':local(v['import_receipt'])}

def consulted(v):
    obj(v, ('variant','reference'))
    if v['variant'] == 'Local':
        return [0, {'reference':local(v['reference'])}]
    if v['variant'] == 'Foreign':
        return [1, {'reference':foreign(v['reference'])}]
    fail()

def estimate(v, mapper):
    if type(v) is dict and set(v) == {'unknown'} and v['unknown'] is True:
        return [0]
    obj(v, ('known',))
    return [1, mapper(v['known'])]

def ppm(v):
    return integer(v, hi=1000000)

def posterior(v):
    if type(v) is not list or not 1 <= len(v) <= 32:
        fail()
    pairs = []
    for p in v:
        if type(p) is not list or len(p) != 2:
            fail()
        pairs.append([token(p[0]), ppm(p[1])])
    if len({p[0] for p in pairs}) != len(pairs) or sum(p[1] for p in pairs) != 1000000:
        fail()
    return sorted(pairs)

def resources(v):
    def reading(r):
        obj(r, ('resource','amount'))
        return {'resource':tag(r['resource'],RESOURCES), 'amount':integer(r['amount'])}
    mapped = collection(v, reading,16)
    if len({r['resource'][0] for r in mapped}) != len(mapped):
        fail()
    return mapped

def versions(v):
    mapped = collection(v,version,32,lambda x:x['component'])
    if len({x['component'] for x in mapped}) != len(mapped):
        fail()
    return mapped

def action(v):
    obj(v, ('capability','action','parameter_classes','phase'))
    return {'capability':version(v['capability']), 'action':token(v['action']), 'parameter_classes':collection(v['parameter_classes'],token,32,lambda x:x), 'phase':tag(v['phase'],('Proposed','Performed'))}

def measurement(v):
    obj(v, ('unit','value'))
    unit = tag(v['unit'],('Unitless','Count','Milliseconds','Bytes','Microjoules'))
    amount = integer(v['value'], -(1<<63), (1<<63)-1)
    if unit[0] in (1,3) and (amount < 0 or amount % 1000000):
        fail()
    return {'unit':unit, 'value':amount}

def consent(v):
    obj(v, ('binding','epoch','policy','attestation'))
    return {'binding':binding(v['binding']), 'epoch':integer(v['epoch']), 'policy':version(v['policy']), 'attestation':local(v['attestation'])}

def retention(v):
    fields = {'Ephemeral':(), 'Session':('session','expires_at'), 'Operational':('expires_at',), 'Durable':('privacy_basis',), 'MandatoryIncident':('obligation',)}
    obj(v, ('variant',), ('session','expires_at','privacy_basis','obligation'))
    variant = v['variant']
    if variant not in fields:
        fail()
    obj(v, ('variant',)+fields[variant])
    index = ('Ephemeral','Session','Operational','Durable','MandatoryIncident').index(variant)
    if not index:
        return [0]
    mappers = {'session':token,'expires_at':integer,'privacy_basis':local,'obligation':local}
    return [index, {k:mappers[k](v[k]) for k in fields[variant]}]

def keyref(v):
    obj(v, ('binding','key_id'))
    return {'binding':binding(v['binding']), 'key_id':token(v['key_id'])}

def edge(v):
    obj(v, ('relation','target','basis'))
    return {'relation':tag(v['relation'],('Supersedes','Contradicts','Revokes','Quarantines','Resolves','Redacts','Forgets')), 'target':local(v['target']), 'basis':local(v['basis'])}

def signer(v):
    if type(v) is not str or not re.fullmatch(r'ed25519:[0-9a-f]{64}',v):
        fail()
    return v

def actor(v):
    obj(v, ('principal','kind'))
    if type(v['kind']) is not str or v['kind'] not in ACTORS:
        fail()
    return {'principal':token(v['principal']), 'kind':ACTORS[v['kind']]}

def monotonic(v):
    obj(v, ('clock','start','end'))
    result = {'clock':token(v['clock']), 'start':integer(v['start']), 'end':integer(v['end'])}
    if result['start'] > result['end']:
        fail()
    return result

def group(v, required, optional=None):
    optional = optional or {}
    obj(v, required,optional)
    return {k:mapper(v[k]) for k,mapper in dict(required,**optional).items() if k in v}

def actions(v):
    if type(v) is not list or len(v)>128:
        fail()
    return [action(x) for x in v]

def maturity(v):
    if type(v) is not str or v not in ('C0','C1','C2','C3','C4','C5','C6','C7'):
        fail()
    return v

def map_fixture(v):
    names = ('identity','time','origin','context','decision','outcome','authority','evidence','lifecycle','provenance')
    obj(v,names)
    est = lambda mapper: lambda x:estimate(x,mapper)
    seq = lambda mapper,cap:lambda x:collection(x,mapper,cap)
    specs = [
        ({'episode_id':uuid,'request_id':uuid,'binding':binding,'contract':version},{}),
        ({'acquired_at':integer,'received_at':integer},{'monotonic':monotonic}),
        ({'source_node':token,'actor':actor,'signer':signer},{}),
        ({'task_id':uuid,'regime':token,'regime_posterior':est(posterior),'habituation':est(ppm),'resources':resources},{'summary':artifact,'reservoir':artifact}),
        ({'route':lambda x:tag(x,('NoModel','LocalModel','CloudModel','Specialist','Deterministic')),'prediction':est(artifact),'actions':actions,'uncertainty':est(ppm),'risk':est(ppm),'novelty':est(ppm)},{}),
        ({'observed':est(artifact),'utility':est(measurement),'safety_effect':lambda x:tag(x,('Unknown','NoViolationObserved','InterlockTriggered','ViolationObserved')),'costs':resources,'rollback':lambda x:tag(x,('NotApplicable','NotAttempted','Attempted','ExactRestoreObserved','Failed','Unknown'))},{}),
        ({'policy':version,'safety_policy':version,'capabilities':versions,'models':versions,'calibrations':versions,'approvals':seq(local,64)},{'consent':consent}),
        ({'maturity':maturity,'source_refs':seq(artifact,256),'observation_refs':seq(artifact,256),'dimensions':lambda x:group(x,{k:est(artifact) for k in DIMENSIONS})},{}),
        ({'retention':retention,'holds':seq(local,64),'edges':seq(edge,64)},{'deletion_key':keyref}),
        ({'causal_parents':seq(local,64),'consulted_parents':seq(consulted,256),'records':seq(artifact,256)},{}),
    ]
    result = {0:'wdbx-complete-episode-v1',1:1}
    for index,(name,(required,optional)) in enumerate(zip(names,specs),2):
        result[index] = group(v[name],required,optional)
    return result

def unique_object(pairs):
    result = {}
    for k,v in pairs:
        if k in result:
            fail()
        result[k] = v
    return result

def request(line):
    try:
        value = json.loads(line,object_pairs_hook=unique_object,parse_constant=lambda _:fail())
        data = cbor(map_fixture(value))
        if len(data) > 1048576:
            fail()
        return {'ok':True,'bytes':data.hex(),'digest':hashlib.sha256(data).hexdigest()}
    except (FixtureError, ValueError, TypeError, KeyError, OverflowError, RecursionError):
        return {'ok':False,'error':'FixtureError'}

def main():
    if sys.argv[1:] != ['differential']:
        return 2
    for line in sys.stdin.buffer:
        try:
            response = request(line.decode('utf-8'))
        except UnicodeDecodeError:
            response = {'ok':False,'error':'FixtureError'}
        print(json.dumps(response,separators=(',',':')),flush=True)
    return 0

if __name__ == '__main__':
    sys.exit(main())
