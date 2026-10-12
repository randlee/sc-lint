use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::CliError;
use crate::DocsGuide;
use crate::command::DocsRequest;

const DOCS_UNAVAILABLE_CODE: &str = "CLI.SC_LINT_DOCS_UNAVAILABLE";

fn candidate_roots_for_executable(executable: &Path) -> Vec<PathBuf> {
    let Some(bin_dir) = executable.parent() else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    if let Some(prefix) = bin_dir.parent() {
        candidates.push(prefix.join("share/sc-lint/sc-lint-docs"));
        candidates.push(prefix.join("share/sc-lint/docs-bundle"));
    }
    candidates.push(bin_dir.join("docs-bundle"));
    candidates.push(bin_dir.join("sc-lint-docs"));
    candidates
}

pub(crate) fn installed_bundle_root_for_executable(executable: &Path) -> Option<PathBuf> {
    bundle_root_from_candidates(&candidate_roots_for_executable(executable)).ok()
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut candidates = std::env::current_exe()
        .map(|executable| candidate_roots_for_executable(&executable))
        .unwrap_or_default();
    // This keeps `cargo run` and source-checkout tests offline while allowing
    // release/Homebrew layouts to win when a physical installed bundle exists.
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs-bundle"));
    candidates
}

fn unavailable(path: &Path, cause: impl Into<String>) -> CliError {
    CliError::capability(format!(
        "the sc-lint documentation bundle is unavailable at `{}`",
        path.display()
    ))
    .with_code(DOCS_UNAVAILABLE_CODE)
    .with_cause(cause)
    .with_detail("bundle_path", json!(path.display().to_string()))
    .with_suggested_action(
        "Install the matching sc-lint-docs bundle in a supported install layout, then rerun `sc-lint docs`.",
    )
    .with_documentation("sc-lint docs installation")
}

#[expect(
    clippy::result_large_err,
    reason = "Missing documentation is reported through the shared top-level CliError contract."
)]
fn bundle_root() -> Result<PathBuf, CliError> {
    let candidates = candidate_roots();
    bundle_root_from_candidates(&candidates)
}

#[expect(
    clippy::result_large_err,
    reason = "Documentation discovery retains the shared top-level CliError contract."
)]
fn bundle_root_from_candidates(candidates: &[PathBuf]) -> Result<PathBuf, CliError> {
    for candidate in candidates {
        if candidate.join("manifest.toml").is_file() && candidate.join("README.md").is_file() {
            return dunce::canonicalize(candidate).map_err(|error| {
                unavailable(
                    candidate,
                    format!("could not canonicalize bundle path: {error}"),
                )
            });
        }
    }
    let searched = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("docs-bundle"));
    Err(unavailable(
        &searched,
        "the bundle manifest or overview file was not found in any supported install layout",
    ))
}

#[expect(
    clippy::result_large_err,
    reason = "Documentation read failures retain the shared top-level CliError contract."
)]
fn read_guide(root: &Path, guide: DocsGuide) -> Result<(PathBuf, String), CliError> {
    let path = root.join(guide.metadata().path);
    let content = fs::read_to_string(&path).map_err(|error| {
        unavailable(
            root,
            format!("could not read guide `{}`: {error}", guide.metadata().path),
        )
    })?;
    Ok((path, content))
}

#[expect(
    clippy::result_large_err,
    reason = "Documentation discovery retains the shared top-level CliError contract."
)]
pub(crate) fn run(request: DocsRequest) -> Result<Value, CliError> {
    let root = bundle_root()?;
    if request.path {
        let path = request
            .guide
            .map_or_else(|| root.clone(), |guide| root.join(guide.metadata().path));
        if !path.is_file() && request.guide.is_some() {
            return Err(unavailable(
                &root,
                format!("guide path `{}` is missing from the bundle", path.display()),
            ));
        }
        return Ok(json!({
            "status": "pass",
            "guide": request.guide.map(|guide| guide.metadata().name),
            "path": path.display().to_string(),
            "bundle_path": root.display().to_string(),
        }));
    }

    let guide = request.guide.unwrap_or(DocsGuide::Overview);
    let (path, content) = read_guide(&root, guide)?;
    Ok(json!({
        "status": "pass",
        "guide": guide.metadata().name,
        "path": if request.guide.is_some() { path.display().to_string() } else { root.display().to_string() },
        "bundle_path": root.display().to_string(),
        "content": content,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const REQUIRED_OPERATOR_GUIDES: &[&str] = &[
        "README.md",
        "installation.md",
        "using-sc-lint.md",
        "configuration.md",
        "just-setup.md",
        "ci.md",
        "upgrade.md",
        "troubleshooting.md",
        "best-practices.md",
    ];

    fn source_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs-bundle")
    }

    #[test]
    fn bundle_manifest_covers_every_publishable_package() {
        let root = source_root();
        let bundle: toml::Value = toml::from_str(
            &fs::read_to_string(root.join("manifest.toml")).expect("bundle manifest"),
        )
        .expect("valid bundle manifest");
        let release: toml::Value =
            toml::from_str(include_str!("../../../release/publish-artifacts.toml"))
                .expect("valid release manifest");
        let documented: BTreeSet<_> = bundle["guides"]
            .as_array()
            .expect("guides")
            .iter()
            .filter(|guide| guide["kind"].as_str() == Some("package"))
            .filter_map(|guide| guide["package"].as_str())
            .collect();
        let publishable: BTreeSet<_> = release["crates"]
            .as_array()
            .expect("crates")
            .iter()
            .filter(|crate_entry| crate_entry["publish"].as_bool() == Some(true))
            .filter_map(|crate_entry| crate_entry["package"].as_str())
            .collect();
        assert_eq!(documented, publishable);
    }

    #[test]
    fn bundle_manifest_files_exist_and_relative_links_resolve() {
        let root = source_root();
        let bundle: toml::Value = toml::from_str(
            &fs::read_to_string(root.join("manifest.toml")).expect("bundle manifest"),
        )
        .expect("valid bundle manifest");
        for entry in bundle["guides"].as_array().expect("guides") {
            let path = entry["path"].as_str().expect("guide path");
            assert!(root.join(path).is_file(), "missing guide {path}");
            let content = fs::read_to_string(root.join(path)).expect("guide contents");
            for link in content
                .split("](")
                .skip(1)
                .filter_map(|tail| tail.split(')').next())
            {
                let target = link.split('#').next().unwrap_or(link);
                if target.is_empty()
                    || target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with("mailto:")
                {
                    continue;
                }
                let link_path = root.join(path).parent().expect("guide parent").join(target);
                assert!(link_path.is_file(), "broken link {path} -> {target}");
            }
        }
    }

    #[test]
    fn bundle_manifest_lists_every_required_operator_guide() {
        let bundle: toml::Value = toml::from_str(
            &fs::read_to_string(source_root().join("manifest.toml")).expect("bundle manifest"),
        )
        .expect("valid bundle manifest");
        let operator_guides: BTreeSet<_> = bundle["guides"]
            .as_array()
            .expect("guides")
            .iter()
            .filter(|guide| matches!(guide["kind"].as_str(), Some("overview" | "operator")))
            .filter_map(|guide| guide["path"].as_str())
            .collect();
        let required: BTreeSet<_> = REQUIRED_OPERATOR_GUIDES.iter().copied().collect();
        assert_eq!(operator_guides, required);
    }

    #[test]
    fn troubleshooting_documents_every_stable_sc_lint_error_code() {
        let troubleshooting = fs::read_to_string(source_root().join("troubleshooting.md"))
            .expect("troubleshooting guide");
        let documented = collect_sc_lint_codes(&troubleshooting);
        let implemented = [
            include_str!("config.rs"),
            include_str!("consumer_integration.rs"),
            include_str!("installer.rs"),
            include_str!("workflow.rs"),
            include_str!("docs.rs"),
        ]
        .into_iter()
        .flat_map(collect_sc_lint_codes)
        .collect::<BTreeSet<_>>();
        assert!(
            implemented.is_subset(&documented),
            "troubleshooting.md omits stable codes: {:?}",
            implemented.difference(&documented).collect::<Vec<_>>()
        );
    }

    fn collect_sc_lint_codes(text: &str) -> BTreeSet<&str> {
        text.match_indices("CLI.SC_LINT_")
            .filter_map(|(offset, _)| {
                let candidate = &text[offset..];
                let end = candidate
                    .find(|character: char| {
                        !(character.is_ascii_uppercase()
                            || character.is_ascii_digit()
                            || character == '_'
                            || character == '.')
                    })
                    .unwrap_or(candidate.len());
                let code = &candidate[..end];
                (code.len() > "CLI.SC_LINT_".len()).then_some(code)
            })
            .collect()
    }

    #[test]
    fn docs_paths_are_relative_to_the_bundle_and_read_only() {
        let root = source_root();
        let request = DocsRequest {
            guide: Some(DocsGuide::JustSetup),
            path: true,
        };
        let result = run(request).expect("docs path");
        assert!(
            result["path"]
                .as_str()
                .expect("path")
                .ends_with("just-setup.md")
        );
        assert!(root.join("just-setup.md").is_file());
    }

    #[test]
    fn installed_layout_candidates_cover_archive_and_homebrew_pkgshare() {
        let release: toml::Value =
            toml::from_str(include_str!("../../../release/publish-artifacts.toml"))
                .expect("valid release manifest");
        let lint_binary = release["release_binaries"]
            .as_array()
            .expect("release binaries")
            .iter()
            .find(|binary| binary["name"].as_str() == Some("sc-lint"))
            .expect("sc-lint release binary");
        let bundled_path = lint_binary["bundled_paths"]
            .as_array()
            .expect("sc-lint bundled paths")
            .first()
            .expect("docs bundle path");
        let archive_destination = Path::new(
            bundled_path["destination"]
                .as_str()
                .expect("archive destination"),
        );
        let archive_root = Path::new("/release/sc-lint_0.7.0_x86_64-unknown-linux-gnu");
        let archive_binary = archive_root.join("bin/sc-lint");
        let expected_archive_docs = archive_root.join(archive_destination);
        assert!(
            candidate_roots_for_executable(&archive_binary).contains(&expected_archive_docs),
            "archive docs destination from the release manifest must be discoverable"
        );

        let formula_path = release["channels"]["homebrew"]["formulas"]
            .as_array()
            .expect("Homebrew formulas")
            .iter()
            .find(|formula| formula["path"].as_str() == Some("Formula/sc-lint.rb"))
            .and_then(|formula| formula["path"].as_str())
            .expect("sc-lint Homebrew formula path");
        let formula_name = Path::new(formula_path)
            .file_stem()
            .expect("Homebrew formula filename");
        let homebrew_components = bundled_path["homebrew_destination_components"]
            .as_array()
            .expect("Homebrew destination components");
        assert_eq!(
            homebrew_components[0].as_str(),
            Some("pkgshare"),
            "Homebrew bundle must start at pkgshare"
        );
        let mut expected_homebrew_docs = PathBuf::from("/opt/homebrew/share").join(formula_name);
        for component in homebrew_components.iter().skip(1) {
            expected_homebrew_docs.push(component.as_str().expect("path component"));
        }
        let homebrew_binary = Path::new("/opt/homebrew/bin/sc-lint");
        assert!(
            candidate_roots_for_executable(homebrew_binary).contains(&expected_homebrew_docs),
            "Homebrew docs destination from the release manifest must be discoverable"
        );

        let root = source_root();
        let discovered =
            bundle_root_from_candidates(std::slice::from_ref(&root)).expect("bundle discovery");
        assert_eq!(
            discovered,
            dunce::canonicalize(root).expect("canonical source bundle")
        );
    }

    #[test]
    fn published_docs_win_over_legacy_bundle_when_both_are_installed() {
        let temporary = tempfile::TempDir::new().expect("temporary installation");
        let prefix = temporary.path();
        let bin = prefix.join("bin");
        let executable = bin.join("sc-lint");
        let published_docs = prefix.join("share/sc-lint/sc-lint-docs");
        let legacy_docs = bin.join("sc-lint-docs");
        fs::create_dir_all(&bin).expect("bin directory");
        fs::write(&executable, "launcher").expect("launcher");
        for docs in [&published_docs, &legacy_docs] {
            fs::create_dir_all(docs).expect("docs directory");
            fs::write(docs.join("manifest.toml"), "schema_version = 1\n").expect("manifest");
            fs::write(docs.join("README.md"), "# docs\n").expect("overview");
        }

        assert_eq!(
            installed_bundle_root_for_executable(&executable),
            Some(dunce::canonicalize(published_docs).expect("published docs path"))
        );
    }

    #[test]
    fn canonical_just_guide_contains_the_product_template_verbatim() {
        let guide = fs::read_to_string(source_root().join("just-setup.md")).expect("just guide");
        let template = include_str!("../assets/consumer-Justfile").trim();
        assert!(
            guide.contains(template),
            "just-setup.md must embed the canonical generated consumer template"
        );
    }
}
