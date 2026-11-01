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
pub(crate) struct PortabilityFinding {
    pub(crate) rule_id: RuleId,
    pub(crate) kind: &'static str,
    pub(crate) message: String,
    pub(crate) source_path: PathBuf,
    pub(crate) line: usize,
    pub(crate) item: Option<String>,
    pub(crate) package: sc_lint_analyzer_support::PackageName,
    pub(crate) target: sc_lint_analyzer_support::TargetName,
    pub(crate) node_label: String,
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
            .then_with(|| left.kind.cmp(right.kind))
            .then_with(|| left.package.cmp(&right.package))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.node_label.cmp(&right.node_label))
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
