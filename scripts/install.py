#!/usr/bin/env python3
"""Copy the portable skill into a documented harness discovery directory."""

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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    destination = parser.add_mutually_exclusive_group(required=True)
    destination.add_argument("--harness", choices=PROJECT_DIRS)
    destination.add_argument("--skills-dir", type=Path, help="Custom parent directory of skills")
    parser.add_argument("--scope", choices=("project", "user"), default="project")
    parser.add_argument("--project", type=Path, help="Project directory (defaults to current directory)")
    parser.add_argument("--dry-run", action="store_true")
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
        if target.exists() or target.is_symlink():
            if identical(SOURCE, target):
                print(f"Already installed (identical): {target}")
                return 0
            raise ValueError(
                f"Destination already exists: {target}. Back it up outside skill discovery "
                "directories and remove that specific directory before updating."
            )
        if args.dry_run:
            print(f"Would copy {SOURCE} to {target}")
            return 0
        target.parent.mkdir(parents=True, exist_ok=True)
        # Stage outside the skills directory, then publish the completed copy.
        with tempfile.TemporaryDirectory(prefix=".maintainable-install-", dir=target.parent.parent) as temporary:
            staged = Path(temporary) / SKILL_NAME
            shutil.copytree(SOURCE, staged, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            staged.rename(target)
        print(f"Installed: {target}")
        print("Load maintainable-code in your harness; start a new session if it is not listed.")
        return 0
    except (OSError, ValueError) as error:
        print(f"Installation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
