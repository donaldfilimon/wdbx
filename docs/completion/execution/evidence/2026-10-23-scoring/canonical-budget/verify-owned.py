"""Inspect only the canonical-budget implementation and captured source freeze."""
from pathlib import Path
import difflib
import hashlib
import json
import subprocess

base = Path(__file__).resolve().parent
root = base.parents[5]
owned = [
    "specimen-studio/native/specimen-kernel/src/lib.rs",
    "specimen-studio/native/specimen-kernel/src/logical_charge_v1.rs",
    "specimen-studio/native/specimen-kernel/src/scoring_v3/budget.rs",
    "specimen-studio/native/specimen-kernel/src/orchestration_v3/budget.rs",
    "specimen-studio/native/specimen-kernel/tests/scoring_orchestration_budget_v1.rs",
]
for name in owned:
    content = (root / name).read_text()
    assert all(line.rstrip() == line for line in content.splitlines()), name
    assert content.endswith("\n") and not content.endswith("\n\n"), name
    assert "std::mem::size_of" not in content, name
    assert "<<<<<<<" not in content, name
print("Direct whitespace/conflict/layout-policy checks: five owned files clean")

patch = []
for name in owned:
    previous = base / "before" / name
    before = previous.read_text() if previous.exists() else ""
    if previous.exists():
        parent_before = root / ".superpowers/sdd/2026-10-20-wdbx-full-completion/baseline-canonical-budget" / name
        assert previous.read_bytes() == parent_before.read_bytes(), name
    patch.extend(difflib.unified_diff(before.splitlines(keepends=True),
                 (root / name).read_text().splitlines(keepends=True),
                 fromfile="before/" + name, tofile=name))
(base / "owned-review.diff").write_text("".join(patch))
print("Three original-file snapshots match root's pre-edit baseline")

paths = set(root / p for p in owned)
for area in ["src/scoring_v3", "src/orchestration_v3"]:
    paths.update((root / "specimen-studio/native/specimen-kernel" / area).rglob("*.rs"))
for pattern in ["scoring_v3*.rs", "orchestration_v3*.rs"]:
    paths.update((root / "specimen-studio/native/specimen-kernel/tests").glob(pattern))
paths.update((root / "specimen-studio/native/specimen-kernel/tests/orchestration_v3").rglob("*.rs"))
manifest = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}
(base / "source-freeze.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Source/API/test freeze: {len(manifest)} files")
for name in ["affected", "full-kernel", "clippy", "format-check"]:
    assert json.loads((base / f"{name}.command.json").read_text())["exit"] == 0, name
print("Affected/full kernel/Clippy/owned fmt command exit codes: all zero")
