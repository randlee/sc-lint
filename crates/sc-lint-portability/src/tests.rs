#![cfg(test)]

use std::fs;
use std::path::Path;

use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use tempfile::TempDir;

use super::*;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;

#[test]
fn findings_report_text_is_stable() {
    let report = FindingsReport {
        tool: "sc-lint-portability",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: Vec::new(),
    };
    assert_eq!(
        render_findings_report(&report),
        "sc-lint-portability 0.1.0 status=pass scanned_crates=2 findings=0"
    );
}

#[test]
fn findings_report_text_renders_failure_status_and_count() {
    let report = FindingsReport {
        tool: "sc-lint-portability",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 4,
        findings: vec![Finding {
            rule_id: RuleId::Port004,
            kind: "ungated_std_os_unix_import".to_string(),
            message: "ungated import".to_string(),
            owner_ids: vec![OwnerId::new("crate::example::example")],
            node_ids: vec![NodeId::new("crate::example::example::portability")],
        }],
    };

    assert_eq!(
        render_findings_report(&report),
        "sc-lint-portability 0.1.0 status=fail scanned_crates=4 findings=1"
    );
}

#[test]
fn analyze_workspace_counts_crate_targets() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.scanned_crates, 1);
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_hardcoded_tmp_path_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                use std::path::PathBuf;

                #[test]
                fn writes_artifact() {
                    // Intentional PORT-001 fixture: hardcoded /tmp/ path should be flagged.
                    let _ = PathBuf::from("/tmp/test-artifact");
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::Port001);
}

#[test]
fn passes_temp_dir_usage_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn writes_artifact() {
                    let _ = std::env::temp_dir().join("test-artifact");
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_unix_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn socket_path() -> std::path::PathBuf {
                // Intentional PORT-001 fixture: hardcoded /tmp/ path should be flagged.
                std::path::PathBuf::from("/tmp/runtime-socket")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_hardcoded_unix_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn socket_dir() -> std::path::PathBuf {
                std::path::PathBuf::from("/var/run/sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
}

#[test]
fn passes_dirs_cache_dir_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn cache_dir() -> std::path::PathBuf {
                dirs::cache_dir().expect("cache directory")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_hardcoded_windows_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn cache_file() -> std::path::PathBuf {
                let _ = r"\\server\share\sc-lint\cache.json";
                std::path::PathBuf::from(r"C:\ProgramData\sc-lint\cache.json")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::Port007)
            .count(),
        2
    );
}

#[test]
fn flags_home_env_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn config_root() -> std::path::PathBuf {
                let home = std::env::var("HOME").expect("HOME");
                std::path::PathBuf::from(home).join(".config").join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port008)
    );
}

#[test]
fn flags_xdg_config_home_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn config_root() -> std::path::PathBuf {
                let root = std::env::var_os("XDG_CONFIG_HOME").expect("XDG_CONFIG_HOME");
                std::path::PathBuf::from(root).join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port008)
    );
}

#[test]
fn passes_dirs_data_dir_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn data_root() -> std::path::PathBuf {
                dirs::data_dir().expect("data directory")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_home_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn config_root() -> std::path::PathBuf {
                let home = std::env::var("HOME").expect("HOME");
                std::path::PathBuf::from(home).join(".config").join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_command_new_sh_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg("git status --short")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_command_new_bash_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("bash")
                    .arg("-lc")
                    .arg("true")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_bin_bash_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("/bin/bash")
                    .arg("-lc")
                    .arg("true")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_bin_sh_path_literal_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub const SHELL_PATH: &str = "/bin/sh";
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn passes_std_process_command_with_binary_name_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_git() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("git")
                    .arg("status")
                    .arg("--short")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_shell_invocation_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn run_unix_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg("true")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_dirs_home_dir_without_override_check() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                fn config_root() -> std::path::PathBuf {
                    dirs::home_dir().expect("home")
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::Port002);
    assert!(report.findings[0].message.contains("ATM_CONFIG_HOME"));
}

#[test]
fn passes_dirs_home_dir_after_override_check() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                fn get_os_home_dir() -> std::path::PathBuf {
                    if let Ok(home) = std::env::var("ATM_CONFIG_HOME") {
                        return std::path::PathBuf::from(home);
                    }
                    dirs::home_dir().expect("home")
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_set_var_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn mutates_env() {
                    unsafe { std::env::set_var("HOME", "test-home") };
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port003)
    );
}

#[test]
fn flags_ungated_std_os_unix_import_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn os_string(bytes: Vec<u8>) -> std::ffi::OsString {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(bytes)
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port004)
    );
}

#[test]
fn passes_cfg_unix_gated_std_os_unix_import_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn os_string(bytes: Vec<u8>) -> std::ffi::OsString {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(bytes)
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_cfg_attr_not_unix_allow_dead_code_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg_attr(not(unix), allow(dead_code))]
            pub fn unix_socket_path() -> &'static str {
                // Intentional PORT-001 fixture: hardcoded /tmp/ path should be flagged.
                "/tmp/runtime.sock"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port005)
    );
}

struct WorkspaceFixture {
    tempdir: TempDir,
}

impl WorkspaceFixture {
    fn new() -> Self {
        Self {
            tempdir: TempDir::new().unwrap(),
        }
    }

    fn root(&self) -> &Path {
        self.tempdir.path()
    }

    fn write_workspace_root(&self) {
        self.write(
            "Cargo.toml",
            r#"
                    [workspace]
                    members = ["crates/example"]
                    resolver = "2"
                "#,
        );
    }

    fn write_package_manifest(&self, package_name: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                r#"
                        [package]
                        name = "{package_name}"
                        version = "0.1.0"
                        edition = "2024"
                    "#
            ),
        );
    }

    fn write_source(&self, package_name: &str, relative_path: &str, contents: &str) {
        self.write(
            &format!("crates/{package_name}/src/{relative_path}"),
            contents,
        );
    }

    fn write_lint_config(&self, contents: &str) {
        self.write("sc-lint.toml", contents);
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, trim_indentation(contents)).unwrap();
    }
}

fn trim_indentation(input: &str) -> String {
    let lines: Vec<_> = input.lines().collect();
    let first_content = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).count())
        .unwrap_or(0);

    let mut output = String::new();
    for line in lines {
        let trimmed = if line.len() >= first_content {
            &line[first_content..]
        } else {
            line.trim_end()
        };
        output.push_str(trimmed);
        output.push('\n');
    }
    output
}
