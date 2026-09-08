#!/usr/bin/env python3
"""Check production frontend assets and the desktop normal dependency graph.

This records build-input exclusion, not a binary audit or runtime GUI test.
Run immediately after the production Tauri build, before creating E2E assets.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BRIDGE_MARKERS = (
    b"__wdbxTestCall",
    b"wdioTauri",
    b"__wdio_spy__",
    b"__wdio_original_tauri__",
    b"WDIO Tauri Plugin",
    b"plugin:wdio",
)
FORBIDDEN_PACKAGES = {
    "tauri-plugin-wdio", "tauri-plugin-wdio-webdriver", "wdio-webdriver",
}


def inspect_assets(assets: Path) -> list[dict]:
    if not assets.is_dir() or not (assets / "index.html").is_file():
        raise ValueError("Production assets must contain index.html")
    scripts = sorted(path for path in assets.rglob("*")
                     if path.is_file() and path.suffix in {".js", ".mjs", ".cjs"})
    if not scripts:
        raise ValueError("Production assets contain no JavaScript; exclusion cannot be established")
    records = []
    for path in [assets / "index.html", *scripts]:
        content = path.read_bytes()
        if not content:
            raise ValueError(f"Production asset is empty: {path.name}")
        for marker in BRIDGE_MARKERS:
            if marker in content:
                raise ValueError(f"Test bridge marker {marker.decode()} in {path.relative_to(assets)}")
        records.append({"path": str(path.relative_to(assets)), "bytes": len(content),
                        "sha256": hashlib.sha256(content).hexdigest()})
    return records


def verify_dependency_tree(tree: str) -> list[str]:
    lines = [line.strip() for line in tree.splitlines() if line.strip()]
    if not lines or not lines[0].startswith("wdbx-studio-desktop v"):
        raise ValueError("Normal dependency graph must be rooted at wdbx-studio-desktop")
    packages = {line.split()[0] for line in lines}
    if "tauri" not in packages:
        raise ValueError("Normal dependency graph is incomplete: tauri is absent")
    forbidden = sorted(packages & FORBIDDEN_PACKAGES)
    if forbidden:
        raise ValueError("Production normal graph contains test dependencies: " + ", ".join(forbidden))
    return sorted(packages)


def capture(command: list[str]) -> str:
    return subprocess.check_output(command, cwd=ROOT, text=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, default=ROOT / "desktop-dist")
    parser.add_argument("--output", type=Path, default=ROOT / "work/production-bridge-verification.json")
    parser.add_argument("--target", help="Cargo target used by the production build; defaults to host")
    args = parser.parse_args()
    if os.environ.get("VITE_NATIVE_E2E") == "1":
        raise ValueError("Production exclusion gate cannot run with VITE_NATIVE_E2E=1")
    records = inspect_assets(args.assets)
    command = ["cargo", "tree", "--locked", "--package", "wdbx-studio-desktop",
               "--features", "custom-protocol", "--edges", "normal", "--prefix", "none",
               "--format", "{p}"]
    if args.target:
        command.extend(["--target", args.target])
    tree = capture(command)
    packages = verify_dependency_tree(tree)
    evidence = {
        "schema": 1,
        "sourceSha": capture(["git", "rev-parse", "HEAD"]).strip(),
        "trackedOrUntrackedChangesPresent": bool(capture(["git", "status", "--porcelain"]).strip()),
        "workflowRunId": os.environ.get("GITHUB_RUN_ID"),
        "assetsRoot": str(args.assets.resolve()),
        "assetFiles": records,
        "dependencyGraph": {
            "command": command,
            "sha256": hashlib.sha256(tree.encode()).hexdigest(),
            "packageCount": len(packages),
            "rustCompiler": capture(["rustc", "-vV"]).strip(),
            "target": args.target or "host",
            "features": ["custom-protocol"],
            "edges": "normal",
        },
        "result": "passed",
        "evidenceBoundary": "Generated frontend bytes and resolved production build inputs only; not a compiled-binary or GUI-runtime exclusion proof",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
