"""Replace a build-host-linked helper while preserving the qualified application."""
import hashlib,json,os,shutil,subprocess,tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
runner=os.environ['WDBX_RUNNER']
expected={'macos-14':'37cdc057f2e341e8a598ffdd7edc0edc569c50b25553d6d5ba6dc7dd7bec2c7b','macos-15-intel':'2d11e9ab876220d948228cc2b6aa4b94567c8ddf47bf7f5cbb1fb83894f31942'}[runner]
download=root/'work/qualified-application'
subprocess.run(['gh','run','download','33878229721','-n','wdbx-studio-'+runner,'-D',str(download)],check=True)
image=next(download.rglob('*.dmg'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
if sha(image)!=expected:raise RuntimeError('Qualified installer digest differs')
app=root/'target/release/bundle/macos/WDBX Specimen Studio.app';app.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wdbx-qualified-mount-') as temporary:
 subprocess.run(['hdiutil','attach',str(image),'-readonly','-nobrowse','-mountpoint',temporary],check=True)
 try:
  source=Path(temporary)/app.name;binary=source/'Contents/MacOS/wdbx-studio-desktop';original=sha(binary)
  subprocess.run(['ditto','--noextattr','--norsrc',str(source),str(app)],check=True)
 finally:subprocess.run(['hdiutil','detach',temporary],check=True)
if sha(app/'Contents/MacOS/wdbx-studio-desktop')!=original:raise RuntimeError('Qualified application changed')
shutil.copy2(root/'src-tauri/binaries/llama-server',app/'Contents/Resources/binaries/llama-server')
evidence={'applicationQualificationRun':33878229721,'applicationSha256BeforeSealing':original,'applicationUnchangedBeforeSealing':True,'runtimeChecks':[]}
for name in ['llama-server','sd-cli']:
 path=app/'Contents/Resources/binaries'/name
 lines=subprocess.check_output(['otool','-L',str(path)],text=True).splitlines()[1:]
 deps=[line.strip().split(' (')[0] for line in lines]
 if any(not dep.startswith(('/usr/lib/','/System/Library/')) for dep in deps):raise RuntimeError(f'Non-system helper dependency: {deps}')
 result=subprocess.run([str(path),'--help'],capture_output=True,text=True,env={**os.environ,'PATH':'/usr/bin:/bin'},timeout=60)
 if result.returncode:raise RuntimeError(f'{name} failed isolated startup: {result.returncode}')
 evidence['runtimeChecks'].append({'name':name,'sha256':sha(path),'dependencies':deps,'isolatedHelpExitCode':result.returncode})
(root/'work/macos-package-verification.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(json.dumps(evidence,indent=2))
