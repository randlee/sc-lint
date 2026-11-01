//! Shared test fixtures for real release executables.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::Value;
use tempfile::TempDir;

struct ReleaseBinaryFixture {
    _target_dir: TempDir,
    binaries: HashMap<String, PathBuf>,
}

/// Builds the launcher and backends once for this test process.
pub(crate) fn release_binaries() -> &'static HashMap<String, PathBuf> {
    static FIXTURE: OnceLock<ReleaseBinaryFixture> = OnceLock::new();
    &FIXTURE.get_or_init(build_release_binary_fixture).binaries
}

fn build_release_binary_fixture() -> ReleaseBinaryFixture {
    let target_dir = TempDir::new().expect("release binary fixture target directory");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("cargo")
        .arg("build")
        .arg("--target-dir")
        .arg(target_dir.path())
        .arg("--message-format=json-render-diagnostics")
        .args([
            "-p",
            "sc-lint",
            "-p",
            "sc-lint-boundary",
            "-p",
            "sc-lint-portability",
            "-p",
            "sc-lint-runtime",
        ])
        .current_dir(workspace_root)
        .output()
        .expect("build shared release binary fixture");
    assert!(
        output.status.success(),
        "shared release binary build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let binaries = executable_artifacts(&output.stdout);
    for name in [
        crate::consts::SERVICE_NAME,
        crate::consts::TOOL_BOUNDARY,
        crate::consts::TOOL_PORTABILITY,
        crate::consts::TOOL_RUNTIME,
    ] {
        assert!(
            binaries.get(name).is_some_and(|path| path.is_file()),
            "Cargo did not produce `{name}` in the shared release binary fixture"
        );
    }

    ReleaseBinaryFixture {
        _target_dir: target_dir,
        binaries,
    }
}

fn executable_artifacts(output: &[u8]) -> HashMap<String, PathBuf> {
    let mut binaries = HashMap::new();
    for line in String::from_utf8_lossy(output).lines() {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact"
            || !message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        {
            continue;
        }
        if let (Some(name), Some(executable)) = (
            message["target"]["name"].as_str(),
            message["executable"].as_str(),
        ) {
            binaries.insert(name.to_owned(), PathBuf::from(executable));
        }
    }
    binaries
}
