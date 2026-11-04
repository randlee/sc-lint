//! One use-tree traversal, with binding policy supplied by each runtime pass.
//! Scoped alias resolution is shared by every runtime rule.
pub(crate) enum Import {
    Name {
        path: Vec<String>,
        rename: Option<String>,
    },
    Glob(Vec<String>),
}

pub(crate) fn visit_use_tree(tree: &syn::UseTree, consume: &mut impl FnMut(Import)) {
    walk(tree, Vec::new(), consume);
}

fn walk(tree: &syn::UseTree, mut path: Vec<String>, consume: &mut impl FnMut(Import)) {
    match tree {
        syn::UseTree::Path(branch) => {
            path.push(branch.ident.to_string());
            walk(&branch.tree, path, consume);
        }
        syn::UseTree::Name(name) => {
            path.push(name.ident.to_string());
            consume(Import::Name { path, rename: None });
        }
        syn::UseTree::Rename(rename) => {
            path.push(rename.ident.to_string());
            consume(Import::Name {
                path,
                rename: Some(rename.rename.to_string()),
            });
        }
        syn::UseTree::Glob(_) => consume(Import::Glob(path)),
        syn::UseTree::Group(group) => {
            for child in &group.items {
                walk(child, path.clone(), consume);
            }
        }
    }
}

mod resolver;
pub(crate) use resolver::Imports;

pub(crate) fn qualified(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_owned()
    } else {
        format!("{module}::{name}")
    }
}

pub(crate) trait ModuleScoped {
    fn module_scope(&mut self) -> (&mut Imports, &mut String);

    fn with_module(&mut self, item: &syn::ItemMod, visit: impl FnOnce(&mut Self))
    where
        Self: Sized,
    {
        let Some((_, items)) = &item.content else {
            return;
        };
        let previous = {
            let (imports, module) = self.module_scope();
            let next = qualified(module, &item.ident.to_string());
            let previous = std::mem::replace(module, next);
            imports.push_items(items, false, module.as_str().into());
            previous
        };
        visit(self);
        let (imports, module) = self.module_scope();
        imports.pop();
        *module = previous;
    }
}
