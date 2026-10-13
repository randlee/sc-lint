use std::fs;
#[cfg(any(unix, windows))]
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
    use std::io::Write;
    use std::process::Stdio;

    // The test process must not own a writable descriptor for a file that a
    // bootstrap fixture will immediately execute. Write it in a child, close
    // the pipe, and wait for that child before returning.
    let mut child = Command::new("sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "write-executable"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("executable fixture writer starts");
    let mut stdin = child.stdin.take().expect("executable fixture writer stdin");
    stdin
        .write_all(contents.as_bytes())
        .expect("write executable fixture contents");
    drop(stdin);
    assert!(
        child
            .wait()
            .expect("executable fixture writer waits")
            .success(),
        "executable fixture writer succeeds"
    );
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
            "[package]\nname = \"consumer-derived-from-atm-core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )
        .expect("derived consumer manifest");
        fs::write(
            root.join("sc-lint.toml"),
            "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n\n[[tool.sc-lint.lint]]\nname = \"ci\"\ncommand = [\".sc-lint/bootstrap\", \"exec\", \"--\", \"sc-lint\", \"profile-marker\"]\n",
        )
        .expect("derived consumer profile configuration");
    } else {
        fs::write(
            root.join("sc-lint.toml"),
            "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n",
        )
        .expect("consumer config");
    }
    root
}

#[cfg(windows)]
fn windows_consumer_fixture(parent: &Path, name: &str, atm_core_derived: bool) -> PathBuf {
    let root = parent.join(name);
    fs::create_dir_all(root.join(".sc-lint")).expect("consumer bootstrap directory");
    fs::write(root.join(".sc-lint/bootstrap.ps1"), BOOTSTRAP_POWERSHELL)
        .expect("consumer PowerShell bootstrap");
    if atm_core_derived {
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"consumer-derived-from-atm-core\"\nversion = \"0.1.0\"\n",
        )
        .expect("derived consumer manifest");
    }
    root
}

#[cfg(windows)]
fn write_windows_stale_status_bootstrap(path: &Path) {
    fs::write(
        path,
        format!("$global:LASTEXITCODE = 37\n{BOOTSTRAP_POWERSHELL}"),
    )
    .expect("PowerShell bootstrap with stale status");
}

#[cfg(windows)]
fn windows_log_value<'a>(captured: &'a str, name: &str) -> &'a str {
    captured
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")))
        .unwrap_or_else(|| panic!("missing {name} in child log: {captured}"))
}

#[cfg(windows)]
fn assert_windows_logged_path(captured: &str, name: &str, expected: &Path) {
    let actual = Path::new(windows_log_value(captured, name))
        .canonicalize()
        .expect("canonical logged Windows path");
    let expected = expected
        .canonicalize()
        .expect("canonical expected Windows path");
    assert!(
        actual
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy()),
        "expected {}={}, got {}",
        name,
        expected.display(),
        actual.display()
    );
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

#[cfg(windows)]
fn powershell_command() -> Command {
    let system_root = std::env::var_os("SystemRoot").expect("Windows SystemRoot");
    let system32 = PathBuf::from(&system_root).join("System32");
    let inherited_path = std::env::var_os("PATH").expect("Windows PATH");
    let search_path = std::env::split_paths(&inherited_path).collect::<Vec<_>>();
    let pwsh = search_path
        .iter()
        .map(|directory| directory.join("pwsh.exe"))
        .find(|candidate| candidate.is_file())
        .expect("pwsh.exe on Windows PATH");
    let mut path = search_path;
    path.retain(|directory| directory == pwsh.parent().expect("pwsh parent"));
    path.push(system32.clone());
    let mut command = Command::new(&pwsh);
    command
        .env_clear()
        .env("SystemRoot", &system_root)
        .env(
            "PATH",
            std::env::join_paths(path).expect("valid Windows PATH"),
        )
        .env("ComSpec", system32.join("cmd.exe"));
    for name in ["TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
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
            !captured.contains("decoy\n"),
            "the PATH decoy must not run: {captured}"
        );
        assert!(
            !captured.contains("arg=profile-marker\n"),
            "the consumer profile must not re-enter the bootstrap: {captured}"
        );
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
        let _ = fs::remove_file(&log);
    }
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_uses_managed_product_for_neutral_and_derived_consumers() {
    let fixture = TempDir::new().expect("fixture root");
    let local_app_data = fixture.path().join("AppData/Local");
    let managed = local_app_data.join("sc-lint/bin");
    fs::create_dir_all(&managed).expect("managed directory");
    let managed_binary = managed.join("sc-lint.exe");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &managed_binary).expect("managed product copy");

    for (name, derived) in [
        ("neutral-consumer", false),
        ("atm-core-derived-consumer", true),
    ] {
        let root = windows_consumer_fixture(fixture.path(), name, derived);
        let output = powershell_command()
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
            .arg(root.join(".sc-lint/bootstrap.ps1"))
            .args(["exec", "--", "sc-lint", "version"])
            .current_dir(&root)
            .env_remove("SC_LINT_BIN")
            .env_remove("SC_LINT_INSTALL_DIR")
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
        assert_eq!(root.join("Cargo.toml").exists(), derived);
    }
    assert!(managed_binary.is_file());
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_reports_missing_windows_managed_product_without_path_fallback() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let missing = fixture.path().join("missing");
    let decoy = fixture.path().join("decoy");
    fs::create_dir_all(&decoy).expect("decoy directory");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), decoy.join("sc-lint.exe"))
        .expect("PATH decoy product copy");
    let path = format!(
        "{};{}",
        decoy.display(),
        std::env::var("PATH").expect("host PATH")
    );

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", "sc-lint", "version"])
        .current_dir(&root)
        .env_remove("SC_LINT_BIN")
        .env("SC_LINT_INSTALL_DIR", &missing)
        .env("PATH", path)
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "sc-lint binary is unavailable at {}; run just setup",
            missing.join("sc-lint.exe").display()
        )),
        "stderr: {stderr}"
    );
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_forwards_windows_config() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let config = root.join("sc-lint.toml");
    fs::write(&config, "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n").expect("consumer config");
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), managed.join("sc-lint.exe"))
        .expect("managed product copy");

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--config"])
        .arg(&config)
        .args(["--", "sc-lint", "version"])
        .current_dir(&root)
        .env_remove("SC_LINT_BIN")
        .env("SC_LINT_INSTALL_DIR", &managed)
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
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_honors_windows_sc_lint_bin_override() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let override_directory = fixture.path().join("override");
    fs::create_dir_all(&override_directory).expect("override directory");
    let override_binary = override_directory.join("sc-lint.exe");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &override_binary).expect("override product copy");
    let system32 = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .expect("Windows SystemRoot")
        .join("System32");
    let path = std::env::join_paths([&override_directory, &system32]).expect("valid test PATH");

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", "sc-lint", "version"])
        .current_dir(&root)
        .env(
            "LOCALAPPDATA",
            fixture.path().join("missing-local-app-data"),
        )
        .env(
            "SC_LINT_INSTALL_DIR",
            fixture.path().join("missing-install-dir"),
        )
        .env("PATH", path)
        .env("SC_LINT_BIN", "sc-lint.exe")
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
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_does_not_fall_back_when_windows_sc_lint_bin_override_is_missing() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), managed.join("sc-lint.exe"))
        .expect("managed product copy");
    let missing_override = fixture.path().join("missing-override.exe");

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", "sc-lint", "version"])
        .current_dir(&root)
        .env("SC_LINT_INSTALL_DIR", &managed)
        .env("SC_LINT_BIN", &missing_override)
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!(
            "sc-lint binary is unavailable at {}; run just setup",
            missing_override.display()
        )),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_returns_zero_for_non_native_command_after_stale_status() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), managed.join("sc-lint.exe"))
        .expect("managed product copy");
    let child = fixture.path().join("consumer-command.ps1");
    fs::write(&child, "Write-Output 'non-native command ran'\n").expect("PowerShell child");
    let bootstrap = fixture.path().join("bootstrap-with-stale-status.ps1");
    write_windows_stale_status_bootstrap(&bootstrap);

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(bootstrap)
        .args(["exec", "--"])
        .arg(child)
        .current_dir(&root)
        .env("SC_LINT_INSTALL_DIR", managed)
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("non-native command ran"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(windows)]
#[test]
fn bootstrap_exec_returns_127_for_launch_failure_after_stale_status() {
    let fixture = TempDir::new().expect("fixture root");
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), managed.join("sc-lint.exe"))
        .expect("managed product copy");
    let missing = fixture.path().join("missing-command.exe");
    let bootstrap = fixture.path().join("bootstrap-with-stale-status.ps1");
    write_windows_stale_status_bootstrap(&bootstrap);

    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(bootstrap)
        .args(["exec", "--"])
        .arg(missing)
        .current_dir(&root)
        .env("SC_LINT_INSTALL_DIR", managed)
        .output()
        .expect("run PowerShell consumer bootstrap");

    assert_eq!(
        output.status.code(),
        Some(127),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "launch failure should be reported to stderr"
    );
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
    let root = windows_consumer_fixture(fixture.path(), "simple-consumer", false);
    let managed = fixture.path().join("managed");
    fs::create_dir_all(&managed).expect("managed directory");
    let managed_binary = managed.join("sc-lint.exe");
    fs::copy(env!("CARGO_BIN_EXE_sc-lint"), &managed_binary).expect("managed product copy");
    let child = fixture.path().join("consumer-command.cmd");
    let log = fixture.path().join("child.log");
    fs::write(
        &child,
        "@echo off\r\n> \"%SC_LINT_EXEC_LOG%\" echo cwd=%CD%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo bin=%SC_LINT_BIN%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo custom=%SC_LINT_TEST_VALUE%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo path=%PATH%\r\n>> \"%SC_LINT_EXEC_LOG%\" echo arg1=%~1\r\n>> \"%SC_LINT_EXEC_LOG%\" echo arg2=%~2\r\nexit /b 23\r\n",
    )
    .expect("consumer command fixture");
    let log_path = log.to_str().expect("log path");
    let managed_path = managed.to_str().expect("managed path");
    let child_path = child.to_str().expect("child path");
    let output = powershell_command()
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
        .args(["exec", "--", child_path, "two words", "tail"])
        .current_dir(&root)
        .env_remove("SC_LINT_BIN")
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
    assert_windows_logged_path(&captured, "cwd", &root);
    assert_windows_logged_path(&captured, "bin", &managed_binary);
    assert_eq!(windows_log_value(&captured, "custom"), "preserved");
    assert!(
        windows_log_value(&captured, "path")
            .to_ascii_lowercase()
            .starts_with(
                &format!(
                    "{};",
                    managed
                        .canonicalize()
                        .expect("canonical managed path")
                        .display()
                )
                .to_ascii_lowercase()
            ),
        "{captured}"
    );
    assert_eq!(windows_log_value(&captured, "arg1"), "two words");
    assert_eq!(windows_log_value(&captured, "arg2"), "tail");
}

#[cfg(unix)]
struct OverrideFixture {
    _fixture: TempDir,
    root: PathBuf,
    on_path: PathBuf,
    log: PathBuf,
}

#[cfg(unix)]
fn override_fixture(cwd_stub: bool, path_stub: bool) -> OverrideFixture {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer_fixture(fixture.path(), "override-consumer", false);
    let on_path = fixture.path().join("on-path");
    fs::create_dir_all(&on_path).expect("PATH directory");
    let log = fixture.path().join("override.log");
    let stub =
        |marker: &str| format!("#!/bin/sh\nprintf '{marker}\\n' > \"$SC_LINT_EXEC_LOG\"\nexit 0\n");
    if cwd_stub {
        write_executable(&root.join("sc-lint"), &stub("cwd"));
    }
    if path_stub {
        write_executable(&on_path.join("sc-lint"), &stub("path"));
    }
    fs::create_dir_all(root.join("bin")).expect("relative override directory");
    write_executable(&root.join("bin/sc-lint"), &stub("relative"));
    OverrideFixture {
        _fixture: fixture,
        root,
        on_path,
        log,
    }
}

#[cfg(unix)]
fn run_with_override(fixture: &OverrideFixture, sc_lint_bin: &str) -> Output {
    run_bootstrap(
        &fixture.root,
        &["exec", "--", "sc-lint", "version"],
        &[
            ("SC_LINT_BIN", sc_lint_bin),
            ("SC_LINT_EXEC_LOG", fixture.log.to_str().expect("log path")),
            (
                "PATH",
                &format!("{}:/usr/bin:/bin", fixture.on_path.display()),
            ),
        ],
    )
}

#[cfg(unix)]
fn assert_override_ran(fixture: &OverrideFixture, output: &Output, marker: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&fixture.log).expect("override log"),
        format!("{marker}\n")
    );
}

#[cfg(unix)]
fn assert_override_unavailable(fixture: &OverrideFixture, output: &Output, sc_lint_bin: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("sc-lint binary is unavailable at {sc_lint_bin}; run just setup\n")
    );
    assert!(
        !fixture.log.exists(),
        "an unresolved override must not fall back"
    );
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_resolves_bare_sc_lint_bin_override_on_path() {
    let fixture = override_fixture(false, true);
    let output = run_with_override(&fixture, "sc-lint");
    assert_override_ran(&fixture, &output, "path");
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_does_not_resolve_bare_sc_lint_bin_override_from_cwd() {
    let fixture = override_fixture(true, false);
    let output = run_with_override(&fixture, "sc-lint");
    assert_override_unavailable(&fixture, &output, "sc-lint");
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_prefers_path_over_cwd_for_bare_sc_lint_bin_override() {
    let fixture = override_fixture(true, true);
    let output = run_with_override(&fixture, "sc-lint");
    assert_override_ran(&fixture, &output, "path");
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_uses_explicit_relative_sc_lint_bin_override() {
    let fixture = override_fixture(false, true);
    let output = run_with_override(&fixture, "bin/sc-lint");
    assert_override_ran(&fixture, &output, "relative");
}

#[cfg(unix)]
#[test]
fn bootstrap_exec_does_not_fall_back_when_sc_lint_bin_override_is_missing() {
    let fixture = override_fixture(true, true);
    for missing in ["missing-sc-lint", "bin/missing-sc-lint"] {
        let output = run_with_override(&fixture, missing);
        assert_override_unavailable(&fixture, &output, missing);
    }
}
