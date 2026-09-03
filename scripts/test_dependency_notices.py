import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from dependency_notices import verify_declared_license


class DeclarationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        manifest = self.root / "Cargo.toml"
        manifest.write_text('[package]\nlicense = "MIT OR Apache-2.0"\n')
        self.package = {"name": "example", "license": "MIT OR Apache-2.0", "manifest_path": str(manifest)}
        self.review = {"declared": self.package["license"], "selected": "MIT",
                       "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest()}
        (self.root / "MIT.txt").write_text("test fixture only")
        (self.root / "index.json").write_text(json.dumps({"MIT": {
            "url": "https://example.invalid/MIT", "sha256": hashlib.sha256(b"test fixture only").hexdigest()}}))

    def test_valid_review(self):
        self.assertEqual(verify_declared_license(self.package, self.review, self.root)["selected"], "MIT")

    def test_no_automatic_fallback(self):
        self.assertIsNone(verify_declared_license(self.package, None, self.root))

    def test_manifest_tampering(self):
        Path(self.package["manifest_path"]).write_text("changed")
        with self.assertRaises(ValueError):
            verify_declared_license(self.package, self.review, self.root)

    def test_expression_change(self):
        self.package["license"] = "GPL-3.0-only"
        with self.assertRaises(ValueError):
            verify_declared_license(self.package, self.review, self.root)

    def test_undeclared_choice(self):
        self.review["selected"] = "BSD-3-Clause"
        with self.assertRaises(ValueError):
            verify_declared_license(self.package, self.review, self.root)

    def test_license_text_tampering(self):
        (self.root / "MIT.txt").write_text("changed")
        with self.assertRaises(ValueError):
            verify_declared_license(self.package, self.review, self.root)


if __name__ == "__main__":
    unittest.main()
