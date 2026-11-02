use super::{Platform, Target};
use crate::profile_host::RowContext;
use std::{collections::BTreeMap, path::Path};

/// A command is decoded and validated at the TOML boundary, so profile
/// orchestration never needs to split shell strings or infer consumer policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsumerProfileStep {
    name: String,
    command: Vec<String>,
    platform: Option<Platform>,
    target: Option<Target>,
    after_all: bool,
    env: BTreeMap<String, String>,
    cwd: Option<std::path::PathBuf>,
    context: RowContext,
}

impl ConsumerProfileStep {
    /// Builds a step from already-decoded fields; semantic checks live in
    /// [`super::validate_lint_step`].
    pub(crate) fn new(
        name: impl Into<String>,
        command: Vec<String>,
        platform: Option<Platform>,
        target: Option<Target>,
    ) -> Self {
        Self {
            name: name.into(),
            command,
            platform,
            target,
            after_all: false,
            env: BTreeMap::new(),
            cwd: None,
            context: RowContext::default(),
        }
    }

    /// Sets the decoded `after_all` flag of the row.
    pub(crate) const fn with_after_all(mut self, after_all: bool) -> Self {
        self.after_all = after_all;
        self
    }

    pub(crate) fn with_execution(
        mut self,
        env: BTreeMap<String, String>,
        cwd: Option<std::path::PathBuf>,
    ) -> Self {
        self.env = env;
        self.cwd = cwd;
        self
    }

    pub(crate) fn with_context(mut self, context: RowContext) -> Self {
        self.context = context;
        self
    }
    pub(crate) fn context(&self) -> &RowContext {
        &self.context
    }

    pub(crate) fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    pub(crate) fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub(crate) const fn after_all(&self) -> bool {
        self.after_all
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn command(&self) -> &[String] {
        &self.command
    }

    pub(crate) const fn platform(&self) -> Option<Platform> {
        self.platform
    }

    pub(crate) const fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }
}
