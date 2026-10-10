# Installation and distribution

## Release archives

Each archive contains a platform-specific `maintainable-code` executable (`.exe` on Windows), README, installation instructions, `THIRD_PARTY.json`, and dependency license notices. Skill resources and language grammars are compiled into the executable. There are no Python requirements, runtime grammar downloads, or harness plugins.

The release matrix currently covers:

| Target | Build environment |
|---|---|
| Linux x86-64, GNU libc | Ubuntu 24.04; older libc environments are not guaranteed |
| macOS ARM64 | macOS 15 |
| macOS x86-64 | macOS 15 Intel |
| Windows x86-64 | MSVC on Windows runner |

Other targets require a source build. Cross-platform jobs are configured in CI; local verification only establishes the tested host's behavior. No release has been published merely by adding this workflow.

Verify the archive's SHA-256 checksum before extraction. On Linux use `sha256sum -c <archive>.sha256`; on macOS use `shasum -a 256 -c <archive>.sha256`; on Windows compare `Get-FileHash <archive> -Algorithm SHA256` with the checksum file. Checksums shipped alongside artifacts do not provide independent publisher authentication.

## Discovery paths

The `install` subcommand exports `SKILL.md`, references, assets, this platform's executable under `bin/`, and `INSTALLATION.json`. It also preserves adjacent release dependency notices/inventory and any project license. The manifest records package version, source revision when supplied by the release build, OS, and architecture. Local builds report unknown revision rather than claiming a clean source commit; an unpackaged build has no adjacent release notices, so package it before redistribution.

| `--harness` | Project scope | User scope |
|---|---|---|
| `shared` | `.agents/skills/maintainable-code` | `$XDG_CONFIG_HOME/agents/skills/maintainable-code` |
| `opencode` | `.opencode/skills/maintainable-code` | `$XDG_CONFIG_HOME/opencode/skills/maintainable-code` |
| `crush` | `.crush/skills/maintainable-code` | `$XDG_CONFIG_HOME/crush/skills/maintainable-code` |
| `claude-code` | `.claude/skills/maintainable-code` | `~/.claude/skills/maintainable-code` |
| `codex` | `.agents/skills/maintainable-code` | `~/.agents/skills/maintainable-code` |

`XDG_CONFIG_HOME` falls back to `~/.config`. Home comes from `HOME`, then `USERPROFILE`. Use `--skills-dir` when your harness uses a different discovery path. Discovery support remains the harness's responsibility.

```sh
./maintainable-code install --harness shared --scope user --dry-run
./maintainable-code install --harness shared --scope user
./maintainable-code install --harness claude-code --project /path/to/project
./maintainable-code install --skills-dir /path/to/skills
```

Project scope is the default; its directory must already exist. `--project` cannot be combined with user scope or a custom skills directory. The installer refuses linked destination/parent paths. Start a fresh harness session if the skill is not listed.

## Updates and removal

An identical installation is a no-op. Differing contents require explicit `--update`. Preview changes using the desired new release executable from outside the installed directory:

```sh
./maintainable-code install --harness shared --scope user --update --dry-run
./maintainable-code install --harness shared --scope user --update
```

Updates replace the entire installed skill, including edits, extra files, and obsolete Python helpers. No backup remains after success; save local edits first. Other skills are untouched. Existing update destinations must be regular directories containing `SKILL.md`; linked paths and special files are refused.

Installation stages the complete package in a hidden `.maintainable-install-*` directory inside the skills directory. This keeps staging and rollback on the destination filesystem and requires no write access to the skills directory's parent. A harness that recursively scans hidden directories may discover the staged skill or backup before cleanup; hidden names do not guarantee discovery isolation.

Failed publication restores the previous directory. If restoration also fails, the temporary recovery directory is retained and reported. Publication uses directory renames, not an atomic exchange. Interruption and concurrent updates can require manual recovery; never update the same destination concurrently. The `install --update` operation is local: the installer does not fetch releases. The separate `update` command downloads and verifies a release before invoking that installer.

To uninstall, remove only the installed `maintainable-code` folder. Remove a custom discovery entry only if it exists exclusively for this skill.

## Agent-driven release updates

An installed executable can use `update --check`, `update --dry-run`, and explicitly authorized `update --apply`. Outside an installation, supply `--skill-dir /path/to/maintainable-code`. The updater fetches the latest stable GitHub release, chooses the host platform archive, verifies its checksum, rejects unsafe extraction, validates executable version, and invokes the new installer outside the destination. A supplied installation manifest is compared so a newer target is not downgraded by an older invoking binary.

Windows apply prepares verified staging and returns an `applyCommand` array instead of replacing a running executable. Run that command after the old process exits, then verify installed version and refresh the harness session. See [the complete agent update procedure](../skills/maintainable-code/references/updating.md), including permission, backup, trust, and recovery obligations.

Automatic metadata checks run only on `context`/`rules` invocations. Newer versions are reported on stderr; stdout and retrieval status are unchanged. Metadata attempts are throttled for 24 hours using a user cache. Requests time out after 2 seconds; advisory errors are silent. `--no-update-check` or `MAINTAINABLE_NO_UPDATE_CHECK=1` disables automatic checks, cache writes, and notices, but does not suppress an explicitly requested update command.

Cache location is `MAINTAINABLE_UPDATE_CACHE_DIR`, then `XDG_CACHE_HOME`, then Windows `LOCALAPPDATA`, then home `.cache`, with a `maintainable-code` subdirectory. Empty/relative environment paths are ignored. Defaults are outside the project/skill; an explicit absolute override chooses a different location. `MAINTAINABLE_RELEASE_REPOSITORY` can select the default public GitHub repository at build time; default builds use `nghiant03/maintainable-code`. There is no private-repository token configuration or Gitea update source in this version. An unavailable cache disables notifications if safe caching cannot be established. A failed saved cache can cause a retry on later invocations.

## Vendoring and team usage

Select an immutable release/version and install into the project's discovery directory. Record the approved archive checksum and inspect `INSTALLATION.json`. The exported executable is platform-specific: do not assume a Linux copy works for macOS/Windows teammates. Teams can keep platform binaries out of Git and provision the appropriate archive during setup, or use per-user installs while vendoring the reviewed text resources separately. The skill's binary invocation requires the matching platform executable.

Keep project vocabulary, architecture boundaries, and check commands in project instructions rather than patching vendored skill resources. Run project checks directly in CI; `context` and `rules` are navigation tools, not quality gates.

## Release maintenance

Bump the package version, run formatting/lint/tests, build the executable, and package it with `scripts/package-release.sh`. Its dependency inventory and license notice collection come from locked Cargo metadata. Resolve missing notices and select the project's own license before public redistribution.

The tag workflow tests each target, builds locked release binaries, packages notices/checksums, and publishes artifacts only after all builds pass. The tag must match the package version. Manual workflow dispatch produces artifacts without creating a release. Creating these files does not publish anything until a maintainer runs or triggers the workflow.
