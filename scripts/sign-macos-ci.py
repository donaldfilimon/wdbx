#!/usr/bin/env python3
"""Run Developer ID distribution signing with an ephemeral CI keychain."""

import base64
import os
import secrets
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "MACOS_SIGNING_CERTIFICATE_BASE64",
    "MACOS_SIGNING_CERTIFICATE_PASSWORD",
    "MACOS_SIGNING_IDENTITY",
    "APPLE_API_KEY_BASE64",
    "APPLE_API_KEY_ID",
    "APPLE_API_ISSUER",
)


def run(*arguments: str) -> str:
    # Do not expose command arguments on failure: several contain credentials.
    result = subprocess.run(arguments, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{Path(arguments[0]).name} operation failed (exit {result.returncode})")
    return result.stdout.strip()


def main() -> None:
    if os.environ.get("GITHUB_ACTIONS") != "true" or sys.platform != "darwin":
        raise RuntimeError("This credential setup is restricted to macOS GitHub Actions runners")
    missing = [name for name in REQUIRED if not os.environ.get(name, "").strip()]
    if missing:
        raise RuntimeError("Missing signing secrets: " + ", ".join(missing))
    images = list((ROOT / "target/release/bundle/dmg").glob("*.dmg"))
    if len(images) != 1:
        raise RuntimeError("Expected exactly one qualified portable DMG")
    original = run("security", "default-keychain", "-d", "user").strip('"')
    search_list = [line.strip().strip('"') for line in run(
        "security", "list-keychains", "-d", "user"
    ).splitlines()]
    with tempfile.TemporaryDirectory(prefix="wdbx-signing-") as directory:
        stage = Path(directory)
        keychain = str(stage / "signing.keychain-db")
        certificate = stage / "certificate.p12"
        api_key = stage / "AuthKey.p8"
        certificate.write_bytes(base64.b64decode(os.environ[REQUIRED[0]], validate=True))
        api_key.write_bytes(base64.b64decode(os.environ["APPLE_API_KEY_BASE64"], validate=True))
        certificate.chmod(0o600)
        api_key.chmod(0o600)
        password = secrets.token_hex(32)
        run("security", "create-keychain", "-p", password, keychain)
        try:
            run("security", "set-keychain-settings", "-lut", "7200", keychain)
            run("security", "unlock-keychain", "-p", password, keychain)
            run("security", "import", str(certificate), "-k", keychain,
                "-P", os.environ["MACOS_SIGNING_CERTIFICATE_PASSWORD"],
                "-T", "/usr/bin/codesign")
            run("security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:",
                "-s", "-k", password, keychain)
            run("security", "list-keychains", "-d", "user", "-s", keychain, *search_list)
            run("security", "default-keychain", "-d", "user", "-s", keychain)
            run("xcrun", "notarytool", "store-credentials", "wdbx-release",
                "--key", str(api_key), "--key-id", os.environ["APPLE_API_KEY_ID"],
                "--issuer", os.environ["APPLE_API_ISSUER"])
            print(run(sys.executable, str(ROOT / "scripts/sign-notarize-macos.py"),
                "--input", str(images[0]), "--identity", os.environ["MACOS_SIGNING_IDENTITY"],
                "--keychain-profile", "wdbx-release", "--output-dir", str(ROOT / "work/notarized")))
        finally:
            try:
                run("security", "default-keychain", "-d", "user", "-s", original)
                run("security", "list-keychains", "-d", "user", "-s", *search_list)
            finally:
                run("security", "delete-keychain", keychain)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
