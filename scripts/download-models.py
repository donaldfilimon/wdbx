#!/usr/bin/env python3
"""Download and verify the curated models into a test or app model directory."""
import argparse,hashlib,json,urllib.request
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('--ids',nargs='*');a=p.parse_args();a.directory.mkdir(parents=True,exist_ok=True)
models=json.loads((Path(__file__).resolve().parents[1]/'native/model-catalog.json').read_text())
for m in models:
 if a.ids and m['id'] not in a.ids:continue
 target=a.directory/m['file']
 if target.exists() and target.stat().st_size==m['size']:
  with target.open('rb') as existing: verified=hashlib.file_digest(existing,'sha256').hexdigest()==m['sha256']
  if verified:print('Verified existing',m['id'],flush=True);continue
 temp=target.with_suffix(target.suffix+'.partial');h=hashlib.sha256();total=0
 print('Downloading',m['id'],m['size'],flush=True)
 with urllib.request.urlopen(m['url'],timeout=120) as source,temp.open('wb') as dest:
  while chunk:=source.read(1024*1024):
   dest.write(chunk);h.update(chunk);total+=len(chunk)
   if total%(256*1024*1024)==0:print(m['id'],total,'/',m['size'],flush=True)
 if total!=m['size'] or h.hexdigest()!=m['sha256']:raise RuntimeError('Model verification failed: '+m['id'])
 temp.replace(target);print('Verified',m['id'],flush=True)
