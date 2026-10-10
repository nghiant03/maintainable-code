use crate::extract::{self, Mode};
use clap::Args;
use glob::Pattern;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

const MAX_SOURCE_BYTES: u64 = 1_000_000;
const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".jj",
    ".crush",
    ".opencode",
    "__pycache__",
    ".venv",
    "venv",
    "node_modules",
    "vendor",
    "build",
    "dist",
    "target",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
];

#[derive(Args)]
pub struct Options {
    #[arg(long)]
    project: PathBuf,
    #[arg(long, value_enum, default_value = "all")]
    mode: Mode,
    /// Name glob filter; repeatable, symbols mode only.
    #[arg(long)]
    symbol: Vec<String>,
    /// Additional relative path/basename glob to skip.
    #[arg(long)]
    exclude: Vec<String>,
    /// Global record budget; 0 means unlimited.
    #[arg(long, default_value_t = 60)]
    max_items: usize,
    /// Characters per text field; 0 means unlimited.
    #[arg(long, default_value_t = 240)]
    text_limit: usize,
    /// Retrieve all records and full text in the selected mode.
    #[arg(long)]
    all: bool,
    /// Project-relative files/directories; default entire project.
    paths: Vec<PathBuf>,
}

#[derive(Serialize)]
struct Problem {
    path: String,
    message: String,
}
#[derive(Serialize)]
struct Skipped {
    path: String,
    reason: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileContext {
    path: String,
    language: &'static str,
    lines: usize,
    records: Vec<Value>,
    total_records: usize,
    omitted_records: usize,
    clipped_fields: usize,
}
#[derive(Serialize)]
struct Report {
    purpose: &'static str,
    mode: Mode,
    limitations: &'static str,
    files: Vec<FileContext>,
    skipped: Vec<Skipped>,
    errors: Vec<Problem>,
}

fn patterns(values: &[String]) -> Result<Vec<Pattern>, String> {
    values
        .iter()
        .map(|value| {
            Pattern::new(value).map_err(|error| format!("Invalid glob {value:?}: {error}"))
        })
        .collect()
}

fn relative(path: &Path, project: &Path) -> String {
    path.strip_prefix(project)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn excluded(path: &Path, project: &Path, patterns: &[Pattern]) -> bool {
    let rel = path.strip_prefix(project).unwrap_or(path);
    rel.components().any(|part| {
        DEFAULT_EXCLUDES
            .iter()
            .any(|name| part.as_os_str() == *name)
    }) || patterns.iter().any(|pattern| {
        pattern.matches(&relative(path, project))
            || pattern.matches(&path.file_name().unwrap_or_default().to_string_lossy())
    })
}

/// Resolve lexical components before touching files; links are never traversed.
fn scope(project: &Path, raw: &Path) -> Result<PathBuf, String> {
    let mut path = project.to_path_buf();
    for part in raw.components() {
        match part {
            Component::CurDir => {}
            Component::Normal(name) => {
                path.push(name);
                if fs::symlink_metadata(&path)
                    .map_err(|error| error.to_string())?
                    .file_type()
                    .is_symlink()
                {
                    return Err("Symlink scopes are not followed".into());
                }
            }
            Component::ParentDir if path != project => {
                path.pop();
            }
            _ => return Err("Expected a project-relative path inside the project".into()),
        }
    }
    Ok(path)
}

fn discover(
    project: &Path,
    paths: &[PathBuf],
    excludes: &[Pattern],
    report: &mut Report,
) -> BTreeMap<String, PathBuf> {
    let mut found = BTreeMap::new();
    for raw in paths {
        let start = match scope(project, raw) {
            Ok(path) => path,
            Err(message) => {
                report.errors.push(Problem {
                    path: raw.display().to_string(),
                    message,
                });
                continue;
            }
        };
        let mut walk = WalkDir::new(&start)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter();
        while let Some(entry) = walk.next() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    report.errors.push(Problem {
                        path: error
                            .path()
                            .map(|path| relative(path, project))
                            .unwrap_or_default(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let path = entry.path();
            let reason = if excluded(path, project, excludes) {
                Some("excluded")
            } else if entry.file_type().is_symlink() {
                Some("symlink")
            } else {
                None
            };
            if let Some(reason) = reason {
                report.skipped.push(Skipped {
                    path: relative(path, project),
                    reason,
                });
                if entry.file_type().is_dir() {
                    walk.skip_current_dir();
                }
            } else if !entry.file_type().is_dir() {
                if extract::language_for(path).is_some() {
                    found.insert(relative(path, project), path.to_path_buf());
                } else {
                    report.skipped.push(Skipped {
                        path: relative(path, project),
                        reason: "unsupported extension",
                    });
                }
            }
        }
    }
    found
}

/// Limit both metadata size and bytes read so growing files cannot bypass the cap.
fn read_source(path: &Path) -> Result<String, String> {
    let info = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !info.is_file() || info.len() > MAX_SOURCE_BYTES {
        return Err("Expected a regular UTF-8 source file no larger than 1 MB".into());
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|error| error.to_string())?;
    if data.len() as u64 > MAX_SOURCE_BYTES {
        return Err("Source grew beyond 1 MB".into());
    }
    String::from_utf8(data)
        .map_err(|_| "Source must be UTF-8; legacy encodings are not supported".into())
}

pub fn run(options: Options) -> Result<(String, u8), String> {
    if !options.symbol.is_empty() && options.mode != Mode::Symbols {
        return Err("--symbol requires --mode symbols".into());
    }
    let project = options
        .project
        .canonicalize()
        .map_err(|error| format!("Invalid project: {error}"))?;
    if !project.is_dir() {
        return Err("--project must be an existing directory".into());
    }
    let excludes = patterns(&options.exclude)?;
    let symbols = patterns(&options.symbol)?;
    let paths = if options.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        options.paths
    };
    let budget = if options.all { 0 } else { options.max_items };
    let text_limit = if options.all { 0 } else { options.text_limit };
    let mut report = Report {
        purpose: "Review navigation only, not violations or a quality pass.",
        mode: options.mode,
        limitations: "Selected declarations, not references or resolved bindings. All source must be UTF-8. Check errors, skipped paths, and omissions before claiming coverage.",
        files: vec![],
        skipped: vec![],
        errors: vec![],
    };
    let mut shown_count = 0;
    for (path, full_path) in discover(&project, &paths, &excludes, &mut report) {
        let language =
            extract::language_for(&full_path).expect("discovery selected a supported extension");
        let source = match read_source(&full_path) {
            Ok(source) => source,
            Err(message) => {
                report.errors.push(Problem { path, message });
                continue;
            }
        };
        let (mut records, problems) = extract::records(&source, language, options.mode)?;
        for message in problems {
            report.errors.push(Problem {
                path: path.clone(),
                message,
            });
        }
        if !symbols.is_empty() {
            records.retain(|record| {
                let name = record["name"].as_str().unwrap_or_default();
                symbols.iter().any(|pattern| {
                    pattern.matches(name)
                        || pattern.matches(name.rsplit('.').next().unwrap_or(name))
                })
            });
        }
        let total_records = records.len();
        if budget != 0 {
            records.truncate(budget.saturating_sub(shown_count));
        }
        let mut clipped_fields = 0;
        for record in &mut records {
            if let Some(fields) = record.as_object_mut() {
                for (key, value) in fields {
                    if let Some(text) = value.as_str()
                        && key != "kind"
                        && text_limit != 0
                        && text.chars().count() > text_limit
                    {
                        *value = Value::String(format!(
                            "{}...[truncated]",
                            text.chars().take(text_limit).collect::<String>()
                        ));
                        clipped_fields += 1;
                    }
                }
            }
        }
        shown_count += records.len();
        report.files.push(FileContext {
            path,
            language: language.name(),
            lines: source.lines().count(),
            omitted_records: total_records - records.len(),
            total_records,
            clipped_fields,
            records,
        });
    }
    let code = if report.errors.is_empty() { 0 } else { 2 };
    Ok((
        format!(
            "{}\n",
            serde_json::to_string(&report).map_err(|error| error.to_string())?
        ),
        code,
    ))
}
