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
record = {
    'command': command,
    'cwd': str(root / 'specimen-studio'),
    'environment': {k: env[k] for k in ('PATH', 'RUSTUP_TOOLCHAIN')},
    'source_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
    'scope': 'Current uncommitted source; gate execution only, not installed/UI/research/owner qualification',
    'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
}
log = folder / (name + '.log')
if log.exists() or (folder / (name + '.json')).exists():
    raise SystemExit('refusing to overwrite historical gate evidence: ' + name)
with log.open('wb') as output:
    result = subprocess.run(command, cwd=root / 'specimen-studio', env=env, stdout=output, stderr=subprocess.STDOUT)
record.update(ended_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), exit_code=result.returncode,
              log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
(folder / (name + '.json')).write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record))
sys.exit(result.returncode)
