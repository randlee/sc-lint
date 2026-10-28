#[cfg(unix)]
mod unix_socket;

#[cfg(windows)]
mod named_pipe;
