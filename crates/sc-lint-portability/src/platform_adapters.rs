use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::FileContext;
use sc_lint_analyzer_support::RelativePathError;
use sc_lint_analyzer_support::RepoRelativePath;
use sc_lint_analyzer_support::WorkspaceMetadata;
use sc_lint_analyzer_support::attr_is_cfg_test;
use sc_lint_analyzer_support::discover_source_files_with_metadata;
use sc_lint_analyzer_support::span_start_line;
use syn::Item;
use syn::ItemMod;
use syn::spanned::Spanned;
use thiserror::Error;

use crate::PortabilityError;
use crate::predicates::attr_is_cfg_unix;
use crate::predicates::attr_is_cfg_windows;

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

#[derive(Debug, Clone)]
struct ModuleDeclaration {
    source_path: PathBuf,
    line: usize,
    is_test_only: bool,
    is_unix: bool,
    is_windows: bool,
}

/// cfg flags from the module declarations that enclose a declaration.
///
/// Rust applies a `mod` attribute to the whole subtree the module declares, so
/// a declaration inherits `test`, `unix` and `windows` gating from every
/// enclosing module. Inline ancestors are threaded through
/// `collect_file_declarations`; file-module ancestors are propagated afterward
/// by `inherit_file_module_cfg`.
#[derive(Debug, Clone, Copy, Default)]
struct EnclosingCfg {
    is_test_only: bool,
    is_unix: bool,
    is_windows: bool,
}

impl EnclosingCfg {
    fn including(self, item_mod: &ItemMod) -> Self {
        Self {
            is_test_only: self.is_test_only || item_mod.attrs.iter().any(attr_is_cfg_test),
            is_unix: self.is_unix || item_mod.attrs.iter().any(attr_is_cfg_unix),
            is_windows: self.is_windows || item_mod.attrs.iter().any(attr_is_cfg_windows),
        }
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
    workspace: &mut Option<WorkspaceMetadata>,
    mut load_metadata: impl FnMut(&Path) -> anyhow::Result<WorkspaceMetadata>,
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
    if workspace.is_none() {
        *workspace = Some(load_metadata(root).map_err(scan_error)?);
    }
    let Some(workspace) = workspace.as_ref() else {
        return Err(scan_error(anyhow::anyhow!(
            "workspace metadata was not loaded for platform adapters"
        )));
    };
    // Discover declaring files from the same root the analyzer scans so their
    // source paths match its lookups; Windows canonical paths are verbatim.
    let declarations = collect_declarations(workspace).map_err(scan_error)?;
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
                &declarations,
                index,
                "unix_module",
                &record.unix_module,
                &unix_path,
                "unix",
            )?;
            validate_declaration(
                &declarations,
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

fn collect_declarations(
    workspace: &WorkspaceMetadata,
) -> anyhow::Result<BTreeMap<PathBuf, ModuleDeclaration>> {
    let mut declarations = BTreeMap::new();
    for context in discover_source_files_with_metadata(workspace)? {
        if context.is_test_file {
            continue;
        }
        let source = fs::read_to_string(&context.source_path)?;
        let file = syn::parse_file(&source)?;
        let module_dir = module_dir(&context.source_path)?;
        collect_file_declarations(
            &file.items,
            &context,
            &module_dir,
            EnclosingCfg::default(),
            &mut declarations,
        )?;
    }
    inherit_file_module_cfg(&mut declarations)?;
    Ok(declarations)
}

/// Propagates cfg flags from file modules to the declarations they contain.
///
/// Each file's declarations are collected with no enclosing cfg, so a
/// declaration inside `unix.rs` does not yet see a `#[cfg(unix)]` on the
/// `mod unix;` that declares that file. Each declaration is OR-ed with the
/// flags of the declaration that declares its file. The pass repeats until no
/// flag changes, so the flags reach every depth.
fn inherit_file_module_cfg(
    declarations: &mut BTreeMap<PathBuf, ModuleDeclaration>,
) -> anyhow::Result<()> {
    loop {
        let mut updates = Vec::new();
        for (child, declaration) in declarations.iter() {
            let parent_path = declaration.source_path.canonicalize()?;
            let Some(parent) = declarations.get(&parent_path) else {
                continue;
            };
            let inherited = ModuleDeclaration {
                is_test_only: declaration.is_test_only || parent.is_test_only,
                is_unix: declaration.is_unix || parent.is_unix,
                is_windows: declaration.is_windows || parent.is_windows,
                ..declaration.clone()
            };
            if inherited.is_test_only != declaration.is_test_only
                || inherited.is_unix != declaration.is_unix
                || inherited.is_windows != declaration.is_windows
            {
                updates.push((child.clone(), inherited));
            }
        }
        if updates.is_empty() {
            return Ok(());
        }
        declarations.extend(updates);
    }
}

fn module_dir(source_path: &Path) -> anyhow::Result<PathBuf> {
    let parent = source_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("source has no parent"))?;
    match source_path.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs" | "main.rs" | "mod.rs") => Ok(parent.to_path_buf()),
        Some(_) => Ok(parent.join(source_path.file_stem().unwrap_or_default())),
        None => Ok(parent.to_path_buf()),
    }
}

fn collect_file_declarations(
    items: &[Item],
    context: &FileContext,
    module_dir: &Path,
    enclosing_cfg: EnclosingCfg,
    declarations: &mut BTreeMap<PathBuf, ModuleDeclaration>,
) -> anyhow::Result<()> {
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        let declaration_cfg = enclosing_cfg.including(item_mod);
        if let Some((_, items)) = &item_mod.content {
            collect_file_declarations(
                items,
                context,
                &module_dir.join(item_mod.ident.to_string()),
                declaration_cfg,
                declarations,
            )?;
            continue;
        }
        let Some(path) = resolve_module_source(&context.source_path, module_dir, item_mod)? else {
            continue;
        };
        let canonical = path.canonicalize()?;
        declarations.insert(
            canonical,
            ModuleDeclaration {
                source_path: context.source_path.clone(),
                line: span_start_line(item_mod.span()),
                is_test_only: declaration_cfg.is_test_only,
                is_unix: declaration_cfg.is_unix,
                is_windows: declaration_cfg.is_windows,
            },
        );
    }
    Ok(())
}

fn resolve_module_source(
    declaring_source: &Path,
    module_dir: &Path,
    item_mod: &ItemMod,
) -> anyhow::Result<Option<PathBuf>> {
    for attr in &item_mod.attrs {
        if !attr.path().is_ident("path") {
            continue;
        }
        let syn::Meta::NameValue(value) = &attr.meta else {
            return Ok(None);
        };
        let syn::Expr::Lit(value) = &value.value else {
            return Ok(None);
        };
        let syn::Lit::Str(value) = &value.lit else {
            return Ok(None);
        };
        return Ok(Some(
            declaring_source
                .parent()
                .ok_or_else(|| anyhow::anyhow!("source has no parent"))?
                .join(value.value()),
        ));
    }

    let flat = module_dir.join(format!("{}.rs", item_mod.ident));
    let nested = module_dir.join(item_mod.ident.to_string()).join("mod.rs");
    match (flat.is_file(), nested.is_file()) {
        (true, false) => Ok(Some(flat)),
        (false, true) => Ok(Some(nested)),
        _ => Ok(None),
    }
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
