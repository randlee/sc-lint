use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use cargo_metadata::MetadataCommand;
use proc_macro2::Span;
use syn::Attribute;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PackageName(String);

impl PackageName {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        debug_assert!(!value.is_empty(), "package names must not be empty");
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TargetName(String);

impl TargetName {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        debug_assert!(!value.is_empty(), "target names must not be empty");
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TargetName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileContext {
    pub(crate) source_path: PathBuf,
    pub(crate) package: PackageName,
    pub(crate) target: TargetName,
    pub(crate) is_test_file: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Test,
    NonTest,
}

pub(crate) fn discover_source_files(root: &Path) -> Result<Vec<FileContext>> {
    let metadata = load_metadata(root)?;
    let workspace_members = metadata.workspace_members.clone();
    let mut files = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for package in &metadata.packages {
        if !workspace_members.iter().any(|id| id == &package.id) {
            continue;
        }
        for target in &package.targets {
            if !is_supported_target(target) {
                continue;
            }
            let package_name = PackageName::new(package.name.to_string());
            let target_name = TargetName::new(target.name.clone());
            let manifest_dir = package
                .manifest_path
                .as_std_path()
                .parent()
                .context("package manifest missing parent")?;
            let src_dir = manifest_dir.join("src");
            let tests_dir = manifest_dir.join("tests");
            collect_rust_files(
                &src_dir,
                false,
                &package_name,
                &target_name,
                &mut seen_paths,
                &mut files,
            )?;
            collect_rust_files(
                &tests_dir,
                true,
                &package_name,
                &target_name,
                &mut seen_paths,
                &mut files,
            )?;
        }
    }

    Ok(files)
}

pub(crate) fn count_scanned_crates(root: &Path) -> Result<usize> {
    let metadata = load_metadata(root)?;
    let workspace_members = metadata.workspace_members.clone();
    Ok(metadata
        .packages
        .iter()
        .filter(|package| workspace_members.iter().any(|id| id == &package.id))
        .count())
}

fn collect_rust_files(
    dir: &Path,
    is_test_file: bool,
    package: &PackageName,
    target: &TargetName,
    seen_paths: &mut BTreeSet<PathBuf>,
    files: &mut Vec<FileContext>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_rust_files(&path, is_test_file, package, target, seen_paths, files)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        files.push(FileContext {
            source_path: path,
            package: package.clone(),
            target: target.clone(),
            is_test_file,
        });
    }
    Ok(())
}

pub(crate) fn span_start_line(span: Span) -> usize {
    span.start().line
}

pub(crate) fn attr_is_cfg_test(attr: &Attribute) -> bool {
    let path = attr.path();
    if !path.is_ident("cfg") {
        return false;
    }
    attr.parse_args::<syn::Ident>()
        .map(|ident| ident == "test")
        .unwrap_or(false)
}

pub(crate) fn attr_is_test(attr: &Attribute) -> bool {
    attr.path().is_ident("test")
}

pub(crate) fn classify_scope(
    attrs: &[Attribute],
    inherited_scope: ScopeKind,
    name_hint_is_tests: Option<bool>,
) -> ScopeKind {
    if inherited_scope == ScopeKind::Test {
        return ScopeKind::Test;
    }
    if attrs.iter().any(attr_is_cfg_test) || attrs.iter().any(attr_is_test) {
        return ScopeKind::Test;
    }
    if name_hint_is_tests.unwrap_or(false) {
        return ScopeKind::Test;
    }
    ScopeKind::NonTest
}

fn is_supported_target(target: &cargo_metadata::Target) -> bool {
    target.kind.iter().any(|kind| {
        matches!(
            kind,
            cargo_metadata::TargetKind::Lib
                | cargo_metadata::TargetKind::Bin
                | cargo_metadata::TargetKind::Example
        )
    })
}

fn load_metadata(root: &Path) -> Result<cargo_metadata::Metadata> {
    MetadataCommand::new()
        .current_dir(root)
        .exec()
        .with_context(|| format!("failed to load cargo metadata for {}", root.display()))
}
