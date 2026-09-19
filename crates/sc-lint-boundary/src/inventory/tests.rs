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

    fn rewrite_valid_planning(&self, rewrite: impl FnOnce(String) -> String) {
        let path = self.root().join("boundaries/planning.toml");
        let contents = fs::read_to_string(&path).expect("read valid planning");
        fs::write(path, rewrite(contents)).expect("rewrite valid planning");
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
fn loads_empty_inventory_without_boundaries_directory() {
    let fixture = InventoryFixture::new();

    let inventory = load_boundary_inventory(fixture.root()).expect("empty inventory loads");

    assert!(inventory.records.is_empty());
    assert!(inventory.planning.planned_items.is_empty());
}

#[test]
fn loads_hyphenated_owner_package_with_matching_underscore_crate_path() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();

    let inventory = load_boundary_inventory(fixture.root()).expect("matching crate path loads");

    assert_eq!(inventory.records[0].owner_package, "sc-lint-boundary");
    assert_eq!(inventory.records[0].owner_crate_path, "sc_lint_boundary");
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
    assert!(error.contains("typo"));
}

#[test]
fn rejects_forbidden_edge_inline_table_missing_to_with_serde_cause() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "forbidden_edges = []",
            "forbidden_edges = [{ from = \"sc-lint-boundary\" }]",
        )
    });
    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("missing to fails")
    );
    assert!(error.contains("missing field `to`"));
}

#[test]
fn rejects_forbidden_edge_non_string_non_table() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace("forbidden_edges = []", "forbidden_edges = [42]")
    });
    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("integer edge fails")
    );
    assert!(error.contains("from -> to"));
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

fn assert_rejects_malformed_arrow_forbidden_edge(value: &str, reason: &str) {
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
    assert!(error.contains(reason));
}

#[test]
fn rejects_forbidden_edge_arrow_without_arrow() {
    assert_rejects_malformed_arrow_forbidden_edge("sc-lint-boundary", "missing `->`");
}

#[test]
fn rejects_forbidden_edge_arrow_with_two_arrows() {
    assert_rejects_malformed_arrow_forbidden_edge(
        "sc-lint-boundary -> sc-lint -> sc-lint",
        "more than one",
    );
}

#[test]
fn rejects_forbidden_edge_arrow_with_empty_side() {
    assert_rejects_malformed_arrow_forbidden_edge(" -> sc-lint", "left `from` side");
}

#[test]
fn rejects_forbidden_edge_arrow_with_whitespace_only_side() {
    assert_rejects_malformed_arrow_forbidden_edge("   -> sc-lint", "left `from` side");
}

#[test]
fn rejects_forbidden_edge_arrow_with_empty_to_side() {
    assert_rejects_malformed_arrow_forbidden_edge("sc-lint ->   ", "right `to` side");
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
    assert!(error.contains("must define exactly one of public.facade or public.trait"));
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

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown ownership field fails")
    );
    assert!(error.contains("unknown field `unexpected`"));
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

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown contracts field fails")
    );
    assert!(error.contains("unknown field `unexpected`"));
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

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown status field fails")
    );
    assert!(error.contains("unknown field `unexpected`"));
}

#[test]
fn rejects_missing_planning_metadata_with_actionable_error() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fs::remove_file(fixture.root().join("boundaries/planning.toml")).expect("remove planning");

    let error = load_boundary_inventory(fixture.root())
        .expect_err("missing planning metadata fails")
        .to_string();
    assert!(error.contains("planning.toml"));
    assert!(error.contains("[planning].current_sprint"));
}

fn assert_rejects_planning_metadata(contents: &str, expected_field: &str) {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.write("boundaries/planning.toml", contents);

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("invalid planning metadata fails")
    );
    assert!(error.contains(expected_field), "{error}");
}

#[test]
fn rejects_planning_metadata_without_planning_table() {
    assert_rejects_planning_metadata("[planned_items]\n", "missing field `planning`");
}

#[test]
fn rejects_planning_metadata_without_current_sprint() {
    assert_rejects_planning_metadata("[planning]\n", "current_sprint");
}

#[test]
fn rejects_planning_metadata_with_empty_current_sprint() {
    assert_rejects_planning_metadata(
        "[planning]\ncurrent_sprint = \"\"\n",
        "sprint ids must not be empty",
    );
}

#[test]
fn rejects_planning_metadata_with_malformed_current_sprint() {
    assert_rejects_planning_metadata(
        "[planning]\ncurrent_sprint = \"not-a-sprint\"\n",
        "sprint ids must use <phase>.<step> format",
    );
}

#[test]
fn rejects_unknown_fields_in_all_boundary_tables() {
    type UnknownFieldCase = (&'static str, Box<dyn Fn(String) -> String>);
    let cases: [UnknownFieldCase; 9] = [
        (
            "unexpected_public",
            Box::new(|contents| {
                contents.replace(
                    "facade = \"analyze_workspace\"",
                    "facade = \"analyze_workspace\"\nunexpected_public = true",
                )
            }),
        ),
        (
            "unexpected_implementation",
            Box::new(|contents| {
                contents.replace(
                    "constructor = \"none\"",
                    "constructor = \"none\"\nunexpected_implementation = true",
                )
            }),
        ),
        (
            "unexpected_composition",
            Box::new(|contents| {
                contents.replace("roots = []", "roots = []\nunexpected_composition = true")
            }),
        ),
        (
            "unexpected_references",
            Box::new(|contents| {
                contents.replace(
                    "forbidden = []",
                    "forbidden = []\nunexpected_references = true",
                )
            }),
        ),
        (
            "unexpected_testing",
            Box::new(|contents| {
                contents.replace(
                    "forbidden_test_bypasses = []",
                    "forbidden_test_bypasses = []\nunexpected_testing = true",
                )
            }),
        ),
        (
            "unexpected_enforcement",
            Box::new(|contents| {
                contents.replace(
                    "review_gates = [\"no_proc_macro_dependency\"]",
                    "review_gates = [\"no_proc_macro_dependency\"]\nunexpected_enforcement = true",
                )
            }),
        ),
        (
            "unexpected_ownership",
            Box::new(|contents| {
                contents.replace("[dependencies]", "[ownership]\nio_owns = []\nio_forbidden = []\nunexpected_ownership = true\n\n[dependencies]")
            }),
        ),
        (
            "unexpected_contracts",
            Box::new(|contents| {
                contents.replace("[dependencies]", "[contracts]\nrequest_types = []\nresponse_types = []\nerror_types = []\nunexpected_contracts = true\n\n[dependencies]")
            }),
        ),
        (
            "unexpected_status",
            Box::new(|contents| {
                contents.replace(
                    "state = \"concrete_landed\"",
                    "state = \"concrete_landed\"\nunexpected_status = true",
                )
            }),
        ),
    ];
    let mut failures = Vec::new();
    for (expected, rewrite) in cases {
        let fixture = InventoryFixture::new();
        fixture.write_valid_inventory();
        fixture.rewrite_valid_boundary(rewrite);
        match load_boundary_inventory(fixture.root()) {
            Ok(_) => failures.push(format!("{expected}: accepted")),
            Err(error) if !format!("{error:#}").contains(expected) => {
                failures.push(format!("{expected}: {error:#}"));
            }
            Err(_) => {}
        }
    }

    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_planning(|contents| {
        contents.replace(
            "current_sprint = \"A.6\"",
            "current_sprint = \"A.6\"\nunexpected_planning = true",
        )
    });
    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown planning field fails")
    );
    if !error.contains("unexpected_planning") {
        failures.push(format!("unexpected_planning: {error}"));
    }

    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_planning(|contents| {
        contents.replace(
            "expires_when = \"sprint_before_current\"",
            "expires_when = \"sprint_before_current\"\nunexpected_item = true",
        )
    });
    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("unknown planned-item field fails")
    );
    if !error.contains("unexpected_item") {
        failures.push(format!("unexpected_item: {error}"));
    }

    assert!(
        failures.is_empty(),
        "unknown-field cases failed: {failures:#?}"
    );
}

#[test]
fn rejects_duplicate_allowed_dependents() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "allowed_dependents = [\"sc-lint\"]",
            "allowed_dependents = [\"sc-lint\", \"sc-lint\"]",
        )
    });

    let error = format!(
        "{:#}",
        load_boundary_inventory(fixture.root()).expect_err("duplicate allowed dependent fails")
    );
    assert!(error.contains("duplicate"), "{error}");
}

#[test]
fn rejects_facade_with_empty_trait() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "facade = \"analyze_workspace\"",
            "facade = \"analyze_workspace\"\ntrait = \"\"",
        )
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("empty trait fails even with facade")
        .to_string();
    assert!(error.contains("empty public.trait"));
}

#[test]
fn loads_atm_boundary_vocabulary() {
    let fixture = InventoryFixture::new();
    fixture.write(
        "boundaries/planning.toml",
        "[planning]\ncurrent_sprint = \"A.0\"\n",
    );
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
    let message = error.to_string();
    assert!(message.contains("implementation.type"));
    assert!(message.contains("public, private, or pub(crate) visibility"));
}

#[test]
fn rejects_private_and_pub_crate_visibility_without_required_implementation_fields() {
    for visibility in ["private", "pub(crate)"] {
        for (field, line, expected) in [
            (
                "type",
                "type = \"analyze_workspace\"\n",
                "must define implementation.type",
            ),
            (
                "module",
                "module = \"sc_lint_boundary\"\n",
                "must define implementation.module",
            ),
            (
                "constructor",
                "constructor = \"none\"\n",
                "must define implementation.constructor",
            ),
        ] {
            let fixture = InventoryFixture::new();
            fixture.write_valid_inventory();
            fixture.rewrite_valid_boundary(|contents| {
                contents
                    .replace(
                        "visibility = \"public\"",
                        &format!("visibility = \"{visibility}\""),
                    )
                    .replace(line, "")
            });
            let error = load_boundary_inventory(fixture.root())
                .expect_err("missing implementation field fails")
                .to_string();
            assert!(error.contains(expected), "{visibility} {field}: {error}");
        }
    }
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
fn rejects_owner_crate_path_mismatch() {
    let fixture = InventoryFixture::new();
    fixture.write_valid_inventory();
    fixture.rewrite_valid_boundary(|contents| {
        contents.replace(
            "owner_crate_path = \"sc_lint_boundary\"",
            "owner_crate_path = \"wrong_crate_path\"",
        )
    });

    let error = load_boundary_inventory(fixture.root())
        .expect_err("owner crate path mismatch fails")
        .to_string();
    assert!(error.contains(
        "boundary `BOUNDARY-ScLintBoundaryAnalyzer` declares owner_crate_path `wrong_crate_path` but expected `sc_lint_boundary` from owner_package `sc-lint-boundary`"
    ));
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

    fixture.write(
        "boundaries/planning.toml",
        r#"
[planning]
current_sprint = "A.6"

[planned_items."BOUNDARY-ScLintCli"]
scheduled_sprint = "A.1a"
tracking_id = "SC-LINT-CLI-003"
expires_when = "sprint_before_current"
"#,
    );

    let error = load_boundary_inventory(fixture.root()).expect_err("planning key fails");
    let message = format!("{error:#}");
    assert!(message.contains("planning keys must use"), "{message}");
}
