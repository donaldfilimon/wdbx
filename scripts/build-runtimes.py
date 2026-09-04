#!/usr/bin/env python3
"""Build the pinned inference executables for the current desktop target."""
import argparse,json,os,platform,shutil,subprocess,tarfile,urllib.request
p=argparse.ArgumentParser();p.add_argument("--only",choices=["llama","diffusion"]);options=p.parse_args()
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
 if options.only and key!=options.only:continue
 src=source(key)
 if key=='diffusion' and not (src/'ggml/CMakeLists.txt').exists():
  ggml=source('ggml');shutil.copytree(ggml,src/'ggml',dirs_exist_ok=True,ignore=shutil.ignore_patterns('.pi'))
 build=work/(key+'-build')
 args=['cmake','-S',str(src),'-B',str(build),'-DCMAKE_BUILD_TYPE=Release','-DBUILD_SHARED_LIBS=OFF','-DGGML_NATIVE=OFF','-DGGML_OPENMP=OFF','-DGGML_METAL=OFF','-DGGML_VULKAN=OFF','-DLLAMA_BUILD_TESTS=OFF','-DLLAMA_BUILD_EXAMPLES=OFF','-DLLAMA_BUILD_SERVER=ON','-DLLAMA_CURL=OFF','-DLLAMA_OPENSSL=OFF','-DSD_BUILD_EXAMPLES=ON']
 if platform.system()=='Windows':
  # The app uses authenticated HTTP on loopback; TLS is owned by model downloads.
  # Static MSVC linkage keeps the sidecars independent of developer-machine DLLs.
  args += ['-UCMAKE_CXX_FLAGS','-DCMAKE_CXX_FLAGS_INIT=/bigobj','-DCMAKE_POLICY_DEFAULT_CMP0091=NEW',
           '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded']
 subprocess.run(args,check=True)
 subprocess.run(['cmake','--build',str(build),'--config','Release','--target',target,'--parallel','2' if platform.system()=='Windows' else '4'],check=True)
 suffix='.exe' if platform.system()=='Windows' else ''
 choices=[build/'bin'/('Release' if platform.system()=='Windows' else '')/(target+suffix),build/'bin'/(target+suffix)]
 executable=next((p for p in choices if p.exists()),None)
 if not executable:raise RuntimeError('Missing runtime executable '+target)
 shutil.copy2(executable,out/(target+suffix))
 for name in ['LICENSE','LICENSE.md','LICENSE.txt']:
  if (src/name).exists():shutil.copy2(src/name,out/(key+'-LICENSE.txt'));break
 if key=='diffusion' and (src/'ggml/LICENSE').exists():shutil.copy2(src/'ggml/LICENSE',out/'ggml-LICENSE.txt')
 print('Built',target,flush=True)
