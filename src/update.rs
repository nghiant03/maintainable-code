use clap::Args;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
const MAX_METADATA_BYTES: u64 = 1_000_000;
const MAX_ARCHIVE_BYTES: u64 = 50_000_000;
const MAX_EXTRACTED_BYTES: u64 = 150_000_000;

#[derive(Args)]
pub struct Options {
    /// Only check availability. This is the default without --apply/--dry-run.
    #[arg(long, conflicts_with_all = ["apply", "dry_run"])]
    check: bool,
    /// Download, verify, and preview replacement; installed contents stay unchanged.
    #[arg(long, conflicts_with = "apply")]
    dry_run: bool,
    /// Explicitly authorize replacing the whole skill, including local edits.
    #[arg(long)]
    apply: bool,
    /// Installed skill directory; inferred when running its bin/ executable.
    #[arg(long)]
    skill_dir: Option<PathBuf>,
    /// GitHub owner/repository; defaults to the configured distribution repository.
    #[arg(long)]
    repository: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}
#[derive(Clone, Deserialize, Serialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Cache {
    repository: String,
    checked_at_unix_seconds: u64,
    release: Option<Release>,
}

/// A transport seam keeps release/checksum/archive tests entirely offline.
trait Transport {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String>;
}
struct Http {
    client: Client,
}
impl Http {
    fn new(timeout_seconds: u64) -> Result<Self, String> {
        let client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(timeout_seconds))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!("maintainable-code/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }
}
impl Transport for Http {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| error.to_string())?;
        if response.content_length().is_some_and(|size| size > limit) {
            return Err("Release response exceeds the size limit".into());
        }
        let mut bytes = Vec::new();
        response
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > limit {
            return Err("Release response exceeds the size limit".into());
        }
        Ok(bytes)
    }
}

fn repository() -> &'static str {
    option_env!("MAINTAINABLE_RELEASE_REPOSITORY").unwrap_or("nghiant03/maintainable-code")
}
fn validate_repository(repository: &str) -> Result<(), String> {
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || matches!(*part, "." | "..")
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
    {
        return Err("Repository must be a GitHub owner/name".into());
    }
    Ok(())
}
fn release_version(release: &Release) -> Result<Version, String> {
    if release.draft || release.prerelease {
        return Err("Only stable published releases are supported".into());
    }
    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .ok_or("Release tag must start with v")?,
    )
    .map_err(|error| error.to_string())?;
    if !version.pre.is_empty() {
        return Err("Prerelease versions are not supported".into());
    }
    Ok(version)
}
fn fetch_release(transport: &impl Transport, repository: &str) -> Result<Release, String> {
    validate_repository(repository)?;
    let data = transport.get(
        &format!("https://api.github.com/repos/{repository}/releases/latest"),
        MAX_METADATA_BYTES,
    )?;
    let release: Release = serde_json::from_slice(&data).map_err(|error| error.to_string())?;
    release_version(&release)?;
    Ok(release)
}
fn newer(release: &Release, current: &Version) -> Result<bool, String> {
    Ok(release_version(release)?.cmp_precedence(current).is_gt())
}
fn target() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") if cfg!(target_env = "gnu") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("windows", "x86_64") if cfg!(target_env = "msvc") => Ok("x86_64-pc-windows-msvc"),
        _ => {
            Err("No configured release target for this platform; update from a source build".into())
        }
    }
}

fn asset_url<'a>(release: &'a Release, repository: &str, name: &str) -> Result<&'a str, String> {
    let matches: Vec<_> = release
        .assets
        .iter()
        .filter(|asset| asset.name == name)
        .collect();
    if matches.len() != 1 {
        return Err(format!("Expected one release asset named {name}"));
    }
    let url = &matches[0].browser_download_url;
    let expected = format!(
        "https://github.com/{repository}/releases/download/{}/{name}",
        release.tag_name
    );
    if *url != expected {
        return Err(format!("Unexpected download URL for {name}"));
    }
    Ok(url)
}

fn verified_archive(
    transport: &impl Transport,
    release: &Release,
    repository: &str,
    target: &str,
) -> Result<(String, Vec<u8>), String> {
    let name = format!(
        "maintainable-code-v{}-{target}.tar.gz",
        release_version(release)?
    );
    let checksum_name = format!("{name}.sha256");
    let checksum = transport.get(asset_url(release, repository, &checksum_name)?, 4_096)?;
    let checksum = std::str::from_utf8(&checksum).map_err(|error| error.to_string())?;
    let fields: Vec<_> = checksum.split_whitespace().collect();
    if fields.len() != 2
        || fields[0].len() != 64
        || !fields[0].bytes().all(|byte| byte.is_ascii_hexdigit())
        || fields[1].trim_start_matches('*') != name
    {
        return Err("Invalid archive checksum file".into());
    }
    let archive = transport.get(asset_url(release, repository, &name)?, MAX_ARCHIVE_BYTES)?;
    let actual = format!("{:x}", Sha256::digest(&archive));
    if !actual.eq_ignore_ascii_case(fields[0]) {
        return Err("Archive SHA-256 mismatch; nothing will be executed or installed".into());
    }
    Ok((name, archive))
}

/// Extract only normal files/directories under the expected package root.
/// Reject links, traversal, duplicates, and oversized expanded data before execution.
fn unpack(archive: &[u8], root_name: &str, destination: &Path) -> Result<PathBuf, String> {
    let mut archive = tar::Archive::new(GzDecoder::new(archive));
    let mut total = 0_u64;
    let mut paths = std::collections::HashSet::new();
    for (count, entry) in archive
        .entries()
        .map_err(|error| error.to_string())?
        .enumerate()
    {
        if count >= 10_000 {
            return Err("Archive has too many entries".into());
        }
        let mut entry = entry.map_err(|error| error.to_string())?;
        let path = entry
            .path()
            .map_err(|error| error.to_string())?
            .into_owned();
        if path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
            || path
                .components()
                .next()
                .is_none_or(|part| part.as_os_str() != root_name)
        {
            return Err("Archive contains a path outside the expected release directory".into());
        }
        if !paths.insert(path.clone()) {
            return Err("Archive contains duplicate entries".into());
        }
        let kind = entry.header().entry_type();
        let output = destination.join(&path);
        if kind.is_dir() {
            fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        } else if kind.is_file() {
            total = total
                .checked_add(entry.size())
                .ok_or("Expanded archive size overflow")?;
            if total > MAX_EXTRACTED_BYTES {
                return Err("Expanded archive exceeds the size limit".into());
            }
            fs::create_dir_all(output.parent().ok_or("Archive entry has no parent")?)
                .map_err(|error| error.to_string())?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut file).map_err(|error| error.to_string())?;
        } else {
            return Err("Archive links and special files are refused".into());
        }
    }
    let executable = destination.join(root_name).join(if cfg!(windows) {
        "maintainable-code.exe"
    } else {
        "maintainable-code"
    });
    if !executable.is_file() {
        return Err("Archive contains no release executable".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
    }
    Ok(executable)
}

fn environment_directory(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn cache_directory() -> Option<PathBuf> {
    let base = environment_directory("MAINTAINABLE_UPDATE_CACHE_DIR")
        .or_else(|| environment_directory("XDG_CACHE_HOME"))
        .or_else(|| {
            if cfg!(windows) {
                environment_directory("LOCALAPPDATA")
            } else {
                None
            }
        })
        .or_else(|| {
            environment_directory("HOME")
                .or_else(|| environment_directory("USERPROFILE"))
                .map(|home| home.join(".cache"))
        })?;
    Some(base.join("maintainable-code"))
}
fn ensure_cache_directory(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|info| info.file_type().is_symlink()) {
            return Err("Update cache paths must not contain symlinks".into());
        }
    }
    fs::create_dir_all(path).map_err(|error| error.to_string())
}
fn cache_path(directory: &Path, repository: &str) -> PathBuf {
    directory.join(format!("{}.json", repository.replace('/', "--")))
}
fn save_cache(directory: &Path, cache: &Cache) -> Result<(), String> {
    ensure_cache_directory(directory)?;
    if fs::symlink_metadata(cache_path(directory, &cache.repository))
        .is_ok_and(|info| info.file_type().is_symlink())
    {
        return Err("Update cache files must not be symlinks".into());
    }
    let mut temporary =
        tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut temporary, cache).map_err(|error| error.to_string())?;
    temporary.flush().map_err(|error| error.to_string())?;
    temporary
        .persist(cache_path(directory, &cache.repository))
        .map_err(|error| error.to_string())?;
    Ok(())
}
fn read_cache(directory: &Path, repository: &str) -> Option<Cache> {
    let path = cache_path(directory, repository);
    let info = fs::symlink_metadata(&path).ok()?;
    if !info.is_file() || info.len() > MAX_METADATA_BYTES {
        return None;
    }
    let cache: Cache = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (cache.repository == repository).then_some(cache)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Automatic checks are advisory: failure cannot alter retrieval output/status.
/// Both successful and failed attempts are throttled; cached notices need no network.
fn advisory_release(
    directory: &Path,
    repository: &str,
    time: u64,
    fetch: impl FnOnce() -> Option<Release>,
) -> Option<Release> {
    let cached = read_cache(directory, repository);
    if cached.as_ref().is_some_and(|cache| {
        time >= cache.checked_at_unix_seconds
            && time - cache.checked_at_unix_seconds < CHECK_INTERVAL_SECONDS
    }) {
        cached.and_then(|cache| cache.release)
    } else {
        let release = fetch().or_else(|| cached.and_then(|cache| cache.release));
        let _ = save_cache(
            directory,
            &Cache {
                repository: repository.into(),
                checked_at_unix_seconds: time,
                release: release.clone(),
            },
        );
        release
    }
}

pub fn notify() {
    if std::env::var_os("MAINTAINABLE_NO_UPDATE_CHECK").is_some_and(|value| value == "1") {
        return;
    }
    let Some(directory) = cache_directory() else {
        return;
    };
    let repository = repository();
    if validate_repository(repository).is_err() || ensure_cache_directory(&directory).is_err() {
        return;
    }
    let release = advisory_release(&directory, repository, now(), || {
        Http::new(2)
            .and_then(|http| fetch_release(&http, repository))
            .ok()
    });
    if let Some(release) = release
        && Version::parse(env!("CARGO_PKG_VERSION"))
            .is_ok_and(|current| newer(&release, &current).unwrap_or(false))
    {
        eprintln!(
            "maintainable-code update available: {} -> {}. Run `maintainable-code update --check`; obtain authorization before `update --apply`.",
            env!("CARGO_PKG_VERSION"),
            release.tag_name
        );
    }
}

fn installed_skill(requested: Option<PathBuf>) -> Result<PathBuf, String> {
    let path = if let Some(path) = requested {
        path
    } else {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let bin = executable.parent().ok_or("Executable has no parent")?;
        if bin.file_name().is_none_or(|name| name != "bin") {
            return Err("Use --skill-dir /path/to/installed/maintainable-code outside an installed executable".into());
        }
        bin.parent()
            .ok_or("Executable has no skill directory")?
            .to_path_buf()
    };
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|info| info.file_type().is_symlink()) {
            return Err("Installed skill path must not contain symlinks".into());
        }
    }
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    if path
        .file_name()
        .is_none_or(|name| name != "maintainable-code")
        || !path.join("SKILL.md").is_file()
    {
        return Err(
            "--skill-dir must select an installed maintainable-code directory containing SKILL.md"
                .into(),
        );
    }
    Ok(path)
}

fn installed_version(skill: &Path, fallback: &Version) -> Result<Version, String> {
    let path = skill.join("INSTALLATION.json");
    let info = match fs::symlink_metadata(&path) {
        Ok(info) => info,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(fallback.clone()),
        Err(error) => return Err(error.to_string()),
    };
    if !info.is_file() || info.len() > 64_000 {
        return Err("Invalid installed version metadata".into());
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    Version::parse(
        metadata["version"]
            .as_str()
            .ok_or("Installed metadata has no version")?,
    )
    .map_err(|error| error.to_string())
}

pub fn run(options: Options) -> Result<(String, u8), String> {
    run_with(
        options,
        &Http::new(30)?,
        &Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| error.to_string())?,
    )
}

fn run_with(
    options: Options,
    transport: &impl Transport,
    current: &Version,
) -> Result<(String, u8), String> {
    let repository = options.repository.as_deref().unwrap_or(repository());
    let skill = if options.apply || options.dry_run || options.skill_dir.is_some() {
        Some(installed_skill(options.skill_dir.clone())?)
    } else {
        None
    };
    let current = if let Some(skill) = &skill {
        installed_version(skill, current)?
    } else {
        current.clone()
    };
    let release = fetch_release(transport, repository)?;
    let available = newer(&release, &current)?;
    let mut report = json!({"currentVersion": current.to_string(), "latestVersion": release_version(&release)?.to_string(),
        "repository": repository, "updateAvailable": available,
        "releaseUrl": format!("https://github.com/{repository}/releases/tag/{}", release.tag_name),
        "status": if available { "available" } else { "current" }, "applied": false});
    if options.apply || options.dry_run {
        if !available {
            return Ok((format!("{report}\n"), 0));
        }
        let skill = skill.ok_or("No installed skill selected")?;
        let (name, archive) = verified_archive(transport, &release, repository, target()?)?;
        let temporary = tempfile::Builder::new()
            .prefix("maintainable-update-")
            .tempdir()
            .map_err(|error| error.to_string())?;
        let executable = unpack(&archive, name.trim_end_matches(".tar.gz"), temporary.path())?;
        let version = Command::new(&executable)
            .arg("--version")
            .output()
            .map_err(|error| error.to_string())?;
        let expected = format!("maintainable-code {}", release_version(&release)?);
        if !version.status.success() || String::from_utf8_lossy(&version.stdout).trim() != expected
        {
            return Err("Downloaded executable version does not match the release tag".into());
        }
        let mut arguments = vec![
            "install".to_owned(),
            "--skills-dir".into(),
            skill
                .parent()
                .ok_or("Skill has no parent")?
                .display()
                .to_string(),
            "--update".into(),
        ];
        if options.dry_run {
            arguments.push("--dry-run".into());
        }
        if cfg!(windows) && options.apply {
            // Windows cannot remove the running old executable. Retain verified
            // staging and let the agent launch installation after this process exits.
            let staging = temporary.keep();
            report["status"] = json!("prepared");
            report["stagingDirectory"] = json!(staging);
            report["applyCommand"] = json!(
                std::iter::once(executable.display().to_string())
                    .chain(arguments)
                    .collect::<Vec<_>>()
            );
        } else {
            let output = Command::new(&executable)
                .args(arguments)
                .output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "Release installer failed; inspect recovery details: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            report["status"] = json!(if options.dry_run {
                "preview"
            } else {
                "updated"
            });
            report["applied"] = json!(options.apply);
            report["installerOutput"] = json!(String::from_utf8_lossy(&output.stdout));
        }
    }
    Ok((format!("{report}\n"), 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Mock {
        bodies: HashMap<String, Vec<u8>>,
    }
    impl Transport for Mock {
        fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
            let body = self
                .bodies
                .get(url)
                .ok_or_else(|| format!("Mock has no response for {url}"))?;
            if body.len() as u64 > limit {
                return Err("Mock response exceeds limit".into());
            }
            Ok(body.clone())
        }
    }
    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: vec![],
        }
    }
    fn archive(root: &str, file: &str, content: &[u8], kind: tar::EntryType) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(kind);
        if kind.is_symlink() {
            header.set_link_name("/outside").unwrap();
        }
        header.set_cksum();
        builder
            .append_data(&mut header, format!("{root}/{file}"), content)
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }
    fn release_mock(archive: Vec<u8>, tag: &str) -> (Release, Mock) {
        let mut release = release(tag);
        let name = format!("maintainable-code-{}-{}.tar.gz", tag, target().unwrap());
        let base = format!("https://github.com/owner/name/releases/download/{tag}");
        let archive_url = format!("{base}/{name}");
        let checksum_url = format!("{base}/{name}.sha256");
        release.assets = vec![
            Asset {
                name: name.clone(),
                browser_download_url: archive_url.clone(),
            },
            Asset {
                name: format!("{name}.sha256"),
                browser_download_url: checksum_url.clone(),
            },
        ];
        let mut bodies = HashMap::new();
        bodies.insert(archive_url, archive.clone());
        bodies.insert(
            checksum_url,
            format!("{:x}  {name}\n", Sha256::digest(&archive)).into_bytes(),
        );
        bodies.insert(
            "https://api.github.com/repos/owner/name/releases/latest".into(),
            serde_json::to_vec(&release).unwrap(),
        );
        (release, Mock { bodies })
    }

    #[test]
    fn semantic_versions_do_not_compare_lexically_or_treat_build_metadata_as_new() {
        assert!(newer(&release("v0.10.0"), &Version::parse("0.9.0").unwrap()).unwrap());
        assert!(!newer(&release("v0.1.0+other"), &Version::parse("0.1.0").unwrap()).unwrap());
        assert!(release_version(&release("v0.2.0-rc.1")).is_err());
        assert!(release_version(&release("not-a-version")).is_err());
        let mut draft = release("v1.0.0");
        draft.draft = true;
        assert!(release_version(&draft).is_err());
    }

    #[test]
    fn repository_and_asset_origins_are_constrained() {
        for repository in [
            "owner",
            "../name",
            "owner/name/extra",
            "owner/name?token=bad",
        ] {
            assert!(validate_repository(repository).is_err());
        }
        let (mut release, _) = release_mock(vec![1], "v0.2.0");
        let name = release.assets[0].name.clone();
        release.assets[0].browser_download_url = "https://evil.example/archive".into();
        assert!(asset_url(&release, "owner/name", &name).is_err());
    }

    #[test]
    fn verifies_checksum_before_unpacking_or_execution() {
        let (release, mut transport) = release_mock(vec![1, 2, 3], "v0.2.0");
        assert_eq!(
            verified_archive(&transport, &release, "owner/name", target().unwrap())
                .unwrap()
                .1,
            vec![1, 2, 3]
        );
        let archive_url = release.assets[0].browser_download_url.clone();
        transport.bodies.insert(archive_url, vec![9]);
        assert!(
            verified_archive(&transport, &release, "owner/name", target().unwrap())
                .unwrap_err()
                .contains("mismatch")
        );
    }

    #[test]
    fn rejects_wrong_checksum_filename_missing_and_duplicate_assets() {
        let (mut release, mut transport) = release_mock(vec![1], "v0.2.0");
        let checksum_url = release.assets[1].browser_download_url.clone();
        transport.bodies.insert(
            checksum_url,
            format!("{} wrong.tar.gz", "0".repeat(64)).into_bytes(),
        );
        assert!(verified_archive(&transport, &release, "owner/name", target().unwrap()).is_err());
        release.assets.push(release.assets[0].clone());
        assert!(asset_url(&release, "owner/name", &release.assets[0].name).is_err());
        assert!(asset_url(&release, "owner/name", "missing").is_err());
    }

    #[test]
    fn archives_refuse_links_other_roots_and_missing_executables() {
        let temporary = tempfile::tempdir().unwrap();
        for (root, kind) in [
            ("release", tar::EntryType::Symlink),
            ("other", tar::EntryType::Regular),
        ] {
            let data = archive(root, "maintainable-code", b"", kind);
            assert!(unpack(&data, "release", temporary.path()).is_err());
        }
        let data = archive("release", "README.md", b"docs", tar::EntryType::Regular);
        assert!(
            unpack(&data, "release", temporary.path())
                .unwrap_err()
                .contains("no release executable")
        );
    }

    #[test]
    fn invalid_tar_and_gzip_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(unpack(b"not a gzip archive", "release", temporary.path()).is_err());
    }

    #[test]
    fn cache_roundtrip_replaces_atomically_and_records_failed_checks() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().canonicalize().unwrap();
        save_cache(
            &directory,
            &Cache {
                repository: "owner/name".into(),
                checked_at_unix_seconds: 123,
                release: Some(release("v1.0.0")),
            },
        )
        .unwrap();
        assert_eq!(
            read_cache(&directory, "owner/name")
                .unwrap()
                .release
                .unwrap()
                .tag_name,
            "v1.0.0"
        );
        save_cache(
            &directory,
            &Cache {
                repository: "owner/name".into(),
                checked_at_unix_seconds: 456,
                release: None,
            },
        )
        .unwrap();
        let cache = read_cache(&directory, "owner/name").unwrap();
        assert_eq!(cache.checked_at_unix_seconds, 456);
        assert!(cache.release.is_none());
        assert!(read_cache(&directory, "other/repository").is_none());
    }

    #[test]
    fn automatic_checks_throttle_failures_and_reuse_cached_metadata() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().canonicalize().unwrap();
        let calls = std::cell::Cell::new(0);
        let failed = || {
            calls.set(calls.get() + 1);
            None
        };
        assert!(advisory_release(&directory, "owner/name", 100, failed).is_none());
        assert!(advisory_release(&directory, "owner/name", 101, failed).is_none());
        assert_eq!(calls.get(), 1);
        let latest = advisory_release(
            &directory,
            "owner/name",
            100 + CHECK_INTERVAL_SECONDS,
            || {
                calls.set(calls.get() + 1);
                Some(release("v1.0.0"))
            },
        )
        .unwrap();
        assert_eq!(latest.tag_name, "v1.0.0");
        assert_eq!(calls.get(), 2);
        assert!(
            advisory_release(
                &directory,
                "owner/name",
                101 + CHECK_INTERVAL_SECONDS,
                || panic!("fresh cache must not fetch")
            )
            .is_some()
        );
    }

    #[test]
    fn explicit_updates_do_not_downgrade_newer_installations() {
        let temporary = tempfile::tempdir().unwrap();
        let skill = temporary
            .path()
            .canonicalize()
            .unwrap()
            .join("maintainable-code");
        fs::create_dir(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "local").unwrap();
        fs::write(skill.join("INSTALLATION.json"), r#"{"version":"0.3.0"}"#).unwrap();
        let (_, transport) = release_mock(vec![], "v0.2.0");
        let (output, _) = run_with(
            Options {
                check: false,
                dry_run: false,
                apply: true,
                skill_dir: Some(skill.clone()),
                repository: Some("owner/name".into()),
            },
            &transport,
            &Version::parse("0.1.0").unwrap(),
        )
        .unwrap();
        let report: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(report["currentVersion"], "0.3.0");
        assert_eq!(report["updateAvailable"], false);
        assert_eq!(report["applied"], false);
        assert_eq!(fs::read_to_string(skill.join("SKILL.md")).unwrap(), "local");
    }

    #[test]
    fn check_only_uses_metadata_and_never_requires_installed_skill() {
        let (_, transport) = release_mock(vec![], "v0.2.0");
        let (output, code) = run_with(
            Options {
                check: true,
                dry_run: false,
                apply: false,
                skill_dir: None,
                repository: Some("owner/name".into()),
            },
            &transport,
            &Version::parse("0.1.0").unwrap(),
        )
        .unwrap();
        let report: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(code, 0);
        assert_eq!(report["status"], "available");
        assert_eq!(report["applied"], false);
    }

    #[cfg(unix)]
    #[test]
    fn preview_runs_verified_installer_without_changing_installed_contents() {
        let temporary = tempfile::tempdir().unwrap();
        let skill = temporary
            .path()
            .canonicalize()
            .unwrap()
            .join("maintainable-code");
        fs::create_dir(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "local edits").unwrap();
        let script = b"#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'maintainable-code 0.2.0'; else printf '%s\\n' \"$@\"; fi\n";
        let root = format!("maintainable-code-v0.2.0-{}", target().unwrap());
        let data = archive(&root, "maintainable-code", script, tar::EntryType::Regular);
        let (_, transport) = release_mock(data, "v0.2.0");
        let (output, _) = run_with(
            Options {
                check: false,
                dry_run: true,
                apply: false,
                skill_dir: Some(skill.clone()),
                repository: Some("owner/name".into()),
            },
            &transport,
            &Version::parse("0.1.0").unwrap(),
        )
        .unwrap();
        let report: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(report["status"], "preview");
        assert_eq!(report["applied"], false);
        assert!(
            report["installerOutput"]
                .as_str()
                .unwrap()
                .contains("--dry-run")
        );
        assert_eq!(
            fs::read_to_string(skill.join("SKILL.md")).unwrap(),
            "local edits"
        );
    }

    #[cfg(unix)]
    #[test]
    fn version_mismatch_never_invokes_installer() {
        let temporary = tempfile::tempdir().unwrap();
        let skill = temporary
            .path()
            .canonicalize()
            .unwrap()
            .join("maintainable-code");
        fs::create_dir(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "local edits").unwrap();
        let root = format!("maintainable-code-v0.2.0-{}", target().unwrap());
        let data = archive(
            &root,
            "maintainable-code",
            b"#!/bin/sh\necho 'maintainable-code 9.0.0'\n",
            tar::EntryType::Regular,
        );
        let (_, transport) = release_mock(data, "v0.2.0");
        let result = run_with(
            Options {
                check: false,
                dry_run: false,
                apply: true,
                skill_dir: Some(skill.clone()),
                repository: Some("owner/name".into()),
            },
            &transport,
            &Version::parse("0.1.0").unwrap(),
        );
        assert!(result.unwrap_err().contains("version"));
        assert_eq!(
            fs::read_to_string(skill.join("SKILL.md")).unwrap(),
            "local edits"
        );
    }
}
