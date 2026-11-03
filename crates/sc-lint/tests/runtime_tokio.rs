use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn sc_lint_runtime_cli_reports_tokio_blocking_sleep_with_full_source_path() {
    let fixture = TempDir::new().expect("fixture directory");
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/example\"]\nexclude = [\"vendor/tokio\"]\nresolver = \"2\"\n",
    );
    write(
        root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname = \"tokio\"\nversion = \"1.0.0\"\nedition = \"2024\"\n",
    );
    write(root, "vendor/tokio/src/lib.rs", "");
    write(
        root,
        "crates/example/Cargo.toml",
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\ntokio = { path = \"../../vendor/tokio\" }\n",
    );
    write(
        root,
        "crates/example/src/lib.rs",
        "pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args([
            "--json",
            "--root",
            root.to_str().expect("UTF-8 fixture path"),
            "lint",
            "sc-runtime",
        ])
        .output()
        .expect("sc-lint CLI runs");

    assert_eq!(
        output.status.code(),
        Some(10),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON envelope");
    assert_eq!(envelope["command"], "lint.sc-runtime");
    assert_eq!(envelope["data"]["status"], "fail");
    let findings = envelope["data"]["findings"]
        .as_array()
        .expect("finding list");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["rule_id"], "SCB-RUNTIME-003");
    let message = findings[0]["message"].as_str().expect("finding message");
    let source_path = message.split(":1: ").next().expect("source path prefix");
    let canonical_root = dunce::canonicalize(root).expect("canonical fixture root");
    let canonical_source = dunce::canonicalize(source_path).expect("canonical finding source");
    let relative = canonical_source
        .strip_prefix(&canonical_root)
        .expect("source under fixture root");
    assert_eq!(
        relative.to_string_lossy().replace('\\', "/"),
        "crates/example/src/lib.rs"
    );
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("fixture parent");
    fs::write(path, contents).expect("fixture file");
}

#[test]
fn runtime_practice_cli_exit_contracts_for_each_call_and_reasoned_allow() {
    let cases = [
        ("SCB-RUNTIME-003", "fn f()", "std::thread::sleep(());"),
        ("SCB-RUNTIME-004", "fn f()", "std::thread::sleep_until(());"),
        (
            "SCB-RUNTIME-005",
            "async fn f()",
            "tokio::time::sleep(()).await;",
        ),
        (
            "SCB-RUNTIME-005",
            "async fn f()",
            "tokio::time::sleep_until(()).await;",
        ),
        (
            "SCB-RUNTIME-006",
            "fn f(rt:&tokio::runtime::Runtime)",
            "rt.block_on(async{});",
        ),
        (
            "SCB-RUNTIME-006",
            "fn f(rt:&tokio::runtime::Handle)",
            "rt.block_on(async{});",
        ),
        (
            "SCB-RUNTIME-006",
            "fn f()",
            "futures::executor::block_on(async{});",
        ),
        (
            "SCB-RUNTIME-007",
            "async fn f(m:&std::sync::Mutex<()>)",
            "let guard=m.lock().unwrap();std::future::pending::<()>().await;",
        ),
    ];
    // Only metadata and source analysis run. The fixture code is never compiled
    // or executed, so these calls cannot sleep, block, or race the assertion.
    for (rule, signature, body) in cases {
        let source = format!("{signature} {{ {body} }}");
        for (mode, text, expected) in [
            ("production", source.clone(), 10),
            ("test", format!("#[cfg(test)] mod unit {{ {source} }}"), 0),
            (
                "allowed",
                format!(
                    "#[sc_lint(allow({rule}), reason=\"Protocol compatibility boundary\")] {source}"
                ),
                0,
            ),
            (
                "missing reason",
                format!("#[sc_lint(allow({rule}))] {source}"),
                1,
            ),
        ] {
            let output = practice_cli(&text);
            let printed = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if expected == 1 {
                assert!(!output.status.success(), "{rule} {mode}: {printed}");
                assert!(
                    printed.contains("DIRECTIVE.SITE_ALLOW.MISSING_REASON"),
                    "{printed}"
                );
            } else {
                assert_eq!(
                    output.status.code(),
                    Some(expected),
                    "{rule} {mode}: {printed}"
                );
                if expected == 10 {
                    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
                    assert_eq!(json["data"]["findings"][0]["rule_id"], rule);
                }
            }
        }
    }
    assert!(
        practice_cli("async fn f(){tokio::task::yield_now().await;}")
            .status
            .success()
    );
}

fn practice_cli(source: &str) -> std::process::Output {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=['crates/example']\nexclude=['vendor/tokio']\nresolver='2'\n",
    );
    write(
        root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname='tokio'\nversion='1.0.0'\nedition='2024'\n",
    );
    write(root, "vendor/tokio/src/lib.rs", "");
    write(
        root,
        "crates/example/Cargo.toml",
        "[package]\nname='example'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ntokio={path='../../vendor/tokio'}\n",
    );
    write(root, "crates/example/src/lib.rs", source);
    Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args([
            "--json",
            "--root",
            root.to_str().unwrap(),
            "lint",
            "sc-runtime",
        ])
        .output()
        .expect("runtime CLI completes")
}
