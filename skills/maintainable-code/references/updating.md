# Updating the installed skill

Updates replace the complete skill: executable, instructions, references, assets, provenance, and release notices. They remove obsolete files and replace local edits. No backup is retained after success. A version notification is not authorization to install software or overwrite files.

## When a notification appears

`context` and `rules` print newer-version notices on stderr. Keep that notice separate from retrieved source/rule output. Briefly tell the user an update is available, then continue the requested work using the installed version unless an update was requested. Do not interrupt an unrelated review to upgrade automatically.

Automatic checks contact the configured public GitHub repository at most once per 24 hours when the user cache is available. Requests have a 2-second timeout; network, rate-limit, missing-release, and cache failures do not fail retrieval. Successful and failed attempts are cached outside project/skill directories. Notices can repeat from cached metadata on subsequent invocations. They may be stale for up to a day and are advisory.

For strict offline/read-only operation, use `<binary> --no-update-check context ...` or `<binary> --no-update-check rules ...`. Setting `MAINTAINABLE_NO_UPDATE_CHECK=1` also disables automatic network checks, cache writes, and notices. An explicit `update` command still performs its requested network operation.

## Agent update procedure

1. Confirm the intended installation. Resolve its binary and read `INSTALLATION.json` for version/platform. Do not guess a different user/project destination. If the user has not authorized updating, explain the available version and replacement risk before obtaining permission.
2. Once network access is permitted, check the latest stable release:

   ```sh
   <binary> update --check
   ```

   Running an installed `bin/` executable allows destination inference for preview/apply. Outside an installation, supply `--skill-dir /path/to/maintainable-code`; this selects the skill folder, not its parent. A supplied installation's manifest version is compared to avoid downgrading a newer installation.

3. Preview the exact replacement:

   ```sh
   <binary> update --dry-run
   ```

   Preview downloads the platform archive and checksum from the configured release repository, verifies SHA-256, safely extracts into temporary staging, validates executable version, and runs the new installer with `--update --dry-run`. It executes downloaded software but does not alter installed contents. Inspect `installerOutput`; preserve local edits outside the installed skill if needed. Do not proceed on checksum, version, extraction, or installer errors.

4. With explicit authorization to replace the entire skill, apply:

   ```sh
   <binary> update --apply
   ```

   On Linux/macOS, the verified release executable runs outside the installation and uses the existing staged installer/rollback behavior. Neither an available-version notice nor a successful preview authorizes this command by itself.

5. **Windows requires a second step.** The running old executable cannot be removed safely. `update --apply` prepares verified staging, reports `status: "prepared"`, `applied: false`, and an `applyCommand` argument array. After that old process exits, execute that exact array as a command with correctly quoted arguments. This invokes the new release installer outside the skill. Do not treat `prepared` as a completed update. After successful installation, remove only the reported updater-owned staging directory if cleanup is desired.
6. Verify the installed binary's `--version`, inspect the updated `INSTALLATION.json`, and smoke-test `rules --rules N3`. Refresh the loaded skill or start a fresh harness session; an ongoing conversation may still contain old instructions. Report applied version, destination, verification, and any remaining recovery/cleanup issue.

## Release trust and failure handling

The default source is `nghiant03/maintainable-code` on GitHub. `--repository owner/name` explicitly selects a different public GitHub distribution; do not change the source merely to bypass a failed download. A distributor can set `MAINTAINABLE_RELEASE_REPOSITORY` at build time for default checks. Only stable `v<semver>` releases and the exact matching platform archive/checksum asset names are accepted. Runtime downloads use HTTPS.

Checksums detect corruption and mismatched assets, not publisher compromise. The archive and checksum are from the same release source, and releases are not independently signed. Updating requires trusting that source before any preview/apply execution.

Update checks do not install anything; `update` without action flags is check-only. Explicit check/download failures exit `2`, while advisory automatic failures are silent. Unsupported platforms require a source build. No published release means checks/previews cannot fetch one yet.

Extraction refuses links, traversal, duplicate paths, special files, wrong archive roots, oversized archives, and oversized expanded data. Installation still refuses linked destinations and protects other skills. Do not run concurrent updates to one destination. If publication/restoration fails, preserve and inspect any reported recovery directory; never delete it to silence an error. Downloads/verification happen before installation, so those failures leave installed contents unchanged.
