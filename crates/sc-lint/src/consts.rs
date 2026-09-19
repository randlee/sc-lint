pub const SERVICE_NAME: &str = "sc-lint";
pub const TOOL_BOUNDARY: &str = "sc-lint-boundary";
pub const TOOL_PORTABILITY: &str = "sc-lint-portability";
pub const TOOL_RUNTIME: &str = "sc-lint-runtime";
pub const CMD_BOUNDARY: &str = "lint.sc-boundary";
pub const CMD_PORTABILITY: &str = "lint.sc-portability";
pub const CMD_RUNTIME: &str = "lint.sc-runtime";
pub const ACTION_CLI_PARSE_ERROR: &str = "cli.parse_error";
pub const FIELD_ADAPTER: &str = "adapter";
pub const FIELD_CONFIG_SCOPE: &str = "config_scope";
pub const FIELD_SCRIPT: &str = "script";
pub const FIELD_SUMMARY: &str = "summary";

pub const FIELD_TOOL: &str = "tool";
pub const FIELD_FINDINGS: &str = "findings";
pub const FIELD_STATUS: &str = "status";
pub const FIELD_VERSION: &str = "version";
pub const FIELD_CRATE_NAME: &str = "crate_name";
pub const FIELD_CRATE_VERSION: &str = "crate_version";

pub const FIELD_CODE: &str = "code";
pub const FIELD_KIND: &str = "kind";
pub const FIELD_MESSAGE: &str = "message";
pub const FIELD_CAUSE: &str = "cause";
pub const FIELD_DETAILS: &str = "details";
pub const FIELD_SUGGESTED_ACTION: &str = "suggested_action";
pub const FIELD_DOCS: &str = "docs";
pub const FIELD_STEPS: &str = "steps";
pub const FIELD_ROOT: &str = "root";
pub const FIELD_EXIT_CODE: &str = "exit_code";
pub const FIELD_BACKEND_PATH: &str = "backend_path";

/// Repo-relative directory of the consumer-provisioned Python virtual environment.
pub const VENV_RELATIVE_DIR: &str = ".sc-lint/venv";
/// Repo-relative `python-source` directory of the source checkout's wheel package.
pub const SOURCE_PYTHON_PACKAGE_DIR: &str = "bindings/sc-lint-py/python";
/// Documentation bundle directory shipped beside the release binaries.
pub const DOCS_BUNDLE_DIR: &str = "sc-lint-docs";
