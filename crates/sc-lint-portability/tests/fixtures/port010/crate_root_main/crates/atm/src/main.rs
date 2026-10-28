fn main() {
    restore_default_sigpipe();
}

#[cfg(unix)]
fn restore_default_sigpipe() {}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}
