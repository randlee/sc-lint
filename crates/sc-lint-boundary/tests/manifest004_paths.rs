//! SCB-MANIFEST-004 source ownership matches crate paths by their first
//! segment, so local module paths are not uses of an owned crate (#136).

use std::fs;
use std::path::Path;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use tempfile::TempDir;

const ATM_CORE_COMMANDS_MOD: &str =
    include_str!("fixtures/manifest004/atm_core_6ffaa97be_commands_mod.rs");

const OWNERSHIP: &str = r#"
[tool.sc-lint.analyzers.boundary]
[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
dependency = "cron"
allowed_manifest_paths = []
allowed_source_roots = []
manifest_message = "cron is not declared here"
source_message = "cron is not imported here"
"#;

const SOURCE_VIOLATION: &str =
    "crates/app/src/lib.rs: cron is not imported here (allowed source roots: )";

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path has a parent")).expect("create dirs");
    fs::write(path, contents).expect("write fixture file");
}

/// A workspace whose only member `app` does not own `cron`, with the given
/// files written under `crates/app/src`.
fn workspace(files: &[(&str, &str)]) -> TempDir {
    let fixture = TempDir::new().expect("tempdir");
    write(
        fixture.path(),
        "Cargo.toml",
        r#"
[workspace]
members = ["crates/app"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT OR Apache-2.0"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"
"#,
    );
    write(
        fixture.path(),
        "crates/app/Cargo.toml",
        r#"
[package]
name = "app"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
"#,
    );
    for (relative, contents) in files {
        write(
            fixture.path(),
            &format!("crates/app/src/{relative}"),
            contents,
        );
    }
    write(fixture.path(), "sc-lint.toml", OWNERSHIP);
    fixture
}

/// SCB-MANIFEST-004 messages reported for `crates/app/src/lib.rs` holding `source`.
fn findings(source: &str) -> Vec<String> {
    findings_for(&[("lib.rs", source)])
}

fn findings_for(files: &[(&str, &str)]) -> Vec<String> {
    let fixture = workspace(files);
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.path().to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        None,
        Some("SCB-MANIFEST-004"),
    )
    .expect("ownership policy analyzes");
    report
        .findings
        .into_iter()
        .map(|finding| {
            assert_eq!(finding.rule_id, RuleId::ScbManifest004);
            assert_eq!(finding.kind, "source_dependency_owner_violation");
            finding.message
        })
        .collect()
}

fn assert_reported(name: &str, source: &str) {
    assert_eq!(findings(source), vec![SOURCE_VIOLATION], "{name}: {source}");
}

fn assert_not_reported(name: &str, source: &str) {
    assert_eq!(findings(source), Vec::<String>::new(), "{name}: {source}");
}

#[test]
fn manifest004_local_module_path() {
    let cases = [
        (
            "use self",
            "mod cron { pub struct X; }\nuse self::cron::X;\n",
        ),
        (
            "use super",
            "mod cron { pub struct X; }\nmod inner { use super::cron::X; }\n",
        ),
        (
            "use crate",
            "mod cron { pub struct X; }\nuse crate::cron::X;\n",
        ),
        (
            "use crate nested module",
            "mod commands { pub mod cron { pub struct X; } }\nuse crate::commands::cron::X;\n",
        ),
        (
            "multi-line grouped use",
            "mod cron { pub struct X; pub struct Y; }\nuse self::cron::{\n    X,\n    Y,\n};\n",
        ),
        (
            "nested group paths",
            "mod commands { pub mod cron { pub struct X; pub mod job { pub struct Y; } } }\n\
             use crate::{\n    commands::{cron::X, cron::job::Y},\n};\n",
        ),
        (
            "bare path with local mod",
            "mod cron { pub struct X; }\nuse cron::X as Local;\nfn f() -> cron::X { cron::X }\n",
        ),
    ];
    for (name, source) in cases {
        assert_not_reported(name, source);
    }
}

#[test]
fn manifest004_owned_crate_positions_still_reported() {
    let owned = [
        ("use", "use cron::Schedule;\n"),
        ("type", "fn f(_: cron::Schedule) {}\n"),
        ("expression", "fn f() { let _ = cron::Schedule::new(); }\n"),
        (
            "pattern",
            "fn f(s: S) { if let cron::Kind::Daily = s.kind {} }\n",
        ),
        (
            "struct literal",
            "fn f() { let _ = cron::Job { id: 1 }; }\n",
        ),
        ("trait bound", "fn f<T: cron::Tick>(_: T) {}\n"),
        ("impl header", "struct S;\nimpl cron::Tick for S {}\n"),
        ("qualified path", "fn f() { <S as cron::Tick>::tick(); }\n"),
        ("attribute", "#[cron::job]\nfn f() {}\n"),
        (
            "macro argument",
            "fn f() { println!(\"{:?}\", cron::Schedule::new()); }\n",
        ),
        ("absolute path", "use ::cron::Schedule;\n"),
        (
            "absolute path with local mod",
            "mod cron {}\nfn f(_: ::cron::Schedule) {}\n",
        ),
        ("extern crate", "extern crate cron;\n"),
    ];
    for (name, source) in owned {
        assert_reported(name, source);
    }
    for prefix in ["self", "crate", "super"] {
        let local = [
            ("use", format!("use {prefix}::cron::Schedule;\n")),
            ("type", format!("fn f(_: {prefix}::cron::Schedule) {{}}\n")),
            (
                "expression",
                format!("fn f() {{ let _ = {prefix}::cron::Schedule::new(); }}\n"),
            ),
            (
                "pattern",
                format!("fn f(s: S) {{ if let {prefix}::cron::Kind::Daily = s.kind {{}} }}\n"),
            ),
            (
                "struct literal",
                format!("fn f() {{ let _ = {prefix}::cron::Job {{ id: 1 }}; }}\n"),
            ),
            (
                "trait bound",
                format!("fn f<T: {prefix}::cron::Tick>(_: T) {{}}\n"),
            ),
            (
                "impl header",
                format!("struct S;\nimpl {prefix}::cron::Tick for S {{}}\n"),
            ),
            (
                "qualified path",
                format!("fn f() {{ <S as {prefix}::cron::Tick>::tick(); }}\n"),
            ),
            (
                "attribute",
                format!("#[{prefix}::cron::job]\nfn f() {{}}\n"),
            ),
        ];
        for (name, source) in local {
            assert_not_reported(&format!("{prefix} {name}"), &source);
        }
    }
}

#[test]
fn manifest004_macro_body_textual() {
    // Macro token bodies keep the textual rule, so any `cron::` there counts,
    // including a local-module path and a string literal.
    assert_reported(
        "macro local path",
        "mod cron { pub struct X; }\nfn f() { let _ = vec![self::cron::X]; }\n",
    );
    assert_reported("macro string", "fn f() { println!(\"cron::Schedule\"); }\n");
    assert_reported(
        "macro deref first",
        "fn f() { let _ = vec![*A, cron::Schedule::new()]; }\n",
    );
    assert_reported("macro use", "m! { use cron; }\n");
    assert_not_reported(
        "macro other crate",
        "fn f() { println!(\"{}\", cronx::Y); }\n",
    );
}

#[test]
fn manifest004_parse_failure_fails_closed() {
    // A file that does not parse falls back to the textual rule.
    assert_reported("unparsable owned path", "fn f( { cron::Schedule::new() }\n");
    assert_reported(
        "unparsable local path",
        "mod cron {}\nfn f( { self::cron::X }\n",
    );
    assert_not_reported("unparsable without mention", "fn f( { other::X }\n");
}

#[test]
fn manifest004_doc_and_string_not_uses() {
    let cases = [
        ("inner doc", "//! Wraps cron::Schedule.\npub struct S;\n"),
        ("outer doc", "/// Uses cron::Schedule.\npub struct S;\n"),
        (
            "block doc",
            "/**\nSee\ncron::Schedule for details.\n*/\npub struct S;\n",
        ),
        (
            "block comment",
            "/* first line\ncron::Schedule\n*/\npub struct S;\n",
        ),
        ("string", "const MENTION: &str = \"cron::Schedule\";\n"),
        (
            "multi-line raw string",
            "const MENTION: &str = r\"\nuse cron::Schedule;\nextern crate cron;\n\";\n",
        ),
    ];
    for (name, source) in cases {
        assert_not_reported(name, source);
    }
}

#[test]
fn manifest004_atm_core_commands_mod_replay() {
    // atm-core 6ffaa97be crates/atm/src/commands/mod.rs: `pub mod cron;` and
    // `Cron(self::cron::CronCommand)` are local module paths.
    assert!(ATM_CORE_COMMANDS_MOD.contains("Cron(self::cron::CronCommand)"));
    assert_eq!(
        findings_for(&[
            ("lib.rs", "pub mod commands;\n"),
            ("commands/mod.rs", ATM_CORE_COMMANDS_MOD),
        ]),
        Vec::<String>::new()
    );
}
