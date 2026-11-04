use crate::api::{Api, lookup};
use crate::imports::Imports;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueKind {
    Mutex,
    Runtime,
    Handle,
    Builder,
    Guard,
    BorrowedGuard,
    PoisonedGuard,
    NeverCallable,
    Other,
}

pub(super) fn type_kind(ty: &syn::Type, imports: &Imports) -> ValueKind {
    match ty {
        syn::Type::FnPtr(function) if super::flow::never_output(&function.output) => {
            ValueKind::NeverCallable
        }
        syn::Type::Reference(reference) => type_kind(&reference.elem, imports),
        syn::Type::Paren(paren) => type_kind(&paren.elem, imports),
        syn::Type::Path(path) => {
            let resolved = imports.resolve(&path.path);
            match lookup(&resolved) {
                Some(Api::Mutex) => ValueKind::Mutex,
                Some(Api::MutexGuard) => ValueKind::Guard,
                Some(Api::Runtime) => ValueKind::Runtime,
                Some(Api::Handle) => ValueKind::Handle,
                Some(Api::Builder) => ValueKind::Builder,
                Some(Api::Owner) => wrapped_kind(path, imports),
                _ => ValueKind::Other,
            }
        }
        _ => ValueKind::Other,
    }
}

fn wrapped_kind(path: &syn::TypePath, imports: &Imports) -> ValueKind {
    if let Some(segment) = path.path.segments.last()
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
    {
        for arg in &args.args {
            if let syn::GenericArgument::Type(ty) = arg {
                return type_kind(ty, imports);
            }
        }
    }
    ValueKind::Other
}
