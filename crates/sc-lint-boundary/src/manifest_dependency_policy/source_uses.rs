//! Whether a Rust source file uses a crate, judged by the first segment of
//! each path (#136). A path whose first segment is `X` or `::X` uses crate
//! `X`; paths that start with `self`, `crate` or `super` never do, and a bare
//! `X::` path is a local module path when the same module declares `mod X`.

use std::collections::BTreeSet;

use std::fmt::Display;

use syn::Ident;
use syn::Item;
use syn::MacroDelimiter;
use syn::Meta;
use syn::Path;
use syn::QSelf;
use syn::UseTree;
use syn::visit::Visit;

/// Whether `source` uses `crate_name` (its underscore spelling). A file that
/// does not parse keeps the textual rule, so a mention there still counts.
pub(super) fn source_mentions_dependency(source: &str, crate_name: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return source_mentions_dependency_textual_fallback(source, crate_name);
    };
    let mut uses = DependencyUses {
        crate_name,
        local_modules: vec![local_modules(&file.items)],
        found: false,
    };
    uses.visit_file(&file);
    uses.found
}

/// The legacy line-based rule: `\bX::`, `use X` or `extern crate X` on a line
/// that does not start a comment. Used only for macro token bodies and files
/// that fail to parse.
pub(super) fn source_mentions_dependency_textual_fallback(source: &str, dependency: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        !line.starts_with("//")
            && !line.starts_with("/*")
            && !line.starts_with('*')
            && (path_mentions(line, dependency)
                || spaced_tokens_match(line, &["use", dependency])
                || spaced_tokens_match(line, &["extern", "crate", dependency]))
    })
}

struct DependencyUses<'a> {
    crate_name: &'a str,
    /// `mod` names declared by each enclosing module, innermost last.
    local_modules: Vec<BTreeSet<String>>,
    found: bool,
}

impl DependencyUses<'_> {
    /// Whether a path starting with `first` names the crate. An absolute
    /// `::X` path always does; a bare `X` path does unless the current module
    /// declares `mod X`.
    fn first_segment_names_crate(&self, first: &Ident, absolute: bool) -> bool {
        first == self.crate_name
            && (absolute
                || !self
                    .local_modules
                    .last()
                    .is_some_and(|modules| modules.contains(self.crate_name)))
    }

    fn use_tree_names_crate(&self, tree: &UseTree, absolute: bool) -> bool {
        match tree {
            UseTree::Path(path) => self.first_segment_names_crate(&path.ident, absolute),
            UseTree::Name(name) => self.first_segment_names_crate(&name.ident, absolute),
            UseTree::Rename(rename) => self.first_segment_names_crate(&rename.ident, absolute),
            UseTree::Glob(_) => false,
            UseTree::Group(group) => group
                .items
                .iter()
                .any(|item| self.use_tree_names_crate(item, absolute)),
        }
    }

    /// Macro and attribute token bodies are not parsed, so they keep the
    /// textual rule over their tokens rendered with their delimiters.
    fn check_tokens(&mut self, delimiter: &MacroDelimiter, tokens: &impl Display) {
        let (open, close) = match delimiter {
            MacroDelimiter::Paren(_) => ('(', ')'),
            MacroDelimiter::Brace(_) => ('{', '}'),
            MacroDelimiter::Bracket(_) => ('[', ']'),
        };
        let text = format!("{open}{tokens}{close}").replace(" ::", "::");
        if source_mentions_dependency_textual_fallback(&text, self.crate_name) {
            self.found = true;
        }
    }

    /// A `<T>::rest` path (qualified self at position 0) starts with an
    /// associated item, not a crate, so only its nested parts are visited.
    fn visit_qualified_path(&mut self, qself: &QSelf, path: &Path) {
        self.visit_qself(qself);
        for segment in &path.segments {
            self.visit_path_segment(segment);
        }
    }
}

impl<'ast> Visit<'ast> for DependencyUses<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let Some((_, items)) = &item.content else {
            syn::visit::visit_item_mod(self, item);
            return;
        };
        self.local_modules.push(local_modules(items));
        syn::visit::visit_item_mod(self, item);
        self.local_modules.pop();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if self.use_tree_names_crate(&item.tree, item.leading_colon.is_some()) {
            self.found = true;
        }
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if item.ident == self.crate_name {
            self.found = true;
        }
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_path(&mut self, path: &'ast Path) {
        let absolute = path.leading_colon.is_some();
        if let Some(first) = path.segments.first()
            && (absolute || path.segments.len() > 1)
            && self.first_segment_names_crate(&first.ident, absolute)
        {
            self.found = true;
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        match &type_path.qself {
            Some(qself) if qself.position == 0 => {
                self.visit_qualified_path(qself, &type_path.path);
            }
            _ => syn::visit::visit_type_path(self, type_path),
        }
    }

    fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
        match &expr_path.qself {
            Some(qself) if qself.position == 0 => {
                for attr in &expr_path.attrs {
                    self.visit_attribute(attr);
                }
                self.visit_qualified_path(qself, &expr_path.path);
            }
            _ => syn::visit::visit_expr_path(self, expr_path),
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.check_tokens(&mac.delimiter, &mac.tokens);
        syn::visit::visit_macro(self, mac);
    }

    fn visit_meta(&mut self, meta: &'ast Meta) {
        if let Meta::List(list) = meta {
            self.check_tokens(&list.delimiter, &list.tokens);
        }
        syn::visit::visit_meta(self, meta);
    }
}

fn local_modules(items: &[Item]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) => Some(module.ident.to_string()),
            _ => None,
        })
        .collect()
}

/// Matches `\bname::`: the crate name at a word start, directly followed by `::`.
fn path_mentions(line: &str, crate_name: &str) -> bool {
    line.match_indices(crate_name).any(|(start, _)| {
        at_word_start(line, start) && line[start + crate_name.len()..].starts_with("::")
    })
}

/// Matches `\bt0\s+t1\s+...tn\b`: the first token at a word start, each later token
/// after at least one whitespace, and the last token at a word end.
fn spaced_tokens_match(line: &str, tokens: &[&str]) -> bool {
    let Some((first, rest)) = tokens.split_first() else {
        return false;
    };
    line.match_indices(first).any(|(start, _)| {
        at_word_start(line, start) && tokens_follow(line, start + first.len(), rest)
    })
}

fn tokens_follow(line: &str, mut position: usize, tokens: &[&str]) -> bool {
    for (index, token) in tokens.iter().enumerate() {
        let rest = &line[position..];
        let trimmed = rest.trim_start();
        if trimmed.len() == rest.len() || !trimmed.starts_with(token) {
            return false;
        }
        position = line.len() - trimmed.len() + token.len();
        if index + 1 == tokens.len() {
            return at_word_end(line, position);
        }
    }
    true
}

fn at_word_start(line: &str, index: usize) -> bool {
    index == 0 || !is_word_byte(line.as_bytes()[index - 1])
}

fn at_word_end(line: &str, index: usize) -> bool {
    index == line.len() || !is_word_byte(line.as_bytes()[index])
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::source_mentions_dependency;
    use super::source_mentions_dependency_textual_fallback;

    #[test]
    fn path_mentions_require_a_word_boundary_before_the_crate_name() {
        assert!(source_mentions_dependency_textual_fallback(
            "let c = db_driver::connect();",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "let c = my_db_driver::connect();",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "let c = db_driver_extra::connect();",
            "db_driver"
        ));
    }

    #[test]
    fn use_mentions_require_whole_crate_name_with_flexible_spacing() {
        assert!(source_mentions_dependency_textual_fallback(
            "use db_driver::Client;",
            "db_driver"
        ));
        assert!(source_mentions_dependency_textual_fallback(
            "use   db_driver;",
            "db_driver"
        ));
        assert!(source_mentions_dependency_textual_fallback(
            "use\tdb_driver;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "use db_driver_extra::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "use my_db_driver::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "useful db_driver;",
            "db_driver"
        ));
    }

    #[test]
    fn extern_crate_mentions_require_whole_crate_name_with_flexible_spacing() {
        assert!(source_mentions_dependency_textual_fallback(
            "extern crate db_driver;",
            "db_driver"
        ));
        assert!(source_mentions_dependency_textual_fallback(
            "extern   crate   db_driver;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "extern crate db_driver_extra;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "extern crate my_db_driver;",
            "db_driver"
        ));
    }

    #[test]
    fn comment_lines_are_not_mentions() {
        assert!(!source_mentions_dependency_textual_fallback(
            "// use db_driver::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "/* db_driver::connect() */",
            "db_driver"
        ));
        assert!(!source_mentions_dependency_textual_fallback(
            "* db_driver::connect()",
            "db_driver"
        ));
    }

    #[test]
    fn parsed_paths_require_the_whole_crate_name_as_first_segment() {
        let mentions = |source| source_mentions_dependency(source, "db_driver");
        assert!(mentions("fn f() { let _c = db_driver::connect(); }"));
        assert!(mentions("use db_driver;"));
        assert!(mentions("use db_driver as driver;"));
        assert!(mentions("use {db_driver::Client, other::Thing};"));
        assert!(!mentions("fn f() { let _c = my_db_driver::connect(); }"));
        assert!(!mentions("fn f() { let _c = db_driver_extra::connect(); }"));
        assert!(!mentions(
            "fn f() { let _c = other::db_driver::connect(); }"
        ));
        assert!(!mentions("fn f(db_driver: u8) -> u8 { db_driver }"));
        assert!(!mentions("use other::*;"));
    }

    #[test]
    fn local_module_exemption_is_scoped_to_the_declaring_module() {
        let mentions = |source| source_mentions_dependency(source, "db_driver");
        assert!(!mentions("mod db_driver {}\nuse db_driver::Client;"));
        assert!(mentions(
            "mod db_driver {}\nmod inner { use db_driver::Client; }"
        ));
        assert!(!mentions(
            "mod inner { mod db_driver {} use db_driver::Client; }"
        ));
        assert!(mentions(
            "mod inner { mod db_driver {} }\nuse db_driver::Client;"
        ));
    }

    #[test]
    fn qualified_self_paths_start_with_an_associated_item() {
        let mentions = |source| source_mentions_dependency(source, "db_driver");
        assert!(!mentions(
            "fn f() -> <T as Tr>::db_driver::X { <T>::db_driver::x() }"
        ));
        assert!(mentions("fn f() { <db_driver::Client>::connect(); }"));
        assert!(mentions("type C = <T as db_driver::Tr>::Out;"));
    }

    #[test]
    fn attribute_token_lists_keep_the_textual_rule() {
        let mentions = |source| source_mentions_dependency(source, "db_driver");
        assert!(mentions("#[derive(db_driver::Row)]\nstruct S;"));
        assert!(!mentions("#[derive(Debug)]\nstruct S;"));
    }
}
