import datetime, hashlib, json, os, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6]
OUT=Path(__file__).resolve().parent
ENV=dict(os.environ,PATH='/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin',RUSTUP_TOOLCHAIN='1.99.1',CARGO_NET_OFFLINE='true')
commands=[
 ('focused-green',['cargo','test','--locked','-p','specimen-kernel','--test','language_final_fix','--test','routes_final_fix']),
 ('kernel-feature-final',['cargo','test','--locked','-p','specimen-kernel','-p','specimen-wasm','--features','language-probe,identity-probe']),
 ('clippy',['cargo','clippy','--locked','-p','specimen-kernel','-p','specimen-wasm','--all-targets','--features','language-probe,identity-probe','--','-D','warnings']),
 ('fmt',['cargo','fmt','--check','-p','specimen-kernel','-p','specimen-wasm']),
 ('language-parity',['bash','scripts/check-language-parity.sh']),
 ('identity-parity',['bash','scripts/check-identity-parity.sh']),
]
records=[]
for name,argv in commands:
 print('RUN',name,flush=True)
 start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (OUT/(name+'.log')).open('wb') as log:
  result=subprocess.run(argv,cwd=ROOT/'specimen-studio',env=ENV,stdout=log,stderr=subprocess.STDOUT)
 records.append(dict(name=name,argv=argv,cwd=str(ROOT/'specimen-studio'),started=start,exit_code=result.returncode,environment={k:ENV[k] for k in ['PATH','RUSTUP_TOOLCHAIN','CARGO_NET_OFFLINE']},log=name+'.log',sha256=hashlib.sha256((OUT/(name+'.log')).read_bytes()).hexdigest()))
 (OUT/'phase-a-commands.json').write_text(json.dumps(records,indent=2)+'\n')
 print('EXIT',name,result.returncode,flush=True)
 if result.returncode: raise SystemExit(result.returncode)
