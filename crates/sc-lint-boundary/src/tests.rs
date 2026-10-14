#![cfg(test)]

use super::*;
use crate::render::hex_encode;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn installer_catalog() -> serde_json::Value {
    let families = BOUNDARY_CATALOG
        .families
        .iter()
        .map(|(name, rules)| {
            (
                *name,
                rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "analyzer": BOUNDARY_CATALOG.analyzer.as_str(),
        "rules": BOUNDARY_CATALOG.rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
        "families": families,
        "own_keys": BOUNDARY_CATALOG.own_keys,
    })
}

#[test]
#[ignore]
fn emit_installer_catalog() {
    let output = std::env::var("SC_LINT_INSTALLER_CATALOG_OUTPUT")
        .expect("catalog output path must be provided");
    fs::write(
        output,
        serde_json::to_vec_pretty(&installer_catalog()).unwrap(),
    )
    .unwrap();
}

#[test]
fn installer_catalog_matches_rust() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/sc-lint-adoption/analyzer-catalog/boundary.json");
    let actual: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

    assert_eq!(actual, installer_catalog());
}

fn fixture_display_path(relative_path: &str) -> String {
    std::env::temp_dir()
        .join("sc-lint-boundary-tests")
        .join(relative_path)
        .display()
        .to_string()
}

#[test]
fn malformed_manifest_owner_has_no_repository_relative_path() {
    let finding = Finding {
        rule_id: RuleId::ScbManifest001,
        kind: "manifest_error".into(),
        message: "invalid manifest path".to_string(),
        owner_ids: vec![OwnerId::new("/outside/Cargo.toml")],
        node_ids: Vec::new(),
    };

    assert!(manifest_finding_path(&finding).is_none());
}

#[test]
fn findings_report_text_is_stable() {
    let report = super::FindingsReport {
        tool: "sc-lint-boundary",
        version: "0.1.0",
        schema_version: "0.2.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: Vec::new(),
    };
    assert_eq!(
        render_findings_report(&report),
        "sc-lint-boundary 0.1.0 status=pass scanned_crates=2 findings=0"
    );
}

#[test]
fn graph_export_serializes_tool_metadata() {
    let graph = GraphExport {
        tool: "sc-lint-boundary",
        version: "0.1.0",
        schema_version: "0.2.0",
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let json = serde_json::to_string(&graph).unwrap();
    assert!(json.contains("\"tool\":\"sc-lint-boundary\""));
    assert!(json.contains("\"version\":\"0.1.0\""));
    assert!(json.contains("\"schema_version\":\"0.2.0\""));
}

#[test]
fn render_graph_export_json_includes_nodes_edges_and_optional_fields() {
    let graph = GraphExport {
        tool: "sc-lint-boundary",
        version: "0.1.0",
        schema_version: "0.2.0",
        nodes: vec![GraphNode {
            id: NodeId::new("crate::example::example"),
            kind: "type",
            label: "Example".to_string(),
            visibility: Some("public"),
            package: "example".to_string(),
            target: Some("example".to_string()),
            // Ephemeral fixture path, not a real workspace root.
            manifest_path: fixture_display_path("example/Cargo.toml"),
            // Ephemeral fixture path, not a real workspace source file.
            source_path: Some(fixture_display_path("example/src/lib.rs")),
            module_path: Some("crate::example".to_string()),
            impl_kind: Some(ImplKind::Inherent),
            impl_trait: Some("crate::Api".to_string()),
            attributes: vec![LintAttribute {
                scope: "boundary",
                name: "internal_only",
                values: Vec::new(),
            }],
        }],
        edges: vec![GraphEdge {
            kind: "contains",
            from: NodeId::new("crate::example"),
            to: NodeId::new("crate::example::example"),
        }],
    };

    let rendered = render_graph_export(&graph, GraphOutputFormat::Json);
    let json: serde_json::Value = serde_json::from_str(&rendered).expect("graph json");

    assert_eq!(json["tool"], "sc-lint-boundary");
    assert_eq!(json["nodes"][0]["label"], "Example");
    assert_eq!(json["nodes"][0]["impl_kind"], "inherent");
    assert_eq!(json["edges"][0]["kind"], "contains");
}

#[test]
fn render_graph_export_turtle_escapes_special_characters_and_attributes() {
    let graph = GraphExport {
        tool: "sc-lint-boundary",
        version: "0.1.0",
        schema_version: "0.2.0",
        nodes: vec![GraphNode {
            id: NodeId::new("crate::example::example"),
            kind: "type",
            label: "Example \"quoted\"\nline".to_string(),
            visibility: Some("public"),
            package: "example".to_string(),
            target: Some("example".to_string()),
            manifest_path: "C:\\repo\\Cargo.toml".to_string(),
            // Ephemeral fixture path, not a real workspace source file.
            source_path: Some(fixture_display_path("example/src/lib.rs")),
            module_path: Some("crate::example".to_string()),
            impl_kind: Some(ImplKind::Trait),
            impl_trait: Some("crate::Api".to_string()),
            attributes: vec![LintAttribute {
                scope: "boundary",
                name: "allow",
                values: vec!["cycle.type_method_self_loop".to_string()],
            }],
        }],
        edges: vec![GraphEdge {
            kind: "contains",
            from: NodeId::new("crate::example"),
            to: NodeId::new("crate::example::example"),
        }],
    };

    let rendered = render_graph_export_turtle(&graph);

    assert!(rendered.contains("sc:label \"Example \\\"quoted\\\"\\nline\" ."));
    assert!(rendered.contains("sc:manifestPath \"C:\\\\repo\\\\Cargo.toml\" ."));
    assert!(rendered.contains("sc:implKind \"trait\" ."));
    assert!(rendered.contains("sc:attribute \"boundary.allow(cycle.type_method_self_loop)\" ."));
    assert!(rendered.contains(" sc:contains "));
}

#[test]
fn exports_graph_for_inline_and_file_modules_and_attributes() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.internal_only)]
                pub struct Root;

                mod file_mod;

                mod inline_mod {
                    #[sc_lint(boundary.allow("cycle.type_method_self_loop"))]
                    pub struct InlineType;

                    impl InlineType {
                        #[sc_lint(boundary.allow("cycle.type_method_self_loop"))]
                        pub fn helper(&self) -> InlineType { InlineType }
                    }
                }
            "#,
    );
    fixture.write_source(
        "example",
        "file_mod.rs",
        r#"
                pub mod nested;
                pub trait Worker {}
            "#,
    );
    fixture.write_source("example", "file_mod/nested.rs", "pub struct FileType;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.id == "crate::example::example")
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.id == "crate::example::example::module::crate::inline_mod")
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.id == "crate::example::example::module::crate::file_mod::nested")
    );

    let root_type = graph
        .nodes
        .iter()
        .find(|node| node.id == "crate::example::example::module::crate::Root")
        .unwrap();
    assert_eq!(
        root_type.attributes,
        vec![LintAttribute {
            scope: "boundary",
            name: "internal_only",
            values: Vec::new(),
        }]
    );

    let inline_type = graph
        .nodes
        .iter()
        .find(|node| node.id == "crate::example::example::module::crate::inline_mod::InlineType")
        .unwrap();
    assert_eq!(
        inline_type.attributes,
        vec![LintAttribute {
            scope: "boundary",
            name: "allow",
            values: vec!["cycle.type_method_self_loop".to_string()],
        }]
    );

    let helper_method = graph
        .nodes
        .iter()
        .find(|node| {
            node.id == "crate::example::example::module::crate::inline_mod::InlineType::helper"
        })
        .unwrap();
    assert_eq!(helper_method.kind, "method");
    assert_eq!(
        helper_method.attributes,
        vec![LintAttribute {
            scope: "boundary",
            name: "allow",
            values: vec!["cycle.type_method_self_loop".to_string()],
        }]
    );
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "declares"
            && edge.from == "crate::example::example::module::crate::inline_mod::InlineType"
            && edge.to == "crate::example::example::module::crate::inline_mod::InlineType::helper"
    }));
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references"
            && edge.from == "crate::example::example::module::crate::inline_mod::InlineType::helper"
            && edge.to == "crate::example::example::module::crate::inline_mod::InlineType"
    }));
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_expr"
            && edge.from == "crate::example::example::module::crate::inline_mod::InlineType::helper"
            && edge.to == "crate::example::example::module::crate::inline_mod::InlineType"
    }));
    assert!(!graph.edges.iter().any(|edge| {
        edge.from == "crate::example::example::module::crate::inline_mod::InlineType::helper"
            && edge.to == "crate::example::example::module::crate::inline_mod::self"
    }));
}

#[test]
fn exports_field_and_variant_nodes_for_type_graph() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Wrapper {
                    pub value: Inner,
                }

                pub struct Inner;

                pub enum Choice {
                    Unit,
                    Pair(Inner),
                }
            "#,
    );

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(graph.nodes.iter().any(|node| {
        node.id == "crate::example::example::module::crate::Wrapper::field::value"
            && node.kind == "field"
            && node.visibility == Some("public")
    }));
    assert!(graph.nodes.iter().any(|node| {
        node.id == "crate::example::example::module::crate::Choice::variant::Pair"
            && node.kind == "variant"
    }));
    assert!(graph.nodes.iter().any(|node| {
        node.id == "crate::example::example::module::crate::Choice::variant::Pair::field::0"
            && node.kind == "field"
    }));
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_type"
            && edge.from == "crate::example::example::module::crate::Wrapper::field::value"
            && edge.to == "crate::example::example::module::crate::Inner"
    }));
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "contains"
            && edge.from == "crate::example::example::module::crate::Choice::variant::Pair"
            && edge.to == "crate::example::example::module::crate::Choice::variant::Pair::field::0"
    }));
}

#[test]
fn renders_graph_as_turtle() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let turtle = render_graph_export_turtle(&graph);

    assert!(turtle.contains("@prefix sc: <urn:sc-lint-boundary:predicate:> ."));
    assert!(turtle.contains("rdf:type sc:type ."));
    assert!(turtle.contains("sc:visibility \"public\" ."));
    assert!(turtle.contains("sc:label \"Example\" ."));
    assert!(turtle.contains("sc:schemaVersion \"0.2.0\" ."));
}

#[test]
fn resolves_mod_rs_module_layout() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "mod outer;");
    fixture.write_source("example", "outer/mod.rs", "mod child;");
    fixture.write_source("example", "outer/child.rs", "pub struct Nested;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(
        graph.nodes.iter().any(|node| {
            node.id == "crate::example::example::module::crate::outer::child::Nested"
        })
    );
}

#[test]
fn resolves_path_attribute_module_layout() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[path = \"support/aliased.rs\"] mod custom;",
    );
    fixture.write_source("example", "support/aliased.rs", "pub struct FromPathAttr;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(
        graph.nodes.iter().any(|node| {
            node.id == "crate::example::example::module::crate::custom::FromPathAttr"
        })
    );
}

#[test]
fn resolves_nested_submodule_inside_path_attribute_module() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[path = \"support/aliased.rs\"] mod custom;",
    );
    fixture.write_source("example", "support/aliased.rs", "mod nested;");
    fixture.write_source("example", "support/nested.rs", "pub struct NestedFromPath;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(graph.nodes.iter().any(|node| {
        node.id == "crate::example::example::module::crate::custom::nested::NestedFromPath"
    }));
}

#[test]
fn fails_when_path_attribute_file_is_missing() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[path = \"support/missing.rs\"] mod custom;",
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("while resolving module `crate::custom`"));
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
        rule: None,
    })
    .unwrap();

    assert_eq!(report.scanned_crates, 1);
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn rejects_unknown_rule_filter() {
    let error = RuleFilter::try_from("unknown").unwrap_err();
    assert_eq!(
        error.to_string(),
        "unsupported rule filter `unknown`; supported: cycles, boundaries, internal_only, forbid_external_impls, dependencies, manifests"
    );
}

#[test]
fn dependency_policy_allows_direct_workspace_dependency() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &["api"], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn issue376_union_matrix() {
    let (fallback_fixture, fallback_options) = issue376_fixture(false);
    assert_issue376_union_row(
        &fallback_fixture,
        &fallback_options,
        &["q"],
        None,
        &["r_lib"],
        None,
    );
    assert_issue376_union_row(
        &fallback_fixture,
        &fallback_options,
        &["q"],
        Some(&[]),
        &["r_lib"],
        None,
    );

    let (fixture, options) = issue376_fixture(true);
    assert_issue376_union_row(
        &fixture,
        &options,
        &["q"],
        Some(&["s_lib"]),
        &["r-pkg"],
        None,
    );
    write_policy_record(&fixture, "p/a.toml", "A", &["q"], Some(&[]), "active");
    write_policy_record(&fixture, "p/b.toml", "B", &[], Some(&[]), "active");
    let report = analyze_workspace(&options).expect("missing dev grants analyze");
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::ScbDependency002)
            .count(),
        2,
        "only dev-only r and s edges should be rejected: {:#?}",
        report.findings
    );

    assert_issue376_union_row(
        &fixture,
        &options,
        &["q"],
        Some(&["s_lib"]),
        &["r-pkg"],
        None,
    );
    let record = fixture.root().join("boundaries/p/a.toml");
    let contents = fs::read_to_string(&record).expect("read policy record");
    fs::write(
        record,
        contents.replace(
            "forbidden_edges = []",
            "forbidden_edges = [{ from = \"q\", to = \"p\" }]",
        ),
    )
    .expect("write forbidden policy");
    let report = analyze_workspace(&options).expect("forbidden edge analyzes");
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::ScbDependency002)
            .count(),
        0,
        "the incoming permissions remain valid: {:#?}",
        report.findings
    );
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::ScbDependency003)
            .count(),
        1,
        "forbidden edges take precedence over allowlists: {:#?}",
        report.findings
    );
}

fn issue376_fixture(with_explicit_dev: bool) -> (WorkspaceFixture, AnalyzeOptions) {
    let fixture = WorkspaceFixture::new();
    let mut members = vec!["crates/p", "crates/q", "crates/r"];
    if with_explicit_dev {
        members.push("crates/s");
    }
    fixture.write_workspace_root_with_members(&members);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write(
        "crates/q/Cargo.toml",
        &identity_member_manifest(
            "q-pkg",
            "q_lib",
            "[dependencies]\np_alias = { package = \"p\", path = \"../p\", version = \"0.1.0\" }\n\n[dev-dependencies]\np_alias = { package = \"p\", path = \"../p\", version = \"0.1.0\" }",
        ),
    );
    fixture.write(
        "crates/r/Cargo.toml",
        &identity_member_manifest(
            "r-pkg",
            "r_lib",
            "[dev-dependencies]\np_alias = { package = \"p\", path = \"../p\", version = \"0.1.0\" }",
        ),
    );
    if with_explicit_dev {
        fixture.write(
            "crates/s/Cargo.toml",
            &identity_member_manifest(
                "s-pkg",
                "s_lib",
                "[dev-dependencies]\np_alias = { package = \"p\", path = \"../p\", version = \"0.1.0\" }",
            ),
        );
    }
    for package in members
        .iter()
        .map(|member| member.trim_start_matches("crates/"))
    {
        fixture.write_source(package, "lib.rs", "pub struct Fixture;");
    }
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };
    (fixture, options)
}

fn assert_issue376_union_row(
    fixture: &WorkspaceFixture,
    options: &AnalyzeOptions,
    first_runtime: &[&str],
    first_dev: Option<&[&str]>,
    second_runtime: &[&str],
    second_dev: Option<&[&str]>,
) {
    write_policy_record(fixture, "p/a.toml", "A", first_runtime, first_dev, "active");
    write_policy_record(
        fixture,
        "p/b.toml",
        "B",
        second_runtime,
        second_dev,
        "active",
    );
    let report = analyze_workspace(options).expect("union policy analyzes");
    assert_eq!(report.status, ReportStatus::Pass, "{:#?}", report.findings);
}

#[test]
fn issue376_stale_active_gate() {
    for (state, expects_stale) in [
        ("active", true),
        ("stub_landed", true),
        ("concrete_landed", true),
        ("unix_implemented_windows_pending", true),
        ("planned", false),
        ("retired", false),
        ("reserved_future", false),
    ] {
        let fixture = WorkspaceFixture::new();
        fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
        fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
        fixture.write("crates/q/Cargo.toml", &good_member_manifest("q", ""));
        fixture.write_source("p", "lib.rs", "pub struct P;");
        fixture.write_source("q", "lib.rs", "pub struct Q;");
        write_policy_record(&fixture, "p/a.toml", "A", &["q"], None, state);
        write_policy_record(&fixture, "p/b.toml", "B", &["q"], None, state);

        let report = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: Some(RuleFilter::Dependencies),
        })
        .expect("stale policy analyzes");
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.rule_id == RuleId::ScbDependency004)
                .count(),
            usize::from(expects_stale),
            "state={state}: {:#?}",
            report.findings
        );
    }
}

#[test]
fn issue376_stale_dependent_diagnostic_has_record_cause_category_and_recovery() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q", "crates/r"]);
    for package in ["p", "q", "r"] {
        fixture.write(
            &format!("crates/{package}/Cargo.toml"),
            &good_member_manifest(package, ""),
        );
        fixture.write_source(package, "lib.rs", "pub struct Fixture;");
    }
    write_policy_record(&fixture, "p/a.toml", "A", &["q"], None, "active");
    write_policy_record(&fixture, "p/b.toml", "B", &[], Some(&["r"]), "active");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("stale policy analyzes");
    let stale = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::ScbDependency004)
        .collect::<Vec<_>>();
    assert_eq!(stale.len(), 2);

    let runtime_and_fallback_dev = stale
        .iter()
        .find(|finding| finding.message.contains("`q`"))
        .expect("runtime allowlist permission is reported");
    let dev_only = stale
        .iter()
        .find(|finding| finding.message.contains("`r`"))
        .expect("dev allowlist permission is reported");
    for (finding, path, field, categories) in [
        (
            runtime_and_fallback_dev,
            fixture.root().join("boundaries/p/a.toml"),
            "dependencies.allowed_dependents",
            "runtime and fallback dev",
        ),
        (
            dev_only,
            fixture.root().join("boundaries/p/b.toml"),
            "dependencies.allowed_dev_dependents",
            "dev",
        ),
    ] {
        assert!(finding.message.contains(&path.display().to_string()));
        assert!(finding.message.contains(field));
        assert!(finding.message.contains(categories));
        assert!(
            finding
                .message
                .contains("no live direct workspace edge exists in any granted category")
        );
        assert!(finding.message.contains(
            "remove the obsolete permission or restore the intended direct workspace edge"
        ));
    }
}

#[test]
fn issue376_owner_unions_explicit_dev_permissions_before_stale_checking() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write(
        "crates/q/Cargo.toml",
        &(good_member_manifest("q", "")
            + "[dev-dependencies]\np = { path = \"../p\", version = \"0.1.0\" }\n"),
    );
    fixture.write_source("p", "lib.rs", "pub struct P;");
    fixture.write_source("q", "lib.rs", "pub struct Q;");
    write_policy_record(
        &fixture,
        "p/runtime.toml",
        "Runtime",
        &["q"],
        Some(&[]),
        "active",
    );
    write_policy_record(
        &fixture,
        "p/development.toml",
        "Development",
        &["q"],
        Some(&["q"]),
        "active",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("owner policy analyzes");
    assert_eq!(report.status, ReportStatus::Pass, "{:#?}", report.findings);
}

#[test]
fn issue376_owner_level_active_gate_retains_planned_record_permissions() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write(
        "crates/q/Cargo.toml",
        &(good_member_manifest("q", "")
            + "[dev-dependencies]\np = { path = \"../p\", version = \"0.1.0\" }\n"),
    );
    fixture.write_source("p", "lib.rs", "pub struct P;");
    fixture.write_source("q", "lib.rs", "pub struct Q;");
    write_policy_record(
        &fixture,
        "p/active.toml",
        "Active",
        &["q"],
        Some(&[]),
        "active",
    );
    write_policy_record(
        &fixture,
        "p/planned.toml",
        "Planned",
        &["q"],
        Some(&["q"]),
        "planned",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("mixed-state owner policy analyzes");
    assert_eq!(report.status, ReportStatus::Pass, "{:#?}", report.findings);
}

#[test]
fn issue377_semantic_mutations() {
    // Valid Rust raw identifiers, owner-local roots, and roots in an explicitly
    // allowed dependency are all accepted by the semantic inventory pass.
    let valid_fixture = WorkspaceFixture::new();
    valid_fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    valid_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    valid_fixture.write("crates/q/Cargo.toml", &good_member_manifest("q", ""));
    valid_fixture.write_source("p", "lib.rs", "pub struct P;");
    valid_fixture.write_source("q", "lib.rs", "pub struct Q;");
    write_policy_record(&valid_fixture, "p/a.toml", "A", &[], None, "active");
    let valid_record = valid_fixture.root().join("boundaries/p/a.toml");
    let valid_contents = fs::read_to_string(&valid_record).expect("read valid record");
    fs::write(
        &valid_record,
        valid_contents
            .replace(
                "roots = [\"run\"]",
                "roots = [\"r#type\", \"p::LocalRoot\", \"q::ExternalRoot\"]",
            )
            .replace(
                "allowed_dependencies = []",
                "allowed_dependencies = [\"q\"]",
            ),
    )
    .expect("write valid roots");
    let valid_report = analyze_workspace(&AnalyzeOptions {
        root: valid_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("valid roots analyze");
    assert_eq!(
        valid_report.status,
        ReportStatus::Pass,
        "{:#?}",
        valid_report.findings
    );
    assert!(
        valid_report.findings.is_empty(),
        "{:#?}",
        valid_report.findings
    );

    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write("crates/q/Cargo.toml", &good_member_manifest("q", ""));
    fixture.write_source("p", "lib.rs", "pub struct P;");
    fixture.write_source("q", "lib.rs", "pub struct Q;");
    write_policy_record(&fixture, "p/a.toml", "A", &[], None, "active");
    write_policy_record(&fixture, "p/b.toml", "B", &[], None, "active");
    let first = fixture.root().join("boundaries/p/a.toml");
    let first_contents = fs::read_to_string(&first).expect("read first record");
    fs::write(
        &first,
        first_contents
            .replace("facade = \"run\"", "facade = \"not-valid\"")
            .replace(
                "type = \"Runner\"\nmodule = \"p\"\nvisibility = \"public\"\nconstructor = \"none\"",
                "visibility = \"trait_only\"\nconstructor = \"public\"",
            )
            .replace("roots = [\"run\"]", "roots = [\"api::\", \"q::Root\"]")
            .replace("forbidden_edges = []", "forbidden_edges = [\"p -> p\"]"),
    )
    .expect("write first record");
    let second = fixture.root().join("boundaries/p/b.toml");
    let second_contents = fs::read_to_string(&second).expect("read second record");
    fs::write(
        &second,
        second_contents.replace("name = \"B\"", "name = \"A\""),
    )
    .expect("write second record");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("semantic defects are findings");
    let kinds = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::ScbBoundary004)
        .map(|finding| finding.kind.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        kinds,
        BTreeSet::from([
            "duplicate_owner_name",
            "invalid_identifier",
            "invalid_public_path",
            "self_forbidden_edge",
            "trait_only_constructor",
            "unapproved_composition_root",
        ])
    );
    assert_eq!(report.status, ReportStatus::Fail);

    let diagnostic_contexts = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::ScbBoundary004)
        .map(|finding| {
            let field = finding
                .message
                .split(" field `")
                .nth(1)
                .and_then(|tail| tail.split('`').next())
                .expect("diagnostic includes field context");
            (finding.kind.as_str(), field.to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostic_contexts,
        vec![
            ("invalid_public_path", "composition.roots[0]".into()),
            ("unapproved_composition_root", "composition.roots[1]".into()),
            ("self_forbidden_edge", "dependencies.forbidden_edges".into()),
            (
                "trait_only_constructor",
                "implementation.constructor".into()
            ),
            ("invalid_identifier", "public".into()),
            ("duplicate_owner_name", "name".into()),
        ],
        "diagnostics retain deterministic record/field context: {:#?}",
        report.findings
    );
    assert!(report.findings.iter().all(|finding| {
        finding.message.contains("boundary `BOUNDARY-PA`")
            || finding.message.contains("boundary `BOUNDARY-PB`")
    }));

    let duplicate_fixture = WorkspaceFixture::new();
    duplicate_fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    duplicate_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    duplicate_fixture.write("crates/q/Cargo.toml", &good_member_manifest("q", ""));
    duplicate_fixture.write_source("p", "lib.rs", "pub struct P;");
    duplicate_fixture.write_source("q", "lib.rs", "pub struct Q;");
    write_policy_record(&duplicate_fixture, "p/a.toml", "A", &["q"], None, "active");
    write_policy_record(&duplicate_fixture, "p/b.toml", "B", &["q"], None, "active");
    let duplicate_record = duplicate_fixture.root().join("boundaries/p/b.toml");
    let duplicate_contents = fs::read_to_string(&duplicate_record).expect("read duplicate record");
    fs::write(
        &duplicate_record,
        duplicate_contents.replace("name = \"B\"", "name = \"A\""),
    )
    .expect("write duplicate owner name");
    let duplicate_report = analyze_workspace(&AnalyzeOptions {
        root: duplicate_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("duplicate names are semantic findings");
    assert_eq!(duplicate_report.status, ReportStatus::Fail);
    assert_eq!(
        duplicate_report
            .findings
            .iter()
            .map(|finding| finding.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["duplicate_owner_name", "duplicate_owner_name"],
        "both distinct boundary IDs are excluded from downstream analysis: {:#?}",
        duplicate_report.findings
    );
    assert!(duplicate_report.findings.iter().all(|finding| {
        finding.message.contains("field `name`:") && finding.kind == "duplicate_owner_name"
    }));
    assert!(
        duplicate_report.findings[0]
            .message
            .contains("boundary `BOUNDARY-PA`")
    );
    assert!(
        duplicate_report.findings[1]
            .message
            .contains("boundary `BOUNDARY-PB`")
    );
}

#[test]
fn issue377_aggregates_and_keeps_fatal_errors() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write_source("p", "lib.rs", "pub struct P;");
    write_policy_record(&fixture, "p/a.toml", "A", &[], None, "active");
    let record = fixture.root().join("boundaries/p/a.toml");
    let contents = fs::read_to_string(&record).expect("read record");
    fs::write(
        &record,
        contents.replace("facade = \"run\"", "facade = \"bad name\""),
    )
    .expect("write semantic record");
    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("semantic defect is not fatal");
    assert_eq!(report.findings[0].kind, "invalid_identifier");
    assert!(report.findings[0].message.contains("field `public`:"));
    assert_eq!(report.status, ReportStatus::Fail);

    fs::write(&record, "not valid = [toml").expect("write malformed record");
    let malformed = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("malformed TOML remains fatal")
    .to_string();
    assert!(
        malformed.contains("failed to parse TOML file"),
        "{malformed}"
    );

    let schema_fixture = WorkspaceFixture::new();
    schema_fixture.write_workspace_root_with_members(&["crates/p"]);
    schema_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    schema_fixture.write_source("p", "lib.rs", "pub struct P;");
    write_policy_record(&schema_fixture, "p/a.toml", "A", &[], None, "active");
    let schema_record = schema_fixture.root().join("boundaries/p/a.toml");
    let schema_contents = fs::read_to_string(&schema_record).expect("read schema record");
    fs::write(
        &schema_record,
        format!("{schema_contents}\nunknown_root_field = true\n"),
    )
    .expect("write unknown schema field");
    let schema_error = analyze_workspace(&AnalyzeOptions {
        root: schema_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("schema violation remains fatal")
    .to_string();
    assert!(
        schema_error.contains("failed to parse TOML file"),
        "{schema_error}"
    );

    let layout_fixture = WorkspaceFixture::new();
    layout_fixture.write_workspace_root_with_members(&["crates/p"]);
    layout_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    layout_fixture.write_source("p", "lib.rs", "pub struct P;");
    write_policy_record(&layout_fixture, "p/nested/a.toml", "A", &[], None, "active");
    let layout_record = layout_fixture.root().join("boundaries/p/nested/a.toml");
    let layout_contents = fs::read_to_string(&layout_record).expect("read layout record");
    fs::write(
        &layout_record,
        layout_contents.replace("owner_package = \"p\"", "owner_package = \"nested\""),
    )
    .expect("write nested-layout record");
    let layout_error = analyze_workspace(&AnalyzeOptions {
        root: layout_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("invalid inventory layout remains fatal")
    .to_string();
    assert!(
        layout_error.contains("must use boundaries/<owner-package>/<boundary>.toml layout"),
        "{layout_error}"
    );

    let duplicate_id_fixture = WorkspaceFixture::new();
    duplicate_id_fixture.write_workspace_root_with_members(&["crates/p"]);
    duplicate_id_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    duplicate_id_fixture.write_source("p", "lib.rs", "pub struct P;");
    write_policy_record(&duplicate_id_fixture, "p/a.toml", "A", &[], None, "active");
    write_policy_record(&duplicate_id_fixture, "p/b.toml", "B", &[], None, "active");
    let duplicate_a = duplicate_id_fixture.root().join("boundaries/p/a.toml");
    let duplicate_b = duplicate_id_fixture.root().join("boundaries/p/b.toml");
    let duplicate_a_contents = fs::read_to_string(&duplicate_a).expect("read first duplicate id");
    let duplicate_b_contents = fs::read_to_string(&duplicate_b).expect("read second duplicate id");
    fs::write(
        &duplicate_b,
        duplicate_b_contents.replace("BOUNDARY-PB", "BOUNDARY-PA"),
    )
    .expect("write duplicate id");
    let duplicate_id_error = analyze_workspace(&AnalyzeOptions {
        root: duplicate_id_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("duplicate boundary ids remain fatal")
    .to_string();
    assert!(
        duplicate_id_error.contains("duplicate boundary_id"),
        "{duplicate_id_error}"
    );
    assert!(duplicate_a_contents.contains("BOUNDARY-PA"));

    let io_fixture = WorkspaceFixture::new();
    io_fixture.write_workspace_root_with_members(&["crates/p"]);
    io_fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    io_fixture.write_source("p", "lib.rs", "pub struct P;");
    io_fixture.write("boundaries", "not a directory");
    let io_error = analyze_workspace(&AnalyzeOptions {
        root: io_fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("inventory I/O failure remains fatal")
    .to_string();
    assert!(
        io_error.contains("failed to read boundary directory"),
        "{io_error}"
    );
}

/// Issue #376 reporter's reproducer: package `p` owns two records; `q`
/// depends on `p` at runtime and `r` dev-depends on `p`. Record A lists `q`,
/// record B lists nothing and no list names `r`. The consumer saw findings on
/// record B for `q` and on both records for `r`, and never a stale entry.
#[test]
fn issue_376_reported_consumer_reproducer() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q", "crates/r", "crates/s"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write(
        "crates/q/Cargo.toml",
        &path_dependency_manifest("q", "p", "../p", "0.1.0"),
    );
    fixture.write(
        "crates/r/Cargo.toml",
        &(good_member_manifest("r", "")
            + "[dev-dependencies]\np = { path = \"../p\", version = \"0.1.0\" }\n"),
    );
    fixture.write("crates/s/Cargo.toml", &good_member_manifest("s", ""));
    for package in ["p", "q", "r", "s"] {
        fixture.write_source(package, "lib.rs", "pub struct Fixture;");
    }
    write_policy_record(&fixture, "p/a.toml", "A", &["q"], None, "active");
    write_policy_record(&fixture, "p/b.toml", "B", &[], None, "active");
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };
    let mentions =
        |finding: &Finding, package: &str| finding.message.contains(&format!("`{package}`"));

    // Expected: one dev-edge finding for `r`, none for record B's runtime `q`.
    let report = analyze_workspace(&options).expect("reproducer analyzes");
    assert_eq!(report.findings.len(), 1, "{:#?}", report.findings);
    assert!(mentions(&report.findings[0], "r"), "{:#?}", report.findings);
    assert!(!report.findings.iter().any(|finding| mentions(finding, "q")));

    // Listing `r` in `allowed_dev_dependents` clears the dev edge.
    write_policy_record(&fixture, "p/a.toml", "A", &["q"], Some(&["r"]), "active");
    let report = analyze_workspace(&options).expect("dev allowlist analyzes");
    assert_eq!(report.status, ReportStatus::Pass, "{:#?}", report.findings);

    // A listed dependent with no live edge to `p` is reported as stale.
    write_policy_record(
        &fixture,
        "p/a.toml",
        "A",
        &["q", "s"],
        Some(&["r"]),
        "active",
    );
    let report = analyze_workspace(&options).expect("stale entry analyzes");
    assert_eq!(report.findings.len(), 1, "{:#?}", report.findings);
    assert_eq!(report.findings[0].rule_id.as_str(), "SCB-DEPENDENCY-004");
    assert!(mentions(&report.findings[0], "s"), "{:#?}", report.findings);
}

/// Issue #377 reporter's reproducer: each defect is a one-line change to a
/// valid record. The consumer saw every mutated inventory pass; each must be
/// a validation finding instead.
#[test]
fn issue_377_reported_consumer_reproducer() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/p", "crates/q"]);
    fixture.write("crates/p/Cargo.toml", &good_member_manifest("p", ""));
    fixture.write("crates/q/Cargo.toml", &good_member_manifest("q", ""));
    fixture.write_source("p", "lib.rs", "pub struct P;");
    fixture.write_source("q", "lib.rs", "pub struct Q;");
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };
    let record = fixture.root().join("boundaries/p/a.toml");
    let reset = || {
        write_policy_record(&fixture, "p/a.toml", "A", &[], None, "active");
        write_policy_record(&fixture, "p/b.toml", "B", &[], None, "active");
    };
    reset();
    let report = analyze_workspace(&options).expect("valid records analyze");
    assert_eq!(report.status, ReportStatus::Pass, "{:#?}", report.findings);

    let valid = fs::read_to_string(&record).expect("read valid record");
    for (defect, from, to) in [
        ("identifier", "facade = \"run\"", "facade = \"not-valid\""),
        ("module path", "module = \"p\"", "module = \"p::\""),
        (
            "implementation type",
            "type = \"Runner\"",
            "type = \"not-valid\"",
        ),
        (
            "composition root path",
            "roots = [\"run\"]",
            "roots = [\"api::\"]",
        ),
        (
            "trait_only constructor",
            "type = \"Runner\"\nmodule = \"p\"\nvisibility = \"public\"\nconstructor = \"none\"",
            "visibility = \"trait_only\"\nconstructor = \"public\"",
        ),
        (
            "self edge",
            "forbidden_edges = []",
            "forbidden_edges = [\"p -> p\"]",
        ),
        ("duplicate name", "name = \"A\"", "name = \"B\""),
        (
            "composition root",
            "roots = [\"run\"]",
            "roots = [\"q::Root\"]",
        ),
    ] {
        reset();
        assert!(
            valid.contains(from),
            "{defect}: fixture line `{from}` not found"
        );
        fs::write(&record, valid.replace(from, to)).expect("write mutated record");
        let report = analyze_workspace(&options).expect("defect is a finding, not a load error");
        assert_eq!(
            report.status,
            ReportStatus::Fail,
            "{defect}: {:#?}",
            report.findings
        );
    }
}

#[test]
fn dependency_policy_resolves_owner_and_policy_aliases_to_canonical_packages() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app-dir", "crates/api-dir"]);
    fixture.write(
        "crates/app-dir/Cargo.toml",
        &identity_member_manifest(
            "app-pkg",
            "app_lib",
            "[dependencies]\napi_alias = { package = \"api-pkg\", path = \"../api-dir\", version = \"0.1.0\" }",
        ),
    );
    fixture.write(
        "crates/api-dir/Cargo.toml",
        &identity_member_manifest("api-pkg", "api_lib", ""),
    );
    fixture.write_boundary_record("app-dir", "app-dir", "app_lib", &["api_lib"], &[], &[]);
    fixture.write_boundary_record("api-dir", "api-dir", "api_lib", &[], &["app-dir"], &[]);
    fixture.write_source("app-dir", "lib.rs", "pub struct App;");
    fixture.write_source("api-dir", "lib.rs", "pub struct Api;");

    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };
    assert_eq!(
        analyze_workspace(&options).unwrap().status,
        ReportStatus::Pass
    );

    fixture.write_boundary_record(
        "app-dir",
        "app-dir",
        "app_lib",
        &["api_lib"],
        &[],
        &[("app-dir", "api_lib")],
    );
    let report = analyze_workspace(&options).unwrap();
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency003);
    assert_eq!(
        report.findings[0].message,
        "forbidden workspace dependency edge `app-pkg -> api-pkg` is present"
    );
}

#[test]
fn validated_inventory_exposes_canonical_package_policy_fields() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app-dir", "crates/api-dir"]);
    fixture.write(
        "crates/app-dir/Cargo.toml",
        &identity_member_manifest("app-pkg", "app_lib", ""),
    );
    fixture.write(
        "crates/api-dir/Cargo.toml",
        &identity_member_manifest("api-pkg", "api_lib", ""),
    );
    fixture.write_boundary_record("app-dir", "app-dir", "app_lib", &["api_lib"], &[], &[]);
    fixture.write_source("app-dir", "lib.rs", "pub struct App;");
    fixture.write_source("api-dir", "lib.rs", "pub struct Api;");

    let raw = inventory::load_boundary_inventory(fixture.root()).expect("load inventory");
    let metadata = graph::load_metadata(fixture.root()).expect("load workspace metadata");
    let workspace_packages = workspace_packages(&metadata);
    let resolver =
        inventory::WorkspacePackageResolver::from_workspace_packages(&workspace_packages);
    let known_non_workspace_packages =
        known_non_workspace_packages(&metadata, &raw, &workspace_packages);
    let (validated, diagnostics) =
        inventory::validate_inventory(raw, &resolver, &known_non_workspace_packages, true)
            .expect("validate inventory");

    assert!(diagnostics.is_empty());
    let policy = validated
        .records()
        .next()
        .and_then(inventory::ValidatedRecord::policy)
        .expect("active record has canonical policy");
    assert_eq!(
        policy.owner,
        inventory::WorkspacePackageName::from_package_name("app-pkg")
    );
    assert_eq!(
        policy.allowed_dependencies,
        BTreeSet::from([inventory::WorkspacePackageName::from_package_name(
            "api-pkg"
        )])
    );
}

#[test]
fn dependency_reference_exact_name_takes_precedence_over_directory_alias() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/exact-dir", "crates/other/shared"]);
    fixture.write(
        "crates/exact-dir/Cargo.toml",
        &identity_member_manifest(
            "exact-dir",
            "exact_lib",
            "[dependencies]\nshared = { path = \"../other/shared\", version = \"0.1.0\" }",
        ),
    );
    fixture.write(
        "crates/other/shared/Cargo.toml",
        &identity_member_manifest("shared", "other_lib", ""),
    );
    fixture.write_boundary_record("exact-dir", "exact-dir", "exact_lib", &["shared"], &[], &[]);
    fixture.write_source("exact-dir", "lib.rs", "pub struct Exact;");
    fixture.write("crates/other/shared/src/lib.rs", "pub struct Other;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
}

#[test]
fn dependency_policy_rejects_unknown_owner_and_edge_references() {
    let unknown_owner = WorkspaceFixture::new();
    unknown_owner.write_workspace_root_with_members(&["crates/app"]);
    unknown_owner.write(
        "crates/app/Cargo.toml",
        &identity_member_manifest("app", "app", ""),
    );
    unknown_owner.write_boundary_record("missing", "missing", "missing", &[], &[], &[]);
    unknown_owner.write_source("app", "lib.rs", "pub struct App;");
    let error = analyze_workspace(&AnalyzeOptions {
        root: unknown_owner.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("unknown owner must fail");
    assert!(
        error
            .to_string()
            .contains("unknown workspace package reference `missing`")
    );
    assert!(error.to_string().contains("owner_package"));

    let unknown_edge = WorkspaceFixture::new();
    unknown_edge.write_workspace_root_with_members(&["crates/app"]);
    unknown_edge.write(
        "crates/app/Cargo.toml",
        &identity_member_manifest("app", "app", ""),
    );
    unknown_edge.write_boundary_record("app", "app", "app", &[], &[], &[("app", "missing")]);
    unknown_edge.write_source("app", "lib.rs", "pub struct App;");
    let error = analyze_workspace(&AnalyzeOptions {
        root: unknown_edge.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect_err("unknown forbidden-edge endpoint must fail");
    assert!(
        error
            .to_string()
            .contains("unknown workspace package reference `missing`")
    );
    assert!(
        error
            .to_string()
            .contains("dependencies.forbidden_edges[].to")
    );
}

#[test]
fn dependency_policy_rejects_ambiguous_directory_and_library_aliases() {
    for (alias, members, manifests) in [
        (
            "shared",
            vec!["crates/owner", "crates/one/shared", "crates/two/shared"],
            vec![
                (
                    "crates/owner/Cargo.toml",
                    identity_member_manifest("owner", "owner", ""),
                ),
                (
                    "crates/one/shared/Cargo.toml",
                    identity_member_manifest("one-pkg", "one", ""),
                ),
                (
                    "crates/two/shared/Cargo.toml",
                    identity_member_manifest("two-pkg", "two", ""),
                ),
            ],
        ),
        (
            "shared_lib",
            vec!["crates/owner", "crates/one", "crates/two"],
            vec![
                (
                    "crates/owner/Cargo.toml",
                    identity_member_manifest("owner", "owner", ""),
                ),
                (
                    "crates/one/Cargo.toml",
                    identity_member_manifest("one-pkg", "shared_lib", ""),
                ),
                (
                    "crates/two/Cargo.toml",
                    identity_member_manifest("two-pkg", "shared_lib", ""),
                ),
            ],
        ),
    ] {
        let fixture = WorkspaceFixture::new();
        fixture.write_workspace_root_with_members(&members);
        for (path, manifest) in manifests {
            fixture.write(path, &manifest);
            fixture.write(
                &path.replace("Cargo.toml", "src/lib.rs"),
                "pub struct Fixture;",
            );
        }
        fixture.write_boundary_record("owner", "owner", "owner", &[alias], &[], &[]);

        let error = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: Some(RuleFilter::Dependencies),
        })
        .expect_err("ambiguous alias must fail");
        assert!(
            error
                .to_string()
                .contains("ambiguous workspace package reference"),
            "{error:#}"
        );
        assert!(
            error.to_string().contains("`one-pkg`, `two-pkg`"),
            "{error:#}"
        );
    }
}

#[test]
fn dependency_policy_flags_disallowed_direct_dependency() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency001);
    assert_eq!(
        report.findings[0].message,
        "workspace package `app` directly depends on `api` but `api` is not listed in `BOUNDARY-App` allowed_dependencies"
    );
}

#[test]
fn dependency_policy_checks_dev_edges_against_both_allow_lists() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &(good_member_manifest("app", "")
            + "[dev-dependencies]\napi = { path = \"../api\", version = \"0.1.0\" }\n"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };

    fixture.write_member_boundary_record("app", &[], &[], &[]);
    let report = analyze_workspace(&options).unwrap();
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        report.findings[0].message,
        "workspace package `app` directly dev-depends on `api` but `api` is not listed in `BOUNDARY-App` allowed_dependencies or allowed_dev_dependencies"
    );

    fixture.write_member_boundary_record("app", &["api"], &[], &[]);
    assert_eq!(
        analyze_workspace(&options).unwrap().status,
        ReportStatus::Pass
    );

    fixture.write_member_boundary_record("app", &[], &[], &[]);
    let boundary = fixture.root().join("boundaries/app/boundary.toml");
    let contents = fs::read_to_string(&boundary).unwrap().replace(
        "forbidden_edges = []",
        "allowed_dev_dependencies = [\"api\"]\nforbidden_edges = []",
    );
    fs::write(boundary, contents).unwrap();
    assert_eq!(
        analyze_workspace(&options).unwrap().status,
        ReportStatus::Pass
    );

    fixture.write(
        "crates/app/Cargo.toml",
        &(good_member_manifest("app", "")
            + "[dependencies]\napi = { path = \"../api\", version = \"0.1.0\" }\n\
               [dev-dependencies]\napi = { path = \"../api\", version = \"0.1.0\" }\n"),
    );
    let report = analyze_workspace(&options).unwrap();
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        report.findings[0].message,
        "workspace package `app` directly depends on `api` but `api` is not listed in `BOUNDARY-App` allowed_dependencies"
    );
}

#[test]
fn dependency_policy_flags_disallowed_direct_dependent() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &["api"], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &[], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency002);
    assert_eq!(
        report.findings[0].message,
        "workspace package `app` directly depends on `api` but `app` is not listed in `BOUNDARY-Api` allowed_dependents"
    );
}

#[test]
fn dependency_policy_flags_forbidden_direct_edge() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &["api"], &[], &[("app", "api")]);
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency003);
    assert_eq!(
        report.findings[0].message,
        "forbidden workspace dependency edge `app -> api` is present"
    );
}

#[test]
fn dependency_policy_enforces_build_dependency_workspace_edges() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &trim_indentation(
            r#"
                [package]
                name = "app"
                version.workspace = true
                edition.workspace = true
                rust-version.workspace = true
                authors.workspace = true
                license.workspace = true
                repository.workspace = true
                homepage.workspace = true

                [build-dependencies]
                api = { path = "../api", version = "0.1.0" }
            "#,
        ),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency001);
}

#[test]
fn dependency_policy_enforces_target_specific_workspace_edges() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &trim_indentation(
            r#"
                [package]
                name = "app"
                version.workspace = true
                edition.workspace = true
                rust-version.workspace = true
                authors.workspace = true
                license.workspace = true
                repository.workspace = true
                homepage.workspace = true

                [target.'cfg(unix)'.dependencies]
                api = { path = "../api", version = "0.1.0" }
            "#,
        ),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &["app"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency001);
}

#[test]
fn dependency_policy_ignores_transitive_only_workspace_edges() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/helper", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "helper", "../helper", "0.1.0"),
    );
    fixture.write(
        "crates/helper/Cargo.toml",
        &path_dependency_manifest("helper", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &["helper"], &[], &[]);
    fixture.write_member_boundary_record("helper", &["api"], &["app"], &[]);
    fixture.write_member_boundary_record("api", &[], &["helper"], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("helper", "lib.rs", "pub struct Helper;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn dependency_policy_ignores_third_party_dependencies() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &trim_indentation(
            r#"
                [package]
                name = "app"
                version.workspace = true
                edition.workspace = true
                rust-version.workspace = true
                authors.workspace = true
                license.workspace = true
                repository.workspace = true
                homepage.workspace = true

                [dependencies]
                serde = "1"
            "#,
        ),
    );
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn manifest_policy_accepts_workspace_inheritance() {
    let fixture = ManifestPolicyFixture::new(&["crates/example"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest("example", &good_member_manifest("example", ""));
    fixture.write_member_source("example", "pub struct Example;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Manifests),
    })
    .expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.scanned_crates, 1);
    assert!(report.findings.is_empty());
}

#[test]
fn manifest_policy_flags_missing_workspace_field() {
    let fixture = ManifestPolicyFixture::new(&["crates/example"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest(
        "example",
        &good_member_manifest("example", "").replace("homepage.workspace = true\n", ""),
    );
    fixture.write_member_source("example", "pub struct Example;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Manifests),
    })
    .expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        finding_messages(&report),
        vec!["crates/example/Cargo.toml: set [package].homepage.workspace = true".to_string()]
    );
}

#[test]
fn manifest_policy_flags_mismatched_internal_path_version() {
    let fixture = ManifestPolicyFixture::new(&["crates/example", "crates/helper"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest(
        "example",
        &path_dependency_manifest("example", "helper", "../helper", "9.9.9"),
    );
    fixture.write_member_manifest("helper", &good_member_manifest("helper", ""));
    fixture.write_member_source("example", "pub struct Example;");
    fixture.write_member_source("helper", "pub struct Helper;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Manifests),
    })
    .expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        finding_messages(&report),
        vec![
            "crates/example/Cargo.toml [dependencies.helper]: path dependency version must match target crate version \"1.1.2\""
                .to_string()
        ]
    );
}

#[test]
fn manifest_policy_accepts_explicit_tool_crate_version() {
    let fixture = ManifestPolicyFixture::new(&["crates/example", "crates/sc-lint-attributes"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest("example", &good_member_manifest("example", ""));
    fixture.write_member_manifest(
        "sc-lint-attributes",
        r#"
            [package]
            name = "sc-lint-attributes"
            version = "0.1.0"
            edition.workspace = true
            rust-version.workspace = true
            authors.workspace = true
            license.workspace = true
            repository.workspace = true
            homepage.workspace = true
        "#,
    );
    fixture.write_member_source("example", "pub struct Example;");
    fixture.write_member_source("sc-lint-attributes", "pub struct Attr;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Manifests),
    })
    .expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn manifest_policy_matches_python_oracle_on_representative_fixtures() {
    let valid = ManifestPolicyFixture::new(&["crates/example"]);
    valid.write_workspace_package_defaults("1.1.2");
    valid.write_member_manifest("example", &good_member_manifest("example", ""));
    valid.write_member_source("example", "pub struct Example;");

    let missing_field = ManifestPolicyFixture::new(&["crates/example"]);
    missing_field.write_workspace_package_defaults("1.1.2");
    missing_field.write_member_manifest(
        "example",
        &good_member_manifest("example", "").replace("homepage.workspace = true\n", ""),
    );
    missing_field.write_member_source("example", "pub struct Example;");

    let mismatched_version = ManifestPolicyFixture::new(&["crates/example", "crates/helper"]);
    mismatched_version.write_workspace_package_defaults("1.1.2");
    mismatched_version.write_member_manifest(
        "example",
        &path_dependency_manifest("example", "helper", "../helper", "9.9.9"),
    );
    mismatched_version.write_member_manifest("helper", &good_member_manifest("helper", ""));
    mismatched_version.write_member_source("example", "pub struct Example;");
    mismatched_version.write_member_source("helper", "pub struct Helper;");

    let explicit_tool =
        ManifestPolicyFixture::new(&["crates/example", "crates/sc-lint-attributes"]);
    explicit_tool.write_workspace_package_defaults("1.1.2");
    explicit_tool.write_member_manifest("example", &good_member_manifest("example", ""));
    explicit_tool.write_member_manifest(
        "sc-lint-attributes",
        r#"
            [package]
            name = "sc-lint-attributes"
            version = "0.1.0"
            edition.workspace = true
            rust-version.workspace = true
            authors.workspace = true
            license.workspace = true
            repository.workspace = true
            homepage.workspace = true
        "#,
    );
    explicit_tool.write_member_source("example", "pub struct Example;");
    explicit_tool.write_member_source("sc-lint-attributes", "pub struct Attr;");

    for fixture in [valid, missing_field, mismatched_version, explicit_tool] {
        let report = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: Some(RuleFilter::Manifests),
        })
        .expect("manifest analysis succeeds");
        if let Some(expected_messages) = python_manifest_findings(fixture.root()) {
            assert_eq!(finding_messages(&report), expected_messages);
        }
    }
}

fn cycle_rule_ids(source: &str) -> Vec<RuleId> {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", source);
    analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap()
    .findings
    .iter()
    .map(|finding| finding.rule_id)
    .collect()
}

#[test]
fn private_helper_delegation_is_not_a_type_method_self_loop() {
    assert!(
        cycle_rule_ids("pub struct T; impl T { pub fn a(&self) { self.b() } fn b(&self) {} }")
            .is_empty()
    );
    assert!(
        cycle_rule_ids("pub struct T; impl T { pub fn a(&self) { Self::b(self) } fn b(&self) {} }")
            .is_empty()
    );
}

#[test]
fn mutual_method_recursion_is_a_type_method_self_loop() {
    assert_eq!(
        cycle_rule_ids(
            "pub struct T; impl T { pub fn a(&self) { Self::b(self) } fn b(&self) { Self::a(self) } }"
        ),
        vec![RuleId::ScbCycle002]
    );
}

#[test]
fn trait_impl_receiver_call_is_not_a_trait_self_loop() {
    assert!(cycle_rule_ids(
        "pub struct T; pub trait Run { fn run(&self); } impl Run for T { fn run(&self) { Self::step(self) } } impl T { pub fn step(&self) {} }"
    )
    .is_empty());
}

#[test]
fn enum_variant_expressions_do_not_resolve_to_methods() {
    assert!(cycle_rule_ids(
        "pub enum E { A } impl E { pub fn get(&self) { let _ = Self::A; } #[allow(non_snake_case)] pub fn A() {} }"
    )
    .is_empty());
}

#[test]
fn reports_type_method_self_loop_as_non_fatal_signal() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    pub fn metric() -> usize {
                        let _ = Loop;
                        1
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle002);
    assert_eq!(report.findings[0].kind, "type_method_self_loop");
    assert_eq!(
        report.findings[0].owner_ids,
        vec!["crate::example::example::module::crate::Loop".to_string()]
    );
}

#[test]
fn does_not_flag_constructor_factory_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    pub fn build() -> Loop {
                        Loop
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_receiver_only_method_as_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Wrapper(String);

                impl Wrapper {
                    pub fn as_str(&self) -> &str {
                        &self.0
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_signature_only_self_return_as_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    pub fn placeholder() -> Self {
                        todo!()
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn suppresses_type_method_self_loop_when_allowed() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.allow("cycle.type_method_self_loop"))]
                pub struct Loop;

                impl Loop {
                    pub fn metric() -> usize {
                        let _ = Loop;
                        1
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn suppresses_type_method_self_loop_when_allowed_on_method() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    #[sc_lint(boundary.allow("cycle.type_method_self_loop"))]
                    pub fn metric() -> usize {
                        let _ = Loop;
                        1
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn keeps_unsuppressed_method_flagged_when_other_method_is_allowed() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    #[sc_lint(boundary.allow("cycle.type_method_self_loop"))]
                    pub fn allowed() -> usize {
                        let _ = Loop;
                        1
                    }

                    pub fn flagged() -> usize {
                        let _ = Loop;
                        2
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle002);
    assert!(
        report.findings[0]
            .node_ids
            .iter()
            .any(|id| id.ends_with("::flagged"))
    );
    assert!(
        !report.findings[0]
            .node_ids
            .iter()
            .any(|id| id.ends_with("::allowed"))
    );
}

#[test]
fn emits_both_inherent_and_trait_self_loop_findings_for_same_owner() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl Loop {
                    pub fn metric() -> usize {
                        let _ = Loop;
                        1
                    }
                }

                impl core::fmt::Display for Loop {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        let _ = Loop;
                        write!(f, "loop")
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.findings.len(), 2);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == RuleId::ScbCycle002)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == RuleId::ScbCycle003)
    );
}

#[test]
fn downgrades_trait_impl_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl core::fmt::Display for Loop {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        let _ = Loop;
                        write!(f, "loop")
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle003);
    assert_eq!(report.findings[0].kind, "trait_impl_self_loop");
}

#[test]
fn does_not_flag_conversion_like_trait_impl_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl core::str::FromStr for Loop {
                    type Err = ();

                    fn from_str(_s: &str) -> Result<Self, Self::Err> {
                        Ok(Loop)
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_comparison_like_trait_impl_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[derive(Debug, Clone)]
                pub struct Loop(String);

                impl core::cmp::PartialEq for Loop {
                    fn eq(&self, other: &Self) -> bool {
                        self.0 == other.0
                    }
                }

                impl core::cmp::Eq for Loop {}
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn normalizes_trait_paths_for_exact_policy_matching() {
    let path: syn::Path = syn::parse_quote!(serde::Deserialize<'de>);
    assert_eq!(graph::trait_path_key(&path), "serde::Deserialize");

    let path: syn::Path = syn::parse_quote!(core::cmp::PartialEq);
    assert_eq!(graph::trait_path_key(&path), "core::cmp::PartialEq");
}

#[test]
fn default_rule_config_ignores_common_non_architectural_trait_paths() {
    assert!(analysis::is_non_architectural_trait_impl_self_loop(
        "core::cmp::PartialEq"
    ));
    assert!(analysis::is_non_architectural_trait_impl_self_loop(
        "serde::Deserialize"
    ));
    assert!(analysis::is_non_architectural_trait_impl_self_loop(
        "FromStr"
    ));
    assert!(analysis::is_non_architectural_trait_impl_self_loop("Parse"));
    assert!(!analysis::is_non_architectural_trait_impl_self_loop(
        "one::Display"
    ));
}

#[test]
fn reports_multi_owner_architectural_cycle_as_failure() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Alpha {
                    pub beta: Beta,
                }

                pub struct Beta {
                    pub alpha: Alpha,
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle001);
    assert_eq!(report.findings[0].kind, "multi_owner_architectural_cycle");
    assert_eq!(
        sorted_owner_ids(&report.findings[0].owner_ids),
        sorted_owner_ids(&[
            "crate::example::example::module::crate::Alpha".to_string(),
            "crate::example::example::module::crate::Beta".to_string(),
        ])
    );
}

#[test]
fn suppresses_recursive_value_container_cycle_when_all_owners_allow_it() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.allow("cycle.recursive_value_container"))]
                pub enum Value {
                    Object(Map),
                }

                #[sc_lint(boundary.allow("cycle.recursive_value_container"))]
                pub struct Map {
                    entries: Vec<Value>,
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn keeps_recursive_value_container_cycle_when_only_one_owner_allows_it() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.allow("cycle.recursive_value_container"))]
                pub enum Value {
                    Object(Map),
                }

                pub struct Map {
                    entries: Vec<Value>,
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle001);
}

#[test]
fn reports_multi_owner_cycle_across_modules_as_failure() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                mod left;
                mod right;
            "#,
    );
    fixture.write_source(
        "example",
        "left.rs",
        "pub struct Alpha { pub beta: crate::right::Beta }",
    );
    fixture.write_source(
        "example",
        "right.rs",
        "pub struct Beta { pub alpha: crate::left::Alpha }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle001);
    assert_eq!(
        sorted_owner_ids(&report.findings[0].owner_ids),
        sorted_owner_ids(&[
            "crate::example::example::module::crate::left::Alpha".to_string(),
            "crate::example::example::module::crate::right::Beta".to_string(),
        ])
    );
}

#[test]
fn fails_when_internal_only_item_is_public() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.internal_only)]
                pub struct Secret;
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::InternalOnly),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbBoundary001);
}

#[test]
fn fails_when_internal_only_item_is_referenced_from_other_module() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "mod owner; mod user;");
    fixture.write_source(
        "example",
        "owner.rs",
        r#"
                #[sc_lint(boundary.internal_only)]
                struct Secret;
            "#,
    );
    fixture.write_source(
        "example",
        "user.rs",
        r#"
                pub struct Uses(crate::owner::Secret);
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::InternalOnly),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbBoundary002);
    assert!(report.findings[0].message.contains("crate::owner::Secret"));
}

#[test]
fn allows_internal_only_item_inside_own_module() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.internal_only)]
                struct Secret;

                struct Wrapper(Secret);
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::InternalOnly),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn fails_when_forbid_external_impls_trait_is_implemented_elsewhere() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "mod api; mod impls;");
    fixture.write_source(
        "example",
        "api.rs",
        r#"
                #[sc_lint(boundary.forbid_external_impls)]
                pub trait Tokenize {
                    fn tokenize(&self) -> usize;
                }

                pub struct Thing;
            "#,
    );
    fixture.write_source(
        "example",
        "impls.rs",
        r#"
                impl crate::api::Tokenize for crate::api::Thing {
                    fn tokenize(&self) -> usize {
                        1
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == RuleId::ScbBoundary003)
    );
}

#[test]
fn allows_forbid_external_impls_trait_impl_in_same_module() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.forbid_external_impls)]
                pub trait Tokenize {
                    fn tokenize(&self) -> usize;
                }

                pub struct Thing;

                impl Tokenize for Thing {
                    fn tokenize(&self) -> usize {
                        1
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn allows_approved_named_external_caller() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/api", "crates/app"]);
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write_source("api", "lib.rs", "pub mod restricted { pub fn run() {} }");
    fixture.write_source(
        "app",
        "lib.rs",
        r#"
            pub mod allowed {
                pub struct Facade;

                impl Facade {
                    pub fn call(&self) {
                        api::restricted::run();
                    }
                }
            }
        "#,
    );
    write_named_caller_boundary_record(
        &fixture,
        &[r#"{ symbol = "restricted::run", callers = ["app::allowed::Facade"] }"#],
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn fails_when_external_named_caller_is_not_approved() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/api", "crates/app", "crates/other"]);
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write(
        "crates/other/Cargo.toml",
        &path_dependency_manifest("other", "api", "../api", "0.1.0"),
    );
    fixture.write_source("api", "lib.rs", "pub mod restricted { pub fn run() {} }");
    fixture.write_source(
        "app",
        "lib.rs",
        r#"
            pub mod allowed {
                pub struct Facade;

                impl Facade {
                    pub fn call(&self) {
                        api::restricted::run();
                    }
                }
            }
        "#,
    );
    fixture.write_source(
        "other",
        "lib.rs",
        r#"
            pub mod blocked {
                pub struct Facade;

                impl Facade {
                    pub fn call(&self) {
                        api::restricted::run();
                    }
                }
            }
        "#,
    );
    write_named_caller_boundary_record(
        &fixture,
        &[r#"{ symbol = "restricted::run", callers = ["app::allowed::Facade"] }"#],
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCaller001);
    assert!(
        report.findings[0]
            .message
            .contains("other::blocked::Facade")
    );

    let rendered_text = render_findings_report(&report);
    assert!(rendered_text.contains("SCB-CALLER-001"));

    let rendered_json = serde_json::to_string(&report).expect("json render");
    assert!(rendered_json.contains("\"SCB-CALLER-001\""));
}

#[test]
fn exempts_owner_crate_callers_for_outside_owner_crate_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/api"]);
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_source(
        "api",
        "lib.rs",
        r#"
            pub mod restricted {
                pub fn run() {}
            }

            pub mod owner {
                pub struct Facade;

                impl Facade {
                    pub fn call(&self) {
                        crate::restricted::run();
                    }
                }
            }
        "#,
    );
    write_named_caller_boundary_record(
        &fixture,
        &[r#"{ symbol = "restricted::run", callers = ["external::allowed::Facade"] }"#],
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn supports_multi_symbol_named_caller_configuration() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/api", "crates/app", "crates/admin"]);
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write(
        "crates/admin/Cargo.toml",
        &path_dependency_manifest("admin", "api", "../api", "0.1.0"),
    );
    fixture.write_source(
        "api",
        "lib.rs",
        r#"
            pub mod restricted {
                pub fn run() {}
                pub fn repair() {}
            }
        "#,
    );
    fixture.write_source(
        "app",
        "lib.rs",
        r#"
            pub mod allowed {
                pub struct RunFacade;

                impl RunFacade {
                    pub fn call(&self) {
                        api::restricted::run();
                    }
                }
            }
        "#,
    );
    fixture.write_source(
        "admin",
        "lib.rs",
        r#"
            pub mod repair {
                pub struct Facade;

                impl Facade {
                    pub fn call(&self) {
                        api::restricted::repair();
                    }
                }
            }
        "#,
    );
    write_named_caller_boundary_record(
        &fixture,
        &[
            r#"{ symbol = "restricted::run", callers = ["app::allowed::RunFacade"] }"#,
            r#"{ symbol = "restricted::repair", callers = ["admin::repair::Facade"] }"#,
        ],
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Boundaries),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_acyclic_chain() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Alpha { pub beta: Beta }
                pub struct Beta { pub value: usize }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_cross_module_acyclic_chain() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                mod left;
                mod right;
            "#,
    );
    fixture.write_source(
        "example",
        "left.rs",
        "pub struct Alpha { pub beta: crate::right::Beta }",
    );
    fixture.write_source(
        "example",
        "right.rs",
        "pub struct Beta { pub value: usize }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn does_not_flag_newtype_factory_self_loop() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Wrapper(String);

                impl Wrapper {
                    pub fn into_inner(self) -> String {
                        self.0
                    }

                    pub fn from_inner(inner: String) -> Wrapper {
                        Wrapper(inner)
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn resolves_self_prefixed_references() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Alpha;
                pub struct Beta(self::Alpha);
            "#,
    );

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references"
            && edge.from == "crate::example::example::module::crate::Beta"
            && edge.to == "crate::example::example::module::crate::Alpha"
    }));
}

#[test]
fn resolves_super_prefixed_references() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Alpha;
                mod inner;
            "#,
    );
    fixture.write_source("example", "inner.rs", "pub struct Beta(super::Alpha);");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();

    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references"
            && edge.from == "crate::example::example::module::crate::inner::Beta"
            && edge.to == "crate::example::example::module::crate::Alpha"
    }));
}

#[test]
fn does_not_promote_function_owned_references_into_module_cycles() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                mod left;
                mod right;
            "#,
    );
    fixture.write_source(
        "example",
        "left.rs",
        "pub fn use_right() -> crate::right::Beta { todo!() }\npub struct Alpha;",
    );
    fixture.write_source(
        "example",
        "right.rs",
        "pub fn use_left() -> crate::left::Alpha { todo!() }\npub struct Beta;",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn preserves_full_trait_path_in_trait_impl_self_loop_messages() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                mod one { pub trait Display { fn render(&self); } }
                pub struct Loop;

                impl one::Display for Loop {
                    fn render(&self) {
                        let _ = Loop;
                    }
                }
            "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle003);
    assert!(report.findings[0].message.contains("one::Display"));
}

#[test]
fn rejects_impl_owners_without_a_named_target() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                pub struct Loop;

                impl core::fmt::Display for (Loop, Loop) {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        write!(f, "loop")
                    }
                }
            "#,
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("unsupported impl owner type"));
    assert!(message.contains("(Loop , Loop)"));
}

#[test]
fn fails_when_external_module_is_missing() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "mod missing;");

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("while resolving module `crate::missing`"));
}

#[test]
fn fails_when_module_resolution_is_ambiguous() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "mod dup;");
    fixture.write_source("example", "dup.rs", "pub struct A;");
    fixture.write_source("example", "dup/mod.rs", "pub struct B;");

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("while resolving module `crate::dup`"));
}

#[test]
fn fails_when_sc_lint_attribute_is_invalid() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
                #[sc_lint(boundary.allow(""))]
                pub struct Example;
            "#,
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("boundary.allow rule ids must not be empty"));
}

#[test]
fn fails_when_sc_lint_attribute_has_no_allow_args() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[sc_lint(boundary.allow())] pub struct Example;",
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("boundary.allow requires at least one rule id string"));
}

#[test]
fn fails_when_sc_lint_attribute_uses_unknown_boundary_directive() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[sc_lint(boundary.unknown(\"x\"))] pub struct Example;",
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("unsupported boundary directive"));
}

#[test]
fn fails_when_sc_lint_attribute_uses_unknown_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[sc_lint(other.internal_only)] pub struct Example;",
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("unsupported sc_lint scope `other`"));
}

#[test]
fn fails_when_sc_lint_attribute_has_mixed_valid_and_invalid_directives() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[sc_lint(boundary.internal_only, boundary.unknown(\"x\"))] pub struct Example;",
    );

    let error = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("unsupported boundary directive"));
}

fn lib_name_fixture(owner_crate_path: &str) -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &(good_member_manifest("app", "") + "[lib]\nname = \"app_core\"\npath = \"src/lib.rs\"\n"),
    );
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    let boundary_path = fixture.root().join("boundaries/app/boundary.toml");
    let boundary = fs::read_to_string(&boundary_path).unwrap().replace(
        "owner_crate_path = \"app\"",
        &format!("owner_crate_path = \"{owner_crate_path}\""),
    );
    fs::write(boundary_path, boundary).unwrap();
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture
}

#[test]
fn owner_crate_path_uses_cargo_lib_target_name() {
    let options = |fixture: &WorkspaceFixture| AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    };
    let lib_named = lib_name_fixture("app_core");
    assert_eq!(
        analyze_workspace(&options(&lib_named)).unwrap().status,
        ReportStatus::Pass
    );

    let package_named = lib_name_fixture("app");
    let error = analyze_workspace(&options(&package_named)).unwrap_err();
    assert!(matches!(error, BoundaryError::InventoryLoad { .. }));
    assert!(
        error
            .to_string()
            .contains("expected `app_core` from the lib target")
    );
}

#[test]
fn bin_only_owner_package_skips_lib_target_validation() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &(good_member_manifest("app", "")
            + "[[bin]]\nname = \"app_bin\"\npath = \"src/main.rs\"\n"),
    );
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_source("app", "main.rs", "fn main() {}");
    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Dependencies),
    })
    .unwrap();
    assert_eq!(report.status, ReportStatus::Pass);
}

#[test]
fn graph_ignores_proc_macro_only_workspace_dependencies() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/macros"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "macros", "../macros", "0.1.0"),
    );
    fixture.write(
        "crates/macros/Cargo.toml",
        &(good_member_manifest("macros", "") + "[lib]\nproc-macro = true\n"),
    );
    fixture.write_source("app", "lib.rs", "pub fn uses(_: macros::Marker) {}");
    fixture.write_source("macros", "lib.rs", "extern crate proc_macro;");

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .expect("graph export succeeds");

    assert!(graph.edges.iter().all(|edge| {
        graph.nodes.iter().any(|node| node.id == edge.from)
            && graph.nodes.iter().any(|node| node.id == edge.to)
    }));
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

                    [workspace.package]
                    version = "0.1.0"
                    edition = "2024"
                    rust-version = "1.94.1"
                    authors = ["sc-lint contributors"]
                    license = "MIT OR Apache-2.0"
                    repository = "https://example.invalid/sc-lint"
                    homepage = "https://example.invalid/sc-lint"
                "#,
        );
    }

    fn write_workspace_root_with_members(&self, members: &[&str]) {
        let members = members
            .iter()
            .map(|member| format!("\"{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        self.write(
            "Cargo.toml",
            &format!(
                r#"
                    [workspace]
                    members = [{members}]
                    resolver = "2"

                    [workspace.package]
                    version = "0.1.0"
                    edition = "2024"
                    rust-version = "1.94.1"
                    authors = ["sc-lint contributors"]
                    license = "MIT OR Apache-2.0"
                    repository = "https://example.invalid/sc-lint"
                    homepage = "https://example.invalid/sc-lint"
                "#
            ),
        );
    }

    fn write_package_manifest(&self, package_name: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                r#"
                        [package]
                        name = "{package_name}"
                        version.workspace = true
                        edition.workspace = true
                        rust-version.workspace = true
                        authors.workspace = true
                        license.workspace = true
                        repository.workspace = true
                        homepage.workspace = true
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

    fn write_member_boundary_record(
        &self,
        owner_package: &str,
        allowed_dependencies: &[&str],
        allowed_dependents: &[&str],
        forbidden_edges: &[(&str, &str)],
    ) {
        let boundary_id = owner_package
            .split('-')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();
        let allowed_dependencies = allowed_dependencies
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let allowed_dependents = allowed_dependents
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges = forbidden_edges
            .iter()
            .map(|(from, to)| format!("{{ from = \"{from}\", to = \"{to}\" }}"))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges_block = if forbidden_edges.is_empty() {
            "[]".to_string()
        } else {
            format!("[{forbidden_edges}]")
        };

        self.write(
            "boundaries/planning.toml",
            r#"
                [planning]
                current_sprint = "D.1"
            "#,
        );
        self.write(
            &format!("boundaries/{owner_package}/boundary.toml"),
            &format!(
                r#"
                    boundary_id = "BOUNDARY-{boundary_id}"
                    owner_package = "{owner_package}"
                    owner_crate_path = "{}"
                    name = "{owner_package}"

                    [public]
                    facade = "run"

                    [implementation]
                    type = "run"
                    module = "{}"
                    visibility = "public"
                    constructor = "none"

                    [composition]
                    roots = ["run"]

                    [dependencies]
                    allowed_dependents = [{allowed_dependents}]
                    allowed_dependencies = [{allowed_dependencies}]
                    forbidden_edges = {forbidden_edges_block}

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
                owner_package.replace('-', "_"),
                owner_package.replace('-', "_"),
            ),
        );
    }

    fn write_boundary_record(
        &self,
        boundary_directory: &str,
        owner_package: &str,
        owner_crate_path: &str,
        allowed_dependencies: &[&str],
        allowed_dependents: &[&str],
        forbidden_edges: &[(&str, &str)],
    ) {
        let boundary_id = boundary_directory
            .split('-')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();
        let allowed_dependencies = allowed_dependencies
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let allowed_dependents = allowed_dependents
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges = forbidden_edges
            .iter()
            .map(|(from, to)| format!("{{ from = \"{from}\", to = \"{to}\" }}"))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges = if forbidden_edges.is_empty() {
            "[]".to_string()
        } else {
            format!("[{forbidden_edges}]")
        };

        self.write(
            "boundaries/planning.toml",
            r#"
                [planning]
                current_sprint = "D.1"
            "#,
        );
        self.write(
            &format!("boundaries/{boundary_directory}/boundary.toml"),
            &format!(
                r#"
                    boundary_id = "BOUNDARY-{boundary_id}"
                    owner_package = "{owner_package}"
                    owner_crate_path = "{owner_crate_path}"
                    name = "{boundary_directory}"

                    [public]
                    facade = "run"

                    [implementation]
                    type = "run"
                    module = "{owner_crate_path}"
                    visibility = "public"
                    constructor = "none"

                    [composition]
                    roots = ["run"]

                    [dependencies]
                    allowed_dependents = [{allowed_dependents}]
                    allowed_dependencies = [{allowed_dependencies}]
                    forbidden_edges = {forbidden_edges}

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

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, trim_indentation(contents)).unwrap();
    }
}

fn write_named_caller_boundary_record(fixture: &WorkspaceFixture, approved_entries: &[&str]) {
    let approved_entries = approved_entries
        .iter()
        .map(|entry| format!("                {entry},"))
        .collect::<Vec<_>>()
        .join("\n");
    fixture.write(
        "boundaries/planning.toml",
        r#"
            [planning]
            current_sprint = "B.2"
        "#,
    );
    fixture.write(
        "boundaries/api/restricted-api.toml",
        &format!(
            r#"
                boundary_id = "BOUNDARY-ApiRestricted"
                owner_package = "api"
                owner_crate_path = "api"
                name = "ApiRestricted"

                [public]
                facade = "restricted::run"

                [implementation]
                type = "run"
                module = "api::restricted"
                visibility = "public"
                constructor = "none"

                [composition]
                roots = ["run", "repair"]

                [callers]
                approved = [
{approved_entries}
                ]

                [dependencies]
                allowed_dependents = ["app", "admin", "other"]
                allowed_dependencies = []
                forbidden_edges = []

                [references]
                scope = "outside_owner_crate"
                forbidden = []

                [testing]
                allowed_test_double_paths = []
                forbidden_test_bypasses = []

                [enforcement]
                lint_rules = ["SCB-CALLER-001"]
                review_gates = ["approved_named_callers"]

                [status]
                state = "concrete_landed"
            "#
        ),
    );
}

fn write_policy_record(
    fixture: &WorkspaceFixture,
    path: &str,
    name: &str,
    allowed_dependents: &[&str],
    allowed_dev_dependents: Option<&[&str]>,
    state: &str,
) {
    let allowed_dependents = allowed_dependents
        .iter()
        .map(|package| format!("\"{package}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let allowed_dev_dependents = allowed_dev_dependents.map(|packages| {
        let packages = packages
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("                allowed_dev_dependents = [{packages}]\n                ")
    });
    fixture.write(
        "boundaries/planning.toml",
        "[planning]\ncurrent_sprint = \"K.3\"\n",
    );
    fixture.write(
        &format!("boundaries/{path}"),
        &format!(
            r#"
                boundary_id = "BOUNDARY-P{name}"
                owner_package = "p"
                owner_crate_path = "p"
                name = "{name}"

                [public]
                facade = "run"

                [implementation]
                type = "Runner"
                module = "p"
                visibility = "public"
                constructor = "none"

                [composition]
                roots = ["run"]

                [dependencies]
                allowed_dependents = [{allowed_dependents}]
                {allowed_dev_dependents}allowed_dependencies = []
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
            allowed_dev_dependents = allowed_dev_dependents.unwrap_or_default(),
        ),
    );
}

struct ManifestPolicyFixture {
    workspace: WorkspaceFixture,
    members: Vec<String>,
}

impl ManifestPolicyFixture {
    fn new(members: &[&str]) -> Self {
        let workspace = WorkspaceFixture::new();
        workspace.write_workspace_root_with_members(members);
        Self {
            workspace,
            members: members.iter().map(|member| (*member).to_string()).collect(),
        }
    }

    fn root(&self) -> &Path {
        self.workspace.root()
    }

    fn write_workspace_package_defaults(&self, version: &str) {
        self.workspace.write(
            "Cargo.toml",
            &format!(
                r#"
                    [workspace]
                    members = [{}]
                    resolver = "2"

                    [workspace.package]
                    version = "{version}"
                    edition = "2024"
                    rust-version = "1.94.1"
                    authors = ["sc-lint contributors"]
                    license = "MIT OR Apache-2.0"
                    repository = "https://example.invalid/sc-lint"
                    homepage = "https://example.invalid/sc-lint"
                "#,
                self.members
                    .iter()
                    .map(|member| format!("\"{member}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    fn write_member_manifest(&self, package_name: &str, contents: &str) {
        self.workspace
            .write(&format!("crates/{package_name}/Cargo.toml"), contents);
    }

    fn write_member_source(&self, package_name: &str, contents: &str) {
        self.workspace
            .write_source(package_name, "lib.rs", contents);
    }
}

fn finding_messages(report: &FindingsReport) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|finding| finding.message.clone())
        .collect()
}

fn sorted_owner_ids<T: ToString>(owner_ids: &[T]) -> Vec<String> {
    let mut owner_ids = owner_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    owner_ids.sort();
    owner_ids
}

fn python_manifest_findings(repo_root: &Path) -> Option<Vec<String>> {
    std::env::var_os("SC_LINT_ENABLE_PYTHON_PARITY")?;

    let repo_root_path = repo_root_from_manifest_dir();
    let script = r#"
import json
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
fixture_root = Path(sys.argv[2])
sys.path.insert(0, str(repo_root / ".just"))
from lint_manifests import collect_manifest_violations

print(json.dumps([violation.render() for violation in collect_manifest_violations(fixture_root)]))
"#;
    let output = match Command::new("python3").arg("--version").output() {
        Ok(_) => Command::new("python3")
            .arg("-c")
            .arg(script)
            .arg(&repo_root_path)
            .arg(repo_root)
            .output()
            .expect("python parity command runs"),
        Err(_) => return None,
    };
    assert!(
        output.status.success(),
        "python parity command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(serde_json::from_slice::<Vec<String>>(&output.stdout).expect("python parity json"))
}

fn repo_root_from_manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn good_member_manifest(package_name: &str, extra: &str) -> String {
    trim_indentation(&format!(
        r#"
            [package]
            name = "{package_name}"
            version.workspace = true
            edition.workspace = true
            rust-version.workspace = true
            authors.workspace = true
            license.workspace = true
            repository.workspace = true
            homepage.workspace = true

            {extra}
        "#
    ))
}

fn identity_member_manifest(package_name: &str, library_name: &str, extra: &str) -> String {
    format!(
        "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nname = \"{library_name}\"\n\n{extra}\n"
    )
}

fn path_dependency_manifest(
    package_name: &str,
    dependency_name: &str,
    dependency_path: &str,
    version: &str,
) -> String {
    trim_indentation(&format!(
        r#"
            [package]
            name = "{package_name}"
            version.workspace = true
            edition.workspace = true
            rust-version.workspace = true
            authors.workspace = true
            license.workspace = true
            repository.workspace = true
            homepage.workspace = true

            [dependencies]
            {dependency_name} = {{ path = "{dependency_path}", version = "{version}" }}
        "#
    ))
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

#[test]
fn supports_generic_field_value_reference_impl_owner() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        mod serde { pub trait Serialize {} }
        pub struct FieldValue<'a, T: ?Sized>(&'a T);
        pub trait SerializeKindTag {}
        impl<T: ?Sized + serde::Serialize> SerializeKindTag for &FieldValue<'_, T> {}
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let owner = graph
        .nodes
        .iter()
        .find(|node| node.label == "FieldValue")
        .unwrap();
    let implementation = graph
        .nodes
        .iter()
        .find(|node| node.impl_trait.as_deref() == Some("SerializeKindTag"))
        .unwrap();
    assert!(implementation.label.contains("&FieldValue"));
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::Targets.as_str()
                && edge.from == implementation.id
                && edge.to == owner.id)
    );
}

#[test]
fn reference_impls_and_methods_do_not_collide_with_owned_impls() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Owner;
        pub trait Action { fn act(&self); }
        impl Action for Owner { fn act(&self) {} }
        impl Action for &Owner { fn act(&self) {} }
        impl Action for &mut Owner { fn act(&self) {} }
        impl Action for &&Owner { fn act(&self) {} }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let owner = graph
        .nodes
        .iter()
        .find(|node| node.label == "Owner")
        .unwrap();
    let implementations: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "impl")
        .collect();
    let methods: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "method" && node.impl_trait.as_deref() == Some("Action"))
        .collect();
    assert_eq!(implementations.len(), 4);
    assert_eq!(methods.len(), 4);
    let owned = implementations
        .iter()
        .find(|node| node.label == "impl Action for Owner")
        .unwrap();
    assert_eq!(
        owned.id.as_str(),
        format!("{}::impl::416374696f6e", owner.id)
    );
    assert!(
        methods
            .iter()
            .any(|node| node.id.as_str() == format!("{}::impl::416374696f6e::act", owner.id))
    );
    for implementation in implementations {
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == EdgeKind::Targets.as_str()
                    && edge.from == implementation.id
                    && edge.to == owner.id)
        );
        let method_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| {
                edge.kind == EdgeKind::Contains.as_str()
                    && edge.from == implementation.id
                    && methods.iter().any(|method| method.id == edge.to)
            })
            .collect();
        assert_eq!(method_edges.len(), 1);
    }
}

#[test]
fn reference_method_cycles_keep_the_underlying_type_owner() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Owner;
        pub trait Action { fn act(&self); }
        impl Action for &Owner { fn act(&self) { let _ = Owner; } }
    "#,
    );
    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle003);
    assert!(
        report.findings[0]
            .owner_ids
            .iter()
            .all(|owner| owner.ends_with("::Owner"))
    );
}

#[test]
fn trait_adapters_preserve_distinct_methods_and_forwarding_edges() {
    for reverse_order in [false, true] {
        let fixture = WorkspaceFixture::new();
        fixture.write_workspace_root();
        fixture.write_package_manifest("example");
        let definitions = "pub struct Adapter; pub trait Typed { fn write(&self); } pub trait Legacy { fn write(&self); }";
        let inherent = "impl Adapter { pub fn write(&self) {} }";
        let traits = "impl Typed for Adapter { fn write(&self) { Adapter::write(self); } } impl Legacy for Adapter { fn write(&self) { <Self as Typed>::write(self); } }";
        let source = if reverse_order {
            format!("{definitions} {traits} {inherent}")
        } else {
            format!("{definitions} {inherent} {traits}")
        };
        fixture.write_source("example", "lib.rs", &source);
        let graph = export_workspace_graph(&ExportGraphOptions {
            root: fixture.root().to_path_buf(),
        })
        .unwrap();
        let owner = graph
            .nodes
            .iter()
            .find(|node| node.label == "Adapter")
            .unwrap();
        let methods: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| {
                node.kind == "method"
                    && node.label == "write"
                    && node.id.starts_with(owner.id.as_str())
            })
            .collect();
        assert_eq!(methods.len(), 3);
        let inherent = methods
            .iter()
            .find(|node| node.impl_kind == Some(ImplKind::Inherent))
            .unwrap();
        let typed = methods
            .iter()
            .find(|node| node.impl_trait.as_deref() == Some("Typed"))
            .unwrap();
        let legacy = methods
            .iter()
            .find(|node| node.impl_trait.as_deref() == Some("Legacy"))
            .unwrap();
        assert_eq!(inherent.id.as_str(), format!("{}::write", owner.id));
        for (from, to) in [
            (typed.id.clone(), inherent.id.clone()),
            (legacy.id.clone(), typed.id.clone()),
        ] {
            assert_ne!(from, to);
            assert!(
                graph.edges.iter().any(|edge| edge.kind == "references_expr"
                    && edge.from == from
                    && edge.to == to)
            );
        }
        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| edge.kind == "references_expr" && edge.from == edge.to)
        );
        let report = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
            rule: Some(RuleFilter::Cycles),
        })
        .unwrap();
        assert!(
            !report.findings.is_empty(),
            "identity correction must not suppress owner-level cycle policy"
        );
        assert!(report.findings.iter().all(|finding| {
            finding
                .owner_ids
                .iter()
                .all(|id| id.as_str() == owner.id.as_str())
        }));
    }
}

#[test]
fn schema_v02_fixture_captures_trait_method_id_migration_in_json_and_turtle() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub struct Adapter; pub trait Typed { fn write(&self); } pub trait Legacy { fn write(&self); } impl Adapter { pub fn write(&self) {} } impl Typed for Adapter { fn write(&self) {} } impl Legacy for Adapter { fn write(&self) {} }",
    );

    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let owner = graph
        .nodes
        .iter()
        .find(|node| node.label == "Adapter" && node.kind == "type")
        .unwrap();
    let owner_id = owner.id.clone();
    let inherent_id = NodeId::new(format!("{owner_id}::write"));
    let typed_id = NodeId::new(format!("{owner_id}::impl::{}::write", hex_encode(b"Typed")));
    let legacy_id = NodeId::new(format!(
        "{owner_id}::impl::{}::write",
        hex_encode(b"Legacy")
    ));

    assert_eq!(graph.schema_version, "0.2.0");
    assert!(graph.nodes.iter().any(|node| node.id == owner_id));
    assert!(graph.nodes.iter().any(|node| node.id == inherent_id));
    assert!(graph.nodes.iter().any(|node| node.id == typed_id));
    assert!(graph.nodes.iter().any(|node| node.id == legacy_id));
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.kind == "method" && node.label == "write")
            .count(),
        3
    );

    let graph_json: serde_json::Value =
        serde_json::from_str(&render_graph_export_json(&graph)).unwrap();
    assert_eq!(graph_json["schema_version"], "0.2.0");
    assert!(graph_json.to_string().contains(typed_id.as_str()));
    assert!(graph_json.to_string().contains(legacy_id.as_str()));

    let turtle = render_graph_export_turtle(&graph);
    assert!(turtle.contains("sc:schemaVersion \"0.2.0\" ."));
    assert!(turtle.contains(&typed_id.to_string()));
    assert!(turtle.contains(&legacy_id.to_string()));

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .unwrap();
    let findings_json = serde_json::to_value(&report).unwrap();
    assert_eq!(findings_json["schema_version"], "0.2.0");
}

#[test]
fn generic_trait_and_self_arguments_distinguish_impl_methods() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Adapter<T>(T);
        pub trait Convert<T> { fn convert(&self); }
        impl Convert<u8> for Adapter<u8> { fn convert(&self) {} }
        impl Convert<u16> for Adapter<u8> { fn convert(&self) {} }
        impl Convert<u8> for Adapter<u16> { fn convert(&self) {} }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let implementations: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "impl" && node.impl_trait.as_deref() == Some("Convert"))
        .collect();
    let methods: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "method" && node.impl_trait.as_deref() == Some("Convert"))
        .collect();
    assert_eq!(implementations.len(), 3);
    assert_eq!(methods.len(), 3);
    for implementation in implementations {
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.kind == "contains"
                    && edge.from == implementation.id
                    && methods.iter().any(|method| method.id == edge.to))
                .count(),
            1
        );
    }
}

#[test]
fn trait_self_calls_resolve_without_inventing_inherent_methods() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Adapter;
        pub trait Action { fn first(); fn second(); }
        impl Action for Adapter { fn first() { Self::second(); <Self as Action>::second(); } fn second() {} }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let first = graph
        .nodes
        .iter()
        .find(|node| node.label == "first" && node.impl_trait.as_deref() == Some("Action"))
        .unwrap();
    let second = graph
        .nodes
        .iter()
        .find(|node| node.label == "second" && node.impl_trait.as_deref() == Some("Action"))
        .unwrap();
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "references_expr"
                && edge.from == first.id
                && edge.to == second.id)
            .count(),
        1
    );
}

#[test]
fn qualified_generic_reference_trait_calls_resolve_to_the_matching_impl_method() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Adapter;
        pub trait Convert<T> { fn first(); fn second(); }
        impl Convert<u8> for &Adapter {
            fn first() { <&Adapter as Convert<u8>>::second(); }
            fn second() {}
        }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let first = graph
        .nodes
        .iter()
        .find(|node| node.label == "first" && node.impl_trait.as_deref() == Some("Convert"))
        .unwrap();
    let second = graph
        .nodes
        .iter()
        .find(|node| node.label == "second" && node.impl_trait.as_deref() == Some("Convert"))
        .unwrap();
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_expr" && edge.from == first.id && edge.to == second.id
    }));
}

#[test]
fn generic_reference_impls_keep_separate_nodes_and_do_not_capture_owned_calls() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Foo<T>(T);
        pub trait Convert<T> { fn convert(); }
        impl Convert<u8> for &Foo<u8> { fn convert() {} }
        impl Convert<u16> for &Foo<u8> { fn convert() {} }
        impl Convert<u8> for &Foo<u16> { fn convert() {} }
        pub fn unresolved() { Foo::convert(); }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let methods: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "method" && node.impl_trait.as_deref() == Some("Convert"))
        .collect();
    assert_eq!(methods.len(), 3);
    assert_eq!(
        methods
            .iter()
            .map(|node| &node.id)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    let unresolved = graph
        .nodes
        .iter()
        .find(|node| node.kind == "function" && node.label == "unresolved")
        .unwrap();
    let expected_target = format!(
        "{}::Foo::convert",
        unresolved.id.rsplit_once("::").unwrap().0
    );
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_expr" && edge.from == unresolved.id && edge.to == expected_target
    }));
    assert!(!graph.edges.iter().any(|edge| {
        edge.kind == "references_expr"
            && edge.from == unresolved.id
            && methods.iter().any(|method| edge.to == method.id)
    }));
}

#[test]
fn ambiguous_unqualified_trait_method_call_remains_unresolved() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Foo;
        pub trait First { fn call(); }
        pub trait Second { fn call(); }
        impl First for Foo { fn call() {} }
        impl Second for Foo { fn call() {} }
        pub fn unresolved() { Foo::call(); }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let unresolved = graph
        .nodes
        .iter()
        .find(|node| node.kind == "function" && node.label == "unresolved")
        .unwrap();
    let expected_target = format!("{}::Foo::call", unresolved.id.rsplit_once("::").unwrap().0);
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_expr" && edge.from == unresolved.id && edge.to == expected_target
    }));
}

#[test]
fn self_method_prefers_inherent_dispatch_while_ufcs_dispatches_to_the_trait_impl() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Adapter;
        pub trait Action { fn first(); fn second(); }
        impl Adapter { fn second() {} }
        impl Action for Adapter {
            fn first() { Self::second(); <Self as Action>::second(); }
            fn second() {}
        }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let first = graph
        .nodes
        .iter()
        .find(|node| node.label == "first" && node.impl_trait.as_deref() == Some("Action"))
        .unwrap();
    let inherent_second = graph
        .nodes
        .iter()
        .find(|node| node.label == "second" && node.impl_kind == Some(ImplKind::Inherent))
        .unwrap();
    let trait_second = graph
        .nodes
        .iter()
        .find(|node| node.label == "second" && node.impl_trait.as_deref() == Some("Action"))
        .unwrap();
    for target in [&inherent_second.id, &trait_second.id] {
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == "references_expr" && edge.from == first.id && edge.to == *target
        }));
    }
}

#[test]
fn qualified_non_generic_self_path_keeps_the_established_trait_impl_id() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Foo;
        pub trait Action { fn act(); }
        impl Action for crate::Foo { fn act() {} }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let owner = graph.nodes.iter().find(|node| node.label == "Foo").unwrap();
    let implementation = graph
        .nodes
        .iter()
        .find(|node| node.kind == "impl" && node.impl_trait.as_deref() == Some("Action"))
        .unwrap();
    assert_eq!(
        implementation.id.as_str(),
        format!("{}::impl::416374696f6e", owner.id)
    );
    assert!(!implementation.id.as_str().contains("::self::"));
}

#[test]
fn non_reference_generic_trait_impl_remains_an_unqualified_method_candidate() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        pub struct Foo;
        pub trait Borrowed<T> { fn call(); }
        impl Borrowed<&'static str> for Foo { fn call() {} }
        pub fn resolved() { Foo::call(); }
    "#,
    );
    let graph = export_workspace_graph(&ExportGraphOptions {
        root: fixture.root().to_path_buf(),
    })
    .unwrap();
    let resolved = graph
        .nodes
        .iter()
        .find(|node| node.kind == "function" && node.label == "resolved")
        .unwrap();
    let method = graph
        .nodes
        .iter()
        .find(|node| node.kind == "method" && node.impl_trait.as_deref() == Some("Borrowed"))
        .unwrap();
    assert!(graph.edges.iter().any(|edge| {
        edge.kind == "references_expr" && edge.from == resolved.id && edge.to == method.id
    }));
}

fn configured(
    root: &Path,
    rule_filter: Option<&str>,
) -> std::result::Result<FindingsReport, BoundaryError> {
    analyze_workspace_configured(
        &AnalyzeOptions {
            root: root.to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        rule_filter,
    )
}

/// `app` depends on `api`; neither record allows the edge, so the
/// dependencies pass reports SCB-DEPENDENCY-001 and SCB-DEPENDENCY-002.
fn undeclared_dependency_fixture() -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/api"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "api", "../api", "0.1.0"),
    );
    fixture.write("crates/api/Cargo.toml", &good_member_manifest("api", ""));
    fixture.write_member_boundary_record("app", &[], &[], &[]);
    fixture.write_member_boundary_record("api", &[], &[], &[]);
    fixture.write_source("app", "lib.rs", "pub struct App;");
    fixture.write_source("api", "lib.rs", "pub struct Api;");
    fixture
}

fn cycle_fixture() -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub struct Alpha { pub beta: Beta }\npub struct Beta { pub alpha: Alpha }\n",
    );
    fixture
}

#[test]
fn disabled_table_reports_pass_without_findings() {
    let fixture = undeclared_dependency_fixture();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            enabled = false
            reason = "boundary policy adopted later"
        "#,
    );

    let report = configured(fixture.root(), None).expect("disabled analyzer succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.scanned_crates, 0);
    assert!(report.findings.is_empty());
}

#[test]
fn unknown_table_key_is_config_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            targets = ["linux"]
        "#,
    );

    match configured(fixture.root(), None) {
        Err(BoundaryError::AnalyzerConfig(error)) => {
            assert_eq!(error.code(), "ANALYZER_CONFIG.UNKNOWN_KEY");
            assert_eq!(
                error.to_string(),
                "boundary: unknown key `targets` in [tool.sc-lint.analyzers.boundary]"
            );
        }
        other => panic!("expected AnalyzerConfig, got {other:?}"),
    }
}

#[test]
fn unknown_rule_filter_is_selector_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    match configured(fixture.root(), Some("SCB-NOPE-001")) {
        Err(BoundaryError::AnalyzerConfig(error)) => {
            assert_eq!(error.code(), "ANALYZER_CONFIG.SELECTOR");
            assert_eq!(
                error.to_string(),
                "boundary: unknown rule selector `SCB-NOPE-001`"
            );
        }
        other => panic!("expected AnalyzerConfig, got {other:?}"),
    }
}

#[test]
fn manifest_policy_shape_errors_are_indexed_analyzer_config_errors() {
    for (config, expected_key, explanation) in [
        (
            "manifest_dependency_allowlists = \"wrong\"",
            "manifest_dependency_allowlists",
            "use array-of-table entries",
        ),
        (
            "manifest_dependency_allowlists = [\"wrong\"]",
            "manifest_dependency_allowlists[0]",
            "replace it with a table entry",
        ),
        (
            "[[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]\nowner_manifest_path = 42",
            "manifest_dependency_allowlists[0].owner_manifest_path",
            "provide a quoted string",
        ),
    ] {
        let fixture = undeclared_dependency_fixture();
        fixture.write(
            "sc-lint.toml",
            &format!("[tool.sc-lint.analyzers.boundary]\n{config}\n"),
        );

        match configured(fixture.root(), None) {
            Err(BoundaryError::AnalyzerConfig(error)) => {
                assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
                assert!(error.to_string().contains(expected_key));
                assert!(error.to_string().contains(explanation));
            }
            other => panic!("expected indexed analyzer config error, got {other:?}"),
        }
    }
}

#[test]
fn rule_filter_selects_exactly_the_rule() {
    let fixture = undeclared_dependency_fixture();

    let all =
        configured(fixture.root(), Some("dependencies")).expect("dependency analysis succeeds");
    let mut all_ids: Vec<RuleId> = all.findings.iter().map(|finding| finding.rule_id).collect();
    all_ids.sort();
    assert_eq!(
        all_ids,
        vec![RuleId::ScbDependency001, RuleId::ScbDependency002]
    );

    let report = configured(fixture.root(), Some("SCB-DEPENDENCY-001"))
        .expect("dependency analysis succeeds");

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbDependency001);
}

#[test]
fn inventory_free_manifest_policy_runs_with_missing_or_invalid_boundaries() {
    for invalid_boundaries in [false, true] {
        for with_policy in [false, true] {
            let fixture = inventory_free_policy_fixture(invalid_boundaries, with_policy);
            let ownership = configured(fixture.root(), Some("SCB-MANIFEST-004"))
                .expect("ownership policy does not require boundary inventory");
            let sections = configured(fixture.root(), Some("SCB-MANIFEST-005"))
                .expect("section policy does not require boundary inventory");

            if with_policy {
                assert_eq!(ownership.status, ReportStatus::Fail);
                assert!(ownership.findings.iter().any(|finding| {
                    finding.rule_id == RuleId::ScbManifest004
                        && finding.message.contains("not permitted")
                }));
                assert_eq!(sections.status, ReportStatus::Fail);
                assert_eq!(sections.findings.len(), 1);
                assert_eq!(sections.findings[0].rule_id, RuleId::ScbManifest005);
            } else {
                assert_eq!(ownership.status, ReportStatus::Pass);
                assert!(ownership.findings.is_empty());
                assert_eq!(sections.status, ReportStatus::Pass);
                assert!(sections.findings.is_empty());
            }
        }
    }
}

#[test]
fn boundary_semantics_only_does_not_parse_unselected_manifest_policy() {
    let fixture = undeclared_dependency_fixture();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            global_dependency_ownership = "not an array of tables"
        "#,
    );

    configured(fixture.root(), Some("SCB-BOUNDARY-004"))
        .expect("boundary semantic selection ignores unselected manifest policy");
}

fn inventory_free_policy_fixture(invalid_boundaries: bool, with_policy: bool) -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app", "crates/dep"]);
    fixture.write(
        "crates/app/Cargo.toml",
        &path_dependency_manifest("app", "dep", "../dep", "0.1.0"),
    );
    fixture.write("crates/dep/Cargo.toml", &good_member_manifest("dep", ""));
    fixture.write_source("app", "lib.rs", "use dep::Thing;\npub struct App;");
    fixture.write_source("dep", "lib.rs", "pub struct Thing;");
    if invalid_boundaries {
        fixture.write("boundaries/planning.toml", "not_planning = true\n");
    }
    if with_policy {
        fixture.write(
            "sc-lint.toml",
            r#"
                [tool.sc-lint.analyzers.boundary]
                manifest_dependency_allowlists = "not selected by ownership or section rules"

                [[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
                dependency = "dep"
                allowed_manifest_paths = []
                allowed_source_roots = []
                manifest_message = "not permitted"
                source_message = "not permitted"

                [[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
                owner_manifest_path = "crates/app/Cargo.toml"
                dependency_package = "dep"
                allowed_sections = ["dev-dependencies"]
                message = "dependency belongs in dev-dependencies"
            "#,
        );
    }
    fixture
}

#[test]
fn rule_filter_family_keeps_existing_cycles_behaviour() {
    let fixture = cycle_fixture();

    let legacy = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .expect("legacy cycles analysis succeeds");
    let report = configured(fixture.root(), Some("cycles")).expect("cycles family succeeds");

    assert_eq!(report, legacy);
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings[0].rule_id, RuleId::ScbCycle001);
}

#[test]
fn every_family_matches_its_legacy_rule_filter() {
    let cycle = cycle_fixture();
    let dependency = undeclared_dependency_fixture();
    let multi_target = multi_target_fixture();
    let invalid_inventory = invalid_inventory_fixture();
    let manifest_policy_error = manifest_policy_error_fixture();
    analyze_workspace(&AnalyzeOptions {
        root: manifest_policy_error.root().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .expect("legacy cycles analysis skips manifest policy");
    let pairs = [
        (RuleFilter::Cycles, "cycles"),
        (RuleFilter::Boundaries, "boundaries"),
        (RuleFilter::InternalOnly, "internal-only"),
        (RuleFilter::ForbidExternalImpls, "forbid-external-impls"),
        (RuleFilter::Dependencies, "dependencies"),
        (RuleFilter::Manifests, "manifests"),
    ];
    assert_eq!(pairs.len(), BOUNDARY_CATALOG.families.len());
    for root in [
        cycle.root(),
        dependency.root(),
        multi_target.root(),
        invalid_inventory.root(),
        manifest_policy_error.root(),
    ] {
        for (filter, family) in pairs {
            let legacy = analyze_workspace(&AnalyzeOptions {
                root: root.to_path_buf(),
                format: OutputFormat::Json,
                rule: Some(filter),
            });
            let configured = configured(root, Some(family));
            match (legacy, configured) {
                (Ok(legacy), Ok(configured)) => assert_eq!(configured, legacy, "{family}"),
                (Err(legacy), Err(configured)) => {
                    assert_eq!(configured.to_string(), legacy.to_string(), "{family}");
                }
                (legacy, configured) => panic!(
                    "{family}: configured and legacy results differ: {configured:?} versus {legacy:?}"
                ),
            }
        }
    }
}

fn multi_target_fixture() -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");
    fixture.write_source("example", "main.rs", "fn main() {}");
    fixture.write_source("example", "bin/extra.rs", "fn main() {}");
    fixture
}

fn invalid_inventory_fixture() -> WorkspaceFixture {
    let fixture = multi_target_fixture();
    fixture.write("boundaries/planning.toml", "not_planning = true\n");
    fixture
}

fn manifest_policy_error_fixture() -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write(
        "Cargo.toml",
        r#"
            [workspace]
            members = ["crates/example"]
            resolver = "2"

            [workspace.package]
            edition = "2024"
            rust-version = "1.94.1"
            authors = ["sc-lint contributors"]
            license = "MIT OR Apache-2.0"
            repository = "https://example.invalid/sc-lint"
            homepage = "https://example.invalid/sc-lint"
        "#,
    );
    fixture.write(
        "crates/example/Cargo.toml",
        &good_member_manifest("example", "").replacen(
            "version.workspace = true\n",
            "version = \"0.1.0\"\n",
            1,
        ),
    );
    fixture.write_member_boundary_record("example", &[], &[], &[]);
    fixture.write_source("example", "lib.rs", "pub struct Example;");
    fixture
}

#[test]
fn kebab_case_family_is_accepted() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let report = configured(fixture.root(), Some("forbid-external-impls"))
        .expect("kebab-case family is a valid selector");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn exclude_paths_drops_manifest_findings() {
    let fixture = ManifestPolicyFixture::new(&["crates/example"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest(
        "example",
        &good_member_manifest("example", "").replace("homepage.workspace = true\n", ""),
    );
    fixture.write_member_source("example", "pub struct Example;");

    let before = configured(fixture.root(), Some("manifests")).expect("manifest analysis succeeds");
    assert_eq!(
        finding_messages(&before),
        vec!["crates/example/Cargo.toml: set [package].homepage.workspace = true".to_string()]
    );
    assert_eq!(
        before.findings[0].owner_ids[0].as_str(),
        "crates/example/Cargo.toml"
    );

    fixture.workspace.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            exclude_paths = ["crates/example/**"]
        "#,
    );
    let report = configured(fixture.root(), Some("manifests")).expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn per_rule_path_exclusion_drops_only_matching_manifest_rule() {
    let fixture = ManifestPolicyFixture::new(&["crates/example"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest(
        "example",
        &good_member_manifest("example", "").replace("homepage.workspace = true\n", ""),
    );
    fixture.write_member_source("example", "pub struct Example;");
    fixture.workspace.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
            "SCB-MANIFEST-001" = ["crates/example/Cargo.toml"]
        "#,
    );

    let report = configured(fixture.root(), Some("manifests")).expect("manifest analysis succeeds");

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn issue385_boundary_manifest_paths() {
    let fixture = ManifestPolicyFixture::new(&["crates/example"]);
    fixture.write_workspace_package_defaults("1.1.2");
    fixture.write_member_manifest(
        "example",
        &good_member_manifest("example", "").replace("homepage.workspace = true\n", ""),
    );
    fixture.write_member_source("example", "pub struct Example;");
    let baseline = configured(fixture.root(), Some("manifests")).unwrap();
    let baseline_json = serde_json::to_vec(&baseline).unwrap();

    fixture
        .workspace
        .write("sc-lint.toml", "[tool.sc-lint.analyzers.boundary]\n");
    assert_eq!(
        serde_json::to_vec(&configured(fixture.root(), Some("manifests")).unwrap()).unwrap(),
        baseline_json
    );

    fixture.workspace.write(
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]\n",
    );
    assert_eq!(
        serde_json::to_vec(&configured(fixture.root(), Some("manifests")).unwrap()).unwrap(),
        baseline_json
    );

    fixture.workspace.write(
        "sc-lint.toml",
        r#"
        [tool.sc-lint.analyzers.boundary]
        exclude_paths = ["crates/example/Cargo.toml"]
        [tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
        "SCB-MANIFEST-001" = ["crates/other/Cargo.toml"]
        "#,
    );
    assert!(
        configured(fixture.root(), Some("manifests"))
            .unwrap()
            .findings
            .is_empty()
    );

    fixture.workspace.write(
        "sc-lint.toml",
        r#"
        [tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
        "SCB-MANIFEST-001" = ["crates/example/Cargo.toml"]
        "#,
    );
    let settings = sc_lint_analyzer_support::load_analyzer_settings(
        &BOUNDARY_CATALOG,
        fixture.root(),
        None,
        Some("manifests"),
    )
    .unwrap();
    let (findings, status) = settings.apply_with_paths(
        vec![
            (
                baseline.findings[0].clone(),
                Some(
                    sc_lint_analyzer_support::RepoRelativePath::parse(
                        "crates\\example\\Cargo.toml",
                    )
                    .unwrap(),
                ),
            ),
            (baseline.findings[0].clone(), None),
            (
                baseline.findings[0].clone(),
                Some(
                    sc_lint_analyzer_support::RepoRelativePath::parse("crates/example/other.toml")
                        .unwrap(),
                ),
            ),
        ],
        |rule| sc_lint_analyzer_support::CatalogRuleId::new(rule.as_str()),
    );
    assert_eq!(findings.len(), 2);
    assert_eq!(status, ReportStatus::Fail);
}
