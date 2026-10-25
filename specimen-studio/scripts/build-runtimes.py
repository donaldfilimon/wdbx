#!/usr/bin/env python3
"""Build the pinned inference executables for the current desktop target."""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request


# Conservative bounds for the reviewed source archives, including linked files.
SOURCE_MAX_ARCHIVE_BYTES = 512 * 1024 * 1024
SOURCE_MAX_MEMBERS = 50000
SOURCE_MAX_FILE_BYTES = 256 * 1024 * 1024
SOURCE_MAX_BYTES = 2 * 1024 * 1024 * 1024
WINDOWS_DEVICE_NAMES = {'CON', 'PRN', 'AUX', 'NUL', 'CONIN$', 'CONOUT$'} | {
    prefix + digit for prefix in ('COM', 'LPT') for digit in '123456789¹²³'
}


def source_path_parts(name, expected_root=None):
    """Reject ambiguous archive paths on both POSIX and Windows."""
    path = PurePosixPath(name)
    parts = name.rstrip('/').split('/')
    if (not name or path.is_absolute() or len(parts) > 64
            or len(name.encode('utf-8')) > 4096 or '\\' in name
            or any(part in ('', '.', '..') or '\x00' in part
                   or any(char in part for char in '<>:"|?*')
                   or part.endswith(('.', ' '))
                   or part.split('.', 1)[0].rstrip(' ').upper() in WINDOWS_DEVICE_NAMES
                   for part in parts)
            or (expected_root is not None and parts[0] != expected_root)):
        raise ValueError('Unsafe or unexpected source archive path: ' + name)
    return tuple(parts)


def extract_source_archive(archive, work, expected_root):
    """Validate first, then atomically publish a bounded Python 3.9 extraction.

    Internal file links are materialized as regular files for Windows portability;
    links to directories or other links are refused. No archive ownership, special
    modes, devices, timestamps or symlinks are installed on the host.
    """
    source_path_parts(expected_root)
    destination = work / expected_root
    if (work.is_symlink() or destination.exists() or destination.is_symlink()
            or archive.stat().st_size > SOURCE_MAX_ARCHIVE_BYTES):
        raise ValueError('Existing tree, linked work directory or oversized source archive')
    with tarfile.open(archive) as contents:
        members = {}
        total = 0
        for member in contents:
            if len(members) >= SOURCE_MAX_MEMBERS:
                raise ValueError('Too many source archive members')
            parts = source_path_parts(member.name, expected_root)
            if parts in members or member.type not in (
                    tarfile.REGTYPE, tarfile.AREGTYPE, tarfile.DIRTYPE,
                    tarfile.SYMTYPE, tarfile.LNKTYPE):
                raise ValueError('Duplicate path or unsupported source archive member')
            if member.size < 0 or member.size > SOURCE_MAX_FILE_BYTES:
                raise ValueError('Oversized source archive member')
            total += member.size
            if total > SOURCE_MAX_BYTES:
                raise ValueError('Source archive exceeds expanded byte limit')
            members[parts] = member
        targets = {}
        for parts, member in members.items():
            for depth in range(1, len(parts)):
                parent = members.get(parts[:depth])
                if parent is not None and not parent.isdir():
                    raise ValueError('Source member has a non-directory ancestor')
            if member.issym() or member.islnk():
                link = source_path_parts(member.linkname)
                target = parts[:-1] + link if member.issym() else link
                original = members.get(target)
                if (not target or target[0] != expected_root or original is None
                        or not original.isfile()):
                    raise ValueError('Source link must name a regular file inside its root')
                targets[parts] = original
                total += original.size
                if total > SOURCE_MAX_BYTES:
                    raise ValueError('Source links exceed expanded byte limit')
        with tempfile.TemporaryDirectory(prefix='.source-', dir=work) as stage:
            stage = Path(stage)
            for parts, member in members.items():
                target = stage.joinpath(*parts)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True, mode=0o755)
                    continue
                target.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
                original = targets.get(parts, member)
                with contents.extractfile(original) as source, target.open('xb') as output:
                    shutil.copyfileobj(source, output, length=1024 * 1024)
                if target.stat().st_size != original.size:
                    raise ValueError('Truncated source archive member')
                target.chmod(0o755 if original.mode & 0o111 else 0o644)
            for directory in stage.rglob('*'):
                if directory.is_dir():
                    directory.chmod(0o755)
            if not (stage / expected_root).is_dir():
                raise ValueError('Source archive has no expected root')
            (stage / expected_root).rename(destination)
    return destination


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


def refuse_embedded_ui(source, build):
    """Refuse stale UI inputs without removing user work or starting a download."""
    for assets in (source / 'tools/ui/dist', build / 'tools/ui/dist'):
        if (assets / 'index.html').exists():
            raise ValueError(
                'Preserve and inspect cached embedded UI assets before '
                'building the headless API sidecar: ' + str(assets)
            )


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
        if dest.is_symlink() or (dest.exists() and not (dest / 'CMakeLists.txt').is_file()):
            raise ValueError('Preserve and inspect the existing incomplete or linked source tree: ' + str(dest))
        if not (dest / 'CMakeLists.txt').exists():
            archive = work / (key + '.tar.gz')
            urllib.request.urlretrieve(
                'https://codeload.github.com/' + pin['repo'] + '/tar.gz/'
                + pin['revision'], archive,
            )
            top = pin['repo'].rsplit('/', 1)[-1] + '-' + pin['revision']
            extract_source_archive(archive, work, top).rename(dest)
        return dest

    out = root / 'src-tauri/binaries'
    out.mkdir(exist_ok=True)
    for key, target in [('llama', 'llama-server'), ('diffusion', 'sd-cli')]:
        if options.only and key != options.only:
            continue
        build = work / (key + '-build')
        if key == 'llama':
            cached_source = work / (key + '-' + pins[key]['revision'][:12])
            refuse_embedded_ui(cached_source, build)
        src = source(key)
        if key == 'llama':
            refuse_embedded_ui(src, build)
        if key == 'diffusion' and not (src / 'ggml/CMakeLists.txt').exists():
            ggml = source('ggml')
            shutil.copytree(
                ggml, src / 'ggml', dirs_exist_ok=True,
                ignore=shutil.ignore_patterns('.pi'),
            )
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
                # Studio consumes the authenticated API, never an embedded UI.
                # Prevent the upstream fallback from fetching mutable HF assets.
                '-DLLAMA_BUILD_UI=OFF', '-DLLAMA_USE_PREBUILT_UI=OFF',
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
