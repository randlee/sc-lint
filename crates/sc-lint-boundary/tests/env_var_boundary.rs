use std::fs;
use std::path::Path;

use sc_lint_boundary::{AnalyzeOptions, RuleId, analyze_workspace_configured};
use sc_lint_schema::OutputFormat;
use tempfile::TempDir;

fn write(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn detects_direct_and_same_file_wrappers_but_respects_boundaries() {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=['restricted', 'other']\nresolver='2'\n[workspace.package]\nversion='0.1.0'\n",
    );
    for name in ["restricted", "other"] {
        write(
            root,
            &format!("{name}/Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2021'\n"),
        );
    }
    write(
        root,
        "sc-lint.toml",
        "[[tool.sc-lint.analyzers.boundary.env_var_boundary]]\nforbidden_env_vars=['SECRET']\nrestricted_crate_roots=['restricted']\nboundary_reader_functions=['restricted::read_secret']\n",
    );
    write(
        root,
        "restricted/src/lib.rs",
        concat!(
            "fn direct() { std::env::var(\"SECRET\"); }\n",
            "fn direct_os() { std::env::var_os(\"SECRET\"); }\n",
            "fn forward(key: &str) { std::env::var(key); }\n",
            "fn middle(key: &str) { forward(key); }\n",
            "fn uses_forward() { forward(\"SECRET\"); }\n",
            "fn uses_middle() { middle(\"SECRET\"); }\n",
            "fn read_secret() { std::env::var(\"SECRET\"); }\n",
            "fn other_var() { std::env::var(\"SAFE\"); }\n",
            "fn guard_style(key: &str) { env::var_os(key); }\n",
            "fn guard_call() { guard_style(\"SECRET\"); }\n",
            "fn fixed() { forward(\"SECRET\"); }\n",
            "fn call_fixed() { fixed(); }\n",
            "fn calls_reader() { read_secret(); }\n",
            "#[cfg(test)] mod tests { fn ignored() { std::env::var(\"SECRET\"); } }\n",
            "#[test] fn ignored_too() { std::env::var(\"SECRET\"); }\n",
        ),
    );
    write(
        root,
        "other/src/lib.rs",
        "fn outside() { std::env::var(\"SECRET\"); }\n",
    );
    write(
        root,
        "restricted/tests/check.rs",
        "fn test_file() { std::env::var(\"SECRET\"); }\n",
    );
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: root.to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-ENV-001"),
    )
    .unwrap();
    let findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == RuleId::ScbEnv001)
        .collect();
    assert_eq!(findings.len(), 7, "{findings:#?}");
    for line in [1, 2, 5, 6, 10, 11, 12] {
        assert!(
            findings.iter().any(|f| f
                .message
                .contains(&format!("restricted/src/lib.rs:{line}:"))),
            "{findings:#?}"
        );
    }
}
