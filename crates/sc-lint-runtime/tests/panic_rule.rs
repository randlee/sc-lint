use sc_lint_runtime::{AnalyzeOptions, PanicPackageError, RuntimeError, analyze_workspace};
use sc_lint_schema::OutputFormat;
use std::{fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn fixture() -> TempDir {
    let fixture = TempDir::new().unwrap();
    write(
        fixture.path(),
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n[workspace]\n",
    );
    fixture
}
fn analyze(root: &Path) -> Result<sc_lint_runtime::FindingsReport, RuntimeError> {
    analyze_workspace(&AnalyzeOptions {
        root: root.to_path_buf(),
        format: OutputFormat::Json,
    })
}
fn config(root: &Path, items: &str) {
    write(
        root,
        "sc-lint.toml",
        &format!("[tool.sc-lint.analyzers.runtime]\npanic_rule_packages={items}\n"),
    );
}
#[test]
fn package_opt_in_detects_six_forms_and_exempts_test_scope() {
    let fixture = fixture();
    let root = fixture.path();
    write(
        root,
        "src/lib.rs",
        "fn f(x: Option<()>) { x.unwrap(); x.expect(\"reason\"); panic!(); unreachable!(); todo!(); unimplemented!(); }\n#[cfg(test)] mod tests { fn test() { panic!(); } }\n#[cfg(test)] const X: () = panic!();\n",
    );
    write(root, "tests/test.rs", "fn f() { panic!(); }");
    write(root, "benches/bench.rs", "fn f() { panic!(); }");
    assert!(analyze(root).unwrap().findings.is_empty());
    config(root, "['sample']");
    let report = analyze(root).unwrap();
    assert_eq!(report.findings.len(), 6, "{report:?}");
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.rule_id.as_str() == "SCB-RUNTIME-008")
    );
}
#[test]
fn allowance_is_scoped_to_function_and_requires_reason() {
    let fixture = fixture();
    let root = fixture.path();
    config(root, "['sample']");
    write(
        root,
        "src/lib.rs",
        "#[sc_lint(allow(SCB-RUNTIME-008), reason = \"approved\")] fn outer() { Some(1).unwrap(); fn inner() { panic!(); } }\n",
    );
    let report = analyze(root).unwrap();
    assert_eq!(report.findings.len(), 1, "{report:?}");
    write(
        root,
        "src/lib.rs",
        "#[sc_lint(allow(SCB-RUNTIME-008), reason = \" \")] fn f() { panic!(); }",
    );
    assert!(analyze(root).is_err());
}
#[test]
fn package_list_rejects_empty_duplicate_and_unknown_names() {
    let fixture = fixture();
    let root = fixture.path();
    write(root, "src/lib.rs", "fn f() {}\n");
    for (value, expected) in [
        ("['']", "empty"),
        ("['sample','sample']", "duplicate"),
        ("['missing']", "unknown"),
    ] {
        config(root, value);
        let error = analyze(root).unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::PanicPackages(
                PanicPackageError::Empty
                    | PanicPackageError::Duplicate(_)
                    | PanicPackageError::Unknown(_)
            )
        ));
        assert!(error.to_string().contains(expected), "{error}");
    }
    config(root, "[]");
    assert!(analyze(root).unwrap().findings.is_empty());
}

#[test]
fn out_of_line_markers_and_parent_only_cfg_keep_existing_scope() {
    let fixture = fixture();
    let root = fixture.path();
    config(root, "['sample']");
    write(
        root,
        "src/lib.rs",
        "#[cfg(test)] mod marked;\n#[cfg(test)] mod unmarked;\n",
    );
    write(
        root,
        "src/marked.rs",
        "#![cfg(test)]\nfn f() { panic!(); }\n",
    );
    write(root, "src/unmarked.rs", "fn f() { panic!(); }\n");
    let report = analyze(root).unwrap();
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert!(report.findings[0].message.contains("unmarked.rs"));
}

#[test]
fn site_allowance_does_not_suppress_other_rules() {
    let fixture = fixture();
    let root = fixture.path();
    config(root, "['sample']");
    write(
        root,
        "src/lib.rs",
        "use std::sync::{Condvar, Mutex};\n#[sc_lint(allow(SCB-RUNTIME-008), reason=\"approved\")] fn f(c: &Condvar, m: &Mutex<bool>) { let g = m.lock().unwrap(); let _ = c.wait(g); }\n",
    );
    let report = analyze(root).unwrap();
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert_eq!(report.findings[0].rule_id.as_str(), "SCB-RUNTIME-001");
}

#[test]
fn common_filters_apply_to_opted_in_rule() {
    use sc_lint_runtime::analyze_workspace_configured;
    use sc_lint_schema::ReportStatus;
    let fixture = fixture();
    let root = fixture.path();
    write(root, "src/lib.rs", "fn f() { panic!(); }\n");
    let options = AnalyzeOptions {
        root: root.to_path_buf(),
        format: OutputFormat::Json,
    };
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\npanic_rule_packages=['sample']\nexclude_rules=['SCB-RUNTIME-008']\n",
    );
    assert!(analyze_workspace(&options).unwrap().findings.is_empty());
    let explicit = analyze_workspace_configured(&options, None, Some("SCB-RUNTIME-008")).unwrap();
    assert_eq!(explicit.findings.len(), 1);
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\npanic_rule_packages=['sample']\nfail_rules=[]\n",
    );
    let nonfailing = analyze_workspace(&options).unwrap();
    assert_eq!(nonfailing.findings.len(), 1);
    assert_eq!(nonfailing.status, ReportStatus::Pass);
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\npanic_rule_packages=['sample']\n[tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]\n'SCB-RUNTIME-008'=['src/lib.rs']\n",
    );
    assert!(analyze_workspace(&options).unwrap().findings.is_empty());
}

#[test]
fn production_function_named_tests_is_not_exempt() {
    let fixture = fixture();
    let root = fixture.path();
    config(root, "['sample']");
    write(root, "src/lib.rs", "fn tests() { panic!(); }\n");
    assert_eq!(analyze(root).unwrap().findings.len(), 1);
}

#[test]
fn disabled_analyzer_still_validates_configured_package_names() {
    let fixture = fixture();
    let root = fixture.path();
    write(root, "src/lib.rs", "fn f() {}\n");
    write(
        root,
        "sc-lint.toml",
        "[tool.sc-lint.analyzers.runtime]\nenabled=false\npanic_rule_packages=['missing']\n",
    );
    assert!(matches!(
        analyze(root),
        Err(RuntimeError::PanicPackages(PanicPackageError::Unknown(_)))
    ));
}

#[test]
fn opt_in_is_per_workspace_package() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=['first','second']\nresolver='2'\n",
    );
    for name in ["first", "second"] {
        write(
            root,
            &format!("{name}/Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2021'\n"),
        );
        write(
            root,
            &format!("{name}/src/lib.rs"),
            "fn f() { panic!(); }\n",
        );
    }
    config(root, "['first']");
    let report = analyze(root).unwrap();
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert!(
        report.findings[0].owner_ids[0]
            .to_string()
            .contains("first")
    );
}
