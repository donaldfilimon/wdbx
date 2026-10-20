import hashlib, json, pathlib, plistlib, shutil, stat, struct, subprocess, tempfile

base = pathlib.Path(__file__).parent
original = base.parents[1] / 'wdbx/specimen-studio/target/release/bundle/macos/WDBX Specimen Studio.app'
copy = base / original.name
commands = []
def run(args):
    p = subprocess.run([str(a) for a in args], text=True, capture_output=True, timeout=30)
    commands.append({'argv': [str(a) for a in args], 'exit': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr})
    return p
def manifest(root):
    return {str(p.relative_to(root)): {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'mode': oct(stat.S_IMODE(p.stat().st_mode)), 'size': p.stat().st_size} for p in sorted(root.rglob('*')) if p.is_file()}
before = manifest(original)
if not copy.exists():
    shutil.copytree(original, copy, symlinks=True)
    assert manifest(copy) == before
run(['/usr/bin/codesign', '--verify', '--deep', '--strict', '--verbose=4', original])
run(['/usr/bin/codesign', '-dvvv', '--entitlements', '-', original])
info = plistlib.loads((copy/'Contents/Info.plist').read_bytes())
machos = []
for p in sorted(copy.rglob('*')):
    if p.is_file():
        result = run(['/usr/bin/file', p])
        if 'Mach-O' in result.stdout:
            machos.append(p)
main = copy/'Contents/MacOS'/info['CFBundleExecutable']
assert main in machos and len(machos) == 3
for p in machos:
    if p != main:
        assert run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none', p]).returncode == 0
assert run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none', copy]).returncode == 0
for p in machos + [copy]:
    assert run(['/usr/bin/codesign', '--verify', '--strict', '--verbose=4', p]).returncode == 0
    run(['/usr/bin/codesign', '-dvvv', '--entitlements', '-', p])
assert run(['/usr/bin/codesign', '--verify', '--deep', '--strict', '--verbose=4', copy]).returncode == 0
pins = json.loads((base.parents[1]/'wdbx/specimen-studio/native/runtime-pins.json').read_text())
for filename, key in [('sd-cli','diffusion'), ('llama-server','llama')]:
    p = copy/'Contents/Resources/binaries'/filename
    result = run([p, '--version'])
    assert result.returncode == 0 and pins[key]['revision'] in result.stdout + result.stderr
    assert run([p, '--help']).returncode == 0
for p in machos:
    run(['/usr/bin/otool', '-L', p])
after = manifest(copy)
changed = [p for p in before if before[p] != after.get(p)]
assert set(changed) == {str(p.relative_to(copy)) for p in machos}
assert manifest(original) == before, 'Original must remain unchanged'
# Compare every original Mach-O payload byte except the exact code-signature
# blob and signing-adjusted __LINKEDIT size / LC_CODE_SIGNATURE size fields.
# No executable section, data section, symbol table or other bytes excluded.
def unsigned_payload(p):
    data = bytearray(p.read_bytes())
    assert struct.unpack_from('<I', data)[0] == 0xfeedfacf
    offset = 32
    signature_offset = None
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19 and data[offset+8:offset+24].rstrip(b'\0') == b'__LINKEDIT':
            data[offset+32:offset+40] = bytes(8)
            data[offset+48:offset+56] = bytes(8)
        if command == 0x1d:
            signature_offset, signature_size = struct.unpack_from('<II', data, offset+8)
            assert signature_offset + signature_size == len(data)
            data[offset+12:offset+16] = bytes(4)
        offset += size
    assert signature_offset is not None
    return hashlib.sha256(data[:signature_offset]).hexdigest()
normalized = {}
for p in machos:
    relative = p.relative_to(copy)
    a, b = unsigned_payload(original/relative), unsigned_payload(p)
    normalized[str(relative)] = {'original_payload_sha256': a, 'adhoc_payload_sha256': b, 'equal': a == b}
assert all(v['equal'] for v in normalized.values()), normalized
receipt = {'original': str(original), 'profile_copy': str(copy), 'original_unchanged': True, 'before': before, 'after': after, 'signature_only_changed_files': changed, 'unsigned_macho_comparison': normalized, 'added': sorted(set(after)-set(before)), 'commands': commands, 'info_plist': info}
(base/'receipt.json').write_text(json.dumps(receipt, indent=2, default=str)+'\n')
print(json.dumps({'verified': True, 'original_unchanged': True, 'changed': changed, 'added': receipt['added'], 'receipt': str(base/'receipt.json')}))
