# Installation across harnesses

## Discovery paths

Each destination below contains a `maintainable-code/` folder with `SKILL.md`, `scripts/`, `references/`, and `assets/`.

| Installer `--harness` | Project scope  | User scope |
|---|---|---|
| `shared` | `.agents/skills/maintainable-code` | `$XDG_CONFIG_HOME/agents/skills/maintainable-code` |
| `opencode` | `.opencode/skills/maintainable-code` | `$XDG_CONFIG_HOME/opencode/skills/maintainable-code` |
| `crush` | `.crush/skills/maintainable-code` | `$XDG_CONFIG_HOME/crush/skills/maintainable-code` |
| `claude-code` | `.claude/skills/maintainable-code` | `~/.claude/skills/maintainable-code` |
| `codex` | `.agents/skills/maintainable-code` | `~/.agents/skills/maintainable-code` |


## Quick commands

From this checkout:

```sh
python3 scripts/install.py --harness shared --scope user --dry-run
python3 scripts/install.py --harness shared --scope user
python3 scripts/install.py --harness claude-code --project /path/to/project
```

Project installs use the same relative directories. For nonstandard home/config paths use an explicit skills directory:

```sh
python3 scripts/install.py --skills-dir /path/to/skills
```


## Loading and invocation

| Harness | Explicit use |
|---|---|
| OpenCode V2 | Mention `@maintainable-code` in a prompt, for example `@maintainable-code review this diff`. |
| Crush v0.97.1 | Ask to use `maintainable-code`, or select `user:maintainable-code` / `project:maintainable-code` in the command palette. |
| Claude Code | `/maintainable-code review this diff` |
| Codex | `$maintainable-code review this diff` |


## Use in a project

The skill reads existing code, instructions, and tooling, then runs the project's existing checks directly.

`quality.toml` is needed if you choose to use the bundled standalone checker. To set that up explicitly, adapt `assets/quality.toml.example` to the project's installed check-only tools and confirmed architecture boundaries.

Use the instruction mechanism your harness loads.

## Updates and removal

Re-running an identical install is a no-op. If contents differ, the installer refuses to replace them unless you supply `--update`. From the desired version of this checkout or release:

```sh
python3 scripts/install.py --harness shared --scope user --update --dry-run
python3 scripts/install.py --harness shared --scope user --update
```

Use the same harness, scope, and project or custom skills directory as the original installation. The dry run lists added, removed, and replaced package files without writing anything. `--update` also installs the skill if it is missing.

`--update` explicitly authorizes replacing the entire installed skill, including local edits and extra files. No backup is retained after success. Save any edits you want to keep before updating. Other skills are left untouched. Symlinks in the existing installation or its parent paths are refused; update a regular installation instead.

The installer stages the new copy before moving the old one aside temporarily, outside the selected skills directory. If publication fails, it restores the old installation. If restoration also fails, it preserves the temporary recovery directory and reports its path. Replacement uses two renames, not an atomic directory exchange; interruption or simultaneous installers can require manual recovery. Do not run concurrent updates to the same destination.

Start a fresh session if needed and rerun the project's checks. The installer updates from local package contents; it does not download releases or update automatically.

To uninstall, remove only the installed `maintainable-code` directory. If you added a custom skill source exclusively for this bundle, remove that specific source entry as well.

## Team and CI usage

Vendor a reviewed version of `skills/maintainable-code`, or obtain this package from an immutable revision/release in your own distribution. No published package or remote installer URL is assumed by these instructions.

For a vendored project installation:

```sh
python3 .agents/skills/maintainable-code/scripts/check.py --project . --mode full --format json
```
