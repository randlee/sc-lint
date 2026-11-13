use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;
use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::BoundaryError;
use sc_lint_boundary::RuleId;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use tar::Builder;
use tar::Header;
use tempfile::TempDir;

struct Fixture {
    tempdir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            tempdir: TempDir::new().expect("fixture directory"),
        };
        fixture.workspace();
        fixture
    }

    fn root(&self) -> &Path {
        self.tempdir.path()
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(&path, contents).expect("write fixture file");
        path
    }

    fn workspace(&self) {
        self.write(
            "Cargo.toml",
            r#"[workspace]
members = ["crates/app", "crates/internal"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"

[workspace.dependencies]
"#,
        );
        self.write(
            "crates/app/Cargo.toml",
            r#"[package]
name = "app"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
internal = { path = "../internal", version = "=0.1.0" }
"#,
        );
        self.write("crates/app/src/lib.rs", "pub struct App;\n");
        self.write(
            "crates/internal/Cargo.toml",
            r#"[package]
name = "internal"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
"#,
        );
        self.write("crates/internal/src/lib.rs", "pub struct Internal;\n");
    }

    fn config(&self, template: &str, tests: &str, archives: &str) {
        self.write(
            "sc-lint.toml",
            &format!(
                "[tool.sc-lint.analyzers.boundary]\n\
                 [[tool.sc-lint.analyzers.boundary.dependency_requirements]]\n\
                 template_manifests = {template}\n\
                 test_manifests = {tests}\n\
                 package_archives = {archives}\n"
            ),
        );
    }

    fn analyze(&self) -> Result<sc_lint_boundary::FindingsReport, BoundaryError> {
        analyze_workspace_configured(
            &AnalyzeOptions {
                root: self.root().to_path_buf(),
                format: OutputFormat::Json,
                rule: None,
            },
            None,
            Some("SCB-MANIFEST-007,SCB-MANIFEST-008"),
        )
    }
}

#[test]
fn dependency_requirement_violations_are_inactive_without_the_config_family() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/app", "crates/internal"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"

[workspace.dependencies]
exact = "=1.2.3"
bounded = ">=1.2, <1.8"
"#,
    );
    fixture.write("sc-lint.toml", "[tool.sc-lint.analyzers.boundary]\n");

    let report = fixture
        .analyze()
        .expect("dependency requirements without config analyze");
    assert!(report.findings.iter().all(|finding| {
        !matches!(
            finding.rule_id,
            RuleId::ScbManifest007 | RuleId::ScbManifest008
        )
    }));
}

#[test]
fn exact_and_tilde_registry_requirements_cover_workspace_template_and_test_manifests() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/app", "crates/internal"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"

[workspace.dependencies]
workspace_exact = "=1.2.3"
workspace_tilde = "~1.2.3"
workspace_caret = "^1.2.3"
"#,
    );
    fixture.write(
        "templates/sample/Cargo.toml",
        r#"[dependencies]
template_exact = "=2.3.4"
template_tilde = { version = "~2.3.4" }
template_caret = "^2.3.4"
"#,
    );
    fixture.write(
        "tests/fixtures/sample/Cargo.toml",
        r#"[dependencies]
test_exact = "=3.4.5"
test_tilde = "~3.4.5"
test_caret_old_line = "^0.8.0"
"#,
    );
    fixture.config(
        "[\"templates/**/Cargo.toml\"]",
        "[\"tests/fixtures/**/Cargo.toml\"]",
        "[]",
    );

    let report = fixture.analyze().expect("dependency requirements analyze");
    let exact_or_tilde = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::ScbManifest007)
        .collect::<Vec<_>>();
    assert_eq!(exact_or_tilde.len(), 6);
    for (path, dependency) in [
        ("Cargo.toml", "workspace_exact"),
        ("Cargo.toml", "workspace_tilde"),
        ("templates/sample/Cargo.toml", "template_exact"),
        ("templates/sample/Cargo.toml", "template_tilde"),
        ("tests/fixtures/sample/Cargo.toml", "test_exact"),
        ("tests/fixtures/sample/Cargo.toml", "test_tilde"),
    ] {
        assert!(exact_or_tilde.iter().any(|finding| {
            finding.message.contains(path) && finding.message.contains(dependency)
        }));
    }
    assert!(report.findings.iter().all(|finding| {
        !finding.message.contains("workspace_caret")
            && !finding.message.contains("template_caret")
            && !finding.message.contains("test_caret_old_line")
            && !finding.message.contains("internal")
    }));
}

#[test]
fn extra_upper_bounds_require_a_matching_comment_and_respect_caret_ranges() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/app", "crates/internal"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"

[workspace.dependencies]
no_comment = ">=1.2.0, <1.8.0"
same_line_match = { version = ">=1.2.0, <1.8.0" } # upper-bound: api break first in 1.8.0
# upper-bound: links break first in 1.8.0
previous_line_match = ">=1.2.0, <1.8.0"
wrong_version = { version = ">=1.2.0, <1.8.0" } # upper-bound: toolchain break first in 1.7.0
caret_equivalent = ">=1.2.0, <2.0.0"
zero_caret_equivalent = ">=0.2.3, <0.3.0"
zero_extra = ">=0.2.3, <0.2.8"
zero_singleton_equivalent = ">=0.0.4, <0.0.5"
prerelease_caret_equivalent = ">=1.2.3-alpha.1, <2.0.0"
prerelease_extra = ">=1.2.3-alpha.1, <1.2.3"
"#,
    );
    fixture.config("[]", "[]", "[]");

    let report = fixture.analyze().expect("dependency requirements analyze");
    let upper_bounds = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::ScbManifest008)
        .collect::<Vec<_>>();
    assert_eq!(upper_bounds.len(), 4);
    for dependency in [
        "no_comment",
        "wrong_version",
        "zero_extra",
        "prerelease_extra",
    ] {
        assert!(
            upper_bounds
                .iter()
                .any(|finding| finding.message.contains(dependency))
        );
    }
    for dependency in [
        "same_line_match",
        "previous_line_match",
        "caret_equivalent",
        "zero_caret_equivalent",
        "zero_singleton_equivalent",
        "prerelease_caret_equivalent",
    ] {
        assert!(
            upper_bounds
                .iter()
                .all(|finding| !finding.message.contains(dependency))
        );
    }
}

#[test]
fn package_archive_findings_name_archive_and_orig_line() {
    let fixture = Fixture::new();
    fixture.config("[]", "[]", "[\"target/package/*.crate\"]");
    let archive_path = fixture.root().join("target/package/sample-1.0.0.crate");
    write_crate_archive(
        &archive_path,
        &[(
            "sample-1.0.0/Cargo.toml.orig",
            "[dependencies]\npin = \"=1.2.3\"\n",
        )],
    );

    let report = fixture.analyze().expect("archive requirements analyze");
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.rule_id == RuleId::ScbManifest007)
        .expect("archive pin finding");
    assert!(
        finding
            .message
            .contains("target/package/sample-1.0.0.crate")
    );
    assert!(finding.message.contains("sample-1.0.0/Cargo.toml.orig:2"));
}

#[test]
fn package_archive_rejects_entries_escaping_its_root_as_a_typed_error() {
    let fixture = Fixture::new();
    fixture.config("[]", "[]", "[\"target/package/*.crate\"]");
    let archive_path = fixture.root().join("target/package/unsafe-1.0.0.crate");
    write_escape_archive(&archive_path, "unsafe-1.0.0/../../outside");

    let error = fixture.analyze().expect_err("archive escape is rejected");
    match error {
        BoundaryError::ArchivePathEscape { archive, entry } => {
            assert!(archive.to_string_lossy().contains("unsafe-1.0.0.crate"));
            assert!(entry.to_string_lossy().contains("outside"));
        }
        other => panic!("expected ArchivePathEscape, got {other:?}"),
    }
}

fn write_crate_archive(path: &Path, entries: &[(&str, &str)]) {
    fs::create_dir_all(path.parent().expect("archive parent")).expect("create archive parent");
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(encoder);
    for (entry_path, contents) in entries {
        let mut header = Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, entry_path, contents.as_bytes())
            .expect("append archive entry");
    }
    let encoder = builder.into_inner().expect("finish tar builder");
    let bytes = encoder.finish().expect("finish gzip stream");
    let mut file = fs::File::create(path).expect("create archive");
    file.write_all(&bytes).expect("write archive");
}

fn write_escape_archive(path: &Path, entry_path: &str) {
    fs::create_dir_all(path.parent().expect("archive parent")).expect("create archive parent");
    let mut header = [0u8; 512];
    header[..entry_path.len()].copy_from_slice(entry_path.as_bytes());
    write_octal(&mut header[100..108], 0o644);
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], 1);
    write_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    let checksum_text = format!("{checksum:06o}\0 ");
    header[148..156].copy_from_slice(checksum_text.as_bytes());

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&header).expect("write tar header");
    encoder.write_all(b"x").expect("write tar contents");
    encoder.write_all(&[0; 511]).expect("pad tar contents");
    encoder.write_all(&[0; 1024]).expect("write tar terminator");
    let bytes = encoder.finish().expect("finish gzip stream");
    fs::write(path, bytes).expect("write archive");
}

fn write_octal(field: &mut [u8], value: u64) {
    let digits = format!("{value:0width$o}", width = field.len() - 1);
    field[..digits.len()].copy_from_slice(digits.as_bytes());
    field[digits.len()] = 0;
}
