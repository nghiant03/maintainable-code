#!/usr/bin/env python3
"""Subprocess-level tests for the bundled maintainable-code checker.

Every test builds a real throwaway project under /tmp/opencode (when it
exists) and runs skills/maintainable-code/scripts/check.py in a separate
interpreter, exactly like a harness would.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

CHECKER = (Path(__file__).resolve().parents[1]
           / "skills" / "maintainable-code" / "scripts" / "check.py")
BASE = "/tmp/opencode" if os.path.isdir("/tmp/opencode") else tempfile.gettempdir()

CLEAN_CONFIG = """\
version = 1
[[commands]]
id = "selftest"
argv = ["{python}", "-c", "pass"]
"""


class CheckerTests(unittest.TestCase):
    maxDiff = None

    def make_project(self, files=None, config=CLEAN_CONFIG) -> Path:
        root = Path(tempfile.mkdtemp(prefix="check-test-", dir=BASE))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        for rel, content in (files or {}).items():
            target = root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content)
        (root / "quality.toml").write_text(config)
        return root

    def run_checker(self, project: Path, *extra: str, fmt="json"):
        argv = [sys.executable, str(CHECKER), "--project", str(project),
                "--format", fmt, *extra]
        return subprocess.run(argv, capture_output=True, text=True, timeout=120)

    def run_json(self, project: Path, *extra: str) -> dict:
        proc = self.run_checker(project, *extra, fmt="json")
        try:
            payload = json.loads(proc.stdout)
        except json.JSONDecodeError:
            self.fail(f"invalid JSON output (rc={proc.returncode}):\n"
                      f"{proc.stdout}\nstderr:\n{proc.stderr}")
        return payload

    # -- basic outcomes ----------------------------------------------------

    def test_pass(self):
        project = self.make_project({"a.py": "x = 1\n"})
        payload = self.run_json(project)
        self.assertEqual(payload["schemaVersion"], 1)
        self.assertEqual(payload["status"], "passed")
        self.assertEqual(self.run_checker(project, fmt="text").returncode, 0)

    def test_json_schema_stable(self):
        payload = self.run_json(self.make_project())
        self.assertEqual(
            set(payload),
            {"schemaVersion", "status", "mode", "project", "checkedState",
             "checks", "findings", "errors", "totalFindings",
             "omittedFindings"})
        self.assertEqual(
            set(payload["checkedState"]),
            {"configPath", "fingerprintBefore", "fingerprintAfter", "stale",
             "fileCount", "snapshotPaths", "scope"})
        self.assertNotEqual(payload["checkedState"]["fingerprintBefore"], None)
        self.assertNotEqual(payload["checkedState"]["fingerprintAfter"], None)
        self.assertFalse(payload["checkedState"]["stale"])
        self.assertEqual(payload["project"], str(Path(payload["project"])))

    def test_failure_exit_code_one_and_finding(self):
        config = CLEAN_CONFIG.replace("pass", "import sys; sys.exit(1)")
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "failed")
        check = payload["checks"][0]
        self.assertEqual(check["status"], "fail")
        self.assertEqual(check["exitCode"], 1)
        finding = payload["findings"][0]
        self.assertEqual(finding["ruleId"], "selftest")
        self.assertEqual(finding["severity"], "error")
        self.assertIn("suggestion", finding)

    def test_warning_severity_fails_soft(self):
        config = CLEAN_CONFIG.replace("pass", "import sys; sys.exit(1)") \
            .replace('id = "selftest"', 'id = "selftest"\nseverity = "warning"')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "passed")
        self.assertEqual(payload["checks"][0]["status"], "warning")
        self.assertEqual(payload["findings"][0]["severity"], "warning")

    def test_missing_executable_is_incomplete(self):
        config = 'version = 1\n[[commands]]\nid = "x"\nargv = ["no-such-binary-xyz"]\n'
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertEqual(payload["checks"][0]["status"], "incomplete")
        self.assertIn("cannot start", payload["checks"][0]["detail"])

    def test_unexpected_exit_code_is_incomplete(self):
        config = CLEAN_CONFIG.replace("pass", "import sys; sys.exit(3)")
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("unexpected exit code 3", payload["checks"][0]["detail"])

    def test_timeout_is_incomplete(self):
        config = CLEAN_CONFIG.replace(
            "pass", "import time; time.sleep(10)").replace(
            'id = "selftest"', 'id = "selftest"\ntimeout = 1')
        started = time.monotonic()
        payload = self.run_json(self.make_project(config=config))
        self.assertLess(time.monotonic() - started, 30)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("timed out", payload["checks"][0]["detail"])

    def test_failure_code_two_configurable(self):
        config = CLEAN_CONFIG.replace(
            "pass", "import sys; sys.exit(2)").replace(
            'id = "selftest"', 'id = "selftest"\nfailure_codes = [2]')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "failed")
        self.assertEqual(payload["checks"][0]["exitCode"], 2)

    def test_empty_failure_codes_means_any_nonzero_incomplete(self):
        # Availability-probe pattern (python -m ruff --version with
        # failure_codes = []): a missing -m module exits nonzero and must
        # be indistinguishable-from-violation no longer -- it is incomplete
        # (tool unavailable), never a "fail" finding.
        for code in (1, 2):
            config = CLEAN_CONFIG.replace(
                "pass", f"import sys; sys.exit({code})").replace(
                'id = "selftest"', 'id = "selftest"\nfailure_codes = []')
            payload = self.run_json(self.make_project(config=config))
            self.assertEqual(payload["status"], "incomplete", code)
            self.assertEqual(payload["checks"][0]["status"], "incomplete")
            self.assertIn(f"unexpected exit code {code}",
                          payload["checks"][0]["detail"])
            self.assertEqual(payload["findings"], [])
        # exit 0 with failure_codes = [] still passes
        payload = self.run_json(self.make_project(
            config=CLEAN_CONFIG.replace(
                'id = "selftest"', 'id = "selftest"\nfailure_codes = []')))
        self.assertEqual(payload["status"], "passed")

    # -- configuration validation ------------------------------------------

    def test_malformed_config(self):
        payload = self.run_json(self.make_project(config="not [ valid"))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("cannot parse", payload["errors"][0])

    def test_empty_config_selects_nothing(self):
        payload = self.run_json(self.make_project(config=""))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("no checks selected", payload["errors"][0])

    def test_unknown_config_key_typo(self):
        config = CLEAN_CONFIG + '\n[snapshots]\npaths = ["."]\n'
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("unknown top-level key 'snapshots'", payload["errors"][0])

    def test_removed_python_rules_rejected_before_commands_run(self):
        legacy = '''
[[python_import_rules]]
id = "ARCH-DOMAIN"
paths = ["src/domain"]
forbidden = ["requests"]
'''
        # Both legacy-only and mixed policies must fail visibly rather than
        # silently drop a project's architecture constraints.
        for prefix in ('version = 1\n', CLEAN_CONFIG):
            with self.subTest(commands=prefix == CLEAN_CONFIG):
                project = self.make_project(config=prefix + legacy)
                proc = self.run_checker(project)
                payload = json.loads(proc.stdout)
                self.assertEqual(proc.returncode, 2)
                self.assertEqual(payload["status"], "incomplete")
                self.assertIn("unknown top-level key 'python_import_rules'",
                              payload["errors"][0])
                self.assertEqual(payload["checks"], [])

    def test_unknown_command_key_typo(self):
        config = CLEAN_CONFIG.replace(
            'id = "selftest"', 'id = "selftest"\ntimeoutt = 5')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("unknown key in commands[0]: 'timeoutt'",
                      payload["errors"][0])

    def test_fast_only_modes_rejected(self):
        config = CLEAN_CONFIG.replace(
            'id = "selftest"', 'id = "selftest"\nmodes = ["fast"]')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("superset", payload["errors"][0])

    def test_full_only_command_mode_selection(self):
        config = """\
version = 1
[[commands]]
id = "full-only"
argv = ["{python}", "-c", "pass"]
modes = ["full"]
[[commands]]
id = "always"
argv = ["{python}", "-c", "pass"]
"""
        project = self.make_project({"src/a.py": "x = 1\n"}, config=config)
        fast = self.run_json(project, "--mode", "fast")
        self.assertEqual([c["id"] for c in fast["checks"]], ["always"])
        full = self.run_json(project, "--mode", "full")
        self.assertEqual(sorted(c["id"] for c in full["checks"]),
                         ["always", "full-only"])

    def test_missing_project_directory(self):
        payload = self.run_json(Path(BASE) / "definitely-missing-project-xyz")
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("project directory not found", payload["errors"][0])

    def test_missing_config_file(self):
        proc = self.run_checker(self.make_project(), "--config", "gone.toml",
                                fmt="json")
        self.assertEqual(proc.returncode, 2)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("config file not found", payload["errors"][0])

    def test_no_checks_selected_for_mode(self):
        config = """\
version = 1
[[commands]]
id = "full-only"
argv = ["{python}", "-c", "pass"]
modes = ["full"]
"""
        proc = self.run_checker(self.make_project(config=config),
                                "--mode", "fast", fmt="json")
        self.assertEqual(proc.returncode, 2)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("no checks selected for mode 'fast'",
                      payload["errors"][0])

    def test_aggregate_exit_codes(self):
        # a warning-severity failure alone is tolerated (exit 0) ...
        config = """\
version = 1
[[commands]]
id = "warn"
argv = ["{python}", "-c", "import sys; sys.exit(1)"]
severity = "warning"
"""
        proc = self.run_checker(self.make_project(config=config), fmt="json")
        self.assertEqual(proc.returncode, 0)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "passed")
        self.assertEqual(len(payload["findings"]), 1)
        # ... but one error-severity failure aggregates to exit 1 ...
        proc = self.run_checker(self.make_project(config=CLEAN_CONFIG.replace(
            "pass", "import sys; sys.exit(1)")), fmt="json")
        self.assertEqual(proc.returncode, 1)
        # ... and any incomplete check aggregates to exit 2 even next to a
        # passing one.
        config = CLEAN_CONFIG + (
            '[[commands]]\nid = "odd"\n'
            'argv = ["{python}", "-c", "import sys; sys.exit(3)"]\n')
        proc = self.run_checker(self.make_project(config=config), fmt="json")
        self.assertEqual(proc.returncode, 2)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "incomplete")
        self.assertEqual([c["status"] for c in payload["checks"]],
                         ["pass", "incomplete"])

    # -- command execution semantics ----------------------------------------

    def test_spaces_and_shell_metacharacters_are_literal(self):
        config = CLEAN_CONFIG.replace(
            "pass", "pass # ; touch pwned && echo $HOME `id`")
        project = self.make_project(config=config)
        root = Path(tempfile.mkdtemp(prefix="check-name-", dir=BASE))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        spaced = root / "pro ject (v1); rm -rf x"
        shutil.copytree(project, spaced)
        payload = self.run_json(spaced)
        self.assertEqual(payload["status"], "passed")
        self.assertFalse((spaced / "pwned").exists())

    def test_cwd_subdir_and_escape(self):
        config = """\
version = 1
[[commands]]
id = "where"
argv = ["{python}", "-c", "import os; print(os.getcwd())"]
cwd = "sub"
"""
        project = self.make_project({"sub/keep.txt": "x"}, config=config)
        payload = self.run_json(project)
        self.assertIn(str(project / "sub"), payload["checks"][0]["output"])
        bad = config.replace('cwd = "sub"', 'cwd = ".."')
        payload = self.run_json(self.make_project(config=bad))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("escapes the project", payload["checks"][0]["detail"])

    def test_output_is_bounded(self):
        config = CLEAN_CONFIG.replace(
            "pass", "import sys; sys.stdout.write('x' * 1_000_000)")
        payload = self.run_json(self.make_project(config=config))
        self.assertLess(len(payload["checks"][0]["output"]), 50_000)

    def test_architecture_command_owns_scope_and_diagnostics(self):
        # A repository tool checks files excluded from the snapshot. Python
        # is just the portable fixture runtime; the runner sees only argv.
        tool = (
            "from pathlib import Path\n"
            "import sys\n"
            "source = Path('src/domain/service.ts').read_text()\n"
            "if '../adapters' in source:\n"
            "    print('src/domain/service.ts:1: forbidden dependency')\n"
            "    sys.exit(7)\n"
        )
        config = '''version = 1
[snapshot]
exclude = ["src"]
[[commands]]
id = "architecture"
argv = ["{python}", "-B", "tools/check_architecture.py"]
failure_codes = [7]
policy = "docs/architecture.md"
'''
        project = self.make_project({
            "tools/check_architecture.py": tool,
            "src/domain/service.ts": "import { save } from '../adapters';\n",
        }, config=config)
        proc = self.run_checker(project)
        payload = json.loads(proc.stdout)
        self.assertEqual(proc.returncode, 1)
        self.assertEqual(payload["status"], "failed")
        check = payload["checks"][0]
        self.assertEqual(check["kind"], "command")
        self.assertEqual(check["exitCode"], 7)
        self.assertIn("service.ts:1: forbidden dependency", check["output"])
        finding = payload["findings"][0]
        self.assertEqual(finding["ruleId"], "architecture")
        self.assertEqual(finding["policy"], "docs/architecture.md")
        self.assertNotIn("path", finding)  # Tool output is not parsed.
        self.assertFalse(payload["checkedState"]["stale"])
        (project / "src/domain/service.ts").write_text("export const x = 1;\n")
        self.assertEqual(self.run_checker(project).returncode, 0)

    # -- staleness (a command mutating the state during the run) --------------

    def test_stale_when_command_edits_file(self):
        config = CLEAN_CONFIG.replace(
            "pass", "open('a.py', 'w').write('tampered')")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])
        self.assertNotEqual(payload["checkedState"]["fingerprintBefore"],
                            payload["checkedState"]["fingerprintAfter"])
        # the check itself passed; staleness is reported separately
        self.assertEqual(payload["checks"][0]["status"], "pass")

    def test_stale_outranks_failed_but_preserves_failures(self):
        config = CLEAN_CONFIG.replace(
            "pass",
            "import sys; open('new.txt', 'w').write('created'); sys.exit(1)")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])
        self.assertEqual(payload["checks"][0]["status"], "fail")
        self.assertEqual(len(payload["findings"]), 1)

    def test_stale_when_command_touches_config(self):
        config = CLEAN_CONFIG.replace(
            "pass", "open('quality.toml', 'a').write('# edited')")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])

    def test_stale_when_command_renames_file(self):
        config = CLEAN_CONFIG.replace(
            "pass", "import os; os.rename('a.py', 'b.py')")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])
        self.assertNotEqual(payload["checkedState"]["fingerprintBefore"],
                            payload["checkedState"]["fingerprintAfter"])

    def test_stale_when_command_deletes_file(self):
        config = CLEAN_CONFIG.replace(
            "pass", "import os; os.remove('a.py')")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])
        self.assertNotEqual(payload["checkedState"]["fingerprintBefore"],
                            payload["checkedState"]["fingerprintAfter"])

    def test_stale_when_command_writes_lockfile(self):
        # A newly created lockfile is an untracked file inside the snapshot
        # scope; it must invalidate the fingerprint like any other change.
        config = CLEAN_CONFIG.replace(
            "pass", "open('poetry.lock', 'w').write('locked')")
        project = self.make_project({"a.py": "x = 1\n"}, config=config)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertTrue(payload["checkedState"]["stale"])

    # -- snapshot coverage ----------------------------------------------------

    def test_snapshot_tracks_mixed_languages_without_parsing(self):
        files = {"src/code" + suffix: "not valid source in any language\n"
                 for suffix in (".py", ".ts", ".go", ".rs", ".java")}
        project = self.make_project(files)
        initial = self.run_json(project)
        self.assertEqual(initial["status"], "passed")
        self.assertEqual(initial["checkedState"]["fileCount"], 7)
        previous = initial["checkedState"]["fingerprintBefore"]
        for name in files:
            with self.subTest(path=name):
                (project / name).write_bytes(b"changed\xff\n")
                payload = self.run_json(project)
                self.assertEqual(payload["status"], "passed")
                current = payload["checkedState"]["fingerprintBefore"]
                self.assertNotEqual(current, previous)
                self.assertFalse(payload["checkedState"]["stale"])
                previous = current

    def test_excludes_and_untracked_files_counted(self):
        files = {
            "a.py": "x = 1\n",
            "notes.txt": "hello\n",
            "node_modules/left-pad/index.js": "module.exports = 1\n",
            ".git/HEAD": "ref: refs/heads/main\n",
            ".venv/pyvenv.cfg": "home = /usr\n",
        }
        config = CLEAN_CONFIG + '\n[snapshot]\nexclude = ["node_modules", ".venv"]\n'
        project = self.make_project(files, config=config)
        payload = self.run_json(project)
        # a.py + notes.txt + quality.toml survive the excludes (both the
        # built-in .git default and the user additions) + checker itself
        self.assertEqual(payload["checkedState"]["fileCount"], 4)

    def test_missing_snapshot_path_is_incomplete(self):
        config = CLEAN_CONFIG.replace(
            'version = 1', 'version = 1\n[snapshot]\npaths = ["src", "gone"]')
        payload = self.run_json(self.make_project({"src/a.py": "x"},
                                                  config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("snapshot path missing: gone", payload["errors"][0])

    def test_symlink_escaping_project_rejected(self):
        project = self.make_project({"a.py": "x = 1\n"})
        outside = Path(tempfile.mkdtemp(prefix="check-outside-", dir=BASE))
        self.addCleanup(shutil.rmtree, outside, ignore_errors=True)
        (outside / "secret.txt").write_text("outside\n")
        (project / "link.txt").symlink_to(outside / "secret.txt")
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("escapes the project", "; ".join(payload["errors"]))

    # -- hardening: snapshot traversal --------------------------------------

    def test_fifo_in_snapshot_rejected_not_hung(self):
        project = self.make_project({"a.py": "x = 1\n"})
        os.mkfifo(project / "pipe")
        payload = self.run_json(project)  # the 120s subprocess timeout guards
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("not a regular file", "; ".join(payload["errors"]))

    @unittest.skipIf(not hasattr(os, "geteuid") or os.geteuid() == 0,
                     "permission bits are ineffective for root")
    def test_unreadable_directory_reported(self):
        project = self.make_project({"a.py": "x = 1\n",
                                     "locked/secret.py": "x = 1\n"})
        os.chmod(project / "locked", 0)
        self.addCleanup(os.chmod, project / "locked", 0o755)
        payload = self.run_json(project)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("cannot list directory", "; ".join(payload["errors"]))

    # -- configuration strictness --------------------------------------------

    def test_version_bool_and_float_rejected(self):
        for bad in ("version = true", "version = 1.0", 'version = "1"'):
            config = CLEAN_CONFIG.replace("version = 1", bad)
            payload = self.run_json(self.make_project(config=config))
            self.assertEqual(payload["status"], "incomplete", bad)
            self.assertIn("unsupported version", payload["errors"][0])

    def test_nonfinite_timeout_rejected(self):
        for bad in ("nan", "inf"):
            config = CLEAN_CONFIG.replace(
                'id = "selftest"', f'id = "selftest"\ntimeout = {bad}')
            payload = self.run_json(self.make_project(config=config))
            self.assertEqual(payload["status"], "incomplete", bad)
            self.assertIn("timeout", payload["errors"][0])

    def test_empty_path_strings_rejected(self):
        config = CLEAN_CONFIG + '\n[snapshot]\npaths = [""]\n'
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("empty strings", payload["errors"][0])

    def test_duplicate_fast_modes_rejected(self):
        config = CLEAN_CONFIG.replace(
            'id = "selftest"', 'id = "selftest"\nmodes = ["fast", "fast"]')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("superset", payload["errors"][0])

    def test_duplicate_command_ids_rejected(self):
        config = CLEAN_CONFIG + (
            '\n[[commands]]\nid = "selftest"\n'
            'argv = ["{python}", "-c", "pass"]\n')
        payload = self.run_json(self.make_project({"src/a.py": "x = 1\n"},
                                                  config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("duplicate id", "; ".join(payload["errors"]))

    def test_nul_argv_is_config_error_without_traceback(self):
        # TOML "\u0000" escape produces a real NUL in the argv string.
        config = CLEAN_CONFIG.replace('"pass"', '"pass\\u0000"')
        proc = self.run_checker(self.make_project(config=config), fmt="json")
        self.assertEqual(proc.returncode, 2)
        self.assertNotIn("Traceback", proc.stderr)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("NUL", "; ".join(payload["errors"]))

    def test_argv_empty_string_argument_allowed(self):
        # shell=False passes empty-string arguments verbatim; only argv[0]
        # must be a usable executable.
        config = CLEAN_CONFIG.replace(
            '"pass"',
            '"import sys; sys.exit(0 if sys.argv[1] == \'\' else 1)", ""')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "passed", payload)

    def test_argv_empty_executable_rejected(self):
        config = CLEAN_CONFIG.replace('"{python}"', '""')
        payload = self.run_json(self.make_project(config=config))
        self.assertEqual(payload["status"], "incomplete")
        self.assertIn("argv", payload["errors"][0])

    def test_absolute_config_paths_rejected(self):
        # Snapshot paths/exclude and cwd must be project-relative.
        cases = [
            CLEAN_CONFIG + '\n[snapshot]\npaths = ["/etc"]\n',
            CLEAN_CONFIG + '\n[snapshot]\nexclude = ["/etc"]\n',
            CLEAN_CONFIG.replace(
                'id = "selftest"', 'id = "selftest"\ncwd = "/tmp"'),
        ]
        for config in cases:
            payload = self.run_json(self.make_project(
                {"src/domain/api.py": "x = 1\n"}, config=config))
            self.assertEqual(payload["status"], "incomplete", config)
            self.assertIn("project-relative", "; ".join(payload["errors"]))

    # -- output bounding ------------------------------------------------------

    def test_truncation_marker_for_oversized_output(self):
        exactly = CLEAN_CONFIG.replace(
            "pass", "import sys; sys.stdout.write('x' * 20_000)")
        payload = self.run_json(self.make_project(config=exactly))
        out = payload["checks"][0]["output"]
        self.assertTrue(out.startswith("x" * 100))
        self.assertNotIn("truncated", out)
        over = CLEAN_CONFIG.replace(
            "pass", "import sys; sys.stdout.write('x' * 20_001)")
        payload = self.run_json(self.make_project(config=over))
        out = payload["checks"][0]["output"]
        self.assertIn("...[truncated]", out)
        self.assertLessEqual(len(out), 20_100)

    # -- report bounding -------------------------------------------------------

    def test_findings_bounded_with_totals(self):
        config = 'version = 1\n' + ''.join(
            f'[[commands]]\nid = "check-{i:03d}"\n'
            'argv = ["{python}", "-c", "import sys; sys.exit(1)"]\n'
            for i in range(210))
        proc = self.run_checker(self.make_project(
            config=config), fmt="json")
        payload = json.loads(proc.stdout)
        # status and exit code reflect the full (untruncated) finding set
        self.assertEqual(payload["status"], "failed")
        self.assertEqual(proc.returncode, 1)
        self.assertEqual(payload["totalFindings"], 210)
        self.assertEqual(payload["omittedFindings"], 10)
        self.assertEqual(len(payload["findings"]), 200)

    def test_problem_detail_bounded(self):
        paths = [f"missing-{i:02d}" for i in range(55)]
        config = CLEAN_CONFIG + '\n[snapshot]\npaths = ' + json.dumps(paths)
        proc = self.run_checker(self.make_project(
            config=config), fmt="json")
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["status"], "incomplete")
        self.assertEqual(proc.returncode, 2)
        detail = payload["errors"][0]
        self.assertIn("...and 5 more", detail)
        self.assertLess(len(detail), 10_000)

    # -- text report -----------------------------------------------------------

    def test_text_report_has_fingerprint_suggestion_policy(self):
        config = CLEAN_CONFIG.replace(
            '"pass"', '"import sys; sys.exit(1)"') + 'policy = "POL-42"\n'
        project = self.make_project(config=config)
        proc = self.run_checker(project, fmt="text")
        self.assertEqual(proc.returncode, 1)
        text = proc.stdout
        self.assertIn("fingerprint: ", text)
        self.assertIn("suggestion:", text)
        self.assertIn("policy: POL-42", text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
