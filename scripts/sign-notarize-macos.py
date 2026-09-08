#!/usr/bin/env python3
"""Developer ID-sign and notarize a current-head portable macOS DMG."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    raise RuntimeError(message)


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def command(
    arguments: list[str], *, capture: bool = False
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=True,
        text=True,
        capture_output=capture,
    )


def captured(arguments: list[str]) -> str:
    result = command(arguments, capture=True)
    return f"{result.stdout}{result.stderr}".strip()


def find_qualification_receipt(image: Path) -> Path:
    candidates = [ROOT / "work/macos-package-verification.json"]
    for parent in (image.parent, *image.parents):
        candidates.extend(
            [
                parent / "macos-package-verification.json",
                parent / "work/macos-package-verification.json",
            ]
        )
    matches = []
    for candidate in candidates:
        resolved = candidate.resolve()
        if resolved.is_file() and resolved not in matches:
            matches.append(resolved)
    if len(matches) != 1:
        fail(
            "Exactly one macos-package-verification.json receipt must be "
            "available beside or above the input DMG"
        )
    return matches[0]


def verify_current_head(image: Path) -> tuple[str, dict[str, Any]]:
    receipt_path = find_qualification_receipt(image)
    try:
        receipt = json.loads(receipt_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"Portable qualification receipt is invalid: {error}")
    head_sha = captured(["git", "-C", str(ROOT), "rev-parse", "HEAD"])
    if receipt.get("sourceSha") != head_sha:
        fail("Portable DMG receipt does not match the current Git commit")
    recorded = receipt.get("adHocSeal", {}).get("dmgSha256")
    if not isinstance(recorded, str) or not re.fullmatch(r"[0-9a-f]{64}", recorded):
        fail("Portable DMG receipt has no valid sealed-image digest")
    if sha256(image) != recorded:
        fail("Portable DMG digest differs from its qualification receipt")
    return head_sha, receipt


def signing_fingerprint(identity: str) -> str:
    identities = captured(
        ["security", "find-identity", "-v", "-p", "codesigning"]
    )
    match = re.search(
        rf"\b([0-9A-Fa-f]{{40}})\s+\"{re.escape(identity)}\"", identities
    )
    if not match:
        fail("Requested Developer ID Application identity is not installed")
    return match.group(1).lower()


def preflight_notary_profile(profile: str) -> None:
    try:
        command(
            [
                "xcrun",
                "notarytool",
                "history",
                "--keychain-profile",
                profile,
                "--output-format",
                "json",
            ],
            capture=True,
        )
    except subprocess.CalledProcessError:
        fail("notarytool keychain profile is unavailable or not authorized")


def mount_application(image: Path, destination: Path) -> Path:
    mount = destination.parent / "qualified-mount"
    mount.mkdir()
    command(
        [
            "hdiutil",
            "attach",
            str(image),
            "-readonly",
            "-nobrowse",
            "-mountpoint",
            str(mount),
        ]
    )
    try:
        applications = list(mount.glob("*.app"))
        if len(applications) != 1:
            fail("Portable DMG must contain exactly one application")
        command(
            [
                "ditto",
                "--noextattr",
                "--norsrc",
                str(applications[0]),
                str(destination),
            ]
        )
    finally:
        command(["hdiutil", "detach", str(mount)])
    return destination


def is_macho(path: Path) -> bool:
    result = subprocess.run(
        ["file", "-b", str(path)], capture_output=True, text=True, check=True
    )
    return "Mach-O" in result.stdout


def sign_application(app: Path, identity: str) -> list[str]:
    candidates = [
        path
        for path in app.rglob("*")
        if path.is_file() and not path.is_symlink() and is_macho(path)
    ]
    signed = []
    for path in sorted(candidates, key=lambda item: len(item.parts), reverse=True):
        command(
            [
                "codesign",
                "--force",
                "--options",
                "runtime",
                "--timestamp",
                "--sign",
                identity,
                str(path),
            ]
        )
        signed.append(str(path.relative_to(app)))
    nested = [
        path
        for path in app.rglob("*")
        if path.is_dir()
        and path.suffix in {".appex", ".app", ".framework", ".xpc"}
    ]
    for path in sorted(nested, key=lambda item: len(item.parts), reverse=True):
        command(
            [
                "codesign",
                "--force",
                "--options",
                "runtime",
                "--timestamp",
                "--sign",
                identity,
                str(path),
            ]
        )
    command(
        [
            "codesign",
            "--force",
            "--options",
            "runtime",
            "--timestamp",
            "--sign",
            identity,
            str(app),
        ]
    )
    command(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)])
    return signed


def create_dmg(payload: Path, output: Path) -> None:
    (payload / "Applications").symlink_to("/Applications")
    command(
        [
            "hdiutil",
            "create",
            "-ov",
            "-volname",
            "WDBX Specimen Studio",
            "-srcfolder",
            str(payload),
            "-format",
            "UDZO",
            str(output),
        ]
    )
    command(["hdiutil", "verify", str(output)])


def gatekeeper(arguments: list[str]) -> str:
    result = subprocess.run(arguments, text=True, capture_output=True)
    output = f"{result.stdout}{result.stderr}".strip()
    if result.returncode:
        fail(f"Gatekeeper assessment failed: {output}")
    return output


def notarize(
    image: Path, profile: str
) -> tuple[str, str, str, str, str]:
    result = command(
        [
            "xcrun",
            "notarytool",
            "submit",
            str(image),
            "--keychain-profile",
            profile,
            "--wait",
            "--output-format",
            "json",
        ],
        capture=True,
    )
    response = json.loads(result.stdout)
    status = response.get("status")
    submission_id = response.get("id")
    if status != "Accepted" or not submission_id:
        fail(f"Apple notarization was not accepted: {status}")
    stapler = captured(["xcrun", "stapler", "staple", str(image)])
    validation = captured(["xcrun", "stapler", "validate", str(image)])
    command(["codesign", "--verify", "--strict", "--verbose=2", str(image)])
    dmg_assessment = gatekeeper(
        [
            "spctl",
            "--assess",
            "--verbose=4",
            "--type",
            "open",
            "--context",
            "context:primary-signature",
            str(image),
        ]
    )
    return str(submission_id), str(status), stapler, validation, dmg_assessment


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument("--input", required=True)
    result.add_argument("--identity", required=True)
    result.add_argument("--keychain-profile", required=True)
    result.add_argument("--output-dir", required=True)
    return result


def main() -> None:
    args = parser().parse_args()
    if not args.identity.startswith("Developer ID Application:"):
        fail("Developer ID Application identity is required; Apple Development is insufficient")
    if platform.system() != "Darwin":
        fail("Developer ID signing and notarization require macOS")
    source = Path(args.input).expanduser().resolve()
    if not source.is_file() or source.suffix.lower() != ".dmg":
        fail("Input must be an existing portable DMG")
    head_sha, qualification = verify_current_head(source)
    fingerprint = signing_fingerprint(args.identity)
    preflight_notary_profile(args.keychain_profile)

    output_dir = Path(args.output_dir).expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    final_image = output_dir / f"{source.stem}-notarized.dmg"
    receipt_path = output_dir / f"{source.stem}-notarized.json"
    if final_image.exists() or receipt_path.exists():
        fail("Refusing to overwrite an existing notarized artifact or receipt")

    input_sha = sha256(source)
    with tempfile.TemporaryDirectory(prefix="wdbx-developer-id-") as temporary:
        stage = Path(temporary)
        payload = stage / "payload"
        payload.mkdir()
        app = mount_application(source, payload / "WDBX Specimen Studio.app")
        signed_files = sign_application(app, args.identity)
        architecture = captured(
            ["lipo", "-archs", str(app / "Contents/MacOS/wdbx-studio-desktop")]
        )
        staged_image = stage / final_image.name
        create_dmg(payload, staged_image)
        command(
            [
                "codesign",
                "--force",
                "--timestamp",
                "--sign",
                args.identity,
                str(staged_image),
            ]
        )
        command(["codesign", "--verify", "--strict", "--verbose=2", str(staged_image)])
        submission_id, status, stapler, stapler_validation, dmg_assessment = notarize(
            staged_image, args.keychain_profile
        )
        application_assessment = gatekeeper(
            ["spctl", "--assess", "--verbose=4", "--type", "execute", str(app)]
        )
        output_sha = sha256(staged_image)
        shutil.copy2(staged_image, final_image)

    receipt = {
        "schema": 1,
        "sourceSha": head_sha,
        "architecture": architecture,
        "input": {"path": str(source), "sha256": input_sha},
        "output": {"path": str(final_image), "sha256": output_sha},
        "qualificationRunId": qualification.get("qualificationRunId"),
        "signingIdentityFingerprint": fingerprint,
        "hardenedRuntime": True,
        "secureTimestamps": True,
        "signedMachOFiles": signed_files,
        "notarization": {"submissionId": submission_id, "status": status},
        "stapler": {"staple": stapler, "validate": stapler_validation},
        "gatekeeper": {
            "application": application_assessment,
            "dmg": dmg_assessment,
        },
    }
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
