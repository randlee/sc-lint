use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use sc_lint_analyzer_support::RepoRelativePath;

use super::repo_relative_source_path;
use crate::CrateId;
use crate::Finding;
use crate::NodeId;
use crate::OwnerId;
use crate::RuleId;

#[derive(Debug, Clone)]
pub(super) struct PortabilityFinding {
    pub(super) rule_id: RuleId,
    pub(super) kind: &'static str,
    pub(super) message: String,
    pub(super) source_path: PathBuf,
    pub(super) line: usize,
    pub(super) package: sc_lint_analyzer_support::PackageName,
    pub(super) target: sc_lint_analyzer_support::TargetName,
    pub(super) node_label: String,
}

pub(super) fn convert_findings(
    root: &Path,
    mut findings: Vec<PortabilityFinding>,
) -> Result<Vec<(Finding, Option<RepoRelativePath>)>> {
    findings.sort_by(|left, right| {
        left.rule_id
            .as_str()
            .cmp(right.rule_id.as_str())
            .then_with(|| left.source_path.cmp(&right.source_path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.message.cmp(&right.message))
    });

    findings
        .into_iter()
        .map(|finding| {
            let source_path = repo_relative_source_path(root, &finding.source_path)?;
            Ok((
                Finding {
                    rule_id: finding.rule_id,
                    kind: finding.kind.into(),
                    message: format!(
                        "{}:{}: {}",
                        finding.source_path.display(),
                        finding.line,
                        finding.message
                    ),
                    owner_ids: vec![OwnerId::new(CrateId::from_parts(
                        finding.package.as_str(),
                        finding.target.as_str(),
                    ))],
                    node_ids: vec![NodeId::new(finding.node_label)],
                },
                Some(source_path),
            ))
        })
        .collect()
}
