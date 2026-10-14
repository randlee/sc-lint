use std::fs;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use tempfile::TempDir;

#[test]
fn issue136_added_and_stale() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record(&[], &[]);
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["missing"]
        "#,
    );
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-003,SCB-MANIFEST-006"),
    )
    .expect("allowlist policy analyzes");

    assert_failed_findings(
        &report,
        &[
            (
                RuleId::ScbManifest003,
                "manifest_dependency_not_allowlisted",
                "crates/app/Cargo.toml: remove dependency `dep` or review/add its permission",
            ),
            (
                RuleId::ScbManifest006,
                "manifest_policy_inconsistent",
                "crates/app/Cargo.toml: remove unused permission `missing`",
            ),
        ],
    );
}

#[test]
fn issue136_record_locks() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record(&["dep"], &[]);
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
            boundary_record_path = ["boundaries/app/app.toml", "boundaries/app/missing.toml"]
        "#,
    );
    let missing_record = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-006"),
    )
    .expect("record locks validate every configured path");
    assert_failed_findings(
        &missing_record,
        &[(
            RuleId::ScbManifest006,
            "manifest_policy_inconsistent",
            "crates/app/Cargo.toml: record lock `boundaries/app/missing.toml` does not name a boundary record",
        )],
    );

    let split_fixture = Fixture::new();
    split_fixture.workspace();
    split_fixture.write(
        "crates/app/Cargo.toml",
        &package(
            "app",
            "[dependencies]\ndep = { path = \"../dep\" }\n\n[dev-dependencies]\ndep = { path = \"../dep\" }\n",
        ),
    );
    split_fixture.boundary_record(&[], &["dep"]);
    split_fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
            boundary_record_path = "boundaries/app/app.toml"
        "#,
    );
    let split_dependencies = analyze_workspace_configured(
        &AnalyzeOptions {
            root: split_fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-006"),
    )
    .expect("record lock compares runtime and development dependencies separately");
    assert_failed_findings(
        &split_dependencies,
        &[(
            RuleId::ScbManifest006,
            "manifest_policy_inconsistent",
            "crates/app/Cargo.toml: record lock `boundaries/app/app.toml` must align with the allowlist and dependency sections",
        )],
    );
}

#[test]
fn issue136_ownership() {
    let ownership_config = r#"
        [tool.sc-lint.analyzers.boundary]
        [[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
        dependency = "dep"
        allowed_manifest_paths = []
        allowed_source_roots = []
        manifest_message = "dep is not declared here"
        source_message = "dep is not imported here"
    "#;
    let comment_fixture = Fixture::new();
    comment_fixture.workspace();
    comment_fixture.write(
        "crates/app/src/lib.rs",
        "// dep::Dep in a comment is ignored.\n",
    );
    comment_fixture.write("sc-lint.toml", ownership_config);
    let comment_only = analyze_workspace_configured(
        &AnalyzeOptions {
            root: comment_fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-004"),
    )
    .expect("ownership policy ignores comment-only source mentions");
    assert_failed_findings(
        &comment_only,
        &[(
            RuleId::ScbManifest004,
            "manifest_dependency_owner_violation",
            "crates/app/Cargo.toml: dep is not declared here (allowed manifests: )",
        )],
    );

    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "crates/app/src/lib.rs",
        "// dep::Dep in a comment is ignored.\nconst MENTION: &str = \"dep::Dep\";\n",
    );
    fixture.write("sc-lint.toml", ownership_config);
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-004"),
    )
    .expect("ownership policy analyzes");
    assert_failed_findings(
        &report,
        &[
            (
                RuleId::ScbManifest004,
                "manifest_dependency_owner_violation",
                "crates/app/Cargo.toml: dep is not declared here (allowed manifests: )",
            ),
            (
                RuleId::ScbManifest004,
                "source_dependency_owner_violation",
                "crates/app/src/lib.rs: dep is not imported here (allowed source roots: )",
            ),
        ],
    );
}

#[test]
fn issue136_sections() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "crates/app/Cargo.toml",
        &package(
            "app",
            "[target.'cfg(unix)'.dev-dependencies]\ndep = { path = \"../dep\" }\n",
        ),
    );
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
            owner_manifest_path = "crates/app/Cargo.toml"
            dependency_package = "dep"
            allowed_sections = ["dev-dependencies"]
            message = "dep is restricted to ordinary development dependencies"
        "#,
    );
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-005"),
    )
    .expect("section policy analyzes exact target section names");
    assert_failed_findings(
        &report,
        &[(
            RuleId::ScbManifest005,
            "manifest_section_restriction",
            "crates/app/Cargo.toml [target.cfg(unix).dev-dependencies.dep]: dep is restricted to ordinary development dependencies (allowed sections: dev-dependencies)",
        )],
    );
}

#[test]
fn issue136_absent_policy_preserves_manifest_fast_path() {
    let fixture = Fixture::new();
    fixture.workspace();
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect("absent policy keeps manifest analysis independent of inventory");
    assert!(report.findings.iter().all(|finding| matches!(
        finding.rule_id,
        RuleId::ScbManifest001 | RuleId::ScbManifest002
    )));
}

fn assert_failed_findings(
    report: &sc_lint_schema::FindingsReport<RuleId>,
    expected: &[(RuleId, &str, &str)],
) {
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), expected.len());
    for (rule_id, kind, message) in expected {
        assert!(report.findings.iter().any(|finding| {
            finding.rule_id == *rule_id
                && finding.kind.as_str() == *kind
                && finding.message == *message
        }));
    }
}

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        Self(TempDir::new().expect("tempdir"))
    }
    fn root(&self) -> &std::path::Path {
        self.0.path()
    }
    fn write(&self, relative: &str, content: &str) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, content).expect("fixture file");
    }
    fn workspace(&self) {
        self.write("Cargo.toml", "[workspace]\nmembers = [\"crates/app\", \"crates/dep\"]\nresolver = \"2\"\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.85\"\nauthors = [\"test\"]\nlicense = \"MIT\"\nrepository = \"https://example.invalid\"\nhomepage = \"https://example.invalid\"\n");
        self.write("crates/dep/Cargo.toml", &package("dep", ""));
        self.write("crates/dep/src/lib.rs", "pub struct Dep;\n");
        self.write(
            "crates/app/Cargo.toml",
            &package("app", "[dependencies]\ndep = { path = \"../dep\" }\n"),
        );
        self.write(
            "crates/app/src/lib.rs",
            "use dep::Dep;\npub fn make() -> Dep { Dep }\n",
        );
    }
    fn boundary_record(&self, allowed_dependencies: &[&str], allowed_dev_dependencies: &[&str]) {
        let allowed_dependencies = toml_array(allowed_dependencies);
        let allowed_dev_dependencies = toml_array(allowed_dev_dependencies);
        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"K.4\"\n",
        );
        self.write(
            "boundaries/app/app.toml",
            &format!(
                r#"
                boundary_id = "BOUNDARY-App"
                owner_package = "app"
                owner_crate_path = "app"
                name = "App"

                [public]
                facade = "App"

                [implementation]
                type = "App"
                module = "app"
                visibility = "public"
                constructor = "none"

                [composition]
                roots = ["App"]

                [dependencies]
                allowed_dependents = []
                allowed_dependencies = [{allowed_dependencies}]
                allowed_dev_dependencies = [{allowed_dev_dependencies}]
                forbidden_edges = []

                [references]
                scope = "outside_owner_crate"
                forbidden = []

                [testing]
                allowed_test_double_paths = []
                forbidden_test_bypasses = []

                [enforcement]
                lint_rules = []
                review_gates = []

                [status]
                state = "concrete_landed"
            "#,
            ),
        );
    }
}

fn package(name: &str, tail: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n\n{tail}"
    )
}

fn toml_array(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ")
}
