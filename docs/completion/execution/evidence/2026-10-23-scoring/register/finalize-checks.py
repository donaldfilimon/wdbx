"""Final observation-only closure and whitespace receipts; no product checks."""
from pathlib import Path
import json,hashlib,subprocess,os,datetime,shutil,difflib
R=Path(__file__).resolve().parents[6];O=Path(__file__).resolve().parent
files=['docs/completion/execution/'+n for n in ['register-v1.json','candidate-paths-v1.json','studio-research-v1.json','studio-research.md','README.md']]+['tools/check_completion_execution.py']
def sha(p):return hashlib.sha256((R/p).read_bytes()).hexdigest()
def dump(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
patch=[]
for p in files:
 name=Path(p).name;old=(O/'before'/name).read_text().splitlines(True);new=(R/p).read_text().splitlines(True)
 patch.extend(difflib.unified_diff(old,new,fromfile='before/'+p,tofile='current/'+p))
(O/'owned-inventory.diff').write_text(''.join(patch))
records=[];env=os.environ.copy();env['PYTHONDONTWRITEBYTECODE']='1'
for flavor,python in [('current',shutil.which('python3')),('system','/usr/bin/python3')]:
 argv=[python,str((O/'check-scoring-closure.py').relative_to(R))];start=datetime.datetime.now(datetime.timezone.utc).isoformat();p=subprocess.run(argv,cwd=R,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=O/(flavor+'-closure-final.log');log.write_bytes(p.stdout)
 rec=dict(command=argv,cwd=str(R),environment={'PYTHONDONTWRITEBYTECODE':'1'},started_utc=start,ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),exit_code=p.returncode,closure_sha256=sha(str((O/'check-scoring-closure.py').relative_to(R))),log=dict(path=str(log.relative_to(R)),sha256=hashlib.sha256(p.stdout).hexdigest()))
 dump(O/(flavor+'-closure-final.json'),rec);records.append(rec);print(flavor+' final closure',p.returncode);assert p.returncode==0,p.stdout.decode()
p=subprocess.run(['git','diff','--check'],cwd=R,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(O/'git-diff-check.log').write_bytes(p.stdout)
dump(O/'git-diff-check.json',dict(command=['git','diff','--check'],cwd=str(R),exit_code=p.returncode,log=dict(path=str((O/'git-diff-check.log').relative_to(R)),sha256=hashlib.sha256(p.stdout).hexdigest()),qualification='Tracked diff only; current execution files are untracked, so separate explicit owned whitespace check follows.'));assert p.returncode==0
paths=[R/p for p in files]+[p for p in O.rglob('*') if p.is_file() and 'before' not in p.relative_to(O).parts and p.suffix in ['.md','.py','.json']]+[R/'docs/completion/execution/evidence/2026-10-23-scoring/evidence/requirement-mapping.json']
errors=[]
for p in paths:
 for i,line in enumerate(p.read_text().splitlines(),1):
  if line.rstrip()!=line:errors.append(str(p.relative_to(R))+':'+str(i)+': trailing whitespace')
assert not errors,errors
dump(O/'owned-whitespace-check.json',dict(scope='Explicit text whitespace inspection of current six inventory/checker files and new owned packet text, including untracked files',files=len(paths),errors=errors,exit_code=0))
dump(O/'final-validation-summary.json',dict(original_six_checks='validation-summary.json',final_closures=records,git_diff_exit=p.returncode if hasattr(p,'returncode') else 0,owned_whitespace_exit=0,product_or_frozen_checks_rerun=False))
print('Tracked diff check and explicit owned whitespace check pass')
