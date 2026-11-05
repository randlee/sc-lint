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
use syn::Ident;
use syn::Item;

use crate::relative_path::relative_path_under_root;

pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    relative_path_under_root(&root, &path, None)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

macro_rules! name_type {
    ($name:ident, $description:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(String);
        impl $name {
            /// Constructs a non-empty name.
            ///
            /// # Panics
            /// Panics if `value` is empty.
            pub fn new(value: impl Into<String>) -> Self {
                let value = value.into();
                assert!(
                    !value.is_empty(),
                    concat!($description, " names must not be empty")
                );
                Self(value)
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}
name_type!(PackageName, "package");
name_type!(TargetName, "target");

#[derive(Debug, Clone)]
pub struct FileContext {
    pub source_path: PathBuf,
    pub package: PackageName,
    pub target: TargetName,
    pub is_test_file: bool,
}

/// Cargo workspace metadata shared by the source scanner and crate counter.
pub struct WorkspaceMetadata {
    metadata: cargo_metadata::Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageTargetKind {
    Lib,
    Bin,
    Example,
    Test,
    Bench,
    CustomBuild,
    Other,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageDependencyKind {
    Normal,
    Build,
    Development,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDependency {
    pub name: PackageName,
    pub rename: Option<String>,
    pub kind: PackageDependencyKind,
    pub target: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTarget {
    pub name: TargetName,
    pub kind: PackageTargetKind,
    pub src_path: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageInfo {
    pub name: PackageName,
    pub dependencies: Vec<PackageDependency>,
    pub targets: Vec<PackageTarget>,
}

impl WorkspaceMetadata {
    pub fn workspace_packages(&self) -> Vec<WorkspacePackageInfo> {
        let mut packages = self
            .metadata
            .packages
            .iter()
            .filter(|package| self.metadata.workspace_members.contains(&package.id))
            .map(|package| WorkspacePackageInfo {
                name: PackageName::new(package.name.to_string()),
                dependencies: package
                    .dependencies
                    .iter()
                    .map(|dependency| PackageDependency {
                        name: PackageName::new(dependency.name.clone()),
                        rename: dependency.rename.clone(),
                        kind: match dependency.kind {
                            cargo_metadata::DependencyKind::Normal => PackageDependencyKind::Normal,
                            cargo_metadata::DependencyKind::Build => PackageDependencyKind::Build,
                            cargo_metadata::DependencyKind::Development => {
                                PackageDependencyKind::Development
                            }
                            _ => PackageDependencyKind::Unknown,
                        },
                        target: dependency.target.as_ref().map(ToString::to_string),
                    })
                    .collect(),
                targets: package
                    .targets
                    .iter()
                    .map(|target| PackageTarget {
                        name: TargetName::new(target.name.clone()),
                        kind: classify_target_kinds(&target.kind),
                        src_path: target.src_path.clone().into_std_path_buf(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        packages.sort_by(|left, right| left.name.cmp(&right.name));
        packages
    }
}

fn classify_target_kinds(kinds: &[cargo_metadata::TargetKind]) -> PackageTargetKind {
    use cargo_metadata::TargetKind::*;
    if kinds.contains(&Bin) {
        PackageTargetKind::Bin
    } else if kinds.contains(&Example) {
        PackageTargetKind::Example
    } else if kinds.contains(&Test) {
        PackageTargetKind::Test
    } else if kinds.contains(&Bench) {
        PackageTargetKind::Bench
    } else if kinds.contains(&CustomBuild) {
        PackageTargetKind::CustomBuild
    } else if kinds
        .iter()
        .any(|kind| matches!(kind, Lib | RLib | DyLib | CDyLib | StaticLib | ProcMacro))
    {
        PackageTargetKind::Lib
    } else {
        PackageTargetKind::Other
    }
}

#[cfg(test)]
pub(super) fn classify_target_kinds_for_test(
    kinds: &[cargo_metadata::TargetKind],
) -> PackageTargetKind {
    classify_target_kinds(kinds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Test,
    NonTest,
}

pub fn discover_source_files(root: &Path) -> Result<Vec<FileContext>> {
    let metadata = load_workspace_metadata(root)?;
    discover_source_files_with_metadata(&metadata)
}

pub fn discover_source_files_with_metadata(
    workspace: &WorkspaceMetadata,
) -> Result<Vec<FileContext>> {
    let metadata = &workspace.metadata;
    let workspace_members = &metadata.workspace_members;
    let mut files = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for package in &metadata.packages {
        if !workspace_members.contains(&package.id) {
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

pub fn count_scanned_crates_with_metadata(workspace: &WorkspaceMetadata) -> usize {
    let metadata = &workspace.metadata;
    let workspace_members = &metadata.workspace_members;
    metadata
        .packages
        .iter()
        .filter(|package| workspace_members.contains(&package.id))
        .count()
}

pub fn span_start_line(span: Span) -> usize {
    span.start().line
}

pub fn attr_is_cfg_test(attr: &Attribute) -> bool {
    cfg_is_test(attr, false)
}

pub(crate) fn attr_is_test(attr: &Attribute) -> bool {
    test_attribute(attr, false)
}

/// Recognize test-only attributes, including qualified test macros and
/// `cfg(all(test, ...))`. Other cfg expressions are not evaluated.
/// Unlike `classify_scope`, this does not apply item-name conventions.
pub fn is_test_only(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| test_attribute(attr, true) || cfg_is_test(attr, true))
}

fn test_attribute(attr: &Attribute, qualified: bool) -> bool {
    let path = attr.path();
    path.is_ident("test")
        || (qualified
            && path.segments.len() >= 2
            && path
                .segments
                .last()
                .is_some_and(|part| part.ident == "test"))
}

fn cfg_is_test(attr: &Attribute, nested_all: bool) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    if !nested_all {
        return attr
            .parse_args::<syn::Ident>()
            .is_ok_and(|ident| ident == "test");
    }
    attr.parse_args::<syn::Meta>()
        .is_ok_and(|meta| meta_is_test_only(&meta))
}

fn meta_is_test_only(meta: &syn::Meta) -> bool {
    if meta.path().is_ident("test") {
        return true;
    }
    let syn::Meta::List(list) = meta else {
        return false;
    };
    list.path.is_ident("all")
        && list
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|items| items.iter().any(meta_is_test_only))
}

pub fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item_const) => &item_const.attrs,
        Item::Enum(item_enum) => &item_enum.attrs,
        Item::ExternCrate(item_extern_crate) => &item_extern_crate.attrs,
        Item::Fn(item_fn) => &item_fn.attrs,
        Item::ForeignMod(item_foreign_mod) => &item_foreign_mod.attrs,
        Item::Impl(item_impl) => &item_impl.attrs,
        Item::Macro(item_macro) => &item_macro.attrs,
        Item::Mod(item_mod) => &item_mod.attrs,
        Item::Static(item_static) => &item_static.attrs,
        Item::Struct(item_struct) => &item_struct.attrs,
        Item::Trait(item_trait) => &item_trait.attrs,
        Item::TraitAlias(item_trait_alias) => &item_trait_alias.attrs,
        Item::Type(item_type) => &item_type.attrs,
        Item::Union(item_union) => &item_union.attrs,
        Item::Use(item_use) => &item_use.attrs,
        _ => &[],
    }
}

pub fn item_name_hint_is_tests(item: &Item) -> Option<bool> {
    match item {
        Item::Fn(item_fn) => Some(item_fn.sig.ident == "tests"),
        Item::Mod(item_mod) => Some(item_mod.ident == "tests"),
        _ => None,
    }
}

pub fn item_identifier(item: &Item) -> Option<&Ident> {
    match item {
        Item::Fn(item_fn) => Some(&item_fn.sig.ident),
        Item::Mod(item_mod) => Some(&item_mod.ident),
        _ => None,
    }
}

pub fn classify_scope(
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

fn is_supported_target(target: &cargo_metadata::Target) -> bool {
    target.kind.iter().any(|kind| {
        matches!(
            kind,
            cargo_metadata::TargetKind::Lib
                | cargo_metadata::TargetKind::RLib
                | cargo_metadata::TargetKind::DyLib
                | cargo_metadata::TargetKind::CDyLib
                | cargo_metadata::TargetKind::StaticLib
                | cargo_metadata::TargetKind::ProcMacro
                | cargo_metadata::TargetKind::Bin
                | cargo_metadata::TargetKind::Example
        )
    })
}

pub fn load_workspace_metadata(root: &Path) -> Result<WorkspaceMetadata> {
    let metadata = MetadataCommand::new()
        .current_dir(root)
        .exec()
        .with_context(|| format!("failed to load cargo metadata for {}", root.display()))?;
    Ok(WorkspaceMetadata { metadata })
}
