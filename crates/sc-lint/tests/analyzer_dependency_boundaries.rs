use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

const ANALYZERS: [&str; 3] = ["sc-lint-boundary", "sc-lint-portability", "sc-lint-runtime"];
const ANALYZER_SUPPORT: &str = "sc-lint-analyzer-support";
const CLI: &str = "sc-lint";

#[test]
fn analyzer_manifests_remain_dependency_independent() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(&workspace)
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("parse cargo metadata");

    for analyzer in ANALYZERS {
        let dependencies = direct_dependencies(&metadata, analyzer);
        for peer in ANALYZERS {
            if analyzer != peer {
                assert!(
                    !dependencies.contains(peer),
                    "{analyzer} must not depend on analyzer crate {peer}"
                );
            }
        }
        assert!(
            !dependencies.contains(CLI),
            "{analyzer} must not depend on the {CLI} CLI crate"
        );
    }

    let support_dependencies = direct_dependencies(&metadata, ANALYZER_SUPPORT);
    assert!(
        !support_dependencies.contains(CLI),
        "{ANALYZER_SUPPORT} must not depend on the {CLI} CLI crate"
    );
}

fn direct_dependencies(metadata: &Value, package_name: &str) -> BTreeSet<String> {
    metadata["packages"]
        .as_array()
        .expect("cargo metadata packages")
        .iter()
        .find(|package| package["name"].as_str() == Some(package_name))
        .unwrap_or_else(|| panic!("package {package_name} is present"))["dependencies"]
        .as_array()
        .expect("cargo metadata dependencies")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .map(ToOwned::to_owned)
        .collect()
}
