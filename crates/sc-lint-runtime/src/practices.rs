//! Syntactic runtime-practice checks; no code under inspection is executed.
pub(crate) mod allowances;
mod bindings;
mod flow;
mod matches;
mod state;
use crate::RuleId;
use crate::api::{Api, lookup};
use crate::imports::{Imports, ModuleScoped, qualified};
use crate::position::SourcePosition;
use crate::scope::production;
use bindings::{ValueKind, type_kind};
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::Visit;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PracticeRule {
    ThreadSleepUntil,
    TokioSleep,
    BlockOn,
    MutexAcrossAwait,
}
impl PracticeRule {
    pub(crate) fn id(self) -> RuleId {
        match self {
            Self::ThreadSleepUntil => RuleId::ScbRuntime004,
            Self::TokioSleep => RuleId::ScbRuntime005,
            Self::BlockOn => RuleId::ScbRuntime006,
            Self::MutexAcrossAwait => RuleId::ScbRuntime007,
        }
    }
    pub(crate) fn recovery(self) -> &'static str {
        match self {
            Self::ThreadSleepUntil => "use controlled deadlines or synchronization instead",
            Self::TokioSleep => "use yield_now or event-driven synchronization instead of sleeping",
            Self::BlockOn => "await the future on the existing runtime instead",
            Self::MutexAcrossAwait => "drop the guard or end its scope before awaiting",
        }
    }
}

pub(crate) struct Site {
    pub(crate) rule: PracticeRule,
    pub(crate) position: SourcePosition,
    pub(crate) function: String,
    pub(crate) detail: String,
}

struct Binding {
    name: String,
    kind: ValueKind,
    guard: bool,
}
#[derive(Clone, Copy)]
struct BindingState {
    kind: ValueKind,
    guard: bool,
}
struct Scanner {
    imports: Imports,
    never_functions: BTreeSet<String>,
    fields: BTreeMap<String, BTreeMap<String, ValueKind>>,
    owner: String,
    module: String,
    bindings: Vec<Binding>,
    state_buffers: Vec<Vec<BindingState>>,
    asynchronous: bool,
    function: String,
    sites: Vec<Site>,
}

pub(crate) fn find(file: &syn::File) -> Vec<Site> {
    let mut imports = Imports::collect(file);
    let mut never = flow::NeverFunctions::default();
    never.visit_file(file);
    let mut fields = Fields {
        imports: &mut imports,
        fields: BTreeMap::new(),
        module: String::new(),
    };
    fields.visit_file(file);
    let fields = fields.fields;
    let mut scanner = Scanner {
        imports,
        never_functions: never.names,
        fields,
        owner: String::new(),
        module: String::new(),
        bindings: Vec::new(),
        state_buffers: Vec::new(),
        asynchronous: false,
        function: "<module>".into(),
        sites: Vec::new(),
    };
    scanner.visit_file(file);
    scanner.sites
}

struct Fields<'a> {
    imports: &'a mut Imports,
    fields: BTreeMap<String, BTreeMap<String, ValueKind>>,
    module: String,
}

fn type_owner(module: &str, path: &str) -> String {
    if let Some(path) = path.strip_prefix("crate::") {
        return path.to_owned();
    }
    if let Some(path) = path.strip_prefix("self::") {
        return qualified(module, path);
    }
    let mut module = module;
    let mut path = path;
    while let Some(parent) = path.strip_prefix("super::") {
        module = module.rsplit_once("::").map_or("", |(parent, _)| parent);
        path = parent;
    }
    qualified(module, path)
}

impl<'ast> Visit<'ast> for Fields<'_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.imports.push_block(block);
        syn::visit::visit_block(self, block);
        self.imports.pop();
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if production(&item.attrs, None) {
            self.fields
                .entry(self.module.clone())
                .or_default()
                .insert(item.ident.to_string(), type_kind(&item.ty, self.imports));
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if production(&item.attrs, Some(item.ident == "tests")) {
            self.with_module(item, |this| syn::visit::visit_item_mod(this, item));
        }
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if !production(&item.attrs, None) {
            return;
        }
        for field in &item.fields {
            if let Some(name) = &field.ident {
                self.fields
                    .entry(qualified(&self.module, &item.ident.to_string()))
                    .or_default()
                    .insert(name.to_string(), type_kind(&field.ty, self.imports));
            }
        }
        syn::visit::visit_item_struct(self, item);
    }
}

impl Scanner {
    fn report_live_guards(&mut self, expr: &syn::Expr) {
        if self.asynchronous {
            for index in 0..self.bindings.len() {
                if self.bindings[index].guard {
                    let detail = format!(
                        "std::sync::Mutex guard `{}` is live across .await",
                        self.bindings[index].name
                    );
                    self.record(PracticeRule::MutexAcrossAwait, expr, detail);
                }
            }
        }
    }
    fn record(&mut self, rule: PracticeRule, expr: &syn::Expr, detail: impl Into<String>) {
        self.sites.push(Site {
            rule,
            position: SourcePosition::start(expr),
            function: self.function.clone(),
            detail: detail.into(),
        });
    }
    fn function(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let bindings = std::mem::take(&mut self.bindings);
        let asynchronous = std::mem::replace(&mut self.asynchronous, sig.asyncness.is_some());
        let function = std::mem::replace(&mut self.function, sig.ident.to_string());
        for arg in &sig.inputs {
            if let syn::FnArg::Typed(arg) = arg
                && let Some(name) = pattern_name(&arg.pat)
            {
                let kind = type_kind(&arg.ty, &self.imports);
                self.bindings.push(Binding {
                    name,
                    kind,
                    guard: self.asynchronous && kind == ValueKind::Guard,
                });
            }
        }
        self.visit_block(block);
        self.bindings = bindings;
        self.asynchronous = asynchronous;
        self.function = function;
    }
    fn kind(&self, expr: &syn::Expr) -> ValueKind {
        match expr {
            syn::Expr::Path(path) => {
                if let Some(name) = path.path.get_ident()
                    && let Some(binding) = self
                        .bindings
                        .iter()
                        .rev()
                        .find(|binding| *name == binding.name)
                {
                    return binding.kind;
                }
                let resolved = type_owner(&self.module, &self.imports.resolve(&path.path));
                let (module, name) = resolved.rsplit_once("::").unwrap_or(("", &resolved));
                self.fields
                    .get(module)
                    .and_then(|fields| fields.get(name))
                    .copied()
                    .unwrap_or(ValueKind::Other)
            }
            syn::Expr::Field(field) if matches!(&*field.base, syn::Expr::Path(path) if path.path.is_ident("self")) => {
                if let syn::Member::Named(name) = &field.member {
                    self.fields
                        .get(&self.owner)
                        .and_then(|fields| {
                            fields
                                .iter()
                                .find(|(key, _)| *name == key.as_str())
                                .map(|(_, value)| *value)
                        })
                        .unwrap_or(ValueKind::Other)
                } else {
                    ValueKind::Other
                }
            }
            syn::Expr::Closure(expr) if flow::never_output(&expr.output) => {
                ValueKind::NeverCallable
            }
            syn::Expr::Reference(expr) => self.kind(&expr.expr),
            syn::Expr::Paren(expr) => self.kind(&expr.expr),
            syn::Expr::Try(expr) => self.kind(&expr.expr),
            syn::Expr::MethodCall(call) => self.method_kind(call),
            syn::Expr::Call(call) => self.call_kind(call),
            _ => ValueKind::Other,
        }
    }
    fn method_kind(&self, call: &syn::ExprMethodCall) -> ValueKind {
        let receiver = self.kind(&call.receiver);
        let method = &call.method;
        if ["unwrap", "expect", "clone"]
            .iter()
            .any(|name| *method == *name)
        {
            receiver
        } else if *method == "into_inner" && receiver == ValueKind::PoisonedGuard {
            ValueKind::Guard
        } else if *method == "handle" && receiver == ValueKind::Runtime {
            ValueKind::Handle
        } else if *method == "build" && receiver == ValueKind::Builder {
            ValueKind::Runtime
        } else if receiver == ValueKind::Builder {
            ValueKind::Builder
        } else {
            ValueKind::Other
        }
    }
    fn call_kind(&self, call: &syn::ExprCall) -> ValueKind {
        let syn::Expr::Path(path) = &*call.func else {
            return ValueKind::Other;
        };
        let path = self.imports.resolve(&path.path);
        match lookup(&path) {
            Some(Api::MutexNew) => ValueKind::Mutex,
            Some(Api::RuntimeNew) => ValueKind::Runtime,
            Some(Api::HandleCurrent) => ValueKind::Handle,
            Some(Api::BuilderNew) => ValueKind::Builder,
            Some(Api::OwnerConstructor) => call
                .args
                .first()
                .map_or(ValueKind::Other, |arg| self.kind(arg)),
            _ => ValueKind::Other,
        }
    }
    fn guard(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::MethodCall(call)
                if ["unwrap", "expect", "unwrap_or_else"]
                    .iter()
                    .any(|name| call.method == *name) =>
            {
                self.guard(&call.receiver)
            }
            syn::Expr::MethodCall(call) => {
                ["lock", "try_lock"].iter().any(|name| call.method == *name)
                    && self.kind(&call.receiver) == ValueKind::Mutex
            }
            syn::Expr::Match(branch) => self.match_result_guard(branch),
            syn::Expr::Try(expr) => self.guard(&expr.expr),
            syn::Expr::Paren(expr) => self.guard(&expr.expr),
            syn::Expr::Call(call) => {
                matches!(&*call.func, syn::Expr::Path(path) if matches!(lookup(&self.imports.resolve(&path.path)), Some(Api::MutexLock | Api::MutexTryLock)))
            }
            _ => false,
        }
    }
    fn call(&mut self, call: &syn::ExprCall, expr: &syn::Expr) {
        let syn::Expr::Path(path) = &*call.func else {
            syn::visit::visit_expr_call(self, call);
            return;
        };
        let path = self.imports.resolve(&path.path);
        let rule = match lookup(&path) {
            Some(Api::ThreadSleepUntil) => Some(PracticeRule::ThreadSleepUntil),
            Some(Api::TokioSleep | Api::TokioSleepUntil) => Some(PracticeRule::TokioSleep),
            Some(Api::BlockOn) => Some(PracticeRule::BlockOn),
            _ => None,
        };
        if let Some(rule) = rule {
            self.record(rule, expr, path.clone());
        }
        syn::visit::visit_expr_call(self, call);
        if lookup(&path) == Some(Api::Drop)
            && call.args.len() == 1
            && let Some(name) = call.args.first().and_then(expression_name)
            && let Some(binding) = self
                .bindings
                .iter_mut()
                .rev()
                .find(|binding| binding.name == name)
            && binding.kind != ValueKind::BorrowedGuard
        {
            binding.guard = false;
        }
    }
}

impl<'ast> Visit<'ast> for Scanner {
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_const(self, item);
        }
    }
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if production(&item.attrs, None) {
            self.function(&item.sig, &item.block);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if production(&item.attrs, None) {
            self.function(&item.sig, &item.block);
        }
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if production(&item.attrs, None)
            && let Some(block) = &item.default
        {
            self.function(&item.sig, block);
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if production(&item.attrs, Some(item.ident == "tests")) {
            self.with_module(item, |this| syn::visit::visit_item_mod(this, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !production(&item.attrs, None) {
            return;
        }
        let owner = self.owner.clone();
        if let syn::Type::Path(path) = &*item.self_ty {
            self.owner = type_owner(&self.module, &self.imports.resolve(&path.path));
        }
        syn::visit::visit_item_impl(self, item);
        self.owner = owner;
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_trait(self, item);
        }
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let count = self.bindings.len();
        self.imports.push_block(block);
        syn::visit::visit_block(self, block);
        self.imports.pop();
        self.bindings.truncate(count);
    }
    fn visit_expr_assign(&mut self, expr: &'ast syn::ExprAssign) {
        self.visit_expr(&expr.right);
        self.visit_expr(&expr.left);
        let kind = self.kind(&expr.right);
        let guard = self.asynchronous && self.guard(&expr.right);
        if let Some(name) = expression_name(&expr.left)
            && let Some(binding) = self
                .bindings
                .iter_mut()
                .rev()
                .find(|binding| binding.name == name)
        {
            binding.kind = kind;
            binding.guard = guard;
        }
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !production(&local.attrs, None) {
            return;
        }
        syn::visit::visit_local(self, local);
        if let Some(init) = &local.init
            && init.diverge.is_some()
            && let Some(binding) = self.pattern_guard(&local.pat, &init.expr)
        {
            self.bindings.push(binding);
        }
        if let Some(name) = pattern_name(&local.pat) {
            let kind = if let syn::Pat::Type(pat) = &local.pat {
                type_kind(&pat.ty, &self.imports)
            } else {
                local
                    .init
                    .as_ref()
                    .map_or(ValueKind::Other, |init| self.kind(&init.expr))
            };
            let guard = self.asynchronous
                && (kind == ValueKind::Guard
                    || local
                        .init
                        .as_ref()
                        .is_some_and(|init| self.guard(&init.expr)));
            self.bindings.push(Binding { name, kind, guard });
        }
    }
    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        if !production(expression_attrs(expr), None) {
            return;
        }
        match expr {
            syn::Expr::Call(call) => self.call(call, expr),
            syn::Expr::MethodCall(call) => {
                if call.method == "block_on"
                    && matches!(
                        self.kind(&call.receiver),
                        ValueKind::Runtime | ValueKind::Handle
                    )
                {
                    self.record(
                        PracticeRule::BlockOn,
                        expr,
                        "Tokio Runtime/Handle::block_on",
                    );
                }
                syn::visit::visit_expr_method_call(self, call);
                self.release_poisoned_guard(call);
            }
            syn::Expr::Await(awaited) => {
                self.visit_expr(&awaited.base);
                self.report_live_guards(expr);
            }
            syn::Expr::Block(block) if block.label.is_some() => {
                self.conditional(|this| this.visit_block(&block.block))
            }
            syn::Expr::Async(block) => self.deferred(true, |this| this.visit_block(&block.block)),
            syn::Expr::Closure(closure) => self.deferred(closure.asyncness.is_some(), |this| {
                this.visit_expr(&closure.body)
            }),
            syn::Expr::If(branch) => self.visit_conditional(branch),
            syn::Expr::Match(branch) => self.visit_match_branches(branch),
            syn::Expr::ForLoop(looped) => {
                self.visit_expr(&looped.expr);
                self.conditional(|this| this.visit_block(&looped.body));
            }
            syn::Expr::While(looped) => {
                self.visit_expr(&looped.cond);
                let binding = self.condition_guard(&looped.cond);
                self.conditional(|this| {
                    let count = this.bindings.len();
                    this.bindings.extend(binding);
                    this.visit_block(&looped.body);
                    this.bindings.truncate(count);
                });
            }
            syn::Expr::Loop(looped) => self.conditional(|this| this.visit_block(&looped.body)),
            _ => syn::visit::visit_expr(self, expr),
        }
    }
}

fn pattern_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(pat) => Some(pat.ident.to_string()),
        syn::Pat::Type(pat) => pattern_name(&pat.pat),
        syn::Pat::Paren(pat) => pattern_name(&pat.pat),
        _ => None,
    }
}
fn expression_name(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        syn::Expr::Paren(expr) => expression_name(&expr.expr),
        _ => None,
    }
}

fn expression_attrs(expr: &syn::Expr) -> &[syn::Attribute] {
    match expr {
        syn::Expr::Call(expr) => &expr.attrs,
        syn::Expr::MethodCall(expr) => &expr.attrs,
        syn::Expr::Await(expr) => &expr.attrs,
        syn::Expr::Async(expr) => &expr.attrs,
        syn::Expr::Block(expr) => &expr.attrs,
        syn::Expr::Closure(expr) => &expr.attrs,
        syn::Expr::If(expr) => &expr.attrs,
        syn::Expr::Match(expr) => &expr.attrs,
        syn::Expr::Loop(expr) => &expr.attrs,
        syn::Expr::While(expr) => &expr.attrs,
        syn::Expr::ForLoop(expr) => &expr.attrs,
        _ => &[],
    }
}

impl ModuleScoped for Scanner {
    fn module_scope(&mut self) -> (&mut Imports, &mut String) {
        (&mut self.imports, &mut self.module)
    }
}
impl ModuleScoped for Fields<'_> {
    fn module_scope(&mut self) -> (&mut Imports, &mut String) {
        (&mut *self.imports, &mut self.module)
    }
}
