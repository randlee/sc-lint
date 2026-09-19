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

    fn rewrite_valid_boundary(&self, rewrite: impl FnOnce(String) -> String) {
        let path = self
            .root()
            .join("boundaries/sc-lint-boundary/boundary-analyzer.toml");
        let contents = fs::read_to_string(&path).expect("read valid boundary");
        fs::write(path, rewrite(contents)).expect("rewrite valid boundary");
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
fn loads_trait_public_boundary_inventory() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write(
        "boundaries/sc-lint-directives/trait-boundary.toml",
        r#"
boundary_id = "BOUNDARY-DirectiveTraitSurface"
owner_package = "sc-lint-directives"
owner_crate_path = "sc_lint_directives"
name = "DirectiveTraitSurface"

[public]
trait = "Directive"
notes = "Trait surfaces are valid public boundary declarations."

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

    let inventory = load_boundary_inventory(fixture.root()).expect("trait inventory loads");
    assert_eq!(inventory.records.len(), 2);
    assert_eq!(
        inventory.records[1].public.trait_name.as_deref(),
        Some("Directive")
    );
    assert_eq!(inventory.records[1].public.facade, None);
}

#[test]
fn rejects_forbidden_edge_inline_table_unknown_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "forbidden_edges = []",
            "forbidden_edges = [{ from = \"sc-lint-boundary\", to = \"sc-lint\", typo = \"reject\" }]",
        )
    });

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown edge field fails")
    );
    assert!(error.contains("boundary-analyzer.toml"));
    assert!(error.contains("forbidden_edges"));
}

#[test]
fn structured_and_arrow_forbidden_edges_produce_equal_edges() {
    let structured_fixture = InventoryFixture::new();
    structured_fixture.write_valid_inventory();
    structured_fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "forbidden_edges = []",
            "forbidden_edges = [{ from = \"sc-lint-boundary\", to = \"sc-lint\" }]",
        )
    });
    let structured = load_boundary_inventory(structured_fixture.root())
        .expect("structured edge loads")
        .records[0]
        .dependencies
        .forbidden_edges
        .clone();

    let arrow_fixture = InventoryFixture::new();
    arrow_fixture.write_valid_inventory();
    arrow_fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "forbidden_edges = []",
            "forbidden_edges = [\"sc-lint-boundary -> sc-lint\"]",
        )
    });
    let arrow = load_boundary_inventory(arrow_fixture.root())
        .expect("arrow edge loads")
        .records[0]
        .dependencies
        .forbidden_edges
        .clone();

    assert_eq!(structured, arrow);
}

fn assert_rejects_malformed_arrow_forbidden_edge(value: &str) {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "forbidden_edges = []",
            &format!("forbidden_edges = [{value:?}]"),
        )
    });

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("malformed arrow edge fails")
    );
    assert!(error.contains("boundary-analyzer.toml"));
    assert!(error.contains("dependencies.forbidden_edges[]"));
}

#[test]
fn rejects_forbidden_edge_arrow_without_arrow() {
    assert_rejects_malformed_arrow_forbidden_edge("sc-lint-boundary");
}

#[test]
fn rejects_forbidden_edge_arrow_with_two_arrows() {
    assert_rejects_malformed_arrow_forbidden_edge("sc-lint-boundary -> sc-lint -> sc-lint");
}

#[test]
fn rejects_forbidden_edge_arrow_with_empty_side() {
    assert_rejects_malformed_arrow_forbidden_edge(" -> sc-lint");
}

#[test]
fn rejects_forbidden_edge_arrow_with_whitespace_only_side() {
    assert_rejects_malformed_arrow_forbidden_edge("   -> sc-lint");
}

#[test]
fn rejects_public_boundary_with_both_facade_and_trait() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "facade = \"analyze_workspace\"",
            "facade = \"analyze_workspace\"\ntrait = \"Analyzer\"",
        )
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("a boundary must choose one public surface")
        .to_string();
    assert!(error.contains("exactly one"));
}

#[test]
fn rejects_public_boundary_with_neither_facade_nor_trait() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("facade = \"analyze_workspace\"\n", "")
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("a boundary must define a public surface")
        .to_string();
    assert!(error.contains("must define a non-empty public.facade or public.trait"));
}

#[test]
fn rejects_public_boundary_with_empty_facade() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("facade = \"analyze_workspace\"", "facade = \"\"")
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("an empty facade is invalid")
        .to_string();
    assert!(error.contains("empty public.facade"));
}

#[test]
fn rejects_public_boundary_with_empty_trait() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("facade = \"analyze_workspace\"", "trait = \"\"")
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("an empty trait is invalid")
        .to_string();
    assert!(error.contains("empty public.trait"));
}

#[test]
fn rejects_public_boundary_with_whitespace_only_trait() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("facade = \"analyze_workspace\"", "trait = \"   \"")
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("a whitespace-only trait is invalid")
        .to_string();
    assert!(error.contains("empty public.trait"));
}

#[test]
fn rejects_public_boundary_with_whitespace_only_facade() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("facade = \"analyze_workspace\"", "facade = \"   \"")
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("a whitespace-only facade is invalid")
        .to_string();
    assert!(error.contains("empty public.facade"));
}

#[test]
fn rejects_unknown_ownership_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "[dependencies]",
            "[ownership]\nio_owns = []\nio_forbidden = []\nunexpected = true\n\n[dependencies]",
        )
    });

    load_boundary_inventory(fixture.root()).expect_err("unknown ownership field fails");
}

#[test]
fn rejects_unknown_contracts_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "[dependencies]",
            "[contracts]\nrequest_types = []\nresponse_types = []\nerror_types = []\nunexpected = true\n\n[dependencies]",
        )
    });

    load_boundary_inventory(fixture.root()).expect_err("unknown contracts field fails");
}

#[test]
fn rejects_unknown_status_fields() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "state = \"concrete_landed\"",
            "state = \"concrete_landed\"\nunexpected = true",
        )
    });

    load_boundary_inventory(fixture.root()).expect_err("unknown status field fails");
}

#[test]
fn loads_boundary_inventory_without_planning_metadata() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fs::remove_file(fixture.root().join("boundaries/planning.toml")).expect("remove planning");

    let inventory =
        load_boundary_inventory(fixture.root()).expect("inventory loads without planning");

    assert_eq!(inventory.planning.planning.current_sprint, "A.0");
    assert!(inventory.planning.planned_items.is_empty());
}

#[test]
fn loads_atm_boundary_vocabulary() {
    let fixture = InventoryFixture::new();
    fixture.write(
        "boundaries/atm/adapter.toml",
        r#"
boundary_id = "BOUNDARY-AtmAdapter"
owner_package = "atm"
owner_crate_path = "atm"
name = "AtmAdapter"

[public]
trait = "AdapterPort"
notes = "A public trait surface."

[implementation]
type = "Adapter"
module = "atm::adapter"
visibility = "public"
constructor = "public"

[composition]
roots = ["atm::bootstrap"]

[ownership]
io_owns = ["adapter_io"]
io_forbidden = ["storage_io"]

[dependencies]
allowed_dependents = []
allowed_dependencies = ["atm-core"]
forbidden_edges = ["atm -> atm-storage"]

[references]
scope = "global"
forbidden = []

[contracts]
request_types = ["Request"]
response_types = ["Response"]
error_types = ["AtmError"]
notes = ["Documented contract."]

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "active"
notes = ["Live."]
"#,
    );
    fixture.write(
        "boundaries/atm-private/private-adapter.toml",
        r#"
boundary_id = "BOUNDARY-AtmPrivateAdapter"
owner_package = "atm-private"
owner_crate_path = "atm_private"
name = "AtmPrivateAdapter"

[public]
facade = "private_adapter"

[implementation]
type = "PrivateAdapter"
module = "atm_private::adapter"
visibility = "private"
constructor = "private"

[composition]
roots = []

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "inside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "retired"
"#,
    );
    fixture.write(
        "boundaries/atm-crate/crate-adapter.toml",
        r#"
boundary_id = "BOUNDARY-AtmCrateAdapter"
owner_package = "atm-crate"
owner_crate_path = "atm_crate"
name = "AtmCrateAdapter"

[public]
facade = "crate_adapter"

[implementation]
type = "CrateAdapter"
module = "atm_crate::adapter"
visibility = "pub(crate)"
constructor = "pub(crate)"

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
state = "unix_implemented_windows_pending"
"#,
    );

    let inventory = load_boundary_inventory(fixture.root()).expect("ATM vocabulary loads");

    assert_eq!(inventory.records.len(), 3);
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
