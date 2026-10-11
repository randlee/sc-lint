use super::*;

#[derive(Default)]
pub(super) struct ReferenceCollector {
    owner_name: Option<String>,
    impl_self_type: Option<Type>,
    local_owner_names: BTreeSet<String>,
    workspace_dependency_roots: BTreeSet<String>,
    references: BTreeSet<CollectedReference>,
}

impl ReferenceCollector {
    fn new(
        local_owner_names: &BTreeSet<String>,
        owner_name: Option<&str>,
        workspace_dependency_roots: &BTreeMap<String, CrateId>,
    ) -> Self {
        Self {
            owner_name: owner_name.map(ToOwned::to_owned),
            impl_self_type: None,
            local_owner_names: local_owner_names.clone(),
            workspace_dependency_roots: workspace_dependency_roots.keys().cloned().collect(),
            references: BTreeSet::new(),
        }
    }

    pub(super) fn set_impl_self_type(&mut self, self_type: &Type) {
        self.impl_self_type = Some(self_type.clone());
    }

    fn qualified_method_path(&self, expression: &syn::ExprPath) -> Option<String> {
        let qself = expression.qself.as_ref()?;
        if qself.position == 0 || expression.path.segments.len() != qself.position + 1 {
            return None;
        }
        let is_self = matches!(qself.ty.as_ref(), Type::Path(path) if path.path.is_ident("Self"));
        let self_type = if is_self {
            self.impl_self_type.as_ref()?
        } else {
            qself.ty.as_ref()
        };
        let owner = impl_owner(self_type).ok()?;
        let trait_path = syn::Path {
            leading_colon: expression.path.leading_colon,
            segments: expression
                .path
                .segments
                .iter()
                .take(qself.position)
                .cloned()
                .collect(),
        };
        let method = expression.path.segments.last()?;
        Some(format!(
            "{}::{}::{}",
            owner.name,
            trait_impl_key(&owner, &trait_path),
            method.ident
        ))
    }

    fn into_references(self) -> BTreeSet<CollectedReference> {
        self.references
    }

    fn maybe_insert_path(&mut self, path: &syn::Path, kind: ReferenceKind) {
        let mut segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if segments.is_empty() {
            return;
        }

        if segments[0] == "Self" {
            if let Some(owner_name) = &self.owner_name {
                segments[0] = owner_name.clone();
                self.references.insert(CollectedReference {
                    path: segments.join("::"),
                    kind,
                });
            }
            return;
        }

        if matches!(kind, ReferenceKind::Expr) && segments.len() == 1 && segments[0] == "self" {
            return;
        }

        let first = &segments[0];
        // `segments` is non-empty because the early return above rejects empty paths.
        let last = segments.last().unwrap();
        let should_collect = matches!(first.as_str(), "crate" | "self" | "super")
            || self.workspace_dependency_roots.contains(first)
            || self.local_owner_names.contains(first)
            || self.local_owner_names.contains(last);

        if should_collect {
            self.references.insert(CollectedReference {
                path: segments.join("::"),
                kind,
            });
        }
    }
}

impl<'ast> Visit<'ast> for ReferenceCollector {
    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        self.maybe_insert_path(&type_path.path, ReferenceKind::Type);
        syn::visit::visit_type_path(self, type_path);
    }

    fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
        if let Some(path) = self.qualified_method_path(expr_path) {
            self.references.insert(CollectedReference {
                path,
                kind: ReferenceKind::Expr,
            });
        } else {
            self.maybe_insert_path(&expr_path.path, ReferenceKind::Expr);
        }
        syn::visit::visit_expr_path(self, expr_path);
    }

    fn visit_receiver(&mut self, _receiver: &'ast Receiver) {}
}

pub(super) fn collect_references_with(
    local_owner_names: &BTreeSet<String>,
    owner_name: Option<&str>,
    workspace_dependency_roots: &BTreeMap<String, CrateId>,
    visit: impl FnOnce(&mut ReferenceCollector),
) -> BTreeSet<CollectedReference> {
    let mut collector =
        ReferenceCollector::new(local_owner_names, owner_name, workspace_dependency_roots);
    visit(&mut collector);
    collector.into_references()
}
