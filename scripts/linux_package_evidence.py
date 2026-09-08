#!/usr/bin/env python3
"""Bind Linux package payloads before and after AppImage construction."""

import argparse
import hashlib
import json
import shutil
from pathlib import Path
from typing import Any

REQUIRED = ("wdbx-studio-desktop", "llama-server", "sd-cli")


class EvidenceError(ValueError):
    pass


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"invalid qualification receipt {path}: {error}") from error
    if not isinstance(value, dict) or value.get("schema") != 1:
        raise EvidenceError(f"invalid qualification receipt {path}")
    return value


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def unique(root: Path, name: str, label: str) -> Path:
    resolved_root = root.resolve()
    matches = {
        path.resolve()
        for path in root.rglob(name)
        if path.is_file() and path.resolve().is_relative_to(resolved_root)
    }
    require(len(matches) == 1, f"expected exactly one {label} {name}, found {len(matches)}")
    return matches.pop()


def raw_records(receipt: dict[str, Any]) -> dict[str, dict[str, str]]:
    application = receipt.get("application")
    helpers = receipt.get("helpers")
    require(isinstance(application, dict), "base receipt application is missing")
    require(isinstance(helpers, list), "base receipt helpers are missing")
    records = [application, *helpers]
    result: dict[str, dict[str, str]] = {}
    for record in records:
        require(isinstance(record, dict), "base receipt payload record is malformed")
        name = str(record.get("name") or Path(str(record.get("path", ""))).name)
        sha = record.get("sha256")
        require(name in REQUIRED and isinstance(sha, str) and len(sha) == 64,
                "base receipt payload identity is malformed")
        require(name not in result, f"base receipt has duplicate payload {name}")
        result[name] = {"path": str(record.get("path", "")), "sha256": sha}
    require(set(result) == set(REQUIRED), "base receipt does not identify all required Linux payloads")
    return result


def appimage_record(receipt: dict[str, Any], installer: Path) -> dict[str, str]:
    records = [item for item in receipt.get("installers", [])
               if isinstance(item, dict) and Path(str(item.get("path", ""))).suffix == ".AppImage"]
    require(len(records) == 1, "base receipt must identify exactly one AppImage")
    record = records[0]
    require(Path(str(record.get("path"))).name == installer.name,
            "AppImage identity differs from base qualification")
    actual = digest(installer)
    require(record.get("sha256") == actual, "AppImage digest differs from base qualification")
    return {"path": str(record["path"]), "sha256": actual}


def validate_linkage(stage: dict[str, Any], base: dict[str, Any], base_path: Path,
                     installer: Path, source_sha: str, run_id: str) -> None:
    require(base.get("headSha") == source_sha and stage.get("sourceSha") == source_sha,
            "source SHA differs from Linux qualification")
    require(str(base.get("workflowRunId")) == run_id and str(stage.get("workflowRunId")) == run_id,
            "workflow run differs from Linux qualification")
    require(stage.get("baseQualification", {}).get("receiptSha256") == digest(base_path),
            "base qualification receipt digest differs")
    raw = raw_records(base)
    require(stage.get("baseQualification", {}).get("payloadSha256") ==
            {name: raw[name]["sha256"] for name in REQUIRED},
            "raw-qualified payload linkage differs")
    installer_record = appimage_record(base, installer)
    require(stage.get("installer") == installer_record,
            "AppImage linkage differs from base qualification")


def create(args: argparse.Namespace) -> None:
    root = Path(args.root).resolve()
    base_path = Path(args.base_receipt).resolve()
    base = load(base_path)
    require(base_path.is_relative_to(root), "base receipt must be within qualification root")
    require(base.get("headSha") == args.expected_source_sha, "source SHA differs from base qualification")
    require(str(base.get("workflowRunId")) == args.expected_run_id,
            "workflow run differs from base qualification")
    raw = raw_records(base)
    installer = Path(args.installer).resolve()
    require(installer.is_relative_to(root), "AppImage must be within qualification root")
    installer_record = appimage_record(base, installer)
    appdir = Path(args.appdir).resolve()
    require(appdir.is_dir() and appdir.is_relative_to(root), "AppDir must be within qualification root")
    retained = Path(args.retained_payload).resolve()
    require(retained.is_relative_to(root), "retained payload must be within qualification root")
    retained.mkdir(parents=True, exist_ok=True)
    payload = []
    for name in REQUIRED:
        source = unique(appdir, name, "staged AppDir executable")
        destination = retained / name
        require(not destination.exists(), f"refusing duplicate retained payload {name}")
        shutil.copy2(source, destination)
        payload.append({
            "name": name,
            "appDirPath": str(source.relative_to(root)),
            "sha256": digest(source),
            "rawQualifiedSha256": raw[name]["sha256"],
            "retainedPath": str(destination.relative_to(root)),
        })
    receipt = {
        "schema": 1,
        "sourceSha": args.expected_source_sha,
        "workflowRunId": args.expected_run_id,
        "baseQualification": {
            "path": str(base_path.relative_to(root)),
            "receiptSha256": digest(base_path),
            "payloadSha256": {name: raw[name]["sha256"] for name in REQUIRED},
        },
        "installer": installer_record,
        "payload": payload,
    }
    output = Path(args.output).resolve()
    require(output.is_relative_to(root), "stage receipt must be within qualification root")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2) + "\n")


def verified_stage(args: argparse.Namespace) -> dict[str, str]:
    root = Path(args.root).resolve()
    receipt_path = Path(args.receipt).resolve()
    base_path = Path(args.base_receipt).resolve()
    require(receipt_path.is_relative_to(root) and base_path.is_relative_to(root),
            "qualification receipts must be within qualification root")
    stage = load(receipt_path)
    base = load(base_path)
    installer = Path(args.installer).resolve()
    require(installer.is_relative_to(root), "AppImage must be within qualification root")
    validate_linkage(stage, base, base_path, installer, args.expected_source_sha,
                     args.expected_run_id)
    records = stage.get("payload")
    require(isinstance(records, list) and len(records) == len(REQUIRED),
            "staged AppDir qualification payload is incomplete")
    expected: dict[str, str] = {}
    for record in records:
        require(isinstance(record, dict) and record.get("name") in REQUIRED,
                "staged AppDir qualification payload identity is malformed")
        name = record["name"]
        require(name not in expected, f"staged AppDir qualification has duplicate payload {name}")
        require(record.get("rawQualifiedSha256") == raw_records(base)[name]["sha256"],
                f"raw-qualified linkage differs for {name}")
        retained = (root / str(record.get("retainedPath", ""))).resolve()
        require(retained.is_relative_to(root) and retained.is_file(),
                f"retained staged AppDir payload is missing: {name}")
        require(digest(retained) == record.get("sha256"),
                f"retained staged AppDir qualification differs: {name}")
        expected[name] = str(record["sha256"])
    return expected


def verify_appimage(args: argparse.Namespace) -> None:
    expected = verified_stage(args)
    extracted = Path(args.extracted).resolve()
    root = Path(args.root).resolve()
    require(extracted.is_dir() and extracted.is_relative_to(root),
            "extracted AppImage must be within qualification root")
    for name in REQUIRED:
        path = unique(extracted, name, "extracted AppImage executable")
        require(digest(path) == expected[name],
                f"extracted payload differs from staged AppDir qualification: {name}")


def verify_raw(args: argparse.Namespace) -> None:
    base = load(Path(args.base_receipt))
    raw = raw_records(base)
    payload_root = Path(args.payload_root).resolve()
    for name in REQUIRED:
        path = unique(payload_root, name, "raw package executable")
        require(digest(path) == raw[name]["sha256"],
                f"packaged payload differs from raw build qualification: {name}")


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser()
    commands = value.add_subparsers(dest="command", required=True)
    create_parser = commands.add_parser("create")
    for flag in ("root", "base-receipt", "appdir", "installer", "output",
                 "retained-payload", "expected-source-sha", "expected-run-id"):
        create_parser.add_argument(f"--{flag}", required=True)
    create_parser.set_defaults(function=create)
    verify_parser = commands.add_parser("verify-appimage")
    for flag in ("root", "receipt", "base-receipt", "installer", "extracted",
                 "expected-source-sha", "expected-run-id"):
        verify_parser.add_argument(f"--{flag}", required=True)
    verify_parser.set_defaults(function=verify_appimage)
    raw_parser = commands.add_parser("verify-raw")
    raw_parser.add_argument("--base-receipt", required=True)
    raw_parser.add_argument("--payload-root", required=True)
    raw_parser.set_defaults(function=verify_raw)
    return value


def main() -> int:
    try:
        args = parser().parse_args()
        args.function(args)
        return 0
    except (EvidenceError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=__import__("sys").stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
