use crate::RuleId;
use sc_lint_directives::{
    AttributeInput, Directive, DirectiveError, is_sc_lint_attribute, unknown_owned_rule,
};
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::visit::Visit;

type Position = (usize, usize);
struct Entry {
    start: Position,
    end: Position,
    rules: Vec<String>,
}
#[derive(Default)]
pub(crate) struct Allowances {
    entries: Vec<Entry>,
    parsed: BTreeMap<Position, Vec<String>>,
    errors: Vec<(Position, String)>,
}

impl Allowances {
    pub(crate) fn collect(file: &syn::File) -> anyhow::Result<Self> {
        let mut result = Self::default();
        result.visit_file(file);
        if !result.errors.is_empty() {
            result.errors.sort_by_key(|(position, _)| *position);
            return Err(anyhow::anyhow!(
                result
                    .errors
                    .iter()
                    .map(|(_, error)| error.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        Ok(result)
    }
    pub(crate) fn allows(&self, rule: RuleId, line: usize, column: usize) -> bool {
        let point = (line, column);
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.start <= point && point < entry.end)
            .is_some_and(|entry| entry.rules.iter().any(|id| id == rule.as_str()))
    }
    fn rules(&mut self, attrs: &[syn::Attribute]) -> Vec<String> {
        attrs
            .iter()
            .flat_map(|attr| self.attribute_rules(attr))
            .collect()
    }
    fn attribute_rules(&mut self, attr: &syn::Attribute) -> Vec<String> {
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
                        let known = crate::RUNTIME_CATALOG
                            .rules
                            .iter()
                            .map(|rule| rule.as_str())
                            .collect::<Vec<_>>();
                        if let Some(rule) = unknown_owned_rule(&ids, "SCB-RUNTIME-", &known) {
                            self.errors.push((position, format!("DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE at line {}: {}; recovery: use a known runtime rule id", start.line, rule)));
                        }
                        rules.extend(
                            ids.iter()
                                .filter(|rule| rule.as_str().starts_with("SCB-RUNTIME-"))
                                .map(|rule| rule.as_str().to_owned()),
                        );
                    }
                }
            }
            Err(error) => {
                let error = DirectiveError::from_diagnostic(&error.to_string());
                self.errors.push((
                    position,
                    format!(
                        "{} at line {}: {}; recovery: {}",
                        error.code(),
                        start.line,
                        error.cause(),
                        error.recovery()
                    ),
                ));
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
