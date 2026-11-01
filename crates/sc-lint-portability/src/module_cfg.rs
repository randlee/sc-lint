//! Rule-neutral out-of-line module resolution and inherited cfg facts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use sc_lint_analyzer_support::{FileContext, PackageTargetKind, WorkspaceMetadata};
use sc_lint_analyzer_support::{
    attr_is_cfg_test, discover_source_files_with_metadata, span_start_line,
};
use syn::spanned::Spanned;
use syn::{Item, ItemMod};

use crate::predicates::{attr_is_cfg_unix, attr_is_cfg_windows, attr_is_exact_cfg_unix};

#[derive(Debug, Clone)]
pub(super) struct ModuleDeclaration {
    pub(super) source_path: PathBuf,
    pub(super) line: usize,
    pub(super) is_test_only: bool,
    pub(super) is_unix: bool,
    pub(super) is_windows: bool,
    unix_only: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct EnclosingCfg {
    is_test_only: bool,
    is_unix: bool,
    is_windows: bool,
    unix_only: bool,
}

impl EnclosingCfg {
    fn including(self, module: &ItemMod) -> Self {
        Self {
            is_test_only: self.is_test_only || module.attrs.iter().any(attr_is_cfg_test),
            is_unix: self.is_unix || module.attrs.iter().any(attr_is_cfg_unix),
            is_windows: self.is_windows || module.attrs.iter().any(attr_is_cfg_windows),
            unix_only: self.unix_only || module.attrs.iter().any(attr_is_exact_cfg_unix),
        }
    }
}

/// One workspace scan shared by adapter validation and parity classification.
#[derive(Debug, Default)]
pub(super) struct ModuleCfg {
    declarations: BTreeMap<PathBuf, ModuleDeclaration>,
    ambiguous: BTreeSet<PathBuf>,
    crate_roots: BTreeSet<PathBuf>,
}

impl ModuleCfg {
    pub(super) fn collect(workspace: &WorkspaceMetadata) -> anyhow::Result<Self> {
        let mut facts = Self::default();
        for package in workspace.workspace_packages() {
            for target in package.targets {
                if matches!(target.kind, PackageTargetKind::Lib | PackageTargetKind::Bin) {
                    facts.crate_roots.insert(target.src_path.canonicalize()?);
                }
            }
        }
        for context in discover_source_files_with_metadata(workspace)? {
            if context.is_test_file {
                continue;
            }
            let source = fs::read_to_string(&context.source_path).with_context(|| {
                format!(
                    "failed to read Rust source for portability analysis: {}",
                    context.source_path.display()
                )
            })?;
            let file = syn::parse_file(&source).with_context(|| {
                format!(
                    "failed to parse Rust source for portability analysis: {}",
                    context.source_path.display()
                )
            })?;
            let parent = context
                .source_path
                .parent()
                .context("source has no parent")?;
            let directory = if facts
                .crate_roots
                .contains(&context.source_path.canonicalize()?)
            {
                parent.to_path_buf()
            } else {
                module_dir(&context.source_path)?
            };
            facts.collect_file(
                &file.items,
                &context,
                &directory,
                parent,
                EnclosingCfg::default(),
            )?;
        }
        facts.inherit_file_module_cfg()?;
        Ok(facts)
    }

    pub(super) fn declarations(&self) -> &BTreeMap<PathBuf, ModuleDeclaration> {
        &self.declarations
    }

    pub(super) fn is_unix_only_file(&self, path: &Path) -> bool {
        let Ok(path) = path.canonicalize() else {
            return false;
        };
        !self.crate_roots.contains(&path)
            && !self.ambiguous.contains(&path)
            && self
                .declarations
                .get(&path)
                .is_some_and(|declaration| declaration.unix_only && !declaration.is_test_only)
    }

    fn collect_file(
        &mut self,
        items: &[Item],
        context: &FileContext,
        directory: &Path,
        path_directory: &Path,
        enclosing: EnclosingCfg,
    ) -> anyhow::Result<()> {
        for item in items {
            let Item::Mod(module) = item else { continue };
            let cfg = enclosing.including(module);
            if let Some((_, items)) = &module.content {
                let directory = match module_path(module) {
                    ModulePath::Default => directory.join(module.ident.to_string()),
                    ModulePath::Explicit(path) => path_directory.join(path),
                    ModulePath::Invalid => continue,
                };
                self.collect_file(items, context, &directory, &directory, cfg)?;
                continue;
            }
            let Some(path) = resolve_module_source(path_directory, directory, module)? else {
                continue;
            };
            let declaration = ModuleDeclaration {
                source_path: context.source_path.clone(),
                line: span_start_line(module.span()),
                is_test_only: cfg.is_test_only,
                is_unix: cfg.is_unix,
                is_windows: cfg.is_windows,
                unix_only: cfg.unix_only,
            };
            if self
                .declarations
                .insert(path.clone(), declaration)
                .is_some()
            {
                // Retain the existing adapter lookup, but never infer S6 from
                // one of several declarations of the same physical file.
                self.ambiguous.insert(path);
            }
        }
        Ok(())
    }

    fn inherit_file_module_cfg(&mut self) -> anyhow::Result<()> {
        loop {
            let mut updates = Vec::new();
            let mut ambiguous = Vec::new();
            for (child, declaration) in &self.declarations {
                let parent_path = declaration.source_path.canonicalize()?;
                if self.crate_roots.contains(&parent_path) {
                    // A crate root may also be named by #[path], but its
                    // target is compiled independently of that declaration.
                    continue;
                }
                let Some(parent) = self.declarations.get(&parent_path) else {
                    continue;
                };
                if self.ambiguous.contains(&parent_path) && !self.ambiguous.contains(child) {
                    ambiguous.push(child.clone());
                }
                let inherited = ModuleDeclaration {
                    is_test_only: declaration.is_test_only || parent.is_test_only,
                    is_unix: declaration.is_unix || parent.is_unix,
                    is_windows: declaration.is_windows || parent.is_windows,
                    unix_only: declaration.unix_only || parent.unix_only,
                    ..declaration.clone()
                };
                if inherited.is_test_only != declaration.is_test_only
                    || inherited.is_unix != declaration.is_unix
                    || inherited.is_windows != declaration.is_windows
                    || inherited.unix_only != declaration.unix_only
                {
                    updates.push((child.clone(), inherited));
                }
            }
            if updates.is_empty() && ambiguous.is_empty() {
                return Ok(());
            }
            self.declarations.extend(updates);
            self.ambiguous.extend(ambiguous);
        }
    }
}

fn module_dir(source: &Path) -> anyhow::Result<PathBuf> {
    let parent = source.parent().context("source has no parent")?;
    match source.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs" | "main.rs" | "mod.rs") => Ok(parent.to_path_buf()),
        Some(_) => Ok(parent.join(source.file_stem().unwrap_or_default())),
        None => Ok(parent.to_path_buf()),
    }
}

enum ModulePath {
    Default,
    Explicit(PathBuf),
    Invalid,
}

fn module_path(module: &ItemMod) -> ModulePath {
    let mut attributes = module
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("path"));
    let Some(attr) = attributes.next() else {
        return ModulePath::Default;
    };
    if attributes.next().is_some() {
        return ModulePath::Invalid;
    }
    let syn::Meta::NameValue(value) = &attr.meta else {
        return ModulePath::Invalid;
    };
    let syn::Expr::Lit(value) = &value.value else {
        return ModulePath::Invalid;
    };
    let syn::Lit::Str(value) = &value.lit else {
        return ModulePath::Invalid;
    };
    ModulePath::Explicit(PathBuf::from(value.value()))
}

fn resolve_module_source(
    path_directory: &Path,
    directory: &Path,
    module: &ItemMod,
) -> anyhow::Result<Option<PathBuf>> {
    // https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
    match module_path(module) {
        ModulePath::Invalid => return Ok(None),
        ModulePath::Explicit(path) => {
            let path = path_directory.join(path);
            return path
                .is_file()
                .then(|| path.canonicalize())
                .transpose()
                .map_err(Into::into);
        }
        ModulePath::Default => {}
    }
    let flat = directory.join(format!("{}.rs", module.ident));
    let nested = directory.join(module.ident.to_string()).join("mod.rs");
    let path = match (flat.is_file(), nested.is_file()) {
        (true, false) => flat,
        (false, true) => nested,
        _ => return Ok(None),
    };
    Ok(Some(path.canonicalize()?))
}
