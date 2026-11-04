use crate::RuleId;
use sc_lint_directives::{
    AttributeInput, Directive, DirectiveError, is_sc_lint_attribute, unknown_owned_rule,
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;
use syn::spanned::Spanned;
use syn::visit::Visit;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{code} at line {line}: {cause}; recovery: {recovery}", cause = .directive.cause(), recovery = .directive.recovery())]
pub(crate) struct AllowanceDiagnostic {
    pub(crate) code: &'static str,
    pub(crate) line: usize,
    pub(crate) column: usize,
    #[source]
    pub(crate) directive: DirectiveError,
}

#[derive(Debug)]
pub(crate) struct AllowanceErrors {
    pub(crate) diagnostics: Vec<AllowanceDiagnostic>,
}
impl fmt::Display for AllowanceErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index != 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for AllowanceErrors {}

fn known_rules() -> &'static [&'static str] {
    static KNOWN: OnceLock<Vec<&'static str>> = OnceLock::new();
    KNOWN.get_or_init(|| {
        crate::RUNTIME_CATALOG
            .rules
            .iter()
            .map(|rule| rule.as_str())
            .collect()
    })
}

type Position = (usize, usize);
struct Entry {
    start: Position,
    end: Position,
    rules: Vec<RuleId>,
}
#[derive(Default)]
pub(crate) struct Allowances {
    entries: Vec<Entry>,
    parsed: BTreeMap<Position, Vec<RuleId>>,
    errors: Vec<AllowanceDiagnostic>,
}

impl Allowances {
    pub(crate) fn collect(file: &syn::File) -> Result<Self, AllowanceErrors> {
        let mut result = Self::default();
        result.visit_file(file);
        if !result.errors.is_empty() {
            result
                .errors
                .sort_by_key(|error| (error.line, error.column));
            return Err(AllowanceErrors {
                diagnostics: result.errors,
            });
        }
        Ok(result)
    }
    fn diagnostic(&mut self, position: Position, directive: DirectiveError) {
        self.errors.push(AllowanceDiagnostic {
            code: directive.code(),
            line: position.0,
            column: position.1,
            directive,
        });
    }
    pub(crate) fn allows(&self, rule: RuleId, line: usize, column: usize) -> bool {
        let point = (line, column);
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.start <= point && point < entry.end)
            .is_some_and(|entry| entry.rules.contains(&rule))
    }
    fn rules(&mut self, attrs: &[syn::Attribute]) -> Vec<RuleId> {
        attrs
            .iter()
            .flat_map(|attr| self.attribute_rules(attr))
            .collect()
    }
    fn attribute_rules(&mut self, attr: &syn::Attribute) -> Vec<RuleId> {
        if !is_sc_lint_attribute(attr) {
            return Vec::new();
        }
        let start = attr.span().start();
        let position = (start.line, start.column);
        if let Some(rules) = self.parsed.get(&position) {
            return rules.clone();
        }
        let mut rules = Vec::new();
        match attr.parse_args::<AttributeInput>() {
            Ok(input) => {
                for directive in input.directives {
                    if let Directive::SiteAllow { rules: ids, .. } = directive {
                        if let Some(rule) = unknown_owned_rule(&ids, "SCB-RUNTIME-", known_rules())
                        {
                            self.diagnostic(
                                position,
                                DirectiveError::new(
                                    "DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE",
                                    rule.as_str(),
                                    "use a known runtime rule id",
                                ),
                            );
                        }
                        rules.extend(
                            ids.iter()
                                .filter_map(|rule| RuleId::from_known(rule.as_str())),
                        );
                    }
                }
            }
            Err(error) => {
                let error = DirectiveError::from_diagnostic(&error.to_string());
                self.diagnostic(position, error);
            }
        }
        self.parsed.insert(position, rules.clone());
        rules
    }
    fn function(&mut self, attrs: &[syn::Attribute], block: &syn::Block) {
        let rules = self.rules(attrs);
        let start = block.span().start();
        let end = block.span().end();
        self.entries.push(Entry {
            start: (start.line, start.column),
            end: (end.line, end.column),
            rules,
        });
    }
}

impl<'ast> Visit<'ast> for Allowances {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        self.attribute_rules(attr);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if super::production(&item.attrs, Some(item.ident == "tests")) {
            syn::visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_impl(self, item);
        }
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_trait(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !super::production(&item.attrs, None) {
            return;
        }
        self.function(&item.attrs, &item.block);
        syn::visit::visit_item_fn(self, item);
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !super::production(&item.attrs, None) {
            return;
        }
        self.function(&item.attrs, &item.block);
        syn::visit::visit_impl_item_fn(self, item);
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !super::production(&item.attrs, None) {
            return;
        }
        if let Some(block) = &item.default {
            self.function(&item.attrs, block);
        }
        syn::visit::visit_trait_item_fn(self, item);
    }
}
