use syn::Attribute;
use syn::Expr;
use syn::ExprCall;
use syn::ExprLit;
use syn::Lit;
use syn::Meta;
use syn::Token;
use syn::UseTree;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

use crate::RuleId;

/// Shell command names intentionally covered by PORT-009.
pub(crate) const UNIX_SHELL_COMMANDS: &[&str] = &["sh", "bash"];
/// Shell path literals intentionally covered by PORT-009 instead of the
/// generic PORT-006 Unix path matcher.
pub(crate) const UNIX_SHELL_PATHS: &[&str] = &[concat!("/bin", "/sh"), concat!("/bin", "/bash")];
/// Production-only Unix path prefixes intentionally covered by PORT-006.
pub(crate) const UNIX_PATH_PREFIXES: &[&str] = &[
    concat!("/", "home", "/"),
    concat!("/", "usr", "/"),
    concat!("/", "etc", "/"),
    concat!("/", "var", "/"),
    concat!("/", "tmp", "/"),
];

pub(crate) fn attr_is_cfg_unix(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    nested_cfg_metas(attr).is_some_and(|metas| metas.iter().any(meta_is_unix_cfg))
}

pub(crate) fn attr_is_platform_cfg(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    nested_cfg_metas(attr).is_some_and(|metas| {
        metas
            .iter()
            .any(|meta| meta_is_unix_cfg(meta) || meta_is_windows_cfg(meta))
    })
}

pub(crate) fn attr_is_cfg_attr_not_unix_allow_dead_code(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg_attr") {
        return false;
    }
    nested_cfg_metas(attr).is_some_and(|metas| {
        let mut metas = metas.iter();
        let Some(first) = metas.next() else {
            return false;
        };
        meta_is_not_unix_cfg(first) && metas.any(meta_is_allow_dead_code)
    })
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
    UNIX_PATH_PREFIXES
        .iter()
        .any(|prefix| value.starts_with(prefix))
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

fn nested_cfg_metas(attr: &Attribute) -> Option<Punctuated<Meta, Token![,]>> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
}

fn meta_is_unix_cfg(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("unix"),
        Meta::List(list) if list.path.is_ident("not") => false,
        Meta::List(list) => Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .ok()
            .is_some_and(|metas: Punctuated<Meta, Token![,]>| metas.iter().any(meta_is_unix_cfg)),
        Meta::NameValue(_) => false,
    }
}

fn meta_is_windows_cfg(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("windows"),
        Meta::List(list) if list.path.is_ident("not") => false,
        Meta::List(list) => Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .ok()
            .is_some_and(|metas: Punctuated<Meta, Token![,]>| {
                metas.iter().any(meta_is_windows_cfg)
            }),
        Meta::NameValue(_) => false,
    }
}

fn meta_is_not_unix_cfg(meta: &Meta) -> bool {
    let Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("not") {
        return false;
    }
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .is_some_and(|metas: Punctuated<Meta, Token![,]>| metas.iter().any(meta_is_unix_cfg))
}

fn meta_is_allow_dead_code(meta: &Meta) -> bool {
    let Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("allow") {
        return false;
    }
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .is_some_and(|metas: Punctuated<Meta, Token![,]>| {
            metas
                .iter()
                .any(|meta| matches!(meta, Meta::Path(path) if path.is_ident("dead_code")))
        })
}
