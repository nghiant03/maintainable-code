# Maintainable Code

A portable coding-agent skill for naming, design review, repository context initialization, and deterministic quality checks.

## Install

Requires **Python 3.11+** for installation/checking, plus whatever check tools your project uses. Run these commands from this checkout.

Install into a harness's own skills directory:

```sh
python3 scripts/install.py --harness claude-code --scope user
python3 scripts/install.py --harness opencode --scope user
python3 scripts/install.py --harness crush --scope user
python3 scripts/install.py --harness codex --scope user
```

Or into `.agents/skills`, the standard Agent Skills location:

```sh
python3 scripts/install.py --harness shared --scope user
```

Append `--project /path/to/project` to any command for a single project. Install to one location to avoid competing copies; add `--dry-run` to print the destination.

To replace an installed version, run the installer from the desired checkout or release:

```sh
python3 scripts/install.py --harness opencode --scope user --update --dry-run
python3 scripts/install.py --harness opencode --scope user --update
```

`--update` replaces existing contents, including local edits. It retains no backup after success. Preview changes first; without `--update`, differing installations are left untouched.

See [Installation Details](docs/installation.md) for paths, invocation, custom locations, updates, and CI.

## Use

After installation, ask your agent:

> Use maintainable-code to review this repository's naming and design. Reuse its existing checks.

A general maintainability review covers naming/API contracts, comments/docstrings, and design/responsibilities. Comment review has separate correctness/contract-coverage and clarity/subtraction passes. Reviews and implementations report inspected scope and results for each dimension before claiming completion; explicitly narrow requests can exclude dimensions. Passing checks alone does not establish semantic review coverage.

Other requests:

- “Review this diff with maintainable-code; focus on API naming and side effects.”
- “Refactor this module with maintainable-code while preserving its public contract.”
- “Run maintainable-code checks and report any incomplete verification.”
- “Update our repository context to reflect the agreed new billing boundary.”

The skill does not require `quality.toml`; it can run existing checks directly. Create that file only when explicitly setting up the bundled checker. Initialization fills useful gaps in the project's context.

## Standalone checker

The optional standalone checker requires a `quality.toml` policy. Skill activation is not required:

```sh
python3 skills/maintainable-code/scripts/check.py --project /path/to/project --mode fast
python3 skills/maintainable-code/scripts/check.py --project /path/to/project --mode full --format json
```

| Exit | Meaning |
|---|---|
| `0` | Selected checks passed; warnings may remain |
| `1` | Rule or test failures |
| `2` | Verification incomplete, including missing tools, invalid policy, timeouts, or changed source state |

The checker runs repository-configured commands and fingerprints selected files regardless of language. Repository tools handle naming conventions, types, import boundaries, dependency cycles, and behavioral tests. The checker is implemented in Python but does not parse project source code. The meaning of names and the quality of a design still need contextual review.

See [checker configuration](skills/maintainable-code/references/checker.md) and [the example policy](skills/maintainable-code/assets/quality.toml.example).

## Contents

```text
skills/maintainable-code/
├── SKILL.md
├── scripts/check.py
├── references/             # Naming, design, checker contract
└── assets/                 # Optional repository context and policy templates
scripts/install.py
docs/installation.md
tests/                     # Checker/installer regression tests
quality.toml               # This package's own verification policy
```

## Verify this package

```sh
python3 skills/maintainable-code/scripts/check.py --project . --mode full
```

On Linux, the tests use temporary projects and installation directories. They exercise exit-code handling, source-state changes, delegated architecture checks, and installation without touching your real harness configuration.

## Design and provenance

- **Skill:** review guidance, initialization workflow, reusable checker.
- **Repository:** commands, vocabulary, dependency policy, explicit exceptions.
- **Harness:** loading and execution. No hook/plugin adapter is needed.
- **CI:** run a pinned copy of the same checker for reproducible enforcement.
