pub fn named_pipe_name(name: &str) -> String {
    if name.starts_with(r"C:\pipe\") {
        name.to_owned()
    } else {
        format!(r"C:\pipe\{name}")
    }
}

pub fn unix_socket_path() -> &'static str {
    "/var/run/sc-lint.sock"
}
