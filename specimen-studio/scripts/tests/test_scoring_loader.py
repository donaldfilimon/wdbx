"""Scoring probe loader follows the compiler selected by the caller."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check-scoring-parity.sh'


class ScoringLoaderTests(unittest.TestCase):
    def test_loader_uses_selected_sysroot_and_preserves_existing_search_path(self):
        with tempfile.TemporaryDirectory(prefix='scoring loader ') as directory:
            root = Path(directory)
            compiler = root / 'rustc'
            compiler.write_text('#!/bin/sh\nprintf "%s\\n" "$PROBE_SYSROOT"\n')
            compiler.chmod(0o755)
            env = dict(os.environ, PATH=f'{root}:/usr/bin:/bin',
                       PROBE_SYSROOT=str(root / 'selected toolchain'),
                       DYLD_LIBRARY_PATH='/existing/library')
            prefix = SCRIPT.read_text().split('for tool in', 1)[0]
            # Run the actual loader initialization without building the probes.
            prefix = prefix.replace('cd "$(dirname "$0")/.."', ':')
            result = subprocess.run(['/bin/bash', '-c', 'export DYLD_LIBRARY_PATH=/existing/library\n' + prefix +
                                     '\nprintf "%s" "$DYLD_LIBRARY_PATH"'],
                                    env=env, text=True, capture_output=True, check=True)
            self.assertEqual(result.stdout,
                             f'{root}/selected toolchain/lib:/existing/library')

    def test_objcopy_uses_selected_host_instead_of_fixed_apple_host(self):
        with tempfile.TemporaryDirectory(prefix='scoring host ') as directory:
            root = Path(directory)
            compiler = root / 'rustc'
            compiler.write_text('#!/bin/sh\ncase "$1" in\n-vV) printf "host: x86_64-unknown-linux-gnu\\n";;\n*) printf "%s\\n" "$PROBE_SYSROOT";;\nesac\n')
            compiler.chmod(0o755)
            env = dict(os.environ, PATH=f'{root}:/usr/bin:/bin',
                       PROBE_SYSROOT=str(root / 'selected toolchain'))
            prefix = SCRIPT.read_text().split('for tool in', 1)[0]
            prefix = prefix.replace('cd "$(dirname "$0")/.."', ':')
            result = subprocess.run(['/bin/bash', '-c', prefix +
                                     '\nprintf "%s" "${objcopy:-unset}"'],
                                    env=env, text=True, capture_output=True, check=True)
            self.assertEqual(result.stdout,
                             f'{root}/selected toolchain/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-objcopy')
