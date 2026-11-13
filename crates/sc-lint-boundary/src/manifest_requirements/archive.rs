use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use flate2::read::GzDecoder;
use tar::Archive;

use crate::BoundaryError;
use crate::Finding;
use crate::OwnerId;

use super::manifest;

pub(super) fn findings(archive_path: &Path, archive_label: &str) -> Result<Vec<Finding>> {
    let file = File::open(archive_path)
        .with_context(|| format!("failed to open package archive `{archive_label}`"))?;
    let mut archive = Archive::new(GzDecoder::new(file));
    let mut root_name = None;
    let mut findings = Vec::new();
    for entry in archive
        .entries()
        .with_context(|| format!("failed to read package archive `{archive_label}`"))?
    {
        let mut entry =
            entry.with_context(|| format!("failed to read entry in `{archive_label}`"))?;
        let entry_path = entry
            .path()
            .with_context(|| format!("failed to read entry path in `{archive_label}`"))?
            .into_owned();
        if !inside_archive_root(&entry_path, &mut root_name) {
            return Err(BoundaryError::ArchivePathEscape {
                archive: PathBuf::from(archive_label),
                entry: entry_path,
            }
            .into());
        }
        if !entry.header().entry_type().is_file()
            || entry_path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml.orig")
        {
            continue;
        }

        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .with_context(|| format!("failed to read Cargo.toml.orig in `{archive_label}`"))?;
        let entry_label = entry_path.to_string_lossy().replace('\\', "/");
        let manifest_label = format!("{archive_label} {entry_label}");
        let mut entry_findings = manifest::findings(&manifest_label, &text)?;
        for finding in &mut entry_findings {
            finding.owner_ids = vec![OwnerId::new(archive_label.to_owned())];
        }
        findings.extend(entry_findings);
    }
    Ok(findings)
}

fn inside_archive_root(path: &Path, root_name: &mut Option<String>) -> bool {
    let raw = path.to_string_lossy().replace('\\', "/");
    if raw.starts_with('/') || has_windows_prefix(&raw) {
        return false;
    }

    let mut components = Vec::<String>::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if components.len() <= 1 {
                    return false;
                }
                components.pop();
            }
            value => components.push(value.to_owned()),
        }
    }
    let Some(first) = components.first() else {
        return false;
    };
    match root_name {
        Some(expected) if expected != first => false,
        Some(_) => true,
        None => {
            *root_name = Some(first.clone());
            true
        }
    }
}

fn has_windows_prefix(path: &str) -> bool {
    path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::inside_archive_root;

    #[test]
    fn archive_paths_must_stay_under_the_first_top_level_directory() {
        let mut root = None;
        assert!(inside_archive_root(
            Path::new("pkg-1.0.0/src/lib.rs"),
            &mut root
        ));
        assert!(inside_archive_root(
            Path::new("pkg-1.0.0/src/../Cargo.toml.orig"),
            &mut root
        ));
        assert!(!inside_archive_root(
            Path::new("pkg-1.0.0/../../outside"),
            &mut root
        ));
        assert!(!inside_archive_root(
            Path::new("other-1.0.0/Cargo.toml.orig"),
            &mut root
        ));
        assert!(!inside_archive_root(Path::new("C:\\outside"), &mut root));
    }
}
