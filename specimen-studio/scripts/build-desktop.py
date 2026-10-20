#!/usr/bin/env python3
"""Build embedded desktop assets, including when invoked by Cargo's build.rs."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[1]


def wasm_environment(root, inherited):
    """Nested Cargo must not wait on its parent's target-directory lock."""
    env = inherited.copy()
    env['CARGO_TARGET_DIR'] = str(root / 'target' / 'desktop-wasm')
    # The outer native invocation may supply host-only compiler flags.
    for name in ('CARGO_ENCODED_RUSTFLAGS', 'RUSTFLAGS'):
        env.pop(name, None)
    return env


def main(native_output=None):
    env = wasm_environment(ROOT, os.environ)
    subprocess.run(
        ['cargo', 'build', '--locked', '-p', 'specimen-wasm',
         '--target', 'wasm32-unknown-unknown', '--release'],
        cwd=ROOT, env=env, check=True,
    )
    destination = ROOT / 'lib/specimen/wasm'
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(
        Path(env['CARGO_TARGET_DIR'])
        / 'wasm32-unknown-unknown/release/specimen_wasm.wasm',
        destination / 'specimen_wasm.wasm',
    )
    if any(not (ROOT / 'src-tauri/binaries' / name).is_file()
           for name in ('llama-server', 'sd-cli')):
        subprocess.run(
            ['python3', 'scripts/build-runtimes.py'], cwd=ROOT, check=True,
        )
    subprocess.run(['bun', 'run', 'desktop:build'], cwd=ROOT, check=True)
    if native_output is not None:
        # Cargo does not run Tauri's bundler, so stage its runtime resources
        # beside the executable for resource_dir() in an unbundled launch.
        shutil.copytree(
            ROOT / 'src-tauri/binaries', native_output / 'binaries',
            dirs_exist_ok=True,
        )


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-output', type=Path)
    main(parser.parse_args().native_output)
