use sc_lint_portability::{AnalyzeOptions, analyze_workspace};
use sc_lint_schema::OutputFormat;
use std::{fs, path::Path};
use tempfile::TempDir;

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn example_targets_do_not_change_portability_findings() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n[workspace]\n",
    );
    write(root, "src/lib.rs", "pub fn clean() {}\n");
    let options = AnalyzeOptions {
        root: root.to_path_buf(),
        format: OutputFormat::Json,
    };
    let before = analyze_workspace(&options).unwrap();
    write(
        root,
        "examples/example.rs",
        "use std::os::unix::net::UnixStream;\nfn main() { let _ = UnixStream::pair(); }\n",
    );
    let after = analyze_workspace(&options).unwrap();
    assert_eq!(before, after);
}
