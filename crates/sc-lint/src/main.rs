use std::process::ExitCode;

fn main() -> ExitCode {
    sc_lint::run(std::env::args_os())
}
