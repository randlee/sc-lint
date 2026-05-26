use quote::ToTokens;
use syn::Attribute;
use syn::Expr;
use syn::ExprCall;
use syn::ExprLit;
use syn::ImplItem;
use syn::ImplItemFn;
use syn::Item;
use syn::Lit;
use syn::Meta;
use syn::Token;
use syn::UseTree;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

use crate::RuleId;
use crate::source_scan::ScopeKind;
use crate::source_scan::classify_scope;
use crate::source_scan::item_attrs;
use crate::source_scan::item_identifier;
use crate::source_scan::item_name_hint_is_tests;

/// Shell command names intentionally covered by PORT-009.
pub(crate) const UNIX_SHELL_COMMANDS: &[&str] = &["sh", "bash"];
/// Shell path literals intentionally covered by PORT-009 instead of the
/// generic PORT-006 Unix path matcher.
pub(crate) const UNIX_SHELL_PATHS: &[&str] = &[concat!("/bin", "/sh"), concat!("/bin", "/bash")];

pub(crate) fn attr_is_cfg_unix(attr: &Attribute) -> bool {
    attr_is_cfg_target(attr, "unix")
}

pub(crate) fn attr_is_platform_cfg(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let rendered = attr.meta.to_token_stream().to_string().replace(' ', "");
    rendered.contains("unix") || rendered.contains("windows")
}

pub(crate) fn attr_is_cfg_windows(attr: &Attribute) -> bool {
    attr_is_cfg_target(attr, "windows")
}

pub(crate) fn attr_is_cfg_attr_not_unix_allow_dead_code(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg_attr") {
        return false;
    }
    let rendered = attr.meta.to_token_stream().to_string().replace(' ', "");
    rendered.contains("not(unix)") && rendered.contains("allow(dead_code)")
}

pub(crate) fn use_tree_contains_std_os_unix(tree: &UseTree) -> bool {
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

pub(crate) fn is_path_like_constructor(expr: &Expr) -> bool {
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

pub(crate) fn is_cfg_unix_production_item(item: &Item, inherited_scope: ScopeKind) -> bool {
    // PORT-010 intentionally operates only on free functions, modules, and impl
    // methods. Struct, enum, and trait items are excluded from this parity pass
    // because they do not yet have a companion-shape contract in the sprint.
    classify_scope(
        item_attrs(item),
        inherited_scope,
        item_name_hint_is_tests(item),
    ) == ScopeKind::NonTest
        && item_attrs(item).iter().any(attr_is_cfg_unix)
        && item_identifier(item).is_some()
}

pub(crate) fn has_windows_companion(unix_item: &Item, sibling_items: &[Item]) -> bool {
    sibling_items.iter().any(|candidate| {
        same_item_identifier(unix_item, candidate) && item_has_cfg_windows(candidate)
    })
}

pub(crate) fn has_portable_fallback(
    unix_item: &Item,
    sibling_items: &[Item],
    inherited_scope: ScopeKind,
) -> bool {
    sibling_items.iter().any(|candidate| {
        same_item_identifier(unix_item, candidate)
            && !item_has_any_cfg(candidate)
            && !item_is_test_scoped(candidate, inherited_scope)
    })
}

pub(crate) fn same_item_identifier(left: &Item, right: &Item) -> bool {
    item_identifier(left) == item_identifier(right)
}

pub(crate) fn item_has_cfg_windows(item: &Item) -> bool {
    item_attrs(item).iter().any(attr_is_cfg_windows)
}

pub(crate) fn item_has_any_cfg(item: &Item) -> bool {
    item_attrs(item)
        .iter()
        .any(|attr| attr.path().is_ident("cfg"))
}

pub(crate) fn item_is_test_scoped(item: &Item, inherited_scope: ScopeKind) -> bool {
    classify_scope(
        item_attrs(item),
        inherited_scope,
        item_name_hint_is_tests(item),
    ) == ScopeKind::Test
}

pub(crate) fn impl_method_is_cfg_unix_production_item(
    item_fn: &ImplItemFn,
    inherited_scope: ScopeKind,
) -> bool {
    classify_scope(&item_fn.attrs, inherited_scope, None) == ScopeKind::NonTest
        && item_fn.attrs.iter().any(attr_is_cfg_unix)
}

pub(crate) fn impl_method_has_windows_companion(
    unix_method: &ImplItemFn,
    sibling_items: &[ImplItem],
) -> bool {
    sibling_items.iter().any(|candidate| {
        let ImplItem::Fn(candidate_fn) = candidate else {
            return false;
        };
        unix_method.sig.ident == candidate_fn.sig.ident
            && candidate_fn.attrs.iter().any(attr_is_cfg_windows)
    })
}

pub(crate) fn impl_method_has_portable_fallback(
    unix_method: &ImplItemFn,
    sibling_items: &[ImplItem],
    inherited_scope: ScopeKind,
) -> bool {
    sibling_items.iter().any(|candidate| {
        let ImplItem::Fn(candidate_fn) = candidate else {
            return false;
        };
        unix_method.sig.ident == candidate_fn.sig.ident
            && !candidate_fn
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("cfg"))
            && classify_scope(&candidate_fn.attrs, inherited_scope, None) == ScopeKind::NonTest
    })
}

pub(crate) fn is_dirs_home_dir_call(expr: &Expr) -> bool {
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

pub(crate) fn is_set_var_call(expr: &Expr) -> bool {
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

pub(crate) fn is_shell_command_call(expr_call: &ExprCall) -> Option<&'static str> {
    UNIX_SHELL_COMMANDS
        .iter()
        .copied()
        .find(|command_name| is_command_new_call(expr_call, command_name))
}

pub(crate) fn is_command_new_call(expr_call: &ExprCall, command_name: &str) -> bool {
    let Expr::Path(expr_path) = &*expr_call.func else {
        return false;
    };
    let segments: Vec<_> = expr_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let is_command_new = match segments.as_slice() {
        [command, new] => command == "Command" && new == "new",
        [process, command, new] => process == "process" && command == "Command" && new == "new",
        [std, process, command, new] => {
            std == "std" && process == "process" && command == "Command" && new == "new"
        }
        _ => false,
    };
    if !is_command_new {
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
    lit.value() == command_name
}

pub(crate) fn extract_env_var_lookup(expr_call: &ExprCall) -> Option<String> {
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

pub(crate) fn production_env_portability_variable(expr_call: &ExprCall) -> Option<String> {
    let variable_name = extract_env_var_lookup(expr_call)?;
    if variable_name == "HOME" || variable_name == "USER" || variable_name.starts_with("XDG_") {
        Some(variable_name)
    } else {
        None
    }
}

pub(crate) fn production_path_rule_id(value: &str) -> Option<RuleId> {
    if is_windows_path_literal(value) {
        Some(RuleId::Port007)
    } else if is_unix_path_literal(value) {
        Some(RuleId::Port006)
    } else {
        None
    }
}

pub(crate) fn is_unix_shell_path_literal(value: &str) -> bool {
    UNIX_SHELL_PATHS.contains(&value)
}

pub(crate) fn is_unix_path_literal(value: &str) -> bool {
    matches!(
        value,
        path if path.starts_with("/home/")
            || path.starts_with("/usr/")
            || path.starts_with("/etc/")
            || path.starts_with("/var/")
            || path.starts_with("/tmp/")
    )
}

pub(crate) fn is_windows_path_literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let unc_absolute = value.starts_with("\\\\") && value.len() > 2;
    drive_absolute || unc_absolute
}

fn attr_is_cfg_target(attr: &Attribute, target: &str) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    nested_cfg_metas(attr)
        .is_some_and(|metas| metas.iter().any(|meta| meta_is_cfg_target(meta, target)))
}

fn nested_cfg_metas(attr: &Attribute) -> Option<Punctuated<Meta, Token![,]>> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
}

fn meta_is_cfg_target(meta: &Meta, target: &str) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident(target),
        Meta::List(list) if list.path.is_ident("not") => false,
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .ok()
                .is_some_and(|metas| metas.iter().any(|meta| meta_is_cfg_target(meta, target)))
        }
        Meta::List(list) if list.path.is_ident("target_os") => false,
        Meta::NameValue(name_value) => {
            name_value.path.is_ident("target_os")
                && matches!(&name_value.value, Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) if lit.value() == target)
        }
        Meta::List(list) => Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .ok()
            .is_some_and(|metas| metas.iter().any(|meta| meta_is_cfg_target(meta, target))),
    }
}
