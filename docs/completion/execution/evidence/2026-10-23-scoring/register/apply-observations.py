"""One-shot reviewed observation application. Does not edit product or frozen C0 inputs."""
from pathlib import Path
from collections import Counter
import json, hashlib, copy, re
ROOT=Path(__file__).resolve().parents[6]
B=ROOT/'docs/completion/execution'; E='docs/completion/execution/evidence/2026-10-23-scoring/'; O=E+'register/'
D=ROOT/'.superpowers/sdd/2026-10-20-wdbx-full-completion/scoring-evidence-draft'
K='specimen-studio/native/specimen-kernel/'; W='specimen-studio/native/specimen-wasm/'
def read(p):return json.loads((ROOT/p).read_text())
def sha(p):return hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
def ref(p):return dict(path=p,sha256=sha(p))
def dump(p,d):(ROOT/p).write_text(json.dumps(d,indent=2)+'\n')
def unique(x):return list(dict.fromkeys(x))
def append(a,paths):
 known={v['path'] for v in a};a.extend(ref(p) for p in unique(paths) if p not in known)
# Draft text is planning input only. New current references are resolved below.
catalog=json.loads((D/'candidate-paths-v1.draft.json').read_text());reg=json.loads((D/'register-v1.draft.json').read_text());studio=json.loads((D/'studio-research-v1.draft.json').read_text());mapping=json.loads((D/'requirement-mapping.draft.json').read_text())
profile='docs/completion/execution/scoring-orchestration-v3-profile.md'
reports=[E+p for p in ['scoring/task-8-scoring-report.md','orchestration/task-8-orchestration-report.md','orchestration/fix1/task-8-orchestration-fix1-report.md','canonical-budget/task-8-canonical-budget-report.md','canonical-budget/fix1/task-8-canonical-budget-fix1-report.md','parity/canonical-final/task-8-scoring-parity-report.md','parity/review-fix1/task-8-scoring-parity-fix1-report.md']]
reviews=[E+p for p in ['scoring/task-8-scoring-review.md','orchestration/task-8-orchestration-review.md','orchestration/fix1/task-8-orchestration-fix1-rereview.md','canonical-budget/task-8-canonical-budget-review.md','canonical-budget/fix1/task-8-canonical-budget-fix1-rereview.md','parity/canonical-final/task-8-scoring-parity-review.md','parity/review-fix1/task-8-scoring-parity-fix1-rereview.md']]
receipts=[E+p for p in ['canonical-budget/fix1/source-freeze.json','canonical-budget/fix1/layout-policy-audit.json','parity/canonical-final/source-freeze.json','parity/canonical-final/verification.json','parity/review-fix1/source-freeze.json','parity/review-fix1/artifact-qualification.json','parity/review-fix1/verification.json','gates/review-fix1/summary.json','final/product-freeze.json']]
history='Historical original orchestration/probe reviews require changes; scoped fix1 reviews address their named findings. Original/intermediate failure logs remain historical; final whole-tranche review is pending.'
common=['Active engine/commands/feedback adoption and ID/save/import projection or migration remain unqualified.','Whole frozen case/requirement remains open/C0/unknown; partial mechanism evidence does not qualify efficacy or acceptance.','Final source-bound local runtime/gate observations are finite synthetic mechanisms, not physical memory/concurrency, real host fencing, scientific efficacy, ABI acceptance, installed/default adoption or all-target qualification. '+history]
for cid,v in mapping['cases'].items():v['blockers']=v['blockers'][:-3]+common
for v in mapping['program_chapter_routes'].values():
 v['bounded_relation_and_blocker']=v['bounded_relation_and_blocker'].replace('PROSPECTIVE ONLY: bind actual final source/report/independent review and four fresh gates before applying current execution claims; current pre-integration comparator exit1 remains historical.','Actual finite feature-WASM/native strict semantic/logical accounting parity with 24 fixed-boundary outcomes and 68 diagnostics; three negative processes and source-bound four Studio gates. Default WASM compile/export inspection only. No all-target, installed, physical-memory/concurrency, scientific or ABI qualification.')
# Add final attachment correction/direct regression and nested probe to exact routes.
attachment_test=K+'tests/scoring_orchestration_attachment_budget_v1.rs'
for cid in ['S12','S13','S14','S15']:
 v=mapping['cases'][cid];v['tests']=unique(v['tests']+[attachment_test]);v['impl']=unique(v['impl']+[K+'src/logical_charge_v1.rs'])
for rid in ['P08.03','P08.09','CH10','CH11','CH20','CH23']:
 v=mapping['program_chapter_routes'][rid];v['tests']=unique(v['tests']+[attachment_test])
v=mapping['program_chapter_routes']['P08.10'];v['implementation']=unique(v['implementation']+[W+'src/scoring_probe/budget_probe/attachment.rs'])
for did in ['D08','D16']:mapping['decisions'][did]['tests']=unique(mapping['decisions'][did]['tests']+[attachment_test])
# Cases keep their frozen/baseline records unchanged; only current bounded observations change.
for row in studio['cases']:
 cid=row['id']
 if cid not in mapping['cases']:continue
 v=mapping['cases'][cid];append(row['source_references'],v['impl']);append(row['test_references'],v['tests']);row['assertion_relation']='partial direct opt-in scoring/orchestration mechanism, with source-bound local executions; no frozen-case qualification';row['blockers']=v['blockers']
for ob in studio['local_mechanism_observations']:
 if not ob['id'].startswith('scoring-'):continue
 v=mapping['cases'][ob['case']];ob['blockers']=v['blockers'];ob['source_references']=[ref(p) for p in v['impl']];ob['test_references']=[ref(p) for p in v['tests']];ob['report_references']=[ref(p) for p in reports];ob['review_references']=[ref(p) for p in reviews];ob['execution_references']=[ref(p) for p in receipts]
for row in studio['decisions']:
 if row['id'] not in mapping['decisions']:continue
 v=mapping['decisions'][row['id']];append(row['source_references'],v['implementation']);append(row['test_references'],v['tests']);row['blockers']=[v['bounded_relation_and_blocker']]+common
allroutes={k:dict(implementation=v['impl'],tests=v['tests'],bounded_relation_and_blocker=v['assertion']) for k,v in mapping['cases'].items()};allroutes.update(mapping['program_chapter_routes'])
for rid,v in allroutes.items():
 c=catalog[rid];r=next(x for x in reg['requirements'] if x['id']==rid)
 c['implementation']=unique(c['implementation']+v['implementation']);c['tests']=unique(c['tests']+v['tests']);c['inspected']=unique(c['inspected']+v['implementation']+v['tests']+[profile]+reports+reviews+receipts)
 for kind in ['implementation','tests']:append(r[kind]['references'],c[kind])
 append(r['reconciliation']['inspected']['references'],c['inspected']);append(r['artifacts']['references'],[profile]+reports+receipts);append(r['review']['references'],reviews)
 r['reconciliation']['reason']='Partial local mechanism candidate only. '+v['bounded_relation_and_blocker']+' All 163 claims stay open/C0/unknown; final whole-tranche review pending.'
 r['blockers']=mapping['cases'][rid]['blockers'] if rid in mapping['cases'] else [v['bounded_relation_and_blocker']]+common
# Current source manifest covers all 40 final tranche sources/registrations/profile/rulings.
groups=[]
for rid,v in allroutes.items():groups.append(dict(ids=[rid],decisions=[d for d,z in mapping['decisions'].items() if set(v['implementation']+v['tests'])&set(z['implementation']+z['tests'])],sources=v['implementation'],tests=v['tests'],boundary=v['bounded_relation_and_blocker']))
for g in groups:
 if g['ids']==['P08.10']:g['decisions']=['D06','D07','D08','D16']
# Entrypoint/private module registration is routed explicitly, not extra behavior.
groups.append(dict(ids=['P08.03','P08.10'],decisions=['D06','D07','D08','D16'],sources=[K+'src/lib.rs',profile,E+'continuation-rulings.md'],tests=[],boundary='Minimal additive registration and current bounded profile/rulings; no engine/default adoption.'))
manifest=[]
for item in read(E+'final/product-freeze.json')['files']:
 p=item['path'];assert sha(p)==item['sha256'],p
 gs=[g for g in groups if p in g['sources']+g['tests']];assert gs,p
 manifest.append(dict(path=p,sha256=sha(p),mapped_ids=unique([r for g in gs for r in g['ids']]),mapped_decisions=unique([d for g in gs for d in g['decisions']])))
mapping.update(kind='Current bounded requirement routing; no claim promotion',hash_policy='Current source refs hash-bound; historical receipts never relabelled as current execution',groups=groups)
dump(E+'evidence/requirement-mapping.json',mapping);dump(O+'requirement-mapping.json',mapping);dump(O+'current-source-manifest.json',dict(scope='Current 40 final-tranche source/profile/ruling identities; finite local observations only, no source authenticity or product acceptance',files=manifest))
# Every old execution observation is historical relative to this new tranche.
for ob in studio['execution_observations']:
 ob['qualification']='Historical prior packet execution only; no validation of changed scoring registrations. '+ob['qualification']
for g in read(E+'gates/review-fix1/summary.json')['gates']:
 studio['execution_observations'].append(dict(command=' '.join(g['command']),working_directory='specimen-studio',outcome='exit 0; 221 browser/unit passes and production build' if g['name']=='browser' else ('exit 0; 311 native passes plus one existing ignored test' if g['name']=='native-test' else 'exit 0'),raw_log=g['log'],source_record=g['record'],qualification='Source-bound local gate; 324 sources unchanged during each gate. Native logs retain objcopy SIGABRT/dyld warning, dependency import/block futurecompat; no installed/UI/research/host/ABI qualification.'))
for command,log,outcome in [('actual native executable + retained feature-WASM with current comparator','03-positive.log','exit 0; 243932 semantic bytes, 24 fixed-boundary outcomes, 68 equal logical diagnostics, host_calls 0; raw wrappers differ by architecture label'),('--self-test','04-self-test.log','exit 0; malformed/incomplete shape refusals'),('--check-json arithmetic corruption','05-negative-arithmetic.log','exit 1 expected; exact effective confidence refusal'),('--check-json quarantine corruption','06-negative-quarantine.log','exit 1 expected; quarantine transition refusal'),('--check-json boundary corruption','07-negative-boundary.log','exit 1 expected; fixed-boundary outcome refusal')]:
 studio['execution_observations'].append(dict(command=command,working_directory='specimen-studio',outcome=outcome,raw_log=ref(E+'parity/review-fix1/'+log),qualification='Finite synthetic feature-WASM runtime; default module compiled/export-inspected only. JS-only rename reused hash-qualified canonical-final native/feature/default artifacts without rebuild. '+history))
# Keep historical limitation texts as historical, and state the current qualification explicitly.
studio['limitations']=studio['limitations'][:-3]
studio['limitations']=[('Historical language-packet limitation: '+x) if ('Pinned rust-objcopy cannot load' in x or 'independent scoped rereview pending' in x) else x for x in studio['limitations']]
studio['limitations']+=['Final canonical fix1 replaces attachment Context/Visit/Arc/capture and policy Span charges. All 14 new production modules are audited free of layout-based policy expressions; logical-charge-v1 remains logical accounting, not physical heap memory or ABI acceptance.','Final finite native/feature-WASM witness: 243932 semantic bytes, SHA256 7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8; 24 fixed-boundary outcomes and 68 matching logical diagnostics. Three normal-comparator corruptions exit 1. Raw architecture-labelled wrappers are unequal.','Broad final native-test/native-clippy logs still contain rust-objcopy SIGABRT/dyld missing-libLLVM diagnostics despite recorded DYLD_LIBRARY_PATH. Possible replay is unadjudicated; isolated loader/build repair does not establish clean broad-gate linking.','Scoring/probe scoped fixes pass their named rereviews; original orchestration and probe CHANGES REQUIRED outcomes remain historical. Probe M1 input-pin hardening is nonblocking final triage; final whole-tranche review is pending.','S14 1501 descriptor fixture uses None completions: model accounting/drain, not 1501 actual scans or physical concurrency. Small actual scanner/broker parity scenarios are separate.']
append(studio['supporting_references'],[profile]+reports+reviews+receipts+[O+'requirement-mapping.json',O+'current-source-manifest.json'])
# Refresh current refs only. Historical expected snapshots, baseline provenance and raw executions remain fixed.
changed=[]
def refresh(o,trail=''):
 if isinstance(o,list):
  for i,v in enumerate(o):refresh(v,trail+'/'+str(i))
 elif isinstance(o,dict):
  if 'path' in o and 'sha256' in o:
   p=o['path'];h=sha(p)
   if o['sha256']!=h:changed.append(dict(location=trail,path=p,prior_sha256=None if o['sha256']=='FINALHASH_PENDING' else o['sha256'],current_sha256=h));o['sha256']=h
  for k,v in o.items():
   if k not in ['baseline_observation','baseline_hash_snapshot','source_snapshot_observations','baseline_audit','audits','execution_observations']:refresh(v,trail+'/'+k)
for record in studio['source_snapshot_observations']:
 record['observed_sha256']=sha(record['path']);record['state']='matched' if record['observed_sha256']==record['historical_expected_sha256'] else 'mismatched'
refresh(studio,'studio');studio['recorded_date']='2026-10-23';dump('docs/completion/execution/studio-research-v1.json',studio)
refresh(reg,'register');reg['recorded_date']='2026-10-23';dump('docs/completion/execution/register-v1.json',reg);dump('docs/completion/execution/candidate-paths-v1.json',catalog)
# Only the candidate catalog pin is changed in the checker.
p=ROOT/'tools/check_completion_execution.py';body=p.read_text();body,n=re.subn(r"CANDIDATE_PATHS_SHA256 = '[a-f0-9]{64}'","CANDIDATE_PATHS_SHA256 = '"+sha('docs/completion/execution/candidate-paths-v1.json')+"'",body);assert n==1;p.write_text(body)
dump(O+'current-reference-refresh.json',dict(scope='Current safe references only; baselines/raw receipts/historical expected hashes retained',changes=changed))
assert Counter(c['disposition'] for c in catalog.values())==dict(candidate=129,local_gap=18,external_gap=16)
assert len(reg['requirements'])==163 and len(studio['cases'])==46 and len(studio['decisions'])==18
assert 'FINALHASH_PENDING' not in (ROOT/'docs/completion/execution/register-v1.json').read_text()+(ROOT/'docs/completion/execution/studio-research-v1.json').read_text()
print('Applied nine partial candidates; 129/18/16, 163 open/C0/unknown, 46 S/18 D, 24 bounded observations; current source manifest40.')
