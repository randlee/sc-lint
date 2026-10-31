use std::path::{Path, PathBuf};

use sc_lint_directives::{AttributeInput, Directive};
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
    pub(super) rules: Vec<String>,
    pub(super) reason: String,
    pub(super) item_label: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("{path}:{line}: {message}")]
pub(super) struct SiteAllowError {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) message: String,
}

pub(super) fn collect_site_allowances(
    file: &syn::File,
    source_path: &Path,
) -> Result<Vec<SiteAllowance>, SiteAllowError> {
    let mut visitor = SiteAllowVisitor {
        path: source_path,
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
    allowances: Vec<SiteAllowance>,
    error: Option<SiteAllowError>,
}

impl SiteAllowVisitor<'_> {
    fn visit_attrs(&mut self, attrs: &[Attribute], item_line: usize, item_label: Option<String>) {
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
                    self.fail(line, error.to_string());
                    return;
                }
            };
            for directive in parsed.directives {
                let Directive::SiteAllow { rules, reason } = directive else {
                    continue;
                };
                for rule in &rules {
                    if rule.starts_with("PORT-") && !is_known_portability_rule(rule) {
                        self.fail(line, format!("unknown portability rule id `{rule}`"));
                        return;
                    }
                }
                self.allowances.push(SiteAllowance {
                    path: self.path.to_path_buf(),
                    line: item_line,
                    rules,
                    reason,
                    item_label: item_label.clone(),
                });
            }
        }
    }

    fn fail(&mut self, line: usize, message: String) {
        let error = SiteAllowError {
            path: self.path.to_path_buf(),
            line,
            message,
        };
        // The visitor preserves the first source-order error.
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

impl<'ast> Visit<'ast> for SiteAllowVisitor<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.error.is_some() {
            return;
        }
        self.visit_attrs(
            item.attrs(),
            item.span().start().line,
            sc_lint_analyzer_support::item_identifier(item).map(ToString::to_string),
        );
        syn::visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if self.error.is_some() {
            return;
        }
        if let ImplItem::Fn(method) = item {
            self.visit_attrs(
                &method.attrs,
                method.span().start().line,
                Some(method.sig.ident.to_string()),
            );
        }
        syn::visit::visit_impl_item(self, item);
    }
}

fn is_sc_lint_attribute(attr: &Attribute) -> bool {
    let segments = &attr.path().segments;
    segments
        .last()
        .is_some_and(|segment| segment.ident == "sc_lint")
        && (segments.len() == 1
            || (segments.len() == 2 && segments[0].ident == "sc_lint_attributes"))
}

fn is_known_portability_rule(rule: &str) -> bool {
    matches!(
        rule,
        "PORT-001"
            | "PORT-002"
            | "PORT-003"
            | "PORT-004"
            | "PORT-005"
            | "PORT-006"
            | "PORT-007"
            | "PORT-008"
            | "PORT-009"
            | "PORT-010"
    )
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
                && allowance
                    .item_label
                    .as_ref()
                    .is_some_and(|label| finding.message.contains(&format!("item `{label}`")))
                && allowance
                    .rules
                    .iter()
                    .any(|allowed_rule| allowed_rule == rule)
        });
        if let Some(allowance) = allowance {
            allowed.push(AllowedFinding {
                rule_id: finding.rule_id,
                reason: allowance.reason.clone(),
                source_path: allowance.path.to_string_lossy().into_owned(),
                line: finding.line,
                owner_ids: vec![sc_lint_schema::OwnerId::new(crate::CrateId::from_parts(
                    finding.package.as_str(),
                    finding.target.as_str(),
                ))],
                node_ids: vec![sc_lint_schema::NodeId::new(finding.node_label)],
            });
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
