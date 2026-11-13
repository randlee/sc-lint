use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn write(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn run(root: &Path) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args([
            "--json",
            "--root",
            root.to_str().unwrap(),
            "lint",
            "sc-boundary",
        ])
        .output()
        .unwrap();
    let json = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap(), json)
}

#[test]
fn configured_manifest_requirements_report_and_clean_or_absent_policy_passes() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "boundaries/planning.toml",
        "[planning]\ncurrent_sprint = \"Q.13\"\n",
    );
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=['app']\nresolver='2'\n[workspace.package]\nversion='0.1.0'\nedition='2021'\nrust-version='1.85'\nauthors=['Example']\nlicense='MIT'\nrepository='https://example.invalid/repo'\nhomepage='https://example.invalid'\n",
    );
    write(
        root,
        "app/Cargo.toml",
        "[package]\nname='app'\nversion.workspace=true\nedition.workspace=true\nrust-version.workspace=true\nauthors.workspace=true\nlicense.workspace=true\nrepository.workspace=true\nhomepage.workspace=true\n",
    );
    write(root, "app/src/lib.rs", "pub fn ok() {}\n");
    write(
        root,
        "templates/Cargo.toml",
        "[dependencies]\nexample='=1.2.3'\nother='>=1.2.3, <1.5.0'\n",
    );
    write(
        root,
        "sc-lint.toml",
        "[[tool.sc-lint.analyzers.boundary.dependency_requirements]]\ntemplate_manifests=['templates/Cargo.toml']\n",
    );
    let (status, report) = run(root);
    assert_eq!(status, 10, "{report}");
    let findings = report["data"]["findings"].as_array().unwrap();
    for id in ["SCB-MANIFEST-007", "SCB-MANIFEST-008"] {
        assert!(
            findings.iter().any(|finding| finding["rule_id"] == id
                && finding["message"]
                    .as_str()
                    .unwrap()
                    .contains("templates/Cargo.toml")),
            "{report}"
        );
    }
    write(
        root,
        "templates/Cargo.toml",
        "[dependencies]\nexample='^1.2.3'\nother='^1.2.3'\n",
    );
    let (status, report) = run(root);
    assert_eq!(status, 0, "{report}");
    write(
        root,
        "templates/Cargo.toml",
        "[dependencies]\nexample='=1.2.3'\nother='>=1.2.3, <1.5.0'\n",
    );
    write(root, "sc-lint.toml", "[tool.sc-lint.analyzers.boundary]\n");
    let (status, report) = run(root);
    assert_eq!(status, 0, "{report}");
    assert!(
        report["data"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["rule_id"] != "SCB-MANIFEST-007"
                && finding["rule_id"] != "SCB-MANIFEST-008"),
        "{report}"
    );
}
