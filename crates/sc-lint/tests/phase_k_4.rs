use sc_lint_boundary::ALL_BOUNDARY_IDS;
use sc_lint_boundary::BOUNDARY_CATALOG;
use sc_lint_boundary::DEFAULT_FAIL_IDS;

#[test]
fn issue136_product_dispatch_registers_manifest_policy_rules() {
    let expected = [
        "SCB-MANIFEST-003",
        "SCB-MANIFEST-004",
        "SCB-MANIFEST-005",
        "SCB-MANIFEST-006",
    ];
    for rule in expected {
        assert!(ALL_BOUNDARY_IDS.iter().any(|id| id.as_str() == rule));
        assert!(DEFAULT_FAIL_IDS.iter().any(|id| id.as_str() == rule));
        assert!(
            BOUNDARY_CATALOG
                .families
                .iter()
                .find(|(name, _)| *name == "manifests")
                .is_some_and(|(_, rules)| rules.iter().any(|id| id.as_str() == rule))
        );
    }
}
