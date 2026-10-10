use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_maintainable-code");

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        Self { _temp: temp, root }
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .current_dir(&self.root)
            .env("MAINTAINABLE_NO_UPDATE_CHECK", "1")
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .output()
            .unwrap()
    }
    fn context(&self, args: &[&str]) -> (Output, Value) {
        let mut command = Command::new(BIN);
        command
            .args(["context", "--project"])
            .arg(&self.root)
            .args(args)
            .current_dir(&self.root);
        command.env("MAINTAINABLE_NO_UPDATE_CHECK", "1");
        let output = command.output().unwrap();
        let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "{error}; stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output, report)
    }
}

fn texts(report: &Value) -> Vec<&str> {
    report["files"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|file| file["records"].as_array().unwrap())
        .filter_map(|record| record["text"].as_str())
        .collect()
}

#[test]
fn comments_in_every_supported_language_ignore_string_markers() {
    let fixture = Fixture::new();
    let samples = [
        ("api.py", "url = '# not comment'\n# real\n", "# real"),
        (
            "api.js",
            "const url = 'https://example.test'; /* real */",
            "/* real */",
        ),
        (
            "api.ts",
            "const text: string = '// nope'; // real\n",
            "// real",
        ),
        (
            "api.tsx",
            "const view = <div>{/* real */}</div>;",
            "/* real */",
        ),
        (
            "api.go",
            "package main\nvar text = \"// nope\"\n// real\n",
            "// real",
        ),
        (
            "api.rs",
            "fn main() { let text = r#\"// nope\"#; /* outer /* inner */ end */ }",
            "/* outer /* inner */ end */",
        ),
        (
            "api.c",
            "const char *url=\"https://example.test\"; // real\n",
            "// real",
        ),
        (
            "api.cpp",
            "const char *text=R\"(// nope)\"; /* real */",
            "/* real */",
        ),
        (
            "Api.java",
            "class Api { String text=\"// nope\"; /* real */ }",
            "/* real */",
        ),
        (
            "Api.cs",
            "class Api { string text=@\"// nope\"; /* real */ }",
            "/* real */",
        ),
        ("api.rb", "text = '# nope'\n# real\n", "# real"),
        ("api.sh", "text='# nope'\n# real\n", "# real"),
        (
            "api.css",
            "/* real */ .x { content: '/* nope */'; }",
            "/* real */",
        ),
        ("api.yaml", "# real\ntext: '# nope'\n", "# real"),
        ("api.toml", "# real\ntext = '# nope'\n", "# real"),
    ];
    for (name, source, expected) in samples {
        fixture.write(name, source);
        let (output, report) = fixture.context(&["--mode", "comments", "--all", name]);
        assert!(output.status.success(), "{name}: {report}");
        assert_eq!(texts(&report), vec![expected], "{name}");
    }
}

#[test]
fn declarations_in_every_supported_language() {
    let fixture = Fixture::new();
    let samples = [
        (
            "api.py",
            "class Client:\n    def lookup(self, key):\n        result = key\n        return result\n",
        ),
        (
            "api.js",
            "const lookup = key => key; class Client { lookup(key) { return key; } }",
        ),
        (
            "api.ts",
            "interface Client { lookup(key: string): string; } const lookup = (key: string) => key;",
        ),
        (
            "api.tsx",
            "function lookup() { return <div/>; } class Client {}",
        ),
        (
            "api.go",
            "package main\ntype Client struct {}\nfunc lookup() {}",
        ),
        ("api.rs", "struct Client {}\nfn lookup() {}"),
        (
            "api.c",
            "struct Client { int key; }; int lookup(int key) {return key;}",
        ),
        (
            "api.cpp",
            "class Client { public: int lookup(int key) {return key;} };",
        ),
        (
            "Api.java",
            "class Client { int lookup(int key) {return key;} }",
        ),
        (
            "Api.cs",
            "class Client { int lookup(int key) {return key;} }",
        ),
        ("api.rb", "class Client\n def lookup(key)\n key\n end\nend"),
        ("api.sh", "Client=1\nlookup() { echo ok; }"),
    ];
    for (name, source) in samples {
        fixture.write(name, source);
        let (output, report) = fixture.context(&["--mode", "symbols", name]);
        assert!(output.status.success(), "{name}: {report}");
        let names: Vec<_> = report["files"][0]["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|record| record["name"].as_str())
            .collect();
        assert!(
            names.iter().any(|name| name.ends_with("lookup")),
            "{name}: {names:?}"
        );
        assert!(names.contains(&"Client"), "{name}: {names:?}");
    }
}

#[test]
fn python_docstrings_preserve_literal_source_and_exclude_ordinary_strings() {
    let fixture = Fixture::new();
    fixture.write("api.py", "# heading\n\"\"\"module\\ncontract\"\"\"\ntext = 'ordinary'\nclass Client:\n    'class contract'\n    def lookup(self):\n        r'function contract'\n        return 'ordinary'\n");
    let (output, report) = fixture.context(&["--mode", "comments", "--all"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(
        texts(&report),
        vec![
            "# heading",
            "\"\"\"module\\ncontract\"\"\"",
            "'class contract'",
            "r'function contract'"
        ]
    );
}

#[test]
fn bytes_and_interpolated_literals_are_not_docstrings() {
    let fixture = Fixture::new();
    fixture.write("api.py", "def first():\n    f'plain'\ndef second():\n    b'bytes'\ndef third():\n    'plain' f'interpolated'\n");
    let (output, report) = fixture.context(&["--mode", "comments", "--all"]);
    assert!(output.status.success(), "{report}");
    assert!(texts(&report).is_empty());
}

#[test]
fn html_embedded_comments_keep_original_locations() {
    let fixture = Fixture::new();
    fixture.write("api.html", "<!-- html -->\n<script>const url='https://example.test'; // js\nfunction lookup() {}\n</script>\n<style>/* css */ .x {color: red;}</style>");
    let (output, report) = fixture.context(&["--mode", "comments", "--all"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(texts(&report), vec!["<!-- html -->", "// js", "/* css */"]);
    let records = report["files"][0]["records"].as_array().unwrap();
    assert_eq!(
        records
            .iter()
            .map(|record| record["line"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 5]
    );
    let (_, report) = fixture.context(&["--mode", "symbols", "--symbol", "lookup"]);
    assert_eq!(report["files"][0]["records"][0]["line"], 3);
}

#[test]
fn recursive_discovery_exclusions_and_overlapping_scopes() {
    let fixture = Fixture::new();
    for name in ["src", "node_modules", ".git", "target", "generated"] {
        fixture.write(&format!("{name}/api.py"), "# real\n");
    }
    fixture.write("README.md", "documentation");
    let (output, report) = fixture.context(&["--mode", "comments", "--exclude", "generated"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["path"], "src/api.py");
    assert_eq!(report["skipped"].as_array().unwrap().len(), 5);
    let (_, report) = fixture.context(&["src", "src/api.py"]);
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
}

#[test]
fn budget_is_global_and_all_removes_text_and_record_limits() {
    let fixture = Fixture::new();
    let long = format!("# {}", "é".repeat(300));
    fixture.write("one.py", &format!("{long}\n# second\n"));
    fixture.write("two.py", "# third\n");
    let (_, report) = fixture.context(&[
        "--mode",
        "comments",
        "--max-items",
        "1",
        "--text-limit",
        "10",
    ]);
    assert_eq!(report["files"][0]["omittedRecords"], 1);
    assert_eq!(report["files"][1]["omittedRecords"], 1);
    assert_eq!(report["files"][0]["clippedFields"], 1);
    assert!(texts(&report)[0].ends_with("...[truncated]"));
    let (_, report) = fixture.context(&["--mode", "comments", "--max-items", "1", "--all"]);
    assert_eq!(texts(&report).len(), 3);
    assert_eq!(texts(&report)[0], long);
    assert_eq!(report["files"][0]["clippedFields"], 0);
}

#[test]
fn symbol_filter_retrieves_scoped_python_names() {
    let fixture = Fixture::new();
    fixture.write("api.py", "class Client:\n    def lookup(self, key: str):\n        result = key\n        return result\n");
    let (output, report) = fixture.context(&["--mode", "symbols", "--symbol", "*lookup*"]);
    assert!(output.status.success(), "{report}");
    let names: Vec<_> = report["files"][0]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Client.lookup"));
    assert!(names.contains(&"Client.lookup.self"));
    assert!(names.contains(&"Client.lookup.key"));
    assert!(names.contains(&"Client.lookup.result"));
}

#[test]
fn input_gaps_keep_successful_and_partial_results() {
    let fixture = Fixture::new();
    fixture.write("bad.js", "// real\nfunction lookup( {");
    fixture.write("ok.py", "# valid\n");
    let (output, report) =
        fixture.context(&["--mode", "comments", "bad.js", "missing.py", "ok.py"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["errors"].as_array().unwrap().len(), 2);
    assert_eq!(texts(&report), vec!["// real", "# valid"]);
}

#[test]
fn unsupported_symbol_languages_and_utf8_are_explicit_gaps() {
    let fixture = Fixture::new();
    fixture.write("api.yaml", "# real\nkey: value\n");
    let (output, report) = fixture.context(&["--mode", "symbols"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        report["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("not configured")
    );
    fs::write(
        fixture.root.join("legacy.py"),
        b"# coding: latin-1\n# caf\xe9\n",
    )
    .unwrap();
    let (output, report) = fixture.context(&["--mode", "comments", "legacy.py"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        report["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("UTF-8")
    );
}

#[test]
fn rejects_escaping_scopes_large_files_and_invalid_arguments() {
    let fixture = Fixture::new();
    fixture.write("large.py", &"#".repeat(1_000_001));
    let (output, report) = fixture.context(&["../outside.py", "large.py"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["errors"].as_array().unwrap().len(), 2);
    for args in [
        vec!["context", "--project", ".", "--symbol", "lookup"],
        vec!["context", "--project", ".", "--max-items", "-1"],
        vec!["rules", "--rules", "N99"],
        vec!["rules"],
    ] {
        assert_eq!(fixture.run(&args).status.code(), Some(2));
    }
}

#[test]
fn source_is_never_executed() {
    let fixture = Fixture::new();
    let source = "raise RuntimeError('do not execute')\nopen('marker', 'w').write('bad')\n";
    fixture.write("api.py", source);
    let (output, _) = fixture.context(&[]);
    assert!(output.status.success());
    assert!(!fixture.root.join("marker").exists());
    assert_eq!(
        fs::read_to_string(fixture.root.join("api.py")).unwrap(),
        source
    );
}

#[test]
fn embedded_rules_preserve_exceptions_and_selection() {
    let fixture = Fixture::new();
    let output = fixture.run(&["rules", "--rules", "N3", "C2", "N3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.matches("| N3 —").count(), 1);
    assert!(text.contains("A duration type supplies the unit"));
    assert!(text.contains("| C2 —"));
    assert!(!text.contains("| N1 —"));
    for (dimension, count) in [("naming", 6), ("comments", 8), ("design", 8)] {
        let output = fixture.run(&["rules", "--dimension", dimension]);
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .matches("<!-- references/")
                .count(),
            count
        );
    }
}

#[test]
fn cached_update_notice_uses_stderr_without_changing_context_json() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let fixture = Fixture::new();
    fixture.write("api.py", "# real\n");
    let repository =
        option_env!("MAINTAINABLE_RELEASE_REPOSITORY").unwrap_or("nghiant03/maintainable-code");
    let cache = serde_json::json!({"repository": repository,
        "checkedAtUnixSeconds": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "release": {"tag_name": "v99.0.0", "draft": false, "prerelease": false, "assets": []}});
    let file = format!(
        "cache/maintainable-code/{}.json",
        repository.replace('/', "--")
    );
    fixture.write(&file, &cache.to_string());
    let before = fs::read(fixture.root.join(&file)).unwrap();
    let output = Command::new(BIN)
        .args(["context", "--project"])
        .arg(&fixture.root)
        .args(["--mode", "comments", "api.py"])
        .env_remove("MAINTAINABLE_NO_UPDATE_CHECK")
        .env("MAINTAINABLE_UPDATE_CACHE_DIR", fixture.root.join("cache"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("update available"));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(texts(&report), vec!["# real"]);
    assert_eq!(fs::read(fixture.root.join(&file)).unwrap(), before);
    let output = Command::new(BIN)
        .args(["--no-update-check", "rules", "--rules", "N3"])
        .env_remove("MAINTAINABLE_NO_UPDATE_CHECK")
        .env("MAINTAINABLE_UPDATE_CACHE_DIR", fixture.root.join("cache"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn explicitly_disabled_notifications_do_not_create_cache_directories() {
    let fixture = Fixture::new();
    let output = Command::new(BIN)
        .args(["rules", "--rules", "N3"])
        .env("MAINTAINABLE_NO_UPDATE_CHECK", "1")
        .env("MAINTAINABLE_UPDATE_CACHE_DIR", fixture.root.join("cache"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(!fixture.root.join("cache").exists());
}

fn installed_binary(target: &Path) -> PathBuf {
    target.join(if cfg!(windows) {
        "bin/maintainable-code.exe"
    } else {
        "bin/maintainable-code"
    })
}

#[test]
fn install_is_self_contained_idempotent_and_executes_from_other_cwd() {
    let fixture = Fixture::new();
    let output = fixture.run(&["install", "--skills-dir", "custom skills"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let target = fixture.root.join("custom skills/maintainable-code");
    assert!(target.join("SKILL.md").is_file());
    assert!(target.join("INSTALLATION.json").is_file());
    assert!(!target.join("scripts/context.py").exists());
    assert!(!target.join("scripts/__pycache__").exists());
    let output = fixture.run(&["install", "--skills-dir", "custom skills"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("identical"));
    let output = Command::new(installed_binary(&target))
        .env("MAINTAINABLE_NO_UPDATE_CHECK", "1")
        .args(["rules", "--rules", "N3"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    fixture.write("api.py", "# real\n");
    let output = Command::new(installed_binary(&target))
        .env("MAINTAINABLE_NO_UPDATE_CHECK", "1")
        .args(["context", "--project"])
        .arg(&fixture.root)
        .args(["--mode", "comments", "api.py"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        texts(&serde_json::from_slice::<Value>(&output.stdout).unwrap()),
        vec!["# real"]
    );
}

#[test]
fn release_notices_survive_skill_export() {
    let fixture = Fixture::new();
    let release = fixture.root.join("release");
    fs::create_dir(&release).unwrap();
    let name = if cfg!(windows) {
        "maintainable-code.exe"
    } else {
        "maintainable-code"
    };
    let executable = release.join(name);
    fs::copy(BIN, &executable).unwrap();
    fixture.write("release/licenses/dependency/LICENSE", "license notice");
    fixture.write("release/THIRD_PARTY.json", "[]");
    let output = Command::new(executable)
        .args(["install", "--skills-dir"])
        .arg(fixture.root.join("skills"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let target = fixture.root.join("skills/maintainable-code");
    assert_eq!(
        fs::read_to_string(target.join("licenses/dependency/LICENSE")).unwrap(),
        "license notice"
    );
    assert!(target.join("THIRD_PARTY.json").is_file());
}

#[test]
fn all_harness_destinations_and_user_config_are_resolved() {
    for (harness, folder) in [
        ("shared", ".agents"),
        ("opencode", ".opencode"),
        ("crush", ".crush"),
        ("claude-code", ".claude"),
        ("codex", ".agents"),
    ] {
        let fixture = Fixture::new();
        assert!(
            fixture
                .run(&["install", "--harness", harness])
                .status
                .success()
        );
        assert!(
            fixture
                .root
                .join(folder)
                .join("skills/maintainable-code/SKILL.md")
                .is_file()
        );
        assert!(
            fixture
                .run(&["install", "--harness", harness, "--scope", "user"])
                .status
                .success()
        );
        let user = match harness {
            "claude-code" => "home/.claude/skills",
            "codex" => "home/.agents/skills",
            "shared" => "config/agents/skills",
            "opencode" => "config/opencode/skills",
            _ => "config/crush/skills",
        };
        assert!(
            fixture
                .root
                .join(user)
                .join("maintainable-code/SKILL.md")
                .is_file()
        );
    }
}

#[test]
fn dry_run_has_no_writes_and_differing_work_is_protected() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .run(&["install", "--skills-dir", "skills", "--dry-run"])
            .status
            .success()
    );
    assert!(!fixture.root.join("skills").exists());
    fixture.write("skills/maintainable-code/SKILL.md", "local edits");
    assert_eq!(
        fixture
            .run(&["install", "--skills-dir", "skills"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("skills/maintainable-code/SKILL.md")).unwrap(),
        "local edits"
    );
    assert!(
        fixture
            .run(&["install", "--skills-dir", "skills", "--update", "--dry-run"])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("skills/maintainable-code/SKILL.md")).unwrap(),
        "local edits"
    );
}

#[test]
fn update_removes_obsolete_files_without_touching_siblings_or_leaving_backups() {
    let fixture = Fixture::new();
    fixture.write("skills/maintainable-code/SKILL.md", "local edits");
    fixture.write(
        "skills/maintainable-code/scripts/review_context.py",
        "obsolete",
    );
    fixture.write("skills/another-skill/SKILL.md", "keep");
    let output = fixture.run(&["install", "--skills-dir", "skills", "--update"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !fixture
            .root
            .join("skills/maintainable-code/scripts/review_context.py")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("skills/another-skill/SKILL.md")).unwrap(),
        "keep"
    );
    assert!(fs::read_dir(&fixture.root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".maintainable-install-")
    }));
}

#[test]
fn non_skill_destinations_and_invalid_projects_are_protected() {
    let fixture = Fixture::new();
    fixture.write("skills/maintainable-code/keep.txt", "keep");
    assert_eq!(
        fixture
            .run(&["install", "--skills-dir", "skills", "--update"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("skills/maintainable-code/keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(
        fixture
            .run(&["install", "--harness", "shared", "--project", "missing"])
            .status
            .code(),
        Some(2)
    );
    assert!(!fixture.root.join("missing").exists());
}

#[cfg(unix)]
#[test]
fn symlinks_and_special_files_are_not_followed() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.write("src/api.py", "# real\n");
    symlink(fixture.root.join("src"), fixture.root.join("alias")).unwrap();
    let (_, report) = fixture.context(&["--mode", "comments"]);
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert!(
        report["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["reason"] == "symlink")
    );
    let (output, _) = fixture.context(&["alias/api.py"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        fixture
            .run(&["install", "--skills-dir", "alias", "--update"])
            .status
            .code(),
        Some(2)
    );
    let fifo = fixture.root.join("pipe.py");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let (output, report) = fixture.context(&["pipe.py"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        report["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("regular")
    );
}
