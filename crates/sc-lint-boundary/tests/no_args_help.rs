//! A bare invocation of `sc-lint-boundary` prints its help to stdout and exits 0.

use std::process::Command;

#[test]
fn no_arguments_prints_help_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-boundary"))
        .output()
        .expect("run sc-lint-boundary without arguments");

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
        stdout.contains("Usage: sc-lint-boundary"),
        "stdout must contain the usage line: {stdout}"
    );
}

#[test]
fn unknown_flag_keeps_the_usage_error_exit_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-boundary"))
        .arg("--definitely-not-a-flag")
        .output()
        .expect("run sc-lint-boundary with an unknown flag");

    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
