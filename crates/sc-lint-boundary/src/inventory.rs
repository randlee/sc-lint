use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct BoundaryId(String);

impl BoundaryId {
    fn parse(value: String) -> std::result::Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("boundary ids must not be empty".to_string());
        }
        if !trimmed.starts_with("BOUNDARY-") {
            return Err(format!(
                "boundary ids must start with `BOUNDARY-` (got `{trimmed}`)"
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BoundaryId {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for BoundaryId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for BoundaryId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for BoundaryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for BoundaryId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct SprintId(String);

impl SprintId {
    fn parse(value: String) -> std::result::Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("sprint ids must not be empty".to_string());
        }
        let Some((phase, step)) = trimmed.split_once('.') else {
            return Err(format!(
                "sprint ids must use <phase>.<step> format (got `{trimmed}`)"
            ));
        };
        if phase.is_empty() || step.is_empty() || trimmed.contains(char::is_whitespace) {
            return Err(format!("invalid sprint id `{trimmed}`"));
        }
        Ok(Self(trimmed.to_string()))
    }

    fn placeholder_empty_inventory() -> Self {
        Self("A.0".to_string())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SprintId {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for SprintId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for SprintId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for SprintId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for SprintId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct TrackingId(String);

impl TrackingId {
    fn parse(value: String) -> std::result::Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("tracking ids must not be empty".to_string());
        }
        if trimmed.contains(char::is_whitespace) {
            return Err(format!(
                "tracking ids must not contain whitespace (got `{trimmed}`)"
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}

impl TryFrom<String> for TrackingId {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundaryInventory {
    pub(crate) records: Vec<BoundaryRecord>,
    pub(crate) planning: PlanningMetadata,
}

impl BoundaryInventory {
    pub(crate) fn summary(&self) -> InventorySummary {
        InventorySummary {
            boundary_count: self.records.len(),
            planned_item_count: self.planning.planned_items.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventorySummary {
    pub(crate) boundary_count: usize,
    pub(crate) planned_item_count: usize,
}

impl InventorySummary {
    pub(crate) fn recommended_finding_capacity(self) -> usize {
        self.boundary_count
            .saturating_add(self.planned_item_count)
            .max(8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundaryRecord {
    pub(crate) boundary_id: BoundaryId,
    pub(crate) owner_package: String,
    pub(crate) owner_crate_path: String,
    pub(crate) name: String,
    pub(crate) public: PublicSection,
    pub(crate) implementation: ImplementationSection,
    pub(crate) composition: CompositionSection,
    pub(crate) dependencies: DependenciesSection,
    pub(crate) references: ReferencesSection,
    pub(crate) testing: TestingSection,
    pub(crate) enforcement: EnforcementSection,
    pub(crate) status: StatusSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicSection {
    pub(crate) facade: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImplementationSection {
    #[serde(rename = "type")]
    pub(crate) implementation_type: Option<String>,
    pub(crate) module: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) constructor: Option<Constructor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompositionSection {
    pub(crate) roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependenciesSection {
    pub(crate) allowed_dependents: Vec<String>,
    pub(crate) allowed_dependencies: Vec<String>,
    pub(crate) forbidden_edges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferencesSection {
    pub(crate) scope: ReferenceScope,
    pub(crate) forbidden: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestingSection {
    pub(crate) allowed_test_double_paths: Vec<String>,
    pub(crate) forbidden_test_bypasses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnforcementSection {
    pub(crate) lint_rules: Vec<String>,
    pub(crate) review_gates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatusSection {
    pub(crate) state: BoundaryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanningMetadata {
    pub(crate) planning: PlanningHeader,
    #[serde(default)]
    pub(crate) planned_items: BTreeMap<String, PlannedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanningHeader {
    pub(crate) current_sprint: SprintId,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlannedItem {
    pub(crate) scheduled_sprint: SprintId,
    pub(crate) tracking_id: TrackingId,
    pub(crate) expires_when: ExpirationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Visibility {
    Public,
    TraitOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Constructor {
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReferenceScope {
    OutsideOwnerCrate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BoundaryState {
    Planned,
    ConcreteLanded,
    ReservedFuture,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExpirationPolicy {
    SprintBeforeCurrent,
}

pub(crate) fn load_boundary_inventory(root: &Path) -> Result<BoundaryInventory> {
    let boundaries_root = root.join("boundaries");
    if !boundaries_root.exists() {
        return Ok(BoundaryInventory {
            records: Vec::new(),
            planning: PlanningMetadata {
                planning: PlanningHeader {
                    current_sprint: SprintId::placeholder_empty_inventory(),
                },
                planned_items: BTreeMap::new(),
            },
        });
    }
    let boundary_paths = discover_boundary_files(&boundaries_root)?;
    let mut records = Vec::new();
    let mut seen_boundary_ids = BTreeMap::<BoundaryId, PathBuf>::new();

    for path in boundary_paths {
        let record: BoundaryRecord = parse_toml_file(&path)?;
        validate_boundary_schema(&record, &path)?;
        validate_boundary_path(&record, &path, &boundaries_root)?;
        if let Some(previous_path) =
            seen_boundary_ids.insert(record.boundary_id.clone(), path.clone())
        {
            anyhow::bail!(
                "duplicate boundary_id `{}` in `{}` and `{}`",
                record.boundary_id,
                previous_path.display(),
                path.display()
            );
        }
        records.push(record);
    }

    let planning_path = boundaries_root.join("planning.toml");
    let planning: PlanningMetadata = parse_toml_file(&planning_path)?;
    validate_planning_metadata(&planning, &planning_path)?;

    Ok(BoundaryInventory { records, planning })
}

fn discover_boundary_files(boundaries_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    collect_boundary_files(boundaries_root, &mut seen, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_boundary_files(
    dir: &Path,
    seen: &mut BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read boundary directory `{}`", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_boundary_files(&path, seen, files)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("planning.toml") {
            continue;
        }
        if seen.insert(path.clone()) {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_toml_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read TOML file `{}`", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse TOML file `{}`", path.display()))
}

fn validate_boundary_path(
    record: &BoundaryRecord,
    path: &Path,
    boundaries_root: &Path,
) -> Result<()> {
    let owner_dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .context("boundary file missing owner-package directory")?;
    if owner_dir != record.owner_package {
        anyhow::bail!(
            "boundary file `{}` is under owner directory `{owner_dir}` but declares owner_package `{}`",
            path.display(),
            record.owner_package
        );
    }

    let expected_owner_crate_path = record.owner_package.replace('-', "_");
    if record.owner_crate_path != expected_owner_crate_path {
        anyhow::bail!(
            "boundary `{}` declares owner_crate_path `{}` but expected `{expected_owner_crate_path}` from owner_package `{}`",
            record.boundary_id,
            record.owner_crate_path,
            record.owner_package
        );
    }

    let relative = path.strip_prefix(boundaries_root).with_context(|| {
        format!(
            "boundary file `{}` is outside boundaries root",
            path.display()
        )
    })?;
    if relative.components().count() != 2 {
        anyhow::bail!(
            "boundary file `{}` must use boundaries/<owner-package>/<boundary>.toml layout",
            path.display()
        );
    }

    Ok(())
}

fn validate_boundary_schema(record: &BoundaryRecord, path: &Path) -> Result<()> {
    if record.public.facade.trim().is_empty() {
        anyhow::bail!(
            "boundary `{}` in `{}` must define a non-empty public.facade",
            record.boundary_id,
            path.display()
        );
    }

    match record.implementation.visibility {
        Visibility::Public => {
            if record
                .implementation
                .implementation_type
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.type for public visibility",
                    record.boundary_id,
                    path.display()
                );
            }
            if record
                .implementation
                .module
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.module for public visibility",
                    record.boundary_id,
                    path.display()
                );
            }
            if record.implementation.constructor.is_none() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must define implementation.constructor for public visibility",
                    record.boundary_id,
                    path.display()
                );
            }
        }
        Visibility::TraitOnly => {
            if record.implementation.implementation_type.is_some() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must omit implementation.type when visibility is trait_only",
                    record.boundary_id,
                    path.display()
                );
            }
            if record.implementation.module.is_some() {
                anyhow::bail!(
                    "boundary `{}` in `{}` must omit implementation.module when visibility is trait_only",
                    record.boundary_id,
                    path.display()
                );
            }
        }
    }

    Ok(())
}

fn validate_planning_metadata(planning: &PlanningMetadata, planning_path: &Path) -> Result<()> {
    for key in planning.planned_items.keys() {
        if !key.starts_with("BOUNDARY-") || !key.contains('.') {
            anyhow::bail!(
                "planning item key `{key}` in `{}` must use <boundary_id>.<section>.<field>[.<subfield>] shape",
                planning_path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::load_boundary_inventory;
    use tempfile::TempDir;

    struct InventoryFixture {
        tempdir: TempDir,
    }

    impl InventoryFixture {
        fn new() -> Self {
            Self {
                tempdir: TempDir::new().expect("tempdir"),
            }
        }

        fn root(&self) -> &Path {
            self.tempdir.path()
        }

        fn write(&self, relative_path: &str, contents: &str) {
            let path = self.root().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture parent");
            }
            fs::write(path, contents).expect("write fixture");
        }

        fn write_valid_inventory(&self) {
            self.write(
                "boundaries/sc-lint-boundary/boundary-analyzer.toml",
                r#"
boundary_id = "BOUNDARY-ScLintBoundaryAnalyzer"
owner_package = "sc-lint-boundary"
owner_crate_path = "sc_lint_boundary"
name = "ScLintBoundaryAnalyzer"

[public]
facade = "analyze_workspace"

[implementation]
type = "analyze_workspace"
module = "sc_lint_boundary"
visibility = "public"
constructor = "none"

[composition]
roots = []

[dependencies]
allowed_dependents = ["sc-lint"]
allowed_dependencies = ["sc-lint-directives"]
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = ["LINT-SC-BOUNDARY-ISOLATION"]
review_gates = ["no_proc_macro_dependency"]

[status]
state = "concrete_landed"
"#,
            );
            self.write(
                "boundaries/planning.toml",
                r#"
[planning]
current_sprint = "A.6"

[planned_items."BOUNDARY-ScLintCli.public.facade"]
scheduled_sprint = "A.1a"
tracking_id = "SC-LINT-CLI-003"
expires_when = "sprint_before_current"
"#,
            );
        }
    }

    use std::fs;
    use std::path::Path;

    #[test]
    fn loads_valid_boundary_inventory() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();

        let inventory = load_boundary_inventory(fixture.root()).expect("inventory loads");

        assert_eq!(inventory.records.len(), 1);
        assert_eq!(
            inventory.records[0].boundary_id,
            "BOUNDARY-ScLintBoundaryAnalyzer"
        );
        assert_eq!(inventory.planning.planning.current_sprint, "A.6");
    }

    #[test]
    fn allows_trait_only_records_to_omit_type_and_module() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/sc-lint-directives/trait-only.toml",
            r#"
boundary_id = "BOUNDARY-DirectiveTraitSurface"
owner_package = "sc-lint-directives"
owner_crate_path = "sc_lint_directives"
name = "DirectiveTraitSurface"

[public]
facade = "Directive"

[implementation]
visibility = "trait_only"

[composition]
roots = ["Directive"]

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
"#,
        );

        let inventory =
            load_boundary_inventory(fixture.root()).expect("trait-only inventory loads");
        assert_eq!(inventory.records.len(), 2);
    }

    #[test]
    fn rejects_unknown_boundary_fields() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/sc-lint-boundary/boundary-analyzer.toml",
            r#"
boundary_id = "BOUNDARY-ScLintBoundaryAnalyzer"
owner_package = "sc-lint-boundary"
owner_crate_path = "sc_lint_boundary"
name = "ScLintBoundaryAnalyzer"
unexpected = "nope"

[public]
facade = "analyze_workspace"

[implementation]
type = "analyze_workspace"
module = "sc_lint_boundary"
visibility = "public"
constructor = "none"

[composition]
roots = []

[dependencies]
allowed_dependents = ["sc-lint"]
allowed_dependencies = ["sc-lint-directives"]
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = ["LINT-SC-BOUNDARY-ISOLATION"]
review_gates = ["no_proc_macro_dependency"]

[status]
state = "concrete_landed"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("schema fails");
        assert!(error.to_string().contains("failed to parse TOML file"));
    }

    #[test]
    fn rejects_public_visibility_without_type_and_module() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/sc-lint-directives/missing-impl-shape.toml",
            r#"
boundary_id = "BOUNDARY-DirectiveModel"
owner_package = "sc-lint-directives"
owner_crate_path = "sc_lint_directives"
name = "DirectiveModel"

[public]
facade = "Directive"

[implementation]
visibility = "public"
constructor = "none"

[composition]
roots = ["Directive"]

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("public impl shape fails");
        assert!(error.to_string().contains("implementation.type"));
    }

    #[test]
    fn rejects_duplicate_boundary_ids() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/sc-lint-directives/duplicate.toml",
            r#"
boundary_id = "BOUNDARY-ScLintBoundaryAnalyzer"
owner_package = "sc-lint-directives"
owner_crate_path = "sc_lint_directives"
name = "DuplicateBoundary"

[public]
facade = "AttributeInput"

[implementation]
type = "AttributeInput"
module = "sc_lint_directives"
visibility = "public"
constructor = "none"

[composition]
roots = []

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("duplicate id fails");
        assert!(error.to_string().contains("duplicate boundary_id"));
    }

    #[test]
    fn rejects_owner_package_directory_mismatch() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/not-sc-lint-boundary/boundary-analyzer.toml",
            r#"
boundary_id = "BOUNDARY-ScLintBoundaryAnalyzer"
owner_package = "sc-lint-boundary"
owner_crate_path = "sc_lint_boundary"
name = "ScLintBoundaryAnalyzer"

[public]
facade = "analyze_workspace"

[implementation]
type = "analyze_workspace"
module = "sc_lint_boundary"
visibility = "public"
constructor = "none"

[composition]
roots = []

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("owner dir fails");
        assert!(error.to_string().contains("owner directory"));
    }

    #[test]
    fn rejects_duplicate_planned_item_keys() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/planning.toml",
            r#"
[planning]
current_sprint = "A.6"

[planned_items."BOUNDARY-ScLintCli.public.facade"]
scheduled_sprint = "A.1a"
tracking_id = "SC-LINT-CLI-003"
expires_when = "sprint_before_current"

[planned_items."BOUNDARY-ScLintCli.public.facade"]
scheduled_sprint = "A.1b"
tracking_id = "SC-LINT-CLI-004"
expires_when = "sprint_before_current"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("duplicate key fails");
        assert!(error.to_string().contains("failed to parse TOML file"));
    }

    #[test]
    fn rejects_invalid_planning_item_key_shape() {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.write(
            "boundaries/planning.toml",
            r#"
[planning]
current_sprint = "A.6"

[planned_items."not-a-boundary-key"]
scheduled_sprint = "A.1a"
tracking_id = "SC-LINT-CLI-003"
expires_when = "sprint_before_current"
"#,
        );

        let error = load_boundary_inventory(fixture.root()).expect_err("planning key fails");
        assert!(
            error
                .to_string()
                .contains("must use <boundary_id>.<section>.<field>")
        );
    }
}
