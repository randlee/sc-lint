use crate::api::{canonical_path, known_glob_path};
use std::borrow::Cow;
use std::collections::BTreeMap;
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
}

impl Imports {
    pub(crate) fn collect(file: &syn::File) -> Self {
        let mut imports = Self::default();
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
            if let syn::Item::Static(item) = item
                && crate::scope::production(&item.attrs, None)
            {
                let prefix = if scope.module.is_empty() {
                    "crate".to_owned()
                } else {
                    format!("crate::{}", scope.module)
                };
                scope
                    .aliases
                    .insert(item.ident.to_string(), format!("{prefix}::{}", item.ident));
            }
            if let syn::Item::Use(item) = item
                && crate::scope::production(&item.attrs, None)
            {
                crate::imports::visit_use_tree(&item.tree, &mut |import| match import {
                    crate::imports::Import::Name { mut path, rename } => {
                        if path.last().is_some_and(|name| name == "self") {
                            path.pop();
                        }
                        if let Some(name) = rename.or_else(|| path.last().cloned()) {
                            let path = path.join("::");
                            let path =
                                if !inherit && let Some(parent) = path.strip_prefix("super::") {
                                    self.expand_aliases(parent.to_owned())
                                } else if path.starts_with("crate::")
                                    || (inherit && path.starts_with("super::"))
                                {
                                    self.expand_aliases(path)
                                } else {
                                    path
                                };
                            scope.aliases.insert(name, path);
                        }
                    }
                    crate::imports::Import::Glob(path) => {
                        let inherited = match path.as_slice() {
                            [prefix] if prefix == "super" => {
                                self.scopes.iter().rev().find(|scope| !scope.inherit)
                            }
                            [prefix] if prefix == "crate" => self.scopes.first(),
                            _ => None,
                        };
                        if let Some(inherited) = inherited {
                            for (name, path) in &inherited.aliases {
                                scope
                                    .aliases
                                    .entry(name.clone())
                                    .or_insert_with(|| self.expand_aliases(path.clone()));
                            }
                            scope.globs.extend(inherited.globs.iter().cloned());
                        } else if !path.is_empty() {
                            scope.globs.push(path.join("::"));
                        }
                    }
                });
            }
        }
        self.scopes.push(scope);
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
                if known_glob_path(&candidate)
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
        let (scope, path) = if let Some(path) = path.strip_prefix("crate::") {
            (self.scopes.first()?, path)
        } else if let Some(path) = path.strip_prefix("self::") {
            (self.scopes.iter().rev().find(|scope| !scope.inherit)?, path)
        } else {
            let mut count = 0;
            let mut rest = path;
            while let Some(path) = rest.strip_prefix("super::") {
                count += 1;
                rest = path;
            }
            if count == 0 {
                return None;
            }
            (
                self.scopes
                    .iter()
                    .rev()
                    .filter(|scope| !scope.inherit)
                    .nth(count)?,
                rest,
            )
        };
        let (name, rest) = path.split_once("::").unwrap_or((path, ""));
        let prefix = scope.aliases.get(name)?;
        Some(if rest.is_empty() {
            prefix.clone()
        } else {
            format!("{prefix}::{rest}")
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
