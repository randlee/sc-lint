use syn::File;
use syn::ImplItem;
use syn::Item;
use syn::ItemImpl;
use syn::spanned::Spanned;

use super::PortabilityConfig;
use super::findings::PortabilityFinding;
use crate::RuleId;
use crate::platform_adapters::PlatformAdapters;
use crate::predicates::attr_is_cfg_unix;
use crate::predicates::attr_is_cfg_windows;
use crate::predicates::attr_is_exact_cfg_unix;
use crate::predicates::attr_is_platform_cfg;
use crate::predicates::has_not_unix_companion;
use crate::predicates::has_portable_fallback;
use crate::predicates::has_windows_companion;
use crate::predicates::impl_method_has_not_unix_companion;
use crate::predicates::impl_method_has_portable_fallback;
use crate::predicates::impl_method_has_windows_companion;
use crate::predicates::impl_method_is_cfg_unix_production_item;
use crate::predicates::is_cfg_unix_production_item;
use crate::predicates::item_is_private;
use crate::site_allow::{item_type_name, qualified_item, source_file_module_path};
use sc_lint_analyzer_support::FileContext;
use sc_lint_analyzer_support::ScopeKind;
use sc_lint_analyzer_support::classify_scope;
use sc_lint_analyzer_support::item_attrs;
use sc_lint_analyzer_support::item_identifier;
use sc_lint_analyzer_support::item_name_hint_is_tests;
use sc_lint_analyzer_support::span_start_line;

#[derive(Clone, Copy)]
pub(super) struct ParityContext<'a> {
    config: &'a PortabilityConfig,
    platform_adapters: Option<&'a PlatformAdapters>,
    file_is_unix_only: bool,
}

impl<'a> ParityContext<'a> {
    fn configured(
        config: &'a PortabilityConfig,
        platform_adapters: &'a PlatformAdapters,
        file_is_unix_only: bool,
    ) -> Self {
        Self {
            config,
            platform_adapters: Some(platform_adapters),
            file_is_unix_only,
        }
    }

    pub(super) fn without_adapters(config: &'a PortabilityConfig) -> Self {
        Self {
            config,
            platform_adapters: None,
            file_is_unix_only: false,
        }
    }
}

pub(super) fn collect_unix_portability_findings(
    file: &File,
    file_context: &FileContext,
    config: &PortabilityConfig,
    platform_adapters: &PlatformAdapters,
    file_is_unix_only: bool,
) -> Vec<PortabilityFinding> {
    let initial_scope = if file_context.is_test_file {
        ScopeKind::Test
    } else {
        ScopeKind::NonTest
    };
    let mut findings = Vec::new();
    let mut visitor = ParityVisitor {
        file_context,
        findings: &mut findings,
        context: ParityContext::configured(config, platform_adapters, file_is_unix_only),
        module_path: source_file_module_path(&file_context.source_path),
    };
    visitor.visit_items(&file.items, initial_scope, false, false);
    findings
}

pub(super) fn visit_item_for_unix_portability(
    item: &Item,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    inherited_platform_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    context: ParityContext<'_>,
) {
    let mut visitor = ParityVisitor {
        file_context,
        findings,
        context,
        module_path: source_file_module_path(&file_context.source_path),
    };
    visitor.visit_item(
        item,
        inherited_scope,
        inherited_unix_gated,
        inherited_platform_gated,
    );
}

struct ParityVisitor<'a> {
    file_context: &'a FileContext,
    findings: &'a mut Vec<PortabilityFinding>,
    context: ParityContext<'a>,
    module_path: Vec<String>,
}

impl ParityVisitor<'_> {
    fn visit_items(
        &mut self,
        items: &[Item],
        inherited_scope: ScopeKind,
        inherited_unix_gated: bool,
        inherited_platform_gated: bool,
    ) {
        self.collect_parity_findings(items, inherited_scope, inherited_unix_gated);
        for item in items {
            self.visit_item(
                item,
                inherited_scope,
                inherited_unix_gated,
                inherited_platform_gated,
            );
        }
    }

    fn visit_item(
        &mut self,
        item: &Item,
        inherited_scope: ScopeKind,
        inherited_unix_gated: bool,
        inherited_platform_gated: bool,
    ) {
        let attrs = item_attrs(item);
        let scope = classify_scope(attrs, inherited_scope, item_name_hint_is_tests(item));
        let unix_gated = inherited_unix_gated || attrs.iter().any(attr_is_cfg_unix);
        let platform_gated = inherited_platform_gated || attrs.iter().any(attr_is_platform_cfg);
        self.visit_item_checks(item, attrs, scope, unix_gated, platform_gated);
        if let Item::Mod(item_mod) = item
            && let Some((_, items)) = &item_mod.content
        {
            self.module_path.push(item_mod.ident.to_string());
            self.visit_items(items, scope, unix_gated, platform_gated);
            self.module_path.pop();
        }
    }

    fn visit_item_checks(
        &mut self,
        item: &Item,
        attrs: &[syn::Attribute],
        scope: ScopeKind,
        unix_gated: bool,
        platform_gated: bool,
    ) {
        if scope == ScopeKind::NonTest
            && !unix_gated
            && let Item::Impl(item_impl) = item
            && !attrs.iter().any(attr_is_cfg_windows)
        {
            collect_impl_method_parity_findings(
                item_impl,
                scope,
                self.file_context,
                self.findings,
                self.context,
                &self.module_path,
            );
        }
        if scope == ScopeKind::NonTest && !platform_gated {
            super::collect_production_path_literal_findings(
                item,
                self.file_context,
                self.findings,
                self.context.config,
            );
        }
        self.visit_suppressor_attributes(attrs, scope);
        self.visit_unix_import(item, scope, unix_gated);
    }

    fn visit_suppressor_attributes(&mut self, attrs: &[syn::Attribute], scope: ScopeKind) {
        if scope != ScopeKind::NonTest {
            return;
        }
        for attr in attrs {
            if crate::predicates::attr_is_cfg_attr_not_unix_allow_dead_code(attr) {
                self.findings.push(PortabilityFinding {
                    rule_id: RuleId::Port005,
                    kind: "cfg_attr_not_unix_allow_dead_code",
                    message: "PORT-005 #[cfg_attr(not(unix), allow(dead_code))] is not an approved portability suppressor in production code; gate the item with #[cfg(unix)] or provide a real cross-platform implementation".to_string(),
                    source_path: self.file_context.source_path.clone(),
                    line: span_start_line(attr.span()),
                    item: None,
                    package: self.file_context.package.clone(),
                    target: self.file_context.target.clone(),
                    node_label: format!("crate::{}::{}::portability", self.file_context.package, self.file_context.target),
                });
            }
        }
    }

    fn visit_unix_import(&mut self, item: &Item, scope: ScopeKind, unix_gated: bool) {
        if scope != ScopeKind::NonTest || unix_gated {
            return;
        }
        if let Item::Use(item_use) = item
            && crate::predicates::use_tree_contains_std_os_unix(&item_use.tree)
        {
            self.findings.push(PortabilityFinding {
                rule_id: RuleId::Port004,
                kind: "ungated_std_os_unix_import",
                message: "PORT-004 ungated std::os::unix import in production code; wrap the item with #[cfg(unix)] or move the import behind a Unix-only boundary".to_string(),
                source_path: self.file_context.source_path.clone(),
                line: span_start_line(item_use.span()),
                item: None,
                package: self.file_context.package.clone(),
                target: self.file_context.target.clone(),
                node_label: format!("crate::{}::{}::portability", self.file_context.package, self.file_context.target),
            });
        }
    }

    fn collect_parity_findings(
        &mut self,
        items: &[Item],
        inherited_scope: ScopeKind,
        inherited_unix_gated: bool,
    ) {
        if inherited_scope != ScopeKind::NonTest
            || inherited_unix_gated
            || self.context.file_is_unix_only
        {
            return;
        }
        for item in items {
            let line = span_start_line(item.span());
            if !self.should_report_item(item, items, inherited_scope, line) {
                continue;
            }
            let detail = item_identifier(item)
                .map(ToString::to_string)
                .unwrap_or_else(|| "cfg(unix) item".to_string());
            let item_path = item_identifier(item)
                .map(ToString::to_string)
                .map(|name| qualified_item(&self.module_path, name))
                .unwrap_or_else(|| qualified_item(&self.module_path, detail.clone()));
            push_cfg_parity_finding(self.findings, self.file_context, line, &detail, item_path);
        }
    }

    fn should_report_item(
        &self,
        item: &Item,
        siblings: &[Item],
        scope: ScopeKind,
        line: usize,
    ) -> bool {
        let private_adapter_item = item_is_private(item)
            && self
                .context
                .platform_adapters
                .is_some_and(|adapters| adapters.is_adapter_file(&self.file_context.source_path));
        let configured_exemption = self.context.platform_adapters.is_some_and(|adapters| {
            adapters.suppresses_port010(&self.file_context.source_path, line)
        });
        is_cfg_unix_production_item(item, scope)
            && !private_adapter_item
            && !configured_exemption
            && !has_windows_companion(item, siblings)
            && !has_not_unix_companion(item, siblings)
            && !matches!(item, Item::Mod(module)
                if module.content.is_none() && module.attrs.iter().any(attr_is_exact_cfg_unix))
            && !has_portable_fallback(item, siblings, scope)
    }
}

fn push_cfg_parity_finding(
    findings: &mut Vec<PortabilityFinding>,
    file_context: &FileContext,
    line: usize,
    detail: &str,
    item_path: String,
) {
    findings.push(PortabilityFinding {
        rule_id: RuleId::Port010,
        kind: "cfg_unix_parity_missing",
        message: format!(
            "PORT-010 production #[cfg(unix)] item `{detail}` has no #[cfg(windows)] companion or explicit portable fallback in the same scope; consider adding a cfg(windows) sibling or an unguarded portable fallback"
        ),
        source_path: file_context.source_path.clone(),
        line,
        item: Some(item_path),
        package: file_context.package.clone(),
        target: file_context.target.clone(),
        node_label: format!("crate::{}::{}::portability", file_context.package, file_context.target),
    });
}

pub(super) fn collect_impl_method_parity_findings(
    item_impl: &ItemImpl,
    inherited_scope: ScopeKind,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    context: ParityContext<'_>,
    module_path: &[String],
) {
    if inherited_scope != ScopeKind::NonTest || context.file_is_unix_only {
        return;
    }
    for impl_item in &item_impl.items {
        let ImplItem::Fn(unix_method) = impl_item else {
            continue;
        };
        let private_adapter_method = item_impl.trait_.is_none()
            && matches!(unix_method.vis, syn::Visibility::Inherited)
            && context
                .platform_adapters
                .is_some_and(|adapters| adapters.is_adapter_file(&file_context.source_path));
        if !impl_method_is_cfg_unix_production_item(unix_method, inherited_scope)
            || private_adapter_method
            || impl_method_has_windows_companion(unix_method, &item_impl.items)
            || (item_impl.trait_.is_none()
                && impl_method_has_not_unix_companion(unix_method, &item_impl.items))
            || impl_method_has_portable_fallback(unix_method, &item_impl.items, inherited_scope)
        {
            continue;
        }
        push_cfg_parity_finding(
            findings,
            file_context,
            span_start_line(unix_method.span()),
            &unix_method.sig.ident.to_string(),
            qualified_item(
                module_path,
                format!(
                    "{}::{}",
                    item_type_name(&item_impl.self_ty),
                    unix_method.sig.ident
                ),
            ),
        );
    }
}
