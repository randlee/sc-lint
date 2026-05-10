use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::Cli;
use crate::CliError;
use crate::command::CommandContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    repo_root: Option<PathBuf>,
    config_path: Option<PathBuf>,
    logging_root: Option<PathBuf>,
    logging_console: bool,
}

impl LoadedConfig {
    pub fn load(cli: &Cli, context: &CommandContext) -> Result<Self, CliError> {
        if !context.requires_repo_root() {
            return Ok(Self {
                repo_root: None,
                config_path: None,
                logging_root: cli.log_root.clone(),
                logging_console: cli.log_console,
            });
        }

        let discovery_base = if let Some(root) = cli.root.as_ref() {
            root.clone()
        } else {
            std::env::current_dir().map_err(|error| {
                CliError::config(format!("failed to read current directory: {error}"))
            })?
        };
        let repo_root = discover_repo_root(&discovery_base)?;
        let config_path = find_repo_config(&repo_root);
        let file_config = if let Some(path) = config_path.as_ref() {
            parse_repo_config(path)?
        } else {
            RepoConfigFile::default()
        };

        let logging_root = cli.log_root.clone().or_else(|| {
            file_config
                .logging
                .as_ref()
                .and_then(|logging| logging.root.as_ref())
                .map(|path| resolve_repo_relative_path(&repo_root, path))
        });
        let logging_console = if cli.log_console {
            true
        } else {
            file_config
                .logging
                .as_ref()
                .and_then(|logging| logging.console)
                .unwrap_or(false)
        };

        Ok(Self {
            repo_root: Some(repo_root),
            config_path,
            logging_root,
            logging_console,
        })
    }

    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    pub fn require_repo_root(&self) -> Result<&Path, CliError> {
        self.repo_root.as_deref().ok_or_else(|| {
            CliError::internal("repo root required but configuration did not resolve one")
        })
    }

    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    pub fn logging_root(&self) -> Option<&PathBuf> {
        self.logging_root.as_ref()
    }

    pub const fn logging_console(&self) -> bool {
        self.logging_console
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RepoConfigFile {
    logging: Option<LoggingConfigFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LoggingConfigFile {
    root: Option<PathBuf>,
    console: Option<bool>,
}

fn discover_repo_root(start: &Path) -> Result<PathBuf, CliError> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map_or_else(|| start.to_path_buf(), Path::to_path_buf)
    };
    current = current.canonicalize().map_err(|error| {
        CliError::config(format!(
            "failed to canonicalize repo-root discovery start `{}`: {error}",
            start.display()
        ))
    })?;

    loop {
        if current.join("Cargo.toml").is_file() && current.join("boundaries").is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(CliError::config(format!(
                "could not discover the sc-lint repo root from `{}`",
                start.display()
            ))
            .with_suggested_action(
                "Run the command inside the repo or pass `--root <path>` to the workspace root.",
            ));
        }
    }
}

fn find_repo_config(repo_root: &Path) -> Option<PathBuf> {
    ["sc-lint.toml", ".just/lint-config.toml"]
        .into_iter()
        .map(|relative| repo_root.join(relative))
        .find(|path| path.exists())
}

fn parse_repo_config(path: &Path) -> Result<RepoConfigFile, CliError> {
    let text = fs::read_to_string(path).map_err(|error| {
        CliError::config(format!(
            "failed to read repo config `{}`: {error}",
            path.display()
        ))
    })?;
    toml::from_str(&text).map_err(|error| {
        CliError::config(format!(
            "failed to parse repo config `{}`: {error}",
            path.display()
        ))
        .with_detail(
            "config_path",
            serde_json::Value::String(path.display().to_string()),
        )
    })
}

fn resolve_repo_relative_path(repo_root: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        repo_root.join(candidate)
    }
}
