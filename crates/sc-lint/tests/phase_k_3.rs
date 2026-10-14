use std::fs;
use std::process::Command;

use sc_lint_boundary::ALL_BOUNDARY_IDS;
use sc_lint_boundary::BOUNDARY_CATALOG;
use sc_lint_boundary::DEFAULT_FAIL_IDS;
use sc_lint_boundary::RuleId;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn issue377_rule_registration_and_dispatch() {
    assert!(
        ALL_BOUNDARY_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-BOUNDARY-004")
    );
    assert!(
        ALL_BOUNDARY_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-DEPENDENCY-004")
    );
    assert!(
        DEFAULT_FAIL_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-BOUNDARY-004")
    );
    assert!(
        DEFAULT_FAIL_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-DEPENDENCY-004")
    );
    assert!(
        BOUNDARY_CATALOG
            .families
            .iter()
            .find(|(name, _)| *name == "boundaries")
            .is_some_and(|(_, rules)| rules
                .iter()
                .any(|id| id.as_str() == RuleId::ScbBoundary004.as_str()))
    );
    assert!(
        BOUNDARY_CATALOG
            .families
            .iter()
            .find(|(name, _)| *name == "dependencies")
            .is_some_and(|(_, rules)| {
                rules
                    .iter()
                    .any(|id| id.as_str() == RuleId::ScbBoundary004.as_str())
                    && rules
                        .iter()
                        .any(|id| id.as_str() == RuleId::ScbDependency004.as_str())
            })
    );

    let fixture = Fixture::new();
    fixture.write_workspace();
    fixture.write_boundary("not-valid");
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let output = Command::new(binary)
        .args([
            "--json",
            "--root",
            fixture.root().to_str().expect("utf-8 root"),
            "lint",
            "sc-boundary",
            "--rule-filter",
            RuleId::ScbBoundary004.as_str(),
        ])
        .output()
        .expect("sc-lint starts");
    assert_eq!(
        output.status.code(),
        Some(10),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(output["data"]["status"], "fail");
    assert_eq!(output["data"]["findings"][0]["rule_id"], "SCB-BOUNDARY-004");
    assert_eq!(output["data"]["findings"][0]["kind"], "invalid_identifier");
}

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        Self(TempDir::new().expect("tempdir"))
    }

    fn root(&self) -> &std::path::Path {
        self.0.path()
    }

    fn write_workspace(&self) {
        fs::create_dir_all(self.root().join("crates/example/src")).expect("crate directory");
        fs::create_dir_all(self.root().join("boundaries/example")).expect("boundary directory");
        fs::write(
            self.root().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
        )
        .expect("workspace manifest");
        fs::write(
            self.root().join("crates/example/Cargo.toml"),
            "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("crate manifest");
        fs::write(
            self.root().join("crates/example/src/lib.rs"),
            "pub struct Example;\n",
        )
        .expect("crate source");
        fs::write(
            self.root().join("boundaries/planning.toml"),
            "[planning]\ncurrent_sprint = \"K.3\"\n",
        )
        .expect("planning inventory");
    }

    fn write_boundary(&self, facade: &str) {
        fs::write(
            self.root().join("boundaries/example/example.toml"),
            format!(
                "boundary_id = \"BOUNDARY-Example\"\nowner_package = \"example\"\nowner_crate_path = \"example\"\nname = \"Example\"\n\n[public]\nfacade = \"{facade}\"\n\n[implementation]\ntype = \"Example\"\nmodule = \"example\"\nvisibility = \"public\"\nconstructor = \"none\"\n\n[composition]\nroots = [\"Example\"]\n\n[dependencies]\nallowed_dependents = []\nallowed_dependencies = []\nforbidden_edges = []\n\n[references]\nscope = \"outside_owner_crate\"\nforbidden = []\n\n[testing]\nallowed_test_double_paths = []\nforbidden_test_bypasses = []\n\n[enforcement]\nlint_rules = []\nreview_gates = []\n\n[status]\nstate = \"concrete_landed\"\n"
            ),
        )
        .expect("boundary inventory");
    }
}
