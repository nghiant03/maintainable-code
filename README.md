# Maintainable Code

A portable coding-agent skill for naming, comments, API contracts, and design review, distributed with a self-contained Rust CLI. The executable bundles skill resources and Tree-sitter grammars. Users need neither Python nor Rust installed. Retrieval works offline; automatic version checks can be disabled.

## Install a release

Download the archive matching your OS/architecture from this repository's Releases page. Verify its `.sha256` checksum, then extract it. No published release is assumed until the release workflow has run.

From the extracted directory:

```sh
./maintainable-code install --harness shared --scope user --dry-run
./maintainable-code install --harness shared --scope user
```

On Windows use `maintainable-code.exe`. Other harness choices are `opencode`, `crush`, `claude-code`, and `codex`. Omit `--scope user` for the current project, or use `--project /path/to/project`. `--skills-dir /path/to/skills` selects a custom parent directory.

Installation exports the complete skill, copies the executable into its `bin/` directory, and writes version/platform provenance in `INSTALLATION.json`. The installer does not download releases or install dependencies. Install to one discovery location to avoid competing copies.

For updates, run the **new release executable outside the installed skill**:

```sh
./maintainable-code install --harness shared --scope user --update --dry-run
./maintainable-code install --harness shared --scope user --update
```

Without `--update`, differing installations are protected. With it, the entire old skill is replaced, including local edits and obsolete files; no backup remains after success. See [installation and distribution](docs/installation.md).

An installed binary can check, preview, or apply a newer release directly:

```sh
<installed-binary> update --check
<installed-binary> update --dry-run
<installed-binary> update --apply
```

Check-only is the default. Preview/apply verify the platform archive's SHA-256 and executable version before invoking its installer. Applying requires authorization to replace local edits. Windows prepares a verified installer and reports a follow-up command to run after the old process exits. See [the agent update procedure](skills/maintainable-code/references/updating.md).

`context` and `rules` announce newer versions on stderr using a daily cached GitHub check with a 2-second request timeout. Failures never fail retrieval; stdout stays unchanged. Use `--no-update-check` or `MAINTAINABLE_NO_UPDATE_CHECK=1` for no automatic networking/cache writes. Default releases come from `nghiant03/maintainable-code`; explicit updates support `--repository owner/name`. No source code is sent during checks.

## Use

Ask your agent:

> Use maintainable-code to review this repository's naming and design. Reuse its existing checks.

The skill covers naming/API, comment correctness and contract coverage, comment readability, and design/responsibilities. Concrete [naming](skills/maintainable-code/references/naming-principles.md), [commenting](skills/maintainable-code/references/commenting-principles.md), and [design](skills/maintainable-code/references/design-principles.md) rules define evidence, finding conditions, corrections, and exceptions. Reviews report inspected scope; passing tests alone does not establish semantic coverage.

The CLI can also be invoked directly:

```sh
./maintainable-code rules --rules N3 C2 D7
./maintainable-code rules --dimension comments
./maintainable-code context --project . --mode symbols --symbol '*lookup*'
./maintainable-code context --project . --mode comments --all
```

`context` retrieves selected declarations or actual comments, not violations or a resolved symbol index. Optional file/directory paths narrow discovery; no paths selects the project. Output includes locations, omission counts, skipped paths, and errors. Default output budgets keep context small; `--all` retrieves full records/text within supported scope.

Symbols/comments support Python, JS/JSX, TS/TSX, Go, Rust, C/C++, Java, C#, Ruby, and Bash. Comments also support HTML, CSS, YAML, and TOML; HTML script/style bodies are parsed as JS/CSS. Python docstrings are returned as source literals. All input must be UTF-8. See [retrieval contracts and limitations](skills/maintainable-code/references/review-helpers.md).

## Build and verify

Contributors need a current stable Rust toolchain and a C compiler for the bundled grammars. Cargo downloads build dependencies; the resulting executable operates offline.

```sh
cargo build --release --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The binary is `target/release/maintainable-code` (`.exe` on Windows). Tests use isolated projects/installations, exercise every supported language, and run the installed executable from another working directory without touching real harness configuration.

## Distribution

Versioned archives contain one platform-specific executable, instructions, dependency license notices, and a dependency inventory. The executable embeds all skill files. The release workflow builds Linux x86-64, macOS ARM64/x86-64, and Windows x86-64 archives and their SHA-256 checksums. Tagged releases require the tag to match `Cargo.toml`'s version. Manual workflow runs create downloadable build artifacts without publishing a release.

For a local host build, packaging requires Bash, Cargo, `jq`, `tar`, and a SHA-256 utility:

```sh
bash scripts/package-release.sh target/release/maintainable-code x86_64-unknown-linux-gnu dist
```

Use the actual host target triple. This packages an already-built executable, not a cross-compiler. Archives are not signed; checksums detect corruption but are not an independent authenticity guarantee. The project's own license has not yet been selected; establish it before public redistribution.

## Contents

```text
src/                         # context extraction, rule retrieval, safe installation
skills/maintainable-code/     # embedded instructions, references, assets
tests/cli.rs                 # native CLI/integration regression tests
scripts/package-release.sh   # contributor-only archive packaging
Cargo.toml / Cargo.lock      # package version and locked build dependencies
.github/workflows/           # cross-platform verification and release builds
```
