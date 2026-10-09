# Checker contract

## Run

Requires Python 3.11+ (standard library only). From any working directory:

```sh
python3 /path/to/maintainable-code/scripts/check.py --project /path/to/repository --mode full --format json
```

`--project` is required. `--config` defaults to `quality.toml` within that project. `--mode` defaults to `fast`, `--format` defaults to `text`. This policy is required only for the standalone checker; the skill can run existing project checks directly without it. Create policy only when the user explicitly requests checker setup.

## Policy

The top-level keys are `version`, `snapshot`, and `commands`. Unknown keys are errors so misspelled settings cannot silently disable checks. See [the template](../assets/quality.toml.example).

### Commands

```toml
version = 1

[[commands]]
id = "tests"
argv = ["{python}", "-B", "-m", "unittest", "discover", "-s", "tests"]
modes = ["full"]
cwd = "."
timeout = 120
severity = "error"
failure_codes = [1]
policy = "CONTRIBUTING.md#tests"
```

| Field | Meaning |
|---|---|
| `id` | Stable, unique check identifier |
| `argv` | Nonempty argument array, executed without a shell |
| `modes` | Default `["fast", "full"]`; `["full"]` selects expensive checks only in full mode |
| `cwd` | Project-relative working directory; default `.` |
| `timeout` | Positive timeout in seconds; default 120 |
| `severity` | `error` (default) or `warning` for ordinary check violations |
| `failure_codes` | Nonzero exit codes indicating violations; default `[1]` |
| `policy` | Optional reference to the repository's agreement/rationale |

`{python}` as the first argument selects the interpreter running the checker. Other arguments have no shell expansion: `*`, `$VAR`, pipes, and semicolons are literal. Use an existing repository script for multi-step checks. Commands run sequentially with closed stdin and bounded diagnostic capture.

Use repository-installed tools and pinned dependencies. Choose commands that verify rather than fix, the runner cannot make an arbitrary command read-only. It does not install dependencies or add formatter fix flags. No built-in tool autodetection or baseline mutation occurs.

Exit `0` from a command passes. Codes in `failure_codes` are violations. Other codes, launch errors, signals, and timeouts mean incomplete verification. Configure tool-specific failure codes intentionally: some test tools use exit `2` for test failure, whereas others use it for invocation errors. 

A launched program can use the same exit code for an internal setup error and a genuine violation. The runner cannot infer the difference from arbitrary log text. For such tools, use a check-only wrapper that normalizes exit codes, or an availability probe with `failure_codes = []`. For example, `argv = ["{python}", "-m", "ruff", "--version"]` makes a missing Python module incomplete even though the Python executable itself launched successfully. Use existing CI/environment checks when they already provide that guarantee.

Full mode includes fast checks; fast-only commands are not supported. An empty selected check set is incomplete, never green by default.

### Architecture checks

```toml
[[commands]]
id = "ARCH-DOMAIN"
argv = ["<architecture-check-executable>", "<argument>"]
severity = "error"
policy = "docs/architecture.md#dependency-direction"
```

Import boundaries and dependency cycles are checked by repository tools through `commands`, in any project language. Keep boundary definitions in those tools' existing configuration. The command owns source selection, module resolution, and diagnostics; snapshot exclusions do not change its scope.

The checker does not parse source code or implement language-specific rules. Python 3.11+ is its runtime requirement, not a restriction on the project being checked.

## Source state and scope

```toml
[snapshot]
paths = ["."]
exclude = [".venv", "node_modules", "dist", "build", ".coverage"]
```

Snapshots include file contents and paths, not just a Git commit. They cover relevant new/untracked files, and changes to names or deletions affect the fingerprint. Git is not required. Policy and checker contents are included even when they are outside the selected source paths. Select paths that cover sources, tests, manifests, lockfiles, and check configuration.

Before/after snapshots detect differing state during verification and report it as incomplete/stale. Later edits invalidate the result. This is not an atomic snapshot: change-and-revert races are not detectable. A content fingerprint is not a cache key for the complete execution environment; installed tool versions, environment variables, external services, and ignored dependencies may change independently. The runner does not cache results. Use pinned environments in CI.

Exclude generated/cache output that commands legitimately produce. Defaults are `.git`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, and `.crush/logs`; configured exclusions add to these. A bare name matches that file/directory at any depth. A slash-separated path matches the project-relative path and descendants. These are literal names/paths, not glob patterns. Project-specific outputs and dependency directories should be explicit. Exclusions affect fingerprinting only; configured commands use their own scope. Do not exclude relevant source merely to hide a stale result. Keep JSON output outside the snapshotted paths when redirecting it, or explicitly exclude the report file as generated output.

Paths must stay within the explicit project. Symlink handling is conservative; unsupported linked directories or paths outside the project result in incomplete verification instead of silently checking a different checkout. Inspect your scope when working with linked monorepos.

## Results

| Exit | Aggregate `status` | Meaning |
|---|---|---|
| `0` | `passed` | Selected checks passed; advisory violations may remain |
| `1` | `failed` | At least one error-severity violation |
| `2` | `incomplete` | Missing/invalid policy, no selected checks, unavailable tools, timeout, scope/snapshot error, or stale source state |

Incomplete takes precedence over failed in the aggregate, while individual failures remain visible. JSON includes `schemaVersion`, `status`, `mode`, `project`, `checkedState`, `checks`, `findings`, and `errors`. `checkedState` records `configPath`, `fingerprintBefore`, `fingerprintAfter`, `stale`, `fileCount`, `snapshotPaths`, and a scope statement. Individual check statuses are `pass`, `fail`, `warning`, or `incomplete`.

Findings use `ruleId` (the command ID), `severity`, `message`, `suggestion`, and an optional `policy` reference. Command output is retained as bounded diagnostic text; the runner does not parse it into source locations. At most 200 findings are displayed; `totalFindings` and `omittedFindings` preserve counts, and the aggregate result considers all findings.

Findings are ordered consistently for the same inputs. External commands can themselves be nondeterministic. Always inspect the selected mode and scope; a fast pass does not certify full verification.

## Deliberate limits

- No universal naming bans, line-count gates, or maintainability score.
- No semantic domain inference, general multi-language parser, or bundled cycle detector.
- No baseline format in this version. Use existing repository tools if accepted debt needs explicit baselining.
- No automatic hooks or harness adapters. Invoke the same script from agents, developers, or CI.
