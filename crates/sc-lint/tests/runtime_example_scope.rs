use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
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
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap(), value)
}

const WAIT: &str = "use std::sync::{Condvar, Mutex};\npub fn waits(c: &Condvar, m: &Mutex<bool>) { let g = m.lock().unwrap(); let _ = c.wait(g); }\n";

#[test]
fn examples_are_discovered_with_target_ownership_and_existing_scopes() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\ntokio = { path = \"vendor/tokio\" }\n[workspace]\nexclude = [\"vendor/tokio\"]\n[[example]]\nname = \"custom\"\npath = \"custom/demo.rs\"\n",
    );
    write(root, "sc-lint.toml", "[tool.sc-lint]\n");
    write(
        root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname = \"tokio\"\nversion = \"1.0.0\"\nedition = \"2021\"\n",
    );
    write(root, "vendor/tokio/src/lib.rs", "");
    write(
        root,
        "src/lib.rs",
        &format!(
            "{WAIT}\nfn sleeps() {{ std::thread::sleep(std::time::Duration::ZERO); std::thread::sleep_until(std::time::Instant::now()); }}"
        ),
    );
    write(
        root,
        "examples/conventional/main.rs",
        &format!("{WAIT}\nmod child;\n#[path = \"custom_child.rs\"] mod renamed;\n"),
    );
    write(
        root,
        "examples/conventional/child.rs",
        &format!(
            "{WAIT}\nfn sleeps() {{ std::thread::sleep(std::time::Duration::ZERO); std::thread::sleep_until(std::time::Instant::now()); }}"
        ),
    );
    write(root, "examples/conventional/custom_child.rs", WAIT);
    write(root, "custom/demo.rs", WAIT);
    write(root, "tests/test_only.rs", WAIT);
    write(root, "benches/bench_only.rs", WAIT);
    let (status, json) = run(root);
    assert_eq!(status, 10, "{json}");
    let findings = json["data"]["findings"].as_array().unwrap();
    let waits = findings
        .iter()
        .filter(|f| f["rule_id"] == "SCB-RUNTIME-001")
        .collect::<Vec<_>>();
    assert_eq!(waits.len(), 5, "{json}");
    for (path, target) in [
        ("src/lib.rs", "sample"),
        ("examples/conventional/main.rs", "conventional"),
        ("examples/conventional/child.rs", "conventional"),
        ("examples/conventional/custom_child.rs", "conventional"),
        ("custom/demo.rs", "custom"),
    ] {
        let item = waits
            .iter()
            .find(|f| {
                f["message"]
                    .as_str()
                    .unwrap()
                    .replace('\\', "/")
                    .contains(path)
            })
            .unwrap_or_else(|| panic!("missing {path}: {json}"));
        assert!(item["owner_ids"].to_string().contains(target), "{item}");
    }
    for id in ["SCB-RUNTIME-003", "SCB-RUNTIME-004"] {
        let matching = findings
            .iter()
            .filter(|f| f["rule_id"] == id)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "{json}");
        assert!(
            matching[0]["message"]
                .as_str()
                .unwrap()
                .replace('\\', "/")
                .contains("src/lib.rs")
        );
    }
}

#[test]
fn example_only_package_is_analyzed_and_clean_example_passes() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"example-only\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n",
    );
    write(root, "sc-lint.toml", "[tool.sc-lint]\n");
    write(root, "examples/demo.rs", "fn main() {}\n");
    let (status, json) = run(root);
    assert_eq!(status, 0, "{json}");
    write(root, "examples/demo.rs", WAIT);
    let (status, json) = run(root);
    assert_eq!(status, 10, "{json}");
    assert_eq!(json["data"]["findings"][0]["rule_id"], "SCB-RUNTIME-001");
}

#[test]
fn shared_example_source_uses_lexicographically_first_target() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(root, "sc-lint.toml", "[tool.sc-lint]\n");
    write(root, "src/lib.rs", WAIT);
    write(root, "shared.rs", WAIT);
    let head = "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\nautoexamples=false\n[workspace]\n";
    let a = "[[example]]\nname='alpha'\npath='shared.rs'\n";
    let z = "[[example]]\nname='zeta'\npath='shared.rs'\n";
    let mut previous = None;
    for manifest in [format!("{head}{a}{z}"), format!("{head}{z}{a}")] {
        write(root, "Cargo.toml", &manifest);
        let (status, json) = run(root);
        assert_eq!(status, 10, "{json}");
        let findings = json["data"]["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 2, "{json}");
        let shared = findings
            .iter()
            .find(|f| f["message"].as_str().unwrap().contains("shared.rs"))
            .unwrap();
        assert!(
            shared["owner_ids"].to_string().contains("alpha"),
            "{shared}"
        );
        let owners = findings
            .iter()
            .map(|f| f["owner_ids"].clone())
            .collect::<Vec<_>>();
        if let Some(ref before) = previous {
            assert_eq!(before, &owners);
        } else {
            previous = Some(owners);
        }
    }
}

#[test]
fn all_applicable_rules_run_on_example_sources() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n[workspace]\n",
    );
    write(root, "sc-lint.toml", "[tool.sc-lint]\n");
    write(root, "src/lib.rs", "pub fn clean() {}\n");
    write(
        root,
        "examples/rules.rs",
        r#"
use std::sync::{Condvar, Mutex};
use std::time::Duration;
fn discarded(c: &Condvar, m: &Mutex<bool>) { let g = m.lock().unwrap(); c.wait_timeout(g, Duration::ZERO).unwrap(); }
async fn timer() { tokio::time::sleep(()).await; }
fn blocking(rt: &tokio::runtime::Runtime) { rt.block_on(async {}); }
async fn guard(m: &Mutex<bool>) { let g = m.lock().unwrap(); std::future::pending::<()>().await; drop(g); }
"#,
    );
    let (status, json) = run(root);
    assert_eq!(status, 10, "{json}");
    let findings = json["data"]["findings"].as_array().unwrap();
    for id in [
        "SCB-RUNTIME-002",
        "SCB-RUNTIME-005",
        "SCB-RUNTIME-006",
        "SCB-RUNTIME-007",
    ] {
        assert!(
            findings.iter().any(|f| f["rule_id"] == id),
            "missing {id}: {json}"
        );
    }
}

#[test]
fn shared_library_example_root_still_discovers_external_modules() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(root, "sc-lint.toml", "[tool.sc-lint]\n");
    write(
        root,
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\nautoexamples=false\n[workspace]\n[[example]]\nname='shared'\npath='src/lib.rs'\n",
    );
    write(
        root,
        "src/lib.rs",
        &format!("{WAIT}\n#[path = \"../outside.rs\"] mod outside;\n"),
    );
    write(root, "outside.rs", WAIT);
    let (status, json) = run(root);
    assert_eq!(status, 10, "{json}");
    let findings = json["data"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{json}");
    assert!(
        findings.iter().any(
            |finding| finding["message"].as_str().unwrap().contains("outside.rs")
                && finding["owner_ids"].to_string().contains("shared")
        ),
        "{json}"
    );
    assert!(
        findings.iter().any(|finding| finding["message"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .contains("src/lib.rs")
            && finding["owner_ids"].to_string().contains("sample")),
        "{json}"
    );
}
