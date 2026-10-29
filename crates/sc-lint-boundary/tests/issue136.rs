use std::fs;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::BoundaryError;
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

    let dev_fixture = Fixture::new();
    dev_fixture.workspace();
    dev_fixture.write(
        "crates/app/Cargo.toml",
        &package("app", "[dev-dependencies]\ndep = { path = \"../dep\" }\n"),
    );
    dev_fixture.write("crates/app/src/lib.rs", "pub struct App;\n");
    dev_fixture.boundary_record(&[], &[]);
    dev_fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = []
        "#,
    );
    let added_dev_dependency = analyze(&dev_fixture, Some("SCB-MANIFEST-003,SCB-MANIFEST-006"))
        .expect("allowlist policy analyzes development dependencies");
    assert_failed_findings(
        &added_dev_dependency,
        &[(
            RuleId::ScbManifest003,
            "manifest_dependency_not_allowlisted",
            "crates/app/Cargo.toml: remove dependency `dep` or review/add its permission",
        )],
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

    let union_fixture = Fixture::new();
    union_fixture.workspace();
    union_fixture.boundary_record(&[], &[]);
    union_fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
            boundary_record_path = "boundaries/app/app.toml"
        "#,
    );
    let union_mismatch = analyze(&union_fixture, Some("SCB-MANIFEST-006"))
        .expect("record lock compares the record union with the allowlist");
    assert_failed_findings(
        &union_mismatch,
        &[(
            RuleId::ScbManifest006,
            "manifest_policy_inconsistent",
            "crates/app/Cargo.toml: record lock `boundaries/app/app.toml` must align with the allowlist and dependency sections",
        )],
    );

    let owner_fixture = Fixture::new();
    owner_fixture.workspace();
    owner_fixture.boundary_record(&["dep"], &[]);
    owner_fixture.boundary_record_for("dep", "dep", "concrete_landed");
    owner_fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
            boundary_record_path = "boundaries/dep/dep.toml"

            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/dep/Cargo.toml"
            allowed_dependencies = []
        "#,
    );
    let other_owner = analyze(&owner_fixture, Some("SCB-MANIFEST-006"))
        .expect("record lock rejects another owner's record");
    assert_failed_findings(
        &other_owner,
        &[(
            RuleId::ScbManifest006,
            "manifest_policy_inconsistent",
            "crates/app/Cargo.toml: record lock `boundaries/dep/dep.toml` belongs to a different owner",
        )],
    );
}

/// Maps atm-core's `collect_manifest_dependency_allowlist_violations` at
/// commit `f4746d500` to sc-lint's finding contract. Each atm-core violation
/// maps to an sc-lint `(RuleId, kind)`:
/// - a record-lock violation ("boundary record allowed_dependencies
///   diverges..." or "boundary record dependency sections diverge...") ->
///   `(ScbManifest006, "manifest_policy_inconsistent")`;
/// - "Cargo dependency 'x' is not allowlisted" -> `(ScbManifest003,
///   "manifest_dependency_not_allowlisted")`;
/// - "allowlisted dependency 'x' is not declared by Cargo.toml" ->
///   `(ScbManifest006, "manifest_policy_inconsistent")`;
/// - no violation -> no finding.
///
/// With actual runtime `dep` and dev `devdep`, the case mapping is:
/// - record runtime `[dep, devdep]`, dev `[]` -> no finding;
/// - record runtime `[dep]`, dev `[]` -> `(ScbManifest006,
///   "manifest_policy_inconsistent")` (record union omits `devdep`);
/// - record runtime `[dep]`, dev `[devdep]` -> no finding;
/// - record runtime `[dep, devdep]`, dev `[devdep]` -> `(ScbManifest006,
///   "manifest_policy_inconsistent")` (`devdep` is in the wrong record key).
#[test]
fn manifest_allowlist_matches_atm_core_cases() {
    let cases = [
        (
            "allowed_dependencies/allowed",
            &["dep", "devdep"][..],
            &[][..],
            None,
        ),
        (
            "allowed_dependencies/violating",
            &["dep"][..],
            &[][..],
            Some((RuleId::ScbManifest006, "manifest_policy_inconsistent")),
        ),
        (
            "allowed_dev_dependencies/allowed",
            &["dep"][..],
            &["devdep"][..],
            None,
        ),
        (
            "allowed_dev_dependencies/violating",
            &["dep", "devdep"][..],
            &["devdep"][..],
            Some((RuleId::ScbManifest006, "manifest_policy_inconsistent")),
        ),
    ];

    for (name, allowed_dependencies, allowed_dev_dependencies, expected) in cases {
        let fixture = Fixture::new();
        fixture.workspace_with_dev_dependency();
        fixture.boundary_record(allowed_dependencies, allowed_dev_dependencies);
        fixture.write(
            "sc-lint.toml",
            r#"
                [tool.sc-lint.analyzers.boundary]
                [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = ["dep", "devdep"]
                boundary_record_path = "boundaries/app/app.toml"
            "#,
        );

        let report = analyze(&fixture, Some("SCB-MANIFEST-003,SCB-MANIFEST-006"))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected_findings = expected.into_iter().collect::<Vec<_>>();
        let findings = report
            .findings
            .iter()
            .map(|finding| (finding.rule_id, finding.kind.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(findings.len(), expected_findings.len(), "{name}");
        assert_eq!(findings, expected_findings, "{name}");
        assert_eq!(
            report.status,
            if expected_findings.is_empty() {
                ReportStatus::Pass
            } else {
                ReportStatus::Fail
            },
            "{name}"
        );
    }
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

#[test]
fn issue136_optional_config_matrix() {
    // Pre-existing findings the fixture plants: the app manifest drops
    // homepage.workspace and its path dependency on dep carries no version.
    let expected_baseline = [
        (
            RuleId::ScbManifest001,
            "package_workspace_field_required",
            "crates/app/Cargo.toml: set [package].homepage.workspace = true",
        ),
        (
            RuleId::ScbManifest002,
            "internal_path_dependency_version_mismatch",
            "crates/app/Cargo.toml [dependencies.dep]: path dependency version must match target crate version \"0.1.0\"",
        ),
    ];
    let missing_boundaries = Fixture::new();
    missing_boundaries.workspace();
    missing_boundaries.remove_workspace_homepage();
    assert_failed_findings(
        &analyze(&missing_boundaries, Some("manifests"))
            .expect("absent policy analyzes without boundary records"),
        &expected_baseline,
    );

    let absent = Fixture::new();
    absent.workspace();
    absent.remove_workspace_homepage();
    // Invalid inventory: the record sits under `other/` but names owner `dep`.
    absent.boundary_record_for("other", "dep", "concrete_landed");
    let baseline = analyze(&absent, Some("manifests"))
        .expect("absent policy never loads the invalid boundary inventory");
    assert_failed_findings(&baseline, &expected_baseline);

    let inventory_required = Fixture::new();
    inventory_required.workspace();
    inventory_required.boundary_record_for("other", "dep", "concrete_landed");
    inventory_required.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = ["dep"]
            "#,
        ),
    );
    let error = analyze(&inventory_required, Some("manifests"))
        .expect_err("an allowlist policy loads the invalid boundary inventory");
    assert!(matches!(error, BoundaryError::InventoryLoad { .. }));
    let record = std::path::Path::new("boundaries")
        .join("other")
        .join("other.toml");
    assert!(error.to_string().ends_with(&format!(
        "{}` is under owner directory `other` but declares owner_package `dep`",
        record.display()
    )));

    let empty = Fixture::new();
    empty.workspace();
    empty.remove_workspace_homepage();
    empty.boundary_record_for("other", "dep", "concrete_landed");
    empty.write(
        "sc-lint.toml",
        &policy(
            "manifest_dependency_allowlists = []\n\
             global_dependency_ownership = []\n\
             manifest_section_rules = []",
        ),
    );
    let present_empty = analyze(&empty, Some("manifests")).expect("empty policy analyzes");
    assert_eq!(
        serde_json::to_vec(&baseline.findings).expect("baseline serializes"),
        serde_json::to_vec(&present_empty.findings).expect("empty policy serializes"),
        "absent and present-empty policy preserve legacy findings byte-for-byte"
    );

    let enforcement = Fixture::new();
    enforcement.workspace();
    enforcement.boundary_record_for("app", "app", "concrete_landed");
    enforcement.boundary_record_for("dep", "dep", "concrete_landed");
    enforcement.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = []

                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = ["dep"]

                [[global_dependency_ownership]]
                dependency = "dep"
                allowed_manifest_paths = []
                allowed_source_roots = []
                manifest_message = "dep is not declared here"
                source_message = "dep is not imported here"

                [[manifest_section_rules]]
                owner_manifest_path = "crates/app/Cargo.toml"
                dependency_package = "dep"
                allowed_sections = []
                message = "dep is forbidden in every section"
            "#,
        ),
    );
    let enforced = analyze(
        &enforcement,
        Some("SCB-MANIFEST-003,SCB-MANIFEST-004,SCB-MANIFEST-005,SCB-MANIFEST-006"),
    )
    .expect("policy enforces");
    let ids = finding_ids(&enforced);
    assert_eq!(
        enforced.status,
        ReportStatus::Fail,
        "new rules default to fail"
    );
    for rule in [
        RuleId::ScbManifest003,
        RuleId::ScbManifest004,
        RuleId::ScbManifest005,
        RuleId::ScbManifest006,
    ] {
        assert!(ids.contains(&rule), "{rule:?} is enforced");
    }
    assert!(
        enforced
            .findings
            .iter()
            .any(|finding| { finding.message.contains("duplicate allowlist owner") })
    );
    assert!(enforced.findings.iter().any(|finding| {
        finding
            .message
            .contains("missing allowlist entry for active owner")
    }));
    let only_allowlist =
        analyze(&enforcement, Some("SCB-MANIFEST-003")).expect("rule filter selects policy rule");
    assert!(
        only_allowlist
            .findings
            .iter()
            .all(|finding| finding.rule_id == RuleId::ScbManifest003)
    );

    let inactive = Fixture::new();
    inactive.workspace();
    inactive.boundary_record_for("app", "app", "planned");
    inactive.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = ["dep"]
            "#,
        ),
    );
    assert!(
        analyze(&inactive, Some("SCB-MANIFEST-006"))
            .expect("inactive owner analyzes")
            .findings
            .iter()
            .any(|finding| finding.message.contains("inactive allowlist owner"))
    );

    let nonworkspace = Fixture::new();
    nonworkspace.workspace();
    nonworkspace.boundary_record_for("app", "app", "concrete_landed");
    nonworkspace.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/missing/Cargo.toml"
                allowed_dependencies = []
            "#,
        ),
    );
    assert!(
        analyze(&nonworkspace, Some("SCB-MANIFEST-006"))
            .expect("nonworkspace owner analyzes")
            .findings
            .iter()
            .any(|finding| finding.message.contains("non-workspace allowlist owner"))
    );

    for (shape, expected_key, expected_repair) in malformed_policy_shapes() {
        let malformed = Fixture::new();
        malformed.workspace();
        malformed.write("sc-lint.toml", &policy(&shape));
        let error =
            analyze(&malformed, Some("manifests")).expect_err("malformed policy shape is rejected");
        let config_error = match error {
            BoundaryError::AnalyzerConfig(error) => error,
            other => panic!("{shape}: expected analyzer config error, got {other:?}"),
        };
        assert_eq!(config_error.code(), "ANALYZER_CONFIG.WRONG_TYPE", "{shape}");
        match config_error {
            sc_lint_analyzer_support::AnalyzerConfigError::WrongType {
                analyzer,
                key,
                expected,
            } => {
                assert_eq!(analyzer, "boundary", "{shape}");
                assert_eq!(key, expected_key, "{shape}");
                assert_eq!(expected, expected_repair, "{shape}");
            }
            other => panic!("{shape}: expected wrong-type error, got {other:?}"),
        }
    }

    let ignored_nested = Fixture::new();
    ignored_nested.workspace_with_renamed_dependency();
    ignored_nested.boundary_record_for("app", "app", "concrete_landed");
    ignored_nested.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = ["dep"]
                ignored = "ignored"

                [manifest_dependency_allowlists.unused]
                nested = true

                [[manifest_section_rules]]
                owner_manifest_path = "crates/app/Cargo.toml"
                dependency_package = "dep"
                allowed_sections = ["dependencies"]
                message = "dep is allowed here"
            "#,
        ),
    );
    let renamed = analyze(
        &ignored_nested,
        Some("SCB-MANIFEST-003,SCB-MANIFEST-004,SCB-MANIFEST-005,SCB-MANIFEST-006"),
    )
    .expect("unknown nested keys and dependency aliases analyze");
    assert!(
        renamed.findings.is_empty(),
        "alias policy applies to renamed dependencies"
    );

    let excluded = Fixture::new();
    excluded.workspace();
    excluded.boundary_record_for("app", "app", "concrete_landed");
    excluded.write(
        "sc-lint.toml",
        &format!(
            "{}\n\
             [tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]\n\
             \"SCB-MANIFEST-003\" = [\"crates/app/Cargo.toml\"]",
            policy(
                r#"
                    [[manifest_dependency_allowlists]]
                    owner_manifest_path = "crates/app/Cargo.toml"
                    allowed_dependencies = []
                "#,
            )
        ),
    );
    assert!(
        analyze(&excluded, Some("SCB-MANIFEST-003"))
            .expect("path filtering analyzes")
            .findings
            .is_empty()
    );

    let restricted = Fixture::new();
    restricted.workspace();
    restricted.boundary_record_for("app", "app", "concrete_landed");
    restricted.write(
        "sc-lint.toml",
        &policy(
            r#"
                [[manifest_dependency_allowlists]]
                owner_manifest_path = "crates/app/Cargo.toml"
                allowed_dependencies = []
            "#,
        ),
    );
    let unrelated = Fixture::new();
    unrelated.workspace();
    assert!(
        finding_ids(
            &analyze(&restricted, Some("SCB-MANIFEST-003")).expect("first workspace analyzes")
        )
        .contains(&RuleId::ScbManifest003)
    );
    assert!(
        !finding_ids(
            &analyze(&unrelated, Some("SCB-MANIFEST-003")).expect("second workspace analyzes")
        )
        .contains(&RuleId::ScbManifest003)
    );
}

#[test]
fn issue136_rejects_empty_allowlist_dependency_with_indexed_error() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep", "  "]
        "#,
    );

    let error = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect_err("empty dependency names are invalid");
    let message = error.to_string();
    assert!(message.contains("manifest_dependency_allowlists[0].allowed_dependencies[1]"));
    assert!(message.contains("non-empty string"));
}

#[test]
fn issue136_rejects_empty_boundary_record_path_array() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = []
            boundary_record_path = []
        "#,
    );

    let error = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect_err("an explicitly empty boundary-record path list is invalid");
    assert!(
        error
            .to_string()
            .contains("manifest_dependency_allowlists[0].boundary_record_path")
    );
}

#[test]
fn issue136_rejects_empty_boundary_record_path_with_indexed_error() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = []
            boundary_record_path = ["boundaries/app.toml", "  "]
        "#,
    );

    let error = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect_err("empty boundary-record paths are invalid");
    let message = error.to_string();
    assert!(message.contains("manifest_dependency_allowlists[0].boundary_record_path[1]"));
    assert!(message.contains("non-empty string"));
}

#[test]
fn issue136_empty_ownership_paths_and_sections_remain_accepted() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
            dependency = "dep"
            allowed_manifest_paths = [""]
            allowed_source_roots = [""]
            manifest_message = "dep is not declared here"
            source_message = "dep is not imported here"

            [[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
            owner_manifest_path = "crates/app/Cargo.toml"
            dependency_package = "dep"
            allowed_sections = [""]
            message = "dep is forbidden in every section"
        "#,
    );

    analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect("empty strings remain accepted in ownership paths and sections");
}

#[test]
fn issue136_names_inactive_allowlist_owners() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record_with_owner("app", "app", "retired");
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
        "#,
    );

    let messages = analyze_messages(&fixture);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("inactive allowlist owner"))
    );
}

#[test]
fn issue136_names_duplicate_allowlist_owners() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record(&[], &[]);
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]

            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
        "#,
    );

    let messages = analyze_messages(&fixture);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("duplicate allowlist owner entry"))
    );
}

#[test]
fn issue136_names_non_workspace_allowlist_owners() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record(&[], &[]);
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/missing/Cargo.toml"
            allowed_dependencies = []
        "#,
    );

    let messages = analyze_messages(&fixture);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("non-workspace allowlist owner"))
    );
}

#[test]
fn issue136_reports_missing_allowlist_for_directory_alias_owner() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.write(
        "crates/app/Cargo.toml",
        &package(
            "application",
            "[dependencies]\ndep = { path = \"../dep\" }\n",
        ),
    );
    fixture.boundary_record_with_owner("app", "application", "concrete_landed");
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/dep/Cargo.toml"
            allowed_dependencies = []
        "#,
    );

    let messages = analyze_messages(&fixture);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing allowlist entry for active owner"))
    );
}

#[test]
fn issue136_one_allowlist_covers_multiple_active_records_for_an_owner() {
    let fixture = Fixture::new();
    fixture.workspace();
    fixture.boundary_record(&[], &[]);
    fixture.write_boundary_record(BoundaryRecord {
        relative: "boundaries/app/extra.toml",
        boundary_id: "BOUNDARY-App-Extra",
        owner_package: "app",
        owner_crate: "app",
        name: "App Extra",
        state: "concrete_landed",
        allowed_dependencies: &[],
        allowed_dev_dependencies: &[],
    });
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
            owner_manifest_path = "crates/app/Cargo.toml"
            allowed_dependencies = ["dep"]
        "#,
    );

    let messages = analyze_messages(&fixture);
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("inactive allowlist owner"))
    );
    assert!(!messages.iter().any(|message| {
        message.contains("exactly one active boundary record")
            || message.contains("multiple active boundary records")
    }));
}

struct Fixture(TempDir);

struct BoundaryRecord<'a> {
    relative: &'a str,
    boundary_id: &'a str,
    owner_package: &'a str,
    owner_crate: &'a str,
    name: &'a str,
    state: &'a str,
    allowed_dependencies: &'a [&'a str],
    allowed_dev_dependencies: &'a [&'a str],
}

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
    fn workspace_with_dev_dependency(&self) {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/app\", \"crates/dep\", \"crates/devdep\"]\nresolver = \"2\"\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.85\"\nauthors = [\"test\"]\nlicense = \"MIT\"\nrepository = \"https://example.invalid\"\nhomepage = \"https://example.invalid\"\n",
        );
        for dependency in ["dep", "devdep"] {
            self.write(
                &format!("crates/{dependency}/Cargo.toml"),
                &package(dependency, ""),
            );
            self.write(
                &format!("crates/{dependency}/src/lib.rs"),
                "pub struct Dep;\n",
            );
        }
        self.write(
            "crates/app/Cargo.toml",
            &package(
                "app",
                "[dependencies]\ndep = { path = \"../dep\" }\n\n[dev-dependencies]\ndevdep = { path = \"../devdep\" }\n",
            ),
        );
        self.write("crates/app/src/lib.rs", "pub struct App;\n");
    }
    fn boundary_record(&self, allowed_dependencies: &[&str], allowed_dev_dependencies: &[&str]) {
        self.write_boundary_record(BoundaryRecord {
            relative: "boundaries/app/app.toml",
            boundary_id: "BOUNDARY-App",
            owner_package: "app",
            owner_crate: "app",
            name: "App",
            state: "concrete_landed",
            allowed_dependencies,
            allowed_dev_dependencies,
        });
    }
    fn boundary_record_with_owner(&self, owner_package: &str, owner_crate: &str, state: &str) {
        self.write_boundary_record(BoundaryRecord {
            relative: "boundaries/app/app.toml",
            boundary_id: "BOUNDARY-App",
            owner_package,
            owner_crate,
            name: "App",
            state,
            allowed_dependencies: &[],
            allowed_dev_dependencies: &[],
        });
    }
    fn write_boundary_record(&self, record: BoundaryRecord<'_>) {
        let allowed_dependencies = toml_array(record.allowed_dependencies);
        let allowed_dev_dependencies = toml_array(record.allowed_dev_dependencies);
        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"K.4\"\n",
        );
        self.write(
            record.relative,
            &format!(
                r#"
                boundary_id = "{}"
                owner_package = "{}"
                owner_crate_path = "{}"
                name = "{}"

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
                state = "{}"
            "#,
                record.boundary_id,
                record.owner_package,
                record.owner_crate,
                record.name,
                record.state,
            ),
        );
    }
    fn workspace_with_renamed_dependency(&self) {
        self.workspace();
        self.write(
            "crates/app/Cargo.toml",
            &package(
                "app",
                "[dependencies]\nrenamed = { package = \"dep\", path = \"../dep\" }\n",
            ),
        );
        self.write(
            "crates/app/src/lib.rs",
            "use renamed::Dep;\npub fn make() -> Dep { Dep }\n",
        );
    }
    fn remove_workspace_homepage(&self) {
        let manifest =
            fs::read_to_string(self.root().join("crates/app/Cargo.toml")).expect("app manifest");
        self.write(
            "crates/app/Cargo.toml",
            &manifest.replace("homepage.workspace = true\n", ""),
        );
    }
    fn boundary_record_for(&self, name: &str, owner: &str, state: &str) {
        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"K.4\"\n",
        );
        self.write(
            &format!("boundaries/{name}/{name}.toml"),
            &format!(
                r#"
                boundary_id = "BOUNDARY-{name}"
                owner_package = "{owner}"
                owner_crate_path = "{owner}"
                name = "{name}"

                [public]
                facade = "{name}"

                [implementation]
                type = "{name}"
                module = "{owner}"
                visibility = "public"
                constructor = "none"

                [composition]
                roots = ["{name}"]

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
                state = "{state}"
            "#,
            ),
        );
    }
}

/// Every policy family's malformed shapes: wrong outer type, non-table entry,
/// each missing required field and each wrongly typed field, with the exact
/// error the parser reports.
fn malformed_policy_shapes() -> Vec<(String, String, &'static str)> {
    let mut shapes = Vec::new();
    for key in [
        "manifest_dependency_allowlists",
        "global_dependency_ownership",
        "manifest_section_rules",
    ] {
        for outer in ["\"invalid\"", "{ owner = \"app\" }", "3"] {
            shapes.push((
                format!("{key} = {outer}"),
                key.to_owned(),
                "an array of tables (the policy field has the wrong TOML type; use array-of-table entries)",
            ));
        }
        shapes.push((
            format!("{key} = [\"invalid\"]"),
            format!("{key}[0]"),
            "a table (the indexed policy item has the wrong TOML type; replace it with a table entry)",
        ));
    }
    shapes.extend([
        (
            "[[manifest_dependency_allowlists]]\nallowed_dependencies = []",
            "manifest_dependency_allowlists[0].owner_manifest_path",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[manifest_dependency_allowlists]]\nowner_manifest_path = 1\nallowed_dependencies = []",
            "manifest_dependency_allowlists[0].owner_manifest_path",
            "a non-empty string (the value has the wrong TOML type; provide a quoted string)",
        ),
        (
            "[[manifest_dependency_allowlists]]\nowner_manifest_path = \"crates/app/Cargo.toml\"",
            "manifest_dependency_allowlists[0].allowed_dependencies",
            "an array of non-empty strings (the required field is missing; add quoted string values in an array)",
        ),
        (
            "[[manifest_dependency_allowlists]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\nallowed_dependencies = \"dep\"",
            "manifest_dependency_allowlists[0].allowed_dependencies",
            "an array of strings (the value has the wrong TOML type; use an array of quoted strings)",
        ),
        (
            "[[manifest_dependency_allowlists]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\nallowed_dependencies = [1]",
            "manifest_dependency_allowlists[0].allowed_dependencies[0]",
            "a string (the array item has the wrong TOML type; replace it with a quoted string)",
        ),
        (
            "[[manifest_dependency_allowlists]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\nallowed_dependencies = []\nboundary_record_path = 1",
            "manifest_dependency_allowlists[0].boundary_record_path",
            "a non-empty string or an array of non-empty strings (the value has the wrong TOML type; provide a repository-relative path or an array of paths)",
        ),
        (
            "[[global_dependency_ownership]]\nmanifest_message = \"m\"\nsource_message = \"s\"",
            "global_dependency_ownership[0].dependency",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[global_dependency_ownership]]\ndependency = 1\nmanifest_message = \"m\"\nsource_message = \"s\"",
            "global_dependency_ownership[0].dependency",
            "a non-empty string (the value has the wrong TOML type; provide a quoted string)",
        ),
        (
            "[[global_dependency_ownership]]\ndependency = \"dep\"\nsource_message = \"s\"",
            "global_dependency_ownership[0].manifest_message",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[global_dependency_ownership]]\ndependency = \"dep\"\nmanifest_message = \"m\"",
            "global_dependency_ownership[0].source_message",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[global_dependency_ownership]]\ndependency = \"dep\"\nallowed_manifest_paths = \"x\"\nmanifest_message = \"m\"\nsource_message = \"s\"",
            "global_dependency_ownership[0].allowed_manifest_paths",
            "an array of non-empty strings (the value has the wrong TOML type; use an array of repository-relative paths)",
        ),
        (
            "[[global_dependency_ownership]]\ndependency = \"dep\"\nallowed_source_roots = [1]\nmanifest_message = \"m\"\nsource_message = \"s\"",
            "global_dependency_ownership[0].allowed_source_roots[0]",
            "a string (the array item has the wrong TOML type; replace it with a quoted string)",
        ),
        (
            "[[manifest_section_rules]]\ndependency_package = \"dep\"\nmessage = \"m\"",
            "manifest_section_rules[0].owner_manifest_path",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[manifest_section_rules]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\nmessage = \"m\"",
            "manifest_section_rules[0].dependency_package",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[manifest_section_rules]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\ndependency_package = \"dep\"",
            "manifest_section_rules[0].message",
            "a non-empty string (the required field is missing; add a quoted string value)",
        ),
        (
            "[[manifest_section_rules]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\ndependency_package = \"dep\"\nmessage = false",
            "manifest_section_rules[0].message",
            "a non-empty string (the value has the wrong TOML type; provide a quoted string)",
        ),
        (
            "[[manifest_section_rules]]\nowner_manifest_path = \"crates/app/Cargo.toml\"\ndependency_package = \"dep\"\nallowed_sections = \"dependencies\"\nmessage = \"m\"",
            "manifest_section_rules[0].allowed_sections",
            "an array of strings (the value has the wrong TOML type; use an array of quoted strings)",
        ),
    ]
    .map(|(shape, key, expected)| (shape.to_owned(), key.to_owned(), expected)));
    shapes
}

fn analyze(
    fixture: &Fixture,
    rule_filter: Option<&str>,
) -> Result<sc_lint_boundary::FindingsReport, sc_lint_boundary::BoundaryError> {
    analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        rule_filter,
    )
}

fn finding_ids(report: &sc_lint_boundary::FindingsReport) -> Vec<RuleId> {
    report
        .findings
        .iter()
        .map(|finding| finding.rule_id)
        .collect()
}

fn policy(contents: &str) -> String {
    let contents = contents
        .replace(
            "[[manifest_dependency_allowlists]]",
            "[[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]",
        )
        .replace(
            "[[global_dependency_ownership]]",
            "[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]",
        )
        .replace(
            "[[manifest_section_rules]]",
            "[[tool.sc-lint.analyzers.boundary.manifest_section_rules]]",
        )
        .replace(
            "[manifest_dependency_allowlists.",
            "[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists.",
        );
    format!("[tool.sc-lint.analyzers.boundary]\n{contents}")
}

fn analyze_messages(fixture: &Fixture) -> Vec<String> {
    analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("manifests"),
    )
    .expect("configured manifest policy analyzes")
    .findings
    .into_iter()
    .map(|finding| finding.message)
    .collect()
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
