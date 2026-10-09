"""Exercise installation without changing real harness configuration."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "scripts/install.py"


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="maintainable-install-", dir="/tmp/opencode" if Path("/tmp/opencode").is_dir() else None)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.project = self.root / "project with spaces"
        self.project.mkdir()
        self.env = {**os.environ, "HOME": str(self.root / "home"), "XDG_CONFIG_HOME": str(self.root / "config")}

    def run_installer(self, *args):
        return subprocess.run([sys.executable, str(INSTALLER), *map(str, args)], env=self.env, text=True, capture_output=True)

    def test_project_targets_and_repeat_install(self):
        for harness, directory in (("shared", ".agents"), ("opencode", ".opencode"), ("crush", ".crush"), ("claude-code", ".claude"), ("codex", ".agents")):
            with self.subTest(harness=harness):
                result = self.run_installer("--harness", harness, "--project", self.project)
                self.assertEqual(result.returncode, 0, result.stderr)
                installed = self.project / directory / "skills/maintainable-code"
                self.assertTrue((installed / "SKILL.md").is_file())
                self.assertTrue((installed / "scripts/check.py").is_file())
                repeated = self.run_installer("--harness", harness, "--project", self.project)
                self.assertEqual(repeated.returncode, 0, repeated.stderr)
                self.assertIn("identical", repeated.stdout)

    def test_dry_run_has_no_writes(self):
        result = self.run_installer("--harness", "crush", "--project", self.project, "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(list(self.project.iterdir()), [])

    def test_user_install_respects_xdg(self):
        for harness in ("opencode", "crush"):
            result = self.run_installer("--harness", harness, "--scope", "user")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((self.root / "config" / harness / "skills/maintainable-code/SKILL.md").is_file())
        result = self.run_installer("--harness", "shared", "--scope", "user")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / "config/agents/skills/maintainable-code/SKILL.md").is_file())

    def test_existing_work_is_not_overwritten(self):
        target = self.project / ".agents/skills/maintainable-code"
        target.mkdir(parents=True)
        (target / "SKILL.md").write_text("user content")
        result = self.run_installer("--harness", "shared", "--project", self.project)
        self.assertEqual(result.returncode, 1)
        self.assertEqual((target / "SKILL.md").read_text(), "user content")

    def test_custom_skills_directory(self):
        parent = self.root / "custom skills"
        result = self.run_installer("--skills-dir", parent)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((parent / "maintainable-code/SKILL.md").is_file())

    def test_installed_checker_uses_explicit_project_from_other_cwd(self):
        parent = self.root / "custom skills"
        installed = self.run_installer("--skills-dir", parent)
        self.assertEqual(installed.returncode, 0, installed.stderr)
        (self.project / "quality.toml").write_text(
            'version = 1\n[[commands]]\nid = "smoke"\n'
            'argv = ["{python}", "-c", "print(123)"]\n'
        )
        result = subprocess.run(
            [sys.executable, str(parent / "maintainable-code/scripts/check.py"),
             "--project", str(self.project), "--format", "json"],
            cwd=self.root, text=True, capture_output=True, timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "passed")
        self.assertEqual(report["project"], str(self.project))

    def test_invalid_project_is_not_created(self):
        missing = self.root / "missing"
        result = self.run_installer("--harness", "crush", "--project", missing)
        self.assertEqual(result.returncode, 2)
        self.assertFalse(missing.exists())

    def test_source_cannot_be_its_own_install_parent(self):
        source = ROOT / "skills/maintainable-code"
        result = self.run_installer("--skills-dir", source, "--dry-run")
        self.assertEqual(result.returncode, 1)
        self.assertIn("inside the source", result.stderr)


if __name__ == "__main__":
    unittest.main()
