from pathlib import Path
import datetime, hashlib, json, os, shutil, subprocess
root=Path('/Users/donald/sources/repos/wdbx');studio=root/'specimen-studio'
ev=root/'docs/completion/execution/evidence/2026-10-23-scoring/parity/review-fix1'
prior=root/'docs/completion/execution/evidence/2026-10-23-scoring/parity/canonical-final'
old=json.loads((prior/'verification.json').read_text());art=Path(old['retained_artifacts'])
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
expected={}
for line in (prior/'09-artifacts.sha256').read_text().splitlines():
    digest,path=line.split('  ',1);expected[Path(path)]=digest
qualified=[]
for name,p in [('native-binary',art/'target/debug/examples/scoring_probe'),('native-json',art/'native.json'),('feature-wasm',art/'scoring.wasm'),('default-wasm',art/'default.wasm')]:
    actual=sha(p);assert actual==expected[p],p;qualified.append({'kind':name,'path':str(p),'sha256':actual,'qualified_against':'canonical-final/09-artifacts.sha256'})
source=json.loads((prior/'source-freeze.json').read_text());unchanged=[]
for row in source['files']:
    if row['path']=='specimen-studio/tests/scoring-parity.mjs':continue
    actual=sha(root/row['path']);assert actual==row['sha256'],row['path'];unchanged.append({**row,'unchanged':True})
kernel=root/'docs/completion/execution/evidence/2026-10-23-scoring/canonical-budget/fix1/source-freeze.json'
for path,want in json.loads(kernel.read_text()).items():assert sha(root/path)==want,path
(ev/'artifact-qualification.json').write_text(json.dumps({'retained_artifacts':str(art),'qualified':qualified,'other_probe_files_unchanged':unchanged,'reviewed_kernel_files_unchanged':25},indent=2)+'\n')
env=os.environ.copy();env.update({'PATH':'/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin','RUSTUP_TOOLCHAIN':'1.99.1','CARGO_NET_OFFLINE':'true','DYLD_LIBRARY_PATH':'/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib'})
relevant={k:env[k] for k in ['PATH','RUSTUP_TOOLCHAIN','CARGO_NET_OFFLINE','DYLD_LIBRARY_PATH']}
(ev/'environment.json').write_text(json.dumps(relevant,indent=2)+'\n')
tools={name:{'path':shutil.which(name,path=env['PATH']),'sha256':sha(shutil.which(name,path=env['PATH']))} for name in ['bun','python3']}
tools['oxlint']={'path':str(studio/'node_modules/.bin/oxlint'),'sha256':sha(studio/'node_modules/.bin/oxlint')}
(ev/'tools.json').write_text(json.dumps(tools,indent=2)+'\n')
commands=[]
def run(name,argv,stdout=None,expected_exit=0,contains=None):
    record={'argv':argv,'cwd':str(studio),'env':relevant,'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat()}
    if stdout:
        with (ev/stdout).open('wb') as out,(ev/(name+'.log')).open('wb') as err:r=subprocess.run(argv,cwd=studio,env=env,stdout=out,stderr=err)
    else:
        with (ev/(name+'.log')).open('wb') as out:r=subprocess.run(argv,cwd=studio,env=env,stdout=out,stderr=subprocess.STDOUT)
    record.update({'exit':r.returncode,'log':name+'.log','stdout':stdout});commands.append(record)
    (ev/(name+'.command.json')).write_text(json.dumps(record,indent=2)+'\n')
    assert r.returncode==expected_exit,(name,r.returncode)
    if contains:assert contains in (ev/(name+'.log')).read_text(),name
run('01-targeted-lint',['bun','run','lint','tests/scoring-parity.mjs'])
run('02-native-reuse',[str(art/'target/debug/examples/scoring_probe')],stdout='native.json')
assert sha(ev/'native.json')==expected[art/'native.json']
run('03-positive',['bun','tests/scoring-parity.mjs',str(ev/'native.json'),str(art/'scoring.wasm'),str(art/'default.wasm'),str(ev/'wasm.json'),str(ev/'corrupt-arithmetic.json'),str(ev/'corrupt-quarantine.json'),str(ev/'corrupt-boundary.json')])
run('04-self-test',['bun','tests/scoring-parity.mjs','--self-test',str(ev/'native.json')])
for i,(kind,marker) in enumerate([('arithmetic','exact confidence/gate effective'),('quarantine','broker independent transition cancel/quarantined'),('boundary','fixed boundary actual refusal/outcome')],5):
    run(f'{i:02}-negative-{kind}',['bun','tests/scoring-parity.mjs','--check-json',str(ev/f'corrupt-{kind}.json')],expected_exit=1,contains=marker)
summary=json.loads(next(l for l in (ev/'03-positive.log').read_text().splitlines() if l.startswith('{"result":"PASS"')))
assert summary==old['summary'],(summary,old['summary'])
js=root/'specimen-studio/tests/scoring-parity.mjs'
ver={'scope':'I1 binding-only lint fix; reused qualified native/WASM binaries without rebuild; original broad gates remain root-owned','commands':commands,'summary':summary,'semantic_unchanged_from_canonical_final':True,'js_sha256':sha(js),'negative_exit_statuses':{'arithmetic':1,'quarantine':1,'boundary':1},'rust_rebuilds':0}
(ev/'verification.json').write_text(json.dumps(ver,indent=2)+'\n')
print(json.dumps({'targeted_lint_exit':0,'positive_exit':0,'summary':summary,'js_sha256':sha(js),'negative_exits':[1,1,1]},indent=2))
