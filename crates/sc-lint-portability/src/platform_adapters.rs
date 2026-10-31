use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::RelativePathError;
use sc_lint_analyzer_support::RepoRelativePath;
use thiserror::Error;

use crate::PortabilityError;
use crate::module_cfg::{ModuleCfg, ModuleDeclaration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlatformAdapterRecord {
    unix_module: String,
    windows_module: String,
    reason: NonEmptyReason,
}

pub(super) fn parse_records(
    settings: &AnalyzerSettings,
) -> Result<Vec<PlatformAdapterRecord>, AnalyzerConfigError> {
    let Some(tables) = settings.own_tables("platform_adapters")? else {
        return Ok(Vec::new());
    };

    tables
        .into_iter()
        .enumerate()
        .map(|(index, table)| {
            for key in table.keys() {
                if !matches!(key.as_str(), "unix_module" | "windows_module" | "reason") {
                    return Err(AnalyzerConfigError::UnknownKey {
                        analyzer: "portability",
                        key: format!("platform_adapters[{index}].{key}"),
                    });
                }
            }
            let field = |name: &str| match table.get(name) {
                Some(toml::Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
                _ => Err(AnalyzerConfigError::WrongType {
                    analyzer: "portability",
                    key: format!("platform_adapters[{index}].{name}"),
                    expected: "a non-empty string",
                }),
            };
            Ok(PlatformAdapterRecord {
                unix_module: field("unix_module")?,
                windows_module: field("windows_module")?,
                reason: field("reason").map(NonEmptyReason::new)?.ok_or_else(|| {
                    AnalyzerConfigError::WrongType {
                        analyzer: "portability",
                        key: format!("platform_adapters[{index}].reason"),
                        expected: "a non-empty string",
                    }
                })?,
            })
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum AdapterConfigError {
    #[error(
        "ANALYZER_CONFIG.ADAPTER_PATH index {index} field `{field}` value `{value}` is invalid ({cause}); use a confined repository-relative .rs file"
    )]
    Path {
        index: usize,
        field: &'static str,
        value: String,
        cause: AdapterPathCause,
    },
    #[error(
        "ANALYZER_CONFIG.ADAPTER_MISSING index {index} field `{field}` value `{value}` must name an existing confined .rs file; use a repository-relative implementation file"
    )]
    Missing {
        index: usize,
        field: &'static str,
        value: String,
    },
    #[error(
        "ANALYZER_CONFIG.ADAPTER_DUPLICATE index {index} field `{field}` value `{value}` resolves to an adapter module already used by this configuration; use distinct files"
    )]
    Duplicate {
        index: usize,
        field: &'static str,
        value: String,
    },
    #[error(
        "ANALYZER_CONFIG.ADAPTER_CFG index {index} field `{field}` value `{value}` must be declared by an out-of-line module with #[cfg({platform})]; declare the production platform module"
    )]
    Cfg {
        index: usize,
        field: &'static str,
        value: String,
        platform: &'static str,
    },
    #[error(
        "ANALYZER_CONFIG.ADAPTER_TEST_ONLY index {index} field `{field}` value `{value}` resolves to a test-only module; supply a production counterpart"
    )]
    TestOnly {
        index: usize,
        field: &'static str,
        value: String,
    },
}

impl AdapterConfigError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Path { .. } => "ANALYZER_CONFIG.ADAPTER_PATH",
            Self::Missing { .. } => "ANALYZER_CONFIG.ADAPTER_MISSING",
            Self::Duplicate { .. } => "ANALYZER_CONFIG.ADAPTER_DUPLICATE",
            Self::Cfg { .. } => "ANALYZER_CONFIG.ADAPTER_CFG",
            Self::TestOnly { .. } => "ANALYZER_CONFIG.ADAPTER_TEST_ONLY",
        }
    }
}

/// Why an adapter path is not a confined repository-relative `.rs` file.
#[derive(Debug, Error)]
pub enum AdapterPathCause {
    #[error(transparent)]
    Relative(#[from] RelativePathError),
    #[error("not a .rs file")]
    NotRustSource,
}

/// A platform adapter's required justification: never empty or blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NonEmptyReason(String);

impl NonEmptyReason {
    pub(super) fn new(value: String) -> Option<Self> {
        (!value.trim().is_empty()).then_some(Self(value))
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct PlatformAdapters {
    unix_declarations: BTreeSet<(PathBuf, usize)>,
    adapter_files: BTreeSet<PathBuf>,
}

impl PlatformAdapters {
    pub(super) fn suppresses_port010(&self, source_path: &Path, line: usize) -> bool {
        self.unix_declarations
            .contains(&(source_path.to_path_buf(), line))
    }

    pub(super) fn is_adapter_file(&self, source_path: &Path) -> bool {
        let key = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.to_path_buf());
        self.adapter_files.contains(&key)
    }
}

pub(super) fn validate(
    root: &Path,
    records: &[PlatformAdapterRecord],
    module_cfg: &ModuleCfg,
) -> Result<PlatformAdapters, PortabilityError> {
    if records.is_empty() {
        return Ok(PlatformAdapters::default());
    }

    let scan_error = |source: anyhow::Error| PortabilityError::AdapterScan {
        root: root.to_path_buf(),
        source: source.into(),
    };
    let canonical_root = root
        .canonicalize()
        .map_err(|source| scan_error(source.into()))?;
    let declarations = module_cfg.declarations();
    let root = canonical_root;
    let mut used_files = BTreeSet::new();
    let mut validate_pair =
        |root: &Path,
         record: &PlatformAdapterRecord,
         index: usize|
         -> Result<(ModuleDeclaration, PathBuf, PathBuf), AdapterConfigError> {
            let unix = validate_path(root, index, "unix_module", &record.unix_module)?;
            let windows = validate_path(root, index, "windows_module", &record.windows_module)?;
            let unix_path = canonical_module_path(root, index, "unix_module", &unix)?;
            let windows_path = canonical_module_path(root, index, "windows_module", &windows)?;
            ensure_distinct(index, record, &mut used_files, &unix_path, &windows_path)?;
            let unix_declaration = validate_declaration(
                declarations,
                index,
                "unix_module",
                &record.unix_module,
                &unix_path,
                "unix",
            )?;
            validate_declaration(
                declarations,
                index,
                "windows_module",
                &record.windows_module,
                &windows_path,
                "windows",
            )?;
            Ok((unix_declaration, unix_path, windows_path))
        };
    let mut unix_declarations = BTreeSet::new();
    let mut adapter_files = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let (declaration, unix_path, windows_path) = validate_pair(&root, record, index)?;
        unix_declarations.insert((declaration.source_path, declaration.line));
        adapter_files.insert(unix_path);
        adapter_files.insert(windows_path);
    }

    Ok(PlatformAdapters {
        unix_declarations,
        adapter_files,
    })
}

fn ensure_distinct(
    index: usize,
    record: &PlatformAdapterRecord,
    used_files: &mut BTreeSet<PathBuf>,
    unix_path: &Path,
    windows_path: &Path,
) -> Result<(), AdapterConfigError> {
    if unix_path == windows_path || !used_files.insert(unix_path.to_path_buf()) {
        return Err(AdapterConfigError::Duplicate {
            index,
            field: "unix_module",
            value: record.unix_module.clone(),
        });
    }
    if !used_files.insert(windows_path.to_path_buf()) {
        return Err(AdapterConfigError::Duplicate {
            index,
            field: "windows_module",
            value: record.windows_module.clone(),
        });
    }
    Ok(())
}

fn validate_path(
    root: &Path,
    index: usize,
    field: &'static str,
    value: &str,
) -> Result<RepoRelativePath, AdapterConfigError> {
    let path = RepoRelativePath::parse(value).map_err(|cause| AdapterConfigError::Path {
        index,
        field,
        value: value.to_owned(),
        cause: cause.into(),
    })?;
    if root
        .join(path.as_str())
        .extension()
        .and_then(|value| value.to_str())
        != Some("rs")
    {
        return Err(AdapterConfigError::Path {
            index,
            field,
            value: value.to_owned(),
            cause: AdapterPathCause::NotRustSource,
        });
    }
    Ok(path)
}

fn canonical_module_path(
    root: &Path,
    index: usize,
    field: &'static str,
    path: &RepoRelativePath,
) -> Result<PathBuf, AdapterConfigError> {
    let candidate = root.join(path.as_str());
    let canonical = candidate
        .canonicalize()
        .map_err(|_| AdapterConfigError::Missing {
            index,
            field,
            value: path.to_string(),
        })?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(AdapterConfigError::Missing {
            index,
            field,
            value: path.to_string(),
        });
    }
    Ok(canonical)
}

fn validate_declaration(
    declarations: &BTreeMap<PathBuf, ModuleDeclaration>,
    index: usize,
    field: &'static str,
    value: &str,
    canonical_path: &Path,
    platform: &'static str,
) -> Result<ModuleDeclaration, AdapterConfigError> {
    let declaration = declarations
        .get(canonical_path)
        .ok_or_else(|| AdapterConfigError::Cfg {
            index,
            field,
            value: value.to_owned(),
            platform,
        })?;
    if declaration.is_test_only {
        return Err(AdapterConfigError::TestOnly {
            index,
            field,
            value: value.to_owned(),
        });
    }
    let valid_cfg = match platform {
        "unix" => declaration.is_unix,
        "windows" => declaration.is_windows,
        _ => false,
    };
    if !valid_cfg {
        return Err(AdapterConfigError::Cfg {
            index,
            field,
            value: value.to_owned(),
            platform,
        });
    }
    Ok(declaration.clone())
}
