//! Black-box coverage of repo-root discovery, the boundary-family
//! `boundaries/` requirement and per-table Python config resolution (l-4).
//! Every test runs the built binary once with `Command::output()` against a
//! tmp-dir repository; nothing waits, polls or depends on timing.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

// Mirrors `EXIT_CONFIG` in crates/sc-lint/src/consts.rs.
const EXIT_CONFIG: i32 = 3;
const PACKAGE_DIR: &str = "bindings/sc-lint-py/python/sc_lint";
// The same support files the logging_integration.rs WorkspaceFixture copies.
const PACKAGE_FILES: [&str; 8] = [
    "__init__.py",
    "_binary.py",
    "lint_common.py",
    "python_adapter.py",
    "view_common.py",
    "lint_line_counts.py",
    "lint_identity_literals.py",
    "view_findings.py",
];
const WORKSPACE_MANIFEST: &str = "[workspace]\nmembers = []\n";
const PACKAGE_MANIFEST: &str = "[package]\nname = \"inner\"\nversion = \"0.1.0\"\n";
const KIT_SC_LINT_TOML: &str = "[logging]\nconsole = false\n";
const LINE_COUNTS_500: &str = "[line_counts]\nmax_total_lines = 500\n";
const LINE_COUNTS_300: &str = "[line_counts]\nmax_total_lines = 300\n";
const LINE_COUNTS_200: &str = "[line_counts]\nmax_total_lines = 200\n";
const LINE_COUNTS_EMPTY: &str = "[line_counts]\n";
const UNKNOWN_CODE_STUB: &str = r#"import sys

from sc_lint.python_adapter import AdapterError, error_payload
from sc_lint.python_adapter import write_json as write_adapter_json


def main(argv):
    write_adapter_json(
        error_payload(AdapterError("config", "stub failure", details={"code": "config.unknown_code"}))
    )
    return 3


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
"#;

/// A tmp-dir tree plus an isolated ATM home, so no test touches the
/// developer's ATM state.
struct Sandbox {
    repo: TempDir,
    atm: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            repo: TempDir::new().expect("repo temp dir"),
            atm: TempDir::new().expect("ATM temp dir"),
        }
    }

    /// The shared fixture: a `[workspace]` manifest, a kit `sc-lint.toml`
    /// holding no Python table, the Python package and no `boundaries/`.
    fn fixture() -> Self {
        let sandbox = Self::new();
        sandbox.write("Cargo.toml", WORKSPACE_MANIFEST);
        sandbox.write("sc-lint.toml", KIT_SC_LINT_TOML);
        sandbox.install_python_package("");
        sandbox
    }

    fn path(&self, relative: &str) -> PathBuf {
        if relative.is_empty() {
            self.repo.path().to_path_buf()
        } else {
            self.repo.path().join(relative)
        }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        std::fs::write(path, contents).expect("write fixture file");
    }

    fn mkdir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(&path).expect("create fixture dir");
        path
    }

    fn install_python_package(&self, under: &str) {
        for file in PACKAGE_FILES {
            let relative = format!("{PACKAGE_DIR}/{file}");
            let source = repo_source_root().join(&relative);
            let contents = std::fs::read_to_string(&source).expect("read package file");
            self.write(&join_relative(under, &relative), &contents);
        }
    }

    fn canonical_root(&self) -> PathBuf {
        dunce::canonicalize(self.repo.path()).expect("canonical repo root")
    }

    /// Runs `sc-lint --json --root <start> <args...>` once.
    fn run(&self, start: &Path, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sc-lint"));
        command.current_dir(start);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("ATM_") {
                command.env_remove(key);
            }
        }
        command.env("ATM_HOME", self.atm.path().join("home"));
        command.env("ATM_CONFIG_HOME", self.atm.path().join("config-home"));
        command
            .arg("--json")
            .arg("--root")
            .arg(start)
            .args(args)
            .output()
            .expect("sc-lint runs")
    }

    fn line_counts(&self) -> Output {
        self.run(&self.path(""), &["lint", "line-counts"])
    }
}

fn join_relative(under: &str, relative: &str) -> String {
    if under.is_empty() {
        relative.to_string()
    } else {
        format!("{under}/{relative}")
    }
}

fn repo_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn context(output: &Output) -> String {
    format!(
        "exit={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn success_json(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{}", context(output));
    serde_json::from_slice(&output.stdout).expect("success JSON on stdout")
}

fn config_error(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(EXIT_CONFIG),
        "{}",
        context(output)
    );
    let envelope: Value = serde_json::from_slice(&output.stderr).expect("error JSON on stderr");
    envelope["error"].clone()
}

fn limits_summary(output: &Output) -> String {
    let payload = success_json(output);
    payload["data"]["limits"]["summary"]
        .as_str()
        .unwrap_or_else(|| panic!("data.limits.summary missing: {payload}"))
        .to_string()
}

fn summary_json(root: &Path) -> PathBuf {
    root.join("artifacts/findings/line-counts/summary.json")
}

#[test]
fn lints_run_without_boundaries_dir() {
    let sandbox = Sandbox::fixture();
    sandbox.write(".just/lint-config.toml", LINE_COUNTS_500);

    assert_eq!(limits_summary(&sandbox.line_counts()), "total<=500");
}

#[test]
fn boundary_family_fails_without_boundaries_dir() {
    let sandbox = Sandbox::fixture();
    sandbox.write(".just/lint-config.toml", LINE_COUNTS_500);
    let canonical_root = sandbox.canonical_root();

    let error = config_error(&sandbox.run(&sandbox.path(""), &["lint", "sc-boundary"]));

    assert_eq!(error["kind"], "config");
    assert_eq!(error["code"], "config.boundaries_dir_missing");
    assert_eq!(
        error["cause"],
        format!(
            "no `boundaries/` directory under repo root `{}`",
            canonical_root.display()
        )
    );
    assert_eq!(
        error["suggested_action"],
        "create `boundaries/` or do not run the boundary family"
    );
    assert_eq!(
        error["details"]["repo_root"],
        canonical_root.display().to_string()
    );
}

#[test]
fn repo_root_nearer_workspace_manifest_beats_farther_sc_lint_toml() {
    let sandbox = Sandbox::new();
    sandbox.write("outer/sc-lint.toml", KIT_SC_LINT_TOML);
    sandbox.write("outer/inner/Cargo.toml", WORKSPACE_MANIFEST);
    sandbox.write("outer/inner/.just/lint-config.toml", LINE_COUNTS_EMPTY);
    sandbox.install_python_package("outer/inner");
    let start = sandbox.mkdir("outer/inner/src");

    let output = sandbox.run(&start, &["lint", "line-counts"]);

    success_json(&output);
    assert!(
        summary_json(&sandbox.path("outer/inner")).is_file(),
        "{}",
        context(&output)
    );
    assert!(!summary_json(&sandbox.path("outer")).exists());
}

#[test]
fn repo_root_nearer_sc_lint_toml_beats_farther_workspace_manifest() {
    let sandbox = Sandbox::new();
    sandbox.write("outer/Cargo.toml", WORKSPACE_MANIFEST);
    sandbox.write("outer/inner/sc-lint.toml", LINE_COUNTS_EMPTY);
    sandbox.write("outer/inner/Cargo.toml", PACKAGE_MANIFEST);
    sandbox.install_python_package("outer/inner");
    let start = sandbox.mkdir("outer/inner/src");

    let output = sandbox.run(&start, &["lint", "line-counts"]);

    success_json(&output);
    assert!(
        summary_json(&sandbox.path("outer/inner")).is_file(),
        "{}",
        context(&output)
    );
    assert!(!summary_json(&sandbox.path("outer")).exists());
}

#[test]
fn repo_root_ignores_package_only_manifest() {
    let sandbox = Sandbox::new();
    sandbox.write("outer/sc-lint.toml", LINE_COUNTS_EMPTY);
    sandbox.write("outer/Cargo.toml", WORKSPACE_MANIFEST);
    sandbox.install_python_package("outer");
    sandbox.write("outer/pkg/Cargo.toml", PACKAGE_MANIFEST);
    let start = sandbox.mkdir("outer/pkg/src");

    let output = sandbox.run(&start, &["lint", "line-counts"]);

    success_json(&output);
    assert!(
        summary_json(&sandbox.path("outer")).is_file(),
        "{}",
        context(&output)
    );
    assert!(!summary_json(&sandbox.path("outer/pkg")).exists());
}

#[test]
fn table_falls_back_per_table() {
    // The kit sc-lint.toml exists but lacks [line_counts].
    let sandbox = Sandbox::fixture();
    sandbox.write(".just/lint-config.toml", LINE_COUNTS_500);

    assert_eq!(limits_summary(&sandbox.line_counts()), "total<=500");
}

#[test]
fn sc_lint_toml_table_wins() {
    let sandbox = Sandbox::fixture();
    sandbox.write("sc-lint.toml", LINE_COUNTS_300);
    sandbox.write(".just/lint-config.toml", LINE_COUNTS_500);

    assert_eq!(limits_summary(&sandbox.line_counts()), "total<=300");
}

#[test]
fn explicit_config_wins_over_repo_files() {
    let sandbox = Sandbox::fixture();
    sandbox.write("sc-lint.toml", LINE_COUNTS_300);
    sandbox.write(".just/lint-config.toml", LINE_COUNTS_500);
    let explicit_dir = TempDir::new().expect("explicit config dir");
    let explicit = explicit_dir.path().join("explicit.toml");
    std::fs::write(&explicit, LINE_COUNTS_200).expect("write explicit config");
    let explicit = explicit.to_str().expect("utf-8 explicit path");

    let output = sandbox.run(
        &sandbox.path(""),
        &["--config", explicit, "lint", "line-counts"],
    );

    assert_eq!(limits_summary(&output), "total<=200");
}

#[test]
fn table_in_neither_file_fails() {
    let sandbox = Sandbox::fixture();
    let message = "configuration table [line_counts] not found in any of: sc-lint.toml, .just/lint-config.toml";

    let error = config_error(&sandbox.line_counts());

    assert_eq!(error["kind"], "config");
    assert_eq!(error["code"], "config.table_missing");
    assert_eq!(error["message"], message);
    assert_eq!(error["cause"], message);
    assert_eq!(error["details"]["code"], "config.table_missing");
    assert_eq!(error["details"]["table"], "line_counts");
    assert_eq!(
        error["details"]["searched"],
        json!(["sc-lint.toml", ".just/lint-config.toml"])
    );
    assert_eq!(
        error["suggested_action"],
        "add the table to `sc-lint.toml` or `.just/lint-config.toml`"
    );
}

#[test]
fn unknown_adapter_code_stays_in_details() {
    let sandbox = Sandbox::fixture();
    sandbox.write(
        &format!("{PACKAGE_DIR}/lint_line_counts.py"),
        UNKNOWN_CODE_STUB,
    );

    let error = config_error(&sandbox.line_counts());

    assert_eq!(error["kind"], "config");
    assert_eq!(error["code"], "CLI.CONFIG_ERROR");
    assert_eq!(error["message"], "stub failure");
    assert_eq!(error["details"]["code"], "config.unknown_code");
    assert!(error.get("cause").is_none(), "unexpected cause: {error}");
}
