use crate::api::{canonical_path, known_glob_path};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::Arc;

#[derive(Default)]
struct ImportScope {
    aliases: BTreeMap<String, String>,
    globs: Vec<String>,
    inherit: bool,
    module: Arc<str>,
}

#[derive(Default)]
pub(crate) struct Imports {
    scopes: Vec<ImportScope>,
    statics: BTreeSet<String>,
}

impl Imports {
    pub(crate) fn collect(file: &syn::File) -> Self {
        let mut imports = Self::default();
        collect_statics(&file.items, "", &mut imports.statics);
        imports.push_items(&file.items, false, Arc::from(""));
        imports
    }

    pub(crate) fn push_items<'a>(
        &mut self,
        items: impl IntoIterator<Item = &'a syn::Item>,
        inherit: bool,
        module: Arc<str>,
    ) {
        let mut scope = ImportScope {
            inherit,
            module,
            ..ImportScope::default()
        };
        for item in items {
            match item {
                syn::Item::Static(item) if crate::scope::production(&item.attrs, None) => {
                    scope.bind_static(item);
                }
                syn::Item::Use(item) if crate::scope::production(&item.attrs, None) => {
                    crate::imports::visit_use_tree(&item.tree, &mut |import| match import {
                        crate::imports::Import::Name { path, rename } => {
                            self.bind_name(&mut scope, path, rename)
                        }
                        crate::imports::Import::Glob(path) => self.bind_glob(&mut scope, path),
                    });
                }
                _ => (),
            }
        }
        self.scopes.push(scope);
    }

    fn bind_name(&self, scope: &mut ImportScope, mut path: Vec<String>, rename: Option<String>) {
        if path.last().is_some_and(|name| name == "self") {
            path.pop();
        }
        if let Some(name) = rename.or_else(|| path.last().cloned()) {
            let path = absolute_path(&scope.module, &path.join("::"));
            scope.aliases.insert(name, self.expand_aliases(path));
        }
    }

    fn bind_glob(&self, scope: &mut ImportScope, path: Vec<String>) {
        let path = self.expand_aliases(absolute_path(&scope.module, &path.join("::")));
        if let Some(inherited) = self
            .scopes
            .iter()
            .rev()
            .find(|source| module_path(&source.module) == path)
        {
            for (name, path) in &inherited.aliases {
                scope
                    .aliases
                    .entry(name.clone())
                    .or_insert_with(|| self.expand_aliases(path.clone()));
            }
            scope.globs.extend(inherited.globs.iter().cloned());
        }
        if !path.is_empty() {
            scope.globs.push(path);
        }
    }

    pub(crate) fn push_block(&mut self, block: &syn::Block) {
        self.push_items(
            block.stmts.iter().filter_map(|stmt| match stmt {
                syn::Stmt::Item(item) => Some(item),
                _ => None,
            }),
            true,
            self.scopes
                .last()
                .map(|scope| scope.module.clone())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn pop(&mut self) {
        self.scopes.pop();
    }

    fn visible(&self) -> impl Iterator<Item = &ImportScope> {
        let mut active = true;
        self.scopes.iter().rev().take_while(move |scope| {
            let include = active;
            active = scope.inherit;
            include
        })
    }

    pub(crate) fn resolve(&self, path: &syn::Path) -> Cow<'static, str> {
        let shadowed = path.segments.first().is_some_and(|first| {
            self.visible().any(|scope| {
                scope
                    .aliases
                    .keys()
                    .any(|name| first.ident == name.as_str())
            })
        });
        if !shadowed && let Some(canonical) = canonical_path(path) {
            return Cow::Borrowed(canonical);
        }
        let mut text = String::new();
        for part in &path.segments {
            if !text.is_empty() {
                text.push_str("::");
            }
            write!(&mut text, "{}", part.ident).expect("writing to String cannot fail");
        }
        let path = self.expand_aliases(text);
        for scope in self.visible() {
            for glob in &scope.globs {
                let candidate = self.expand_aliases(format!("{glob}::{path}"));
                if self.statics.contains(&candidate)
                    || known_glob_path(&candidate)
                    || candidate
                        .rsplit_once("::")
                        .is_some_and(|(parent, _)| known_glob_path(parent))
                {
                    return Cow::Owned(candidate);
                }
            }
        }
        Cow::Owned(path)
    }

    fn scoped_alias(&self, path: &str) -> Option<String> {
        let module = self.scopes.last()?.module.as_ref();
        let absolute = absolute_path(module, path);
        if !absolute.starts_with("crate::") {
            return None;
        }
        self.scopes.iter().rev().find_map(|scope| {
            let prefix = format!("{}::", module_path(&scope.module));
            let local = absolute.strip_prefix(&prefix)?;
            let (name, rest) = local.split_once("::").unwrap_or((local, ""));
            let target = scope.aliases.get(name)?;
            Some(if rest.is_empty() {
                target.clone()
            } else {
                format!("{target}::{rest}")
            })
        })
    }

    fn expand_aliases(&self, mut path: String) -> String {
        let limit = self
            .scopes
            .iter()
            .map(|scope| scope.aliases.len())
            .sum::<usize>();
        for _ in 0..=limit {
            if let Some(expanded) = self.scoped_alias(&path) {
                if expanded == path {
                    break;
                }
                path = expanded;
                continue;
            }
            let (first, rest) = path.split_once("::").unwrap_or((&path, ""));
            let Some(prefix) = self.visible().find_map(|scope| scope.aliases.get(first)) else {
                break;
            };
            if prefix.split("::").next() == Some(first) {
                break;
            }
            path = if rest.is_empty() {
                prefix.clone()
            } else {
                format!("{prefix}::{rest}")
            };
        }
        path
    }
}

impl ImportScope {
    fn bind_static(&mut self, item: &syn::ItemStatic) {
        self.aliases.insert(
            item.ident.to_string(),
            format!("{}::{}", module_path(&self.module), item.ident),
        );
    }
}

fn module_path(module: &str) -> String {
    if module.is_empty() {
        "crate".to_owned()
    } else {
        format!("crate::{module}")
    }
}

// Resolve relative module prefixes before binding an alias; its use site may
// belong to a different module or inherit the binding through a glob.
fn absolute_path(module: &str, path: &str) -> String {
    if let Some(path) = path.strip_prefix("self::") {
        return format!("{}::{path}", module_path(module));
    }
    if path == "self" {
        return module_path(module);
    }
    let mut parent = module;
    let mut rest = path;
    while let Some(suffix) = rest
        .strip_prefix("super::")
        .or_else(|| (rest == "super").then_some(""))
    {
        parent = parent.rsplit_once("::").map_or("", |(parent, _)| parent);
        rest = suffix;
    }
    if rest != path {
        let prefix = module_path(parent);
        return if rest.is_empty() {
            prefix
        } else {
            format!("{prefix}::{rest}")
        };
    }
    path.to_owned()
}

fn collect_statics(items: &[syn::Item], module: &str, result: &mut BTreeSet<String>) {
    for item in items {
        match item {
            syn::Item::Static(item) if crate::scope::production(&item.attrs, None) => {
                result.insert(format!("{}::{}", module_path(module), item.ident));
            }
            syn::Item::Mod(item)
                if crate::scope::production(&item.attrs, Some(item.ident == "tests")) =>
            {
                if let Some((_, items)) = &item.content {
                    collect_statics(
                        items,
                        &crate::imports::qualified(module, &item.ident.to_string()),
                        result,
                    );
                }
            }
            _ => (),
        }
    }
}
