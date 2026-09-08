#!/usr/bin/env python3
"""Consolidate frozen-SHA GitHub qualification evidence without overstating it."""

import argparse
import hashlib
import json
import math
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
    normalized = recorded_path.replace("\\", "/")
    source_path = Path(normalized)
    if ".." in source_path.parts:
        raise EvidenceError(f"unsafe recorded artifact path: {recorded_path!r}")
    relative = Path(source_path.name) if source_path.is_absolute() else source_path
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
    relative_receipt = receipt_path.resolve().relative_to(root.resolve())
    scope = root if len(relative_receipt.parts) == 1 else root / relative_receipt.parts[0]
    records: list[dict[str, str]] = []
    for key in ("application", "input", "output", "finalInstaller"):
        value = receipt.get(key)
        if isinstance(value, dict) and isinstance(value.get("path"), str) and isinstance(value.get("sha256"), str):
            strict = kind in ("macos-signing", "windows-signing") and key in ("output", "finalInstaller")
            recorder = artifact_record if strict else available_artifact_record
            records.append(recorder(scope, value["path"], value["sha256"]))
    for key in ("helpers", "installers"):
        value = receipt.get(key, [])
        if isinstance(value, list):
            for item in value:
                if isinstance(item, dict) and isinstance(item.get("path"), str) and isinstance(item.get("sha256"), str):
                    recorder = artifact_record if key == "installers" else available_artifact_record
                    records.append(recorder(scope, item["path"], item["sha256"]))
    ad_hoc = receipt.get("adHocSeal")
    if isinstance(ad_hoc, dict) and isinstance(ad_hoc.get("dmgPath"), str) and isinstance(ad_hoc.get("dmgSha256"), str):
        records.append(artifact_record(scope, ad_hoc["dmgPath"], ad_hoc["dmgSha256"]))
    named = {
        "resultSha256": f"{receipt.get('mode')}.json",
        "imageSha256": "generated-image.png",
        "logSha256": "cpu-gpu-parity.log",
    }
    for key, filename in named.items():
        if key in receipt:
            records.append(artifact_record(scope, filename, str(receipt[key])))
    if kind in ("text-inference", "ocr", "image-inference"):
        mode = {"text-inference": "text", "ocr": "ocr", "image-inference": "image"}[kind]
        result_path = find_payload(scope, f"{mode}.json")
        result = load_json(result_path, f"{kind} result")
        require(isinstance(result, dict) and bool(result), f"{kind} result is empty or malformed")
        if kind == "text-inference":
            require(isinstance(result.get("text"), str) and bool(result["text"].strip()), "text inference result contains no generated text")
        elif kind == "ocr":
            lines = result.get("ocr")
            require(isinstance(lines, list), "OCR result has no recognized lines")
            text = " ".join(str(line.get("text", "")) for line in lines if isinstance(line, dict))
            require("HELLO" in text and "WORLD" in text, "OCR result does not contain HELLO and WORLD")
        else:
            require(isinstance(result.get("artifactDigest"), str) and bool(result["artifactDigest"]), "image inference result has no artifact identity")
            image_path = find_payload(scope, "generated-image.png")
            require(image_path.stat().st_size > 0, "image inference artifact is empty")
    receipt_relative = str(receipt_path.resolve().relative_to(root.resolve()))
    records.append({
        "identity": receipt_path.name,
        "path": receipt_relative,
        "recordedSha256": None,
        "actualSha256": digest(receipt_path),
        "verification": "receipt-identity-only",
    })
    return records


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def validate_domain(kind: str, receipt: dict[str, Any], source_sha: str, run_id: str, package_qualification_run_id: str | None) -> dict[str, Any]:
    receipt_sha = receipt.get("headSha", receipt.get("sourceSha"))
    require(receipt_sha == source_sha, f"{kind} receipt must identify the frozen source SHA")
    workflow_run = receipt.get("workflowRunId")
    if kind not in ("macos-signing", "windows-signing", "browser"):
        require(str(workflow_run) == run_id, f"{kind} receipt run identity differs")
    if kind == "browser":
        require(receipt.get("passed") is True and isinstance(receipt.get("engine"), str), "browser receipt does not record successful browser checks")
        return {"engine": receipt["engine"], "checks": "passed"}
    require(receipt.get("schema") == 1, f"{kind} receipt is malformed")
    if kind == "desktop":
        def valid_file(value: Any) -> bool:
            return isinstance(value, dict) and isinstance(value.get("path"), str) and bool(value["path"]) and SHA256.fullmatch(str(value.get("sha256", ""))) is not None

        require(valid_file(receipt.get("application")), "desktop receipt has no application identity")
        require(isinstance(receipt.get("helpers"), list) and bool(receipt["helpers"]) and all(valid_file(item) and isinstance(item.get("name"), str) and bool(item["name"]) for item in receipt["helpers"]), "desktop receipt has no helper identities")
        require(isinstance(receipt.get("installers"), list) and bool(receipt["installers"]), "desktop receipt has no installer identity")
        require(all(valid_file(item) for item in receipt["installers"]), "desktop receipt installer identity is malformed")
        runner = receipt.get("runner")
        require(isinstance(runner, dict) and all(isinstance(runner.get(key), str) and bool(runner[key]) for key in ("label", "os", "arch")), "desktop receipt runner identity is malformed")
        return {"receiptValidation": "passed", "runner": receipt.get("runner")}
    if kind in ("text-inference", "ocr", "image-inference"):
        mode = {"text-inference": "text", "ocr": "ocr", "image-inference": "image"}[kind]
        require(isinstance(receipt.get("runner"), str) and bool(receipt["runner"]), f"{kind} receipt runner identity is missing")
        require(receipt.get("mode") == mode, f"{kind} receipt mode differs")
        require(SHA256.fullmatch(str(receipt.get("resultSha256", ""))) is not None, f"{kind} receipt has no valid result digest")
        if kind == "image-inference":
            require(SHA256.fullmatch(str(receipt.get("imageSha256", ""))) is not None, "image-inference receipt has no valid image digest")
        return {"mode": mode, "inference": "passed"}
    if kind == "accelerator":
        require(isinstance(receipt.get("runner"), str) and bool(receipt["runner"]), "accelerator receipt runner identity is missing")
        require(receipt.get("backend") == "wgpu", "accelerator receipt backend differs")
        max_delta = receipt.get("maxDelta")
        threshold = receipt.get("threshold")
        require(isinstance(max_delta, (int, float)) and not isinstance(max_delta, bool) and math.isfinite(max_delta) and 0 <= max_delta < 0.0001, "accelerator max delta must be finite, nonnegative, and below 0.0001")
        require(isinstance(threshold, (int, float)) and not isinstance(threshold, bool) and threshold == 0.0001, "accelerator threshold differs from 0.0001")
        require(SHA256.fullmatch(str(receipt.get("logSha256", ""))) is not None, "accelerator receipt has no valid log digest")
        return {"backend": "wgpu", "maxDelta": receipt["maxDelta"], "threshold": receipt["threshold"], "parity": "passed"}
    qualification = receipt.get("qualificationRunId")
    require(str(qualification).isdecimal(), f"{kind} qualification run linkage is malformed")
    if package_qualification_run_id is not None:
        require(str(qualification) == package_qualification_run_id, f"{kind} qualification run linkage differs from desktop qualification")
    if kind == "macos-signing":
        if "notarization" not in receipt:
            runtime_checks = receipt.get("runtimeChecks")
            ad_hoc = receipt.get("adHocSeal", {})
            require(receipt.get("applicationUnchangedBeforeSealing") is True, "macos portable application qualification was not preserved")
            require(isinstance(runtime_checks, list) and bool(runtime_checks) and all(item.get("isolatedHelpExitCode") == 0 for item in runtime_checks if isinstance(item, dict)) and all(isinstance(item, dict) for item in runtime_checks), "macos portable runtime checks are incomplete")
            require(ad_hoc.get("codesignStrictVerification") == "passed" and ad_hoc.get("dmgIntegrityVerification") == "passed", "macos portable seal verification is incomplete")
            require(SHA256.fullmatch(str(ad_hoc.get("dmgSha256", ""))) is not None, "macos portable DMG digest is missing")
            return {"portablePackaging": "passed", "signing": "unverified"}
        require(receipt.get("hardenedRuntime") is True and receipt.get("secureTimestamps") is True, "macos-signing security properties are incomplete")
        require(receipt.get("finalInstallerPayloadVerification") == "passed", "macos-signing final payload verification failed")
        notarization = receipt.get("notarization", {})
        require(notarization.get("status") == "Accepted", "macos-signing notarization was not accepted")
        require(isinstance(receipt.get("signingIdentityFingerprint"), str) and bool(receipt["signingIdentityFingerprint"]), "macos-signing identity is missing")
        return {"notarization": "Accepted", "finalInstallerPayloadVerification": "passed", "hardenedRuntime": True, "secureTimestamps": True, "stapler": receipt.get("stapler"), "gatekeeper": receipt.get("gatekeeper")}
    verification = receipt.get("verification")
    require(receipt.get("applicationQualifiedBeforeSigning") is True, "windows-signing application was not qualification-linked")
    require(receipt.get("timestampStatus") == "verified", "windows-signing timestamp is not verified")
    require(isinstance(verification, list) and bool(verification), "windows-signing has no signature verification results")
    require(all(item.get("signtool") == "passed" and item.get("authenticodeStatus") == "Valid" and item.get("timestampStatus") == "verified" for item in verification if isinstance(item, dict)) and all(isinstance(item, dict) for item in verification), "windows-signing verification failed")
    smoke = receipt.get("installSmoke", {})
    required_smoke = ("silentInstall", "installedHelperIsolatedStartup", "launch", "cleanShutdown", "silentUninstall")
    require(all(smoke.get(key) == "passed" for key in required_smoke), "windows-signing install smoke is incomplete")
    return {"timestampStatus": "verified", "verification": verification, "installSmoke": smoke}


def inspect_run(entry: dict[str, Any], repository: str, source_sha: str, package_qualification_run_id: str | None = None) -> dict[str, Any]:
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
    root = Path(entry["artifacts"])
    json_paths = list(root.rglob("*.json")) if root.is_dir() else []
    expected = RECEIPTS[kind]
    if kind == "macos-signing":
        receipt_paths = [path for path in json_paths if path.name == "macos-package-verification.json" or path.name.endswith("-notarized.json")]
    else:
        receipt_paths = [path for path in json_paths if any(token in path.name for token in expected)]
    if not receipt_paths:
        if status == "passed":
            result["status"] = "unverified"
            result["blocker"] = "successful workflow has no required domain receipt"
        return result
    if status == "passed" and kind == "macos-signing" and not any(path.name.endswith("-notarized.json") for path in receipt_paths):
        result["status"] = "blocked"
        result["blocker"] = "portable packaging passed without Developer ID notarization evidence"
    domains = []
    for path in receipt_paths:
        receipt = load_json(path, f"{kind} receipt")
        if not isinstance(receipt, dict):
            raise EvidenceError(f"{kind} receipt is malformed")
        domains.append(validate_domain(kind, receipt, source_sha, run_id, package_qualification_run_id))
        result["artifacts"].extend(receipt_artifacts(root, path, receipt, kind))
        result.setdefault("receipts", []).append(str(path.relative_to(root)))
    result["domain"] = domains[0] if len(domains) == 1 else {"receipts": domains}
    if status == "passed" and any(item.get("verification") == "payload-not-uploaded" for item in result["artifacts"]):
        result["status"] = "unverified"
        result["blocker"] = "receipt-recorded package payload was not uploaded for independent hash verification"
    return result


def render_markdown(summary: dict[str, Any]) -> str:
    lines = ["# Qualification summary", "", f"Source SHA: `{summary['sourceSha']}`", f"Overall: **{summary['overall']}**", "", "## Hosted runs", ""]
    for run in summary["runs"]:
        target = f"[{run.get('runId')}]({run.get('url')})" if run.get("url") else "not supplied"
        lines.append(f"- **{run['kind']}**: {run['status']} ({target})")
        if run.get("blocker"):
            lines.append(f"  - Blocker: {run['blocker']}")
        for error in run.get("errors", []):
            lines.append(f"  - Evidence error: {error}")
        for job in run.get("jobs", []):
            lines.append(f"  - Job `{job.get('name')}`: {job.get('conclusion') or 'unverified'}")
        if run.get("domain"):
            lines.append(f"  - Domain result: `{json.dumps(run['domain'], sort_keys=True)}`")
        for artifact in run.get("artifacts", []):
            lines.append(f"  - `{artifact['identity']}`: `{artifact['actualSha256']}` ({artifact['verification']})")
    lines.extend(["", "## Local and manual evidence", ""])
    for item in summary["localManualEvidence"]:
        lines.append(f"- **{item.get('label', 'evidence')}**: {item.get('status', 'unverified')}")
        for key in ("sourceSha", "url", "sha256", "notes", "blocker"):
            if item.get(key) is not None:
                lines.append(f"  - {key}: `{item[key]}`")
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
        entries = manifest.get("runs", [])
        if not isinstance(entries, list):
            raise EvidenceError("qualification runs must be an array")
        runs = []
        package_qualification_run_id = manifest.get("packageQualificationRunId")
        if package_qualification_run_id is not None and not str(package_qualification_run_id).isdecimal():
            raise EvidenceError("package qualification run linkage is malformed")
        for entry in entries:
            kind = entry.get("kind", "unknown") if isinstance(entry, dict) else "unknown"
            try:
                runs.append(inspect_run(entry, repository, source_sha, str(package_qualification_run_id) if package_qualification_run_id is not None else None))
            except (EvidenceError, OSError, KeyError, TypeError) as error:
                message = str(error)
                print(f"error: {message}", file=sys.stderr)
                failed = {"kind": kind, "status": "failed", "errors": [message]}
                if isinstance(entry, dict):
                    failed["runId"] = str(entry.get("runId", ""))
                    try:
                        metadata = load_json(Path(entry["metadata"]), f"{kind} workflow metadata")
                        if isinstance(metadata, dict):
                            failed["url"] = metadata.get("html_url")
                            failed["conclusion"] = metadata.get("conclusion")
                    except (EvidenceError, OSError, KeyError, TypeError):
                        pass
                runs.append(failed)
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
