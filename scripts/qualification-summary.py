#!/usr/bin/env python3
"""Consolidate frozen-SHA GitHub qualification evidence without overstating it."""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

GIT_SHA = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
WORKFLOWS = {
    "desktop": ".github/workflows/desktop.yml",
    "browser": ".github/workflows/browser.yml",
    "text-inference": ".github/workflows/desktop.yml",
    "image-inference": ".github/workflows/image-model.yml",
    "ocr": ".github/workflows/ocr.yml",
    "accelerator": ".github/workflows/accelerator.yml",
    "macos-signing": ".github/workflows/macos-package.yml",
    "windows-signing": ".github/workflows/windows-package.yml",
}
RECEIPTS = {
    "desktop": ("package-qualification.json",),
    "browser": ("browser-checks",),
    "text-inference": ("run-receipt.json",),
    "image-inference": ("run-receipt.json",),
    "ocr": ("run-receipt.json",),
    "accelerator": ("run-receipt.json",),
    "macos-signing": ("macos-package-verification.json", "-notarized.json"),
    "windows-signing": ("windows-package-verification.json",),
}


class EvidenceError(ValueError):
    pass


def load_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"{label} is not valid JSON: {error}") from error


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            value.update(chunk)
    return value.hexdigest()


def find_payload(root: Path, recorded_path: str) -> Path:
    relative = Path(recorded_path)
    if relative.is_absolute() or ".." in relative.parts:
        raise EvidenceError(f"unsafe recorded artifact path: {recorded_path!r}")
    resolved_root = root.resolve()
    direct = (resolved_root / relative).resolve()
    if direct.is_file() and direct.is_relative_to(resolved_root):
        return direct
    matches = []
    for path in root.rglob(relative.name):
        resolved = path.resolve()
        if path.is_file() and resolved.is_relative_to(resolved_root):
            matches.append(resolved)
    if len(matches) != 1:
        raise EvidenceError(
            f"recorded artifact {recorded_path!r} has {len(matches)} matching payloads"
        )
    return matches[0]


def artifact_record(root: Path, path: str, recorded: str) -> dict[str, str]:
    if not SHA256.fullmatch(recorded):
        raise EvidenceError(f"recorded artifact {path!r} has an invalid SHA-256")
    payload = find_payload(root, path)
    actual = digest(payload)
    if actual != recorded:
        raise EvidenceError(f"artifact digest differs for {path}")
    return {
        "identity": payload.name,
        "path": str(payload.relative_to(root.resolve())),
        "recordedSha256": recorded,
        "actualSha256": actual,
        "verification": "passed",
    }


def available_artifact_record(root: Path, path: str, recorded: str) -> dict[str, str]:
    try:
        return artifact_record(root, path, recorded)
    except EvidenceError as error:
        if "has 0 matching payloads" not in str(error):
            raise
        return {
            "identity": Path(path).name,
            "path": path,
            "recordedSha256": recorded,
            "actualSha256": "unavailable",
            "verification": "payload-not-uploaded",
        }


def receipt_artifacts(root: Path, receipt_path: Path, receipt: dict[str, Any], kind: str) -> list[dict[str, Any]]:
    records: list[dict[str, str]] = []
    for key in ("application", "input", "output", "finalInstaller"):
        value = receipt.get(key)
        if isinstance(value, dict) and isinstance(value.get("path"), str) and isinstance(value.get("sha256"), str):
            strict = kind in ("macos-signing", "windows-signing") and key in ("output", "finalInstaller")
            recorder = artifact_record if strict else available_artifact_record
            records.append(recorder(root, value["path"], value["sha256"]))
    for key in ("helpers", "installers"):
        value = receipt.get(key, [])
        if isinstance(value, list):
            for item in value:
                if isinstance(item, dict) and isinstance(item.get("path"), str) and isinstance(item.get("sha256"), str):
                    recorder = artifact_record if key == "installers" else available_artifact_record
                    records.append(recorder(root, item["path"], item["sha256"]))
    ad_hoc = receipt.get("adHocSeal")
    if isinstance(ad_hoc, dict) and isinstance(ad_hoc.get("dmgPath"), str) and isinstance(ad_hoc.get("dmgSha256"), str):
        records.append(artifact_record(root, ad_hoc["dmgPath"], ad_hoc["dmgSha256"]))
    named = {
        "resultSha256": f"{receipt.get('mode')}.json",
        "imageSha256": "generated-image.png",
        "logSha256": "cpu-gpu-parity.log",
    }
    for key, filename in named.items():
        if key in receipt:
            records.append(artifact_record(root, filename, str(receipt[key])))
    receipt_relative = str(receipt_path.resolve().relative_to(root.resolve()))
    records.append({
        "identity": receipt_path.name,
        "path": receipt_relative,
        "recordedSha256": None,
        "actualSha256": digest(receipt_path),
        "verification": "receipt-identity-only",
    })
    return records


def inspect_run(entry: dict[str, Any], repository: str, source_sha: str) -> dict[str, Any]:
    kind = entry.get("kind")
    if kind not in WORKFLOWS:
        raise EvidenceError(f"unknown qualification kind: {kind!r}")
    if not entry.get("runId"):
        if entry.get("required", True):
            return {"kind": kind, "status": "unverified", "blocker": "run ID was not supplied"}
        return {"kind": kind, "status": "blocked", "blocker": "credentials or qualifying run were not supplied"}
    run_id = str(entry["runId"])
    if not run_id.isdecimal():
        raise EvidenceError(f"{kind} run ID is malformed")
    metadata = load_json(Path(entry["metadata"]), f"{kind} workflow metadata")
    if not isinstance(metadata, dict):
        raise EvidenceError(f"{kind} workflow metadata must be an object")
    if str(metadata.get("id")) != run_id:
        raise EvidenceError(f"{kind} run identity differs")
    if metadata.get("head_sha") != source_sha:
        raise EvidenceError(f"{kind} source SHA differs")
    if metadata.get("repository", {}).get("full_name") != repository:
        raise EvidenceError(f"{kind} repository differs")
    if metadata.get("path") != WORKFLOWS[kind]:
        raise EvidenceError(f"{kind} workflow differs")
    if metadata.get("status") != "completed":
        status = "unverified"
    elif metadata.get("conclusion") != "success":
        status = "failed"
    else:
        status = "passed"
    jobs_value = entry.get("jobs", [])
    if isinstance(jobs_value, str):
        jobs_document = load_json(Path(jobs_value), f"{kind} jobs metadata")
        if not isinstance(jobs_document, dict) or not isinstance(jobs_document.get("jobs"), list):
            raise EvidenceError(f"{kind} jobs metadata is malformed")
        jobs_value = [
            {"name": job.get("name"), "conclusion": job.get("conclusion")}
            for job in jobs_document["jobs"]
            if isinstance(job, dict)
        ]
    result: dict[str, Any] = {
        "kind": kind,
        "status": status,
        "runId": run_id,
        "url": metadata.get("html_url"),
        "workflow": metadata["path"],
        "conclusion": metadata.get("conclusion"),
        "jobs": jobs_value,
        "artifacts": [],
    }
    if status != "passed":
        return result
    root = Path(entry["artifacts"])
    json_paths = list(root.rglob("*.json")) if root.is_dir() else []
    expected = RECEIPTS[kind]
    if kind == "macos-signing":
        receipt_paths = [path for path in json_paths if path.name.endswith("-notarized.json")]
    else:
        receipt_paths = [path for path in json_paths if any(token in path.name for token in expected)]
    if not receipt_paths:
        result["status"] = "unverified"
        result["blocker"] = "successful workflow has no required domain receipt"
        return result
    for path in receipt_paths:
        receipt = load_json(path, f"{kind} receipt")
        valid_browser = isinstance(receipt, dict) and kind == "browser" and receipt.get("passed") is True
        if not isinstance(receipt, dict) or (receipt.get("schema") != 1 and not valid_browser):
            raise EvidenceError(f"{kind} receipt is malformed")
        receipt_sha = receipt.get("headSha", receipt.get("sourceSha"))
        if receipt_sha is not None and receipt_sha != source_sha:
            raise EvidenceError(f"{kind} receipt source SHA differs")
        workflow_run = receipt.get("workflowRunId")
        if workflow_run is not None and str(workflow_run) != run_id:
            raise EvidenceError(f"{kind} receipt run identity differs")
        result["artifacts"].extend(receipt_artifacts(root, path, receipt, kind))
        result.setdefault("receipts", []).append(str(path.relative_to(root)))
    return result


def render_markdown(summary: dict[str, Any]) -> str:
    lines = ["# Qualification summary", "", f"Source SHA: `{summary['sourceSha']}`", f"Overall: **{summary['overall']}**", "", "## Hosted runs", ""]
    for run in summary["runs"]:
        target = f"[{run.get('runId')}]({run.get('url')})" if run.get("url") else "not supplied"
        lines.append(f"- **{run['kind']}**: {run['status']} ({target})")
        if run.get("blocker"):
            lines.append(f"  - Blocker: {run['blocker']}")
        for artifact in run.get("artifacts", []):
            lines.append(f"  - `{artifact['identity']}`: `{artifact['actualSha256']}` ({artifact['verification']})")
    lines.extend(["", "## Local and manual evidence", ""])
    for item in summary["localManualEvidence"]:
        lines.append(f"- **{item.get('label', 'evidence')}**: {item.get('status', 'unverified')}")
    lines.extend(["", "> A green workflow establishes only its recorded checks. Inference and signing require their specific verified receipts.", ""])
    return "\n".join(lines)


def normalize_local_evidence(value: Any, source_sha: str) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        raise EvidenceError("local/manual evidence must be an array")
    normalized = []
    for item in value:
        if not isinstance(item, dict) or not isinstance(item.get("label"), str):
            raise EvidenceError("local/manual evidence record is malformed")
        record = {
            key: item[key]
            for key in ("label", "status", "sourceSha", "url", "sha256", "notes")
            if key in item
        }
        if record.get("sourceSha") != source_sha:
            record["status"] = "unverified"
            record["blocker"] = "evidence source SHA differs from frozen source"
        elif record.get("status") not in ("passed", "failed", "blocked", "unverified"):
            raise EvidenceError("local/manual evidence status is malformed")
        if "sha256" in record and not SHA256.fullmatch(str(record["sha256"])):
            raise EvidenceError("local/manual evidence digest is malformed")
        normalized.append(record)
    return normalized


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True)
    args = parser.parse_args()
    try:
        manifest = load_json(Path(args.manifest), "qualification manifest")
        if not isinstance(manifest, dict) or manifest.get("schema") != 1:
            raise EvidenceError("qualification manifest is malformed")
        source_sha = manifest.get("sourceSha")
        if not isinstance(source_sha, str) or not GIT_SHA.fullmatch(source_sha):
            raise EvidenceError("source SHA must be a lowercase 40-character Git SHA")
        repository = manifest.get("repository")
        if not isinstance(repository, str) or "/" not in repository:
            raise EvidenceError("repository identity is malformed")
        runs = [inspect_run(entry, repository, source_sha) for entry in manifest.get("runs", [])]
        local_evidence = normalize_local_evidence(manifest.get("localManualEvidence", []), source_sha)
        priority = ("failed", "unverified", "blocked", "passed")
        statuses = [run["status"] for run in runs] + [item["status"] for item in local_evidence]
        overall = next((state for state in priority if state in statuses), "unverified")
        summary = {
            "schema": 1,
            "repository": repository,
            "sourceSha": source_sha,
            "overall": overall,
            "runs": runs,
            "localManualEvidence": local_evidence,
        }
        Path(manifest["outputJson"]).write_text(json.dumps(summary, indent=2) + "\n")
        Path(manifest["outputMarkdown"]).write_text(render_markdown(summary))
        print(json.dumps(summary, sort_keys=True))
        return 0 if overall in ("passed", "blocked") else 1
    except (EvidenceError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
