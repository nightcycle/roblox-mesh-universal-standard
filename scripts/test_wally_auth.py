import os
from pathlib import Path
import tempfile
import tomllib
import unittest

from wally_auth import PUBLIC_API, auth_config, write_auth


class WallyAuthTests(unittest.TestCase):
    def test_raw_token_becomes_wally_config(self):
        token = "0123456789abcdef" * 2 + "01234567"
        result = tomllib.loads(auth_config("\n" + token + "\r\n"))
        self.assertEqual(result, {"tokens": {PUBLIC_API: token}})

    def test_full_config_preserves_tokens(self):
        config = '[tokens]\n"https://api.wally.run/" = "test-token"\n"https://private.example/" = "other-token"\n'
        self.assertEqual(tomllib.loads(auth_config(config)), tomllib.loads(config))

    def test_invalid_input_does_not_disclose_secrets(self):
        secret_marker = "do-not-log-this-token"
        invalid = [
            "", " \n", "[tokens", '[tokens]\nkey = "' + secret_marker,
            'token = "' + secret_marker + '"', "[tokens]",
            '[tokens]\n"https://api.wally.run/" = 42',
            '[tokens]\n"https://api.wally.run/" = ""',
            '[tokens]\n"https://wrong.example/" = "' + secret_marker + '"',
        ]
        for value in invalid:
            with self.subTest(value=value):
                with self.assertRaises(ValueError) as result:
                    auth_config(value)
                self.assertNotIn(secret_marker, str(result.exception))

    def test_invalid_input_does_not_overwrite_existing_file(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "auth.toml"
            path.write_text("existing config", encoding="utf-8")
            with self.assertRaises(ValueError):
                write_auth("[tokens", path)
            self.assertEqual(path.read_text(encoding="utf-8"), "existing config")

    def test_writes_only_to_requested_location(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / ".wally" / "auth.toml"
            write_auth("test-token", path)
            self.assertEqual(tomllib.loads(path.read_text())["tokens"][PUBLIC_API], "test-token")
            if os.name != "nt":
                self.assertEqual(path.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
