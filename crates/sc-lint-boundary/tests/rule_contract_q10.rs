use std::fs;
use std::path::Path;
use std::process::Command;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use tempfile::TempDir;

struct Workspace(TempDir);

impl Workspace {
    fn new() -> Self {
        let fixture = TempDir::new().expect("workspace fixture");
        write(
            fixture.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"app\"]\nresolver = \"2\"\n\
             [workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(
            fixture.path(),
            "app/Cargo.toml",
            "[package]\nname = \"app\"\nversion.workspace = true\nedition.workspace = true\n",
        );
        write(fixture.path(), "app/src/lib.rs", "pub struct App;\n");
        Self(fixture)
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn config(&self, body: &str) {
        write(
            self.root(),
            "sc-lint.toml",
            &format!("[tool.sc-lint.analyzers.boundary]\n{body}"),
        );
    }
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create directories");
    fs::write(path, contents).expect("write fixture file");
}

fn analyze_new_rules(workspace: &Workspace) -> sc_lint_boundary::FindingsReport {
    analyze_workspace_configured(
        &AnalyzeOptions {
            root: workspace.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-ENV-001,SCB-MANIFEST-007,SCB-MANIFEST-008"),
    )
    .expect("analyze selected q-10 rules")
}

#[test]
fn new_policy_families_parse_and_stay_inactive_until_their_rule_layers() {
    let workspace = Workspace::new();
    workspace.config(
        "[[tool.sc-lint.analyzers.boundary.env_var_boundary]]\n\
         forbidden_env_vars = [\"ATM_HOME\"]\n\
         restricted_crate_roots = [\"app\"]\n\
         boundary_reader_functions = [\"app::read_home\"]\n\
         [[tool.sc-lint.analyzers.boundary.dependency_requirements]]\n\
         template_manifests = [\"templates/**/Cargo.toml\"]\n\
         package_archives = [\"target/package/*.crate\"]\n\
         test_manifests = [\"tests/fixtures/**/Cargo.toml\"]\n",
    );

    let report = analyze_new_rules(&workspace);
    assert!(
        report.findings.iter().all(|finding| !matches!(
            finding.rule_id,
            RuleId::ScbEnv001 | RuleId::ScbManifest007 | RuleId::ScbManifest008
        )),
        "q-10 registers contracts only; q-11/q-12 add detection"
    );

    let no_config = Workspace::new();
    no_config.config("");
    let report = analyze_new_rules(&no_config);
    assert!(report.findings.iter().all(|finding| !matches!(
        finding.rule_id,
        RuleId::ScbEnv001 | RuleId::ScbManifest007 | RuleId::ScbManifest008
    )));
}

#[test]
fn new_rule_ids_accept_per_rule_path_exclusions() {
    let workspace = Workspace::new();
    workspace.config(
        "exclude_paths_by_rule = {\n\
         \"SCB-ENV-001\" = [\"app/src/**\"],\n\
         \"SCB-MANIFEST-007\" = [\"app/Cargo.toml\"],\n\
         \"SCB-MANIFEST-008\" = [\"app/Cargo.toml\"]\n}\n",
    );

    let report = analyze_new_rules(&workspace);
    assert!(report.findings.is_empty());
}

#[test]
fn invalid_policy_fields_exit_three_and_name_the_field() {
    for (table, field, value) in [
        ("env_var_boundary", "unexpected", "true"),
        ("dependency_requirements", "template_manifests", "42"),
    ] {
        let workspace = Workspace::new();
        workspace.config(&format!(
            "[[tool.sc-lint.analyzers.boundary.{table}]]\n{field} = {value}\n"
        ));
        let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-boundary"))
            .arg("analyze")
            .arg("--root")
            .arg(workspace.root())
            .output()
            .expect("run boundary CLI");

        assert_eq!(output.status.code(), Some(3));
        let stderr = String::from_utf8(output.stderr).expect("CLI stderr is UTF-8");
        assert!(stderr.contains(field), "{stderr}");
    }
}
