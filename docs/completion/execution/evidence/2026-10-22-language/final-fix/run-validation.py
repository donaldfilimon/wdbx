"""Capture separate inventory/contract checks with their actual interpreter."""
import argparse,datetime,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6]
OUT=Path(__file__).resolve().parent
p=argparse.ArgumentParser();p.add_argument('--label',required=True);a=p.parse_args()
env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1')
commands=[['tools/check_completion_execution.py'],['-m','unittest','discover','-s','tools/tests','-p','test_check_completion_execution.py'],[str(OUT/'check-final-closure.py')],['tools/check_completion.py'],['-m','unittest','discover','-s','tools/tests','-p','test_check_completion.py']]
rows=[]
for name,args in zip(['execution-checker','execution-27-mutations','final-closure','frozen-completion-checker','frozen-completion-tests'],commands):
 argv=[sys.executable]+args
 start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 done=subprocess.run(argv,cwd=ROOT,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 log=a.label+'-'+name+'.log';(OUT/log).write_text(done.stdout)
 rows.append({'command':argv,'cwd':str(ROOT),'environment':{'PYTHONDONTWRITEBYTECODE':'1'},'python_version':sys.version,'started_utc':start,'exit_code':done.returncode,'log':log,'log_sha256':hashlib.sha256((OUT/log).read_bytes()).hexdigest()})
 print(a.label,name,'exit',done.returncode,done.stdout[-180:].strip())
(OUT/(a.label+'-commands.json')).write_text(json.dumps(rows,indent=2)+'\n')
sys.exit(any(row['exit_code'] for row in rows))
