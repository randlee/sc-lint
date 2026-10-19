use std::fs;
use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use sc_lint_boundary::ALL_BOUNDARY_IDS;
use sc_lint_boundary::BOUNDARY_CATALOG;
use sc_lint_boundary::DEFAULT_FAIL_IDS;
use sc_lint_boundary::RuleId;
use serde_json::Value;
use tempfile::TempDir;

const CHILD_DEADLINE: Duration = Duration::from_secs(60);

fn drain<R: Read + Send + 'static>(mut pipe: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

fn run_bounded(command: &mut Command, description: &str) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {description}: {error}"));
    let stdout = drain(child.stdout.take().expect("piped stdout"));
    let stderr = drain(child.stderr.take().expect("piped stderr"));
    let deadline = Instant::now() + CHILD_DEADLINE;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let status = terminate_process_tree(&mut child);
                let stderr = stderr.join().expect("stderr reader");
                panic!(
                    "{description} did not exit within {CHILD_DEADLINE:?}; status after kill: {status}\nstderr:\n{}",
                    String::from_utf8_lossy(&stderr)
                );
            }
            Err(error) => {
                let _ = terminate_process_tree(&mut child);
                let stderr = stderr.join().expect("stderr reader");
                panic!(
                    "failed waiting for {description}: {error}\nstderr:\n{}",
                    String::from_utf8_lossy(&stderr)
                );
            }
        }
    };
    Output {
        status,
        stdout: stdout.join().expect("stdout reader"),
        stderr: stderr.join().expect("stderr reader"),
    }
}

fn terminate_process_tree(child: &mut Child) -> std::process::ExitStatus {
    let process_id = child.id();
    #[cfg(unix)]
    if let Ok(group) = libc::pid_t::try_from(process_id)
        && group > 0
    {
        // SAFETY: a negative pid signals the process group assigned to this child.
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &process_id.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    child.wait().expect("reap terminated process tree leader")
}

#[test]
fn issue377_rule_registration_and_dispatch() {
    assert!(
        ALL_BOUNDARY_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-BOUNDARY-004")
    );
    assert!(
        ALL_BOUNDARY_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-DEPENDENCY-004")
    );
    assert!(
        DEFAULT_FAIL_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-BOUNDARY-004")
    );
    assert!(
        DEFAULT_FAIL_IDS
            .iter()
            .any(|id| id.as_str() == "SCB-DEPENDENCY-004")
    );
    assert!(
        BOUNDARY_CATALOG
            .families
            .iter()
            .find(|(name, _)| *name == "boundaries")
            .is_some_and(|(_, rules)| rules
                .iter()
                .any(|id| id.as_str() == RuleId::ScbBoundary004.as_str()))
    );
    assert!(
        BOUNDARY_CATALOG
            .families
            .iter()
            .find(|(name, _)| *name == "dependencies")
            .is_some_and(|(_, rules)| {
                rules
                    .iter()
                    .any(|id| id.as_str() == RuleId::ScbBoundary004.as_str())
                    && rules
                        .iter()
                        .any(|id| id.as_str() == RuleId::ScbDependency004.as_str())
            })
    );

    let fixture = Fixture::new();
    fixture.write_workspace();
    fixture.write_boundary("not-valid");
    fixture.write_stale_consumer_boundary();
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let output = run_bounded(
        Command::new(binary).args([
            "--json",
            "--root",
            fixture.root().to_str().expect("utf-8 root"),
            "lint",
            "sc-boundary",
            "--rule-filter",
            RuleId::ScbBoundary004.as_str(),
        ]),
        "sc-lint sc-boundary boundaries probe",
    );
    let status = output.status;
    let stdout = output.stdout;
    let stderr = output.stderr;
    assert_eq!(
        status.code(),
        Some(10),
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    let output: Value = serde_json::from_slice(&stdout).expect("JSON output");
    assert_eq!(output["data"]["status"], "fail");
    assert_eq!(output["data"]["findings"][0]["rule_id"], "SCB-BOUNDARY-004");
    assert_eq!(output["data"]["findings"][0]["kind"], "invalid_identifier");

    let output = run_bounded(
        Command::new(binary).args([
            "--json",
            "--root",
            fixture.root().to_str().expect("utf-8 root"),
            "lint",
            "sc-boundary",
            "--rule-filter",
            "dependencies",
        ]),
        "sc-lint sc-boundary dependencies probe",
    );
    assert_eq!(
        output.status.code(),
        Some(10),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(output["data"]["status"], "fail");
    let findings = output["data"]["findings"]
        .as_array()
        .expect("findings array");
    let rule_ids = findings
        .iter()
        .map(|finding| finding["rule_id"].as_str().expect("rule id"))
        .collect::<Vec<_>>();
    assert_eq!(
        rule_ids,
        ["SCB-BOUNDARY-004", "SCB-DEPENDENCY-004"],
        "dependencies-family findings must include both new ids in stable order"
    );
    assert_eq!(
        findings[1]["kind"], "stale_allowed_dependent",
        "the stale dependent fixture must dispatch SCB-DEPENDENCY-004"
    );
}

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        Self(TempDir::new().expect("tempdir"))
    }

    fn root(&self) -> &std::path::Path {
        self.0.path()
    }

    fn write_workspace(&self) {
        fs::create_dir_all(self.root().join("crates/example/src")).expect("crate directory");
        fs::create_dir_all(self.root().join("boundaries/example")).expect("boundary directory");
        fs::create_dir_all(self.root().join("crates/consumer/src")).expect("consumer directory");
        fs::write(
            self.root().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/example\", \"crates/consumer\"]\nresolver = \"2\"\n",
        )
        .expect("workspace manifest");
        fs::write(
            self.root().join("crates/example/Cargo.toml"),
            "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("crate manifest");
        fs::write(
            self.root().join("crates/consumer/Cargo.toml"),
            "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .expect("consumer manifest");
        fs::write(
            self.root().join("crates/example/src/lib.rs"),
            "pub struct Example;\n",
        )
        .expect("crate source");
        fs::write(
            self.root().join("crates/consumer/src/lib.rs"),
            "pub struct Consumer;\n",
        )
        .expect("consumer source");
        fs::write(
            self.root().join("boundaries/planning.toml"),
            "[planning]\ncurrent_sprint = \"K.3\"\n",
        )
        .expect("planning inventory");
    }

    fn write_boundary(&self, facade: &str) {
        fs::write(
            self.root().join("boundaries/example/example.toml"),
            format!(
                "boundary_id = \"BOUNDARY-Example\"\nowner_package = \"example\"\nowner_crate_path = \"example\"\nname = \"Example\"\n\n[public]\nfacade = \"{facade}\"\n\n[implementation]\ntype = \"Example\"\nmodule = \"example\"\nvisibility = \"public\"\nconstructor = \"none\"\n\n[composition]\nroots = [\"Example\"]\n\n[dependencies]\nallowed_dependents = []\nallowed_dependencies = []\nforbidden_edges = []\n\n[references]\nscope = \"outside_owner_crate\"\nforbidden = []\n\n[testing]\nallowed_test_double_paths = []\nforbidden_test_bypasses = []\n\n[enforcement]\nlint_rules = []\nreview_gates = []\n\n[status]\nstate = \"concrete_landed\"\n"
            ),
        )
        .expect("boundary inventory");
    }

    fn write_stale_consumer_boundary(&self) {
        fs::create_dir_all(self.root().join("boundaries/consumer"))
            .expect("consumer boundary directory");
        fs::write(
            self.root().join("boundaries/consumer/consumer.toml"),
            "boundary_id = \"BOUNDARY-Consumer\"\nowner_package = \"consumer\"\nowner_crate_path = \"consumer\"\nname = \"Consumer\"\n\n[public]\nfacade = \"Consumer\"\n\n[implementation]\ntype = \"Consumer\"\nmodule = \"consumer\"\nvisibility = \"public\"\nconstructor = \"none\"\n\n[composition]\nroots = [\"Consumer\"]\n\n[dependencies]\nallowed_dependents = [\"example\"]\nallowed_dependencies = []\nforbidden_edges = []\n\n[references]\nscope = \"outside_owner_crate\"\nforbidden = []\n\n[testing]\nallowed_test_double_paths = []\nforbidden_test_bypasses = []\n\n[enforcement]\nlint_rules = []\nreview_gates = []\n\n[status]\nstate = \"concrete_landed\"\n",
        )
        .expect("consumer boundary inventory");
    }
}
