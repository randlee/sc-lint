use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::process::Output;

use tempfile::TempDir;

#[cfg(unix)]
const BOOTSTRAP: &str = include_str!("../assets/bootstrap");
#[cfg(windows)]
const BOOTSTRAP_POWERSHELL: &str = include_str!("../assets/bootstrap.ps1");

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents).expect("write executable fixture");
    let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fixture executable");
}

#[cfg(unix)]
fn consumer_fixture(parent: &Path, name: &str, atm_core_derived: bool) -> PathBuf {
    let root = parent.join(name);
    fs::create_dir_all(root.join(".sc-lint")).expect("consumer bootstrap directory");
    let bootstrap = root.join(".sc-lint/bootstrap");
    write_executable(&bootstrap, BOOTSTRAP);
    if atm_core_derived {
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"consumer-derived-from-atm-core\"\nversion = \"0.1.0\"\n",
        )
        .expect("derived consumer manifest");
    }
    root
}

#[cfg(unix)]
fn capture_script() -> &'static str {
    "#!/bin/sh\n{\nprintf 'cwd=%s\\n' \"$PWD\"\nprintf 'bin=%s\\n' \"${SC_LINT_BIN:-}\"\nprintf 'custom=%s\\n' \"${SC_LINT_TEST_VALUE:-}\"\nprintf 'path=%s\\n' \"$PATH\"\nfor arg in \"$@\"; do printf 'arg=%s\\n' \"$arg\"; done\n} > \"$SC_LINT_EXEC_LOG\"\nexit \"${SC_LINT_EXEC_STATUS:-0}\"\n"
}

#[cfg(unix)]
fn run_bootstrap(root: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new("/bin/sh");
    command
        .arg(root.join(".sc-lint/bootstrap"))
        .args(arguments)
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin");
    for (name, value) in environment {
        command.env(name, value);
    }
    command.output().expect("run consumer bootstrap")
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_does_not_reenter_consumer_profile() {
    let fixture = TempDir::new().expect("fixture root");
    let log = fixture.path().join("managed.log");
    let managed = fixture.path().join("managed");
    let decoy = fixture.path().join("decoy");
    fs::create_dir_all(&managed).expect("managed directory");
    fs::create_dir_all(&decoy).expect("decoy directory");
    let managed_binary = managed.join("sc-lint");
    let decoy_binary = decoy.join("sc-lint");
    write_executable(&managed_binary, capture_script());
    write_executable(
        &decoy_binary,
        "#!/bin/sh\nprintf 'decoy\\n' > \"$SC_LINT_EXEC_LOG\"\nexit 99\n",
    );

    for (name, derived) in [
        ("neutral-consumer", false),
        ("atm-core-derived-consumer", true),
    ] {
        let root = consumer_fixture(fixture.path(), name, derived);
        let config = root.join("sc-lint.toml");
        fs::write(&config, "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n")
            .expect("consumer config");
        let config = config.to_str().expect("config path");
        let managed_binary_path = managed_binary.to_str().expect("managed binary path");
        let search_path = format!("{}:/usr/bin:/bin", decoy.display());
        let log_path = log.to_str().expect("log path");
        let canonical_root = root.canonicalize().expect("canonical consumer path");
        let root_path = canonical_root.to_str().expect("consumer path");
        let output = run_bootstrap(
            &root,
            &[
                "exec",
                "--config",
                config,
                "--",
                "sc-lint",
                "lint",
                "sc-boundary",
                "--rule-filter",
                "SCB-TEST",
            ],
            &[
                ("SC_LINT_BIN", managed_binary_path),
                ("SC_LINT_EXEC_LOG", log_path),
                ("SC_LINT_EXEC_STATUS", "23"),
                ("SC_LINT_TEST_VALUE", "kept with spaces"),
                ("PATH", &search_path),
            ],
        );

        assert_eq!(
            output.status.code(),
            Some(23),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let captured = fs::read_to_string(&log).expect("managed process log");
        assert!(
            captured.contains(&format!("cwd={root_path}\n")),
            "{captured}"
        );
        assert!(
            captured.contains(&format!("bin={}\n", managed_binary.display())),
            "{captured}"
        );
        assert!(captured.contains("custom=kept with spaces\n"), "{captured}");
        assert!(
            captured.contains(&format!("path={}:{}\n", managed.display(), search_path)),
            "{captured}"
        );
        assert!(captured.contains(&format!("arg=--config\narg={config}\narg=lint\narg=sc-boundary\narg=--rule-filter\narg=SCB-TEST\n")), "{captured}");
        assert!(
            !captured.contains("arg=sc-lint\n"),
            "the product must replace argv[0]: {captured}"
        );
        assert!(
            !captured.contains("decoy\\n"),
            "the PATH decoy must not run: {captured}"
        );
        assert!(root.join("Cargo.toml").exists() == derived);
        let _ = fs::remove_file(&log);
    }
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_uses_managed_product() {
    let fixture = TempDir::new().expect("fixture root");
    let data_home = fixture.path().join("user-data");
    let managed = data_home.join("sc-lint/bin");
    fs::create_dir_all(&managed).expect("managed directory");
    write_executable(&managed.join("sc-lint"), capture_script());
    let log = fixture.path().join("managed.log");
    let data_home = data_home.to_str().expect("data home");
    let log_path = log.to_str().expect("log path");

    for (name, derived) in [
        ("neutral-consumer", false),
        ("atm-core-derived-consumer", true),
    ] {
        let root = consumer_fixture(fixture.path(), name, derived);
        let output = run_bootstrap(
            &root,
            &["exec", "--", "sc-lint", "version"],
            &[
                ("XDG_DATA_HOME", data_home),
                ("SC_LINT_EXEC_LOG", log_path),
                ("PATH", "/usr/bin:/bin"),
            ],
        );

        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let captured = fs::read_to_string(&log).expect("managed process log");
        assert!(
            captured.contains(&format!("bin={}\n", managed.join("sc-lint").display())),
            "{captured}"
        );
        assert!(captured.contains("arg=version\n"), "{captured}");
        assert!(
            !captured.contains("arg=--config\n"),
            "omitted config must preserve discovery: {captured}"
        );
        assert_eq!(root.join("Cargo.toml").exists(), derived);
        let _ = fs::remove_file(&log);
    }
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_uses_managed_product() {
    let fixture = TempDir::new().expect("fixture root");
    let root = fixture.path().join("simple-consumer");
    fs::create_dir_all(root.join(".sc-lint")).expect("consumer bootstrap directory");
    fs::write(root.join(".sc-lint/bootstrap.ps1"), BOOTSTRAP_POWERSHELL)
        .expect("consumer PowerShell bootstrap");
    let local_app_data = fixture.path().join("AppData/Local");
    let managed = local_app_data.join("sc-lint/bin");
    fs::create_dir_all(&managed).expect("managed directory");
    let managed_binary = managed.join("sc-lint.exe");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &managed_binary).expect("managed product copy");

    let output = Command::new("pwsh")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", "sc-lint", "version"])
        .current_dir(&root)
        .env("LOCALAPPDATA", &local_app_data)
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("sc-lint"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(managed_binary.is_file());
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_preserves_argv_and_exit_status() {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    write_executable(&managed.join("sc-lint"), "#!/bin/sh\nexit 0\n");
    let child = fixture.path().join("consumer-command");
    let log = fixture.path().join("child.log");
    write_executable(&child, capture_script());
    let child_path = child.to_str().expect("child path");
    let managed_path = managed.to_str().expect("managed path");
    let log_path = log.to_str().expect("log path");
    let canonical_root = root.canonicalize().expect("canonical consumer path");
    let root_path = canonical_root.to_str().expect("consumer path");

    let output = run_bootstrap(
        &root,
        &["exec", "--", child_path, "two words", "tail"],
        &[
            ("SC_LINT_INSTALL_DIR", managed_path),
            ("SC_LINT_EXEC_LOG", log_path),
            ("SC_LINT_EXEC_STATUS", "19"),
            ("SC_LINT_TEST_VALUE", "preserved"),
            ("PATH", "/usr/bin:/bin"),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(19),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = fs::read_to_string(log).expect("child process log");
    assert!(
        captured.contains(&format!("cwd={root_path}\n")),
        "{captured}"
    );
    assert!(
        captured.contains(&format!("bin={}\n", managed.join("sc-lint").display())),
        "{captured}"
    );
    assert!(captured.contains("custom=preserved\n"), "{captured}");
    assert!(
        captured.contains(&format!("path={}:{}\n", managed.display(), "/usr/bin:/bin")),
        "{captured}"
    );
    assert!(captured.contains("arg=two words\narg=tail\n"), "{captured}");
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_supports_non_sc_lint_argv0() {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    write_executable(&managed.join("sc-lint"), "#!/bin/sh\nexit 0\n");
    let child = fixture.path().join("consumer-command");
    let log = fixture.path().join("child.log");
    write_executable(&child, capture_script());
    let output = run_bootstrap(
        &root,
        &[
            "exec",
            "--",
            child.to_str().expect("child path"),
            "arg one",
            "arg two",
        ],
        &[
            (
                "SC_LINT_INSTALL_DIR",
                managed.to_str().expect("managed path"),
            ),
            ("SC_LINT_EXEC_LOG", log.to_str().expect("log path")),
            ("PATH", "/usr/bin:/bin"),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = fs::read_to_string(log).expect("child process log");
    assert!(
        captured.contains("arg=arg one\narg=arg two\n"),
        "{captured}"
    );
    assert!(
        captured.contains(&format!("bin={}\n", managed.join("sc-lint").display())),
        "{captured}"
    );
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_reports_missing_managed_product() {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer_fixture(fixture.path(), "simple-consumer", false);
    let decoy = fixture.path().join("decoy");
    fs::create_dir_all(&decoy).expect("decoy directory");
    let log = fixture.path().join("decoy.log");
    write_executable(
        &decoy.join("sc-lint"),
        "#!/bin/sh\nprintf 'decoy\\n' > \"$SC_LINT_EXEC_LOG\"\nexit 0\n",
    );
    let missing_dir = fixture.path().join("missing");
    let output = run_bootstrap(
        &root,
        &["exec", "--", "sc-lint", "version"],
        &[
            (
                "SC_LINT_INSTALL_DIR",
                missing_dir.to_str().expect("missing path"),
            ),
            ("SC_LINT_EXEC_LOG", log.to_str().expect("log path")),
            ("PATH", &format!("{}:/usr/bin:/bin", decoy.display())),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "sc-lint binary is unavailable at {}/sc-lint; run just setup\n",
            missing_dir.display()
        )
    );
    assert!(
        !log.exists(),
        "a PATH decoy must not substitute for the managed binary"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Traceback"));
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_preserves_windows_child_argv_cwd_environment_and_status() {
    let fixture = TempDir::new().expect("fixture root");
    let root = fixture.path().join("simple-consumer");
    fs::create_dir_all(root.join(".sc-lint")).expect("consumer bootstrap directory");
    fs::write(root.join(".sc-lint/bootstrap.ps1"), BOOTSTRAP_POWERSHELL)
        .expect("consumer PowerShell bootstrap");
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    let managed_binary = managed.join("sc-lint.exe");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &managed_binary).expect("managed product copy");
    let child = fixture.path().join("consumer-command.cmd");
    let log = fixture.path().join("child.log");
    fs::write(
        &child,
        "@echo off\r\n> \"%SC_LINT_EXEC_LOG%\" echo cwd=%CD%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo bin=%SC_LINT_BIN%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo custom=%SC_LINT_TEST_VALUE%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo path=%PATH%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo args=%*\r\nexit /b 23\r\n",
    )
    .expect("consumer command fixture");
    let root_path = root.to_str().expect("consumer path");
    let log_path = log.to_str().expect("log path");
    let managed_path = managed.to_str().expect("managed path");
    let child_path = child.to_str().expect("child path");
    let output = Command::new("pwsh")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", child_path, "two words", "tail"])
        .current_dir(root_path)
        .env("SC_LINT_INSTALL_DIR", managed_path)
        .env("SC_LINT_EXEC_LOG", log_path)
        .env("SC_LINT_TEST_VALUE", "preserved")
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert_eq!(
        output.status.code(),
        Some(23),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = fs::read_to_string(log).expect("child process log");
    assert!(captured.contains(&format!("cwd={root_path}")), "{captured}");
    assert!(
        captured.contains(&format!("bin={}", managed_binary.display())),
        "{captured}"
    );
    assert!(captured.contains("custom=preserved"), "{captured}");
    assert!(
        captured.contains(&format!("path={};", managed.display())),
        "{captured}"
    );
    assert!(captured.contains("args=two words tail"), "{captured}");
}
