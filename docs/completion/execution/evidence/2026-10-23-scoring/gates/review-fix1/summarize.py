"""Summarize separate fresh gates without promoting claims."""
import hashlib,json
from pathlib import Path
folder=Path(__file__).resolve().parent
root=folder.parents[6]
records=[]
for name in ('browser','lint','native-test','native-clippy'):
 p=folder/(name+'.json'); r=json.loads(p.read_text())
 assert r['exit_code']==0 and r['source_unchanged_during_gate'],name
 log=folder/(name+'.log')
 assert hashlib.sha256(log.read_bytes()).hexdigest()==r['log_sha256']
 for f in r['source_files']:
  assert hashlib.sha256((root/f['path']).read_bytes()).hexdigest()==f['sha256'],f['path']
 records.append({'name':name,'command':r['command'],'exit_code':r['exit_code'],'source_unchanged_during_gate':True,'record':{'path':p.relative_to(root).as_posix(),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()},'log':{'path':log.relative_to(root).as_posix(),'sha256':r['log_sha256']}})
out=folder/'summary.json'
assert not out.exists()
out.write_text(json.dumps({'scope':'Four separate Studio source gates; no installed, efficacy, ABI, UI-theme or platform qualification.','source_files':len(r['source_files']),'gates':records},indent=2)+'\n')
print(json.dumps({'gates':len(records),'source_files':len(r['source_files']),'all_passed':True}))
