use std::fmt;

use sc_lint_analyzer_support::AnalyzerName;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RuleCatalog;
use serde::Serialize;
use serde::Serializer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RuleId {
    ScbCycle001,
    ScbCycle002,
    ScbCycle003,
    ScbBoundary001,
    ScbBoundary002,
    ScbBoundary003,
    ScbBoundary004,
    ScbCaller001,
    ScbDependency001,
    ScbDependency002,
    ScbDependency003,
    ScbDependency004,
    ScbFeature001,
    ScbManifest001,
    ScbManifest002,
    ScbManifest003,
    ScbManifest004,
    ScbManifest005,
    ScbManifest006,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScbCycle001 => "SCB-CYCLE-001",
            Self::ScbCycle002 => "SCB-CYCLE-002",
            Self::ScbCycle003 => "SCB-CYCLE-003",
            Self::ScbBoundary001 => "SCB-BOUNDARY-001",
            Self::ScbBoundary002 => "SCB-BOUNDARY-002",
            Self::ScbBoundary003 => "SCB-BOUNDARY-003",
            Self::ScbBoundary004 => "SCB-BOUNDARY-004",
            Self::ScbCaller001 => "SCB-CALLER-001",
            Self::ScbDependency001 => "SCB-DEPENDENCY-001",
            Self::ScbDependency002 => "SCB-DEPENDENCY-002",
            Self::ScbDependency003 => "SCB-DEPENDENCY-003",
            Self::ScbDependency004 => "SCB-DEPENDENCY-004",
            Self::ScbFeature001 => "SCB-FEATURE-001",
            Self::ScbManifest001 => "SCB-MANIFEST-001",
            Self::ScbManifest002 => "SCB-MANIFEST-002",
            Self::ScbManifest003 => "SCB-MANIFEST-003",
            Self::ScbManifest004 => "SCB-MANIFEST-004",
            Self::ScbManifest005 => "SCB-MANIFEST-005",
            Self::ScbManifest006 => "SCB-MANIFEST-006",
        }
    }
}

impl Serialize for RuleId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFilter {
    Cycles,
    // SCB-CALLER-001 is enforced as part of boundary policy, so it routes
    // through the existing boundaries filter instead of a separate callers
    // variant.
    Boundaries,
    InternalOnly,
    ForbidExternalImpls,
    Dependencies,
    Manifests,
}

impl RuleFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cycles => "cycles",
            Self::Boundaries => "boundaries",
            Self::InternalOnly => "internal_only",
            Self::ForbidExternalImpls => "forbid_external_impls",
            Self::Dependencies => "dependencies",
            Self::Manifests => "manifests",
        }
    }
}

impl fmt::Display for RuleFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFilterParseError {
    invalid_value: String,
}

impl RuleFilterParseError {
    fn new(invalid_value: impl Into<String>) -> Self {
        Self {
            invalid_value: invalid_value.into(),
        }
    }
}

impl fmt::Display for RuleFilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported rule filter `{}`; supported: cycles, boundaries, internal_only, forbid_external_impls, dependencies, manifests",
            self.invalid_value
        )
    }
}

impl std::error::Error for RuleFilterParseError {}

impl TryFrom<&str> for RuleFilter {
    type Error = RuleFilterParseError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "cycles" => Ok(Self::Cycles),
            "boundaries" => Ok(Self::Boundaries),
            "internal_only" => Ok(Self::InternalOnly),
            "forbid_external_impls" => Ok(Self::ForbidExternalImpls),
            "dependencies" => Ok(Self::Dependencies),
            "manifests" => Ok(Self::Manifests),
            other => Err(RuleFilterParseError::new(other)),
        }
    }
}

pub(crate) const fn rule(id: RuleId) -> CatalogRuleId {
    CatalogRuleId::new(id.as_str())
}

pub const ALL_BOUNDARY_IDS: &[CatalogRuleId] = &[
    rule(RuleId::ScbCycle001),
    rule(RuleId::ScbCycle002),
    rule(RuleId::ScbCycle003),
    rule(RuleId::ScbBoundary001),
    rule(RuleId::ScbBoundary002),
    rule(RuleId::ScbBoundary003),
    rule(RuleId::ScbBoundary004),
    rule(RuleId::ScbCaller001),
    rule(RuleId::ScbDependency001),
    rule(RuleId::ScbDependency002),
    rule(RuleId::ScbDependency003),
    rule(RuleId::ScbDependency004),
    rule(RuleId::ScbFeature001),
    rule(RuleId::ScbManifest001),
    rule(RuleId::ScbManifest002),
    rule(RuleId::ScbManifest003),
    rule(RuleId::ScbManifest004),
    rule(RuleId::ScbManifest005),
    rule(RuleId::ScbManifest006),
];

/// Every id except SCB-CYCLE-002/003: the set `analysis::finding_is_failure` fails on.
pub const DEFAULT_FAIL_IDS: &[CatalogRuleId] = &[
    rule(RuleId::ScbCycle001),
    rule(RuleId::ScbBoundary001),
    rule(RuleId::ScbBoundary002),
    rule(RuleId::ScbBoundary003),
    rule(RuleId::ScbBoundary004),
    rule(RuleId::ScbCaller001),
    rule(RuleId::ScbDependency001),
    rule(RuleId::ScbDependency002),
    rule(RuleId::ScbDependency003),
    rule(RuleId::ScbDependency004),
    rule(RuleId::ScbFeature001),
    rule(RuleId::ScbManifest001),
    rule(RuleId::ScbManifest002),
    rule(RuleId::ScbManifest003),
    rule(RuleId::ScbManifest004),
    rule(RuleId::ScbManifest005),
    rule(RuleId::ScbManifest006),
];

/// Rules with findings that can be matched to repository source paths.
pub const PATH_ADDRESSABLE_IDS: &[CatalogRuleId] = &[
    rule(RuleId::ScbCycle002),
    rule(RuleId::ScbCycle003),
    rule(RuleId::ScbManifest001),
    rule(RuleId::ScbManifest002),
    rule(RuleId::ScbManifest003),
    rule(RuleId::ScbManifest004),
    rule(RuleId::ScbManifest005),
    rule(RuleId::ScbManifest006),
];

/// Families keep today's CLI spellings (kebab-case, as clap rendered the old
/// `RuleFilterArg`); members are what each `RuleFilter` arm runs.
pub const BOUNDARY_CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("boundary"),
    rules: ALL_BOUNDARY_IDS,
    path_addressable_rules: PATH_ADDRESSABLE_IDS,
    families: &[
        (
            "cycles",
            &[
                rule(RuleId::ScbCycle001),
                rule(RuleId::ScbCycle002),
                rule(RuleId::ScbCycle003),
            ],
        ),
        (
            "boundaries",
            &[
                rule(RuleId::ScbBoundary001),
                rule(RuleId::ScbBoundary002),
                rule(RuleId::ScbBoundary003),
                rule(RuleId::ScbBoundary004),
                rule(RuleId::ScbCaller001),
            ],
        ),
        (
            "internal-only",
            &[rule(RuleId::ScbBoundary001), rule(RuleId::ScbBoundary002)],
        ),
        ("forbid-external-impls", &[rule(RuleId::ScbBoundary003)]),
        (
            "dependencies",
            &[
                rule(RuleId::ScbBoundary004),
                rule(RuleId::ScbDependency001),
                rule(RuleId::ScbDependency002),
                rule(RuleId::ScbDependency003),
                rule(RuleId::ScbDependency004),
                rule(RuleId::ScbFeature001),
            ],
        ),
        (
            "manifests",
            &[
                rule(RuleId::ScbManifest001),
                rule(RuleId::ScbManifest002),
                rule(RuleId::ScbManifest003),
                rule(RuleId::ScbManifest004),
                rule(RuleId::ScbManifest005),
                rule(RuleId::ScbManifest006),
            ],
        ),
    ],
    default_fail: DEFAULT_FAIL_IDS,
    own_keys: &[
        "manifest_dependency_allowlists",
        "global_dependency_ownership",
        "manifest_section_rules",
    ],
};
