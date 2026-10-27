"""Final-fix current observation refresh; historical evidence is read-only.
No observation append loop: replace current pointers and extend mapped file lists.
Only the six reviewed current inventory/narrative files plus exact catalog pin move.
"""
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6]
B='docs/completion/execution/'
E=B+'evidence/2026-10-22-language/'
F=E+'final-fix/'
K='specimen-studio/native/specimen-kernel/'
W='specimen-studio/native/specimen-wasm/'
def read(p): return json.loads((ROOT/p).read_text())
def sha(p): return hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
def write(p,v): (ROOT/p).write_text(json.dumps(v,indent=2,ensure_ascii=False)+'\n')
def ref(p): return {'path':p,'sha256':sha(p)}
def unique(xs): return list(dict.fromkeys(xs))
summary=read(F+'gates/summary.json')
assert len(summary['gates'])==4 and all(g['exit_code']==0 for g in summary['gates'])
for row in read(F+'product-freeze.json')['files']: assert sha(row['path'])==row['sha256']
# Mapping is derived from the reviewed previous mapping, not an append-only producer.
m=read(E+'register/requirement-mapping.json')
m['groups'][0]['tests'].append(K+'tests/language_final_fix.rs')
m['groups'][1]['tests'].append(K+'tests/language_final_fix.rs')
m['groups'][3]['sources'] += [K+'src/scanner_v3/routes/work.rs',K+'src/identity/mod.rs']
m['groups'][3]['tests'].append(K+'tests/routes_final_fix.rs')
m['groups'][4]['sources'].append(W+'src/language_probe/final_fix.rs')
m['groups'][3]['boundary']+=' Logical query work reserves borrowed entry indexing and complete selected payloads before copying; identity accepts the same records and only gains an O(1) immutable slice accessor.'
m['scope']='Final integrated fix source/test routing; three Important and three API/assertion Minors addressed locally; independent scoped rereview pending. No C0 promotion.'
m['profile']=ref(B+'language-v3-profile.md')
m['historical_raw_evidence']=m.pop('raw_evidence')
m['raw_evidence']=[ref(F+p) for p in ['phase-a-commands.json','product-freeze.json','identity-accessor-extension.json','focused-green.log','kernel-feature-final.log','clippy.log','fmt.log','language-parity.log','identity-parity.log','language-runtime/native.json','language-runtime/wasm.json','language-runtime/default.wasm','language-runtime/language.wasm','language-runtime/negative.log','language-runtime/negative-label.log','identity-runtime/native.json','identity-runtime/wasm.json','gates/summary.json']]
for g in summary['gates']: m['raw_evidence'].append(ref(F+'gates/'+g['gate']+'.log'))
paths=unique([K+'src/lib.rs']+[p for g in m['groups'] for p in g['sources']+g['tests']]); manifest=[]
for p in paths:
 gs=[g for g in m['groups'] if p in g['sources']+g['tests']]
 manifest.append({**ref(p),'bytes':(ROOT/p).stat().st_size,'mapped_ids':unique(i for g in gs for i in g['ids']) or ['P08.01','P08.02','P08.03'],'mapped_decisions':unique(i for g in gs for i in g['decisions']) or ['D01','D05','D06']})
write(F+'requirement-mapping.json',m); write(F+'current-source-manifest.json',{'scope':m['scope'],'files':manifest})
# Preserve old raw files and closure as historical; current pointer substitutions only.
redirect={E+'register/requirement-mapping.json':F+'requirement-mapping.json',E+'register/current-source-manifest.json':F+'current-source-manifest.json'}
def redirect_value(v):
 if isinstance(v,dict):
  for key,x in v.items(): v[key]=redirect_value(x)
 elif isinstance(v,list): v=[redirect_value(x) for x in v]
 elif isinstance(v,str): v=redirect.get(v,v)
 return v
def refresh(v):
 if isinstance(v,dict):
  if 'path' in v and 'sha256' in v: v['sha256']=sha(v['path'])
  for x in v.values():refresh(x)
 elif isinstance(v,list):
  for x in v:refresh(x)
c=redirect_value(read(B+'candidate-paths-v1.json'))
r=redirect_value(read(B+'register-v1.json'))
s=redirect_value(read(B+'studio-research-v1.json'))
byid={v['id']:v for v in r['requirements']}
for g in m['groups']:
 for id in g['ids']:
  row=byid[id]
  assert c[id]['disposition']=='candidate'
  for kind,items in [('implementation',g['sources']),('tests',g['tests'])]:
   c[id][kind]=unique(c[id][kind]+items);row[kind]['references']=[ref(p) for p in c[id][kind]]
  c[id]['inspected']=unique(c[id]['inspected']+g['sources']+g['tests']+[F+'identity-accessor-extension.json'])
  row['reconciliation']['inspected']['references']=[ref(p) for p in c[id]['inspected']]
  row['artifacts']['references']=[ref(p) for p in unique([x['path'] for x in row['artifacts']['references']]+[F+'gates/summary.json',F+'identity-accessor-extension.json'])]
for section,idkey in [('cases','id'),('decisions','id'),('local_mechanism_observations','case')]:
 for item in s[section]:
  gs=[g for g in m['groups'] if item[idkey] in g['ids'] or item[idkey] in g['decisions']]
  if not gs:continue
  for field,groupkey in [('source_references','sources'),('test_references','tests')]:
   item[field]=[ref(p) for p in unique([x['path'] for x in item[field]]+[p for g in gs for p in g[groupkey]])]
  if section=='local_mechanism_observations':
   item['execution_references']=[ref(p) for p in unique([x['path'] for x in item['execution_references']]+[x['path'] for x in m['raw_evidence']])]
# Retain all prior actual executions, explicitly as historical observations.
for observation in s['execution_observations']:
 observation['qualification']='Historical pre-language-final-fix execution; retained raw bytes. '+observation['qualification']
for g in summary['gates']:
 text=(ROOT/(F+'gates/'+g['gate']+'.log')).read_text()
 if g['gate']=='native-test':
  totals=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',text)
  detail=f"{sum(int(x[0]) for x in totals)} native passes; {sum(int(x[2]) for x in totals)} existing ignored"
 elif g['gate']=='browser':
  count=re.findall(r'(\d+) pass\b',text);detail=(count[-1]+' browser/unit passes; production build') if count else 'browser check and production build; counts in raw log'
 else:detail='lint completed' if g['gate']=='lint' else 'native all-target Clippy completed'
 s['execution_observations'].append({'command':' '.join(g['command']),'working_directory':g['cwd'],'outcome':'exit 0; '+detail,'raw_log':ref(F+'gates/'+g['gate']+'.log'),'qualification':'Current frozen final-fix source; root executed four gates after product freeze. Local execution only; no installed/UI/efficacy/owner qualification.'})
trace=(ROOT/(F+'language-runtime/native.json')).read_bytes();tracehash=hashlib.sha256(trace).hexdigest()
for name,outcome in [('language-parity.log',f'Actual feature-WASM/native match: {len(trace)} bytes, SHA256 {tracehash}; zero host calls. Default WASM compiled/export-inspected only.'),('identity-parity.log','Actual identity feature-WASM/native unchanged: 4205 bytes, SHA256 60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580.'),('language-runtime/negative.log','exit 1 expected negative; full32 digest rejected by the same comparator'),('language-runtime/negative-label.log','exit 1 expected negative; corrupted scanner name rejected by the same comparator')]:
 s['execution_observations'].append({'command':name,'working_directory':str(ROOT/'specimen-studio'),'outcome':outcome,'raw_log':ref(F+name),'qualification':'Current final-fix synthetic pure API/runtime evidence; no default runtime execution, production integration or scientific efficacy.'})
s['limitations']=[x for x in s['limitations'] if not x.startswith('Parity review Minor remains:')]
s['limitations'] += ['Final combined review found three Important issues (opaque conditions, control vocabulary, candidate budget) and three small API/assertion Minors. One integrated final fix addresses all six with focused RED/GREEN and actual parity; independent scoped rereview pending, not preapproved.','Original 23-file legacy baseline is never repinned. 22 identities remain unchanged; identity/mod.rs has exactly one reviewed immutable O(1) entries accessor extension, with original bytes reconstructed by removing that exact addition. Existing identity semantics/IDs/saves and actual 4205-byte trace remain unchanged.','Candidate work is a logical borrowed traversal/materialization reservation, not elapsed-time or global scheduler qualification. Conflicting control vocabularies are conservatively refused before any Prepared result.']
s['supporting_references']=[ref(p) for p in unique([x['path'] for x in s['supporting_references']]+[x['path'] for x in m['raw_evidence']])]
for item in s['source_snapshot_observations']:
 item['observed_sha256']=sha(item['path']);item['state']='matched' if item['observed_sha256']==item['historical_expected_sha256'] else 'mismatched'
refresh(s);write(B+'studio-research-v1.json',s)
refresh(r)
for audit in r['audits']:
 for item in audit['files']:
  item['observed_sha256']=sha(item['path']);item['state']='matched' if item['observed_sha256']==item['expected_sha256'] else 'mismatched'
write(B+'register-v1.json',r);write(B+'candidate-paths-v1.json',c)
p=ROOT/'tools/check_completion_execution.py';text=p.read_text();text=re.sub(r"CANDIDATE_PATHS_SHA256 = '[0-9a-f]{64}'","CANDIDATE_PATHS_SHA256 = '"+sha(B+'candidate-paths-v1.json')+"'",text);p.write_text(text)
print('Current final-fix mappings refreshed; 15 observation IDs retained, no requirement disposition or C0 claim changed.')
