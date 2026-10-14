use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fenced_block_after<'a>(markdown: &'a str, heading: &str, language: &str) -> &'a str {
    let section = markdown
        .split_once(heading)
        .map(|(_, section)| section)
        .unwrap_or_else(|| panic!("missing documentation heading `{heading}`"));
    let fence = format!("```{language}");
    let block = section
        .split_once(&fence)
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("missing `{language}` example after `{heading}`"));
    block
        .split_once("\n```")
        .map(|(contents, _)| contents.trim_start_matches('\n'))
        .unwrap_or_else(|| panic!("unterminated `{language}` example after `{heading}`"))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create staged directory");
    for entry in fs::read_dir(source).expect("read documentation bundle") {
        let entry = entry.expect("documentation bundle entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy documentation file");
        }
    }
}

fn assert_success(binary: &Path, args: &[&str], cwd: &Path) -> String {
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {binary:?} {args:?}: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{binary:?} {args:?} failed with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );
    stdout
}

#[test]
fn docs_examples_parse_and_run_with_the_staged_branch_binary() {
    let root = workspace_root();
    let guide = fs::read_to_string(root.join("docs-bundle/using-sc-lint.md"))
        .expect("read using-sc-lint guide");
    let lint_example = fenced_block_after(
        &guide,
        "## Profiles, platform scope, and declared tools",
        "toml",
    );
    let install_example = fenced_block_after(&guide, "The persisted JSON", "json");

    let parsed_lint: toml::Value = toml::from_str(lint_example).expect("documented TOML parses");
    let parsed_tools: Value =
        serde_json::from_str(install_example).expect("documented JSON parses");
    assert_eq!(
        parsed_lint["tool"]["sc-lint"]["lint_profiles"]["ci"][2],
        "windows-check".into()
    );
    assert_eq!(parsed_tools["tools"]["cargo"]["cargo-xwin"], "0.18.4");
    assert_eq!(
        parsed_tools["tools"]["rustup_targets"][0],
        "x86_64-pc-windows-msvc"
    );

    let fixture = TempDir::new().expect("consumer fixture");
    let fixture_root = fixture.path();
    fs::write(
        fixture_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"demo\"]\nresolver = \"2\"\n",
    )
    .expect("write fixture workspace");
    fs::create_dir_all(fixture_root.join("demo/src")).expect("create fixture crate");
    fs::write(
        fixture_root.join("demo/Cargo.toml"),
        "[package]\nname = \"docs-example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write fixture package");
    fs::write(
        fixture_root.join("demo/src/lib.rs"),
        "pub fn example() {}\n",
    )
    .expect("write fixture source");

    let config = format!(
        "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n\n{lint_example}\n\n[[tool.sc-lint.test]]\nname = \"smoke\"\ncommand = [\"cargo\", \"check\", \"--workspace\"]\n"
    );
    fs::write(fixture_root.join("sc-lint.toml"), &config).expect("write documented config");
    let installer_input = fixture_root.join(".sc-lint/install.json");
    fs::create_dir_all(installer_input.parent().expect("installer input parent"))
        .expect("create installer input directory");
    fs::write(&installer_input, install_example).expect("write documented installer input");

    let stage = TempDir::new().expect("staged product");
    let bin_dir = stage.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("create staged bin directory");
    let staged_binary = bin_dir.join(if cfg!(windows) {
        "sc-lint.exe"
    } else {
        "sc-lint"
    });
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &staged_binary).expect("stage branch binary");
    copy_tree(&root.join("docs-bundle"), &bin_dir.join("docs-bundle"));

    for (args, expected) in [
        (&["docs"][..], "sc-lint"),
        (&["docs", "--path"][..], "docs-bundle"),
        (&["docs", "configuration"][..], "configuration"),
        (&["docs", "setup"][..], "installation"),
        (
            &["docs", "troubleshooting"][..],
            "setup_tools.inspection_unavailable",
        ),
    ] {
        let output = assert_success(&staged_binary, args, fixture_root);
        assert!(
            output.to_lowercase().contains(expected),
            "{args:?} output did not include `{expected}`:\n{output}"
        );
    }

    // Both placements shown in just-setup.md are accepted around the global
    // --config option. The fixture's native Cargo steps run; its Windows step
    // is selected only on Windows by the documented host scope.
    for args in [
        vec![
            "--config",
            "sc-lint.toml",
            "lint",
            "ci",
            "--consumer",
            "--platform-scope",
            "host",
        ],
        vec![
            "lint",
            "ci",
            "--consumer",
            "--platform-scope",
            "host",
            "--config",
            "sc-lint.toml",
        ],
    ] {
        let output = Command::new(&staged_binary)
            .args(&args)
            .current_dir(fixture_root)
            .env("CARGO_TARGET_DIR", fixture_root.join("cargo-target"))
            .output()
            .expect("run documented lint profile");
        assert!(
            output.status.success(),
            "lint config ordering failed: {args:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // The JSON snippet is consumed by the managed Python helper in inspect-
    // only mode so the test neither installs tools nor accesses real cargo state.
    let python = if cfg!(windows) { "python" } else { "python3" };
    let cargo_home = fixture_root.join("cargo-home");
    let package_path = root.join("bindings/sc-lint-py/python");
    let python_path = std::env::join_paths([package_path]).expect("compose Python import path");
    let output = Command::new(python)
        .args([
            "-m",
            "sc_lint.setup_tools",
            "--config",
            "sc-lint.toml",
            "--check",
        ])
        .current_dir(fixture_root)
        .env("PYTHONPATH", python_path)
        .env("CARGO_HOME", cargo_home)
        .output()
        .expect("run setup_tools against documented JSON");
    assert!(
        output.status.success(),
        "setup_tools rejected documented install.json example\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
