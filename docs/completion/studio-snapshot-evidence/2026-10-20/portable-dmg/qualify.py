import hashlib, json, pathlib, shutil, stat, subprocess, time

base = pathlib.Path(__file__).parent
source = base.parent/'local-adhoc/WDBX Specimen Studio.app'
workflow = pathlib.Path('/Users/donald/sources/repos/wdbx/specimen-studio/scripts/seal-macos-package.py')
stage = base/'staging'
image = base/'WDBX Specimen Studio_0.2.0_aarch64_local-adhoc.dmg'
receipt = {'profile': 'portable local ad hoc DMG', 'commands': [], 'mounted': False, 'installed': False}
def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda: f.read(1024*1024), b''): h.update(b)
    return h.hexdigest()
def manifest(root):
    return {str(p.relative_to(root)): {'sha256': digest(p), 'size': p.stat().st_size, 'mode': oct(stat.S_IMODE(p.stat().st_mode))} for p in sorted(root.rglob('*')) if p.is_file()}
def run(argv, timeout):
    start = time.monotonic()
    p = subprocess.run([str(a) for a in argv], capture_output=True, text=True, timeout=timeout)
    receipt['commands'].append({'argv': [str(a) for a in argv], 'timeout_seconds': timeout, 'exit': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr, 'elapsed_seconds': time.monotonic()-start})
    (base/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    assert p.returncode == 0, receipt['commands'][-1]
    return p
assert not stage.exists() and not image.exists(), 'Never overwrite earlier profile artifacts'
receipt['workflow_sha256'] = digest(workflow)
receipt['workflow_path'] = str(workflow)
receipt['input_app'] = str(source)
receipt['input_before'] = manifest(source)
stage.mkdir()
app = stage/source.name
run(['/usr/bin/ditto', '--noextattr', '--norsrc', source, app], 60)
assert manifest(app) == receipt['input_before']
run(['/usr/bin/codesign', '--verify', '--deep', '--strict', '--verbose=4', app], 30)
(stage/'Applications').symlink_to('/Applications')
receipt['applications_symlink'] = str((stage/'Applications').readlink())
run(['/usr/bin/hdiutil', 'create', '-volname', 'WDBX Specimen Studio', '-srcfolder', stage, '-format', 'UDZO', image], 240)
run(['/usr/bin/hdiutil', 'verify', image], 120)
run(['/usr/bin/hdiutil', 'imageinfo', image], 30)
receipt['image'] = {'path': str(image), 'sha256': digest(image), 'size': image.stat().st_size}
receipt['input_after'] = manifest(source)
receipt['staging_after'] = manifest(app)
receipt['input_unchanged'] = receipt['input_before'] == receipt['input_after']
receipt['staging_unchanged'] = receipt['input_before'] == receipt['staging_after']
assert receipt['input_unchanged'] and receipt['staging_unchanged']
assert digest(workflow) == receipt['workflow_sha256']
receipt['pass'] = True
(base/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
print(json.dumps({'pass': True, 'image': receipt['image'], 'input_unchanged': True, 'staging_unchanged': True}))
