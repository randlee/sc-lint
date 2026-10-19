use serde_json::Value;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use sc_lint_boundary::{ALL_BOUNDARY_IDS, BOUNDARY_CATALOG, DEFAULT_FAIL_IDS};

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn issue136_manifest_policy_rules_are_registered_in_fail_catalog() {
    let expected = [
        "SCB-MANIFEST-003",
        "SCB-MANIFEST-004",
        "SCB-MANIFEST-005",
        "SCB-MANIFEST-006",
    ];
    for rule in expected {
        assert!(ALL_BOUNDARY_IDS.iter().any(|id| id.as_str() == rule));
        assert!(DEFAULT_FAIL_IDS.iter().any(|id| id.as_str() == rule));
        assert!(
            BOUNDARY_CATALOG
                .families
                .iter()
                .find(|(name, _)| *name == "manifests")
                .is_some_and(|(_, rules)| rules.iter().any(|id| id.as_str() == rule))
        );
    }
}

#[test]
fn issue136_product_dispatch() {
    let fixture = Fixture::new();
    fixture.write_workspace();
    fixture.write(
        "sc-lint.toml",
        r#"
            [tool.sc-lint.analyzers.boundary]
            [[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
            dependency = "dep"
            allowed_manifest_paths = []
            allowed_source_roots = []
            manifest_message = "dep is not declared here"
            source_message = "dep is not imported here"

            [[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
            owner_manifest_path = "crates/app/Cargo.toml"
            dependency_package = "dep"
            allowed_sections = []
            message = "dep is forbidden in every section"
        "#,
    );

    let output = run_bounded(Command::new(env!("CARGO_BIN_EXE_sc-lint")).args([
        "--json",
        "--root",
        fixture.root().to_str().expect("utf-8 fixture path"),
        "lint",
        "sc-boundary",
        "--rule-filter",
        "manifests",
    ]));
    assert_eq!(
        output.status.code(),
        Some(10),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(report["data"]["status"], "fail");
    let findings = report["data"]["findings"]
        .as_array()
        .expect("findings array");
    let ids = findings
        .iter()
        .filter_map(|finding| finding["rule_id"].as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"SCB-MANIFEST-004"), "findings: {findings:?}");
    assert!(ids.contains(&"SCB-MANIFEST-005"), "findings: {findings:?}");
}

fn run_bounded(command: &mut Command) -> Output {
    let label = format!(
        "{:?} {:?}",
        command.get_program(),
        command.get_args().collect::<Vec<_>>()
    );
    let stdout = tempfile::tempfile().expect("create stdout capture");
    let stderr = tempfile::tempfile().expect("create stderr capture");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            stdout.try_clone().expect("clone stdout capture"),
        ))
        .stderr(Stdio::from(
            stderr.try_clone().expect("clone stderr capture"),
        ));

    let started = Instant::now();
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {label}: {error}"));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < SUBPROCESS_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let status = child.wait().expect("reap timed out subprocess");
                panic!(
                    "subprocess exceeded {SUBPROCESS_TIMEOUT:?}: {label}; status after kill: {status}; stderr: {}",
                    String::from_utf8_lossy(&captured(stderr))
                );
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "failed waiting for {label}: {error}; stderr: {}",
                    String::from_utf8_lossy(&captured(stderr))
                );
            }
        }
    };

    Output {
        status,
        stdout: captured(stdout),
        stderr: captured(stderr),
    }
}

fn captured(mut file: std::fs::File) -> Vec<u8> {
    let mut contents = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    file.read_to_end(&mut contents).expect("read capture");
    contents
}

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        Self(TempDir::new().expect("tempdir"))
    }

    fn root(&self) -> &std::path::Path {
        self.0.path()
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(path, content).expect("fixture file");
    }

    fn write_workspace(&self) {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/app\", \"crates/dep\"]\nresolver = \"2\"\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"1.85\"\nauthors = [\"test\"]\nlicense = \"MIT\"\nrepository = \"https://example.invalid\"\nhomepage = \"https://example.invalid\"\n[workspace.dependencies]\ndep = { path = \"crates/dep\" }\n",
        );
        self.write("crates/dep/Cargo.toml", &package("dep", ""));
        self.write("crates/dep/src/lib.rs", "pub struct Dep;\n");
        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"K.4\"\n",
        );
        self.write(
            "crates/app/Cargo.toml",
            &package("app", "[dependencies]\ndep.workspace = true\n"),
        );
        self.write(
            "crates/app/src/lib.rs",
            "use dep::Dep;\npub fn make() -> Dep { Dep }\n",
        );
    }
}

fn package(name: &str, tail: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n\n{tail}"
    )
}
