use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

pub(super) fn expand(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut matched = BTreeSet::new();
    for pattern in patterns {
        let normalized = pattern.replace('\\', "/");
        let segments = normalized.split('/').collect::<Vec<_>>();
        if normalized.starts_with('/')
            || segments
                .iter()
                .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
        {
            continue;
        }

        let first_wildcard = segments.iter().position(|segment| is_wildcard(segment));
        let prefix_len = first_wildcard.unwrap_or(segments.len());
        let mut base = root.to_path_buf();
        for segment in &segments[..prefix_len] {
            base.push(segment);
        }

        if first_wildcard.is_none() {
            if base.is_file() {
                matched.insert(base);
            }
            continue;
        }

        if base.is_file() {
            if let Some(relative) = base.strip_prefix(root).ok().and_then(Path::to_str)
                && matches(&segments, &relative.replace('\\', "/"))
            {
                matched.insert(base);
            }
            continue;
        }
        if !base.is_dir() {
            continue;
        }

        let mut files = Vec::new();
        collect_files(&base, &mut files)?;
        for path in files {
            let Some(relative) = path.strip_prefix(root).ok().and_then(Path::to_str) else {
                continue;
            };
            if matches(&segments, &relative.replace('\\', "/")) {
                matched.insert(path);
            }
        }
    }
    Ok(matched.into_iter().collect())
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to enumerate `{}`", directory.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in `{}`", directory.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect `{}`", entry.path().display()))?;
        if file_type.is_dir() {
            if entry.file_name() != ".git" {
                collect_files(&entry.path(), files)?;
            }
        } else if file_type.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn is_wildcard(segment: &str) -> bool {
    segment.contains('*') || segment.contains('?')
}

fn matches(pattern: &[&str], path: &str) -> bool {
    let path = path.split('/').collect::<Vec<_>>();
    let mut previous = vec![false; path.len() + 1];
    previous[0] = true;
    for segment in pattern {
        let mut current = vec![false; path.len() + 1];
        if *segment == "**" {
            current[0] = previous[0];
            for path_index in 1..=path.len() {
                current[path_index] = previous[path_index] || current[path_index - 1];
            }
        } else {
            for path_index in 1..=path.len() {
                current[path_index] =
                    previous[path_index - 1] && segment_matches(segment, path[path_index - 1]);
            }
        }
        previous = current;
    }
    previous[path.len()]
}

fn segment_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for byte in pattern {
        let mut current = vec![false; value.len() + 1];
        if *byte == b'*' {
            current[0] = previous[0];
            for index in 1..=value.len() {
                current[index] = previous[index] || current[index - 1];
            }
        } else {
            for index in 1..=value.len() {
                current[index] =
                    previous[index - 1] && (*byte == b'?' || *byte == value[index - 1]);
            }
        }
        previous = current;
    }
    previous[value.len()]
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn path_globs_match_recursive_and_segment_wildcards() {
        assert!(matches(
            &["templates", "**", "Cargo.toml"],
            "templates/a/b/Cargo.toml"
        ));
        assert!(matches(
            &["target", "package", "*.crate"],
            "target/package/x-1.2.crate"
        ));
        assert!(!matches(
            &["target", "package", "*.crate"],
            "target/package/nested/x.crate"
        ));
        assert!(!matches(&["tests", "?"], "tests/fixtures"));
    }
}
