"""Offline failure checks; never accesses OpenRouter or a normal user profile."""
import copy
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("theme_eval", HERE / "theme-eval.py")
ev = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ev)


class EvalFailures(unittest.TestCase):
    def setUp(self):
        self.template = ev.read_json(HERE / "theme-eval-reference.json")

    def test_dock_and_hover_fail_even_with_readable_canvas(self):
        bad = copy.deepcopy(self.template)
        bad["theme"]["app"]["presetDock"]["buttonHover"] = "#ffffff"
        bad["theme"]["app"]["shell"]["accentHover"] = "#ffffff"
        result = ev.validate(bad, self.template)
        self.assertTrue(any("dock text / hover" in e for e in result["errors"]))
        self.assertTrue(any("white hover / accentHover" in e for e in result["errors"]))
        self.assertFalse(any("main text / canvas" in e for e in result["errors"]))

    def test_malformed_or_unsupported_model_output_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            ev.parse_json('{"version": 1, "version": 3}')
        bad = copy.deepcopy(self.template)
        bad["theme"]["terminal"]["ansiColors"].pop("brightCyan")
        self.assertTrue(ev.validate(bad, self.template)["errors"])
        bad = copy.deepcopy(self.template)
        bad["tabs"] = ["imaginary-v3"]
        self.assertTrue(ev.validate(bad, self.template)["errors"])

    def test_modified_theme_cannot_be_installed_after_native_validation(self):
        with tempfile.TemporaryDirectory(prefix="buttonscli-eval-tests-") as temp:
            run = Path(temp) / "run"
            (run / "themes").mkdir(parents=True)
            (run / "captures").mkdir()
            source = run / "themes" / "theme.json"
            ev.write_json(source, self.template)
            original = ev.digest(source)
            ev.write_json(run / "manifest.json", {"results": [{"accepted": True, "file": "theme.json", "sha256": original}]})
            (run / "report.md").write_text("Completed owned fixture", encoding="utf-8")
            ev.write_json(run / "captures" / "native-validation.json", {"passed": True, "files": [{"file": "theme.json", "sha256": original}]})
            source.write_text(source.read_text() + "\n", encoding="utf-8")
            native = Path(temp) / "owned-profile-root"
            with self.assertRaisesRegex(ValueError, "changed or lacks matching"):
                ev.install(SimpleNamespace(run=run, native_root=native))
            self.assertEqual(list(native.rglob("*.json")), [])

    def test_reference_image_must_match_captured_json(self):
        with tempfile.TemporaryDirectory(prefix="buttonscli-eval-tests-") as temp:
            root = Path(temp)
            source, image = root / "theme.json", root / "theme.png"
            ev.write_json(source, self.template)
            image.write_bytes(b"synthetic-hash-test-only")
            ev.write_json(root / "native-validation.json", {"passed": True, "files": [{
                "file": source.name, "sha256": ev.digest(source), "image": image.name, "imageSha256": "wrong-image"}]})
            with self.assertRaisesRegex(ValueError, "hash-matched"):
                ev.matched_reference(source, image)

    def test_invalid_profile_cannot_escape_owned_root(self):
        with tempfile.TemporaryDirectory(prefix="buttonscli-eval-tests-") as temp:
            root = Path(temp)
            ev.write_json(root / "active-profile.json", {"name": "../other"})
            with self.assertRaisesRegex(ValueError, "profile name is invalid"):
                ev.theme_destination(root)
            self.assertFalse((root / "profiles").exists())


if __name__ == "__main__":
    unittest.main()
