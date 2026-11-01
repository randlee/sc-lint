mod allowlist;
mod config;
mod ownership;
mod sections;
mod source_uses;

use std::path::Path;

use anyhow::Result;
use cargo_metadata::Metadata;

pub(crate) use config::ManifestPolicyConfig;
pub(crate) use config::parse_policy;

use crate::Finding;
use crate::inventory::ValidatedInventory;

pub(crate) fn analyze_policy(
    root: &Path,
    metadata: &Metadata,
    inventory: Option<&ValidatedInventory>,
    config: &ManifestPolicyConfig,
) -> Result<Vec<Finding>> {
    if config.is_empty() {
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    findings.extend(allowlist::analyze(
        root,
        metadata,
        inventory,
        &config.allowlists,
    )?);
    findings.extend(ownership::analyze(root, metadata, &config.ownership)?);
    findings.extend(sections::analyze(root, metadata, &config.sections)?);
    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });
    Ok(findings)
}
