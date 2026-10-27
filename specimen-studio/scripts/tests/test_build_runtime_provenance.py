"""Exercise runtime orchestration offline; fake CMake derives emitted identities."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tarfile
import unittest
from unittest.mock import patch

HELPER = Path(__file__).resolve().parents[1] / 'build-runtimes.py'
PINS = {'llama': '1' * 40, 'diffusion': '2' * 40, 'ggml': '3' * 40}
FAKE_CMAKE = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args=sys.argv[1:]
if '--build' not in args:
    build=pathlib.Path(args[args.index('-B')+1]); build.mkdir(exist_ok=True)
    cache=build/'state.json'
    state=json.loads(cache.read_text()) if cache.exists() else {'GIT_EXE':'/stale/git','GIT_EXECUTABLE':'/stale/git','GGML_GIT_DIRTY':'-dirty'}
    for arg in args:
        if arg.startswith('-D'):
            key,value=arg[2:].split('=',1); state[key.split(':')[0]]=value
        if arg.startswith('-U'): state.pop(arg[2:],None)
    state['env']={k:v for k,v in os.environ.items() if k.startswith('GIT_')}
    state['args']=args
    cache.write_text(json.dumps(state))
else:
    build=pathlib.Path(args[args.index('--build')+1]); state=json.loads((build/'state.json').read_text())
    state['build_args']=args
    state['build_env']={k:v for k,v in os.environ.items() if k.startswith('GIT_')}
    (build/'state.json').write_text(json.dumps(state))
    target=args[args.index('--target')+1]
    key='LLAMA_BUILD_COMMIT' if target=='llama-server' else 'SDCPP_BUILD_COMMIT'
    commit=state.get(key,'parent-wdbx')
    if state.get('GIT_EXE')!='OFF' or state.get('GIT_EXECUTABLE')!='OFF' or state.get('GGML_GIT_DIRTY'): commit='parent-wdbx-dirty'
    if os.environ.get('WRONG_VERSION'): commit='unrelated-commit'
    windows=state.get('CMAKE_MSVC_RUNTIME_LIBRARY')=='MultiThreaded'
    out=build/'bin'/('Release' if windows else '')/(target+('.exe' if windows else '')); out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text('#!'+sys.executable+'\nimport sys\nassert sys.argv[1:]==["--version"]\nprint('+repr('commit '+commit)+')\n')
    out.chmod(0o755)
'''


class RuntimeProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'studio'
        (self.root / 'scripts').mkdir(parents=True)
        shutil.copy2(HELPER, self.root / 'scripts/build-runtimes.py')
        (self.root / 'native').mkdir()
        (self.root / 'src-tauri/binaries').mkdir(parents=True)
        (self.root / 'native/runtime-pins.json').write_text(json.dumps({
            k: {'repo': 'fixture/' + k, 'revision': v} for k, v in PINS.items()
        }))
        for key, pin in PINS.items():
            source = self.root / 'work/runtimes' / (key + '-' + pin[:12])
            source.mkdir(parents=True)
            (source / 'CMakeLists.txt').write_text('# fixture\n')
            (source / 'LICENSE').write_text(key + ' license')
        bin_dir = Path(self.temp.name) / 'bin'
        bin_dir.mkdir()
        cmake = bin_dir / 'cmake'
        cmake.write_text(FAKE_CMAKE.replace('/usr/bin/env python3', sys.executable))
        cmake.chmod(0o755)
        self.env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ['PATH'],
                        GIT_DIR='/foreign/git', GIT_WORK_TREE='/foreign/tree',
                        GIT_COMMON_DIR='/foreign/common', GIT_CEILING_DIRECTORIES='/existing/ceiling')

    def run_helper(self, only=None, **env):
        args = [sys.executable, str(self.root / 'scripts/build-runtimes.py')]
        if only:
            args += ['--only', only]
        return subprocess.run(args, env=dict(self.env, **env), capture_output=True, text=True, timeout=30)

    def test_each_selection_embeds_correct_source_and_clears_stale_git(self):
        # Removing metadata/cache overrides makes the fake emit the parent identity.
        for only in ('llama', 'diffusion', None):
            with self.subTest(only=only):
                for build in (self.root / 'work/runtimes').glob('*-build'):
                    shutil.rmtree(build)
                result = self.run_helper(only)
                self.assertEqual(result.returncode, 0, result.stderr)
                keys = [only] if only else ['llama', 'diffusion']
                for key in keys:
                    state = json.loads((self.root / 'work/runtimes' / (key + '-build/state.json')).read_text())
                    self.assertNotIn('GGML_GIT_DIRTY', state)
                    for flag in ('BUILD_SHARED_LIBS', 'GGML_NATIVE', 'GGML_OPENMP', 'GGML_METAL', 'GGML_VULKAN', 'LLAMA_CURL', 'LLAMA_OPENSSL'):
                        self.assertEqual(state[flag], 'OFF')
                    self.assertEqual(state['CMAKE_BUILD_TYPE'], 'Release')
                    self.assertEqual(state['build_args'][-2:], ['--parallel', '4'])
                    self.assertEqual(state['CMAKE_DISABLE_FIND_PACKAGE_Git'], 'TRUE')
                    for stage in ('env', 'build_env'):
                        self.assertFalse(set(state[stage]) & {'GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR'})
                        self.assertIn(str((self.root / 'work/runtimes').resolve()), state[stage]['GIT_CEILING_DIRECTORIES'].split(os.pathsep))
                    target = 'llama-server' if key == 'llama' else 'sd-cli'
                    identity = subprocess.check_output([str(self.root / 'src-tauri/binaries' / target), '--version'], text=True)
                    self.assertIn(PINS[key], identity)
                    if key == 'llama':
                        self.assertEqual(state['LLAMA_BUILD_NUMBER'], '0')
                        self.assertNotIn('GGML_BUILD_COMMIT', state)
                    else:
                        self.assertEqual(state['SDCPP_BUILD_VERSION'], 'unknown')
                        self.assertEqual(state['GGML_BUILD_COMMIT'], PINS['ggml'])
                        self.assertEqual(state['GGML_BUILD_NUMBER'], '0')
                if only == 'llama':
                    self.assertFalse((self.root / 'work/runtimes/diffusion-build').exists())
                if only == 'diffusion':
                    self.assertFalse((self.root / 'work/runtimes/llama-build').exists())
                    self.assertTrue((self.root / 'work/runtimes' / ('diffusion-' + PINS['diffusion'][:12]) / 'ggml/LICENSE').exists())

    def test_llama_api_sidecar_disables_both_unpinned_ui_routes(self):
        result = self.run_helper('llama')
        self.assertEqual(result.returncode, 0, result.stderr)
        state = json.loads((self.root / 'work/runtimes/llama-build/state.json').read_text())
        self.assertEqual(state.get('LLAMA_BUILD_UI'), 'OFF')
        self.assertEqual(state.get('LLAMA_USE_PREBUILT_UI'), 'OFF')
        self.assertEqual(state['LLAMA_BUILD_SERVER'], 'ON')

    def test_existing_ui_cache_is_refused_without_deleting_it(self):
        for relative in ['work/runtimes/llama-build/tools/ui/dist',
                         'work/runtimes/llama-' + PINS['llama'][:12] + '/tools/ui/dist']:
            with self.subTest(cache=relative):
                cache = self.root / relative
                cache.mkdir(parents=True, exist_ok=True)
                index = cache / 'index.html'
                index.write_bytes(b'unqualified cached UI')
                result = self.run_helper('llama')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(index.read_bytes(), b'unqualified cached UI')
                index.unlink()

    def test_stale_build_ui_stops_before_any_source_download(self):
        source = self.root / 'work/runtimes' / ('llama-' + PINS['llama'][:12])
        shutil.rmtree(source)
        assets = self.root / 'work/runtimes/llama-build/tools/ui/dist'
        assets.mkdir(parents=True)
        (assets / 'index.html').write_bytes(b'preserve unqualified UI')
        spec = importlib.util.spec_from_file_location('runtime_builder_no_download', HELPER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with patch.object(module, '__file__', str(self.root / 'scripts/build-runtimes.py')), \
             patch.object(module.urllib.request, 'urlretrieve', side_effect=AssertionError('download attempted')), \
             patch.dict(os.environ, self.env, clear=True):
            with self.assertRaisesRegex(ValueError, 'cached embedded UI'):
                module.main(['--only', 'llama'])
        self.assertEqual((assets / 'index.html').read_bytes(), b'preserve unqualified UI')

    def test_clean_source_extraction_runs_on_gate_python(self):
        source = self.root / 'work/runtimes' / ('llama-' + PINS['llama'][:12])
        shutil.rmtree(source)
        archive = Path(self.temp.name) / 'source.tar.gz'
        top = 'llama-' + PINS['llama']
        with tarfile.open(archive, 'w:gz') as contents:
            for relative, data in [('CMakeLists.txt', b'# fixture\n'), ('LICENSE', b'llama license')]:
                member = tarfile.TarInfo(top + '/' + relative)
                member.size = len(data)
                contents.addfile(member, io.BytesIO(data))
        spec = importlib.util.spec_from_file_location('runtime_builder_clean', HELPER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with patch.object(module, '__file__', str(self.root / 'scripts/build-runtimes.py')), \
             patch.object(module.urllib.request, 'urlretrieve', side_effect=lambda url, dest: shutil.copy2(archive, dest)), \
             patch.dict(os.environ, self.env, clear=True), contextlib.redirect_stdout(io.StringIO()):
            module.main(['--only', 'llama'])
        self.assertEqual((source / 'LICENSE').read_text(), 'llama license')
        self.assertTrue((self.root / 'src-tauri/binaries/llama-server').is_file())

    def test_wrong_version_leaves_existing_destination_unchanged(self):
        # Omitting the pre-copy identity check overwrites this known-good artifact.
        for key, target in [('llama', 'llama-server'), ('diffusion', 'sd-cli')]:
            with self.subTest(key=key):
                destination = self.root / 'src-tauri/binaries' / target
                destination.write_bytes(b'previous qualified executable')
                result = self.run_helper(key, WRONG_VERSION='1')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(destination.read_bytes(), b'previous qualified executable')

    def test_windows_preserves_static_msvc_paths_and_parallelism(self):
        spec = importlib.util.spec_from_file_location('runtime_builder_windows', HELPER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with patch.object(module, '__file__', str(self.root / 'scripts/build-runtimes.py')), \
             patch.object(module.platform, 'system', return_value='Windows'), \
             patch.dict(os.environ, self.env, clear=True), contextlib.redirect_stdout(io.StringIO()):
            before = dict(os.environ)
            module.main([])
            self.assertEqual(dict(os.environ), before)
        for key, target in [('llama', 'llama-server'), ('diffusion', 'sd-cli')]:
            state = json.loads((self.root / 'work/runtimes' / (key + '-build/state.json')).read_text())
            self.assertEqual(state['CMAKE_MSVC_RUNTIME_LIBRARY'], 'MultiThreaded')
            self.assertEqual(state['CMAKE_CXX_FLAGS_INIT'], '/bigobj')
            self.assertEqual(state['build_args'][-2:], ['--parallel', '2'])
            self.assertTrue((self.root / 'src-tauri/binaries' / (target + '.exe')).is_file())
            self.assertEqual((self.root / 'src-tauri/binaries' / (key + '-LICENSE.txt')).read_text(), key + ' license')

    def test_real_cmake_blocks_cached_git_and_emits_archive_pins(self):
        # Removing either OFF override enables a real CMake Git discovery path.
        # This synthetic fixture runs real CMake, never an upstream build or model.
        cmake = Path('/opt/homebrew/bin/cmake')
        self.assertTrue(cmake.is_file(), 'real CMake is a required prerequisite')
        tool = Path(self.temp.name) / 'bin/cmake'
        tool.unlink()
        tool.symlink_to(cmake)
        git = Path(self.temp.name) / 'bin/git'
        git.write_text('#!/bin/sh\necho called >> "$GIT_PROBE_LOG"\necho parent-wdbx\n')
        git.chmod(0o755)
        probe_log = Path(self.temp.name) / 'git-calls'
        fixture = r'''cmake_minimum_required(VERSION 3.20)
project(provenance NONE)
find_program(GIT_EXE NAMES git)
find_package(Git QUIET)
if(GIT_EXE)
 execute_process(COMMAND "${GIT_EXE}" rev-parse HEAD OUTPUT_VARIABLE FOREIGN)
 set(LLAMA_BUILD_COMMIT "parent-wdbx")
 set(SDCPP_BUILD_COMMIT "parent-wdbx")
endif()
if(GIT_EXECUTABLE)
 execute_process(COMMAND "${GIT_EXECUTABLE}" rev-parse HEAD)
endif()
if(DEFINED LLAMA_BUILD_COMMIT)
 set(COMMIT "${LLAMA_BUILD_COMMIT}")
 set(TARGET_NAME llama-server)
 set(GGML_BUILD_COMMIT "${LLAMA_BUILD_COMMIT}")
else()
 set(COMMIT "${SDCPP_BUILD_COMMIT}")
 set(TARGET_NAME sd-cli)
endif()
file(WRITE "${CMAKE_BINARY_DIR}/identity.txt" "${COMMIT};${GGML_BUILD_COMMIT};${LLAMA_BUILD_NUMBER};${GGML_BUILD_NUMBER};${SDCPP_BUILD_VERSION};${GGML_GIT_DIRTY}")
file(MAKE_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
configure_file("${CMAKE_SOURCE_DIR}/executable.in" "${CMAKE_BINARY_DIR}/bin/${TARGET_NAME}" @ONLY)
file(CHMOD "${CMAKE_BINARY_DIR}/bin/${TARGET_NAME}" PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE)
add_custom_target(${TARGET_NAME})
'''
        for key in ('llama', 'diffusion'):
            source = self.root / 'work/runtimes' / (key + '-' + PINS[key][:12])
            (source / 'CMakeLists.txt').write_text(fixture)
            (source / 'executable.in').write_text('#!' + sys.executable + '\nprint("commit @COMMIT@")\n')
            build = self.root / 'work/runtimes' / (key + '-build')
            build.mkdir()
            (build / 'CMakeCache.txt').write_text(
                'GIT_EXE:FILEPATH=' + str(git) + '\nGIT_EXECUTABLE:FILEPATH=' + str(git) +
                '\nGGML_GIT_DIRTY:STRING=-dirty\n')
        result = self.run_helper(GIT_PROBE_LOG=str(probe_log))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(probe_log.exists(), 'Git must not execute on configure or build')
        llama = (self.root / 'work/runtimes/llama-build/identity.txt').read_text()
        diffusion = (self.root / 'work/runtimes/diffusion-build/identity.txt').read_text()
        self.assertEqual(llama, PINS['llama'] + ';' + PINS['llama'] + ';0;;;')
        self.assertEqual(diffusion, PINS['diffusion'] + ';' + PINS['ggml'] + ';;0;unknown;')

    def test_import_has_no_build_side_effects_and_child_environment_is_copied(self):
        spec = importlib.util.spec_from_file_location('runtime_builder', HELPER)
        module = importlib.util.module_from_spec(spec)
        with patch.object(sys, 'argv', ['build-runtimes.py', '--invalid']):
            spec.loader.exec_module(module)
        with patch.dict(os.environ, self.env, clear=True):
            before = dict(os.environ)
            child = module.build_environment(self.root / 'work/runtimes')
            self.assertEqual(dict(os.environ), before)
            self.assertNotIn('GIT_DIR', child)
            self.assertIn('/existing/ceiling', child['GIT_CEILING_DIRECTORIES'].split(os.pathsep))


if __name__ == '__main__':
    unittest.main()
