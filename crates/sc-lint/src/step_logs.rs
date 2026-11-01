//! Per-command output files for consumer profile steps.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

fn sanitized_step_name(name: &str) -> String {
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
    if safe.is_empty() {
        "step".to_owned()
    } else {
        safe
    }
}

/// Assign stable file names in declaration order, disambiguating sanitized-name collisions.
pub(crate) fn step_log_file_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let stems = names
        .into_iter()
        .map(sanitized_step_name)
        .collect::<Vec<_>>();
    let reserved = stems.iter().cloned().collect::<HashSet<_>>();
    let mut used = HashSet::new();
    let mut next_suffix = HashMap::<String, usize>::new();

    stems
        .into_iter()
        .map(|stem| {
            let suffix = next_suffix.entry(stem.clone()).or_insert(1);
            let mut candidate = stem.clone();
            if *suffix > 1 || used.contains(&candidate) {
                let mut number = (*suffix).max(2);
                loop {
                    candidate = format!("{stem}-{number}");
                    if !used.contains(&candidate) && !reserved.contains(&candidate) {
                        *suffix = number + 1;
                        break;
                    }
                    number += 1;
                }
            } else {
                *suffix = 2;
            }
            used.insert(candidate.clone());
            format!("{candidate}.log")
        })
        .collect()
}

pub(crate) fn step_log_path(root: &Path, file_name: &str) -> PathBuf {
    root.join("steps").join(file_name)
}

pub(crate) fn write_step_output(
    root: &Path,
    file_name: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> io::Result<PathBuf> {
    let destination = step_log_path(root, file_name);
    let directory = destination.parent().expect("step log path has a parent");
    fs::create_dir_all(directory)?;
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
    fn step_log_names_are_sanitized_and_collisions_are_ordered() {
        assert_eq!(
            step_log_file_names(["alpha/../beta", "alpha____beta", "alpha____beta"]),
            [
                "alpha____beta.log",
                "alpha____beta-2.log",
                "alpha____beta-3.log"
            ]
        );
        assert_eq!(
            step_log_file_names(["safe-2", "safe", "safe"]),
            ["safe-2.log", "safe.log", "safe-3.log"]
        );
        assert_eq!(step_log_file_names(["...", ""]), ["___.log", "step.log"]);
    }

    #[test]
    fn step_logs_overwrite_per_run() {
        let root = tempfile::tempdir().expect("log root");
        let path = write_step_output(
            root.path(),
            "check.log",
            b"old stdout with a longer tail\n",
            b"old stderr with a longer tail\n",
        )
        .expect("first write");
        write_step_output(root.path(), "check.log", b"new\n", b"new error\n")
            .expect("replacement write");
        assert_eq!(path, step_log_path(root.path(), "check.log"));
        assert_eq!(
            fs::read(path).expect("read log"),
            b"[stdout]\nnew\n[stderr]\nnew error\n"
        );
    }
}
