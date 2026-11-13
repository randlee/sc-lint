use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn write(root: &Path, path: &str, contents: &str) {
    let path = root.join(path);
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
            "sc-runtime",
        ])
        .output()
        .unwrap();
    let bytes = if output.status.code() == Some(1) {
        &output.stderr
    } else {
        &output.stdout
    };
    (
        output.status.code().unwrap(),
        serde_json::from_slice(bytes).unwrap(),
    )
}
#[test]
fn opted_in_examples_emit_panicking_call_findings() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n[dependencies]\ntokio={path='vendor/tokio'}\n[workspace]\nexclude=['vendor/tokio']\n",
    );
    write(
        root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname='tokio'\nversion='1.0.0'\nedition='2021'\n",
    );
    write(root, "vendor/tokio/src/lib.rs", "");
    write(root, "src/lib.rs", "pub fn clean() {}\n");
    write(
        root,
        "examples/demo.rs",
        "fn main() { Some(1).unwrap(); Some(1).expect(\"reason\"); panic!(); unreachable!(); todo!(); unimplemented!(); std::thread::sleep(std::time::Duration::ZERO); std::thread::sleep_until(std::time::Instant::now()); }\n",
    );
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\npanic_rule_packages=['sample']\n",
    );
    let (status, json) = run(root);
    assert_eq!(status, 10, "{json}");
    let findings = json["data"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 6, "{json}");
    assert!(
        findings
            .iter()
            .all(|finding| finding["rule_id"] == "SCB-RUNTIME-008")
    );
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .contains("examples/demo.rs")
    );
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\npanic_rule_packages=[]\n",
    );
    let (status, json) = run(root);
    assert_eq!(status, 0, "{json}");
}
