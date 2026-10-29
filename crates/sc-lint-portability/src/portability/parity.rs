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
use crate::predicates::attr_is_platform_cfg;
use crate::predicates::has_portable_fallback;
use crate::predicates::has_windows_companion;
use crate::predicates::impl_method_has_portable_fallback;
use crate::predicates::impl_method_has_windows_companion;
use crate::predicates::impl_method_is_cfg_unix_production_item;
use crate::predicates::is_cfg_unix_production_item;
use crate::predicates::item_is_private;
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
}

impl<'a> ParityContext<'a> {
    fn configured(config: &'a PortabilityConfig, platform_adapters: &'a PlatformAdapters) -> Self {
        Self {
            config,
            platform_adapters: Some(platform_adapters),
        }
    }

    pub(super) fn without_adapters(config: &'a PortabilityConfig) -> Self {
        Self {
            config,
            platform_adapters: None,
        }
    }
}

pub(super) fn collect_unix_portability_findings(
    file: &File,
    file_context: &FileContext,
    config: &PortabilityConfig,
    platform_adapters: &PlatformAdapters,
) -> Vec<PortabilityFinding> {
    let mut findings = Vec::new();
    let initial_scope = if file_context.is_test_file {
        ScopeKind::Test
    } else {
        ScopeKind::NonTest
    };
    let context = ParityContext::configured(config, platform_adapters);
    visit_items_for_unix_portability(
        &file.items,
        initial_scope,
        false,
        false,
        file_context,
        &mut findings,
        context,
    );
    findings
}

fn visit_items_for_unix_portability(
    items: &[Item],
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    inherited_platform_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    context: ParityContext<'_>,
) {
    collect_parity_findings(
        items,
        inherited_scope,
        inherited_unix_gated,
        file_context,
        findings,
        context,
    );
    for item in items {
        visit_item_for_unix_portability(
            item,
            inherited_scope,
            inherited_unix_gated,
            inherited_platform_gated,
            file_context,
            findings,
            context,
        );
    }
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
    let attrs = item_attrs(item);
    let scope = classify_scope(attrs, inherited_scope, item_name_hint_is_tests(item));
    let unix_gated = inherited_unix_gated || attrs.iter().any(attr_is_cfg_unix);
    let platform_gated = inherited_platform_gated || attrs.iter().any(attr_is_platform_cfg);

    if scope == ScopeKind::NonTest
        && !unix_gated
        && let Item::Impl(item_impl) = item
        && !attrs.iter().any(attr_is_cfg_windows)
    {
        collect_impl_method_parity_findings(item_impl, scope, file_context, findings, context);
    }

    if scope == ScopeKind::NonTest && !platform_gated {
        super::collect_production_path_literal_findings(
            item,
            file_context,
            findings,
            context.config,
        );
    }

    if scope == ScopeKind::NonTest {
        for attr in attrs {
            if crate::predicates::attr_is_cfg_attr_not_unix_allow_dead_code(attr) {
                findings.push(PortabilityFinding {
                    rule_id: RuleId::Port005,
                    kind: "cfg_attr_not_unix_allow_dead_code",
                    message: "PORT-005 #[cfg_attr(not(unix), allow(dead_code))] is not an approved portability suppressor in production code; gate the item with #[cfg(unix)] or provide a real cross-platform implementation".to_string(),
                    source_path: file_context.source_path.clone(),
                    line: span_start_line(attr.span()),
                    package: file_context.package.clone(),
                    target: file_context.target.clone(),
                    node_label: format!("crate::{}::{}::portability", file_context.package, file_context.target),
                });
            }
        }
        if let Item::Use(item_use) = item
            && crate::predicates::use_tree_contains_std_os_unix(&item_use.tree)
            && !unix_gated
        {
            findings.push(PortabilityFinding {
                rule_id: RuleId::Port004,
                kind: "ungated_std_os_unix_import",
                message: "PORT-004 ungated std::os::unix import in production code; wrap the item with #[cfg(unix)] or move the import behind a Unix-only boundary".to_string(),
                source_path: file_context.source_path.clone(),
                line: span_start_line(item_use.span()),
                package: file_context.package.clone(),
                target: file_context.target.clone(),
                node_label: format!("crate::{}::{}::portability", file_context.package, file_context.target),
            });
        }
    }

    if let Item::Mod(item_mod) = item
        && let Some((_, items)) = &item_mod.content
    {
        visit_items_for_unix_portability(
            items,
            scope,
            unix_gated,
            platform_gated,
            file_context,
            findings,
            context,
        );
    }
}

fn push_cfg_parity_finding(
    findings: &mut Vec<PortabilityFinding>,
    file_context: &FileContext,
    line: usize,
    detail: &str,
) {
    findings.push(PortabilityFinding {
        rule_id: RuleId::Port010,
        kind: "cfg_unix_parity_missing",
        message: format!(
            "PORT-010 production #[cfg(unix)] item `{detail}` has no #[cfg(windows)] companion or explicit portable fallback in the same scope; consider adding a cfg(windows) sibling or an unguarded portable fallback"
        ),
        source_path: file_context.source_path.clone(),
        line,
        package: file_context.package.clone(),
        target: file_context.target.clone(),
        node_label: format!("crate::{}::{}::portability", file_context.package, file_context.target),
    });
}

fn collect_parity_findings(
    items: &[Item],
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    context: ParityContext<'_>,
) {
    if inherited_scope != ScopeKind::NonTest || inherited_unix_gated {
        return;
    }
    for item in items {
        let line = span_start_line(item.span());
        let private_adapter_item = item_is_private(item)
            && context
                .platform_adapters
                .is_some_and(|adapters| adapters.is_adapter_file(&file_context.source_path));
        let configured_exemption = context
            .platform_adapters
            .is_some_and(|adapters| adapters.suppresses_port010(&file_context.source_path, line));
        if !is_cfg_unix_production_item(item, inherited_scope)
            || private_adapter_item
            || configured_exemption
            || has_windows_companion(item, items)
            || has_portable_fallback(item, items, inherited_scope)
        {
            continue;
        }
        let detail = item_identifier(item)
            .map(ToString::to_string)
            .unwrap_or_else(|| "cfg(unix) item".to_string());
        push_cfg_parity_finding(findings, file_context, line, &detail);
    }
}

pub(super) fn collect_impl_method_parity_findings(
    item_impl: &ItemImpl,
    inherited_scope: ScopeKind,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    context: ParityContext<'_>,
) {
    if inherited_scope != ScopeKind::NonTest {
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
            || impl_method_has_portable_fallback(unix_method, &item_impl.items, inherited_scope)
        {
            continue;
        }
        push_cfg_parity_finding(
            findings,
            file_context,
            span_start_line(unix_method.span()),
            &unix_method.sig.ident.to_string(),
        );
    }
}
