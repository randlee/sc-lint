//! Parallel per-file ingestion and deterministic merge for the graph build.

use super::ingest::ingest_module_items;
use super::*;
use rayon::prelude::*;

pub(super) fn merge_source_file_partials(
    builder: &mut GraphBuilder,
    pending_files: &mut Vec<SourceFileTask>,
) -> Result<()> {
    while !pending_files.is_empty() {
        pending_files.sort_by(|left, right| {
            left.source_path
                .cmp(&right.source_path)
                .then_with(|| left.context.crate_id.cmp(&right.context.crate_id))
                .then_with(|| left.module_path.as_str().cmp(right.module_path.as_str()))
        });
        let partials = pending_files
            .par_iter()
            .map(process_source_file)
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        // Determinism relies on two sorts: the per-round sort of `pending_files`
        // above fixes the order partials are merged in regardless of rayon
        // scheduling (`collect` preserves input order), and `GraphBuilder::finish`
        // sorts the final nodes and edges. Neither may be removed.
        let mut next_files = Vec::new();
        for partial in partials {
            builder.merge(partial.builder);
            next_files.extend(partial.pending_files);
        }
        *pending_files = next_files;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(super) struct SourceFileTask {
    pub(super) context: TargetContext,
    pub(super) parent_module_id: NodeId,
    pub(super) module_path: ModulePath,
    pub(super) module_dir: PathBuf,
    pub(super) source_path: PathBuf,
    pub(super) resolution_context: Option<ModulePath>,
}

struct SourceFilePartial {
    builder: GraphBuilder,
    pending_files: Vec<SourceFileTask>,
}

fn process_source_file(task: &SourceFileTask) -> Result<SourceFilePartial> {
    let file = if let Some(module_path) = &task.resolution_context {
        parse_rust_file(&task.source_path)
            .with_context(|| format!("while resolving module `{module_path}`"))?
    } else {
        parse_rust_file(&task.source_path)?
    };
    let mut builder = GraphBuilder::default();
    let mut pending_files = Vec::new();
    ingest_module_items(&mut builder, task, file, &mut pending_files)?;
    Ok(SourceFilePartial {
        builder,
        pending_files,
    })
}

impl GraphBuilder {
    fn merge(&mut self, partial: Self) {
        for node in partial.nodes {
            self.add_node(node);
        }
        for edge in partial.edges {
            if !self.edges.contains(&edge) {
                self.edges.push(edge);
            }
        }
        self.reference_impl_ids.extend(partial.reference_impl_ids);
    }
}
