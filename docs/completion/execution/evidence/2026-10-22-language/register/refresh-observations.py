"""One-shot reviewed observation refresh. Does not execute product or promote C0."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
BASE = 'docs/completion/execution/'
E = BASE + 'evidence/2026-10-22-language/'
OUT = E + 'register/'
K = 'specimen-studio/native/specimen-kernel/'
W = 'specimen-studio/native/specimen-wasm/'

def read(p): return json.loads((ROOT/p).read_text())
def write(p, v): (ROOT/p).write_text(json.dumps(v, indent=2, ensure_ascii=False)+'\n')
def sha(p): return hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
def ref(p): return {'path':p, 'sha256':sha(p)}
def uniq(items): return list(dict.fromkeys(items))
def files(p): return [str(x.relative_to(ROOT)) for x in sorted((ROOT/p).rglob('*.rs'))]
def refs(paths): return [ref(p) for p in uniq(paths)]

language=files(K+'src/language_v3')
text=files(K+'src/text_v3')
scanner=[K+'src/scanner_v3/mod.rs', K+'src/scanner_v3/rational.rs']
routes=files(K+'src/scanner_v3/routes')
lt=[K+'tests/'+x+'.rs' for x in ['language_v3','language_v3_limits','language_v3_numeric','language_v3_scripts']]
tt=[K+'tests/text_v3.rs']; st=[K+'tests/scanner_v3.rs']; rt=[K+'tests/scanner_v3_routes.rs']
parity_source=[W+'Cargo.toml',W+'src/lib.rs',W+'src/language_probe.rs',W+'examples/language_probe.rs']
parity_tests=[W+'tests/language_probe.rs','specimen-studio/tests/language-parity.mjs','specimen-studio/scripts/check-language-parity.sh']
reviews=[E+p for p in ['language/language-task-review.md','language/fix-1/language-fix1-review.md','language/fix-2/language-fix2-review.md','language/numeric-followup/language-numeric-review.md','text-scan/text-scan-review.md','text-scan/fix-1/text-scan-fix1-review.md','parity/language-parity-review.md']]
reports=[E+p for p in ['language/fix-2/task-8-language-report.md','language/numeric-followup/task-8-language-numeric-report.md','text-scan/fix-1/text-scan-report.md','parity/task-8-language-parity-report.md']]
profile=BASE+'language-v3-profile.md'
raw=[E+p for p in ['gates/summary.json','gates/browser.log','gates/lint.log','gates/native-test.log','gates/native-clippy.log','language/fix-2/green-focused.log','language/numeric-followup/green-focused.log','text-scan/fix-1/green-focused.log','parity/runtime-final.log','parity/negative.log','parity/checks.json','parity/native.json','parity/wasm.json','parity/default.wasm','parity/language.wasm']]

# Every new source/test file has explicit applicable requirement/decision routing.
groups=[
 {'name':'language preparation, utilities and bounded scripts','sources':language,'tests':lt,'ids':['P08.02','CH07','CH08','CH21','CH23','S19','S20','S21','S25','S46'],'decisions':['D05','D17'], 'boundary':'Pure immutable preparation and EffectPlans; no durable utility ingestion, authenticated host effects, active engine integration or general language efficacy.'},
 {'name':'class-specific text-v3 transforms','sources':text,'tests':tt,'ids':['P08.01','CH05','CH06','S01','S04'],'decisions':['D01','D02'], 'boundary':'Versioned local structural projection; no legacy ID conversion, saved schema migration, production registration or calibrated retrieval efficacy.'},
 {'name':'full-original checked scanner and exact reducer','sources':scanner,'tests':st,'ids':['P08.03','CH09','CH23','S05','S06'],'decisions':['D06'], 'boundary':'Action-scoped full validated features and checked i128 rational reducer; no whole-chunk scanner, jitter/ATP/voting/global scheduler or research calibration.'},
 {'name':'explicit routes and joined candidate origins','sources':routes,'tests':rt,'ids':['P08.01','P08.02','CH06','CH08','CH23','S05','S20','S21','S46'],'decisions':['D05','D17'], 'boundary':'Population/scope-fenced local candidates and pure Tape adapter; no host authorization, vote/admission, production scheduler or durable registration.'},
 {'name':'actual native/feature-WASM synthetic integration witness','sources':parity_source,'tests':parity_tests,'ids':['P08.10','P08.01','P08.02','P08.03','CH05','CH06','CH07','CH08','CH09','CH21','S01','S04','S05','S06','S19','S20','S21','S46'],'decisions':['D01','D02','D05','D06','D17'], 'boundary':'Executed feature WASM only, default compiled/export-inspected; one arm64 native/Bun runtime, no all-target, model/GPU/OCR, installed desktop or UI theme qualification.'},
]
manifest=[]
for p in uniq([K+'src/lib.rs']+language+text+scanner+routes+lt+tt+st+rt+parity_source+parity_tests):
 gs=[g for g in groups if p in g['sources']+g['tests']]
 manifest.append({**ref(p),'bytes':(ROOT/p).stat().st_size,'mapped_ids':uniq(i for g in gs for i in g['ids']) or ['P08.01','P08.02','P08.03'],'mapped_decisions':uniq(i for g in gs for i in g['decisions']) or ['D01','D05','D06']})
write(OUT+'current-source-manifest.json',{'scope':'Current bytes inspected for bounded partial mappings; historical manifest bytes remain untouched. Presence and byte identity do not establish execution.','files':manifest})
write(OUT+'requirement-mapping.json',{'claim_policy':'C0-only; no promotion','groups':groups,'reports':refs(reports),'independent_reviews':refs(reviews),'profile':ref(profile),'raw_evidence':refs(raw)})

s=read(BASE+'studio-research-v1.json')
r=read(BASE+'register-v1.json')
c=read(BASE+'candidate-paths-v1.json')
byid={x['id']:x for x in r['requirements']}

# Explicit bounded current observations replace stale missing-implementation wording.
obs={
'S01':('Shared low keys retain independently scannable full originals; this supplements the reviewed exact-entry foundation.', ['Production independent qualification/voting, calibrated paraphrase recall and ID/save projection migration remain open.']),
'S04':('Text-v3 emits class-specific transforms and complete 32-byte PatternKeys at 6/7/16/17 lexical-token boundaries.', ['Production key registration, legacy ID conversion, import/save migration and full-case qualification remain open.']),
'S05':('Actual HybridTable retrieval returns both same-low-key originals 2+3 and 8+9; secondary full-feature scans retain different exact operand evidence, independently repeated by native/feature-WASM witness.', ['The action-scoped opt-in scanner is not wired into the active engine; full-chunk scanning, calibrated qualification/voting and efficacy remain open.']),
'S06':('Direct checked i128 rational reducer tests exceed i64 capacity, preserve signed cancellation and exact fractions, subtract before one clamp and explicitly refuse overflow.', ['Production accumulation/orchestration integration, confidence calibration and full-case/efficacy qualification remain open; synthetic large totals are reducer evidence only.']),
'S19':('Paired two plus two/2+2 preserve ordered operands/operators and compute 4; decimal captures retain exact coefficient/scale evidence separately from finite f64 computation.', ['Production bind/action registration, host effect execution, broader intent/language coverage and independent full-case efficacy remain open.']),
'S20':('Quote-aware preparation retains scoped negation, unknown conditions and fan-out guards; guarded script execution returns ActionBlocked and routes retain unexecuted Blocked Tape origins.', ['Real prohibited host effects and authenticated host contacts are not exercised; active engine, growth pairing, exact frozen-case qualification and efficacy remain open.']),
'S21':('Actual bounded Tape execution yields bytes; runaway +[] returns typed StepBudget and query-wide/output/depth errors return no partial result.', ['Production script registration, active engine cancellation/scheduling, actual host-effect bounds and full-case qualification remain open.']),
'S25':('Versioned utility alternatives preserve charge evidence, original/source spans and safety constraints under bounded host draws.', ['Charge preservation is not a tone-tolerance oracle; ATP/tone retry equations, meaning-preserving efficacy and full frozen-case qualification remain open.']),
'S46':('Nested capability intersection denies a globally allowed parent-undeclared LookUp; dynamic/unknown contacts and stale registry content/revision refuse before a complete pure EffectPlan.', ['Internal population/registry capabilities do not authenticate a real host; real contact authorization/effects, durable registration and full-case qualification remain open.']),
}
# Direct test names were read from current assertions, not inferred from labels.
direct={
'S01':(rt[0],'s05_distinct_originals_share_low_key_but_full_scanner_evidence_differs'),
'S04':(tt[0],'exact_ast_token_boundaries_and_full_digest_framing'),
'S05':(rt[0],'s05_distinct_originals_share_low_key_but_full_scanner_evidence_differs'),
'S06':(st[0],'s06_direct_checked_i128_reducer_subtracts_before_single_clamp'),
'S19':(lt[0],'s19_numeric_grammar_preserves_exact_operands_and_source_spans'),
'S20':(lt[3],'execute_for_action_preserves_preparation_guards'),
'S21':(lt[3],'s21_actual_tape_execution_and_atomic_failures'),
'S25':(lt[0],'all_nine_utilities_have_observable_semantics_and_safe_fanout'),
'S46':(lt[3],'s46_nested_contacts_narrow_parent_and_current_registry'),
}
for case in s['cases']:
 id=case['id']
 if id not in obs: continue
 gs=[g for g in groups if id in g['ids']]
 case['source_references']=refs([x['path'] for x in case['source_references']]+[p for g in gs for p in g['sources']])
 case['test_references']=refs([x['path'] for x in case['test_references']]+[p for g in gs for p in g['tests']])
 case['assertion_relation']='partial direct local mechanism; no whole-case qualification'
 case['current_language_observation']=obs[id][0]
 case['blockers']=obs[id][1]+['Whole frozen case remains open/C0/unknown; no promotion.']
 p,name=direct[id]
 s['local_mechanism_observations'].append({'id':'language-'+id,'case':id,'claim_policy':'C0-only; no promotion','assertion':obs[id][0],'direct_test':{**ref(p),'name':name},'source_references':refs([p for g in gs for p in g['sources']]),'test_references':refs([p for g in gs for p in g['tests']]),'report_references':refs(reports),'review_references':refs(reviews),'execution_references':refs(raw),'blockers':case['blockers']})

for d in s['decisions']:
 gs=[g for g in groups if d['id'] in g['decisions']]
 if not gs: continue
 d['source_references']=refs([x['path'] for x in d['source_references']]+[p for g in gs for p in g['sources']])
 d['test_references']=refs([x['path'] for x in d['test_references']]+[p for g in gs for p in g['tests']])
 d['assertion_relation']='partial direct opt-in language/text/scanner mechanisms; local profile only'
 d['current_language_observation']='; '.join(g['name'] for g in gs)+'. See current requirement-mapping/source manifest and scoped independent reviews.'
 d['blockers']=[g['boundary'] for g in gs]+['Decision remains open/C0/unknown; active engine, published/versioned consumer adoption and full scientific qualification remain open.']

# Add minimal source/test routing for existing candidates; only S05/S06 move gaps.
for g in groups:
 for id in g['ids']:
  row=byid[id]
  assert row['reconciliation']['disposition']=='candidate' or id in ['S05','S06'],id
  row['reconciliation']['disposition']='candidate'
  for kind, paths in [('implementation',g['sources']),('tests',g['tests'])]:
   c[id][kind]=uniq(c[id][kind]+paths)
   row[kind]['references']=refs(c[id][kind])
  c[id]['disposition']='candidate'
  c[id]['inspected']=uniq(c[id]['inspected']+g['sources']+g['tests']+[profile,OUT+'requirement-mapping.json',OUT+'current-source-manifest.json'])
  row['reconciliation']['inspected']['references']=refs(c[id]['inspected'])
  row['reconciliation']['reason']='Reviewed opt-in '+g['name']+' supplement existing partial candidates; '+g['boundary']+' Whole requirement remains open/C0/unknown.'
  row['artifacts']['references']=refs([x['path'] for x in row['artifacts']['references']]+[OUT+'requirement-mapping.json',OUT+'current-source-manifest.json',profile]+reports+[E+'gates/summary.json'])
  row['review']['references']=refs([x['path'] for x in row['review']['references']]+reviews)
  if id in obs: row['blockers']=obs[id][1]+['Whole frozen requirement remains open/C0/unknown; no promotion.']
  elif not any('active engine' in b for b in row['blockers']): row['blockers'].append(g['boundary'])
# Stale foundation-only statements are historical, not current missing-source claims.
for id in ['P08.01','P08.02','P08.03','P08.10','CH05','CH06','CH07','CH08','CH09','CH21','CH23']:
 row=byid[id]
 row['blockers']=[b for b in row['blockers'] if not any(term in b for term in ['encoders/scanners remain','text-v3','Current test execution, independent review','direct source/test mapping not independently'])]
 row['blockers']+=['Active engine/commands and ID/save projection migration, durable/versioned utility ingestion, real host contact authorization/effects, independent research efficacy/contamination controls/adjudication remain open.','ATP/jitter/strength/voting, global scheduler/budgets, ABI-owner receipt, OCR/model/GPU/all-target/installed desktop/UI-theme and elapsed-operation qualification remain separate gaps.','Whole frozen requirement remains open/C0/unknown; reviewed mechanism execution is not efficacy or acceptance.']

summary=read(E+'gates/summary.json')
for gate in summary['gates']:
 s['execution_observations'].append({'command':' '.join(gate['command']),'working_directory':gate['cwd'],'outcome':'exit 0; '+({'browser':'221 browser/unit tests and production build','native-test':'221 native passed; one existing ignored test','lint':'lint completed','native-clippy':'native all-target Clippy completed'}[gate['gate']]),'raw_log':ref(E+'gates/'+gate['gate']+'.log'),'qualification':'Retained final combined product gate execution only; source is pinned separately. Not acceptance, research, hardware, owner or installed/UI qualification.'})
s['execution_observations'] += [
 {'command':'bash specimen-studio/scripts/check-language-parity.sh','working_directory':'repository root','outcome':'Actual feature WASM/native trace match: 133421 bytes; SHA256 0874bb27c8a593db459c47f06faa46e62cf2c917f31e7eaa78845a5df7475bae; zero host calls. Default WASM only compiled/export-inspected.','raw_log':ref(E+'parity/runtime-final.log'),'qualification':'Synthetic mechanism witness on one native/Bun runtime; no default runtime execution or full engine/target/efficacy qualification.'},
 {'command':'same semantic comparator --check-json on corrupted full-32-byte digest','working_directory':'repository root','outcome':'exit 1 expected negative; real digest assertion refusal','raw_log':ref(E+'parity/negative.log'),'qualification':'Negative mechanism witness only; no broad adversarial/security qualification.'}]
s['limitations']+=summary['limitations']+[
 'Current profile retains visible missing-component penalties: identical red blue is +20; opaque is -20. These are bounded profile consequences, not language accuracy or drop-in compatibility.',
 'Exact decimal captures use canonical checked i128 coefficient/scale comparisons; script arithmetic computes finite f64 and does not provide exact arithmetic results.',
 'Language fix-2, numeric followup, text/scanner fix-1 and parity task reviews approve their scoped snapshots; reviewer scopes differ and do not equal final whole-tranche approval.',
 'Parity review Minor remains: JavaScript compares component positions/weights/fractions/reasons/spans but does not pin emitted component names; broad final whole-tranche review is pending.',
 'Historical source manifests, RED/GREEN logs and receipt203 bytes are retained. Old gates are not reassigned to changed source; current source manifests document byte identities only.'
]
s['supporting_references']=refs([x['path'] for x in s['supporting_references']]+[OUT+'requirement-mapping.json',OUT+'current-source-manifest.json',profile]+reports+reviews+raw)
# Refresh current reference identities only. Historical raw files are never edited.
def refresh(v):
 if isinstance(v,dict):
  if 'path' in v and 'sha256' in v: v['sha256']=sha(v['path'])
  for x in v.values(): refresh(x)
 elif isinstance(v,list):
  for x in v: refresh(x)
for item in s['source_snapshot_observations']:
 item['observed_sha256']=sha(item['path']); item['state']='matched' if item['observed_sha256']==item['historical_expected_sha256'] else 'mismatched'
refresh(s)
write(BASE+'studio-research-v1.json',s)
refresh(r)
for audit in r['audits']:
 for item in audit['files']:
  item['observed_sha256']=sha(item['path']); item['state']='matched' if item['observed_sha256']==item['expected_sha256'] else 'mismatched'
write(BASE+'register-v1.json',r)
write(BASE+'candidate-paths-v1.json',c)
checker=ROOT/'tools/check_completion_execution.py'
content=checker.read_text()
import re
content=re.sub(r"CANDIDATE_PATHS_SHA256 = '[0-9a-f]{64}'", "CANDIDATE_PATHS_SHA256 = '"+sha(BASE+'candidate-paths-v1.json')+"'", content)
checker.write_text(content)
print('Updated current observations, source/test mappings and exact catalog pin; only S05/S06 changed disposition.')
