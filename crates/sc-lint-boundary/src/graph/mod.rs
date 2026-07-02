use super::*;
use cargo_metadata::MetadataCommand;

mod build;
mod reference_collector;

pub(crate) use build::build_workspace_graph;

pub(crate) fn node_has_allow_rule(node: &GraphNode, rule_id: &str) -> bool {
    node.attributes.iter().any(|attr| {
        attr.scope == "boundary"
            && attr.name == "allow"
            && attr.values.iter().any(|value| value == rule_id)
    })
}

fn parse_lint_attributes(attrs: &[Attribute]) -> Result<Vec<LintAttribute>> {
    let mut parsed = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("sc_lint") {
            continue;
        }
        let input = attr.parse_args::<AttributeInput>()?;
        for directive in input.directives {
            match directive {
                Directive::Allow(values) => {
                    parsed.push(LintAttribute {
                        scope: "boundary",
                        name: "allow",
                        values,
                    });
                }
                Directive::InternalOnly => {
                    parsed.push(LintAttribute {
                        scope: "boundary",
                        name: "internal_only",
                        values: Vec::new(),
                    });
                }
                Directive::ForbidExternalImpls => {
                    parsed.push(LintAttribute {
                        scope: "boundary",
                        name: "forbid_external_impls",
                        values: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(parsed)
}

fn resolve_module_source(
    declaring_source_path: &Path,
    module_dir: &Path,
    module_name: &str,
    attrs: &[Attribute],
) -> Result<PathBuf> {
    if let Some(explicit_path) = explicit_module_source(declaring_source_path, attrs)? {
        if explicit_path.is_file() {
            return Ok(explicit_path);
        }
        anyhow::bail!(
            "module `{module_name}` path attribute resolved to missing file {}",
            explicit_path.display()
        );
    }

    let flat = module_dir.join(format!("{module_name}.rs"));
    let nested = module_dir.join(module_name).join("mod.rs");

    let flat_exists = flat.is_file();
    let nested_exists = nested.is_file();

    match (flat_exists, nested_exists) {
        (true, false) => Ok(flat),
        (false, true) => Ok(nested),
        (true, true) => anyhow::bail!(
            "ambiguous module `{module_name}`: found both {} and {}",
            flat.display(),
            nested.display()
        ),
        (false, false) => anyhow::bail!(
            "module `{module_name}` not found; expected {} or {}",
            flat.display(),
            nested.display()
        ),
    }
}

fn has_explicit_module_path(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("path"))
}

fn explicit_module_source(
    declaring_source_path: &Path,
    attrs: &[Attribute],
) -> Result<Option<PathBuf>> {
    for attr in attrs {
        if !attr.path().is_ident("path") {
            continue;
        }

        match &attr.meta {
            syn::Meta::NameValue(name_value) => match &name_value.value {
                syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Str(lit) => {
                        let declaring_dir = declaring_source_path.parent().ok_or_else(|| {
                            anyhow::anyhow!(
                                "declaring source path has no parent: {}",
                                declaring_source_path.display()
                            )
                        })?;
                        // Absolute #[path = "..."] values intentionally bypass the
                        // declaring source directory because PathBuf::join preserves
                        // an absolute right-hand operand unchanged.
                        return Ok(Some(declaring_dir.join(lit.value())));
                    }
                    _ => anyhow::bail!(
                        "path attribute must use a string literal: {}",
                        attr.to_token_stream()
                    ),
                },
                _ => anyhow::bail!(
                    "path attribute must use a string literal: {}",
                    attr.to_token_stream()
                ),
            },
            _ => anyhow::bail!(
                "unsupported path attribute syntax: {}",
                attr.to_token_stream()
            ),
        }
    }

    Ok(None)
}

fn parse_rust_file(path: &Path) -> Result<File> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read Rust source {}", path.display()))?;
    syn::parse_file(&source)
        .with_context(|| format!("failed to parse Rust source {}", path.display()))
}

fn impl_owner_name(self_ty: &Type) -> Result<String> {
    match self_ty {
        Type::Path(type_path) => {
            // `syn::Type::Path` always stores at least one segment for a valid path type.
            if let Some(segment) = type_path.path.segments.last() {
                Ok(segment.ident.to_string())
            } else {
                Err(anyhow::anyhow!(
                    "impl owner path is missing a terminal segment"
                ))
            }
        }
        _ => anyhow::bail!(
            "unsupported impl owner type `{}`; only path owners are supported",
            self_ty.to_token_stream()
        ),
    }
}

pub(crate) fn trait_path_key(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

pub(crate) fn trait_terminal_name(trait_path: &str) -> &str {
    trait_path.rsplit("::").next().unwrap_or(trait_path)
}

pub(crate) fn default_rule_defaults() -> &'static RuleDefaults {
    static DEFAULTS: OnceLock<RuleDefaults> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        toml::from_str(DEFAULT_RULES_TOML)
            .expect("embedded sc-lint-boundary default rule config must parse")
    })
}

pub(crate) fn is_supported_target(target: &cargo_metadata::Target) -> bool {
    target.kind.iter().any(|kind| {
        matches!(
            kind,
            cargo_metadata::TargetKind::Lib
                | cargo_metadata::TargetKind::Bin
                | cargo_metadata::TargetKind::Example
        )
    })
}

pub(crate) fn crate_id(package_name: &str, target_name: &str) -> CrateId {
    CrateId::from_parts(package_name, target_name)
}

pub(crate) fn load_metadata(root: &Path) -> Result<cargo_metadata::Metadata> {
    MetadataCommand::new()
        .current_dir(root)
        .exec()
        .with_context(|| format!("failed to load cargo metadata for {}", root.display()))
}
