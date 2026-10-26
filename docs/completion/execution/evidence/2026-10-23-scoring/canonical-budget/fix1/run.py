#!/usr/bin/env python3
"""Capture one exact local validation command without installing anything."""
import datetime
import json
import os
from pathlib import Path
import subprocess
import sys

base = Path(__file__).resolve().parent
root = base.parents[6]
name, *command = sys.argv[1:]
env = os.environ.copy()
env.update(
    RUSTUP_TOOLCHAIN="1.99.1",
    CARGO_NET_OFFLINE="true",
    DYLD_LIBRARY_PATH="/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib",
)
env["PATH"] = "/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:" + env["PATH"]
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
with (base / f"{name}.log").open("w") as log:
    result = subprocess.run(command, cwd=root / "specimen-studio", env=env,
                            stdout=log, stderr=subprocess.STDOUT)
(base / f"{name}.command.json").write_text(json.dumps({
    "command": command, "cwd": str(root / "specimen-studio"),
    "env": {k: env[k] for k in ["RUSTUP_TOOLCHAIN", "CARGO_NET_OFFLINE", "DYLD_LIBRARY_PATH", "PATH"]},
    "exit": result.returncode, "started_utc": started,
    "completed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
}, indent=2) + "\n")
print(f"{name}: exit {result.returncode}; {base / (name + '.log')}")
print((base / f"{name}.log").read_text()[-5000:])
sys.exit(result.returncode)
