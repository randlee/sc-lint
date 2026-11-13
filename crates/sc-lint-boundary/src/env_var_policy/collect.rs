use std::collections::{BTreeMap, BTreeSet};

use sc_lint_analyzer_support::{ScopeKind, classify_scope, is_test_only, span_start_line};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, FnArg, Item, ItemFn, ItemMod, Lit, Pat};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Value {
    Literal(String),
    Parameter(usize),
}

#[derive(Clone, Debug)]
pub(super) struct Call {
    pub target: String,
    pub args: Vec<Option<Value>>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Function {
    pub path: String,
    pub direct: Vec<(Value, usize)>,
    pub calls: Vec<Call>,
}

struct BodyCollector {
    parameters: BTreeMap<String, usize>,
    direct: Vec<(Value, usize)>,
    calls: Vec<Call>,
}

impl BodyCollector {
    fn value(&self, expr: &Expr) -> Option<Value> {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Str(value) => Some(Value::Literal(value.value())),
                _ => None,
            },
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => self
                .parameters
                .get(&path.path.segments[0].ident.to_string())
                .copied()
                .map(Value::Parameter),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for BodyCollector {
    fn visit_item(&mut self, _item: &'ast Item) {}

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref() {
            let segments: Vec<_> = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            let target = segments.join("::");
            let args: Vec<_> = call.args.iter().map(|arg| self.value(arg)).collect();
            let line = span_start_line(call.span());
            if matches!(
                target.as_str(),
                "std::env::var" | "std::env::var_os" | "env::var" | "env::var_os"
            ) {
                if let Some(Some(value)) = args.first() {
                    self.direct.push((value.clone(), line));
                }
            } else {
                self.calls.push(Call { target, args, line });
            }
        }
        visit::visit_expr_call(self, call);
    }
}

struct FunctionCollector {
    module: Vec<String>,
    functions: Vec<Function>,
}

impl FunctionCollector {
    fn function(
        &mut self,
        name: &str,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
        block: &syn::Block,
    ) {
        let parameters = inputs
            .iter()
            .enumerate()
            .filter_map(|(index, input)| match input {
                FnArg::Typed(arg) => match arg.pat.as_ref() {
                    Pat::Ident(ident) => Some((ident.ident.to_string(), index)),
                    _ => None,
                },
                FnArg::Receiver(_) => None,
            })
            .collect();
        let mut body = BodyCollector {
            parameters,
            direct: Vec::new(),
            calls: Vec::new(),
        };
        body.visit_block(block);
        let path = self
            .module
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(name))
            .collect::<Vec<_>>()
            .join("::");
        self.functions.push(Function {
            path,
            direct: body.direct,
            calls: body.calls,
        });
    }
}

impl<'ast> Visit<'ast> for FunctionCollector {
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = sc_lint_analyzer_support::item_attrs(item);
        if classify_scope(
            attrs,
            ScopeKind::NonTest,
            sc_lint_analyzer_support::item_name_hint_is_tests(item),
        ) == ScopeKind::Test
        {
            return;
        }
        visit::visit_item(self, item);
    }
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        self.module.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.module.pop();
    }
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.function(&item.sig.ident.to_string(), &item.sig.inputs, &item.block);
        self.visit_block(&item.block);
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !is_test_only(&item.attrs) {
            self.function(&item.sig.ident.to_string(), &item.sig.inputs, &item.block);
        }
    }
}

pub(super) fn functions(file: &syn::File, prefix: Vec<String>) -> Vec<Function> {
    if is_test_only(&file.attrs) {
        return Vec::new();
    }
    let mut collector = FunctionCollector {
        module: prefix,
        functions: Vec::new(),
    };
    collector.visit_file(file);
    collector.functions
}

pub(super) fn summaries(functions: &[Function], readers: &[String]) -> Vec<BTreeSet<Value>> {
    let mut summaries: Vec<BTreeSet<Value>> = functions
        .iter()
        .map(|f| f.direct.iter().map(|(v, _)| v.clone()).collect())
        .collect();
    loop {
        let previous = summaries.clone();
        for (index, function) in functions.iter().enumerate() {
            for call in &function.calls {
                for callee in matching_functions(functions, &function.path, &call.target) {
                    if readers.contains(&functions[callee].path) {
                        continue;
                    }
                    for effect in &previous[callee] {
                        if let Some(value) = substitute(effect, &call.args) {
                            summaries[index].insert(value);
                        }
                    }
                }
            }
        }
        if summaries == previous {
            return summaries;
        }
    }
}

pub(super) fn matching_functions<'a>(
    functions: &'a [Function],
    caller: &str,
    target: &str,
) -> impl Iterator<Item = usize> + 'a {
    let mut module: Vec<_> = caller.split("::").collect();
    module.pop();
    let mut parts = target.split("::").peekable();
    match parts.peek().copied() {
        Some("crate") => {
            module.truncate(1);
            parts.next();
        }
        Some("self") => {
            parts.next();
        }
        Some("super") => {
            while parts.peek() == Some(&"super") {
                if module.len() > 1 {
                    module.pop();
                }
                parts.next();
            }
        }
        _ => {}
    }
    module.extend(parts);
    let resolved = module.join("::");
    functions
        .iter()
        .enumerate()
        .filter(move |(_, f)| f.path == resolved)
        .map(|(index, _)| index)
}

pub(super) fn substitute(value: &Value, args: &[Option<Value>]) -> Option<Value> {
    match value {
        Value::Literal(value) => Some(Value::Literal(value.clone())),
        Value::Parameter(index) => args.get(*index).cloned().flatten(),
    }
}
