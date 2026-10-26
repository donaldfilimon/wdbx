"""Hash-only qualification of reusable frozen checks; never evaluates corpus data."""
import hashlib,json,subprocess,ast
from pathlib import Path
R=Path(__file__).resolve().parents[6];E='docs/completion/execution/evidence/2026-10-23-scoring/';O=E+'register/'
def sha(p):return hashlib.sha256((R/p).read_bytes()).hexdigest()
def ref(p):return dict(path=p,sha256=sha(p))
def dump(p,d):(R/p).write_text(json.dumps(d,indent=2)+'\n')
paths=['tools/check_completion.py','tools/tests/test_check_completion.py']
paths += [str(p.relative_to(R)) for p in (R/'docs/completion').rglob('*') if p.is_file() and '__pycache__' not in p.parts and 'execution' not in p.relative_to(R/'docs/completion').parts and p.name not in ['execution-status.json']]
records=[]
for p in sorted(paths):
 c=subprocess.run(['git','show','HEAD:'+p],cwd=R,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 records.append(dict(path=p,sha256=sha(p),git_HEAD_sha256=hashlib.sha256(c.stdout).hexdigest() if c.returncode==0 else None,git_HEAD_equal=c.returncode==0 and hashlib.sha256(c.stdout).hexdigest()==sha(p)))
# AST literals only; this does not execute the checker or inspect corpus contents.
a=ast.parse((R/'tools/check_completion.py').read_text());constants={}
for n in a.body:
 if isinstance(n,ast.Assign) and len(n.targets)==1 and isinstance(n.targets[0],ast.Name) and n.targets[0].id in ['PINS','SOURCE_PATHS','CONTRACT_PINS','CORPUS_PIN','PROTOCOL_PIN']:constants[n.targets[0].id]=ast.literal_eval(n.value)
pins=[]
for name,h in constants['CONTRACT_PINS'].items():p='docs/completion/'+name;pins.append(dict(path=p,sha256=sha(p),expected_sha256=h,matched=sha(p)==h))
for name,h in constants['PINS'].items():
 for p in ['docs/completion/sources/'+name+'.md',constants['SOURCE_PATHS'][name]]:
  if (R/p).is_file():pins.append(dict(path=p,sha256=sha(p),expected_sha256=h,matched=sha(p)==h))
for p,h in [('docs/completion/experiments.json',constants['PROTOCOL_PIN']),('docs/completion/synthetic-corpus-v2.jsonl',constants['CORPUS_PIN'])]:pins.append(dict(path=p,sha256=sha(p),expected_sha256=h,matched=sha(p)==h))
assert all(x['git_HEAD_equal'] for x in records),[x['path'] for x in records if not x['git_HEAD_equal']]
assert all(x['matched'] for x in pins)
receipts=[ref(E+'contracts/'+x+'.json') for x in ['current-checker','current-tests','system-checker','system-tests']]
for x in receipts:
 d=json.loads((R/x['path']).read_text());assert d['exit_code']==0;assert sha(d['log']['path'])==d['log']['sha256']
dump(O+'frozen-check-reuse-qualification.json',dict(scope='Reuse existing frozen checker and 37-test receipts only: all checker/test and frozen contracts equal git HEAD, embedded expected pins match live/snapshot inputs. Hash-only corpus handling; no efficacy evaluation.',limitation='Historical command receipts did not capture a full run-time input manifest. Current HEAD equality and embedded pins corroborate unchanged reviewed inputs; no claim that this is a contemporaneous full input snapshot. No new frozen/product gate execution.',input_files=records,embedded_pin_observations=pins,receipts=receipts))
print('Qualified frozen receipt reuse:',len(records),'HEAD-equal files;',len(pins),'embedded source/contract pins')
