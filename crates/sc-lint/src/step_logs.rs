//! Per-command output files for consumer profile steps.

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn step_log_file_name(expanded_plan_index: usize, name: &str) -> String {
    let safe = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
                char::from(byte)
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!(
        "{expanded_plan_index:04}-{}.log",
        if safe.is_empty() { "step" } else { &safe }
    )
}

pub(crate) fn write_step_output(
    root: &Path,
    expanded_plan_index: usize,
    name: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> io::Result<PathBuf> {
    let directory = root.join("steps");
    fs::create_dir_all(&directory)?;
    let destination = directory.join(step_log_file_name(expanded_plan_index, name));
    let mut file = fs::File::create(&destination)?;
    use std::io::Write;
    file.write_all(b"[stdout]\n")?;
    file.write_all(stdout)?;
    if !stdout.ends_with(b"\n") {
        file.write_all(b"\n")?;
    }
    file.write_all(b"[stderr]\n")?;
    file.write_all(stderr)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_log_names_safe_and_unique() {
        assert_eq!(
            step_log_file_name(0, "alpha/../beta"),
            "0000-alpha____beta.log"
        );
        assert_eq!(
            step_log_file_name(1, "alpha/../beta"),
            "0001-alpha____beta.log"
        );
        assert_eq!(step_log_file_name(2, "..."), "0002-___.log");
        assert_eq!(step_log_file_name(3, ""), "0003-step.log");
    }

    #[test]
    fn step_logs_overwrite_per_run() {
        let root = tempfile::tempdir().expect("log root");
        let path = write_step_output(root.path(), 0, "check", b"old\n", b"old error\n")
            .expect("first write");
        write_step_output(root.path(), 0, "check", b"new\n", b"new error\n")
            .expect("replacement write");
        assert_eq!(path, root.path().join("steps/0000-check.log"));
        assert_eq!(
            fs::read(path).expect("read log"),
            b"[stdout]\nnew\n[stderr]\nnew error\n"
        );
    }
}
