from pathlib import Path
import tempfile
import unittest

from release import validate


def write_versions(root, rust="9.8.7", wally="9.8.7", locked="9.8.7"):
    (root / "mesh-runtime").mkdir()
    (root / "Cargo.toml").write_text(f'[package]\nversion = "{rust}"\n', encoding="utf-8")
    (root / "mesh-runtime/wally.toml").write_text(f'[package]\nversion = "{wally}"\n', encoding="utf-8")
    (root / "Cargo.lock").write_text(
        f'[[package]]\nname = "roblox-mesh-universal-standard"\nversion = "{locked}"\n',
        encoding="utf-8",
    )


class ReleaseValidationTests(unittest.TestCase):
    def test_validates_separate_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            write_versions(root)
            self.assertEqual(validate("v9.8.7", folder), "9.8.7")
            self.assertFalse((root / "scripts/wally_auth.py").exists())

    def test_rejects_mismatched_versions_in_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            write_versions(root, wally="9.8.6")
            with self.assertRaises(SystemExit):
                validate("v9.8.7", root)

    def test_rejects_tag_that_does_not_match_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            write_versions(root)
            with self.assertRaises(SystemExit):
                validate("v9.8.6", root)

    def test_rejects_stale_lockfile(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            write_versions(root, locked="9.8.6")
            with self.assertRaisesRegex(SystemExit, "Cargo.lock"):
                validate("v9.8.7", root)

    def test_rejects_lockfile_without_project_entry(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            write_versions(root)
            (root / "Cargo.lock").write_text('[[package]]\nname = "other"\nversion = "9.8.7"\n')
            with self.assertRaisesRegex(SystemExit, "Cargo.lock"):
                validate("v9.8.7", root)


if __name__ == "__main__":
    unittest.main()
