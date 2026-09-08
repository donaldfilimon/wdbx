#!/usr/bin/env python3
"""Qualify Linux package bytes, dependencies, helper startup and process launch.

Only run on disposable GitHub Linux runners. This is a process smoke test;
save/reopen, upgrades and graceful UI shutdown require separate interface tests.
"""

import hashlib
import json
import os
import platform
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(*arguments: str, **kwargs) -> str:
    return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT, **kwargs).strip()


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def payload(paths: list[Path], receipt: dict, environment: dict) -> list[dict]:
    records = []
    for record in [receipt["application"], *receipt["helpers"]]:
        name = Path(record["path"]).name
        matches = {path.resolve() for path in paths if path.name == name and path.is_file()}
        if len(matches) != 1:
            raise RuntimeError(f"Expected exactly one packaged {name}")
        path = matches.pop()
        if digest(path) != record["sha256"]:
            raise RuntimeError(f"Packaged payload differs from qualification: {name}")
        architecture = run("file", "-b", str(path))
        if "ELF 64-bit" not in architecture or "x86-64" not in architecture:
            raise RuntimeError(f"Packaged executable has wrong architecture: {name}")
        dependencies = run("ldd", str(path), env=environment)
        if "not found" in dependencies:
            raise RuntimeError(f"Packaged executable has unresolved dependencies: {name}")
        item = {"name": name, "sha256": digest(path), "architecture": architecture,
                "dependencies": dependencies}
        if name in {"llama-server", "sd-cli"}:
            run(str(path), "--help", env=environment, timeout=60)
            item["isolatedHelp"] = "passed"
        records.append(item)
    return records


def launch(executable: Path, environment: dict, log: Path) -> dict:
    with log.open("w") as output:
        process = subprocess.Popen([str(executable)], env=environment,
                                   stdout=output, stderr=subprocess.STDOUT)
        try:
            try:
                code = process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                pass
            else:
                raise RuntimeError(f"Packaged application exited early: {code}; see {log}")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    raise RuntimeError("Packaged application did not stop after SIGTERM")
    return {"aliveAfterSeconds": 10, "sigtermCleanup": "passed", "log": str(log.relative_to(ROOT))}


def main() -> None:
    if os.environ.get("GITHUB_ACTIONS") != "true" or platform.system() != "Linux":
        raise RuntimeError("Linux install checks require a disposable GitHub Actions Linux runner")
    receipt = json.loads((ROOT / "work/package-qualification.json").read_text())
    run("python3", str(ROOT / "scripts/package-evidence.py"), "verify", "--root", str(ROOT),
        "--receipt", "work/package-qualification.json", "--expected-head-sha", os.environ["GITHUB_SHA"],
        "--expected-run-id", os.environ["GITHUB_RUN_ID"], "--required-helper", "llama-server",
        "--required-helper", "sd-cli")
    evidence = {"schema": 1, "sourceSha": receipt["headSha"],
                "workflowRunId": receipt["workflowRunId"], "packages": [],
                "limitations": ["Process launch is not visual acceptance or save/reopen proof",
                                "Upgrade from a prior release is not tested"]}
    for record in receipt["installers"]:
        installer = ROOT / record["path"]
        with tempfile.TemporaryDirectory(prefix="wdbx-linux-package-") as directory:
            stage = Path(directory)
            environment = {**os.environ, "PATH": "/usr/bin:/bin", "HOME": str(stage / "home"),
                "XDG_DATA_HOME": str(stage / "data"), "XDG_CONFIG_HOME": str(stage / "config"),
                "XDG_CACHE_HOME": str(stage / "cache")}
            for name in ("HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"):
                Path(environment[name]).mkdir()
            item = {"installer": record}
            if installer.suffix == ".AppImage":
                run(str(installer), "--appimage-extract", cwd=stage)
                extracted = stage / "squashfs-root"
                item["payload"] = payload(list(extracted.rglob("*")), receipt, environment)
                # Extraction avoids a runner dependency on FUSE; AppRun still resolves bundled resources.
                item["launch"] = launch(extracted / "AppRun", environment, ROOT / "work/appimage-launch.log")
                item["mode"] = "AppImage extracted AppRun (no FUSE)"
            elif installer.suffix == ".deb":
                package = run("dpkg-deb", "-f", str(installer), "Package")
                existing = subprocess.run(["dpkg-query", "-W", "-f=${Status}", package],
                                          capture_output=True, text=True)
                if "installed" in existing.stdout:
                    raise RuntimeError("Refusing to replace a pre-existing Debian package")
                try:
                    run("sudo", "apt-get", "install", "-y", str(installer))
                    paths = [Path(line) for line in run("dpkg-query", "-L", package).splitlines()]
                    item["payload"] = payload(paths, receipt, environment)
                    name = Path(receipt["application"]["path"]).name
                    app = next(path for path in paths if path.name == name and path.is_file())
                    item["launch"] = launch(app, environment, ROOT / "work/deb-launch.log")
                finally:
                    run("sudo", "dpkg", "--remove", package)
                if app.exists():
                    raise RuntimeError("Debian uninstall left the application executable")
                item["install"] = "passed"
                item["uninstall"] = "passed"
            else:
                raise RuntimeError(f"Unexpected Linux installer: {installer.name}")
            evidence["packages"].append(item)
    if {Path(item["installer"]["path"]).suffix for item in evidence["packages"]} != {".deb", ".AppImage"}:
        raise RuntimeError("Both Debian and AppImage qualification are required")
    (ROOT / "work/linux-package-verification.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
