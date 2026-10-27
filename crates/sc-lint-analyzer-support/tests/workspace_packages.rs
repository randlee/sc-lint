use sc_lint_analyzer_support::{PackageDependencyKind, PackageTargetKind, load_workspace_metadata};
use tempfile::TempDir;

fn write(root: &TempDir, path: &str, contents: &str) {
    let path = root.path().join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

#[test]
fn workspace_packages_reports_dependency_kinds_and_renames() {
    let root = TempDir::new().unwrap();
    write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/example\"]\nexclude = [\"vendor/tokio\", \"vendor/tokio-renamed\", \"vendor/tokio-unix\", \"vendor/tokio-build\", \"vendor/tokio-dev\"]\nresolver = \"2\"\n",
    );
    write(
        &root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname=\"tokio\"\nversion=\"1.0.0\"\nedition=\"2024\"\n",
    );
    write(&root, "vendor/tokio/src/lib.rs", "");
    for (stub, version) in [
        ("tokio-renamed", "1.0.1"),
        ("tokio-unix", "1.0.2"),
        ("tokio-build", "1.0.3"),
        ("tokio-dev", "1.0.4"),
    ] {
        write(
            &root,
            &format!("vendor/{stub}/Cargo.toml"),
            &format!("[package]\nname=\"tokio\"\nversion=\"{version}\"\nedition=\"2024\"\n"),
        );
        write(&root, &format!("vendor/{stub}/src/lib.rs"), "");
    }
    write(&root, "crates/example/src/lib.rs", "");
    write(
        &root,
        "crates/example/Cargo.toml",
        "[package]\nname=\"example\"\nversion=\"1.0.0\"\nedition=\"2024\"\n[dependencies]\ntokio = { path=\"../../vendor/tokio\" }\nrt = { package=\"tokio\", path=\"../../vendor/tokio-renamed\" }\n[target.'cfg(unix)'.dependencies]\ntokio_unix = { package=\"tokio\", path=\"../../vendor/tokio-unix\" }\n[build-dependencies]\ntokio_build = { package=\"tokio\", path=\"../../vendor/tokio-build\" }\n[dev-dependencies]\ntokio_dev = { package=\"tokio\", path=\"../../vendor/tokio-dev\" }\n",
    );
    let packages = load_workspace_metadata(root.path())
        .unwrap()
        .workspace_packages();
    let package = packages.first().unwrap();
    assert_eq!(package.name.as_str(), "example");
    assert!(
        package
            .dependencies
            .iter()
            .any(|dependency| dependency.name == "tokio"
                && dependency.kind == PackageDependencyKind::Normal)
    );
    assert!(
        package
            .dependencies
            .iter()
            .any(|dependency| dependency.rename.as_deref() == Some("rt")
                && dependency.kind == PackageDependencyKind::Normal)
    );
    assert!(
        package
            .dependencies
            .iter()
            .any(|dependency| dependency.kind == PackageDependencyKind::Build)
    );
    assert!(
        package
            .dependencies
            .iter()
            .any(|dependency| dependency.kind == PackageDependencyKind::Development)
    );
    assert!(
        package
            .dependencies
            .iter()
            .any(|dependency| dependency.target.as_deref() == Some("cfg(unix)"))
    );
}

#[test]
fn workspace_packages_maps_target_kinds_and_sorts_members() {
    let root = TempDir::new().unwrap();
    write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers=[\"crates/zeta\", \"crates/alpha\"]\nresolver=\"2\"\n",
    );
    for package in ["zeta", "alpha"] {
        write(&root, &format!("crates/{package}/src/lib.rs"), "");
        write(&root, &format!("crates/{package}/src/main.rs"), "");
        write(&root, &format!("crates/{package}/examples/demo.rs"), "");
        write(&root, &format!("crates/{package}/tests/check.rs"), "");
        write(&root, &format!("crates/{package}/benches/bench.rs"), "");
        write(
            &root,
            &format!("crates/{package}/build.rs"),
            "fn main() {}\n",
        );
        write(
            &root,
            &format!("crates/{package}/Cargo.toml"),
            &format!(
                "[package]\nname=\"{package}\"\nversion=\"1.0.0\"\nedition=\"2024\"\nbuild=\"build.rs\"\n[[example]]\nname=\"demo\"\npath=\"examples/demo.rs\"\n[[bench]]\nname=\"bench\"\npath=\"benches/bench.rs\"\nharness=false\n"
            ),
        );
    }
    let packages = load_workspace_metadata(root.path())
        .unwrap()
        .workspace_packages();
    assert_eq!(
        packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    let kinds = packages[0]
        .targets
        .iter()
        .map(|target| target.kind)
        .collect::<Vec<_>>();
    for expected in [
        PackageTargetKind::Lib,
        PackageTargetKind::Bin,
        PackageTargetKind::Example,
        PackageTargetKind::Test,
        PackageTargetKind::Bench,
        PackageTargetKind::CustomBuild,
    ] {
        assert!(kinds.contains(&expected), "missing {expected:?}");
    }
}
