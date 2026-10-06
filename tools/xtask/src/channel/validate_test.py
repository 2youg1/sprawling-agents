"""Generated definitions are rewritten without decoding native payloads."""

import contextlib
import pathlib
import runpy
import sys
import tempfile
import unittest
from unittest import mock


class ArchiveValidation(unittest.TestCase):
    def test_native_payloads_survive_the_validation_entry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            channels = root / "generated"
            assets = root / "assets"
            local = root / "validation"
            for directory in (assets, local):
                directory.mkdir(parents=True)
            url = "https://example.invalid/releases/download/v1/archive.zip"
            definitions = ("aur/PKGBUILD", "aur/.SRCINFO")
            for relative in definitions:
                definition = channels / relative
                definition.parent.mkdir(parents=True, exist_ok=True)
                definition.write_text(url, encoding="utf-8")
            payloads = {
                "@sprawling/linux-x64/bin/sprawling": bytes.fromhex("7f454c46ff00"),
                "@sprawling/darwin-arm64/bin/sprawling": bytes.fromhex("cffaedfeff00"),
                "unrelated": url.encode("utf-8"),
                "package.json": url.encode("utf-8"),
            }
            for relative, payload in payloads.items():
                binary = channels / relative
                binary.parent.mkdir(parents=True, exist_ok=True)
                binary.write_bytes(payload)
            commands = []

            def run(arguments, **kwargs):
                self.assertEqual(arguments[0], "docker", "validation must only install AUR")
                self.assertTrue(kwargs["check"])
                commands.append(arguments)
                for relative, payload in payloads.items():
                    self.assertEqual((local / "channels" / relative).read_bytes(), payload)
                for relative in definitions:
                    text = (local / "channels" / relative).read_text(encoding="utf-8")
                    self.assertIn("http://127.0.0.1:", text)
                    self.assertNotIn("/releases/download/", text)

            script = pathlib.Path(__file__).with_name("validate.py")
            with (
                mock.patch.object(sys, "argv", [str(script), str(channels), str(assets)]),
                mock.patch("tempfile.TemporaryDirectory", return_value=contextlib.nullcontext(str(local))),
                mock.patch("subprocess.run", side_effect=run),
            ):
                try:
                    runpy.run_path(str(script), run_name="__main__")
                except UnicodeError as failure:
                    self.fail(f"validator decoded a native payload: {failure}")
            self.assertTrue(any(arguments[0] == "docker" for arguments in commands))
            for relative, payload in payloads.items():
                self.assertEqual((channels / relative).read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
