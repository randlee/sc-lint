//! One use-tree traversal, with binding policy supplied by each runtime pass.
//! Keep `self` in leaf paths so legacy rule 003 can retain its alias contract.
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
