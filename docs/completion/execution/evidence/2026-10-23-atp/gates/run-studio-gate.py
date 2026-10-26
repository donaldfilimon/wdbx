"""Record one separate Studio gate; no claim promotion or environment fallback."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

COMMANDS = {
    'browser': ['bun', 'run', 'check'],
    'lint': ['bun', 'run', 'lint'],
    'native-test': ['bun', 'run', 'test:native'],
    'native-clippy': ['bun', 'run', 'check:native'],
}
name = sys.argv[1]
command = COMMANDS[name]
folder = Path(__file__).resolve().parent
root = folder.parents[5]
assert (root / 'specimen-studio' / 'Cargo.toml').is_file()
env = dict(os.environ)
env['PATH'] = '/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin'
env['RUSTUP_TOOLCHAIN'] = '1.99.1'
env['CARGO_NET_OFFLINE'] = 'true'
env['DYLD_LIBRARY_PATH'] = '/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib'
def source_state():
    paths = subprocess.check_output([
        'git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard', '--',
        'specimen-studio/native/specimen-kernel',
        'specimen-studio/native/specimen-wasm',
        'specimen-studio/native/specimen-core',
        'specimen-studio/src-tauri', 'specimen-studio/app',
        'specimen-studio/lib', 'specimen-studio/tests',
        'specimen-studio/scripts', 'specimen-studio/Cargo.toml',
        'specimen-studio/Cargo.lock', 'specimen-studio/package.json',
        'specimen-studio/bun.lock',
    ], cwd=root).decode().split('\0')
    return [{'path': p, 'sha256': hashlib.sha256((root / p).read_bytes()).hexdigest()}
            for p in sorted(set(paths)) if p and (root / p).is_file()]

source_before = source_state()
record = {
    'command': command,
    'cwd': str(root / 'specimen-studio'),
    'environment': {k: env[k] for k in ('PATH', 'RUSTUP_TOOLCHAIN', 'CARGO_NET_OFFLINE', 'DYLD_LIBRARY_PATH')},
    'source_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
    'scope': 'Current uncommitted source; gate execution only, not installed/UI/research/owner qualification',
    'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
}
log = folder / (name + '.log')
if log.exists() or (folder / (name + '.json')).exists():
    raise SystemExit('refusing to overwrite historical gate evidence: ' + name)
with log.open('wb') as output:
    result = subprocess.run(command, cwd=root / 'specimen-studio', env=env, stdout=output, stderr=subprocess.STDOUT)
source_after = source_state()
record['source_unchanged_during_gate'] = source_before == source_after
record['source_files'] = source_before
record.update(ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), exit_code=result.returncode,
              log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
(folder / (name + '.json')).write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record))
sys.exit(result.returncode if result.returncode else (0 if source_before == source_after else 1))
