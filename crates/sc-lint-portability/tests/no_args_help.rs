//! A bare invocation of `sc-lint-portability` prints its help to stdout and exits 0.

use std::process::Command;

#[test]
fn no_arguments_prints_help_and_exits_zero() {
    // Clap colours help by environment; pin it off so the plain usage line is asserted.
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("run sc-lint-portability without arguments");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "help must not be written to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage: sc-lint-portability"),
        "stdout must contain the usage line: {stdout}"
    );
}

#[test]
fn unknown_flag_keeps_the_usage_error_exit_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .arg("--definitely-not-a-flag")
        .output()
        .expect("run sc-lint-portability with an unknown flag");

    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
