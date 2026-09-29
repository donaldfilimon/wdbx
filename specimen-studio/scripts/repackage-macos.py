#!/usr/bin/env python3
"""Repackage a current-head qualified macOS app with a portable text helper."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "target/release/bundle/macos/WDBX Specimen Studio.app"


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def required_environment(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"{name} is required")
    return value


def record_by_name(records: Any, name: str) -> dict[str, str]:
    if not isinstance(records, list):
        raise RuntimeError("Qualification receipt has no helper records")
    matches = [record for record in records if record.get("name") == name]
    if len(matches) != 1:
        raise RuntimeError(f"Qualification receipt must contain one {name}")
    return matches[0]


def require_digest(path: Path, expected: str, label: str) -> str:
    actual = sha256(path)
    if actual != expected:
        raise RuntimeError(f"{label} digest differs from qualification receipt")
    return actual


def main() -> None:
    if os.uname().sysname != "Darwin":
        raise RuntimeError("macOS repackaging requires macOS")
    runner = required_environment("WDBX_RUNNER")
    run_id = required_environment("WDBX_QUALIFICATION_RUN")
    head_sha = required_environment("GITHUB_SHA")
    repository = required_environment("GITHUB_REPOSITORY")
    if not run_id.isdecimal():
        raise RuntimeError("WDBX_QUALIFICATION_RUN must be a numeric run ID")

    download = ROOT / "work/qualified-application"
    if download.exists():
        shutil.rmtree(download)
    subprocess.run(
        [
            "gh",
            "run",
            "download",
            run_id,
            "-n",
            f"wdbx-studio-{runner}",
            "-D",
            str(download),
        ],
        check=True,
    )
    receipts = list(download.rglob("package-qualification.json"))
    if len(receipts) != 1:
        raise RuntimeError("Qualified artifact must contain exactly one receipt")
    receipt_path = receipts[0]
    metadata_path = ROOT / "work/qualification-run.json"
    metadata_path.write_text(subprocess.check_output(
        ["gh", "api", f"repos/{repository}/actions/runs/{run_id}"], text=True
    ))
    subprocess.run(
        [
            "python3",
            str(ROOT / "scripts/package-evidence.py"),
            "verify",
            "--root",
            str(download),
            "--receipt",
            str(receipt_path.relative_to(download)),
            "--expected-head-sha",
            head_sha,
            "--expected-run-id",
            run_id,
            "--expected-runner-label",
            runner,
            "--required-helper",
            "llama-server",
            "--required-helper",
            "sd-cli",
            "--file-scope",
            "installers",
            "--workflow-run-metadata",
            str(metadata_path),
            "--expected-repository",
            repository,
            "--expected-workflow",
            ".github/workflows/desktop.yml",
        ],
        check=True,
    )

    receipt = json.loads(receipt_path.read_text())
    images = [
        record
        for record in receipt["installers"]
        if record["path"].endswith(".dmg")
    ]
    if len(images) != 1:
        raise RuntimeError("Qualified artifact must contain exactly one DMG")
    image_record = images[0]
    image = download / image_record["path"]
    qualified_dmg_sha = require_digest(
        image, image_record["sha256"], "Qualified DMG"
    )

    APP.parent.mkdir(parents=True, exist_ok=True)
    if APP.exists():
        shutil.rmtree(APP)
    application_record = receipt["application"]
    helper_records = {
        name: record_by_name(receipt["helpers"], name)
        for name in ("llama-server", "sd-cli")
    }
    qualified_helper_hashes: dict[str, str] = {}
    with tempfile.TemporaryDirectory(prefix="wdbx-qualified-mount-") as temporary:
        subprocess.run(
            [
                "hdiutil",
                "attach",
                str(image),
                "-readonly",
                "-nobrowse",
                "-mountpoint",
                temporary,
            ],
            check=True,
        )
        try:
            source = Path(temporary) / APP.name
            if not source.is_dir():
                raise RuntimeError("Qualified DMG does not contain the application")
            binary = source / "Contents/MacOS/wdbx-studio-desktop"
            qualified_application_sha = require_digest(
                binary,
                application_record["sha256"],
                "Qualified application",
            )
            for name, record in helper_records.items():
                helper = source / "Contents/Resources/binaries" / name
                qualified_helper_hashes[name] = require_digest(
                    helper, record["sha256"], f"Qualified helper {name}"
                )
            subprocess.run(
                ["ditto", "--noextattr", "--norsrc", str(source), str(APP)],
                check=True,
            )
        finally:
            subprocess.run(["hdiutil", "detach", temporary], check=True)

    staged_binary = APP / "Contents/MacOS/wdbx-studio-desktop"
    require_digest(
        staged_binary, qualified_application_sha, "Copied qualified application"
    )
    portable_helper = ROOT / "src-tauri/binaries/llama-server"
    destination = APP / "Contents/Resources/binaries/llama-server"
    portable_helper_sha = sha256(portable_helper)
    shutil.copy2(portable_helper, destination)
    require_digest(destination, portable_helper_sha, "Portable text helper")

    runtime_checks = []
    for name in ("llama-server", "sd-cli"):
        path = APP / "Contents/Resources/binaries" / name
        lines = subprocess.check_output(["otool", "-L", str(path)], text=True)
        dependencies = [
            line.strip().split(" (")[0] for line in lines.splitlines()[1:]
        ]
        if any(
            not dependency.startswith(("/usr/lib/", "/System/Library/"))
            for dependency in dependencies
        ):
            raise RuntimeError(f"Non-system helper dependency: {dependencies}")
        result = subprocess.run(
            [str(path), "--help"],
            capture_output=True,
            text=True,
            env={**os.environ, "PATH": "/usr/bin:/bin"},
            timeout=60,
        )
        if result.returncode:
            raise RuntimeError(
                f"{name} failed isolated startup: {result.returncode}"
            )
        runtime_checks.append(
            {
                "name": name,
                "sha256BeforeSealing": sha256(path),
                "dependencies": dependencies,
                "isolatedHelpExitCode": result.returncode,
            }
        )

    evidence = {
        "schema": 1,
        "sourceSha": head_sha,
        "qualificationRunId": run_id,
        "qualificationRunner": runner,
        "qualifiedDmgSha256": qualified_dmg_sha,
        "applicationSha256BeforeSealing": qualified_application_sha,
        "applicationUnchangedBeforeSealing": True,
        "qualifiedHelperSha256": qualified_helper_hashes,
        "replacement": {
            "name": "llama-server",
            "qualifiedSha256": qualified_helper_hashes["llama-server"],
            "portableSha256": portable_helper_sha,
        },
        "runtimeChecks": runtime_checks,
    }
    output = ROOT / "work/macos-package-verification.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
