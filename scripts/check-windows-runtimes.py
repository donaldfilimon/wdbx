import struct,sys,json
from pathlib import Path

def inspect(path):
 b=path.read_bytes();u16=lambda at:struct.unpack_from('<H',b,at)[0];u32=lambda at:struct.unpack_from('<I',b,at)[0]
 assert b[:2]==b'MZ';pe=u32(0x3c);assert b[pe:pe+4]==b'PE\0\0'
 machine=u16(pe+4);sections=u16(pe+6);opt=pe+24;opt_size=u16(pe+20);magic=u16(opt)
 directory=opt+(112 if magic==0x20b else 96);import_rva=u32(directory+8)
 table=opt+opt_size
 def offset(rva):
  for i in range(sections):
   s=table+i*40;vs,va,raw,ptr=struct.unpack_from('<IIII',b,s+8)
   if va<=rva<va+max(vs,raw):return ptr+rva-va
  raise ValueError('RVA outside sections')
 imports=[]
 if import_rva:
  at=offset(import_rva)
  while any(b[at:at+20]):
   name=offset(u32(at+12));end=b.index(0,name);imports.append(b[name:end].decode());at+=20
 return {'file':str(path),'architecture':{0x8664:'x64',0x14c:'x86',0xaa64:'arm64'}.get(machine,hex(machine)),'imports':imports}
if __name__ == '__main__':
 import os,subprocess
 root=Path(__file__).resolve().parents[1]
 results=[]
 for name in ['llama-server.exe','sd-cli.exe']:
  path=root/'src-tauri/binaries'/name
  item=inspect(path)
  forbidden=[dll for dll in item['imports'] if dll.lower().startswith(('msvcp','vcruntime','libssl','libcrypto'))]
  if forbidden:raise RuntimeError(f'{name} requires unbundled libraries: {forbidden}')
  if item['architecture']!='x64':raise RuntimeError('Unexpected Windows runtime architecture')
  env=os.environ.copy();env['PATH']=str(Path(os.environ['SystemRoot'])/'System32')
  process=subprocess.run([str(path),'--help'],env=env,cwd=path.parent,capture_output=True,text=True,timeout=60)
  if process.returncode != 0:raise RuntimeError(f'{name} failed isolated startup: {process.returncode}: {process.stderr[-500:]}')
  item['isolatedHelpExitCode']=process.returncode;results.append(item)
 dest=root/'work/windows-runtime-verification.json';dest.parent.mkdir(exist_ok=True)
 dest.write_text(json.dumps(results,indent=2)+'\n');print(dest.read_text())
