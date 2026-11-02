use crate::RuleId;
use sc_lint_directives::{AttributeInput, Directive, DirectiveError};
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
    error: Option<anyhow::Error>,
}

impl Allowances {
    pub(crate) fn collect(file: &syn::File) -> anyhow::Result<Self> {
        let mut result = Self::default();
        result.visit_file(file);
        if let Some(error) = result.error.take() {
            return Err(error);
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
        let mut rules = Vec::new();
        for attr in attrs {
            let path = attr.path();
            if !(path.is_ident("sc_lint")
                || (path.segments.len() == 2
                    && path.segments[0].ident == "sc_lint_attributes"
                    && path.segments[1].ident == "sc_lint"))
            {
                continue;
            }
            match attr.parse_args::<AttributeInput>() {
                Ok(input) => {
                    for directive in input.directives {
                        if let Directive::SiteAllow { rules: ids, .. } = directive {
                            for id in ids {
                                if id.as_str().starts_with("SCB-RUNTIME-") {
                                    if !crate::RUNTIME_CATALOG
                                        .rules
                                        .iter()
                                        .any(|known| known.as_str() == id.as_str())
                                    {
                                        self.error = Some(anyhow::anyhow!(
                                            "DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE: {id}; recovery: use a known runtime rule id"
                                        ));
                                    }
                                    rules.push(id.as_str().to_owned());
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    let error = DirectiveError::from_diagnostic(&error.to_string());
                    self.error = Some(anyhow::anyhow!(
                        "{} at line {}: {}; recovery: {}",
                        error.code(),
                        attr.span().start().line,
                        error.cause(),
                        error.recovery()
                    ));
                }
            }
        }
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
        self.rules(std::slice::from_ref(attr));
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
