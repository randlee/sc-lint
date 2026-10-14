use std::fs;
use std::path::Path;

use anyhow::Result;
use cargo_metadata::Metadata;

use super::config::DependencyOwnership;
use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

pub(crate) fn analyze(
    root: &Path,
    metadata: &Metadata,
    rules: &[DependencyOwnership],
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let packages = crate::workspace_packages(metadata);
    for rule in rules {
        for package in &packages {
            let manifest = sc_lint_analyzer_support::relative_slash_path(
                root,
                package.manifest_path.as_std_path(),
            );
            if package
                .dependencies
                .iter()
                .any(|dependency| dependency.name == rule.dependency.0)
                && !rule.manifests.iter().any(|path| path.display() == manifest)
            {
                findings.push(Finding {
                    rule_id: RuleId::ScbManifest004,
                    kind: "manifest_dependency_owner_violation".into(),
                    message: format!(
                        "{manifest}: {} (allowed manifests: {})",
                        rule.manifest_message,
                        displays(&rule.manifests)
                    ),
                    owner_ids: vec![OwnerId::new(manifest)],
                    node_ids: Vec::new(),
                });
            }
        }
        let crate_spelling = rule.dependency.0.replace('-', "_");
        for file in sc_lint_analyzer_support::discover_source_files(root)? {
            let relative = sc_lint_analyzer_support::relative_slash_path(root, &file.source_path);
            if rule.roots.iter().any(|path| {
                relative == path.display() || relative.starts_with(&format!("{}/", path.display()))
            }) {
                continue;
            }
            let source = fs::read_to_string(&file.source_path)?;
            if source_mentions_dependency(&source, &crate_spelling) {
                findings.push(Finding {
                    rule_id: RuleId::ScbManifest004,
                    kind: "source_dependency_owner_violation".into(),
                    message: format!(
                        "{relative}: {} (allowed source roots: {})",
                        rule.source_message,
                        displays(&rule.roots)
                    ),
                    owner_ids: vec![OwnerId::new(relative)],
                    node_ids: Vec::new(),
                });
            }
        }
    }
    Ok(findings)
}

fn source_mentions_dependency(source: &str, crate_name: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        !line.starts_with("//")
            && !line.starts_with("/*")
            && !line.starts_with('*')
            && (path_mentions(line, crate_name)
                || spaced_tokens_match(line, &["use", crate_name])
                || spaced_tokens_match(line, &["extern", "crate", crate_name]))
    })
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

fn displays(paths: &[super::config::PolicyPath]) -> String {
    paths
        .iter()
        .map(super::config::PolicyPath::display)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::source_mentions_dependency;

    #[test]
    fn path_mentions_require_a_word_boundary_before_the_crate_name() {
        assert!(source_mentions_dependency(
            "let c = db_driver::connect();",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "let c = my_db_driver::connect();",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "let c = db_driver_extra::connect();",
            "db_driver"
        ));
    }

    #[test]
    fn use_mentions_require_whole_crate_name_with_flexible_spacing() {
        assert!(source_mentions_dependency(
            "use db_driver::Client;",
            "db_driver"
        ));
        assert!(source_mentions_dependency("use   db_driver;", "db_driver"));
        assert!(source_mentions_dependency("use\tdb_driver;", "db_driver"));
        assert!(!source_mentions_dependency(
            "use db_driver_extra::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "use my_db_driver::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "useful db_driver;",
            "db_driver"
        ));
    }

    #[test]
    fn extern_crate_mentions_require_whole_crate_name_with_flexible_spacing() {
        assert!(source_mentions_dependency(
            "extern crate db_driver;",
            "db_driver"
        ));
        assert!(source_mentions_dependency(
            "extern   crate   db_driver;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "extern crate db_driver_extra;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "extern crate my_db_driver;",
            "db_driver"
        ));
    }

    #[test]
    fn comment_lines_are_not_mentions() {
        assert!(!source_mentions_dependency(
            "// use db_driver::Client;",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "/* db_driver::connect() */",
            "db_driver"
        ));
        assert!(!source_mentions_dependency(
            "* db_driver::connect()",
            "db_driver"
        ));
    }
}
