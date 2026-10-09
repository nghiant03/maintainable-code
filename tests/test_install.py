"""Exercise installation without changing real harness configuration."""

import json
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "scripts/install.py"
SPEC = importlib.util.spec_from_file_location("skill_installer", INSTALLER)
installer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(installer)


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

    def install_custom(self):
        parent = self.root / "custom skills"
        result = self.run_installer("--skills-dir", parent)
        self.assertEqual(result.returncode, 0, result.stderr)
        return parent / "maintainable-code"

    def test_update_replaces_contents_without_retained_backup(self):
        target = self.install_custom()
        (target / "SKILL.md").write_text("old version with local edits")
        (target / "obsolete.txt").write_text("remove with update")
        (target / "references/commenting-principles.md").unlink()
        sibling = target.parent / "other-skill"
        sibling.mkdir()
        (sibling / "SKILL.md").write_text("leave unchanged")
        before = set(self.root.iterdir())
        result = self.run_installer("--skills-dir", target.parent, "--update")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Updated:", result.stdout)
        self.assertIn("replace: SKILL.md", result.stdout)
        self.assertIn("remove: obsolete.txt", result.stdout)
        self.assertIn("add: references/commenting-principles.md", result.stdout)
        self.assertTrue(installer.identical(installer.SOURCE, target))
        self.assertEqual((sibling / "SKILL.md").read_text(), "leave unchanged")
        self.assertEqual(set(self.root.iterdir()), before)

    def test_update_dry_run_has_no_writes(self):
        target = self.install_custom()
        (target / "SKILL.md").write_text("local edits")
        before = set(self.root.iterdir())
        result = self.run_installer("--skills-dir", target.parent, "--update", "--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(str(target), result.stdout)
        self.assertIn("replace: SKILL.md", result.stdout)
        self.assertIn("no backup is retained", result.stdout)
        self.assertEqual((target / "SKILL.md").read_text(), "local edits")
        self.assertEqual(set(self.root.iterdir()), before)

    def test_update_identical_install_is_noop(self):
        target = self.install_custom()
        before = set(self.root.iterdir())
        result = self.run_installer("--skills-dir", target.parent, "--update")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("identical", result.stdout)
        self.assertEqual(set(self.root.iterdir()), before)

    def test_update_missing_install_creates_it(self):
        parent = self.root / "new skills"
        result = self.run_installer("--skills-dir", parent, "--update")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(installer.identical(installer.SOURCE, parent / "maintainable-code"))

    def test_update_refuses_symlinks(self):
        target = self.install_custom()
        original = self.root / "original"
        original.write_text("do not touch")
        (target / "linked.txt").symlink_to(original)
        result = self.run_installer("--skills-dir", target.parent, "--update")
        self.assertEqual(result.returncode, 1)
        self.assertIn("symlinked", result.stderr)
        self.assertEqual(original.read_text(), "do not touch")

    def test_update_refuses_symlinked_directory_and_parent(self):
        target = self.install_custom()
        alias = self.root / "alias"
        alias.symlink_to(target.parent, target_is_directory=True)
        result = self.run_installer("--skills-dir", alias, "--update")
        self.assertEqual(result.returncode, 1)
        self.assertIn("symlinked", result.stderr)
        parent = self.root / "linked skills"
        parent.mkdir()
        (parent / "maintainable-code").symlink_to(target, target_is_directory=True)
        result = self.run_installer("--skills-dir", parent, "--update")
        self.assertEqual(result.returncode, 1)
        self.assertIn("symlinked", result.stderr)

    def test_update_refuses_non_skill_directory(self):
        parent = self.root / "skills"
        target = parent / "maintainable-code"
        target.mkdir(parents=True)
        (target / "keep.txt").write_text("user work")
        result = self.run_installer("--skills-dir", parent, "--update")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no SKILL.md", result.stderr)
        self.assertEqual((target / "keep.txt").read_text(), "user work")

    def test_failed_publication_restores_old_install(self):
        target = self.install_custom()
        (target / "SKILL.md").write_text("previous contents")
        previous = self.root / "previous"
        staged = self.root / "staged"
        staged.mkdir()
        rename = Path.rename

        def fail_publication(path, destination):
            if path == staged:
                raise OSError("publication failed")
            return rename(path, destination)

        with patch.object(Path, "rename", fail_publication):
            with self.assertRaisesRegex(OSError, "previous installation restored"):
                installer.publish_update(staged, target, previous)
        self.assertEqual((target / "SKILL.md").read_text(), "previous contents")
        self.assertFalse(previous.exists())

    def test_failed_rollback_preserves_recovery_directory(self):
        target = self.install_custom()
        (target / "SKILL.md").write_text("previous contents")
        rename = Path.rename

        def fail_publication_and_rollback(path, destination):
            if path == target:
                return rename(path, destination)
            raise OSError("rename failed")

        with patch.object(Path, "rename", fail_publication_and_rollback), patch.object(
            sys, "argv", [str(INSTALLER), "--skills-dir", str(target.parent), "--update"]
        ):
            self.assertEqual(installer.main(), 1)
        recoveries = list(self.root.glob(".maintainable-install-*/previous/SKILL.md"))
        self.assertEqual(len(recoveries), 1)
        self.assertEqual(recoveries[0].read_text(), "previous contents")

    def test_failed_staging_keeps_old_install(self):
        target = self.install_custom()
        (target / "SKILL.md").write_text("previous contents")
        with patch.object(installer.shutil, "copytree", side_effect=OSError("copy failed")), patch.object(
            sys, "argv", [str(INSTALLER), "--skills-dir", str(target.parent), "--update"]
        ):
            self.assertEqual(installer.main(), 1)
        self.assertEqual((target / "SKILL.md").read_text(), "previous contents")
        self.assertEqual(list(self.root.glob(".maintainable-install-*")), [])

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
