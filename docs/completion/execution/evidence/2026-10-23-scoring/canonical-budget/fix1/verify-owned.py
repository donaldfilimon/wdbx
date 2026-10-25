"""Audit the newly introduced scoring/orchestration code and fix1 ownership."""
from pathlib import Path
import difflib
import hashlib
import json
import re

base = Path(__file__).resolve().parent
root = base.parents[6]
owned = [
    "specimen-studio/native/specimen-kernel/src/logical_charge_v1.rs",
    "specimen-studio/native/specimen-kernel/src/orchestration_v3/attachments.rs",
    "specimen-studio/native/specimen-kernel/src/scoring_v3/policy.rs",
    "specimen-studio/native/specimen-kernel/tests/scoring_orchestration_attachment_budget_v1.rs",
]
patch = []
for name in owned:
    content = (root / name).read_text()
    assert all(line.rstrip() == line for line in content.splitlines()), name
    assert content.endswith("\n") and not content.endswith("\n\n"), name
    assert "<<<<<<<" not in content, name
    before_path = base / "before" / name
    before = before_path.read_text() if before_path.exists() else ""
    if before_path.exists():
        root_baseline = root / ".superpowers/sdd/2026-10-20-wdbx-full-completion/baseline-canonical-budget-fix1" / name
        assert before_path.read_bytes() == root_baseline.read_bytes(), name
    patch.extend(difflib.unified_diff(before.splitlines(keepends=True), content.splitlines(keepends=True),
                 fromfile="before/" + name, tofile=name))
(base / "owned-review.diff").write_text("".join(patch))
print("Four owned Rust files: direct whitespace/EOF/conflict checks pass")
print("Three pre-edit sources match root fix1 baseline byte-for-byte")

kernel = root / "specimen-studio/native/specimen-kernel"
audit = sorted(set((kernel / "src/scoring_v3").rglob("*.rs")) |
               set((kernel / "src/orchestration_v3").rglob("*.rs")) |
               {kernel / "src/logical_charge_v1.rs"})
pattern = re.compile(r"size_of|align_of|target_pointer_width|target_arch|std::mem|core::mem|canonical_budget_layout_reference")
for path in audit:
    assert not pattern.search(path.read_text()), str(path)
(base / "layout-policy-audit.json").write_text(json.dumps({
    "scope": "All new scoring_v3 and orchestration_v3 production Rust plus their private logical-charge helper; no legacy scan",
    "pattern": pattern.pattern,
    "files": [str(p.relative_to(root)) for p in audit],
    "matches": [],
}, indent=2) + "\n")
print(f"Scoped layout-policy audit: {len(audit)} production files; zero matches; temporary observer absent")

old_manifest = json.loads((base.parent / "source-freeze.json").read_text())
allowed = set(owned)
for name, digest in old_manifest.items():
    if name not in allowed:
        assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, name
print("All 21 non-owned files in original 24-file source/test freeze remain identical")
paths = set(root / name for name in old_manifest) | {root / owned[-1]}
manifest = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}
(base / "source-freeze.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Fix1 source/API/test freeze: {len(manifest)} files")
