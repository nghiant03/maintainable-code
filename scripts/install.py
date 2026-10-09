#!/usr/bin/env python3
"""Install or explicitly update the portable skill in a harness discovery directory."""

import argparse
import filecmp
import os
from pathlib import Path
import shutil
import sys
import tempfile


SKILL_NAME = "maintainable-code"
SOURCE = Path(__file__).resolve().parents[1] / "skills" / SKILL_NAME
PROJECT_DIRS = {
    "shared": ".agents/skills",
    "opencode": ".opencode/skills",
    "crush": ".crush/skills",
    "claude-code": ".claude/skills",
    "codex": ".agents/skills",
}


def discovery_directory(harness, scope, project):
    if scope == "project":
        return project / PROJECT_DIRS[harness]
    home = Path.home()
    config_home = Path(os.environ.get("XDG_CONFIG_HOME") or home / ".config")
    return {
        "shared": config_home / "agents/skills",
        "opencode": config_home / "opencode/skills",
        "crush": config_home / "crush/skills",
        "claude-code": home / ".claude/skills",
        "codex": home / ".agents/skills",
    }[harness]


def package_files(directory):
    return {
        path.relative_to(directory).as_posix(): path
        for path in directory.rglob("*")
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc"
    }


def identical(source, destination):
    if not destination.is_dir() or destination.is_symlink():
        return False
    source_files = package_files(source)
    destination_files = package_files(destination)
    return source_files.keys() == destination_files.keys() and all(
        not destination_files[name].is_symlink()
        and filecmp.cmp(path, destination_files[name], shallow=False)
        for name, path in source_files.items()
    )


def validate_update_target(target):
    if not target.is_dir():
        raise ValueError(f"Update destination must be a directory: {target}")
    for path in (target, *target.parents, *target.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"Refusing to update a symlinked path: {path}")
    if not (target / "SKILL.md").is_file():
        raise ValueError(f"Update destination has no SKILL.md: {target}")


def report_changes(source, target):
    old = package_files(target)
    new = package_files(source)
    for name in sorted(old.keys() | new.keys()):
        if name not in old:
            print(f"  add: {name}")
        elif name not in new:
            print(f"  remove: {name}")
        elif not filecmp.cmp(old[name], new[name], shallow=False):
            print(f"  replace: {name}")


class UpdateRollbackError(OSError):
    """Publication and restoration both failed; preserve the recovery directory."""


def publish_update(staged, target, previous):
    """Restore the previous installation if publication fails."""
    target.rename(previous)
    try:
        staged.rename(target)
    except OSError as publication_error:
        try:
            previous.rename(target)
        except OSError as rollback_error:
            raise UpdateRollbackError(
                f"Update failed: {publication_error}; rollback failed: {rollback_error}. "
                f"Previous installation remains at {previous}"
            ) from rollback_error
        raise OSError(f"Update failed; previous installation restored: {publication_error}") from publication_error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    destination = parser.add_mutually_exclusive_group(required=True)
    destination.add_argument("--harness", choices=PROJECT_DIRS)
    destination.add_argument("--skills-dir", type=Path, help="Custom parent directory of skills")
    parser.add_argument("--scope", choices=("project", "user"), default="project")
    parser.add_argument("--project", type=Path, help="Project directory (defaults to current directory)")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--update", action="store_true", help="Replace an existing skill, including local edits; no backup is retained")
    args = parser.parse_args()
    if args.project is not None and (args.scope == "user" or args.skills_dir):
        parser.error("--project is only used for project-scoped harness installs")
    project = (args.project or Path.cwd()).expanduser().resolve()
    if args.harness and args.scope == "project" and not project.is_dir():
        parser.error(f"Project directory does not exist: {project}")
    parent = args.skills_dir or discovery_directory(args.harness, args.scope, project)
    target = parent.expanduser().absolute() / SKILL_NAME
    try:
        if not (SOURCE / "SKILL.md").is_file():
            raise ValueError(f"Skill source missing: {SOURCE}")
        if target.resolve() != SOURCE and target.resolve().is_relative_to(SOURCE):
            raise ValueError("Install destination must not be inside the source skill")
        if args.update and SOURCE.is_relative_to(target.resolve()):
            raise ValueError("Update destination must not contain the source skill")
        updating = False
        if target.exists() or target.is_symlink():
            if args.update:
                validate_update_target(target)
            if identical(SOURCE, target):
                print(f"Already installed (identical): {target}")
                return 0
            if not args.update:
                raise ValueError(f"Destination already exists: {target}. Use --update to replace it.")
            updating = True
            print(f"Update destination: {target}")
            report_changes(SOURCE, target)
        if args.dry_run:
            if updating:
                print(f"Would replace with {SOURCE}; no backup is retained")
            else:
                print(f"Would copy {SOURCE} to {target}")
            return 0
        target.parent.mkdir(parents=True, exist_ok=True)
        # Stage outside the skills directory, then publish the completed copy.
        temporary = tempfile.mkdtemp(prefix=".maintainable-install-", dir=target.parent.parent)
        preserve_recovery = False
        try:
            staged = Path(temporary) / SKILL_NAME
            shutil.copytree(SOURCE, staged, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            if updating:
                publish_update(staged, target, Path(temporary) / "previous")
            else:
                staged.rename(target)
        except UpdateRollbackError:
            preserve_recovery = True
            raise
        finally:
            if not preserve_recovery:
                shutil.rmtree(temporary)
        print(f"{'Updated' if updating else 'Installed'}: {target}")
        print("Load maintainable-code in your harness; start a new session if it is not listed.")
        return 0
    except (OSError, ValueError) as error:
        print(f"Installation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
