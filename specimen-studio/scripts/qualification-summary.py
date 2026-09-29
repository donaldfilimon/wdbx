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
    "desktop": ".github/workflows/specimen-studio-desktop.yml",
    "browser": ".github/workflows/specimen-studio-browser.yml",
    "text-inference": ".github/workflows/specimen-studio-desktop.yml",
    "image-inference": ".github/workflows/specimen-studio-image-model.yml",
    "ocr": ".github/workflows/specimen-studio-ocr.yml",
    "accelerator": ".github/workflows/specimen-studio-accelerator.yml",
    "macos-signing": ".github/workflows/specimen-studio-macos-package.yml",
}
RECEIPTS = {
    "desktop": ("package-qualification.json",),
    "browser": ("browser-checks", "accessibility-checks"),
    "text-inference": ("run-receipt.json",),
    "image-inference": ("run-receipt.json",),
    "ocr": ("run-receipt.json",),
    "accelerator": ("run-receipt.json",),
    "macos-signing": ("macos-package-verification.json", "-notarized.json"),
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


def find_payload(root: Path, recorded_path: str, recorded_sha256: str | None = None) -> Path:
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
    if recorded_sha256 is not None and len(matches) > 1:
        digest_matches = [path for path in matches if digest(path) == recorded_sha256]
        if len(digest_matches) == 1:
            return digest_matches[0]
        if not digest_matches:
            return matches[0]
        # Artifact uploads can retain both raw and post-packaging copies when
        # linuxdeploy leaves a payload byte-identical. Every candidate has
        # independently matched the receipt digest, so choose deterministically.
        return sorted(digest_matches)[0]
    if len(matches) != 1:
        raise EvidenceError(
            f"recorded artifact {recorded_path!r} has {len(matches)} matching payloads"
        )
    return matches[0]


def artifact_record(root: Path, path: str, recorded: str) -> dict[str, str]:
    if not SHA256.fullmatch(recorded):
        raise EvidenceError(f"recorded artifact {path!r} has an invalid SHA-256")
    payload = find_payload(root, path, recorded)
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
            strict = kind == "macos-signing" and key in ("output", "finalInstaller")
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


def valid_file_record(value: Any) -> bool:
    return isinstance(value, dict) and isinstance(value.get("path"), str) and bool(value["path"]) and SHA256.fullmatch(str(value.get("sha256", ""))) is not None


def validate_linux_stage(root: Path, package_path: Path, package: dict[str, Any],
                         source_sha: str, run_id: str, receipt_name: str,
                         installer_suffix: str, result_key: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    relative = package_path.resolve().relative_to(root.resolve())
    scope = root if len(relative.parts) == 1 else root / relative.parts[0]
    candidates = list(scope.rglob(receipt_name))
    require(len(candidates) == 1,
            f"ubuntu-24.04 evidence must contain exactly one {receipt_name}")
    stage_path = candidates[0]
    stage = load_json(stage_path, "Linux AppDir qualification receipt")
    require(isinstance(stage, dict) and stage.get("schema") == 1,
            "Linux AppDir qualification receipt is malformed")
    if installer_suffix == ".deb":
        require(stage.get("stageKind") == "deb",
                "Linux Debian stage receipt kind differs")
    require(stage.get("sourceSha") == source_sha and str(stage.get("workflowRunId")) == run_id,
            "Linux AppDir qualification source or run linkage differs")
    base = stage.get("baseQualification")
    require(isinstance(base, dict) and base.get("receiptSha256") == digest(package_path),
            "Linux AppDir base qualification receipt digest differs")
    raw_records = [package["application"], *package["helpers"]]
    raw = {str(item.get("name") or Path(item["path"]).name): item["sha256"] for item in raw_records}
    require(base.get("payloadSha256") == raw,
            "Linux AppDir raw-qualified payload linkage differs")
    installers = [item for item in package["installers"]
                  if Path(item["path"]).suffix == installer_suffix]
    require(len(installers) == 1 and stage.get("installer") == installers[0],
            f"Linux {result_key} installer linkage differs")
    artifacts = [artifact_record(scope, installers[0]["path"], installers[0]["sha256"])]
    payload = stage.get("payload")
    require(isinstance(payload, list) and len(payload) == len(raw),
            "Linux AppDir staged payload evidence is incomplete")
    observed = set()
    for item in payload:
        require(isinstance(item, dict) and item.get("name") in raw and item["name"] not in observed,
                "Linux AppDir staged payload identity is malformed or duplicated")
        observed.add(item["name"])
        require(item.get("rawQualifiedSha256") == raw[item["name"]],
                f"Linux AppDir raw-qualified hash differs for {item['name']}")
        require(valid_file_record({"path": item.get("retainedPath"), "sha256": item.get("sha256")}),
                f"Linux AppDir retained payload record is malformed for {item['name']}")
        record = artifact_record(scope, item["retainedPath"], item["sha256"])
        record["identity"] = item["name"]
        artifacts.append(record)
    require(observed == set(raw), "Linux AppDir staged payload identities are incomplete")
    artifacts.append({
        "identity": stage_path.name,
        "path": str(stage_path.relative_to(root)),
        "recordedSha256": None,
        "actualSha256": digest(stage_path),
        "verification": "receipt-identity-only",
    })
    return {result_key: "passed"}, artifacts


def validate_accessibility_widths(widths: Any) -> list[int]:
    require(isinstance(widths, dict), "browser accessibility widths are malformed")
    normalized = {str(key): value for key, value in widths.items()}
    required_states = {
        "studio", "open-provenance-dossier", "cycle-trace-completed",
        "disabled-contributor-feedback", "zoomed-keyboard-scrollable-topology",
        "specification-reader", "live-engine", "spec-model", "spec-diagrams",
        "store-explorer", "network-layers", "network-topology", "lab-browser",
    }
    required_targets = {
        "Network", "Cycle trace", "Zoom out topology", "Fit topology to canvas",
        "Zoom in topology", "Run dossier", "Right", "Wrong",
    }
    for width in (390, 768, 1440):
        record = normalized.get(str(width))
        require(isinstance(record, dict), f"browser accessibility width {width} is missing or malformed")
        audits = record.get("audits")
        require(isinstance(audits, list) and bool(audits), f"browser accessibility width {width} has no audits")
        states = set()
        for audit in audits:
            require(isinstance(audit, dict) and isinstance(audit.get("state"), str), f"browser accessibility width {width} has a malformed audit")
            require(all(audit.get(key) == 0 and not isinstance(audit.get(key), bool) for key in ("seriousOrCriticalViolations", "unnamedControls", "orphanedControls")), f"browser accessibility width {width} contains violations or unlabeled controls")
            states.add(audit["state"])
        expected_states = required_states | ({"mobile-drawer"} if width == 390 else set())
        require(expected_states <= states, f"browser accessibility width {width} is missing required audit states")
        targets = record.get("touchTargets")
        require(isinstance(targets, list) and required_targets <= set(targets), f"browser accessibility width {width} has incomplete touch-target evidence")
        require(record.get("keyboardContainment") == "passed", f"browser accessibility width {width} keyboard containment did not pass")
        if width == 1440:
            require(record.get("traceOutcomes") == ["Processing", "Cancelled", "Failed", "Completed"], "browser accessibility trace outcomes are incomplete")
    return [390, 768, 1440]


def validate_domain(kind: str, receipt: dict[str, Any], source_sha: str, run_id: str, package_qualification_run_id: str | None, receipt_name: str) -> dict[str, Any]:
    receipt_sha = receipt.get("headSha", receipt.get("sourceSha"))
    require(receipt_sha == source_sha, f"{kind} receipt must identify the frozen source SHA")
    workflow_run = receipt.get("workflowRunId")
    if kind not in ("macos-signing", "browser"):
        require(str(workflow_run) == run_id, f"{kind} receipt run identity differs")
    if kind == "browser":
        require(receipt.get("passed") is True and isinstance(receipt.get("engine"), str), "browser receipt does not record successful browser checks")
        if receipt_name.startswith("accessibility-checks-"):
            require(receipt.get("errors") == [], "browser accessibility receipt contains errors")
            observed_widths = validate_accessibility_widths(receipt.get("widths"))
            return {"engine": receipt["engine"], "accessibility": "passed", "widths": observed_widths}
        return {"engine": receipt["engine"], "functional": "passed"}
    require(receipt.get("schema") == 1, f"{kind} receipt is malformed")
    if kind == "desktop":
        require(valid_file_record(receipt.get("application")), "desktop receipt has no application identity")
        require(isinstance(receipt.get("helpers"), list) and bool(receipt["helpers"]) and all(valid_file_record(item) and isinstance(item.get("name"), str) and bool(item["name"]) for item in receipt["helpers"]), "desktop receipt has no helper identities")
        require(isinstance(receipt.get("installers"), list) and bool(receipt["installers"]), "desktop receipt has no installer identity")
        require(all(valid_file_record(item) for item in receipt["installers"]), "desktop receipt installer identity is malformed")
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
        require(valid_file_record(receipt.get("input")), "macos-signing input identity is missing or malformed")
        require(valid_file_record(receipt.get("output")), "macos-signing output identity is missing or malformed")
        require(receipt.get("finalInstallerPayloadVerification") == "passed", "macos-signing final payload verification failed")
        notarization = receipt.get("notarization", {})
        require(notarization.get("status") == "Accepted", "macos-signing notarization was not accepted")
        require(isinstance(receipt.get("signingIdentityFingerprint"), str) and bool(receipt["signingIdentityFingerprint"]), "macos-signing identity is missing")
        signed_files = receipt.get("signedMachOFiles")
        signed_hashes = receipt.get("signedPayloadSha256")
        require(isinstance(signed_files, list) and bool(signed_files) and all(isinstance(path, str) and bool(path) for path in signed_files), "macos-signing has no signed payload identities")
        require(isinstance(signed_hashes, dict) and bool(signed_hashes) and all(SHA256.fullmatch(str(value)) is not None for value in signed_hashes.values()), "macos-signing signed payload hashes are missing")
        stapler = receipt.get("stapler")
        gatekeeper = receipt.get("gatekeeper")
        require(isinstance(stapler, dict) and all(isinstance(stapler.get(key), str) and bool(stapler[key].strip()) for key in ("staple", "validate")), "macos-signing stapler evidence is missing")
        require(isinstance(gatekeeper, dict) and all(isinstance(gatekeeper.get(key), str) and bool(gatekeeper[key].strip()) for key in ("application", "dmg")), "macos-signing Gatekeeper evidence is missing")
        return {"notarization": "Accepted", "finalInstallerPayloadVerification": "passed", "hardenedRuntime": True, "secureTimestamps": True, "stapler": receipt.get("stapler"), "gatekeeper": receipt.get("gatekeeper")}
    raise EvidenceError(f"unsupported qualification kind: {kind!r}")


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
        try:
            receipt = load_json(path, f"{kind} receipt")
            if not isinstance(receipt, dict):
                raise EvidenceError(f"{kind} receipt is malformed")
            domain = validate_domain(kind, receipt, source_sha, run_id, package_qualification_run_id, path.name)
            receipt_records = receipt_artifacts(root, path, receipt, kind)
            if kind == "desktop" and domain.get("runner", {}).get("label") == "ubuntu-24.04":
                stage_artifacts = []
                for receipt_name, suffix, result_key in (
                    ("linux-appdir-qualification.json", ".AppImage", "appDirPayloadVerification"),
                    ("linux-deb-qualification.json", ".deb", "debPayloadVerification"),
                ):
                    stage_domain, artifacts = validate_linux_stage(
                        root, path, receipt, source_sha, run_id,
                        receipt_name, suffix, result_key)
                    domain.update(stage_domain)
                    stage_artifacts.extend(artifacts)
            domains.append(domain)
            result["artifacts"].extend(receipt_records)
            if kind == "desktop" and domain.get("runner", {}).get("label") == "ubuntu-24.04":
                result["artifacts"].extend(stage_artifacts)
            result.setdefault("receipts", []).append(str(path.relative_to(root)))
        except (EvidenceError, OSError, KeyError, TypeError) as error:
            message = f"{path.relative_to(root)}: {error}"
            incomplete_accessibility = kind == "browser" and any(
                phrase in str(error)
                for phrase in (
                    "is missing or malformed", "has no audits",
                    "is missing required audit states",
                    "has incomplete touch-target evidence",
                    "trace outcomes are incomplete",
                )
            )
            if incomplete_accessibility:
                result.setdefault("evidenceGaps", []).append(message)
            else:
                print(f"error: {message}", file=sys.stderr)
                result.setdefault("errors", []).append(message)
    result["domain"] = domains[0] if len(domains) == 1 else {"receipts": domains}
    expected_matrix = {
        "desktop": {"macos-14"},
        "browser": {"chrome", "firefox", "webkit"},
    }.get(kind)
    if expected_matrix:
        job_conclusions = {}
        for required in expected_matrix:
            match = next((job for job in jobs_value if isinstance(job, dict) and f"({required})" in str(job.get("name", ""))), None)
            job_conclusions[required] = match.get("conclusion") if match else None
        failed_conclusions = {"failure", "cancelled", "timed_out", "action_required", "startup_failure", "stale"}
        if any(value in failed_conclusions for value in job_conclusions.values()):
            result["status"] = "failed"
            result["blocker"] = "required matrix job failed"
        elif any(value != "success" for value in job_conclusions.values()):
            result["status"] = "unverified"
            result["blocker"] = "required matrix job is missing, skipped, or incomplete"
        if kind == "desktop":
            observed = {domain.get("runner", {}).get("label") for domain in domains}
            missing = sorted(expected_matrix - observed)
        else:
            functional = {domain.get("engine") for domain in domains if domain.get("functional") == "passed"}
            accessibility = {domain.get("engine") for domain in domains if domain.get("accessibility") == "passed"}
            missing = sorted((expected_matrix - functional) | (expected_matrix - accessibility))
        if missing and result["status"] != "failed":
            result["status"] = "unverified"
            result["blocker"] = f"required {kind} evidence is missing: {', '.join(missing)}"
    if result.get("errors"):
        result["status"] = "failed"
    if result["status"] == "passed" and any(item.get("verification") == "payload-not-uploaded" for item in result["artifacts"]):
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
        for gap in run.get("evidenceGaps", []):
            lines.append(f"  - Evidence gap: {gap}")
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
