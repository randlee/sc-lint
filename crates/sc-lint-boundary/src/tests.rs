#![cfg(test)]

use super::*;
use crate::inventory::owner_crate_path_for_package;
use crate::render::hex_encode;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn fixture_display_path(relative_path: &str) -> String {
    std::env::temp_dir()
        .join("sc-lint-boundary-tests")
        .join(relative_path)
        .display()
        .to_string()
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
                owner_crate_path_for_package(owner_package),
                owner_crate_path_for_package(owner_package),
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
