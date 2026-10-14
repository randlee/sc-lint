use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(120);

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

fn run_bounded(command: &mut Command) -> Output {
    run_bounded_for(command, SUBPROCESS_TIMEOUT)
}

fn run_bounded_for(command: &mut Command, timeout: Duration) -> Output {
    let label = format!(
        "{:?} {:?}",
        command.get_program(),
        command.get_args().collect::<Vec<_>>()
    );
    let stdout = tempfile::tempfile().expect("create subprocess stdout capture");
    let stderr = tempfile::tempfile().expect("create subprocess stderr capture");
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
            Ok(None) if started.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let status = child.wait().expect("reap timed out subprocess");
                let stdout = captured(stdout, "timed out subprocess stdout");
                let stderr = captured(stderr, "timed out subprocess stderr");
                panic!(
                    "subprocess exceeded {timeout:?}: {label}; status after kill: {status}\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&stdout),
                    String::from_utf8_lossy(&stderr)
                );
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = captured(stdout, "failed subprocess stdout");
                let stderr = captured(stderr, "failed subprocess stderr");
                panic!(
                    "failed waiting for subprocess {label}: {error}\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&stdout),
                    String::from_utf8_lossy(&stderr)
                );
            }
        }
    };

    Output {
        status,
        stdout: captured(stdout, "subprocess stdout"),
        stderr: captured(stderr, "subprocess stderr"),
    }
}

fn captured(mut file: std::fs::File, description: &str) -> Vec<u8> {
    let mut contents = Vec::new();
    file.seek(SeekFrom::Start(0))
        .unwrap_or_else(|error| panic!("rewind {description}: {error}"));
    file.read_to_end(&mut contents)
        .unwrap_or_else(|error| panic!("read {description}: {error}"));
    contents
}

fn output_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_success(binary: &Path, args: &[&str], cwd: &Path) -> String {
    let output = run_bounded(Command::new(binary).args(args).current_dir(cwd));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{binary:?} {args:?} failed with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );
    stdout
}

fn host_tool(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

fn supported_cargo(cargo: &Path) -> bool {
    let output = run_bounded(Command::new(cargo).arg("--version"));
    if !output.status.success() {
        eprintln!(
            "skipping docs examples: cargo --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return false;
    }
    let version = output_text(&output);
    let version = version.split_whitespace().nth(1).and_then(|version| {
        version
            .split('.')
            .take(2)
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    });
    match version.as_deref() {
        Some([major, minor]) if *major > 1 || (*major == 1 && *minor >= 85) => true,
        _ => {
            eprintln!(
                "skipping docs examples: Rust 1.85+ is required for the edition-2024 fixture; cargo reported {version:?}"
            );
            false
        }
    }
}

#[test]
fn bounded_runner_timeout_child() {
    if std::env::var_os("SC_LINT_TIMEOUT_TEST_CHILD").is_some() {
        println!("intentional timeout stdout");
        eprintln!("intentional timeout stderr");
        thread::sleep(Duration::from_secs(30));
    }
}

#[test]
fn bounded_runner_kills_and_reports_a_hung_child() {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args(["--exact", "bounded_runner_timeout_child", "--nocapture"])
        .env("SC_LINT_TIMEOUT_TEST_CHILD", "1");
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_bounded_for(&mut command, Duration::from_secs(2));
    }))
    .expect_err("hung subprocess should hit its deadline");
    let panic = match panic.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload
            .downcast::<&str>()
            .map(|message| (*message).to_owned())
            .expect("timeout panic has a string payload"),
    };
    assert!(panic.contains("bounded_runner_timeout_child"), "{panic}");
    assert!(panic.contains("intentional timeout stdout"), "{panic}");
    assert!(panic.contains("intentional timeout stderr"), "{panic}");
}

#[test]
fn docs_examples_parse_and_run_with_the_staged_branch_binary() {
    let cargo = match host_tool(if cfg!(windows) {
        &["cargo.exe"]
    } else {
        &["cargo"]
    }) {
        Some(cargo) => cargo,
        None => {
            eprintln!("skipping docs examples: cargo is not available on PATH");
            return;
        }
    };
    let rustup = match host_tool(if cfg!(windows) {
        &["rustup.exe"]
    } else {
        &["rustup"]
    }) {
        Some(rustup) => rustup,
        None => {
            eprintln!("skipping docs examples: rustup is not available on PATH");
            return;
        }
    };
    let python = match host_tool(if cfg!(windows) {
        &["python.exe", "python3.exe"]
    } else {
        &["python3", "python"]
    }) {
        Some(python) => python,
        None => {
            eprintln!("skipping docs examples: Python is not available on PATH");
            return;
        }
    };
    if !supported_cargo(&cargo) {
        return;
    }
    let rustup_version = run_bounded(Command::new(&rustup).arg("--version"));
    if !rustup_version.status.success() {
        eprintln!(
            "skipping docs examples: rustup --version failed: {}",
            String::from_utf8_lossy(&rustup_version.stderr)
        );
        return;
    }

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
        let mut command = Command::new(&staged_binary);
        command
            .args(&args)
            .current_dir(fixture_root)
            .env("CARGO_TARGET_DIR", fixture_root.join("cargo-target"))
            .env("CARGO_HOME", fixture_root.join("cargo-home"))
            .env("CARGO_NET_OFFLINE", "true");
        let output = run_bounded(&mut command);
        assert!(
            output.status.success(),
            "lint config ordering failed: {args:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // The JSON snippet is consumed by the managed Python helper in inspect-
    // only mode so the test neither installs tools nor accesses real cargo state.
    let cargo_home = fixture_root.join("cargo-home");
    let package_path = root.join("bindings/sc-lint-py/python");
    let python_path = std::env::join_paths([package_path]).expect("compose Python import path");
    let output = run_bounded(
        Command::new(&python)
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
            .env("CARGO_NET_OFFLINE", "true"),
    );
    assert!(
        output.status.success(),
        "setup_tools rejected documented install.json example\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
