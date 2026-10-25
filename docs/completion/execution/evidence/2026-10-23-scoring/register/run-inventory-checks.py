"""Run only mutable inventory validation; preserve raw outputs and explicit statuses."""
from pathlib import Path
import json,hashlib,subprocess,os,sys,datetime,shutil
R=Path(__file__).resolve().parents[6];O=Path(__file__).resolve().parent
commands=[('checker',['tools/check_completion_execution.py']),('tests',['-m','unittest','discover','-s','tools/tests','-p','test_check_completion_execution.py']),('closure',[str((O/'check-scoring-closure.py').relative_to(R))])]
env=os.environ.copy();env['PYTHONDONTWRITEBYTECODE']='1'
results=[]
for flavor,python in [('current',shutil.which('python3')),('system','/usr/bin/python3')]:
 for name,args in commands:
  tag=flavor+'-'+name;argv=[python]+args;started=datetime.datetime.now(datetime.timezone.utc).isoformat()
  p=subprocess.run(argv,cwd=R,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=O/(tag+'.log');log.write_bytes(p.stdout)
  record=dict(command=argv,cwd=str(R),environment={'PYTHONDONTWRITEBYTECODE':'1'},started_utc=started,ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),exit_code=p.returncode,log=dict(path=str(log.relative_to(R)),sha256=hashlib.sha256(p.stdout).hexdigest()),scope='Mutable inventory/checker/observation closure only; no product or frozen-check rerun.')
  (O/(tag+'.json')).write_text(json.dumps(record,indent=2)+'\n');results.append(record);print(tag,p.returncode,flush=True)
  if p.returncode:print(p.stdout.decode(),flush=True)
(O/'validation-summary.json').write_text(json.dumps({'checks':results,'all_pass':all(x['exit_code']==0 for x in results)},indent=2)+'\n')
if any(x['exit_code'] for x in results):sys.exit(1)
