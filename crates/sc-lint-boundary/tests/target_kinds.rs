use std::path::PathBuf;
use std::process::Command;

use sc_lint_analyzer_support::discover_source_files;
use sc_lint_boundary::{AnalyzeOptions, RuleFilter, analyze_workspace};
use sc_lint_schema::OutputFormat;
use serde_json::Value;

#[test]
fn library_kinds_are_exported_scanned_and_resolved() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/target-kinds");
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-boundary"))
        .args(["export-graph", "--format", "json", "--root"])
        .arg(&root)
        .output()
        .expect("run graph export");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let graph: Value = serde_json::from_slice(&output.stdout).expect("graph JSON");
    let nodes = graph["nodes"].as_array().expect("nodes");
    let sources = discover_source_files(&root).expect("discover source files");
    for name in [
        "rlib",
        "dylib",
        "cdylib",
        "staticlib",
        "proc_macro",
        "multi",
    ] {
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node["kind"] == "crate" && node["package"] == name)
                .count(),
            1,
            "{name}"
        );
        assert!(
            sources.iter().any(|file| file.package.as_str() == name),
            "{name}"
        );
    }
    assert!(
        !nodes
            .iter()
            .any(|node| node["kind"] == "crate" && node["package"] == "excluded")
    );
    assert!(
        !sources
            .iter()
            .any(|file| file.package.as_str() == "excluded")
    );
    let report = analyze_workspace(&AnalyzeOptions {
        root,
        format: OutputFormat::Json,
        rule: Some(RuleFilter::Cycles),
    })
    .expect("analyze workspace");
    assert_eq!(report.scanned_crates, 6);
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| {
                edge["kind"] == "references_expr"
                    && edge["from"].as_str().unwrap_or_default().contains("multi")
                    && edge["to"].as_str().unwrap_or_default().contains("rlib")
                    && edge["to"].as_str().unwrap_or_default().contains("marker")
            }),
        "missing cross-crate reference: {}",
        graph["edges"]
    );
}
