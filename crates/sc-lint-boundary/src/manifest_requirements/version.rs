use semver::Comparator;
use semver::Op;
use semver::Prerelease;
use semver::Version;
use semver::VersionReq;

use crate::RuleId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Violation {
    pub(super) rule_id: RuleId,
    pub(super) kind: &'static str,
    pub(super) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BoundComment {
    pub(super) version: Version,
}

pub(super) fn violations(
    raw_requirement: &str,
    bound_comment: Option<&BoundComment>,
) -> anyhow::Result<Vec<Violation>> {
    let requirement = VersionReq::parse(raw_requirement)
        .map_err(anyhow::Error::new)
        .map_err(|error| {
            error.context(format!("invalid semver requirement `{raw_requirement}`"))
        })?;
    let mut findings = Vec::new();

    if requirement
        .comparators
        .iter()
        .any(|comparator| matches!(comparator.op, Op::Exact | Op::Tilde))
    {
        findings.push(Violation {
            rule_id: RuleId::ScbManifest007,
            kind: "registry_dependency_exact_or_tilde_requirement",
            detail: format!("uses exact-or-tilde requirement `{raw_requirement}`"),
        });
    }

    let Some(lower) = lower_version(&requirement) else {
        return Ok(findings);
    };
    let caret = caret_range(&lower);
    for comparator in &requirement.comparators {
        if !matches!(comparator.op, Op::Less | Op::LessEq) {
            continue;
        }
        let upper = comparator_version(comparator);
        if !upper_excludes_caret(comparator, &lower, &caret) {
            continue;
        }
        if bound_comment.is_some_and(|comment| comment.version == upper) {
            continue;
        }
        let detail = if bound_comment.is_some() {
            format!(
                "has extra upper bound `{raw_requirement}` whose bound comment names a different version"
            )
        } else {
            format!("has extra upper bound `{raw_requirement}` without a valid bound comment")
        };
        findings.push(Violation {
            rule_id: RuleId::ScbManifest008,
            kind: "registry_dependency_unexplained_upper_bound",
            detail,
        });
    }
    Ok(findings)
}

pub(super) fn bound_comment_on(text: &str, line: usize) -> Option<BoundComment> {
    let lines = text.lines().collect::<Vec<_>>();
    let candidates = [line.checked_sub(1), line.checked_sub(2)];
    candidates
        .into_iter()
        .flatten()
        .filter_map(|index| lines.get(index).copied())
        .find_map(parse_bound_comment)
}

fn parse_bound_comment(line: &str) -> Option<BoundComment> {
    let (_, comment) = line.split_once('#')?;
    let value = comment.trim().strip_prefix("upper-bound:")?.trim();
    let mut words = value.split_whitespace();
    let kind = words.next()?;
    if !matches!(kind, "api" | "links" | "toolchain")
        || words.next()? != "break"
        || words.next()? != "first"
        || words.next()? != "in"
    {
        return None;
    }
    let version = words.next()?;
    if words.next().is_some() {
        return None;
    }
    Some(BoundComment {
        version: Version::parse(version).ok()?,
    })
}

#[derive(Debug, Clone)]
struct CaretRange {
    exclusive_upper: Version,
    singleton: bool,
}

fn caret_range(lower: &Version) -> CaretRange {
    let (exclusive_upper, singleton) = if lower.major > 0 {
        (Version::new(lower.major + 1, 0, 0), false)
    } else if lower.minor > 0 {
        (Version::new(0, lower.minor + 1, 0), false)
    } else {
        (lower.clone(), true)
    };
    CaretRange {
        exclusive_upper,
        singleton,
    }
}

fn lower_version(requirement: &VersionReq) -> Option<Version> {
    requirement
        .comparators
        .iter()
        .filter_map(|comparator| match comparator.op {
            Op::Exact | Op::Greater | Op::GreaterEq | Op::Caret | Op::Tilde | Op::Wildcard => {
                Some(effective_lower(comparator))
            }
            Op::Less | Op::LessEq => None,
            _ => None,
        })
        .max()
}

fn effective_lower(comparator: &Comparator) -> Version {
    let mut version = comparator_version(comparator);
    if comparator.op == Op::Greater {
        if comparator.minor.is_none() {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
            version.pre = Prerelease::EMPTY;
        } else if comparator.patch.is_none() {
            version.minor += 1;
            version.patch = 0;
            version.pre = Prerelease::EMPTY;
        }
    }
    version
}

fn comparator_version(comparator: &Comparator) -> Version {
    let mut version = Version::new(
        comparator.major,
        comparator.minor.unwrap_or(0),
        comparator.patch.unwrap_or(0),
    );
    version.pre = comparator.pre.clone();
    version
}

fn upper_excludes_caret(comparator: &Comparator, lower: &Version, caret: &CaretRange) -> bool {
    let upper = comparator_version(comparator);
    if caret.singleton {
        return match comparator.op {
            Op::Less => upper <= *lower,
            Op::LessEq => upper < *lower,
            _ => false,
        };
    }

    let effective_upper = if comparator.op == Op::LessEq
        && (comparator.minor.is_none() || comparator.patch.is_none())
    {
        if comparator.minor.is_none() {
            Version::new(upper.major + 1, 0, 0)
        } else {
            Version::new(upper.major, upper.minor + 1, 0)
        }
    } else {
        upper
    };
    effective_upper < caret.exclusive_upper
}

#[cfg(test)]
mod tests {
    use super::BoundComment;
    use super::bound_comment_on;
    use super::violations;
    use crate::RuleId;
    use semver::Version;

    fn rule_ids(requirement: &str, comment: Option<BoundComment>) -> Vec<RuleId> {
        violations(requirement, comment.as_ref())
            .expect("requirement parses")
            .into_iter()
            .map(|violation| violation.rule_id)
            .collect()
    }

    #[test]
    fn exact_and_tilde_are_flagged_but_caret_is_allowed() {
        assert_eq!(rule_ids("=1.2.3", None), [RuleId::ScbManifest007]);
        assert_eq!(rule_ids("~1.2.3", None), [RuleId::ScbManifest007]);
        assert!(rule_ids("^1.2.3", None).is_empty());
    }

    #[test]
    fn equivalent_caret_upper_bound_is_allowed_and_extra_bound_needs_matching_comment() {
        assert!(rule_ids(">=1.2.3, <2.0.0", None).is_empty());
        assert_eq!(rule_ids(">=1.2.3, <1.8.0", None), [RuleId::ScbManifest008]);
        let matching = BoundComment {
            version: Version::parse("1.8.0").unwrap(),
        };
        assert!(rule_ids(">=1.2.3, <1.8.0", Some(matching)).is_empty());
    }

    #[test]
    fn caret_ranges_follow_zero_major_and_prerelease_rules() {
        assert!(rule_ids(">=0.2.3, <0.3.0", None).is_empty());
        assert_eq!(rule_ids(">=0.2.3, <0.2.8", None), [RuleId::ScbManifest008]);
        assert!(rule_ids(">=0.0.4, <0.0.5", None).is_empty());
        assert_eq!(rule_ids(">=0.0.4, <0.0.4", None), [RuleId::ScbManifest008]);
        assert!(rule_ids(">=1.2.3-alpha.1, <2.0.0", None).is_empty());
        assert_eq!(
            rule_ids(">=1.2.3-alpha.1, <1.2.3", None),
            [RuleId::ScbManifest008]
        );
    }

    #[test]
    fn bound_comment_may_be_on_the_requirement_line_or_the_line_above() {
        let text = "# upper-bound: api break first in 1.8.0\nfoo = \"<1.8.0\" # other\nbar = \"<2.0.0\" # upper-bound: links break first in 2.0.0\n";
        let above = bound_comment_on(text, 2).expect("comment above");
        assert_eq!(above.version, Version::parse("1.8.0").unwrap());
        let same = bound_comment_on(text, 3).expect("same-line comment");
        assert_eq!(same.version, Version::parse("2.0.0").unwrap());
    }
}
