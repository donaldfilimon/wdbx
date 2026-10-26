"""Regression for the two independently reachable GTK 3 macro parents."""
import pathlib
import sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from check_gtk_vendor import toml_section
import unittest

STUDIO = pathlib.Path(__file__).resolve().parents[2]


class MacroSourceContract(unittest.TestCase):
    def test_both_parents_use_real_syn2_error3_dependency(self):
        for name in ("glib-macros-0.18.5", "gtk3-macros-0.18.2"):
            with self.subTest(parent=name):
                manifest = STUDIO / "native/vendor" / name / "Cargo.toml"
                self.assertTrue(manifest.exists(), f"missing local macro parent: {name}")
                dependencies = manifest.read_text()
                self.assertNotIn("[dependencies.proc-macro-error]", dependencies)
                self.assertNotIn("[dependencies.proc-macro-error2]", dependencies)
                self.assertEqual(toml_section(dependencies, "dependencies.proc-macro-error3"),
                                 'version="=3.1.1"default-features=falsefeatures=["syn2-error"]')

    def test_both_workspaces_patch_all_three_sources(self):
        for relative in ("Cargo.toml", "native/vendor/Cargo.toml"):
            with self.subTest(workspace=relative):
                path = STUDIO / relative
                self.assertTrue(path.exists(), f"missing workspace: {relative}")
                patches = toml_section(path.read_text(), "patch.crates-io")
                for name in ("glib", "glib-macros", "gtk3-macros"):
                    self.assertIn(name, patches)


if __name__ == "__main__":
    unittest.main()
