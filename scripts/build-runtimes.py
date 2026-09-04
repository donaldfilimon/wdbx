#!/usr/bin/env python3
"""Build the pinned inference executables for the current desktop target."""
import json,os,platform,shutil,subprocess,tarfile,urllib.request
from pathlib import Path
root=Path(__file__).resolve().parents[1]
work=root/'work'/'runtimes';work.mkdir(parents=True,exist_ok=True)
pins=json.loads((root/'native/runtime-pins.json').read_text())
def source(key):
 pin=pins[key];dest=work/(key+'-'+pin['revision'][:12])
 if not (dest/'CMakeLists.txt').exists():
  archive=work/(key+'.tar.gz')
  urllib.request.urlretrieve('https://codeload.github.com/'+pin['repo']+'/tar.gz/'+pin['revision'],archive)
  with tarfile.open(archive) as f:
   top=f.getnames()[0].split('/')[0];f.extractall(work,filter='data')
  (work/top).rename(dest)
 return dest
out=root/'src-tauri/binaries';out.mkdir(exist_ok=True)
for key,target in [('llama','llama-server'),('diffusion','sd-cli')]:
 src=source(key)
 if key=='diffusion' and not (src/'ggml/CMakeLists.txt').exists():
  ggml=source('ggml');shutil.copytree(ggml,src/'ggml',dirs_exist_ok=True)
 build=work/(key+'-build')
 args=['cmake','-S',str(src),'-B',str(build),'-DCMAKE_BUILD_TYPE=Release','-DBUILD_SHARED_LIBS=OFF','-DGGML_NATIVE=OFF','-DGGML_OPENMP=OFF','-DGGML_METAL=OFF','-DGGML_VULKAN=OFF','-DLLAMA_BUILD_TESTS=OFF','-DLLAMA_BUILD_EXAMPLES=OFF','-DLLAMA_BUILD_SERVER=ON','-DLLAMA_CURL=OFF','-DSD_BUILD_EXAMPLES=ON']
 subprocess.run(args,check=True)
 subprocess.run(['cmake','--build',str(build),'--config','Release','--target',target,'--parallel','4'],check=True)
 suffix='.exe' if platform.system()=='Windows' else ''
 choices=[build/'bin'/('Release' if platform.system()=='Windows' else '')/(target+suffix),build/'bin'/(target+suffix)]
 executable=next((p for p in choices if p.exists()),None)
 if not executable:raise RuntimeError('Missing runtime executable '+target)
 shutil.copy2(executable,out/(target+suffix))
 print('Built',target,flush=True)
