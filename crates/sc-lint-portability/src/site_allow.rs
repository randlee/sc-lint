use std::path::{Path, PathBuf};

use sc_lint_directives::{AttributeInput, Directive, DirectiveError, SiteAllowReason, SiteRuleId};
use sc_lint_schema::AllowedFinding;
use syn::Attribute;
use syn::ImplItem;
use syn::Item;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::portability::findings::PortabilityFinding;
use crate::RuleId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SiteAllowance {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) rules: Vec<SiteRuleId>,
    pub(super) reason: SiteAllowReason,
    pub(super) item: String,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum SiteAllowError {
    #[error("{path}:{line}: {error}")]
    InvalidDirective {
        path: PathBuf,
        line: usize,
        error: DirectiveError,
    },
    #[error("{path}:{line}: unsupported portability rule `{rule}`")]
    UnknownRule {
        path: PathBuf,
        line: usize,
        rule: SiteRuleId,
    },
}

pub(super) fn collect_site_allowances(
    file: &syn::File,
    source_path: &Path,
) -> Result<Vec<SiteAllowance>, SiteAllowError> {
    let mut visitor = SiteAllowVisitor {
        path: source_path,
        module_path: source_file_module_path(source_path),
        allowances: Vec::new(),
        error: None,
    };
    visitor.visit_file(file);
    match visitor.error {
        Some(error) => Err(error),
        None => Ok(visitor.allowances),
    }
}

struct SiteAllowVisitor<'a> {
    path: &'a Path,
    module_path: Vec<String>,
    allowances: Vec<SiteAllowance>,
    error: Option<SiteAllowError>,
}

impl SiteAllowVisitor<'_> {
    fn visit_attrs(&mut self, attrs: &[Attribute], item_line: usize, item_path: &str) {
        for attr in attrs {
            if !is_sc_lint_attribute(attr) {
                continue;
            }
            let line = attr.span().start().line;
            let parsed = match attr.parse_args::<AttributeInput>() {
                Ok(parsed) => parsed,
                Err(error) => {
                    // Boundary directives retain their existing diagnostics; only site syntax
                    // is owned by this portability-specific validation pass.
                    if matches!(&attr.meta, syn::Meta::List(list) if list.tokens.to_string().starts_with("boundary."))
                    {
                        continue;
                    }
                    self.fail(line, DirectiveError::from_diagnostic(&error.to_string()));
                    return;
                }
            };
            for directive in parsed.directives {
                let Directive::SiteAllow { rules, reason } = directive else {
                    continue;
                };
                for rule in &rules {
                    if rule.as_str().starts_with("PORT-") && rule.as_str() != "PORT-010" {
                        self.fail_unknown_rule(line, rule.clone());
                        return;
                    }
                }
                self.allowances.push(SiteAllowance {
                    path: self.path.to_path_buf(),
                    line: item_line,
                    rules,
                    reason,
                    item: item_path.to_string(),
                });
            }
        }
    }

    fn fail(&mut self, line: usize, error: DirectiveError) {
        let error = SiteAllowError::InvalidDirective {
            path: self.path.to_path_buf(),
            line,
            error,
        };
        // The visitor preserves the first source-order error.
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn fail_unknown_rule(&mut self, line: usize, rule: SiteRuleId) {
        if self.error.is_none() {
            self.error = Some(SiteAllowError::UnknownRule {
                path: self.path.to_path_buf(),
                line,
                rule,
            });
        }
    }
}

impl<'ast> Visit<'ast> for SiteAllowVisitor<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.error.is_some() {
            return;
        }
        match item {
            Item::Mod(module) => {
                let item_path = qualified_item(&self.module_path, module.ident.to_string());
                self.visit_attrs(&module.attrs, module.span().start().line, &item_path);
                if let Some((_, items)) = &module.content {
                    self.module_path.push(module.ident.to_string());
                    for child in items {
                        self.visit_item(child);
                    }
                    self.module_path.pop();
                }
            }
            Item::Impl(item_impl) => {
                let type_name = item_type_name(&item_impl.self_ty);
                let item_path = qualified_item(&self.module_path, type_name.clone());
                self.visit_attrs(&item_impl.attrs, item_impl.span().start().line, &item_path);
                self.module_path.push(type_name);
                for child in &item_impl.items {
                    if item_impl.trait_.is_none() {
                        self.visit_impl_item(child);
                    } else if let ImplItem::Fn(method) = child {
                        self.visit_block(&method.block);
                    }
                }
                self.module_path.pop();
            }
            _ => {
                let name = sc_lint_analyzer_support::item_identifier(item)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "item".to_string());
                let item_path = qualified_item(&self.module_path, name);
                self.visit_attrs(item.attrs(), item.span().start().line, &item_path);
                syn::visit::visit_item(self, item);
            }
        }
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if self.error.is_some() {
            return;
        }
        if let ImplItem::Fn(method) = item {
            self.visit_attrs(
                &method.attrs,
                method.span().start().line,
                &qualified_item(&self.module_path, method.sig.ident.to_string()),
            );
            self.visit_block(&method.block);
        }
    }
}

pub(super) fn qualified_item(parent: &[String], item: String) -> String {
    parent
        .iter()
        .cloned()
        .chain(std::iter::once(item))
        .collect::<Vec<_>>()
        .join("::")
}

pub(super) fn item_type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "impl".to_string()),
        _ => "impl".to_string(),
    }
}

pub(super) fn source_file_module_path(path: &Path) -> Vec<String> {
    let components = path.components().collect::<Vec<_>>();
    let Some(src_index) = components
        .iter()
        .rposition(|component| component.as_os_str() == "src")
    else {
        return Vec::new();
    };
    let tail = &components[src_index + 1..];
    if tail.is_empty() {
        return Vec::new();
    }
    let mut modules = tail[..tail.len() - 1]
        .iter()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let file = tail[tail.len() - 1].as_os_str().to_string_lossy();
    if matches!(file.as_ref(), "lib.rs" | "main.rs" | "build.rs" | "mod.rs") {
        return modules;
    }
    let stem = file.strip_suffix(".rs").unwrap_or(&file).to_string();
    if matches!(
        modules.first().map(String::as_str),
        Some("bin" | "examples" | "benches" | "tests")
    ) {
        modules.clear();
    } else {
        modules.push(stem);
    }
    modules
}

fn is_sc_lint_attribute(attr: &Attribute) -> bool {
    let segments = &attr.path().segments;
    segments
        .last()
        .is_some_and(|segment| segment.ident == "sc_lint")
        && (segments.len() == 1
            || (segments.len() == 2 && segments[0].ident == "sc_lint_attributes"))
}

pub(super) fn apply_site_allowances(
    findings: Vec<PortabilityFinding>,
    allowances: Vec<SiteAllowance>,
) -> (Vec<PortabilityFinding>, Vec<AllowedFinding<RuleId>>) {
    let mut remaining = Vec::with_capacity(findings.len());
    let mut allowed = Vec::new();
    for finding in findings {
        let rule = finding.rule_id.as_str();
        let allowance = allowances.iter().find(|allowance| {
            allowance.path == finding.source_path
                && allowance.line == finding.line
                && finding.item.as_deref() == Some(allowance.item.as_str())
                && allowance
                    .rules
                    .iter()
                    .any(|allowed_rule| allowed_rule.as_str() == rule)
        });
        if let Some(allowance) = allowance {
            allowed.push(
                AllowedFinding::new(
                    finding.rule_id,
                    allowance.reason.as_str(),
                    allowance.item.clone(),
                    allowance.path.to_string_lossy().into_owned(),
                    finding.line,
                )
                .expect("collected site allowances and findings satisfy AllowedFinding invariants"),
            );
        } else {
            remaining.push(finding);
        }
    }
    (remaining, allowed)
}

trait ItemAttrs {
    fn attrs(&self) -> &[Attribute];
}

impl ItemAttrs for Item {
    fn attrs(&self) -> &[Attribute] {
        match self {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::ExternCrate(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Macro(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            _ => &[],
        }
    }
}
