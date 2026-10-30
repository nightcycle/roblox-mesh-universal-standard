from pathlib import Path
import tempfile
import unittest

from release import validate


class ReleaseValidationTests(unittest.TestCase):
    def test_validates_separate_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "mesh-runtime").mkdir()
            for path in (root / "Cargo.toml", root / "mesh-runtime/wally.toml"):
                path.write_text('[package]\nversion = "9.8.7"\n', encoding="utf-8")
            self.assertEqual(validate("v9.8.7", folder), "9.8.7")
            self.assertFalse((root / "scripts/wally_auth.py").exists())

    def test_rejects_mismatched_versions_in_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "mesh-runtime").mkdir()
            (root / "Cargo.toml").write_text('[package]\nversion = "9.8.7"\n', encoding="utf-8")
            (root / "mesh-runtime/wally.toml").write_text('[package]\nversion = "9.8.6"\n', encoding="utf-8")
            with self.assertRaises(SystemExit):
                validate("v9.8.7", root)

    def test_rejects_tag_that_does_not_match_tagged_checkout(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "mesh-runtime").mkdir()
            for path in (root / "Cargo.toml", root / "mesh-runtime/wally.toml"):
                path.write_text('[package]\nversion = "9.8.7"\n', encoding="utf-8")
            with self.assertRaises(SystemExit):
                validate("v9.8.6", root)


if __name__ == "__main__":
    unittest.main()
