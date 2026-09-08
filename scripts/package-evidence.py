#!/usr/bin/env python3
"""Create and verify current-run desktop package qualification receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

SHA256 = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA = re.compile(r"^[0-9a-f]{40}$")


def fail(message: str) -> None:
    raise ValueError(message)


def inside(root: Path, relative: str, *, must_exist: bool = True) -> Path:
    if not relative or Path(relative).is_absolute():
        fail(f"Receipt path must be relative: {relative!r}")
    resolved_root = root.resolve()
    resolved = (resolved_root / relative).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError:
        fail(f"Receipt path escapes the package root: {relative}")
    if must_exist and not resolved.is_file():
        fail(f"Qualified file is missing: {relative}")
    return resolved


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def checked_sha(value: Any) -> str:
    if not isinstance(value, str) or not SHA256.fullmatch(value):
        fail(f"Invalid SHA-256 digest: {value!r}")
    return value


def file_record(root: Path, relative: str, *, named: bool = False) -> dict[str, str]:
    path = inside(root, relative)
    record = {"path": Path(relative).as_posix(), "sha256": digest(path)}
    if named:
        return {"name": path.name, **record}
    return record


def create(args: argparse.Namespace) -> dict[str, Any]:
    if not GIT_SHA.fullmatch(args.head_sha):
        fail("Source commit must be a lowercase 40-character Git SHA")
    if not args.workflow_run_id.isdecimal():
        fail("Workflow run ID must contain only digits")
    if not args.helper:
        fail("Record at least one packaged helper")
    if not args.installer:
        fail("Record at least one installer")
    root = Path(args.root)
    receipt = {
        "schema": 1,
        "headSha": args.head_sha,
        "workflowRunId": args.workflow_run_id,
        "runner": {
            "label": args.runner_label,
            "os": args.runner_os,
            "arch": args.runner_arch,
        },
        "application": file_record(root, args.application),
        "helpers": [file_record(root, path, named=True) for path in args.helper],
        "installers": [file_record(root, path) for path in args.installer],
    }
    output = inside(root, args.output, must_exist=False)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def verify_file(
    root: Path, record: Any, *, verify_contents: bool = True
) -> str:
    if not isinstance(record, dict) or not isinstance(record.get("path"), str):
        fail("Receipt file entry is malformed")
    expected = checked_sha(record.get("sha256"))
    path = inside(root, record["path"], must_exist=verify_contents)
    if verify_contents and digest(path) != expected:
        fail(f"Qualified file digest differs: {record['path']}")
    return record["path"]


def verify(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.root)
    receipt_path = inside(root, args.receipt)
    try:
        receipt = json.loads(receipt_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        fail(f"Qualification receipt is not valid JSON: {error}")
    if not isinstance(receipt, dict) or receipt.get("schema") != 1:
        fail("Unsupported package qualification receipt schema")
    if receipt.get("headSha") != args.expected_head_sha:
        fail("Receipt source commit differs from the expected commit")
    if receipt.get("workflowRunId") != args.expected_run_id:
        fail("Receipt workflow run differs from the requested qualification run")
    if not GIT_SHA.fullmatch(str(receipt.get("headSha", ""))):
        fail("Receipt source commit is not a valid Git SHA")
    if not str(receipt.get("workflowRunId", "")).isdecimal():
        fail("Receipt workflow run ID is malformed")
    runner = receipt.get("runner")
    if not isinstance(runner, dict) or not all(
        isinstance(runner.get(key), str) and runner[key]
        for key in ("label", "os", "arch")
    ):
        fail("Receipt runner identity is malformed")
    if (
        args.expected_runner_label
        and runner["label"] != args.expected_runner_label
    ):
        fail("Receipt runner differs from the requested qualification runner")
    helpers = receipt.get("helpers")
    installers = receipt.get("installers")
    if not isinstance(helpers, list) or not isinstance(installers, list):
        fail("Receipt package collections are malformed")
    verify_all_files = args.file_scope == "all"
    application = verify_file(
        root, receipt.get("application"), verify_contents=verify_all_files
    )
    helper_paths = [
        verify_file(root, entry, verify_contents=verify_all_files)
        for entry in helpers
    ]
    installer_paths = [verify_file(root, entry) for entry in installers]
    helper_names = [
        entry.get("name") for entry in helpers if isinstance(entry, dict)
    ]
    if not all(isinstance(name, str) and name for name in helper_names):
        fail("Receipt helper identity is malformed")
    missing = sorted(set(args.required_helper) - set(helper_names))
    if missing:
        fail(f"Missing required helper: {', '.join(missing)}")
    return {
        "application": application,
        "headSha": receipt["headSha"],
        "helpers": helper_names,
        "installers": installer_paths,
        "verified": True,
        "workflowRunId": receipt["workflowRunId"],
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    commands = result.add_subparsers(dest="command", required=True)
    create_parser = commands.add_parser("create")
    create_parser.add_argument("--root", required=True)
    create_parser.add_argument("--output", required=True)
    create_parser.add_argument("--head-sha", required=True)
    create_parser.add_argument("--workflow-run-id", required=True)
    create_parser.add_argument("--runner-label", required=True)
    create_parser.add_argument("--runner-os", required=True)
    create_parser.add_argument("--runner-arch", required=True)
    create_parser.add_argument("--application", required=True)
    create_parser.add_argument("--helper", action="append", default=[])
    create_parser.add_argument("--installer", action="append", default=[])
    create_parser.set_defaults(handler=create)
    verify_parser = commands.add_parser("verify")
    verify_parser.add_argument("--root", required=True)
    verify_parser.add_argument("--receipt", required=True)
    verify_parser.add_argument("--expected-head-sha", required=True)
    verify_parser.add_argument("--expected-run-id", required=True)
    verify_parser.add_argument("--expected-runner-label")
    verify_parser.add_argument("--required-helper", action="append", default=[])
    verify_parser.add_argument(
        "--file-scope", choices=("all", "installers"), default="all"
    )
    verify_parser.set_defaults(handler=verify)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        output = args.handler(args)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(output, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
