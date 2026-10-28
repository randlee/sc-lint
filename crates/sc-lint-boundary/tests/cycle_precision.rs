//! SCB-CYCLE-002/003 precision (ADR-020 decision 11) and path exclusions for
//! path-less cycle findings (ADR-020 decision 5).

use std::fs;
use std::path::Path;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::ExportGraphOptions;
use sc_lint_boundary::RuleFilter;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_boundary::export_workspace_graph;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use tempfile::TempDir;

const CASES: [(&str, &str); 7] = [
    (
        "case_1",
        include_str!("fixtures/cycle_precision/case_1/lib.rs"),
    ),
    (
        "case_2",
        include_str!("fixtures/cycle_precision/case_2/lib.rs"),
    ),
    (
        "case_3",
        include_str!("fixtures/cycle_precision/case_3/lib.rs"),
    ),
    (
        "case_4",
        include_str!("fixtures/cycle_precision/case_4/lib.rs"),
    ),
    (
        "case_5",
        include_str!("fixtures/cycle_precision/case_5/lib.rs"),
    ),
    (
        "case_6",
        include_str!("fixtures/cycle_precision/case_6/lib.rs"),
    ),
    (
        "case_7",
        include_str!("fixtures/cycle_precision/case_7/lib.rs"),
    ),
];

const LOOP: &str =
    "pub struct T; impl T { pub fn a(&self) { self.b() } pub fn b(&self) { self.a() } }";

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path has a parent")).expect("create dirs");
    fs::write(path, contents).expect("write fixture file");
}

fn write_workspace(source: &str) -> TempDir {
    let fixture = TempDir::new().expect("tempdir");
    write(
        fixture.path(),
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
    write(
        fixture.path(),
        "crates/example/Cargo.toml",
        r#"
[package]
name = "example"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
"#,
    );
    write(fixture.path(), "crates/example/src/lib.rs", source);
    fixture
}

fn cycle_rule_ids(source: &str) -> Vec<RuleId> {
    let fixture = write_workspace(source);
    analyze_workspace(&AnalyzeOptions {
        root: fixture.path().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .expect("cycle analysis succeeds")
    .findings
    .iter()
    .map(|finding| finding.rule_id)
    .collect()
}

fn configured_cycle_rule_ids(source: &str, sc_lint_toml: &str) -> (Vec<RuleId>, ReportStatus) {
    let fixture = write_workspace(source);
    write(fixture.path(), "sc-lint.toml", sc_lint_toml);
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.path().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("cycles"),
    )
    .expect("configured cycle analysis succeeds");
    let rule_ids = report
        .findings
        .iter()
        .map(|finding| finding.rule_id)
        .collect();
    (rule_ids, report.status)
}

fn references_expr_edges(source: &str) -> Vec<(String, String)> {
    let fixture = write_workspace(source);
    export_workspace_graph(&ExportGraphOptions {
        root: fixture.path().to_path_buf(),
    })
    .expect("graph export succeeds")
    .edges
    .iter()
    .filter(|edge| edge.kind == "references_expr")
    .map(|edge| (edge.from.as_str().to_owned(), edge.to.as_str().to_owned()))
    .collect()
}

fn assert_cases_report_nothing(names: &[&str]) {
    for name in names {
        let (_, source) = CASES
            .iter()
            .find(|(case, _)| case == name)
            .expect("case exists");
        assert_eq!(cycle_rule_ids(source), Vec::<RuleId>::new(), "{name}");
    }
}

#[test]
fn method_call_loop_reports_cycle_002() {
    assert_eq!(cycle_rule_ids(LOOP), vec![RuleId::ScbCycle002]);
}

#[test]
fn method_call_chain_without_loop_reports_nothing() {
    assert!(
        cycle_rule_ids("pub struct T; impl T { pub fn a(&self) { self.b() } pub fn b(&self) {} }")
            .is_empty()
    );
}

#[test]
fn parenthesized_and_reference_self_receivers_create_edges() {
    assert_eq!(
        cycle_rule_ids(
            "pub struct T; impl T { pub fn a(&self) { (*self).b() } pub fn b(&self) { (&self).a() } }"
        ),
        vec![RuleId::ScbCycle002]
    );
}

#[test]
fn non_self_receivers_create_no_edges() {
    assert!(
        cycle_rule_ids(
            "pub struct T; impl T { pub fn a(&self, o: &T) { o.b(o) } pub fn b(&self, o: &T) { o.a(o) } }"
        )
        .is_empty()
    );
}

#[test]
fn trait_impl_forwarding_to_own_method_stays_cycle_003() {
    assert_eq!(
        cycle_rule_ids(
            "pub struct T; pub trait Run { fn run(&self); } impl Run for T { fn run(&self) { self.run() } }"
        ),
        vec![RuleId::ScbCycle003]
    );
}

#[test]
fn clone_impl_field_clones_are_not_cycles() {
    assert_cases_report_nothing(&["case_1", "case_2"]);
}

#[test]
fn non_ignored_trait_returning_self_still_reports_cycle_003() {
    assert_eq!(
        cycle_rule_ids(
            "pub struct T { n: u32 } pub trait Duplicate { fn duplicate(&self) -> Self; } impl Duplicate for T { fn duplicate(&self) -> Self { Self { n: self.n } } }"
        ),
        vec![RuleId::ScbCycle003]
    );
}

#[test]
fn delegation_edge_trait_impl_to_same_named_inherent() {
    assert_cases_report_nothing(&["case_3", "case_4", "case_5"]);
}

#[test]
fn delegation_edge_pub_to_other_pub() {
    assert_cases_report_nothing(&["case_6", "case_7"]);
}

#[test]
fn self_receiver_call_creates_references_expr_edge() {
    let edges = references_expr_edges(
        "pub struct T; impl T { pub fn a(&self) { self.step() } pub fn step(&self) {} }",
    );
    assert_eq!(edges.len(), 1, "{edges:?}");
    let (from, to) = &edges[0];
    assert!(from.ends_with("::T::a"), "{from}");
    assert!(to.ends_with("::T::step"), "{to}");
}

#[test]
fn field_receiver_call_creates_no_edge() {
    let edges = references_expr_edges(
        "pub struct T { x: Vec<u8> } impl T { pub fn a(&self) -> Vec<u8> { self.x.clone() } }",
    );
    assert!(edges.is_empty(), "{edges:?}");
}

#[test]
fn unconfigured_loop_reports_cycle_002_with_pass_status() {
    assert_eq!(
        configured_cycle_rule_ids(LOOP, ""),
        (vec![RuleId::ScbCycle002], ReportStatus::Pass)
    );
}

#[test]
fn path_exclusion_applies_to_path_less_cycle_finding() {
    assert_eq!(
        configured_cycle_rule_ids(
            LOOP,
            r#"
[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
"SCB-CYCLE-002" = ["crates/example/src/lib.rs"]
"#
        ),
        (Vec::new(), ReportStatus::Pass)
    );
}

#[test]
fn global_exclude_paths_apply_to_path_less_cycle_finding() {
    assert_eq!(
        configured_cycle_rule_ids(
            LOOP,
            r#"
[tool.sc-lint.analyzers.boundary]
exclude_paths = ["crates/example/src/lib.rs"]
"#
        ),
        (Vec::new(), ReportStatus::Pass)
    );
}

#[test]
fn path_exclusion_for_unrelated_file_keeps_finding() {
    assert_eq!(
        configured_cycle_rule_ids(
            LOOP,
            r#"
[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
"SCB-CYCLE-002" = ["crates/example/src/other.rs"]
"#
        ),
        (vec![RuleId::ScbCycle002], ReportStatus::Pass)
    );
    assert_eq!(
        configured_cycle_rule_ids(
            LOOP,
            r#"
[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
"SCB-CYCLE-003" = ["crates/example/src/lib.rs"]
"#
        ),
        (vec![RuleId::ScbCycle002], ReportStatus::Pass)
    );
}

const TRAIT_SELF_LOOP: &str = "pub struct T; pub trait Run { fn run(&self); } impl Run for T { fn run(&self) { self.run() } }";

fn cycle_findings(source: &str) -> Vec<(RuleId, String, usize)> {
    let fixture = write_workspace(source);
    analyze_workspace(&AnalyzeOptions {
        root: fixture.path().to_path_buf(),
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .expect("cycle analysis succeeds")
    .findings
    .into_iter()
    .map(|finding| (finding.rule_id, finding.message, finding.node_ids.len()))
    .collect()
}

#[test]
fn multi_node_trait_impl_cycle_stays_cycle_003() {
    // Two trait-impl methods calling each other reach back, so neither edge is a
    // delegation edge (ADR-020 decision 11): each trait reports the two-node loop.
    let findings = cycle_findings(
        "pub struct T; pub trait A { fn a(&self); } pub trait B { fn b(&self); } impl A for T { fn a(&self) { self.b() } } impl B for T { fn b(&self) { self.a() } }",
    );
    assert_eq!(findings.len(), 2, "{findings:?}");
    for (via, (rule_id, message, nodes)) in ["A", "B"].iter().zip(&findings) {
        assert_eq!(*rule_id, RuleId::ScbCycle003, "{findings:?}");
        assert!(message.ends_with(&format!("via {via}")), "{message}");
        assert_eq!(*nodes, 2, "{findings:?}");
    }

    // A trait-impl method looping through a private inherent helper keeps its
    // SCB-CYCLE-003; the inherent edge adds the SCB-CYCLE-002 type loop.
    let findings = cycle_findings(
        "pub struct T; pub trait Run { fn run(&self); } impl Run for T { fn run(&self) { self.helper() } } impl T { fn helper(&self) { self.run() } }",
    );
    let rule_ids = findings
        .iter()
        .map(|(rule_id, _, _)| *rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        rule_ids,
        vec![RuleId::ScbCycle002, RuleId::ScbCycle003],
        "{findings:?}"
    );
    assert!(findings[1].1.ends_with("via Run"), "{findings:?}");
    assert!(
        findings.iter().all(|(_, _, nodes)| *nodes == 2),
        "{findings:?}"
    );
}

#[test]
fn path_exclusion_drops_real_cycle_003_finding() {
    assert_eq!(
        configured_cycle_rule_ids(TRAIT_SELF_LOOP, ""),
        (vec![RuleId::ScbCycle003], ReportStatus::Pass)
    );
    assert_eq!(
        configured_cycle_rule_ids(
            TRAIT_SELF_LOOP,
            r#"
[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
"SCB-CYCLE-003" = ["crates/example/src/lib.rs"]
"#
        ),
        (Vec::new(), ReportStatus::Pass)
    );
    assert_eq!(
        configured_cycle_rule_ids(
            TRAIT_SELF_LOOP,
            r#"
[tool.sc-lint.analyzers.boundary.exclude_paths_by_rule]
"SCB-CYCLE-002" = ["crates/example/src/lib.rs"]
"#
        ),
        (vec![RuleId::ScbCycle003], ReportStatus::Pass)
    );
}
