use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;

mod dependency_policy;
mod semantic;
mod types;

pub(crate) use dependency_policy::WorkspacePackageName;
pub(crate) use semantic::InventoryDiagnostic;
pub(crate) use semantic::ValidatedDependencyPolicy;
pub(crate) use semantic::ValidatedInventory;
pub(crate) use semantic::ValidatedRecord;
pub(crate) use semantic::validate_inventory;
pub(crate) use types::BoundaryInventory;
pub(crate) use types::BoundaryRecord;
pub(crate) use types::CallersSection;
pub(crate) use types::ReferenceScope;

/// Resolves boundary inventory package references to canonical Cargo package names.
///
/// Boundary files keep their owner directory layout, but policy evaluation accepts
/// a workspace member directory basename or a library target name when each maps
/// to exactly one package.
pub(crate) struct WorkspacePackageResolver {
    packages: BTreeMap<String, WorkspacePackageName>,
    member_directories: BTreeMap<String, BTreeSet<WorkspacePackageName>>,
    library_targets: BTreeMap<String, BTreeSet<WorkspacePackageName>>,
}

impl WorkspacePackageResolver {
    pub(crate) fn from_workspace_packages(packages: &[&cargo_metadata::Package]) -> Self {
        let mut resolver = Self {
            packages: BTreeMap::new(),
            member_directories: BTreeMap::new(),
            library_targets: BTreeMap::new(),
        };

        for package in packages {
            let package_name = WorkspacePackageName::from_package_name(package.name.to_string());
            resolver
                .packages
                .insert(package.name.to_string(), package_name.clone());

            if let Some(member_directory) = package
                .manifest_path
                .parent()
                .and_then(|parent| parent.file_name())
            {
                resolver
                    .member_directories
                    .entry(member_directory.to_owned())
                    .or_default()
                    .insert(package_name.clone());
            }

            for target in package
                .targets
                .iter()
                .filter(|target| crate::is_library_target(target))
            {
                resolver
                    .library_targets
                    .entry(target.name.clone())
                    .or_default()
                    .insert(package_name.clone());
            }
        }

        resolver
    }

    pub(crate) fn resolve(
        &self,
        reference: &str,
        context: impl AsRef<str>,
    ) -> Result<WorkspacePackageName> {
        if let Some(package) = self.resolve_if_known(reference, context.as_ref())? {
            return Ok(package);
        }

        let known_packages = self
            .packages
            .keys()
            .map(|package| format!("`{package}`"))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "unknown workspace package reference `{reference}` in {}; use an exact package name, a workspace member directory, or a library target name; known package names: {known_packages}",
            context.as_ref(),
        );
    }

    pub(crate) fn resolve_if_known(
        &self,
        reference: &str,
        context: &str,
    ) -> Result<Option<WorkspacePackageName>> {
        if let Some(package) = self.packages.get(reference) {
            return Ok(Some(package.clone()));
        }
        if let Some(package) = self.resolve_alias(
            reference,
            context,
            &self.member_directories,
            "workspace member directory",
        )? {
            return Ok(Some(package));
        }
        self.resolve_alias(reference, context, &self.library_targets, "library target")
    }

    fn resolve_alias(
        &self,
        reference: &str,
        context: &str,
        aliases: &BTreeMap<String, BTreeSet<WorkspacePackageName>>,
        alias_kind: &str,
    ) -> Result<Option<WorkspacePackageName>> {
        let Some(packages) = aliases.get(reference) else {
            return Ok(None);
        };
        if packages.len() == 1 {
            return Ok(packages.iter().next().cloned());
        }

        let candidates = packages
            .iter()
            .map(|package| format!("`{package}`"))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "ambiguous workspace package reference `{reference}` in {context}: {alias_kind} matches {candidates}; use an exact package name",
        );
    }
}

pub(crate) fn load_boundary_inventory(root: &Path) -> Result<BoundaryInventory> {
    let boundaries_root = root.join("boundaries");
    if !boundaries_root.exists() {
        return Ok(BoundaryInventory {
            records: Vec::new(),
            planning: types::PlanningMetadata {
                planning: types::PlanningHeader {
                    current_sprint: types::SprintId::placeholder_empty_inventory(),
                },
                planned_items: BTreeMap::new(),
            },
        });
    }
    let boundary_paths = discover_boundary_files(&boundaries_root)?;
    let mut records = Vec::new();
    let mut seen_boundary_ids = BTreeMap::<types::BoundaryId, PathBuf>::new();

    for path in boundary_paths {
        let raw_record: types::RawBoundaryRecord = parse_toml_file(&path)?;
        let mut record = types::BoundaryRecord::try_from(raw_record).with_context(|| {
            format!(
                "failed to validate dependency policy in `{}`",
                path.display()
            )
        })?;
        record.record_path = path.clone();
        validate_boundary_schema(&record, &path)?;
        validate_boundary_path(&record, &path, &boundaries_root)?;
        if let Some(previous_path) =
            seen_boundary_ids.insert(record.boundary_id.clone(), path.clone())
        {
            anyhow::bail!(
                "duplicate boundary_id `{}` in `{}` and `{}`",
                record.boundary_id,
                previous_path.display(),
                path.display()
            );
        }
        records.push(record);
    }

    let planning_path = boundaries_root.join("planning.toml");
    if !planning_path.exists() {
        anyhow::bail!(
            "boundary inventory requires authoritative planning metadata at `{}`; add [planning].current_sprint",
            planning_path.display()
        );
    }
    let planning: types::PlanningMetadata = parse_toml_file(&planning_path)?;

    Ok(BoundaryInventory { records, planning })
}

fn discover_boundary_files(boundaries_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    collect_boundary_files(boundaries_root, &mut seen, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_boundary_files(
    dir: &Path,
    seen: &mut BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read boundary directory `{}`", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_boundary_files(&path, seen, files)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("planning.toml") {
            continue;
        }
        if seen.insert(path.clone()) {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_toml_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read TOML file `{}`", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse TOML file `{}`", path.display()))
}

fn validate_boundary_path(
    record: &BoundaryRecord,
    path: &Path,
    boundaries_root: &Path,
) -> Result<()> {
    let owner_dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .context("boundary file missing owner-package directory")?;
    if record.owner_package != owner_dir {
        anyhow::bail!(
            "boundary file `{}` is under owner directory `{owner_dir}` but declares owner_package `{}`",
            path.display(),
            record.owner_package
        );
    }

    let relative = path.strip_prefix(boundaries_root).with_context(|| {
        format!(
            "boundary file `{}` is outside boundaries root",
            path.display()
        )
    })?;
    if relative.components().count() != 2 {
        anyhow::bail!(
            "boundary file `{}` must use boundaries/<owner-package>/<boundary>.toml layout",
            path.display()
        );
    }

    Ok(())
}

pub(crate) fn validate_owner_crate_paths(
    inventory: &BoundaryInventory,
    metadata: &cargo_metadata::Metadata,
) -> Result<()> {
    let workspace_packages = crate::workspace_packages(metadata);
    let resolver = WorkspacePackageResolver::from_workspace_packages(&workspace_packages);
    for record in &inventory.records {
        if !record.status.state.requires_workspace_package() {
            continue;
        }
        let owner_package = resolver.resolve(
            record.owner_package.as_ref(),
            format!("owner_package for boundary `{}`", record.boundary_id),
        )?;
        let package = workspace_packages
            .iter()
            .find(|package| package.name.as_str() == owner_package.as_ref())
            .expect("resolved workspace package must exist");
        let Some(library) = package
            .targets
            .iter()
            .find(|target| crate::is_library_target(target))
        else {
            continue;
        };
        if record.owner_crate_path.as_str() != library.name {
            anyhow::bail!(
                "boundary `{}` declares owner_crate_path `{}` but expected `{}` from the lib target of owner_package `{}`",
                record.boundary_id,
                record.owner_crate_path,
                library.name,
                record.owner_package
            );
        }
    }
    Ok(())
}

fn validate_boundary_schema(record: &BoundaryRecord, path: &Path) -> Result<()> {
    let has_public_surface = [
        ("public.facade", record.public.facade.as_deref()),
        ("public.trait", record.public.trait_name.as_deref()),
    ]
    .into_iter()
    .map(|(field, value)| {
        let value = value.map(str::trim);
        if value.is_some_and(str::is_empty) {
            anyhow::bail!(
                "boundary `{}` in `{}` defines an empty {field}",
                record.boundary_id,
                path.display()
            );
        }
        Ok(value.is_some())
    })
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .any(|present| present);

    if record.public.facade.is_some() && record.public.trait_name.is_some() {
        anyhow::bail!(
            "boundary `{}` in `{}` must define exactly one of public.facade or public.trait",
            record.boundary_id,
            path.display()
        );
    }

    if !has_public_surface {
        anyhow::bail!(
            "boundary `{}` in `{}` must define a non-empty public.facade or public.trait",
            record.boundary_id,
            path.display()
        );
    }

    match record.implementation.visibility {
        types::Visibility::Public | types::Visibility::Private | types::Visibility::PubCrate => {
            if record
                .implementation
                .implementation_type
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.type for public, private, or pub(crate) visibility",
                    record.boundary_id,
                    path.display()
                );
            }
            if record
                .implementation
                .module
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.module for public, private, or pub(crate) visibility",
                    record.boundary_id,
                    path.display()
                );
            }
            if record.implementation.constructor.is_none() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.constructor for public, private, or pub(crate) visibility",
                    record.boundary_id,
                    path.display()
                );
            }
        }
        types::Visibility::TraitOnly => {
            if record.implementation.implementation_type.is_some() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must omit implementation.type when visibility is trait_only",
                    record.boundary_id,
                    path.display()
                );
            }
            if record.implementation.module.is_some() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must omit implementation.module when visibility is trait_only",
                    record.boundary_id,
                    path.display()
                );
            }
        }
    }

    if let Some(callers) = &record.callers {
        let mut seen_symbols = BTreeSet::new();
        for approved_entry in &callers.approved {
            if approved_entry.callers.is_empty() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define at least one approved caller for symbol `{}`",
                    record.boundary_id,
                    path.display(),
                    approved_entry.symbol.as_str()
                );
            }
            if !seen_symbols.insert(approved_entry.symbol.clone()) {
                anyhow::bail!(
                    "boundary `{}` in `{}` defines duplicate approved caller symbol `{}`",
                    record.boundary_id,
                    path.display(),
                    approved_entry.symbol.as_str()
                );
            }
            let mut seen_callers = BTreeSet::new();
            for approved_caller in &approved_entry.callers {
                if !seen_callers.insert(approved_caller.clone()) {
                    anyhow::bail!(
                        "boundary `{}` in `{}` defines duplicate approved caller `{}` for symbol `{}`",
                        record.boundary_id,
                        path.display(),
                        approved_caller.as_str(),
                        approved_entry.symbol.as_str()
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
