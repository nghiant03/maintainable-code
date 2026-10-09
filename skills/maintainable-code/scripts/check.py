#!/usr/bin/env python3
"""Bundled checker for the `maintainable-code` skill.

Evaluates a project against its own ``quality.toml`` policy. Stdlib-only
(Python 3.11+).

The runner itself performs no writes: it never edits project code or
policy, installs tools, or auto-fixes. It is *not* a sandbox, however:
the ``[[commands]]`` you configure are executed with the invoking user's
privileges and can modify files, so every configured command must be
check-only (report; never reformat, rewrite, or install anything). The
checker neither enforces nor certifies that -- review your own config.
A passing report is therefore a verified snapshot of the checked state,
not a hard certification of code quality.

Usage:
    python3 check.py --project PATH [--config quality.toml]
                     [--mode fast|full] [--format text|json]

Exit codes:
    0  passed (warnings permitted)
    1  failed (at least one error-severity check failed)
    2  incomplete (invalid/missing config, no checks selected, missing
       tools, timeouts, stale snapshot, unreadable sources, interrupt)

Report contract: JSON output always contains ``totalFindings`` and
``omittedFindings``; at most 200 findings are listed while the aggregate
status is computed from the full set (same for error/detail text). The
text report identifies the checked source state via its ``fingerprint``
line and prints each finding with its suggestion and policy reference.

Configuration schema (quality.toml)::

    version = 1

    [snapshot]
    # Project-relative files/directories to fingerprint (default ["."]).
    # Absolute paths are rejected.
    paths = ["."]
    # Additional directory names/paths to prune; literal names/paths only
    # (no globs), matched against project-relative paths; ADDED to the
    # built-in defaults (.git, __pycache__, .pytest_cache, .mypy_cache,
    # .ruff_cache, .crush/logs).
    exclude = ["node_modules", ".venv", "dist"]

    [[commands]]
    id = "lint"                          # required, unique
    argv = ["python3", "-m", "ruff", "check", "."]
    modes = ["fast", "full"]             # default both; ['full'] allowed;
                                         # fast-only (['fast']) rejected so
                                         # full is always a superset
    cwd = "."                            # default "."; project-relative
    timeout = 120                        # seconds, positive, default 120
    severity = "error"                   # "error" | "warning", default error
    failure_codes = [1]                  # default [1]; may include 2;
                                         # [] means every nonzero exit is
                                         # incomplete (availability probes)
    policy = "optional reference"        # optional string

Path contract: every path in the configuration ([snapshot] ``paths`` and
``exclude`` and command ``cwd``) must be
project-relative; absolute paths are rejected. ``..`` components are
normalized first, and any path that resolves outside the project makes the
run incomplete instead of being followed.

The first ``argv`` token may be the literal ``"{python}"``, resolved to the
running interpreter (``sys.executable``) for portable self-test commands.
Empty-string arguments are allowed after the first (executable) element;
with ``shell=False`` they are passed to the child verbatim, and legitimate
tools accept them (e.g. empty prefix/ delimiter options). No other
expansion is performed; argv is passed to the child verbatim
(shell=False, stdin=DEVNULL, process group killed on timeout).

Documented limitations (by design):
  * The fingerprint covers only the files selected by ``[snapshot]`` (the
    config and the checker itself are always included). The tool
    environment (interpreter/tool versions, installed linters) is neither
    cached nor certified; the report states this scope.
  * Language-specific analysis, including import boundaries and dependency
    cycles, belongs to repository tools invoked through ``commands``.
  * Directory symlinks make the snapshot incomplete.
    File symlinks inside the project are hashed through to
    their target and recorded as links; symlinks escaping the project,
    directory symlinks, and non-regular files (FIFOs, sockets, devices --
    opening them could block forever) are rejected rather than silently
    trusted or opened within the configured snapshot scope.
    Unreadable directories are reported (``os.walk(onerror=...)``), never
    silently skipped.
  * Command exit codes are the only failure signal; output is shown but
    never parsed. A missing interpreter/module exits nonzero exactly like
    a real violation, so gate tools with an availability probe (e.g.
    ``argv = ["{python}", "-m", "ruff", "--version"]`` plus
    ``failure_codes = []``, where any nonzero exit is incomplete instead
    of a violation).
  * No baseline support and no auto-generation of policy in this version.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import signal
import stat
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

SCHEMA_VERSION = 1
DEFAULT_CONFIG = "quality.toml"
DEFAULT_TIMEOUT = 120
OUTPUT_LIMIT = 20_000  # bytes of child output kept per check (disk-buffered)
MAX_FINDINGS = 200      # findings kept in the report; totals always retained
MAX_PROBLEMS_SHOWN = 50  # problem strings joined into check details/errors
MAX_ERRORS = 100        # hard cap on report["errors"] entries
DEFAULT_EXCLUDES = (
    ".git", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache",
    ".crush/logs",
)
MODES = ("fast", "full")
SEVERITIES = ("error", "warning")
EXIT_OK, EXIT_FAILED, EXIT_INCOMPLETE = 0, 1, 2
TOP_KEYS = {"version", "snapshot", "commands"}
SNAPSHOT_KEYS = {"exclude", "paths"}
COMMAND_KEYS = {"id", "argv", "modes", "cwd", "timeout", "severity",
                "failure_codes", "policy"}
CHECKER_TAG = "<checker>"


class Incomplete(Exception):
    """A problem that makes verification incomplete (checker exit 2)."""


class Config:
    def __init__(self) -> None:
        self.snapshot_paths: list[str] = ["."]
        self.exclude: list[str] = list(DEFAULT_EXCLUDES)
        self.commands: list[dict] = []


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------

def within(base: Path, target: Path) -> bool:
    """True if resolved ``target`` is inside resolved ``base``."""
    try:
        target.resolve().relative_to(base)
        return True
    except ValueError:
        return False


def sha256_file(path: Path) -> str | None:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1 << 20), b""):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def is_excluded(rel_posix: str, name: str, excludes) -> bool:
    return any(
        rel_posix == pat or name == pat or rel_posix.startswith(pat + "/")
        for pat in excludes
    )


def str_list(value, label: str, errors: list[str], allow_empty=False):
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        errors.append(f"{label} must be a list of strings")
        return None
    if any(not v for v in value):
        errors.append(f"{label} must not contain empty strings")
        return None
    if not value and not allow_empty:
        errors.append(f"{label} must not be empty")
        return None
    return value


def ensure_relative_paths(values, label: str, errors: list[str]) -> None:
    """Config paths are project-relative by contract; absolute is rejected."""
    if any(os.path.isabs(v) for v in values):
        errors.append(
            f"{label} must be project-relative (absolute paths are "
            "rejected)")


def join_problems(problems: list[str],
                  max_shown: int = MAX_PROBLEMS_SHOWN) -> str:
    """Join problem strings, bounding detail/error size in the report."""
    if len(problems) <= max_shown:
        return "; ".join(problems)
    return ("; ".join(problems[:max_shown])
            + f"; ...and {len(problems) - max_shown} more")


def checker_path() -> Path:
    return Path(__file__).resolve()


# --------------------------------------------------------------------------
# configuration
# --------------------------------------------------------------------------

def load_config(path: Path) -> tuple[Config, list[str]]:
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise Incomplete(f"config file not found: {path}") from None
    except (OSError, UnicodeDecodeError) as exc:
        raise Incomplete(f"cannot read config {path}: {exc}") from None
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise Incomplete(f"cannot parse config {path}: {exc}") from None

    errors: list[str] = []
    cfg = Config()
    for key in raw:
        if key not in TOP_KEYS:
            errors.append(f"unknown top-level key {key!r}")
    version = raw.get("version", 1)
    if (isinstance(version, bool) or not isinstance(version, int)
            or version != 1):
        # ``true`` and ``1.0`` both compare equal to 1 in Python; only the
        # integer 1 is the documented schema version.
        errors.append(f"unsupported version {version!r}; expected integer 1")

    snap = raw.get("snapshot", {})
    if not isinstance(snap, dict):
        errors.append("[snapshot] must be a table")
        snap = {}
    for key in snap:
        if key not in SNAPSHOT_KEYS:
            errors.append(f"unknown [snapshot] key {key!r}")
    if "paths" in snap:
        paths = str_list(snap["paths"], "[snapshot] paths", errors)
        if paths:
            ensure_relative_paths(paths, "[snapshot] paths", errors)
            cfg.snapshot_paths = paths
    if "exclude" in snap:
        exclude = str_list(snap["exclude"], "[snapshot] exclude", errors,
                           allow_empty=True)
        if exclude is not None:
            ensure_relative_paths(exclude, "[snapshot] exclude", errors)
            cfg.exclude = list(DEFAULT_EXCLUDES) + exclude

    commands = raw.get("commands", [])
    if not isinstance(commands, list):
        errors.append("[[commands]] must be an array of tables")
        commands = []
    # Command identifiers must be unique so report output is unambiguous.
    seen: set[str] = set()
    for index, cmd in enumerate(commands):
        if not isinstance(cmd, dict):
            errors.append(f"commands[{index}] must be a table")
            continue
        for key in cmd:
            if key not in COMMAND_KEYS:
                errors.append(f"unknown key in commands[{index}]: {key!r}")
        cid = cmd.get("id")
        if not isinstance(cid, str) or not cid:
            errors.append(f"commands[{index}] needs a non-empty string id")
            continue
        if cid in seen:
            errors.append(
                f"duplicate id {cid!r} (ids must be unique across commands)")
        seen.add(cid)
        label = f"commands[{cid}]"
        argv = cmd.get("argv")
        if (not isinstance(argv, list) or not argv
                or not all(isinstance(a, str) for a in argv)
                or not argv[0]):
            # argv[0] must be a usable executable; later elements may be
            # empty strings (shell=False passes them verbatim, and tools
            # legitimately accept empty-valued arguments).
            errors.append(
                f"{label}.argv must be a non-empty list of strings whose "
                "first element (the executable) is non-empty")
            continue
        if any("\0" in a for a in argv):
            # subprocess would raise ValueError on exec; reject up front so
            # the run degrades to a clean "incomplete" instead.
            errors.append(f"{label}.argv must not contain NUL characters")
            continue
        modes = str_list(cmd.get("modes", list(MODES)), f"{label}.modes", errors) or []
        if set(modes) - set(MODES):
            errors.append(f"{label}.modes values must be in {list(MODES)}")
        if len(set(modes)) != len(modes):
            errors.append(f"{label}.modes contains duplicate entries")
        if set(modes) == {"fast"}:
            # set comparison so ['fast', 'fast'] is caught too
            errors.append(
                f"{label}.modes ['fast'] is not allowed; full must be a "
                "superset (use ['fast','full'] or omit modes)")
        timeout = cmd.get("timeout", DEFAULT_TIMEOUT)
        if (not isinstance(timeout, (int, float)) or isinstance(timeout, bool)
                or not math.isfinite(timeout) or timeout <= 0):
            # nan/inf slip past ``timeout <= 0`` comparisons
            errors.append(f"{label}.timeout must be a finite positive number")
            timeout = DEFAULT_TIMEOUT
        severity = cmd.get("severity", "error")
        if severity not in SEVERITIES:
            errors.append(f"{label}.severity must be 'error' or 'warning'")
            severity = "error"
        failure_codes = cmd.get("failure_codes", [1])
        if (not isinstance(failure_codes, list) or not all(
                isinstance(c, int) and not isinstance(c, bool) and c >= 1
                for c in failure_codes)):
            errors.append(f"{label}.failure_codes must be a list of ints >= 1")
            failure_codes = [1]
        cwd = cmd.get("cwd", ".")
        if not isinstance(cwd, str) or not cwd:
            errors.append(f"{label}.cwd must be a non-empty string")
            cwd = "."
        ensure_relative_paths([cwd], f"{label}.cwd", errors)
        policy = cmd.get("policy")
        if policy is not None and not isinstance(policy, str):
            errors.append(f"{label}.policy must be a string")
            policy = None
        cfg.commands.append({
            "id": cid, "argv": argv, "modes": modes, "cwd": cwd,
            "timeout": timeout, "severity": severity,
            "failure_codes": failure_codes, "policy": policy,
        })

    return cfg, errors


# --------------------------------------------------------------------------
# snapshot / fingerprint
# --------------------------------------------------------------------------

def iter_project_files(top: Path, project: Path, excludes,
                       problems: list[str], context: str):
    """Walk directory ``top`` (inside ``project``) safely.

    Yields ``(full_path, rel_posix, is_symlink)`` for every regular file
    and every project-internal symlink to a regular file. Nothing is
    silently skipped: unreadable directories (``os.walk(onerror=...)``),
    directory symlinks, non-regular files (FIFOs, sockets, devices --
    opening those could block forever) and symlinks escaping the project
    are reported as problems, which make the enclosing check incomplete.
    """

    def onerror(exc: OSError) -> None:
        where = getattr(exc, "filename", None) or str(top)
        problems.append(
            f"cannot list directory under {context}: {where}: {exc}")

    for root, dirs, names in os.walk(top, onerror=onerror):
        root_p = Path(root)
        keep = []
        for name in sorted(dirs):
            rel = (root_p / name).relative_to(project).as_posix()
            if is_excluded(rel, name, excludes):
                continue
            if (root_p / name).is_symlink():
                problems.append(
                    "directory symlink not supported (documented "
                    f"limitation): {rel}")
                continue
            keep.append(name)
        dirs[:] = keep
        for name in sorted(names):
            full = root_p / name
            rel = full.relative_to(project).as_posix()
            if is_excluded(rel, name, excludes):
                continue
            if full.is_symlink():
                target = Path(os.path.realpath(full))
                if not within(project, target) or not target.is_file():
                    problems.append(
                        "symlink escapes the project or is not a regular "
                        f"file: {rel}")
                    continue
                yield full, rel, True
                continue
            try:
                mode = full.stat().st_mode
            except OSError as exc:
                problems.append(f"cannot stat file: {rel}: {exc}")
                continue
            if not stat.S_ISREG(mode):
                problems.append(
                    f"not a regular file (rejected instead of opened; "
                    f"special files could block forever): {rel}")
                continue
            yield full, rel, False


def collect_snapshot(project: Path, cfg: Config, config_path: Path,
                     check_script: Path) -> tuple[list[dict], list[str]]:
    """Hash every selected file, sorted by path. Returns entries, problems."""
    problems: list[str] = []
    entries: list[dict] = []
    seen: set[str] = set()

    def add(key: str, kind: str, digest: str, target: str = "") -> None:
        if key in seen:
            return
        seen.add(key)
        entries.append({"path": key, "kind": kind, "sha256": digest,
                        "target": target})

    def add_file(full: Path, key: str) -> None:
        digest = sha256_file(full)
        if digest is None:
            problems.append(f"cannot read file: {key}")
            return
        add(key, "file", digest)

    def add_symlink(link: Path, key: str) -> None:
        target = Path(os.path.realpath(link))
        if not within(project, target) or not target.is_file():
            problems.append(
                f"symlink escapes the project or is not a regular file: {key}")
            return
        digest = sha256_file(target)
        if digest is None:
            problems.append(f"cannot read symlink target: {key}")
            return
        add(key, "symlink", digest, target.relative_to(project).as_posix())

    for raw in cfg.snapshot_paths:
        top = Path(os.path.normpath(project / raw))
        if not within(project, top):
            problems.append(f"snapshot path escapes the project: {raw}")
            continue
        rel_top = top.relative_to(project).as_posix()
        if top.is_symlink():
            add_symlink(top, rel_top)
        elif top.is_dir():
            for full, rel, is_link in iter_project_files(
                    top, project, cfg.exclude, problems,
                    f"snapshot path {raw!r}"):
                if is_link:
                    add_symlink(full, rel)
                else:
                    add_file(full, rel)
        elif top.is_file():
            add_file(top, rel_top)
        elif top.exists():
            problems.append(
                f"snapshot path is not a regular file or directory: {raw}")
        else:
            problems.append(f"snapshot path missing: {raw}")

    # The policy file and the checker itself are always part of the state.
    digest = sha256_file(config_path)
    if digest is None:
        problems.append("cannot read config file for fingerprint")
    else:
        add(config_path.relative_to(project).as_posix(), "file", digest)
    digest = sha256_file(check_script)
    if digest is None:
        problems.append("cannot read checker script for fingerprint")
    else:
        add(CHECKER_TAG + str(check_script), "file", digest)
    return entries, problems


def fingerprint(entries: list[dict]) -> str:
    digest = hashlib.sha256()
    for entry in sorted(entries, key=lambda e: e["path"]):
        digest.update(
            f'{entry["path"]}\0{entry["kind"]}\0{entry["sha256"]}'
            f'\0{entry.get("target", "")}\n'.encode())
    return digest.hexdigest()


# --------------------------------------------------------------------------
# command checks
# --------------------------------------------------------------------------

def _kill_group(proc: subprocess.Popen) -> None:
    try:
        if hasattr(os, "killpg"):
            os.killpg(proc.pid, signal.SIGKILL)
        else:
            proc.kill()
    except (ProcessLookupError, PermissionError, OSError):
        pass


def _read_bounded(stdout, stderr, limit: int = OUTPUT_LIMIT) -> str:
    # Read limit+1 bytes so an oversized stream is detectable even when the
    # first read returns exactly ``limit`` bytes (a bare read(limit) cannot
    # distinguish "exactly full" from "more followed").
    stdout.seek(0)
    stderr.seek(0)
    out_raw = stdout.read(limit + 1)
    err_raw = stderr.read(limit + 1)
    over = len(out_raw) > limit or len(err_raw) > limit
    text = out_raw[:limit].decode("utf-8", "replace")
    err = err_raw[:limit].decode("utf-8", "replace")
    if err:
        text += ("\n--- stderr ---\n" + err) if text else err
    if over or len(text) > limit:
        text = text[:limit] + "\n...[truncated]"
    return text


def run_command(cmd: dict, project: Path) -> dict:
    argv = list(cmd["argv"])
    if argv[0] == "{python}":
        argv[0] = sys.executable
    result = {"id": cmd["id"], "kind": "command", "status": "incomplete",
              "exitCode": None, "output": "", "detail": ""}
    cwd = Path(os.path.normpath(project / cmd["cwd"]))
    if not within(project, cwd) or not cwd.is_dir():
        result["detail"] = (
            f"cwd is missing or escapes the project: {cmd['cwd']!r}")
        return result
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        try:
            proc = subprocess.Popen(
                argv, cwd=str(cwd), stdin=subprocess.DEVNULL, stdout=out,
                stderr=err, start_new_session=True)
        except (OSError, ValueError) as exc:
            # ValueError: e.g. an argument containing a NUL byte reached
            # exec anyway; degrade to incomplete, never a traceback.
            result["detail"] = f"cannot start {argv[0]!r}: {exc}"
            return result
        try:
            code = proc.wait(timeout=cmd["timeout"])
        except subprocess.TimeoutExpired:
            _kill_group(proc)
            proc.wait()
            result["detail"] = (
                f"timed out after {cmd['timeout']:g}s (process group killed)")
            result["output"] = _read_bounded(out, err)
            return result
        except BaseException:
            _kill_group(proc)
            proc.wait()
            raise
        result["exitCode"] = code
        result["output"] = _read_bounded(out, err)
        if code == 0:
            result["status"] = "pass"
        elif code in cmd["failure_codes"]:
            result["status"] = ("fail" if cmd["severity"] == "error"
                                else "warning")
        else:
            result["detail"] = (
                f"unexpected exit code {code} "
                f"(failure_codes={cmd['failure_codes']})")
    return result


def command_finding(cmd: dict, result: dict) -> dict:
    finding = {
        "ruleId": cmd["id"],
        "severity": cmd["severity"],
        "message": f"command exited with code {result.get('exitCode')}",
        "suggestion": "inspect the command output in this report",
    }
    if cmd.get("policy"):
        finding["policy"] = cmd["policy"]
    return finding


# --------------------------------------------------------------------------
# reporting
# --------------------------------------------------------------------------

def empty_state() -> dict:
    return {
        "configPath": None,
        "fingerprintBefore": None,
        "fingerprintAfter": None,
        "stale": False,
        "fileCount": 0,
        "snapshotPaths": [],
        "scope": ("fingerprint covers selected project files (always "
                  "including the config and the checker itself); the tool "
                  "environment is not certified or cached"),
    }


def text_report(report: dict) -> str:
    lines = [
        f"status: {report['status']}",
        f"mode: {report['mode']}",
        f"project: {report['project']}",
    ]
    state = report["checkedState"]
    if state.get("fingerprintBefore"):
        lines.append(f"files: {state['fileCount']}  stale: {state['stale']}")
        # The fingerprint identifies exactly which source state was checked.
        lines.append(f"fingerprint: {state['fingerprintBefore']}")
    lines.extend(f"error: {err}" for err in report.get("errors", []))
    for check in report["checks"]:
        extra = (f" exit={check['exitCode']}"
                 if check.get("exitCode") is not None else "")
        lines.append(f"[{check['status']}] {check['kind']}:{check['id']}{extra}")
        if check.get("detail"):
            lines.append(f"  detail: {check['detail']}")
        if check.get("output"):
            shown = check["output"][:2_000]
            lines.append("  output: " + shown.replace("\n", "\n    "))
            if len(check["output"]) > 2_000:
                lines.append("  ...[output truncated in text report]")
    for finding in report["findings"]:
        location = finding.get("path") or "-"
        if finding.get("line") is not None:
            location += f":{finding['line']}"
        lines.append(
            f"finding [{finding['severity']}] {finding['ruleId']} "
            f"{location}: {finding['message']}")
        if finding.get("suggestion"):
            lines.append(f"  suggestion: {finding['suggestion']}")
        if finding.get("policy"):
            lines.append(f"  policy: {finding['policy']}")
    omitted = report.get("omittedFindings") or 0
    if omitted:
        lines.append(
            f"... {omitted} of {report.get('totalFindings')} findings "
            "omitted from this report (the status reflects all of them)")
    return "\n".join(lines)


# --------------------------------------------------------------------------
# entry point
# --------------------------------------------------------------------------

def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        prog="check.py",
        description="Bundled maintainable-code project checker (read-only).")
    parser.add_argument("--project", required=True,
                        help="project directory (explicit, required)")
    parser.add_argument("--config", default=DEFAULT_CONFIG,
                        help="config file, project-relative "
                             f"(default: {DEFAULT_CONFIG})")
    parser.add_argument("--mode", choices=MODES, default="fast")
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)

    project = Path(args.project).expanduser().resolve()
    report: dict = {
        "schemaVersion": SCHEMA_VERSION,
        "status": "incomplete",
        "mode": args.mode,
        "project": str(project),
        "checkedState": empty_state(),
        "checks": [],
        "findings": [],
        "errors": [],
    }
    try:
        if not project.is_dir():
            raise Incomplete(f"project directory not found: {project}")
        config_path = Path(args.config)
        # Normalize ``..`` components first so a path like
        # ``/abs/other/../proj/quality.toml`` cannot crash ``relative_to``
        # below; the containment check decides, not lexical shape.
        config_path = Path(os.path.normpath(
            str(config_path if config_path.is_absolute()
                else project / config_path)))
        if not within(project, config_path):
            raise Incomplete(f"config path escapes the project: {args.config}")
        config_path = config_path.resolve()
        report["checkedState"]["configPath"] = (
            config_path.relative_to(project).as_posix())
        cfg, errors = load_config(config_path)
        if errors:
            raise Incomplete("invalid config: " + "; ".join(errors))
        report["checkedState"]["snapshotPaths"] = cfg.snapshot_paths
        if not cfg.commands:
            raise Incomplete(
                "no checks selected: config has no [[commands]]")
        entries, problems = collect_snapshot(
            project, cfg, config_path, checker_path())
        if problems:
            raise Incomplete("snapshot incomplete: " + join_problems(problems))
        state = report["checkedState"]
        state["fileCount"] = len(entries)
        state["fingerprintBefore"] = fingerprint(entries)

        selected = [c for c in cfg.commands if args.mode in c["modes"]]
        if not selected:
            raise Incomplete(f"no checks selected for mode {args.mode!r}")

        interrupted = False
        for cmd in selected:
            try:
                result = run_command(cmd, project)
            except KeyboardInterrupt:
                interrupted = True
                break
            report["checks"].append(result)
            if result["status"] in ("fail", "warning"):
                report["findings"].append(command_finding(cmd, result))
        after_entries, after_problems = collect_snapshot(
            project, cfg, config_path, checker_path())
        if after_problems:
            state["stale"] = True
            report["errors"].append(
                "snapshot problems after checks: "
                + join_problems(after_problems))
        else:
            state["fingerprintAfter"] = fingerprint(after_entries)
            state["stale"] = (state["fingerprintAfter"]
                              != state["fingerprintBefore"])
        if interrupted:
            report["errors"].append("interrupted before all checks completed")

        statuses = [check["status"] for check in report["checks"]]
        if interrupted or state["stale"] or after_problems \
                or "incomplete" in statuses:
            report["status"] = "incomplete"
        elif "fail" in statuses:
            report["status"] = "failed"
        else:
            report["status"] = "passed"
    except Incomplete as exc:
        report["errors"].append(str(exc))
    except (OSError, ValueError, RuntimeError) as exc:
        # Expected environment/OS failures must degrade to an incomplete
        # report, never a traceback.
        report["status"] = "incomplete"
        report["errors"].append(f"checker error ({type(exc).__name__}): {exc}")

    # Aggregate over the full finding set, then bound what is displayed.
    report["totalFindings"] = len(report["findings"])
    report["findings"].sort(key=lambda f: (f.get("path") or "",
                                           f.get("line") or 0,
                                           f.get("ruleId") or ""))
    report["omittedFindings"] = max(0, report["totalFindings"] - MAX_FINDINGS)
    if report["omittedFindings"]:
        report["findings"] = report["findings"][:MAX_FINDINGS]
    if len(report["errors"]) > MAX_ERRORS:
        extra = len(report["errors"]) - MAX_ERRORS
        report["errors"] = report["errors"][:MAX_ERRORS]
        report["errors"].append(f"...{extra} more errors omitted")
    if args.format == "json":
        print(json.dumps(report, indent=2))
    else:
        print(text_report(report))
    return {"passed": EXIT_OK, "failed": EXIT_FAILED,
            "incomplete": EXIT_INCOMPLETE}[report["status"]]


if __name__ == "__main__":
    sys.exit(main())
