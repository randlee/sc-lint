use super::*;

pub(super) fn parse_rust_file(path: &Path) -> Result<File> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read Rust source {}", path.display()))?;
    syn::parse_file(&source)
        .with_context(|| format!("failed to parse Rust source {}", path.display()))
}

pub(super) fn impl_owner_name(self_ty: &Type) -> Result<String> {
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
