#!/usr/bin/env python3
"""Build the pinned inference executables for the current desktop target."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import urllib.request


def build_environment(work):
    """Keep archive builds from inheriting a caller's Git repository identity."""
    env = os.environ.copy()
    for name in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR'):
        env.pop(name, None)
    ceiling = env.get('GIT_CEILING_DIRECTORIES', '')
    directory = str(work.resolve())
    env['GIT_CEILING_DIRECTORIES'] = (
        ceiling + os.pathsep + directory if ceiling else directory
    )
    return env


def verify_executable(executable, revision, env):
    """Reject foreign identity before replacing an existing sidecar."""
    result = subprocess.run(
        [str(executable), '--version'], check=True, capture_output=True,
        text=True, timeout=30, env=env,
    )
    if revision not in result.stdout + result.stderr:
        raise RuntimeError(
            'Runtime executable does not report pinned source revision: '
            + str(executable)
        )


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('--only', choices=['llama', 'diffusion'])
    options = parser.parse_args(argv)
    root = Path(__file__).resolve().parents[1]
    work = root / 'work' / 'runtimes'
    work.mkdir(parents=True, exist_ok=True)
    pins = json.loads((root / 'native/runtime-pins.json').read_text())
    env = build_environment(work)

    def source(key):
        pin = pins[key]
        dest = work / (key + '-' + pin['revision'][:12])
        if not (dest / 'CMakeLists.txt').exists():
            archive = work / (key + '.tar.gz')
            urllib.request.urlretrieve(
                'https://codeload.github.com/' + pin['repo'] + '/tar.gz/'
                + pin['revision'], archive,
            )
            with tarfile.open(archive) as contents:
                top = contents.getnames()[0].split('/')[0]
                contents.extractall(work, filter='data')
            (work / top).rename(dest)
        return dest

    out = root / 'src-tauri/binaries'
    out.mkdir(exist_ok=True)
    for key, target in [('llama', 'llama-server'), ('diffusion', 'sd-cli')]:
        if options.only and key != options.only:
            continue
        src = source(key)
        if key == 'diffusion' and not (src / 'ggml/CMakeLists.txt').exists():
            ggml = source('ggml')
            shutil.copytree(
                ggml, src / 'ggml', dirs_exist_ok=True,
                ignore=shutil.ignore_patterns('.pi'),
            )
        build = work / (key + '-build')
        args = [
            'cmake', '-S', str(src), '-B', str(build),
            '-DCMAKE_BUILD_TYPE=Release', '-DBUILD_SHARED_LIBS=OFF',
            '-DGGML_NATIVE=OFF', '-DGGML_OPENMP=OFF', '-DGGML_METAL=OFF',
            '-DGGML_VULKAN=OFF', '-DLLAMA_BUILD_TESTS=OFF',
            '-DLLAMA_BUILD_EXAMPLES=OFF', '-DLLAMA_BUILD_SERVER=ON',
            '-DLLAMA_CURL=OFF', '-DLLAMA_OPENSSL=OFF', '-DSD_BUILD_EXAMPLES=ON',
            # OFF blocks discovery even when a reused cache contains real Git.
            '-DGIT_EXE:FILEPATH=OFF', '-DGIT_EXECUTABLE:FILEPATH=OFF',
            '-DCMAKE_DISABLE_FIND_PACKAGE_Git:BOOL=TRUE', '-UGGML_GIT_DIRTY',
        ]
        if key == 'llama':
            # Its bundled ggml belongs to this tree, not the external ggml pin.
            args += [
                '-DLLAMA_BUILD_COMMIT:STRING=' + pins['llama']['revision'],
                '-DLLAMA_BUILD_NUMBER:STRING=0',
            ]
        else:
            args += [
                '-DSDCPP_BUILD_COMMIT:STRING=' + pins['diffusion']['revision'],
                '-DSDCPP_BUILD_VERSION:STRING=unknown',
                '-DGGML_BUILD_COMMIT:STRING=' + pins['ggml']['revision'],
                '-DGGML_BUILD_NUMBER:STRING=0',
            ]
        windows = platform.system() == 'Windows'
        if windows:
            # HTTP loopback is authenticated; TLS belongs to model downloads.
            # Static MSVC linkage avoids developer-machine DLL dependencies.
            args += [
                '-UCMAKE_CXX_FLAGS', '-DCMAKE_CXX_FLAGS_INIT=/bigobj',
                '-DCMAKE_POLICY_DEFAULT_CMP0091=NEW',
                '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded',
            ]
        subprocess.run(args, check=True, env=env)
        subprocess.run([
            'cmake', '--build', str(build), '--config', 'Release',
            '--target', target, '--parallel', '2' if windows else '4',
        ], check=True, env=env)
        suffix = '.exe' if windows else ''
        choices = [
            build / 'bin' / ('Release' if windows else '') / (target + suffix),
            build / 'bin' / (target + suffix),
        ]
        executable = next((path for path in choices if path.exists()), None)
        if not executable:
            raise RuntimeError('Missing runtime executable ' + target)
        verify_executable(executable, pins[key]['revision'], env)
        shutil.copy2(executable, out / (target + suffix))
        for name in ['LICENSE', 'LICENSE.md', 'LICENSE.txt']:
            if (src / name).exists():
                shutil.copy2(src / name, out / (key + '-LICENSE.txt'))
                break
        if key == 'diffusion' and (src / 'ggml/LICENSE').exists():
            shutil.copy2(src / 'ggml/LICENSE', out / 'ggml-LICENSE.txt')
        print('Built', target, flush=True)


if __name__ == '__main__':
    main()
