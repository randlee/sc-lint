use std::fs;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use tempfile::TempDir;

#[test]
fn issue136_ownership_and_sections_are_enforced_from_configured_manifests() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
            dependency = "dep"
            allowed_manifest_paths = []
            allowed_source_roots = []
            manifest_message = "dep is not declared here"
            source_message = "dep is not imported here"

            [[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
            owner_manifest_path = "crates/app/Cargo.toml"
            dependency_package = "dep"
            allowed_sections = []
            message = "dep is forbidden in every section"
        "#,
    );

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-004,SCB-MANIFEST-005"),
    )
    .expect("configured manifest policy analyzes");
    let ids = report
        .findings
        .iter()
        .map(|finding| finding.rule_id)
        .collect::<Vec<_>>();
    assert!(ids.contains(&RuleId::ScbManifest004));
    assert!(ids.contains(&RuleId::ScbManifest005));
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

#[test]
fn issue136_added_and_stale_allowlist_entries_are_reported() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record();
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
    let ids = report
        .findings
        .iter()
        .map(|finding| finding.rule_id)
        .collect::<Vec<_>>();
    assert!(ids.contains(&RuleId::ScbManifest003));
    assert!(ids.contains(&RuleId::ScbManifest006));
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
    fn boundary_record(&self) {
        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"K.4\"\n",
        );
        self.write(
            "boundaries/app/app.toml",
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
                allowed_dependencies = []
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
        );
    }
}

fn package(name: &str, tail: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n\n{tail}"
    )
}
