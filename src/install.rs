use crate::resources;
use clap::{Args, ValueEnum};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, ValueEnum)]
enum Harness {
    Shared,
    Opencode,
    Crush,
    ClaudeCode,
    Codex,
}
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Scope {
    Project,
    User,
}

#[derive(Args)]
#[command(group(clap::ArgGroup::new("destination").required(true).args(["harness", "skills_dir"])))]
pub struct Options {
    #[arg(long, value_enum)]
    harness: Option<Harness>,
    /// Custom parent directory of skills.
    #[arg(long)]
    skills_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "project")]
    scope: Scope,
    /// Existing project directory; defaults to current directory.
    #[arg(long)]
    project: Option<PathBuf>,
    #[arg(long)]
    dry_run: bool,
    /// Replace the entire existing skill, including edits; no backup after success.
    #[arg(long)]
    update: bool,
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Cannot determine home directory; use --skills-dir".into())
}

fn expand(path: PathBuf) -> Result<PathBuf, String> {
    let text = path.to_string_lossy();
    if text == "~" {
        return home();
    }
    if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
        return Ok(home()?.join(rest));
    }
    Ok(path)
}

fn destination(options: &Options) -> Result<PathBuf, String> {
    if options.project.is_some() && (options.scope == Scope::User || options.skills_dir.is_some()) {
        return Err("--project is only used for project-scoped harness installs".into());
    }
    let parent = if let Some(path) = &options.skills_dir {
        expand(path.clone())?
    } else {
        let harness = options.harness.ok_or("Select --harness or --skills-dir")?;
        let directory = match harness {
            Harness::Shared | Harness::Codex => ".agents/skills",
            Harness::Opencode => ".opencode/skills",
            Harness::Crush => ".crush/skills",
            Harness::ClaudeCode => ".claude/skills",
        };
        if options.scope == Scope::Project {
            let project = expand(
                options
                    .project
                    .clone()
                    .unwrap_or(std::env::current_dir().map_err(|error| error.to_string())?),
            )?;
            if !project.is_dir() {
                return Err(format!(
                    "Project directory does not exist: {}",
                    project.display()
                ));
            }
            project.join(directory)
        } else {
            let home = home()?;
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"));
            match harness {
                Harness::Shared => config.join("agents/skills"),
                Harness::Opencode => config.join("opencode/skills"),
                Harness::Crush => config.join("crush/skills"),
                Harness::ClaudeCode => home.join(".claude/skills"),
                Harness::Codex => home.join(".agents/skills"),
            }
        }
    };
    let parent = if parent.is_absolute() {
        parent
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(parent)
    };
    // Normalize parent components without creating it or following any symlink.
    let mut normalized = PathBuf::new();
    for component in parent.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized.join("maintainable-code"))
}

fn reject_links(target: &Path) -> Result<(), String> {
    for path in target.ancestors() {
        match fs::symlink_metadata(path) {
            Ok(info) if info.file_type().is_symlink() => {
                return Err(format!("Refusing a symlinked path: {}", path.display()));
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(error.to_string());
            }
            _ => {}
        }
    }
    Ok(())
}

fn installed_files(target: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    for entry in WalkDir::new(target).follow_links(false) {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "Refusing a symlinked installation: {}",
                entry.path().display()
            ));
        }
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(format!(
                "Not a regular installed file: {}",
                entry.path().display()
            ));
        }
        files.insert(
            entry
                .path()
                .strip_prefix(target)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
            fs::read(entry.path()).map_err(|error| error.to_string())?,
        );
    }
    Ok(files)
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "bin/maintainable-code.exe"
    } else {
        "bin/maintainable-code"
    }
}

struct PublicationError {
    message: String,
    recovery_required: bool,
}

/// The rename seam lets tests exercise failure and recovery without permissions
/// tricks. Only failure to restore an old installation requires retaining staging.
fn publish(
    staged: &Path,
    target: &Path,
    previous: &Path,
    updating: bool,
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), PublicationError> {
    if updating {
        rename(target, previous).map_err(|error| PublicationError {
            message: error.to_string(),
            recovery_required: false,
        })?;
    }
    if let Err(publication) = rename(staged, target) {
        if updating && let Err(restoration) = rename(previous, target) {
            return Err(PublicationError {
                message: format!(
                    "Publication failed: {publication}; restoration failed: {restoration}"
                ),
                recovery_required: true,
            });
        }
        return Err(PublicationError {
            message: format!("Publication failed; previous state restored: {publication}"),
            recovery_required: false,
        });
    }
    Ok(())
}

pub fn run(options: Options) -> Result<(String, u8), String> {
    let target = destination(&options)?;
    reject_links(&target)?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut expected = resources::package_files();
    expected.insert(
        executable_name().into(),
        fs::read(&executable).map_err(|error| error.to_string())?,
    );
    // Preserve release notices when exporting the executable for vendoring.
    // Installed binaries live under bin/, while extracted releases keep the
    // executable alongside their notice directory.
    let executable_parent = executable.parent().ok_or("Executable has no parent")?;
    let distribution = if executable_parent
        .file_name()
        .is_some_and(|name| name == "bin")
    {
        executable_parent
            .parent()
            .ok_or("Installed executable has no skill parent")?
    } else {
        executable_parent
    };
    for name in ["THIRD_PARTY.json", "LICENSE", "LICENSE.md"] {
        let path = distribution.join(name);
        if path.exists() {
            reject_links(&path)?;
            expected.insert(
                name.into(),
                fs::read(path).map_err(|error| error.to_string())?,
            );
        }
    }
    let notices = distribution.join("licenses");
    if notices.exists() {
        reject_links(&notices)?;
        for (name, contents) in installed_files(&notices)? {
            expected.insert(format!("licenses/{name}"), contents);
        }
    }
    expected.insert("INSTALLATION.json".into(), serde_json::to_vec_pretty(&json!({
        "version": env!("CARGO_PKG_VERSION"),
        "sourceRevision": option_env!("MAINTAINABLE_SOURCE_REVISION").unwrap_or("unknown (local build)"),
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
    })).map_err(|error| error.to_string())?);
    let exists = target.exists();
    let old = if exists {
        if !target.is_dir() || !target.join("SKILL.md").is_file() {
            return Err(format!(
                "Destination is not an installed skill: {}",
                target.display()
            ));
        }
        installed_files(&target)?
    } else {
        BTreeMap::new()
    };
    if exists && old == expected {
        return Ok((
            format!("Already installed (identical): {}\n", target.display()),
            0,
        ));
    }
    if exists && !options.update {
        return Err(format!(
            "Destination differs: {}. Use --update to replace it, including local edits",
            target.display()
        ));
    }
    if exists && executable.starts_with(&target) {
        return Err(
            "Update using the new release executable outside the installed skill directory".into(),
        );
    }
    let mut output = format!(
        "{} destination: {}\n",
        if exists { "Update" } else { "Install" },
        target.display()
    );
    for name in old
        .keys()
        .chain(expected.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let change = match (old.get(name), expected.get(name)) {
            (None, Some(_)) => Some("add"),
            (Some(_), None) => Some("remove"),
            (Some(before), Some(after)) if before != after => Some("replace"),
            _ => None,
        };
        if let Some(change) = change {
            output.push_str(&format!("  {change}: {name}\n"));
        }
    }
    if options.dry_run {
        output.push_str("Dry run: no writes; no backup is retained after a successful update\n");
        return Ok((output, 0));
    }
    let parent = target.parent().ok_or("Destination has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    // Keep staging and rollback on the destination filesystem without requiring
    // write access to the skills directory's parent.
    let temporary = tempfile::Builder::new()
        .prefix(".maintainable-install-")
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?;
    let staged = temporary.path().join("maintainable-code");
    fs::create_dir(&staged).map_err(|error| error.to_string())?;
    for (name, contents) in expected {
        let path = staged.join(name);
        fs::create_dir_all(path.parent().ok_or("Package file has no parent")?)
            .map_err(|error| error.to_string())?;
        fs::write(path, contents).map_err(|error| error.to_string())?;
    }
    fs::set_permissions(
        staged.join(executable_name()),
        fs::metadata(&executable)
            .map_err(|error| error.to_string())?
            .permissions(),
    )
    .map_err(|error| error.to_string())?;
    let previous = temporary.path().join("previous");
    if let Err(error) = publish(&staged, &target, &previous, exists, |from, to| {
        fs::rename(from, to)
    }) {
        if error.recovery_required {
            let recovery = temporary.keep();
            return Err(format!(
                "{}. Recovery directory: {}",
                error.message,
                recovery.display()
            ));
        }
        return Err(error.message);
    }
    if let Err(error) = temporary.close() {
        return Err(format!(
            "Installed successfully, but temporary backup cleanup failed: {error}"
        ));
    }
    output.push_str(
        "Installed complete skill and executable. Start a fresh harness session if needed.\n",
    );
    Ok((output, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        let staged = root.path().join("staged");
        let previous = root.path().join("previous");
        fs::create_dir(&target).unwrap();
        fs::create_dir(&staged).unwrap();
        fs::write(target.join("SKILL.md"), "old").unwrap();
        (root, staged, target, previous)
    }

    #[test]
    fn failed_publication_restores_previous_installation() {
        let (_root, staged, target, previous) = setup();
        let error = publish(&staged, &target, &previous, true, |from, to| {
            if from == staged {
                return Err(std::io::Error::other("publication failed"));
            }
            fs::rename(from, to)
        })
        .err()
        .unwrap();
        assert!(!error.recovery_required);
        assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "old");
        assert!(!previous.exists());
    }

    #[test]
    fn failed_restoration_marks_recovery_directory_for_retention() {
        let (_root, staged, target, previous) = setup();
        let error = publish(&staged, &target, &previous, true, |from, to| {
            if from == target {
                fs::rename(from, to)
            } else {
                Err(std::io::Error::other("rename failed"))
            }
        })
        .err()
        .unwrap();
        assert!(error.recovery_required);
        assert_eq!(
            fs::read_to_string(previous.join("SKILL.md")).unwrap(),
            "old"
        );
        assert!(!target.exists());
    }

    #[test]
    fn failed_old_directory_move_leaves_target_untouched() {
        let (_root, staged, target, previous) = setup();
        let error = publish(&staged, &target, &previous, true, |_, _| {
            Err(std::io::Error::other("move failed"))
        })
        .err()
        .unwrap();
        assert!(!error.recovery_required);
        assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "old");
    }
}
