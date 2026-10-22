"""Cargo's embedded-asset build must not inherit its native target lock."""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    'build_desktop', Path(__file__).resolve().parents[1] / 'build-desktop.py',
)
build = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build)


class DesktopBuildTests(unittest.TestCase):
    def test_nested_cargo_has_a_separate_target_and_no_host_flags(self):
        inherited = {
            'CARGO_TARGET_DIR': '/native-target',
            'CARGO_ENCODED_RUSTFLAGS': '-Ctarget-cpu=native',
            'RUSTFLAGS': '-C target-cpu=native',
            'PATH': '/tools',
        }
        env = build.wasm_environment(Path('/studio'), inherited)
        self.assertEqual(env['CARGO_TARGET_DIR'], '/studio/target/desktop-wasm')
        self.assertNotIn('RUSTFLAGS', env)
        self.assertNotIn('CARGO_ENCODED_RUSTFLAGS', env)
        self.assertEqual(env['PATH'], '/tools')
        self.assertEqual(inherited['CARGO_TARGET_DIR'], '/native-target')

    @patch.object(build.shutil, 'copy2')
    @patch.object(build.subprocess, 'run')
    def test_wasm_is_built_before_the_frontend(self, run, copy):
        with patch.object(Path, 'is_file', return_value=True):
            build.main()
        self.assertEqual(run.call_count, 2)
        self.assertEqual(run.call_args_list[0].args[0][:2], ['cargo', 'build'])
        self.assertEqual(run.call_args_list[1].args[0],
                         ['bun', 'run', 'desktop:build'])
        copy.assert_called_once()

    @patch.object(build.shutil, 'copy2')
    @patch.object(build.subprocess, 'run')
    def test_missing_runtimes_are_built(self, run, _copy):
        with patch.object(Path, 'is_file', return_value=False):
            build.main()
        self.assertEqual(run.call_args_list[1].args[0],
                         ['python3', 'scripts/build-runtimes.py'])

    @patch.object(build.shutil, 'copytree')
    @patch.object(build.shutil, 'copy2')
    @patch.object(build.subprocess, 'run')
    def test_unbundled_launch_has_runtime_resources(self, _run, _copy, tree):
        with patch.object(Path, 'is_file', return_value=True):
            build.main(Path('/native-output'))
        tree.assert_called_once_with(
            build.ROOT / 'src-tauri/binaries',
            Path('/native-output/binaries'), dirs_exist_ok=True,
        )


if __name__ == '__main__':
    unittest.main()
