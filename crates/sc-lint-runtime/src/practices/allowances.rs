use crate::position::SourcePosition;
use crate::{KNOWN_RULES, RuleId};
use sc_lint_directives::{
    AttributeInput, Directive, DirectiveCode, DirectiveError, is_sc_lint_attribute,
    unknown_owned_rules, unknown_rule,
};
use std::collections::BTreeMap;
use std::fmt;
use syn::visit::Visit;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{code} at line {line}: {cause}; recovery: {recovery}", code = .directive.code(), line = .position.line, cause = .directive.cause(), recovery = .directive.recovery())]
pub(crate) struct AllowanceDiagnostic {
    pub(crate) position: SourcePosition,
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

struct Entry {
    start: SourcePosition,
    end: SourcePosition,
    rules: Vec<RuleId>,
}
#[derive(Default)]
pub(crate) struct Allowances {
    entries: Vec<Entry>,
    parsed: BTreeMap<SourcePosition, Vec<RuleId>>,
    errors: Vec<AllowanceDiagnostic>,
}

impl Allowances {
    pub(crate) fn collect(file: &syn::File) -> Result<Self, AllowanceErrors> {
        let mut result = Self::default();
        result.visit_file(file);
        if !result.errors.is_empty() {
            result.errors.sort_by_key(|error| error.position);
            return Err(AllowanceErrors {
                diagnostics: result.errors,
            });
        }
        Ok(result)
    }
    fn diagnostic(&mut self, position: SourcePosition, directive: DirectiveError) {
        self.errors.push(AllowanceDiagnostic {
            position,
            directive,
        });
    }
    pub(crate) fn allows(&self, rule: RuleId, point: SourcePosition) -> bool {
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
        let position = SourcePosition::start(attr);
        if let Some(rules) = self.parsed.get(&position) {
            return rules.clone();
        }
        let mut rules = Vec::new();
        match AttributeInput::parse_attribute(attr) {
            Ok(input) => {
                for directive in input.directives {
                    if let Directive::SiteAllow { rules: ids, .. } = directive {
                        rules.extend(self.site_allow_rules(&ids, position));
                    }
                }
            }
            Err(error) => {
                self.diagnostic(position, error);
            }
        }
        self.parsed.insert(position, rules.clone());
        rules
    }
    fn site_allow_rules(
        &mut self,
        ids: &[sc_lint_directives::SiteRuleId],
        position: SourcePosition,
    ) -> Vec<RuleId> {
        for rule in unknown_owned_rules(ids, "SCB-RUNTIME-", KNOWN_RULES) {
            self.diagnostic(position, unknown_rule(rule, KNOWN_RULES));
        }
        ids.iter()
            .filter(|id| KNOWN_RULES.contains(&id.as_str()))
            .map(|id| {
                RuleId::from_known(id.as_str())
                    .expect("runtime catalog and typed rule inventory agree")
            })
            .collect()
    }
    fn function(&mut self, attrs: &[syn::Attribute], block: &syn::Block) {
        let rules = self.rules(attrs);

        self.entries.push(Entry {
            start: SourcePosition::start(block),
            end: SourcePosition::end(block),
            rules,
        });
    }
}

impl<'ast> Visit<'ast> for Allowances {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if is_sc_lint_attribute(attr) && !self.parsed.contains_key(&SourcePosition::start(attr)) {
            let rules = self.attribute_rules(attr);
            if !rules.is_empty() {
                self.diagnostic(
                    SourcePosition::start(attr),
                    DirectiveError::typed(
                        DirectiveCode::UnsupportedItem,
                        "runtime allowances require a function or method",
                    ),
                );
            }
        }
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
