use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use quote::ToTokens;
use serde::Deserialize;
use syn::Attribute;
use syn::Block;
use syn::Expr;
use syn::ExprCall;
use syn::ExprLit;
use syn::ExprUnsafe;
use syn::File;
use syn::ImplItem;
use syn::Item;
use syn::ItemFn;
use syn::ItemImpl;
use syn::ItemMod;
use syn::Lit;
use syn::Stmt;
use syn::UseTree;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::CrateId;
use crate::Finding;
use crate::NodeId;
use crate::OwnerId;
use crate::RuleId;
use crate::source_scan::FileContext;
use crate::source_scan::PackageName;
use crate::source_scan::ScopeKind;
use crate::source_scan::TargetName;
use crate::source_scan::classify_scope;
use crate::source_scan::count_scanned_crates as count_crates;
use crate::source_scan::discover_source_files;
use crate::source_scan::item_attrs;
use crate::source_scan::item_name_hint_is_tests;
use crate::source_scan::span_start_line;

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
struct RepoLintConfig {
    portability: Option<RepoPortabilityConfig>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
struct RepoPortabilityConfig {
    config_home_env: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct PortabilityDefaults {
    unix_path_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct BuiltInDefaults {
    portability: PortabilityDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PortabilityConfig {
    config_home_env: Option<String>,
    unix_path_prefixes: Vec<String>,
}

/// Unix-centric env vars flagged by PORT-008 before callers normalize through a
/// platform-aware abstraction.
const PORTABILITY_ENV_NAMES: &[&str] = &["HOME", "USER"];
/// Unix-centric env var prefixes flagged by PORT-008 in ungated production code.
const PORTABILITY_ENV_PREFIXES: &[&str] = &["XDG_"];

#[derive(Debug, Clone)]
struct PortabilityFinding {
    rule_id: RuleId,
    kind: &'static str,
    message: String,
    source_path: PathBuf,
    line: usize,
    package: PackageName,
    target: TargetName,
    node_label: String,
}

pub(crate) fn analyze_portability(root: &Path) -> Result<Vec<Finding>> {
    let config = load_portability_config(root)?;
    let source_files = discover_source_files(root)?;
    let mut findings = Vec::new();

    for file_context in source_files {
        let parsed = syn::parse_file(&fs::read_to_string(&file_context.source_path).with_context(
            || {
                format!(
                    "failed to read Rust source for portability analysis: {}",
                    file_context.source_path.display()
                )
            },
        )?)
        .with_context(|| {
            format!(
                "failed to parse Rust source for portability analysis: {}",
                file_context.source_path.display()
            )
        })?;

        let mut collector = PortabilityCollector::new(&file_context, &config);
        collector.visit_file_items(&parsed);
        findings.extend(collector.findings);
        findings.extend(collect_unix_portability_findings(&parsed, &file_context));
    }

    findings.sort_by(|left, right| {
        left.rule_id
            .as_str()
            .cmp(right.rule_id.as_str())
            .then_with(|| left.source_path.cmp(&right.source_path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.message.cmp(&right.message))
    });

    Ok(findings
        .into_iter()
        .map(|finding| Finding {
            rule_id: finding.rule_id,
            kind: finding.kind.to_string(),
            message: format!(
                "{}:{}: {}",
                finding.source_path.display(),
                finding.line,
                finding.message
            ),
            owner_ids: vec![OwnerId::new(CrateId::from_parts(
                finding.package.as_str(),
                finding.target.as_str(),
            ))],
            node_ids: vec![NodeId::new(finding.node_label)],
        })
        .collect())
}

pub(crate) fn count_scanned_crates(root: &Path) -> Result<usize> {
    count_crates(root)
}

struct PortabilityCollector<'a> {
    file_context: &'a FileContext,
    config: &'a PortabilityConfig,
    findings: Vec<PortabilityFinding>,
}

fn collect_unix_portability_findings(
    file: &File,
    file_context: &FileContext,
) -> Vec<PortabilityFinding> {
    let mut findings = Vec::new();
    let initial_scope = if file_context.is_test_file {
        ScopeKind::Test
    } else {
        ScopeKind::NonTest
    };
    visit_items_for_unix_portability(
        &file.items,
        initial_scope,
        false,
        file_context,
        &mut findings,
    );
    findings
}

fn visit_items_for_unix_portability(
    items: &[Item],
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    for item in items {
        visit_item_for_unix_portability(
            item,
            inherited_scope,
            inherited_unix_gated,
            file_context,
            findings,
        );
    }
}

fn visit_item_for_unix_portability(
    item: &Item,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    let attrs = item_attrs(item);
    let scope = classify_scope(attrs, inherited_scope, item_name_hint_is_tests(item));
    let unix_gated = inherited_unix_gated || attrs.iter().any(attr_is_cfg_unix);

    if scope == ScopeKind::NonTest && !unix_gated {
        collect_production_path_literal_findings(item, file_context, findings);
    }

    if scope == ScopeKind::NonTest {
        for attr in attrs {
            if attr_is_cfg_attr_not_unix_allow_dead_code(attr) {
                findings.push(PortabilityFinding {
                    rule_id: RuleId::Port005,
                    kind: "cfg_attr_not_unix_allow_dead_code",
                    message: "PORT-005 #[cfg_attr(not(unix), allow(dead_code))] is not an approved portability suppressor in production code; gate the item with #[cfg(unix)] or provide a real cross-platform implementation".to_string(),
                    source_path: file_context.source_path.clone(),
                    line: span_start_line(attr.span()),
                    package: file_context.package.clone(),
                    target: file_context.target.clone(),
                    node_label: format!(
                        "crate::{}::{}::portability",
                        file_context.package, file_context.target
                    ),
                });
            }
        }

        if let Item::Use(item_use) = item
            && use_tree_contains_std_os_unix(&item_use.tree)
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
                node_label: format!(
                    "crate::{}::{}::portability",
                    file_context.package, file_context.target
                ),
            });
        }
    }

    if let Item::Mod(item_mod) = item
        && let Some((_, items)) = &item_mod.content
    {
        visit_items_for_unix_portability(items, scope, unix_gated, file_context, findings);
    }
}

fn collect_production_path_literal_findings(
    item: &Item,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    match item {
        Item::Fn(item_fn) => visit_block_for_unix_portability(
            &item_fn.block,
            ScopeKind::NonTest,
            false,
            file_context,
            findings,
        ),
        Item::Const(item_const) => visit_expr_for_unix_portability(
            &item_const.expr,
            ScopeKind::NonTest,
            false,
            file_context,
            findings,
        ),
        Item::Static(item_static) => visit_expr_for_unix_portability(
            &item_static.expr,
            ScopeKind::NonTest,
            false,
            file_context,
            findings,
        ),
        Item::Impl(item_impl) => {
            for impl_item in &item_impl.items {
                if let ImplItem::Fn(item_fn) = impl_item {
                    let fn_scope = classify_scope(&item_fn.attrs, ScopeKind::NonTest, None);
                    if fn_scope == ScopeKind::NonTest {
                        visit_block_for_unix_portability(
                            &item_fn.block,
                            ScopeKind::NonTest,
                            false,
                            file_context,
                            findings,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

fn visit_block_for_unix_portability(
    block: &Block,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Item(item) => visit_item_for_unix_portability(
                item,
                inherited_scope,
                inherited_unix_gated,
                file_context,
                findings,
            ),
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    visit_expr_for_unix_portability(
                        &init.expr,
                        inherited_scope,
                        inherited_unix_gated,
                        file_context,
                        findings,
                    );
                }
            }
            Stmt::Expr(expr, _) => visit_expr_for_unix_portability(
                expr,
                inherited_scope,
                inherited_unix_gated,
                file_context,
                findings,
            ),
            Stmt::Macro(_) => {}
        }
    }
}

fn visit_expr_for_unix_portability(
    expr: &Expr,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    if inherited_scope != ScopeKind::NonTest || inherited_unix_gated {
        return;
    }
    let mut visitor = ProductionPathLiteralVisitor::new(file_context, findings);
    visitor.visit_expr(expr);
}

impl<'a> PortabilityCollector<'a> {
    fn new(file_context: &'a FileContext, config: &'a PortabilityConfig) -> Self {
        Self {
            file_context,
            config,
            findings: Vec::new(),
        }
    }

    fn visit_file_items(&mut self, file: &File) {
        let initial_scope = if self.file_context.is_test_file {
            ScopeKind::Test
        } else {
            ScopeKind::NonTest
        };
        self.visit_items(&file.items, initial_scope);
    }

    fn visit_items(&mut self, items: &[Item], inherited_scope: ScopeKind) {
        for item in items {
            self.visit_item(item, inherited_scope);
        }
    }

    fn visit_item(&mut self, item: &Item, inherited_scope: ScopeKind) {
        match item {
            Item::Fn(item_fn) => self.visit_item_fn(item_fn, inherited_scope),
            Item::Mod(item_mod) => self.visit_item_mod(item_mod, inherited_scope),
            Item::Impl(item_impl) => self.visit_item_impl(item_impl, inherited_scope),
            Item::Const(item_const) => {
                if classify_scope(&item_const.attrs, inherited_scope, None) == ScopeKind::Test {
                    self.visit_expr_for_portability(&item_const.expr, "const");
                }
            }
            Item::Static(item_static) => {
                if classify_scope(&item_static.attrs, inherited_scope, None) == ScopeKind::Test {
                    self.visit_expr_for_portability(&item_static.expr, "static");
                }
            }
            _ => {}
        }
    }

    fn visit_item_fn(&mut self, item_fn: &ItemFn, inherited_scope: ScopeKind) {
        let scope = classify_scope(
            &item_fn.attrs,
            inherited_scope,
            Some(item_fn.sig.ident == "tests"),
        );
        if scope == ScopeKind::Test {
            self.check_home_dir_rule_in_function(&item_fn.block, item_fn.sig.ident.to_string());
            self.visit_block_for_portability(&item_fn.block, &item_fn.sig.ident.to_string());
        }
    }

    fn visit_item_mod(&mut self, item_mod: &ItemMod, inherited_scope: ScopeKind) {
        let scope = classify_scope(
            &item_mod.attrs,
            inherited_scope,
            Some(item_mod.ident == "tests"),
        );
        if let Some((_, items)) = &item_mod.content {
            self.visit_items(items, scope);
        }
    }

    fn visit_item_impl(&mut self, item_impl: &ItemImpl, inherited_scope: ScopeKind) {
        let scope = classify_scope(&item_impl.attrs, inherited_scope, None);
        if scope != ScopeKind::Test {
            return;
        }
        for item in &item_impl.items {
            if let ImplItem::Fn(item_fn) = item {
                let fn_scope = classify_scope(&item_fn.attrs, scope, None);
                if fn_scope == ScopeKind::Test {
                    self.check_home_dir_rule_in_function(
                        &item_fn.block,
                        item_fn.sig.ident.to_string(),
                    );
                    self.visit_block_for_portability(
                        &item_fn.block,
                        &item_fn.sig.ident.to_string(),
                    );
                }
            }
        }
    }

    fn check_home_dir_rule_in_function(&mut self, block: &Block, fn_name: String) {
        let Some(config_home_env) = &self.config.config_home_env else {
            return;
        };
        let mut env_check_lines = Vec::new();
        let mut home_dir_calls = Vec::new();

        let mut visitor = FunctionBodyVisitor::default();
        visitor.visit_block(block);
        for usage in visitor.usages {
            match usage {
                Usage::EnvVarCheck { name, line } if name == *config_home_env => {
                    env_check_lines.push(line)
                }
                Usage::DirsHomeDir { line } => home_dir_calls.push(line),
                _ => {}
            }
        }

        for line in home_dir_calls {
            let has_prior_env_check = env_check_lines.iter().any(|env_line| *env_line < line);
            if has_prior_env_check {
                continue;
            }
            self.findings.push(PortabilityFinding {
                rule_id: RuleId::Port002,
                kind: "config_home_override_missing",
                message: format!(
                    "PORT-002 direct dirs::home_dir() in test helper `{fn_name}` without checking {} first; use the configured home-dir wrapper or check {} before falling back to dirs::home_dir()",
                    config_home_env, config_home_env
                ),
                source_path: self.file_context.source_path.clone(),
                line,
                package: self.file_context.package.clone(),
                target: self.file_context.target.clone(),
                node_label: format!(
                    "crate::{}::{}::{}",
                    self.file_context.package, self.file_context.target, fn_name
                ),
            });
        }
    }

    fn visit_block_for_portability(&mut self, block: &Block, label: &str) {
        for stmt in &block.stmts {
            self.visit_stmt_for_portability(stmt, label);
        }
    }

    fn visit_stmt_for_portability(&mut self, stmt: &Stmt, label: &str) {
        match stmt {
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr_for_portability(&init.expr, label);
                }
            }
            Stmt::Expr(expr, _) => self.visit_expr_for_portability(expr, label),
            Stmt::Item(item) => self.visit_item(item, ScopeKind::Test),
            Stmt::Macro(_) => {}
        }
    }

    fn visit_expr_for_portability(&mut self, expr: &Expr, label: &str) {
        match expr {
            Expr::Call(expr_call) => {
                let flagged_path_call = self.check_path_literal_call(expr_call, label);
                self.check_env_set_var(expr_call, label);
                if !flagged_path_call {
                    for arg in &expr_call.args {
                        self.visit_expr_for_portability(arg, label);
                    }
                }
                self.visit_expr_for_portability(&expr_call.func, label);
            }
            Expr::Unsafe(ExprUnsafe { block, .. }) => {
                self.visit_block_for_portability(block, label)
            }
            Expr::Block(expr_block) => self.visit_block_for_portability(&expr_block.block, label),
            Expr::If(expr_if) => {
                self.visit_expr_for_portability(&expr_if.cond, label);
                self.visit_block_for_portability(&expr_if.then_branch, label);
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    self.visit_expr_for_portability(else_expr, label);
                }
            }
            Expr::Match(expr_match) => {
                self.visit_expr_for_portability(&expr_match.expr, label);
                for arm in &expr_match.arms {
                    self.visit_expr_for_portability(&arm.body, label);
                }
            }
            Expr::MethodCall(expr_method) => {
                for arg in &expr_method.args {
                    self.visit_expr_for_portability(arg, label);
                }
                self.visit_expr_for_portability(&expr_method.receiver, label);
            }
            Expr::Lit(ExprLit {
                lit: Lit::Str(lit), ..
            }) => {
                if self.path_prefix_match(&lit.value()) {
                    self.findings.push(PortabilityFinding {
                        rule_id: RuleId::Port001,
                        kind: "hardcoded_unix_path_literal",
                        message: format!(
                            "PORT-001 hardcoded Unix-only absolute path literal `{}` in test code; prefer std::env::temp_dir(), dirs::home_dir(), or another platform-aware path source",
                            lit.value()
                        ),
                        source_path: self.file_context.source_path.clone(),
                        line: span_start_line(lit.span()),
                        package: self.file_context.package.clone(),
                        target: self.file_context.target.clone(),
                        node_label: format!(
                            "crate::{}::{}::{}",
                            self.file_context.package, self.file_context.target, label
                        ),
                    });
                }
            }
            _ => {}
        }
    }

    fn check_path_literal_call(&mut self, expr_call: &ExprCall, label: &str) -> bool {
        if !is_path_like_constructor(&expr_call.func) {
            return false;
        }
        let Some(first_arg) = expr_call.args.first() else {
            return false;
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = first_arg
        else {
            return false;
        };
        let value = lit.value();
        if !self.path_prefix_match(&value) {
            return false;
        }
        self.findings.push(PortabilityFinding {
            rule_id: RuleId::Port001,
            kind: "hardcoded_unix_path_constructor",
            message: format!(
                "PORT-001 hardcoded Unix-only absolute path `{value}` in test code; prefer std::env::temp_dir(), dirs::home_dir(), or another platform-aware path source"
            ),
            source_path: self.file_context.source_path.clone(),
            line: span_start_line(expr_call.span()),
            package: self.file_context.package.clone(),
            target: self.file_context.target.clone(),
            node_label: format!(
                "crate::{}::{}::{}",
                self.file_context.package, self.file_context.target, label
            ),
        });
        true
    }

    fn check_env_set_var(&mut self, expr_call: &ExprCall, label: &str) {
        if !is_set_var_call(&expr_call.func) {
            return;
        }
        self.findings.push(PortabilityFinding {
            rule_id: RuleId::Port003,
            kind: "global_env_mutation_in_test",
            message: "PORT-003 std::env::set_var() in test code mutates global process state; prefer cmd.env(\"KEY\", \"value\") or temp_env::with_var for scoped isolation".to_string(),
            source_path: self.file_context.source_path.clone(),
            line: span_start_line(expr_call.span()),
            package: self.file_context.package.clone(),
            target: self.file_context.target.clone(),
            node_label: format!(
                "crate::{}::{}::{}",
                self.file_context.package, self.file_context.target, label
            ),
        });
    }

    fn path_prefix_match(&self, value: &str) -> bool {
        self.config
            .unix_path_prefixes
            .iter()
            .any(|prefix| value.starts_with(prefix))
    }
}

#[derive(Default)]
struct FunctionBodyVisitor {
    usages: Vec<Usage>,
}

struct ProductionPathLiteralVisitor<'a, 'b> {
    file_context: &'a FileContext,
    findings: &'b mut Vec<PortabilityFinding>,
    platform_gated_depth: usize,
}

impl<'a, 'b> ProductionPathLiteralVisitor<'a, 'b> {
    fn new(file_context: &'a FileContext, findings: &'b mut Vec<PortabilityFinding>) -> Self {
        Self {
            file_context,
            findings,
            platform_gated_depth: 0,
        }
    }

    fn push_path_finding(&mut self, rule_id: RuleId, value: &str, line: usize) {
        let message = match rule_id {
            RuleId::Port006 => format!(
                "PORT-006 hardcoded Unix-only absolute path literal `{value}` in production code; prefer dirs::cache_dir(), dirs::config_dir(), std::env::temp_dir(), or a platform-gated path abstraction"
            ),
            RuleId::Port007 => format!(
                "PORT-007 hardcoded Windows-only absolute path literal `{value}` in production code; prefer dirs::cache_dir(), dirs::config_dir(), or a platform-gated path abstraction"
            ),
            _ => return,
        };
        self.findings.push(PortabilityFinding {
            rule_id,
            kind: "hardcoded_production_path_literal",
            message,
            source_path: self.file_context.source_path.clone(),
            line,
            package: self.file_context.package.clone(),
            target: self.file_context.target.clone(),
            node_label: format!(
                "crate::{}::{}::portability",
                self.file_context.package, self.file_context.target
            ),
        });
    }

    fn push_env_finding(&mut self, variable_name: &str, line: usize) {
        self.findings.push(PortabilityFinding {
            rule_id: RuleId::Port008,
            kind: "production_env_portability_lookup",
            message: format!(
                "PORT-008 direct std::env lookup of `{variable_name}` in production code bypasses platform-neutral path or identity abstractions; prefer dirs::data_dir(), dirs::config_dir(), dirs::home_dir(), or another platform-aware wrapper"
            ),
            source_path: self.file_context.source_path.clone(),
            line,
            package: self.file_context.package.clone(),
            target: self.file_context.target.clone(),
            node_label: format!(
                "crate::{}::{}::portability",
                self.file_context.package, self.file_context.target
            ),
        });
    }
}

impl<'ast> Visit<'ast> for ProductionPathLiteralVisitor<'_, '_> {
    fn visit_expr_block(&mut self, node: &'ast syn::ExprBlock) {
        let is_platform_gated = node.attrs.iter().any(attr_is_platform_cfg);
        if is_platform_gated {
            self.platform_gated_depth += 1;
        }
        syn::visit::visit_expr_block(self, node);
        if is_platform_gated {
            self.platform_gated_depth -= 1;
        }
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if self.platform_gated_depth == 0
            && let Some(variable_name) = production_env_portability_variable(node)
        {
            self.push_env_finding(&variable_name, span_start_line(node.span()));
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast ExprLit) {
        if self.platform_gated_depth == 0
            && let Lit::Str(lit) = &node.lit
            && let Some(rule_id) = production_path_rule_id(&lit.value())
        {
            self.push_path_finding(rule_id, &lit.value(), span_start_line(lit.span()));
        }
        syn::visit::visit_expr_lit(self, node);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Usage {
    EnvVarCheck { name: String, line: usize },
    DirsHomeDir { line: usize },
}

impl<'ast> Visit<'ast> for FunctionBodyVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Some(env_var_name) = extract_env_var_lookup(node) {
            self.usages.push(Usage::EnvVarCheck {
                name: env_var_name,
                line: span_start_line(node.span()),
            });
        }
        if is_dirs_home_dir_call(&node.func) {
            self.usages.push(Usage::DirsHomeDir {
                line: span_start_line(node.span()),
            });
        }
        syn::visit::visit_expr_call(self, node);
    }
}

fn load_portability_config(root: &Path) -> Result<PortabilityConfig> {
    let defaults = toml::from_str::<BuiltInDefaults>(crate::DEFAULT_RULES_TOML)
        .context("failed to parse built-in portability defaults")?;
    let repo_config_path = ["sc-lint.toml", ".just/lint-config.toml"]
        .into_iter()
        .map(|relative| root.join(relative))
        .find(|path| path.exists());
    let repo_config = if let Some(repo_config_path) = repo_config_path {
        let text = fs::read_to_string(&repo_config_path).with_context(|| {
            format!(
                "failed to read repo portability config: {}",
                repo_config_path.display()
            )
        })?;
        toml::from_str::<RepoLintConfig>(&text).with_context(|| {
            format!(
                "failed to parse repo portability config: {}",
                repo_config_path.display()
            )
        })?
    } else {
        RepoLintConfig::default()
    };

    Ok(PortabilityConfig {
        config_home_env: repo_config.portability.and_then(|cfg| cfg.config_home_env),
        unix_path_prefixes: defaults.portability.unix_path_prefixes,
    })
}

fn attr_is_cfg_unix(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let rendered = attr.meta.to_token_stream().to_string().replace(' ', "");
    rendered.contains("unix") && !rendered.contains("not(unix)")
}

fn attr_is_platform_cfg(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let rendered = attr.meta.to_token_stream().to_string().replace(' ', "");
    rendered.contains("unix") || rendered.contains("windows")
}

fn attr_is_cfg_attr_not_unix_allow_dead_code(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg_attr") {
        return false;
    }
    let rendered = attr.meta.to_token_stream().to_string().replace(' ', "");
    rendered.contains("not(unix)") && rendered.contains("allow(dead_code)")
}

fn use_tree_contains_std_os_unix(tree: &UseTree) -> bool {
    fn walk(tree: &UseTree, prefix: &mut Vec<String>) -> bool {
        match tree {
            UseTree::Path(use_path) => {
                prefix.push(use_path.ident.to_string());
                let matched = walk(&use_path.tree, prefix);
                prefix.pop();
                matched
            }
            UseTree::Name(use_name) => {
                prefix.push(use_name.ident.to_string());
                let matched = matches!(prefix.as_slice(), [std, os, unix, ..] if std == "std" && os == "os" && unix == "unix");
                prefix.pop();
                matched
            }
            UseTree::Rename(use_rename) => {
                prefix.push(use_rename.ident.to_string());
                let matched = matches!(prefix.as_slice(), [std, os, unix, ..] if std == "std" && os == "os" && unix == "unix");
                prefix.pop();
                matched
            }
            UseTree::Glob(_) => {
                matches!(prefix.as_slice(), [std, os, unix, ..] if std == "std" && os == "os" && unix == "unix")
            }
            UseTree::Group(group) => group.items.iter().any(|item| walk(item, prefix)),
        }
    }

    walk(tree, &mut Vec::new())
}

fn is_path_like_constructor(expr: &Expr) -> bool {
    let Expr::Path(expr_path) = expr else {
        return false;
    };
    let segments: Vec<_> = expr_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    match segments.as_slice() {
        [single, from] => {
            single == "PathBuf" && from == "from" || single == "Path" && from == "new"
        }
        [std, path_mod, path_type, method] => {
            std == "std"
                && path_mod == "path"
                && ((path_type == "PathBuf" && method == "from")
                    || (path_type == "Path" && method == "new"))
        }
        _ => false,
    }
}

fn is_dirs_home_dir_call(expr: &Expr) -> bool {
    let Expr::Path(expr_path) = expr else {
        return false;
    };
    let segments: Vec<_> = expr_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    matches!(segments.as_slice(), [dirs, home_dir] if dirs == "dirs" && home_dir == "home_dir")
}

fn is_set_var_call(expr: &Expr) -> bool {
    let Expr::Path(expr_path) = expr else {
        return false;
    };
    let segments: Vec<_> = expr_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    match segments.as_slice() {
        [env, set_var] => env == "env" && set_var == "set_var",
        [std, env, set_var] => std == "std" && env == "env" && set_var == "set_var",
        _ => false,
    }
}

fn extract_env_var_lookup(expr_call: &ExprCall) -> Option<String> {
    let Expr::Path(expr_path) = &*expr_call.func else {
        return None;
    };
    let segments: Vec<_> = expr_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let is_env_var = match segments.as_slice() {
        [env, var] => env == "env" && (var == "var" || var == "var_os"),
        [std, env, var] => std == "std" && env == "env" && (var == "var" || var == "var_os"),
        _ => false,
    };
    if !is_env_var {
        return None;
    }
    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = expr_call.args.first()?
    else {
        return None;
    };
    Some(lit.value())
}

fn production_env_portability_variable(expr_call: &ExprCall) -> Option<String> {
    let variable_name = extract_env_var_lookup(expr_call)?;
    if PORTABILITY_ENV_NAMES.contains(&variable_name.as_str())
        || PORTABILITY_ENV_PREFIXES
            .iter()
            .any(|prefix| variable_name.starts_with(prefix))
    {
        Some(variable_name)
    } else {
        None
    }
}

fn production_path_rule_id(value: &str) -> Option<RuleId> {
    if is_windows_path_literal(value) {
        Some(RuleId::Port007)
    } else if is_unix_path_literal(value) {
        Some(RuleId::Port006)
    } else {
        None
    }
}

fn is_unix_path_literal(value: &str) -> bool {
    matches!(
        value,
        path if path.starts_with("/home/")
            || path.starts_with("/usr/")
            || path.starts_with("/etc/")
            || path.starts_with("/var/")
            || path.starts_with("/tmp/")
    )
}

fn is_windows_path_literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let unc_absolute = value.starts_with("\\\\") && value.len() > 2;
    drive_absolute || unc_absolute
}
