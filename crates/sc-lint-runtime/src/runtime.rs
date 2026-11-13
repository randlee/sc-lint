use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use rayon::prelude::*;
use syn::Block;
use syn::Expr;
use syn::ExprMethodCall;
use syn::File;
use syn::ImplItem;
use syn::Item;
use syn::ItemFn;
use syn::ItemImpl;
use syn::ItemMod;
use syn::Pat;
use syn::Stmt;

use crate::CrateId;
use crate::Finding;
use crate::NodeId;
use crate::OwnerId;
use crate::RuleId;
use crate::position::SourcePosition;
use crate::scope::classify as classify_scope;
use sc_lint_analyzer_support::FileContext;
use sc_lint_analyzer_support::PackageDependencyKind;
use sc_lint_analyzer_support::PackageName;
use sc_lint_analyzer_support::PackageTargetKind;
use sc_lint_analyzer_support::RepoRelativePath;
use sc_lint_analyzer_support::ScopeKind;
use sc_lint_analyzer_support::TargetName;
use sc_lint_analyzer_support::WorkspaceMetadata;
use sc_lint_analyzer_support::discover_source_files_with_metadata;
use sc_lint_analyzer_support::repo_relative_source_path;

#[derive(Debug, Clone)]
struct RuntimeFinding {
    rule_id: RuleId,
    message: String,
    source_path: PathBuf,
    position: SourcePosition,
    package: PackageName,
    target: TargetName,
    node_label: String,
}

fn node_label(file: &FileContext, function: &str) -> String {
    format!("crate::{}::{}::{function}", file.package, file.target)
}

impl RuntimeFinding {
    fn new(
        file: &FileContext,
        rule_id: RuleId,
        message: impl AsRef<str>,
        position: SourcePosition,
        function: &str,
    ) -> Self {
        Self {
            rule_id,
            message: format!("{} {}", rule_id.as_str(), message.as_ref()),
            source_path: file.source_path.clone(),
            position,
            package: file.package.clone(),
            target: file.target.clone(),
            node_label: node_label(file, function),
        }
    }
}

pub(crate) fn analyze_runtime_liveness(
    root: &Path,
    workspace: &WorkspaceMetadata,
) -> Result<Vec<(Finding, Option<RepoRelativePath>)>> {
    let source_files = discover_runtime_sources(workspace)?;
    let sleep_scope = SleepScope::from_workspace(workspace);
    let file_findings = source_files
        .par_iter()
        .map(|file_context| analyze_file(&file_context.file, &sleep_scope, file_context.kind))
        .collect::<Vec<Result<Vec<_>>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let mut findings = file_findings.into_iter().flatten().collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        left.rule_id
            .as_str()
            .cmp(right.rule_id.as_str())
            .then_with(|| left.source_path.cmp(&right.source_path))
            .then_with(|| left.position.cmp(&right.position))
            .then_with(|| left.message.cmp(&right.message))
            .then_with(|| left.package.cmp(&right.package))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.node_label.cmp(&right.node_label))
    });

    findings
        .into_iter()
        .map(|finding| {
            let source_path = repo_relative_source_path(root, &finding.source_path)?;
            Ok((
                Finding {
                    rule_id: finding.rule_id,
                    kind: finding.rule_id.kind().into(),
                    message: format!(
                        "{}:{}: {}",
                        finding.source_path.display(),
                        finding.position.line,
                        finding.message
                    ),
                    owner_ids: vec![OwnerId::new(CrateId::from_parts(
                        finding.package.as_str(),
                        finding.target.as_str(),
                    ))],
                    node_ids: vec![NodeId::new(finding.node_label)],
                },
                Some(source_path),
            ))
        })
        .collect()
}

fn analyze_file(
    file_context: &FileContext,
    sleep_scope: &SleepScope,
    target_kind: PackageTargetKind,
) -> Result<Vec<RuntimeFinding>> {
    let source = fs::read_to_string(&file_context.source_path).with_context(|| {
        format!(
            "failed to read Rust source for runtime liveness analysis: {}",
            file_context.source_path.display()
        )
    })?;
    let parsed = syn::parse_file(&source).with_context(|| {
        format!(
            "failed to parse Rust source for runtime liveness analysis: {}",
            file_context.source_path.display()
        )
    })?;

    if file_context.is_test_file || !crate::scope::production(&parsed.attrs, None) {
        return Ok(Vec::new());
    }
    let allowances =
        crate::practices::allowances::Allowances::collect(&parsed).with_context(|| {
            format!(
                "invalid runtime allowance in {}",
                file_context.source_path.display()
            )
        })?;
    let mut collector = RuntimeCollector::new(file_context);
    collector.visit_file_items(&parsed);
    if sleep_scope.includes(file_context, target_kind) {
        for site in crate::sleep::find_blocking_sleeps(&parsed) {
            collector
                .findings
                .push(blocking_sleep_finding(file_context, site));
        }
    }
    for site in crate::practices::find(&parsed) {
        if site.rule.id() != RuleId::ScbRuntime004
            || sleep_scope.includes(file_context, target_kind)
        {
            collector
                .findings
                .push(practice_finding(file_context, site));
        }
    }
    collector
        .findings
        .retain(|site| !allowances.allows(site.rule_id, site.position));
    Ok(collector.findings)
}

fn blocking_sleep_finding(
    file_context: &FileContext,
    site: crate::sleep::SleepSite,
) -> RuntimeFinding {
    RuntimeFinding::new(
        file_context,
        RuleId::ScbRuntime003,
        format!(
            concat!(
                "std::thread::sleep blocks the thread in Tokio package `{}`; ",
                "use event-driven synchronization or `yield_now` instead"
            ),
            file_context.package
        ),
        site.position,
        site.enclosing_fn.as_deref().unwrap_or("<module>"),
    )
}

fn practice_finding(file: &FileContext, site: crate::practices::Site) -> RuntimeFinding {
    let recovery = site.rule.recovery();
    let rule_id = site.rule.id();
    RuntimeFinding::new(
        file,
        rule_id,
        format!("{}; {}", site.detail, recovery),
        site.position,
        &site.function,
    )
}

#[derive(Debug)]
struct RuntimeFileContext {
    file: FileContext,
    kind: PackageTargetKind,
}

fn discover_runtime_sources(workspace: &WorkspaceMetadata) -> Result<Vec<RuntimeFileContext>> {
    let packages = workspace.workspace_packages();
    let existing = discover_source_files_with_metadata(workspace)?;
    let mut seen = existing
        .iter()
        .map(|file| canonical_or_original(&file.source_path))
        .collect::<BTreeSet<_>>();
    let mut files = existing
        .into_iter()
        .map(|file| {
            let kind = packages
                .iter()
                .find(|package| package.name == file.package)
                .and_then(|package| {
                    package
                        .targets
                        .iter()
                        .find(|target| target.name == file.target)
                })
                .map_or(PackageTargetKind::Other, |target| target.kind);
            RuntimeFileContext { file, kind }
        })
        .collect::<Vec<_>>();
    for package in packages {
        let mut examples = package
            .targets
            .into_iter()
            .filter(|target| target.kind == PackageTargetKind::Example)
            .collect::<Vec<_>>();
        examples.sort_by(|left, right| left.name.cmp(&right.name));
        for target in examples {
            let root = &target.src_path;
            let module_dir = root.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
            collect_example_module(
                root,
                &module_dir,
                &package.name,
                &target.name,
                &mut seen,
                &mut BTreeSet::new(),
                &mut files,
            )?;
        }
    }
    Ok(files)
}

fn module_directory(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    match path.file_name().and_then(|name| name.to_str()) {
        Some("main.rs" | "lib.rs" | "mod.rs") => parent.to_path_buf(),
        _ => parent.join(path.file_stem().unwrap_or_default()),
    }
}

fn collect_example_module(
    path: &Path,
    module_dir: &Path,
    package: &PackageName,
    target: &TargetName,
    seen: &mut BTreeSet<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
    files: &mut Vec<RuntimeFileContext>,
) -> Result<()> {
    let canonical = canonical_or_original(path);
    if !path.is_file() || !visited.insert(canonical.clone()) {
        return Ok(());
    }
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read example source {}", path.display()))?;
    let parsed = syn::parse_file(&source)
        .with_context(|| format!("failed to parse example source {}", path.display()))?;
    if seen.insert(canonical) {
        files.push(RuntimeFileContext {
            file: FileContext {
                source_path: path.to_path_buf(),
                package: package.clone(),
                target: target.clone(),
                is_test_file: false,
            },
            kind: PackageTargetKind::Example,
        });
    }
    collect_nested_example_modules(
        &parsed.items,
        module_dir,
        package,
        target,
        seen,
        visited,
        files,
    )
}

fn collect_nested_example_modules(
    items: &[Item],
    module_dir: &Path,
    package: &PackageName,
    target: &TargetName,
    seen: &mut BTreeSet<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
    files: &mut Vec<RuntimeFileContext>,
) -> Result<()> {
    for item in items {
        let Item::Mod(module) = item else { continue };
        let name = module.ident.to_string();
        if let Some((_, nested)) = &module.content {
            collect_nested_example_modules(
                nested,
                &module_dir.join(name),
                package,
                target,
                seen,
                visited,
                files,
            )?;
        } else {
            let direct = module_dir.join(format!("{name}.rs"));
            let nested = module_dir.join(&name).join("mod.rs");
            let explicit = module.attrs.iter().find_map(|attr| {
                let syn::Meta::NameValue(value) = &attr.meta else {
                    return None;
                };
                if !value.path.is_ident("path") {
                    return None;
                }
                let syn::Expr::Lit(expr) = &value.value else {
                    return None;
                };
                let syn::Lit::Str(path) = &expr.lit else {
                    return None;
                };
                Some(module_dir.join(path.value()))
            });
            let path = explicit.unwrap_or_else(|| if direct.is_file() { direct } else { nested });
            collect_example_module(
                &path,
                &module_directory(&path),
                package,
                target,
                seen,
                visited,
                files,
            )?;
        }
    }
    Ok(())
}

#[derive(Default)]
struct SleepScope {
    excluded_roots: BTreeMap<PackageName, BTreeSet<PathBuf>>,
}

impl SleepScope {
    fn from_workspace(workspace: &WorkspaceMetadata) -> Self {
        let packages = workspace
            .workspace_packages()
            .into_iter()
            .filter(package_declares_tokio)
            .filter_map(|package| {
                let has_production_target = package.targets.iter().any(|target| {
                    matches!(target.kind, PackageTargetKind::Lib | PackageTargetKind::Bin)
                });
                let excluded = package
                    .targets
                    .iter()
                    .filter(|target| {
                        !matches!(target.kind, PackageTargetKind::Lib | PackageTargetKind::Bin)
                    })
                    .map(|target| canonical_or_original(&target.src_path))
                    .collect::<BTreeSet<_>>();
                has_production_target.then_some((package.name, excluded))
            })
            .collect();
        Self {
            excluded_roots: packages,
        }
    }

    fn includes(&self, file: &FileContext, kind: PackageTargetKind) -> bool {
        kind != PackageTargetKind::Example
            && !file.is_test_file
            && self
                .excluded_roots
                .get(&file.package)
                .is_some_and(|roots| !roots.contains(&canonical_or_original(&file.source_path)))
    }
}

fn package_declares_tokio(package: &sc_lint_analyzer_support::WorkspacePackageInfo) -> bool {
    package.dependencies.iter().any(|dependency| {
        dependency.name.as_str() == "tokio"
            && matches!(
                dependency.kind,
                PackageDependencyKind::Normal | PackageDependencyKind::Build
            )
    })
}

fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

struct RuntimeCollector<'a> {
    file_context: &'a FileContext,
    findings: Vec<RuntimeFinding>,
}

impl<'a> RuntimeCollector<'a> {
    fn new(file_context: &'a FileContext) -> Self {
        Self {
            file_context,
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
            _ => {}
        }
    }

    fn visit_item_fn(&mut self, item_fn: &ItemFn, inherited_scope: ScopeKind) {
        let scope = classify_scope(
            &item_fn.attrs,
            inherited_scope,
            Some(item_fn.sig.ident == "tests"),
        );
        if scope == ScopeKind::NonTest {
            self.visit_block(&item_fn.block, &item_fn.sig.ident.to_string());
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
        if scope != ScopeKind::NonTest {
            return;
        }
        for item in &item_impl.items {
            if let ImplItem::Fn(item_fn) = item {
                let fn_scope = classify_scope(&item_fn.attrs, scope, None);
                if fn_scope == ScopeKind::NonTest {
                    self.visit_block(&item_fn.block, &item_fn.sig.ident.to_string());
                }
            }
        }
    }

    fn visit_block(&mut self, block: &Block, label: &str) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt, label);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt, label: &str) {
        match stmt {
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    if discarded_timeout_wait_pattern(&local.pat) {
                        self.check_discarded_timeout_wait(&init.expr, label);
                    }
                    self.visit_expr(&init.expr, label);
                }
            }
            Stmt::Expr(expr, Some(_semi)) => {
                self.check_discarded_timeout_wait(expr, label);
                self.visit_expr(expr, label);
            }
            Stmt::Expr(expr, None) => self.visit_expr(expr, label),
            Stmt::Item(item) => self.visit_item(item, ScopeKind::NonTest),
            Stmt::Macro(_) => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr, label: &str) {
        match expr {
            Expr::MethodCall(method_call) => {
                self.check_bare_wait(method_call, label);
                self.visit_expr(&method_call.receiver, label);
                for arg in &method_call.args {
                    self.visit_expr(arg, label);
                }
            }
            Expr::Block(expr_block) => self.visit_block(&expr_block.block, label),
            Expr::If(expr_if) => {
                self.visit_expr(&expr_if.cond, label);
                self.visit_block(&expr_if.then_branch, label);
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    self.visit_expr(else_expr, label);
                }
            }
            Expr::Match(expr_match) => {
                self.visit_expr(&expr_match.expr, label);
                for arm in &expr_match.arms {
                    self.visit_expr(&arm.body, label);
                }
            }
            Expr::Call(expr_call) => {
                self.visit_expr(&expr_call.func, label);
                for arg in &expr_call.args {
                    self.visit_expr(arg, label);
                }
            }
            Expr::Tuple(expr_tuple) => {
                for elem in &expr_tuple.elems {
                    self.visit_expr(elem, label);
                }
            }
            Expr::Paren(expr_paren) => self.visit_expr(&expr_paren.expr, label),
            Expr::Unary(expr_unary) => self.visit_expr(&expr_unary.expr, label),
            Expr::Reference(expr_reference) => self.visit_expr(&expr_reference.expr, label),
            Expr::Try(expr_try) => self.visit_expr(&expr_try.expr, label),
            _ => {}
        }
    }

    fn check_bare_wait(&mut self, method_call: &ExprMethodCall, label: &str) {
        if method_call.method != "wait" || method_call.args.len() != 1 {
            return;
        }
        // This is an AST approximation over `.wait(...)` calls and does not
        // attempt receiver type resolution before flagging the pattern.
        self.findings.push(RuntimeFinding::new(
            self.file_context,
            RuleId::ScbRuntime001,
            concat!(
                "bare Condvar::wait(...) in non-test production code; ",
                "use wait_timeout(...) or wait_timeout_while(...) and inspect the WaitTimeoutResult"
            ),
            SourcePosition::start(method_call),
            label,
        ));
    }

    fn check_discarded_timeout_wait(&mut self, expr: &Expr, label: &str) {
        if !contains_timeout_wait_call(expr) {
            return;
        }
        self.findings.push(RuntimeFinding::new(
            self.file_context,
            RuleId::ScbRuntime002,
            concat!(
                "wait_timeout* result discarded in non-test production code; ",
                "inspect the returned WaitTimeoutResult before proceeding"
            ),
            SourcePosition::start(expr),
            label,
        ));
    }
}

fn contains_timeout_wait_call(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(method_call) => {
            ["wait_timeout", "wait_timeout_while"]
                .iter()
                .any(|name| method_call.method == *name)
                || contains_timeout_wait_call(&method_call.receiver)
                || method_call.args.iter().any(contains_timeout_wait_call)
        }
        Expr::Try(expr_try) => contains_timeout_wait_call(&expr_try.expr),
        Expr::Paren(expr_paren) => contains_timeout_wait_call(&expr_paren.expr),
        Expr::Reference(expr_reference) => contains_timeout_wait_call(&expr_reference.expr),
        Expr::Unary(expr_unary) => contains_timeout_wait_call(&expr_unary.expr),
        Expr::Call(expr_call) => {
            contains_timeout_wait_call(&expr_call.func)
                || expr_call.args.iter().any(contains_timeout_wait_call)
        }
        _ => false,
    }
}

fn discarded_timeout_wait_pattern(pat: &Pat) -> bool {
    match pat {
        Pat::Wild(_) => true,
        Pat::Ident(pat_ident) => pat_ident.ident.to_string().starts_with('_'),
        Pat::Tuple(pat_tuple) => pat_tuple
            .elems
            .iter()
            .nth(1)
            .is_some_and(discarded_timeout_wait_pattern),
        _ => false,
    }
}
