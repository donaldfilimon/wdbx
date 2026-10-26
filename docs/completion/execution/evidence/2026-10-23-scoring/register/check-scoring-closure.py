"""Current scoring observation closure. Inventory/byte identities only, no promotion."""
from pathlib import Path, PurePosixPath
from collections import Counter
import hashlib,json,re
R=Path(__file__).resolve().parents[6]; E='docs/completion/execution/evidence/2026-10-23-scoring/'; O=E+'register/'; B='docs/completion/execution/'
def safe(p):
 assert isinstance(p,str) and p and '\\' not in p
 assert not PurePosixPath(p).is_absolute() and all(s not in ('','.','..') for s in p.split('/')),p
 c=R
 for part in PurePosixPath(p).parts:c=c/part;assert not c.is_symlink(),p
 assert c.is_file() and c.resolve().is_relative_to(R.resolve()),p
 return c
def sha(p):return hashlib.sha256(safe(p).read_bytes()).hexdigest()
def read(p):return json.loads(safe(p).read_text())
def checkrefs(value):
 if isinstance(value,dict):
  if 'path' in value and 'sha256' in value:assert sha(value['path'])==value['sha256'],value['path']
  for child in value.values():checkrefs(child)
 elif isinstance(value,list):
  for child in value:checkrefs(child)
reg=read(B+'register-v1.json');cat=read(B+'candidate-paths-v1.json');studio=read(B+'studio-research-v1.json');frozen=read('docs/completion/studio-acceptance-v1.json')
before=read(O+'before/register-v1.json');priorstudio=read(O+'before/studio-research-v1.json');priorcat=read(O+'before/candidate-paths-v1.json')
assert reg['version']==1 and reg['claim_policy']=='C0-only; no promotion'
assert len(reg['requirements'])==163 and len(set(r['id'] for r in reg['requirements']))==163
assert Counter(r['reconciliation']['disposition'] for r in reg['requirements'])==dict(candidate=129,local_gap=18,external_gap=16)
assert set(cat)=={r['id'] for r in reg['requirements']}
changed=set()
for row,old in zip(reg['requirements'],before['requirements']):
 assert [row[k] for k in ['id','owner','task','plan','status','evidence_level','test_result']]==[old[k] for k in ['id','owner','task','plan','status','evidence_level','test_result']]
 assert (row['status'],row['evidence_level'],row['test_result'])==('open','C0','unknown')
 if row['reconciliation']['disposition']!=old['reconciliation']['disposition']:
  assert old['reconciliation']['disposition']=='local_gap' and row['reconciliation']['disposition']=='candidate';changed.add(row['id'])
 for kind in ['implementation','tests']:
  assert set(priorcat[row['id']][kind])<=set(cat[row['id']][kind]),row['id']
assert changed=={'S%02d'%i for i in range(7,16)},changed
for section,prefix,count in [('cases','S',46),('decisions','D',18)]:
 assert [x['id'] for x in studio[section]]==[prefix+str(i).zfill(2) for i in range(1,count+1)]
 assert [x['frozen_row'] for x in studio[section]]==[x['row'] for x in frozen[section]]
 for row,old in zip(studio[section],priorstudio[section]):
  assert (row['status'],row['evidence_level'],row['test_result'])==('open','C0','unknown') and row['claim_policy']=='C0-only; no promotion'
  assert row['baseline_observation']==old['baseline_observation'];assert row['blockers']
assert studio['baseline_audit']==priorstudio['baseline_audit'] and studio['baseline_hash_snapshot']==priorstudio['baseline_hash_snapshot']
priorids=[x['id'] for x in priorstudio['local_mechanism_observations']];nowids=[x['id'] for x in studio['local_mechanism_observations']]
assert nowids[:len(priorids)]==priorids and len(nowids)==len(set(nowids))==24
assert nowids[len(priorids):]==['scoring-S%02d'%i for i in range(7,16)]
for observation in studio['local_mechanism_observations']:
 t=observation['direct_test'];assert re.search(r'fn\s+'+re.escape(t['name'])+r'\s*\(',safe(t['path']).read_text()),t
for s in studio['source_snapshot_observations']:
 assert sha(s['path'])==s['observed_sha256'];assert s['state']==('matched' if s['observed_sha256']==s['historical_expected_sha256'] else 'mismatched')
checkrefs(studio);checkrefs(reg)
for inv,count in [(E+'preservation-baseline.json',80),(E+'frozen-contract-baseline.json',3)]:
 records=read(inv)['files'];assert len(records)==count
 for p in records:assert sha(p['path'])==p['sha256'],p['path']
for p in read(O+'historical-evidence-baseline.json')['files']:assert sha(p['path'])==p['sha256'],p['path']
manifest=read(O+'current-source-manifest.json')['files'];assert len(manifest)==40 and len({p['path'] for p in manifest})==40
assert {p['path']:p['sha256'] for p in manifest}=={p['path']:p['sha256'] for p in read(E+'final/product-freeze.json')['files']}
mapdata=read(O+'requirement-mapping.json');assert mapdata==read(E+'evidence/requirement-mapping.json')
assert {p for g in mapdata['groups'] for p in g['sources']+g['tests']}=={p['path'] for p in manifest}
known={r['id'] for r in reg['requirements']};decs={d['id'] for d in studio['decisions']}
for p in manifest:
 assert sha(p['path'])==p['sha256'];assert p['mapped_ids'] and len(p['mapped_ids'])==len(set(p['mapped_ids'])) and set(p['mapped_ids'])<=known
 assert p['mapped_decisions'] and len(p['mapped_decisions'])==len(set(p['mapped_decisions'])) and set(p['mapped_decisions'])<=decs
for g in mapdata['groups']:
 assert g['ids'] and len(g['ids'])==len(set(g['ids'])) and set(g['ids'])<=known
 assert g['boundary'] and len(g['decisions'])==len(set(g['decisions'])) and set(g['decisions'])<=decs
# Exact catalog pin-only checker delta; no validation policy relaxation.
old=safe(O+'before/check_completion_execution.py').read_text();new=safe('tools/check_completion_execution.py').read_text()
pattern=r"CANDIDATE_PATHS_SHA256 = '[a-f0-9]{64}'"
assert re.sub(pattern,'CANDIDATE_PATHS_SHA256 = PIN',old)==re.sub(pattern,'CANDIDATE_PATHS_SHA256 = PIN',new)
assert "CANDIDATE_PATHS_SHA256 = '"+sha(B+'candidate-paths-v1.json')+"'" in new
# Supplied gate receipts remain source bound; no gate execution here.
gates=read(E+'gates/review-fix1/summary.json');assert gates['source_files']==324 and len(gates['gates'])==4
for gate in gates['gates']:
 assert gate['exit_code']==0 and gate['source_unchanged_during_gate'];checkrefs(gate)
 record=read(gate['record']['path']);assert len(record['source_files'])==324 and record['exit_code']==0 and record['source_unchanged_during_gate']
 for p in record['source_files']:assert sha(p['path'])==p['sha256'],p['path']
 assert sha(gate['log']['path'])==record['log_sha256']
# Actual final parity outputs are finite recorded observations, not all-target qualification.
P=E+'parity/review-fix1/';verify=read(P+'verification.json');summary=verify['summary']
assert summary['result']=='PASS' and summary['fixed_boundary_rows']==24 and summary['matching_logical_diagnostics']==68 and summary['host_calls']==0
assert verify['negative_exit_statuses']==dict(arithmetic=1,quarantine=1,boundary=1)
assert [c['exit'] for c in verify['commands']]==[0,0,0,0,1,1,1]
native=read(P+'native.json');wasm=read(P+'wasm.json');assert native['semantic']==wasm['semantic']
assert len(native['semantic']['logical_budget']['rows'])==24
assert len(read(E+'canonical-budget/fix1/layout-policy-audit.json')['files'])==14 and read(E+'canonical-budget/fix1/layout-policy-audit.json')['matches']==[]
assert native['diagnostics']['target_arch']!=wasm['diagnostics']['target_arch']
assert native['diagnostics']['observations']==wasm['diagnostics']['observations'] and len(native['diagnostics']['observations'])==68
h='7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8'
for n in ['native','wasm']:
 p=P+n+'.json.semantic.json';assert len(safe(p).read_bytes())==243932 and sha(p)==h
 assert sha(P+n+'.json')==summary[n+'_sha256']
assert sha('specimen-studio/tests/scoring-parity.mjs')==verify['js_sha256']
# Historical frozen-check reuse is qualified, not a new frozen gate result.
q=read(O+'frozen-check-reuse-qualification.json');assert len(q['input_files'])==74
for x in q['input_files']:assert sha(x['path'])==x['sha256']==x['git_HEAD_sha256'] and x['git_HEAD_equal']
for x in q['embedded_pin_observations']:
 assert hashlib.sha256((R/x['path']).read_bytes()).hexdigest()==x['sha256']==x['expected_sha256'] and x['matched']
checkrefs(q['receipts'])
for p in [B+'candidate-paths-v1.json',B+'register-v1.json',B+'studio-research-v1.json',O+'requirement-mapping.json']:
 assert 'FINALHASH_PENDING' not in safe(p).read_text(),p
print('PASS: 163 open/C0/unknown; only S07-S15 become partial candidates; 129/18/16 dispositions.')
print('PASS: 46 S and 18 D ordered frozen rows/baselines, previous 15 observation IDs plus 9 bounded additions.')
print('PASS: safe current references, exact catalog-pin-only checker change, all40 final-source mappings; 80+3 preservation identities and historical evidence unchanged.')
print('PASS: supplied 4 gates/source324 continuity, actual243932 semantic bytes/24fixed-boundary outcomes/68logical diagnostics and 3 negative processes; frozen-check reuse inputs qualified.')
print('Inventory closure only; final combined independent review pending. No acceptance, efficacy, host, hardware or installed qualification.')
