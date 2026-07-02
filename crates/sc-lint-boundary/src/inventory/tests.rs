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

    let inventory = load_boundary_inventory(fixture.root()).expect("trait-only inventory loads");
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
fn rejects_unknown_approved_caller_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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

[callers]
approved = [
  { symbol = "send::hook::maybe_run_post_send_hook", callers = ["example::Api"], unexpected = "nope" },
]

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

    let error = load_boundary_inventory(fixture.root()).expect_err("unknown caller field fails");
    assert!(error.to_string().contains("failed to parse TOML file"));
}

#[test]
fn rejects_duplicate_allowed_dependency_names() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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
allowed_dependencies = ["sc-lint-directives", "sc-lint-directives"]
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

    let message = load_boundary_inventory(fixture.root())
        .expect_err("duplicate dependency fails")
        .to_string();
    assert!(!message.trim().is_empty());
}

#[test]
fn rejects_duplicate_forbidden_edges() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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
forbidden_edges = [
  { from = "sc-lint-boundary", to = "sc-lint-attributes" },
  { from = "sc-lint-boundary", to = "sc-lint-attributes" },
]

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

    let message = load_boundary_inventory(fixture.root())
        .expect_err("duplicate edge fails")
        .to_string();
    assert!(!message.trim().is_empty());
}

#[test]
fn rejects_unknown_dependency_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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
unexpected = ["nope"]

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

    let message = load_boundary_inventory(fixture.root())
        .expect_err("unknown dependency field fails")
        .to_string();
    assert!(message.contains("failed to parse TOML file"));
}

#[test]
fn rejects_invalid_dependency_package_name_with_field_context() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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
allowed_dependencies = ["bad package"]
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

    let message = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("invalid dependency package name fails")
    );
    assert!(message.contains("allowed_dependencies"));
    assert!(message.contains("bad package"));
}

#[test]
fn rejects_malformed_forbidden_edge_inline_table() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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
forbidden_edges = [{ from = "sc-lint-boundary" }]

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

    let error = load_boundary_inventory(fixture.root()).expect_err("malformed edge fails");
    assert!(error.to_string().contains("failed to parse TOML file"));
    assert!(error.to_string().contains("to"));
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
fn rejects_duplicate_approved_caller_symbols() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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

[callers]
approved = [
  { symbol = "send::hook::maybe_run_post_send_hook", callers = ["example::Api"] },
  { symbol = "send::hook::maybe_run_post_send_hook", callers = ["example::OtherApi"] },
]

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

    let error =
        load_boundary_inventory(fixture.root()).expect_err("duplicate approved symbol fails");
    assert!(
        error
            .to_string()
            .contains("duplicate approved caller symbol")
    );
}

#[test]
fn rejects_empty_approved_caller_list() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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

[callers]
approved = [
  { symbol = "send::hook::maybe_run_post_send_hook", callers = [] },
]

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

    let error =
        load_boundary_inventory(fixture.root()).expect_err("empty approved caller list fails");
    assert!(error.to_string().contains("at least one approved caller"));
}

#[test]
fn rejects_malformed_approved_caller_symbol() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
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

[callers]
approved = [
  { symbol = "send::hook::", callers = ["example::Api"] },
]

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

    let error =
        load_boundary_inventory(fixture.root()).expect_err("malformed approved symbol fails");
    assert!(error.to_string().contains("failed to parse TOML file"));
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
    let message = error.to_string();
    assert!(message.contains("failed to parse TOML file"));
}
