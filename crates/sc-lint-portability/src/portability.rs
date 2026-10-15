use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use rayon::prelude::*;
use serde::Deserialize;
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
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::Finding;
use crate::RuleId;
use crate::platform_adapters;
use crate::predicates::attr_is_platform_cfg;
use crate::predicates::extract_env_var_lookup;
use crate::predicates::is_dirs_home_dir_call;
use crate::predicates::is_path_like_constructor;
use crate::predicates::is_set_var_call;
use crate::predicates::is_shell_command_call;
use crate::predicates::is_unix_shell_path_literal;
use crate::predicates::production_env_portability_variable;
use crate::predicates::production_path_rule_id;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::FileContext;
use sc_lint_analyzer_support::RepoRelativePath;
use sc_lint_analyzer_support::ScopeKind;
use sc_lint_analyzer_support::WorkspaceMetadata;
use sc_lint_analyzer_support::classify_scope;
use sc_lint_analyzer_support::count_scanned_crates_with_metadata as count_crates;
use sc_lint_analyzer_support::discover_source_files_with_metadata;
use sc_lint_analyzer_support::relative_slash_path;
use sc_lint_analyzer_support::span_start_line;

mod findings;
mod parity;

use self::findings::PortabilityFinding;
use self::findings::convert_findings;

pub(super) fn repo_relative_source_path(
    root: &Path,
    source_path: &Path,
) -> Result<RepoRelativePath> {
    let relative = relative_slash_path(root, source_path);
    RepoRelativePath::parse(&relative).with_context(|| {
        format!(
            "failed to build repository-relative source path for `{}` under `{}`",
            source_path.display(),
            root.display()
        )
    })
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
struct RepoPortabilityConfig {
    config_home_env: Option<String>,
}

fn legacy_portability_config(
    document: &toml::Table,
) -> std::result::Result<RepoPortabilityConfig, AnalyzerConfigError> {
    let wrong_type = |key: &str, expected: &'static str| AnalyzerConfigError::WrongType {
        analyzer: "portability",
        key: key.to_string(),
        expected,
    };
    let Some(value) = document.get("portability") else {
        return Ok(RepoPortabilityConfig::default());
    };
    let table = value
        .as_table()
        .ok_or_else(|| wrong_type("portability", "a table"))?;
    let config_home_env = match table.get("config_home_env") {
        None => None,
        Some(toml::Value::String(value)) => Some(value.clone()),
        Some(_) => return Err(wrong_type("config_home_env", "a string")),
    };
    Ok(RepoPortabilityConfig { config_home_env })
}

pub(crate) fn validate_legacy_portability_config(
    document: &toml::Table,
) -> std::result::Result<(), AnalyzerConfigError> {
    legacy_portability_config(document).map(|_| ())
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
    /// The Unix path prefixes PORT-001 and PORT-006 share (ADR-010): the
    /// built-in list, extended by any configured `unix_path_prefixes`.
    unix_path_prefixes: Vec<String>,
}

impl PortabilityConfig {
    fn unix_path_prefix_match(&self, value: &str) -> bool {
        self.unix_path_prefixes
            .iter()
            .any(|prefix| value.starts_with(prefix))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AnalyzerPortabilityKeys {
    config_home_env: Option<String>,
    unix_path_prefixes: Vec<String>,
    platform_adapters: Vec<platform_adapters::PlatformAdapterRecord>,
}

impl AnalyzerPortabilityKeys {
    pub(crate) fn from_settings(
        settings: &AnalyzerSettings,
    ) -> std::result::Result<Self, AnalyzerConfigError> {
        let wrong_type = |key: &str, expected: &'static str| AnalyzerConfigError::WrongType {
            analyzer: "portability",
            key: key.to_string(),
            expected,
        };
        let config_home_env = settings.own_string("config_home_env")?;
        settings.own_strings("targets")?;
        let unix_path_prefixes = settings
            .own_strings("unix_path_prefixes")?
            .unwrap_or_default();
        if unix_path_prefixes
            .iter()
            .any(|prefix| prefix.is_empty() || !prefix.starts_with('/'))
        {
            return Err(wrong_type(
                "unix_path_prefixes",
                "an array of non-empty strings starting with `/`",
            ));
        }
        let platform_adapters = platform_adapters::parse_records(settings)?;
        Ok(Self {
            config_home_env,
            unix_path_prefixes,
            platform_adapters,
        })
    }

    pub(crate) fn platform_adapter_records(&self) -> &[platform_adapters::PlatformAdapterRecord] {
        &self.platform_adapters
    }
}

pub(crate) fn analyze_portability(
    root: &Path,
    workspace: &WorkspaceMetadata,
    document: &toml::Table,
    _settings: &AnalyzerSettings,
    analyzer_keys: &AnalyzerPortabilityKeys,
    platform_adapters: &platform_adapters::PlatformAdapters,
) -> Result<Vec<(Finding, Option<RepoRelativePath>)>> {
    let config = load_portability_config(document, analyzer_keys)?;
    let source_files = discover_source_files_with_metadata(workspace)?;
    let file_findings = source_files
        .par_iter()
        .map(|file_context| {
            let source = fs::read_to_string(&file_context.source_path).with_context(|| {
                format!(
                    "failed to read Rust source for portability analysis: {}",
                    file_context.source_path.display()
                )
            })?;
            let parsed = syn::parse_file(&source).with_context(|| {
                format!(
                    "failed to parse Rust source for portability analysis: {}",
                    file_context.source_path.display()
                )
            })?;

            let mut collector = PortabilityCollector::new(file_context, &config);
            collector.visit_file_items(&parsed);
            let mut findings = collector.findings;
            findings.extend(parity::collect_unix_portability_findings(
                &parsed,
                file_context,
                &config,
                platform_adapters,
            ));
            Ok(findings)
        })
        .collect::<Vec<Result<Vec<_>>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let findings = file_findings.into_iter().flatten().collect();

    convert_findings(root, findings)
}

pub(crate) fn count_scanned_crates(workspace: &WorkspaceMetadata) -> usize {
    count_crates(workspace)
}

struct PortabilityCollector<'a> {
    file_context: &'a FileContext,
    config: &'a PortabilityConfig,
    findings: Vec<PortabilityFinding>,
}

fn collect_production_path_literal_findings(
    item: &Item,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    config: &PortabilityConfig,
) {
    match item {
        Item::Fn(item_fn) => visit_block_for_unix_portability(
            &item_fn.block,
            ScopeKind::NonTest,
            false,
            false,
            file_context,
            findings,
            config,
        ),
        Item::Const(item_const) => visit_expr_for_unix_portability(
            &item_const.expr,
            ScopeKind::NonTest,
            false,
            false,
            file_context,
            findings,
            config,
        ),
        Item::Static(item_static) => visit_expr_for_unix_portability(
            &item_static.expr,
            ScopeKind::NonTest,
            false,
            false,
            file_context,
            findings,
            config,
        ),
        Item::Impl(item_impl) => {
            parity::collect_impl_method_parity_findings(
                item_impl,
                ScopeKind::NonTest,
                file_context,
                findings,
            );
            for impl_item in &item_impl.items {
                if let ImplItem::Fn(item_fn) = impl_item {
                    let fn_scope = classify_scope(&item_fn.attrs, ScopeKind::NonTest, None);
                    let fn_platform_gated = item_fn.attrs.iter().any(attr_is_platform_cfg);
                    if fn_scope == ScopeKind::NonTest && !fn_platform_gated {
                        visit_block_for_unix_portability(
                            &item_fn.block,
                            ScopeKind::NonTest,
                            false,
                            false,
                            file_context,
                            findings,
                            config,
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
    inherited_platform_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    config: &PortabilityConfig,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Item(item) => parity::visit_item_for_unix_portability(
                item,
                inherited_scope,
                inherited_unix_gated,
                inherited_platform_gated,
                file_context,
                findings,
                parity::ParityContext::without_adapters(config),
            ),
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    visit_expr_for_unix_portability(
                        &init.expr,
                        inherited_scope,
                        inherited_unix_gated,
                        inherited_platform_gated,
                        file_context,
                        findings,
                        config,
                    );
                }
            }
            Stmt::Expr(expr, _) => visit_expr_for_unix_portability(
                expr,
                inherited_scope,
                inherited_unix_gated,
                inherited_platform_gated,
                file_context,
                findings,
                config,
            ),
            Stmt::Macro(_) => {}
        }
    }
}

fn visit_expr_for_unix_portability(
    expr: &Expr,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    inherited_platform_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
    config: &PortabilityConfig,
) {
    if inherited_scope != ScopeKind::NonTest || inherited_unix_gated || inherited_platform_gated {
        return;
    }
    let mut visitor = ProductionPathLiteralVisitor::new(file_context, findings, config);
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
                let value = lit.value();
                if self.config.unix_path_prefix_match(&value) {
                    self.findings.push(PortabilityFinding {
                        rule_id: RuleId::Port001,
                        kind: "hardcoded_unix_path_literal",
                        message: format!(
                            "PORT-001 hardcoded Unix-only absolute path literal `{}` in test code; prefer std::env::temp_dir(), dirs::home_dir(), or another platform-aware path source",
                            value
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
        if !self.config.unix_path_prefix_match(&value) {
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
}

#[derive(Default)]
struct FunctionBodyVisitor {
    usages: Vec<Usage>,
}

struct ProductionPathLiteralVisitor<'a, 'b> {
    file_context: &'a FileContext,
    findings: &'b mut Vec<PortabilityFinding>,
    platform_gated_depth: usize,
    config: &'a PortabilityConfig,
}

impl<'a, 'b> ProductionPathLiteralVisitor<'a, 'b> {
    fn new(
        file_context: &'a FileContext,
        findings: &'b mut Vec<PortabilityFinding>,
        config: &'a PortabilityConfig,
    ) -> Self {
        Self {
            file_context,
            findings,
            platform_gated_depth: 0,
            config,
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

    fn push_shell_finding(&mut self, detail: &str, line: usize) {
        self.findings.push(PortabilityFinding {
            rule_id: RuleId::Port009,
            kind: "production_shell_portability",
            message: format!(
                "PORT-009 Unix shell invocation `{detail}` in production code assumes a Unix shell exists; prefer invoking the target binary directly or move the shell path behind an explicit Unix-only boundary"
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
            && let Some(command_name) = is_shell_command_call(node)
        {
            self.push_shell_finding(
                &format!("Command::new(\"{command_name}\")"),
                span_start_line(node.span()),
            );
        }
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
        {
            let value = lit.value();
            if is_unix_shell_path_literal(&value) {
                self.push_shell_finding(&value, span_start_line(lit.span()));
            } else if self.config.unix_path_prefix_match(&value) {
                self.push_path_finding(RuleId::Port006, &value, span_start_line(lit.span()));
            } else if let Some(rule_id) = production_path_rule_id(&value) {
                self.push_path_finding(rule_id, &value, span_start_line(lit.span()));
            }
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

fn load_portability_config(
    document: &toml::Table,
    analyzer_keys: &AnalyzerPortabilityKeys,
) -> Result<PortabilityConfig> {
    let defaults = toml::from_str::<BuiltInDefaults>(crate::DEFAULT_RULES_TOML)
        .context("failed to parse built-in portability defaults")?;
    let repo_config = legacy_portability_config(document)
        .map_err(anyhow::Error::from)
        .context("failed to parse repo portability config")?;

    Ok(PortabilityConfig {
        config_home_env: analyzer_keys
            .config_home_env
            .clone()
            .or(repo_config.config_home_env),
        unix_path_prefixes: defaults
            .portability
            .unix_path_prefixes
            .into_iter()
            .chain(analyzer_keys.unix_path_prefixes.iter().cloned())
            .collect(),
    })
}
