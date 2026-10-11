use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::BoundaryError as BoundaryBackendError;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_schema::OutputFormat;
use serde_json::Error as JsonError;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::command::CommandContext;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;
use crate::consts;

#[derive(Debug)]
enum BoundaryDispatchError {
    Analysis(Box<BoundaryBackendError>),
    Serialize(JsonError),
    Normalize(JsonError),
}

impl BoundaryDispatchError {
    fn into_cli_error(self, tool: &str, repo_root: Option<&Path>) -> CliError {
        match self {
            Self::Analysis(error) => match *error {
                BoundaryBackendError::InventoryLoad { .. }
                | BoundaryBackendError::ManifestPolicyAnalysis { .. }
                | BoundaryBackendError::PackagePolicyAnalysis { .. } => {
                    CliError::config(format!("{tool} rejected repository configuration"))
                        .with_source(error)
                        .with_detail(consts::FIELD_TOOL, json!(tool))
                        .with_detail(
                            consts::FIELD_ROOT,
                            json!(repo_root.map(|root| root.display().to_string())),
                        )
                        .with_suggested_action(
                            "Check the boundary inventory and manifest configuration, then rerun `sc-lint lint sc-boundary` for a focused failure report.",
                        )
                }
                BoundaryBackendError::WorkspaceGraphBuild { .. } => {
                    CliError::backend_failure(format!("{tool} failed to analyze the workspace"))
                        .with_source(error)
                        .with_detail(consts::FIELD_TOOL, json!(tool))
                        .with_detail(
                            consts::FIELD_ROOT,
                            json!(repo_root.map(|root| root.display().to_string())),
                        )
                        .with_suggested_action(
                            "Check the boundary inventory and workspace sources, then rerun `sc-lint lint sc-boundary` for a focused failure report.",
                        )
                }
                BoundaryBackendError::AnalyzerConfig(ref config_error) => {
                    let code = config_error.code();
                    // A bad selector is a usage error (exit 2); every other
                    // config error is a configuration error (exit 3).
                    let err = if config_error.is_selector() {
                        CliError::usage(format!("{tool} rejected --rule-filter"))
                    } else {
                        CliError::config(format!(
                            "{tool} rejected [tool.sc-lint.analyzers.boundary]"
                        ))
                    };
                    err.with_source(error)
                        .with_detail(consts::FIELD_TOOL, json!(tool))
                        .with_detail("analyzer_config_code", json!(code))
                        .with_suggested_action(
                            "Fix the [tool.sc-lint.analyzers.boundary] table or the rule selector named in the error, then rerun `sc-lint lint sc-boundary`.",
                        )
                }
            },
            Self::Serialize(error) => CliError::backend_protocol(format!(
                "{tool} produced a report that could not be encoded as machine JSON"
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_suggested_action(
                "Inspect the backend report payload for non-serializable fields and rerun the analyzer.",
            ),
            Self::Normalize(error) => {
                CliError::backend_protocol(format!("{tool} returned malformed machine output"))
                    .with_source(error)
                    .with_detail(consts::FIELD_TOOL, json!(tool))
                    .with_suggested_action(
                        "Run the backend directly and inspect its JSON output for schema mismatches.",
                    )
            }
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Dispatch failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_boundary(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let tool = consts::TOOL_BOUNDARY;
    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: repo_root.to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        },
        loaded_config.config_path(),
        context.rule_filter(),
    )
    .map_err(|error| BoundaryDispatchError::Analysis(Box::new(error)))
    .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;

    let raw = serde_json::to_string(&report)
        .map_err(BoundaryDispatchError::Serialize)
        .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;
    let normalized = normalize_backend_json(tool, &raw)?;
    let finding_count = normalized
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new(tool, finding_count),
    ))
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_portability(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    run_delegated_backend(
        loaded_config,
        consts::TOOL_PORTABILITY,
        context.rule_filter(),
    )
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_runtime(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    run_delegated_backend(loaded_config, consts::TOOL_RUNTIME, context.rule_filter())
}

#[expect(
    clippy::result_large_err,
    reason = "Backend protocol violations must remain in the shared top-level CliError contract."
)]
pub fn normalize_backend_json(tool: &str, raw: &str) -> Result<Value, CliError> {
    serde_json::from_str(raw)
        .map_err(BoundaryDispatchError::Normalize)
        .map_err(|error| error.into_cli_error(tool, None))
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
fn run_delegated_backend(
    loaded_config: &LoadedConfig,
    tool: &'static str,
    rule_filter: Option<&str>,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let backend_binary = find_backend_binary(tool);
    let output = ProcessCommand::new(&backend_binary)
        .args(backend_args(
            repo_root,
            loaded_config.config_path(),
            rule_filter,
        ))
        .output()
        .map_err(|error| {
            CliError::backend_failure(format!(
                "{tool} failed to start for `{}`",
                repo_root.display()
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_detail(consts::FIELD_ROOT, json!(repo_root.display().to_string()))
            .with_detail(
                consts::FIELD_BACKEND_PATH,
                json!(backend_binary.display().to_string()),
            )
            .with_suggested_action(
                "Install the release archive so the backend binary sits beside `sc-lint`, or run `cargo build --workspace` in a source checkout, then rerun the profile.",
            )
            .with_documentation("sc-lint docs setup")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let cause = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("delegated backend `{tool}` exited non-zero")
        };
        if let Some(error) = backend_rejection(tool, output.status.code(), &cause) {
            return Err(error);
        }
        return Err(CliError::backend_failure(format!(
            "{tool} failed to analyze `{}`",
            repo_root.display()
        ))
        .with_cause(cause)
        .with_detail(consts::FIELD_TOOL, json!(tool))
        .with_detail(consts::FIELD_ROOT, json!(repo_root.display().to_string()))
        .with_detail(
            consts::FIELD_BACKEND_PATH,
            json!(backend_binary.display().to_string()),
        )
        .with_detail(consts::FIELD_EXIT_CODE, json!(output.status.code()))
        .with_suggested_action(format!(
            "Run `{tool} analyze --root <repo> --format json` directly for the backend's own diagnostics, then rerun the profile."
        )));
    }

    let raw = std::str::from_utf8(&output.stdout).map_err(|error| {
        CliError::backend_protocol(format!("{tool} returned non-utf8 machine output"))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_suggested_action(
                "Run the backend binary directly and inspect its stdout encoding before rerunning sc-lint.",
            )
    })?;
    let normalized = normalize_backend_json(tool, raw)?;
    let finding_count = normalized
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new(tool, finding_count),
    ))
}

fn backend_args(root: &Path, config: Option<&Path>, rule_filter: Option<&str>) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "analyze".into(),
        "--root".into(),
        root.into(),
        "--format".into(),
        "json".into(),
    ];
    if let Some(config) = config {
        args.extend(["--config".into(), config.into()]);
    }
    if let Some(filter) = rule_filter {
        args.extend(["--rule-filter".into(), filter.into()]);
    }
    args
}

/// Backend exit 2 rejected its arguments (`--rule-filter`); exit 3 rejected
/// its `[tool.sc-lint.analyzers.<name>]` table.
fn backend_rejection(tool: &str, code: Option<i32>, cause: &str) -> Option<CliError> {
    let error = match code {
        Some(2) => CliError::usage(format!("{tool} rejected arguments")),
        Some(3) => CliError::config(format!("{tool} rejected [tool.sc-lint.analyzers.*]")),
        _ => return None,
    };
    Some(
        error
            .with_cause(cause)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_suggested_action(
                "Fix the named key in [tool.sc-lint.analyzers.<name>] or the --rule-filter selector, then rerun the lint target.",
            ),
    )
}

/// The running `sc-lint` executable, used when a profile step re-enters the
/// product instead of a source-tree helper.
pub(crate) fn product_binary() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from(consts::SERVICE_NAME))
}

/// True when the running executable sits in the released archive layout:
/// every declared backend binary and the documentation bundle are beside it,
/// so no step depends on a source checkout.
pub(crate) fn self_contained_layout() -> bool {
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(self_contained_layout_in_dir)
}

fn self_contained_layout_in_dir(dir: &Path) -> bool {
    [
        consts::TOOL_BOUNDARY,
        consts::TOOL_PORTABILITY,
        consts::TOOL_RUNTIME,
    ]
    .into_iter()
    .all(|tool| find_backend_binary_in_dir(dir, tool).is_some())
        && dir.join(consts::DOCS_BUNDLE_DIR).is_dir()
}

fn find_backend_binary(tool: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|current_exe| find_backend_binary_from_exe(&current_exe, tool))
        // Falling back to PATH keeps local development, CI, and non-sibling
        // binary layouts working when the installed sibling lookup does not.
        .unwrap_or_else(|| PathBuf::from(tool))
}

fn find_backend_binary_from_exe(current_exe: &Path, tool: &str) -> Option<PathBuf> {
    let parent = current_exe.parent()?;
    find_backend_binary_in_dir(parent, tool).or_else(|| {
        parent
            .parent()
            .and_then(|grandparent| find_backend_binary_in_dir(grandparent, tool))
    })
}

fn find_backend_binary_in_dir(dir: &Path, tool: &str) -> Option<PathBuf> {
    let sibling = dir.join(tool);
    if sibling.is_file() {
        return Some(sibling);
    }
    if cfg!(windows) {
        let sibling_exe = dir.join(format!("{tool}.exe"));
        if sibling_exe.is_file() {
            return Some(sibling_exe);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::BoundaryDispatchError;
    use super::backend_args;
    use super::backend_rejection;
    use super::find_backend_binary_from_exe;
    use super::self_contained_layout_in_dir;
    use crate::consts;
    use crate::error::CliErrorKind;
    use sc_lint_boundary::AnalyzeOptions;
    use sc_lint_boundary::analyze_workspace_configured;
    use sc_lint_schema::OutputFormat;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn assert_analyzer_config_error(
        error: sc_lint_boundary::BoundaryError,
        expected_kind: CliErrorKind,
        expected_code: &str,
    ) {
        let cli_error = BoundaryDispatchError::Analysis(Box::new(error))
            .into_cli_error(consts::TOOL_BOUNDARY, None);

        assert_eq!(cli_error.kind, expected_kind);
        assert_eq!(
            cli_error.details.get("analyzer_config_code"),
            Some(&serde_json::json!(expected_code))
        );
    }

    #[test]
    fn analyzer_config_errors_map_selector_to_usage_and_unknown_key_to_config() {
        let temp_dir = TempDir::new().expect("temp dir");
        let root = temp_dir.path();
        let options = AnalyzeOptions {
            root: root.to_path_buf(),
            format: OutputFormat::Json,
            rule: None,
        };

        let selector_error = analyze_workspace_configured(&options, None, Some("SCB-NOPE-001"))
            .expect_err("unknown selector is rejected before workspace analysis");
        assert_analyzer_config_error(
            selector_error,
            CliErrorKind::Usage,
            "ANALYZER_CONFIG.SELECTOR",
        );

        let config_path = root.join("sc-lint.toml");
        fs::write(
            &config_path,
            "[tool.sc-lint.analyzers.boundary]\nunknown = true\n",
        )
        .expect("config");
        let config_error = analyze_workspace_configured(&options, Some(&config_path), None)
            .expect_err("unknown analyzer key is rejected before workspace analysis");
        assert_analyzer_config_error(
            config_error,
            CliErrorKind::Config,
            "ANALYZER_CONFIG.UNKNOWN_KEY",
        );
    }

    #[test]
    fn backend_args_forward_config_and_rule_filter() {
        let args = backend_args(
            Path::new("/r"),
            Some(Path::new("/r/sc-lint.toml")),
            Some("PORT-001,PORT-006"),
        );
        assert_eq!(
            args,
            [
                "analyze",
                "--root",
                "/r",
                "--format",
                "json",
                "--config",
                "/r/sc-lint.toml",
                "--rule-filter",
                "PORT-001,PORT-006",
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn backend_args_omit_absent_options() {
        assert_eq!(
            backend_args(Path::new("/r"), None, None),
            ["analyze", "--root", "/r", "--format", "json"].map(OsString::from)
        );
    }

    #[test]
    fn backend_exit_two_is_usage_and_exit_three_is_config() {
        let usage = backend_rejection("sc-lint-portability", Some(2), "unknown rule `NOPE`")
            .expect("exit 2 maps");
        assert_eq!(usage.kind, crate::CliErrorKind::Usage);
        assert_eq!(usage.exit_code(), 2);
        assert!(usage.message.contains("rejected arguments"));
        assert_eq!(usage.cause.as_deref(), Some("unknown rule `NOPE`"));

        let config = backend_rejection("sc-lint-runtime", Some(3), "unknown key `targets`")
            .expect("exit 3 maps");
        assert_eq!(config.kind, crate::CliErrorKind::Config);
        assert_eq!(config.exit_code(), 3);

        assert!(backend_rejection("sc-lint-runtime", Some(1), "boom").is_none());
        assert!(backend_rejection("sc-lint-runtime", None, "signal").is_none());
    }

    #[test]
    fn self_contained_layout_requires_every_backend_and_the_docs_bundle() {
        let temp_dir = TempDir::new().expect("temp dir");
        let dir = temp_dir.path();
        assert!(!self_contained_layout_in_dir(dir));
        for tool in ["sc-lint-boundary", "sc-lint-portability", "sc-lint-runtime"] {
            fs::write(dir.join(tool), "").expect("backend");
        }
        assert!(!self_contained_layout_in_dir(dir));
        fs::create_dir_all(dir.join(crate::consts::DOCS_BUNDLE_DIR)).expect("docs");
        assert!(self_contained_layout_in_dir(dir));
    }

    #[test]
    fn prefers_installed_sibling_backend_binary() {
        let temp_dir = TempDir::new().expect("temp dir");
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        let current_exe = bin_dir.join("sc-lint");
        let backend = bin_dir.join("sc-lint-portability");
        fs::write(&current_exe, "").expect("current exe");
        fs::write(&backend, "").expect("backend");

        assert_eq!(
            find_backend_binary_from_exe(&current_exe, "sc-lint-portability"),
            Some(backend)
        );
    }

    #[test]
    fn falls_back_to_target_debug_backend_binary_for_test_layouts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let deps_dir = temp_dir.path().join("target").join("debug").join("deps");
        fs::create_dir_all(&deps_dir).expect("deps dir");
        let current_exe = deps_dir.join("sc-lint-tests");
        let backend = deps_dir
            .parent()
            .expect("target debug")
            .join("sc-lint-runtime");
        fs::write(&current_exe, "").expect("current exe");
        fs::write(&backend, "").expect("backend");

        assert_eq!(
            find_backend_binary_from_exe(&current_exe, "sc-lint-runtime"),
            Some(backend)
        );
    }
}
