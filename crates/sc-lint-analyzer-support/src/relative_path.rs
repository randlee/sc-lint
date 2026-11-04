use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use crate::source_scan::relative_slash_path;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepoRelativePath(String);

impl RepoRelativePath {
    #[cfg(test)]
    pub(crate) fn from_raw_for_test(value: &str) -> Self {
        Self(value.to_owned())
    }

    pub fn parse(value: &str) -> Result<Self, RelativePathError> {
        let value = value.replace('\\', "/");
        if value.starts_with('/') || is_windows_absolute(&value) {
            return Err(RelativePathError::Absolute);
        }

        let mut segments = Vec::new();
        for segment in value.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(RelativePathError::EscapesRoot);
                    }
                }
                _ => segments.push(segment),
            }
        }
        if segments.is_empty() {
            return Err(RelativePathError::Empty);
        }
        Ok(Self(segments.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RepoRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RepoRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum RelativePathError {
    #[error("repository-relative path must not be empty")]
    Empty,
    #[error("repository-relative path must not be absolute")]
    Absolute,
    #[error("repository-relative path must not escape the repository root")]
    EscapesRoot,
}

fn is_windows_absolute(value: &str) -> bool {
    value.len() >= 3
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':'
        && value.as_bytes()[2] == b'/'
}

/// Failure to represent a source path relative to its repository root.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "failed to build repository-relative source path for `{}` under `{}`",
    source_path.display(),
    root.display()
)]
pub struct RepoRelativeSourcePathError {
    pub source_path: PathBuf,
    pub root: PathBuf,
    #[source]
    pub source: RelativePathError,
}

/// Normalize a source path relative to its repository, retaining path context on failure.
pub fn repo_relative_source_path(
    root: &Path,
    source_path: &Path,
) -> Result<RepoRelativePath, RepoRelativeSourcePathError> {
    let relative = relative_slash_path(root, source_path);
    RepoRelativePath::parse(&relative).map_err(|source| RepoRelativeSourcePathError {
        source_path: source_path.to_path_buf(),
        root: root.to_path_buf(),
        source,
    })
}

/// Strip the repository root or a caller-resolved alias without filesystem access.
///
/// Callers may supply a cached canonical root with their platform's preferred
/// normalization, such as a Windows root without the verbatim path prefix.
pub fn relative_path_under_root<'a>(
    root: &Path,
    path: &'a Path,
    canonical_root: Option<&Path>,
) -> Option<&'a Path> {
    path.strip_prefix(root)
        .ok()
        .or_else(|| canonical_root.and_then(|root| path.strip_prefix(root).ok()))
}
