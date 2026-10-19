use std::fs;
use std::process::Command;

#[test]
fn feature_rule_obeys_dependency_family_filter() {
    let fixture = tempfile::TempDir::new().unwrap();
    for (path, contents) in [
        (
            "Cargo.toml",
            include_str!("fixtures/scb-feature-001/Cargo.toml"),
        ),
        (
            "app/Cargo.toml",
            include_str!("fixtures/scb-feature-001/app/Cargo.toml"),
        ),
        (
            "app/src/lib.rs",
            include_str!("fixtures/scb-feature-001/app/src/lib.rs"),
        ),
        (
            "api/Cargo.toml",
            include_str!("fixtures/scb-feature-001/api/Cargo.toml"),
        ),
        (
            "api/src/lib.rs",
            include_str!("fixtures/scb-feature-001/api/src/lib.rs"),
        ),
        (
            "boundaries/planning.toml",
            include_str!("fixtures/scb-feature-001/boundaries/planning.toml"),
        ),
        (
            "boundaries/app/boundary.toml",
            include_str!("fixtures/scb-feature-001/boundaries/app/boundary.toml"),
        ),
    ] {
        let path = fixture.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    for (filter, expected) in [
        (None, true),
        (Some("dependencies"), true),
        (Some("manifests"), false),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sc-lint-boundary"));
        command
            .args(["analyze", "--format", "json", "--root"])
            .arg(fixture.path());
        if let Some(filter) = filter {
            command.args(["--rule-filter", filter]);
        }
        let output = command.output().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stdout.contains("SCB-FEATURE-001"),
            expected,
            "filter {filter:?}; stdout: {stdout}; stderr: {stderr}"
        );
        assert!(
            output.status.success(),
            "filter {filter:?}; stdout: {stdout}; stderr: {stderr}"
        );
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["status"], if expected { "fail" } else { "pass" });
    }
}
