#!/usr/bin/env python3
"""Ad-hoc seal a macOS bundle and record the resulting portable DMG."""

from __future__ import annotations

import hashlib
import json
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path

if platform.system() != "Darwin":
    raise SystemExit("macOS packaging only")

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "target/release/bundle/macos/WDBX Specimen Studio.app"
EVIDENCE = ROOT / "work/macos-package-verification.json"


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


images = list((ROOT / "target/release/bundle/dmg").glob("*.dmg"))
if not images:
    architecture = "aarch64" if platform.machine() == "arm64" else "x64"
    images = [
        ROOT
        / "target/release/bundle/dmg"
        / f"WDBX Specimen Studio_0.2.0_{architecture}.dmg"
    ]
    images[0].parent.mkdir(parents=True, exist_ok=True)
if len(images) != 1:
    raise SystemExit("Build exactly one macOS target before sealing")

binary_relative = Path("Contents/MacOS/wdbx-studio-desktop")
helpers_relative = [
    Path("Contents/Resources/binaries") / name
    for name in ("llama-server", "sd-cli")
]
pre_seal = {
    "application": sha256(SOURCE / binary_relative),
    "helpers": {
        path.name: sha256(SOURCE / path) for path in helpers_relative
    },
}

with tempfile.TemporaryDirectory(prefix="wdbx-package-") as temporary:
    stage = Path(temporary)
    app = stage / SOURCE.name
    subprocess.run(
        ["ditto", "--noextattr", "--norsrc", str(SOURCE), str(app)], check=True
    )
    for relative in helpers_relative:
        subprocess.run(
            ["codesign", "--force", "--sign", "-", str(app / relative)],
            check=True,
        )
    subprocess.run(
        ["codesign", "--force", "--sign", "-", str(app)], check=True
    )
    subprocess.run(
        ["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)],
        check=True,
    )
    post_seal = {
        "application": sha256(app / binary_relative),
        "helpers": {path.name: sha256(app / path) for path in helpers_relative},
    }
    (stage / "Applications").symlink_to("/Applications")
    subprocess.run(
        [
            "hdiutil",
            "create",
            "-ov",
            "-volname",
            "WDBX Specimen Studio",
            "-srcfolder",
            str(stage),
            "-format",
            "UDZO",
            str(images[0]),
        ],
        check=True,
    )
    subprocess.run(["hdiutil", "verify", str(images[0])], check=True)
    shutil.rmtree(SOURCE)
    subprocess.run(
        ["ditto", "--noextattr", "--norsrc", str(app), str(SOURCE)], check=True
    )

if EVIDENCE.is_file():
    evidence = json.loads(EVIDENCE.read_text())
    evidence["adHocSeal"] = {
        "applicationSha256Before": pre_seal["application"],
        "applicationSha256After": post_seal["application"],
        "helperSha256Before": pre_seal["helpers"],
        "helperSha256After": post_seal["helpers"],
        "dmgPath": str(images[0].relative_to(ROOT)),
        "dmgSha256": sha256(images[0]),
        "codesignStrictVerification": "passed",
        "dmgIntegrityVerification": "passed",
    }
    EVIDENCE.write_text(json.dumps(evidence, indent=2) + "\n")

print(
    "Ad-hoc bundle sealing and DMG integrity passed. "
    "Developer ID notarization is separate."
)
