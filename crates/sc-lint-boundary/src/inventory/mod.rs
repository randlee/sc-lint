use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;

mod dependency_policy;
mod types;

pub(crate) use dependency_policy::ForbiddenPackageEdge;
pub(crate) use dependency_policy::WorkspacePackageName;
pub(crate) use types::BoundaryInventory;
pub(crate) use types::BoundaryRecord;
pub(crate) use types::CallersSection;
pub(crate) use types::ReferenceScope;

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
        let record = types::BoundaryRecord::try_from(raw_record).with_context(|| {
            format!(
                "failed to validate dependency policy in `{}`",
                path.display()
            )
        })?;
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
    let planning: types::PlanningMetadata = parse_toml_file(&planning_path)?;
    validate_planning_metadata(&planning, &planning_path)?;

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

    let expected_owner_crate_path = record.owner_package.replace('-', "_");
    if record.owner_crate_path.as_str() != expected_owner_crate_path {
        anyhow::bail!(
            "boundary `{}` declares owner_crate_path `{}` but expected `{expected_owner_crate_path}` from owner_package `{}`",
            record.boundary_id,
            record.owner_crate_path,
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

fn validate_boundary_schema(record: &BoundaryRecord, path: &Path) -> Result<()> {
    if record.public.facade.trim().is_empty() {
        anyhow::bail!(
            "boundary `{}` in `{}` must define a non-empty public.facade",
            record.boundary_id,
            path.display()
        );
    }

    match record.implementation.visibility {
        types::Visibility::Public => {
            if record
                .implementation
                .implementation_type
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.type for public visibility",
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
                    "boundary `{}` in `{}` must define implementation.module for public visibility",
                    record.boundary_id,
                    path.display()
                );
            }
            if record.implementation.constructor.is_none() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.constructor for public visibility",
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

fn validate_planning_metadata(
    planning: &types::PlanningMetadata,
    planning_path: &Path,
) -> Result<()> {
    for key in planning.planned_items.keys() {
        if !key.starts_with("BOUNDARY-") || !key.contains('.') {
            anyhow::bail!(
                "planning item key `{key}` in `{}` must use <boundary_id>.<section>.<field>[.<subfield>] shape",
                planning_path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
