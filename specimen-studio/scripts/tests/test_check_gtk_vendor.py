"""Negative controls for source, attribution and resolved dependency identity."""
import copy
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "check_gtk_vendor.py"
spec = importlib.util.spec_from_file_location("gtk_vendor", SCRIPT)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class SourceControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.studio = Path(self.temp.name)
        shutil.copytree(checker.STUDIO / "native/vendor", self.studio / "native/vendor")
        shutil.copyfile(checker.STUDIO / "Cargo.toml", self.studio / "Cargo.toml")
        shutil.copyfile(checker.STUDIO / "Cargo.lock", self.studio / "Cargo.lock")

    def test_pristine_patched_sources_pass(self):
        checker.verify_sources(self.studio)

    def test_license_metadata_version_and_source_drift_fail(self):
        vendor = self.studio / "native/vendor/glib-macros-0.18.5"
        for name in ["LICENSE", ".cargo_vcs_info.json", "Cargo.toml", "src/enum_derive.rs"]:
            with self.subTest(file=name):
                path = vendor / name
                original = path.read_bytes()
                path.write_bytes(original + b"\n# unexpected change\n")
                with self.assertRaisesRegex(ValueError, "source/license/metadata drift"):
                    checker.verify_sources(self.studio)
                path.write_bytes(original)

    def test_undeclared_source_and_changed_manifest_fail(self):
        path = self.studio / "native/vendor/glib-0.18.5/unexpected.rs"
        path.write_text("unexpected source")
        with self.assertRaisesRegex(ValueError, "source/license/metadata drift"):
            checker.verify_sources(self.studio)
        path.unlink()
        path = self.studio / "native/vendor/source-manifest.json"
        path.write_text(path.read_text() + "\n")
        with self.assertRaisesRegex(ValueError, "source manifest pin changed"):
            checker.verify_sources(self.studio)

    def test_replacement_lock_checksum_drift_is_refused(self):
        for relative in ["Cargo.lock", "native/vendor/Cargo.lock"]:
            with self.subTest(lock=relative):
                path = self.studio / relative
                original = path.read_text()
                path.write_text(original.replace(checker.PINS["proc-macro-error3-3.1.1"], "0" * 64))
                with self.assertRaisesRegex(ValueError, "checksum"):
                    checker.verify_sources(self.studio)
                path.write_text(original)

    def test_vendor_base_symlink_is_refused(self):
        vendor = self.studio / "native/vendor"
        real = vendor.with_name("vendor-real")
        vendor.rename(real)
        vendor.symlink_to(real, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            checker.verify_sources(self.studio)

    def test_crate_root_symlink_is_refused(self):
        crate = self.studio / "native/vendor/glib-0.18.5"
        real = crate.with_name("glib-real")
        crate.rename(real)
        crate.symlink_to(real, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            checker.verify_sources(self.studio)

    def test_sibling_patch_and_missing_optimized_profile_fail(self):
        path = self.studio / "Cargo.toml"
        original = path.read_text()
        path.write_text(original.replace('path = "native/vendor/glib-0.18.5"', 'path = "../abbey/vendor/glib-0.18.5"'))
        with self.assertRaisesRegex(ValueError, "nonlocal glib patch"):
            checker.verify_sources(self.studio)
        path.write_text(original.replace('[profile.dev.package."glib@0.18.5"]\nopt-level = 3', '[profile.dev.package."glib@0.18.5"]\nopt-level = 0'))
        with self.assertRaisesRegex(ValueError, "optimized GLib dev profile missing"):
            checker.verify_sources(self.studio)


class ResolutionControls(unittest.TestCase):
    def setUp(self):
        packages = []
        for name, version in checker.LOCAL.items():
            packages.append({"name": name, "version": version, "id": name,
                             "source": None, "manifest_path": str(checker.STUDIO / f"native/vendor/{name}-{version}/Cargo.toml"),
                             "dependencies": [] if name == "glib" else [{"name": "proc-macro-error3", "rename": None,
                                 "req": "=3.1.1", "uses_default_features": False, "features": ["syn2-error"]}]})
        for name in ["proc-macro-error3", "proc-macro-error-attr3"]:
            packages.append({"name": name, "version": "3.1.1", "id": name,
                             "source": "registry+https://github.com/rust-lang/crates.io-index"})
        self.graph = {"workspace_root": str(checker.STUDIO), "workspace_members": [], "packages": packages, "resolve": {"nodes": [{"id": "proc-macro-error3", "features": ["syn2-error"]}]}}

    def test_expected_resolution_passes(self):
        checker.verify_graph(self.graph)

    def test_consumer_cannot_absorb_vendor_workspace_members(self):
        self.graph["workspace_members"].append("glib")
        with self.assertRaisesRegex(ValueError, "workspace boundary"):
            checker.verify_graph(self.graph)

    def test_registry_substitution_old_error_api_and_syn3_features_fail(self):
        for mode in ["registry", "obsolete", "features", "version", "alias"]:
            graph = copy.deepcopy(self.graph)
            if mode == "registry":
                graph["packages"][0]["source"] = "registry+https://github.com/rust-lang/crates.io-index"
            elif mode == "obsolete":
                graph["packages"].append({"name": "proc-macro-error", "version": "1.0.4"})
            elif mode == "features":
                graph["resolve"]["nodes"][0]["features"].append("syn3-error")
            elif mode == "version":
                graph["packages"][-1]["version"] = "3.1.2"
            else:
                graph["packages"][1]["dependencies"][0]["rename"] = "proc-macro-error"
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                checker.verify_graph(graph)


if __name__ == "__main__":
    unittest.main()
