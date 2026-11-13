use sc_lint_boundary::{AnalyzeOptions, analyze_workspace_configured};
use sc_lint_schema::OutputFormat;
use std::{fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn example_targets_do_not_change_boundary_findings() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=['app']\nresolver='2'\n[workspace.package]\nversion='0.1.0'\nedition='2021'\nrust-version='1.85'\nauthors=['test']\nlicense='MIT'\nrepository='https://example.invalid'\nhomepage='https://example.invalid'\n",
    );
    write(
        root,
        "app/Cargo.toml",
        "[package]\nname='app'\nversion.workspace=true\nedition.workspace=true\nrust-version.workspace=true\nauthors.workspace=true\nlicense.workspace=true\nrepository.workspace=true\nhomepage.workspace=true\n",
    );
    write(root, "app/src/lib.rs", "pub fn clean() {}\n");
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.boundary]\n[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]\ndependency='cron'\nallowed_manifest_paths=[]\nallowed_source_roots=[]\nmanifest_message='not declared'\nsource_message='not imported'\n",
    );
    let options = AnalyzeOptions {
        root: root.to_path_buf(),
        format: OutputFormat::Json,
        rule: None,
    };
    let before = analyze_workspace_configured(&options, None, Some("SCB-MANIFEST-004")).unwrap();
    write(
        root,
        "app/examples/example.rs",
        "use cron::Schedule;\nfn main() {}\n",
    );
    let after = analyze_workspace_configured(&options, None, Some("SCB-MANIFEST-004")).unwrap();
    assert!(before.findings.is_empty());
    assert_eq!(before.findings, after.findings);
    assert_eq!(before.status, after.status);
}
