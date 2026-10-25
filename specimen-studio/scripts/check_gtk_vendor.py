#!/usr/bin/env python3
"""Verify Studio's attributed GTK compatibility sources and Cargo resolution.

No registry cache is needed for the normal gate. --archives adds a comparison
against independently hash-pinned original archives without writing to them.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import re

STUDIO = Path(__file__).resolve().parents[1]
PINS = {
    "glib-0.18.5": "233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5",
    "glib-macros-0.18.5": "0bb0228f477c0900c880fd78c8759b95c7636dbd7842707f49e132378aa2acdc",
    "gtk3-macros-0.18.2": "52ff3c5b21f14f0736fed6dcfc0bfb4225ebf5725f3c0209edeec181e4d73e9d",
    "proc-macro-error3-3.1.1": "8f0d4471b3436c22106b21913b1dda531558918ae9b7ec55d58aa84b43552233",
    "proc-macro-error-attr3-3.1.1": "9e564d14133360e1ae169ffde5da25881b5fa47261665b8e5713c212c27799da",
}
MANIFEST_SHA256 = "f0da7ea3319364955f713f3b405702755667b6762343205ecee3a6837d2b47b4"
LOCAL = {"glib": "0.18.5", "glib-macros": "0.18.5", "gtk3-macros": "0.18.2"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def toml_section(text, name):
    """Read the fixed reviewed sections; Cargo validates complete TOML in graph mode."""
    lines = text.splitlines()
    heading = f"[{name}]"
    require(lines.count(heading) == 1, f"missing/duplicate TOML section {heading}")
    body = []
    for line in lines[lines.index(heading)+1:]:
        if line.startswith("["):
            break
        if not line.lstrip().startswith("#"):
            body.append(line)
    return re.sub(r"\s+", "", "".join(body))


def verify_lock(path):
    packages = []
    for block in path.read_text().split("[[package]]")[1:]:
        packages.append(dict(re.findall(r'^([a-z_-]+) = "([^"\n]*)"$', block, re.MULTILINE)))
    obsolete = {"proc-macro-error", "proc-macro-error2", "proc-macro-error-attr", "proc-macro-error-attr2"}
    require(not any(p.get("name") in obsolete for p in packages), f"obsolete macro lock graph: {path}")
    for name, version in LOCAL.items():
        matches = [p for p in packages if p.get("name") == name and p.get("version") == version]
        require(len(matches) == 1 and not {"source", "checksum"} & matches[0].keys(),
                f"local source identity changed in lock: {name}")
    for name in ("proc-macro-error3", "proc-macro-error-attr3"):
        matches = [p for p in packages if p.get("name") == name]
        require(len(matches) == 1 and matches[0].get("version") == "3.1.1", f"replacement lock version: {name}")
        require(matches[0].get("source") == "registry+https://github.com/rust-lang/crates.io-index" and
                matches[0].get("checksum") == PINS[f"{name}-3.1.1"], f"replacement lock source/checksum: {name}")


def verify_sources(studio=STUDIO, archives=None):
    vendor = studio / "native/vendor"
    require(not any(p.is_symlink() for p in (studio, studio / "native", vendor)), "symlink in vendor base")
    manifest_bytes = (vendor / "source-manifest.json").read_bytes()
    require(sha(manifest_bytes) == MANIFEST_SHA256, "source manifest pin changed; review provenance")
    manifest = json.loads(manifest_bytes)
    require(len(manifest["packages"]) == len(PINS), "package inventory changed")
    for package in manifest["packages"]:
        identity = f'{package["name"]}-{package["version"]}'
        require(package["archive_sha256"] == PINS[identity], f"archive pin changed: {identity}")
        if "patched_files" in package:
            folder = vendor / identity
            require(not folder.is_symlink(), f"symlink crate root: {identity}")
            actual = {str(p.relative_to(folder)): sha(p.read_bytes())
                      for p in folder.rglob("*") if p.is_file()}
            require(not any(p.is_symlink() for p in folder.rglob("*")), f"symlink in {identity}")
            require(actual == package["patched_files"], f"patched source/license/metadata drift: {identity}")
            require(actual.keys() == package["original_files"].keys(), f"original inventory changed: {identity}")
            changed = sorted(k for k in actual if actual[k] != package["original_files"][k])
            require(changed == sorted(package["changed_files"]), f"unexpected source changes: {identity}")
            if package["name"] != "glib":
                body = toml_section((folder / "Cargo.toml").read_text(), "dependencies.proc-macro-error3")
                require(body == 'version="=3.1.1"default-features=falsefeatures=["syn2-error"]',
                        f"wrong macro feature/API contract: {identity}")
        if archives:
            archive = archives / (identity + ".crate")
            require(sha(archive.read_bytes()) == PINS[identity], f"original archive drift: {identity}")
            with tarfile.open(archive) as source:
                files = {p.name[len(identity)+1:]: source.extractfile(p).read()
                         for p in source.getmembers() if p.isfile()}
            require({k: sha(v) for k, v in files.items()} == package["original_files"],
                    f"original file inventory mismatch: {identity}")
            if "patched_files" in package:
                for relative, original in files.items():
                    expected = original
                    if identity == "glib-0.18.5" and relative == "src/variant_iter.rs":
                        expected = original.replace(b"let p: *mut libc::c_char", b"let mut p: *mut libc::c_char").replace(b"                &p,", b"                &mut p,")
                    elif identity != "glib-0.18.5":
                        if relative.startswith("src/") and relative.endswith(".rs"):
                            expected = original.replace(b"proc_macro_error::", b"proc_macro_error3::")
                        elif relative == "Cargo.toml":
                            expected = original.replace(b'[dependencies.proc-macro-error]\nversion = "1.0"', b'''# Local compatibility patch: maintained error API, preserving syn 2.
[dependencies.proc-macro-error3]
version = "=3.1.1"
default-features = false
features = ["syn2-error"]''')
                        elif relative == "Cargo.toml.orig":
                            expected = original.replace(b'proc-macro-error = "1.0"', b'''# Local compatibility patch: maintained error API, preserving syn 2.
proc-macro-error3 = { version = "=3.1.1", default-features = false, features = ["syn2-error"] }''')
                    require((vendor / identity / relative).read_bytes() == expected,
                            f"change outside reviewed transformation: {identity}/{relative}")
            else:
                declared = files["Cargo.toml"].decode()
                if package["name"] == "proc-macro-error3":
                    require(toml_section(declared, "dependencies.proc-macro-error-attr3") == 'version="=3.1.1"', "attr3 exact pin")
                    require('syn2-error=["dep:syn2"]' in toml_section(declared, "features"), "published syn2 feature changed")
                else:
                    require(b"::proc_macro_error3::entry_point" in files["src/lib.rs"], "attr3 entry point mismatch")
    for root in (studio, vendor):
        verify_lock(root / "Cargo.lock")
        workspace = (root / "Cargo.toml").read_text()
        patches = toml_section(workspace, "patch.crates-io")
        prefix = "native/vendor/" if root == studio else ""
        for name, version in LOCAL.items():
            require(f'{name}={{path="{prefix}{name}-{version}"}}' in patches, f"nonlocal {name} patch at {root}")
        require(toml_section(workspace, 'profile.dev.package."glib@0.18.5"') == 'opt-level=3',
                f"optimized GLib dev profile missing at {root}")
    return manifest


def verify_graph(graph, studio=STUDIO):
    vendor = studio / "native/vendor"
    packages = graph["packages"]
    local_ids = set()
    for name, version in LOCAL.items():
        matches = [p for p in packages if p["name"] == name and p["version"] == version]
        require(len(matches) == 1, f"expected exactly one {name}@{version}")
        package = matches[0]
        local_ids.add(package["id"])
        require(package["source"] is None and Path(package["manifest_path"]).resolve() ==
                (vendor / f"{name}-{version}/Cargo.toml").resolve(), f"registry/sibling source substitution: {name}")
    members = set(graph["workspace_members"])
    is_consumer = Path(graph["workspace_root"]).resolve() == studio.resolve()
    require(not (local_ids & members) if is_consumer else local_ids <= members,
            "consumer/vendor workspace boundary changed")
    require(not any(p["name"] in {"proc-macro-error", "proc-macro-error2", "proc-macro-error-attr", "proc-macro-error-attr2"} for p in packages),
            "obsolete proc-macro-error graph remains")
    for name in ("proc-macro-error3", "proc-macro-error-attr3"):
        matches = [p for p in packages if p["name"] == name]
        require(len(matches) == 1 and matches[0]["version"] == "3.1.1", f"replacement version drift: {name}")
        require(matches[0]["source"] == "registry+https://github.com/rust-lang/crates.io-index", f"replacement source drift: {name}")
    error3 = next(p for p in packages if p["name"] == "proc-macro-error3")
    node = next(n for n in graph["resolve"]["nodes"] if n["id"] == error3["id"])
    require("syn2-error" in node["features"] and not {"default", "syn3-error", "syn-error"} & set(node["features"]),
            f"replacement feature drift: {node['features']}")
    for parent in ("glib-macros", "gtk3-macros"):
        package = next(p for p in packages if p["name"] == parent and p["version"] == LOCAL[parent])
        dependency = [d for d in package["dependencies"] if d["name"] == "proc-macro-error3"]
        require(len(dependency) == 1 and dependency[0]["rename"] is None and dependency[0]["req"] == "=3.1.1"
                and not dependency[0]["uses_default_features"] and dependency[0]["features"] == ["syn2-error"],
                f"macro parent dependency mismatch: {parent}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archives", type=Path, help="read-only original Cargo archive directory")
    parser.add_argument("--metadata", action="store_true", help="verify locked Cargo metadata from both roots")
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    try:
        verify_sources(archives=args.archives)
        if args.metadata:
            for root in (STUDIO, STUDIO / "native/vendor"):
                result = subprocess.run([args.cargo, "metadata", "--locked", "--offline", "--all-features", "--format-version", "1"],
                                        cwd=root, stdin=subprocess.DEVNULL, capture_output=True, text=True, check=True)
                if result.stderr:
                    print(result.stderr, file=sys.stderr, end="")
                verify_graph(json.loads(result.stdout))
        print("GTK vendor source/license/identity checks passed" + ("; both locked resolution graphs passed" if args.metadata else ""))
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"GTK vendor verification failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError):
            print(error.stderr, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
