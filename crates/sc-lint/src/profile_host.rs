//! Host selectors and lazy lint defaults, separate from target command adaptation.
#![expect(
    clippy::result_large_err,
    reason = "Profile validation preserves structured CLI errors."
)]
use crate::CliError;
use crate::profile_plan::{
    ConsumerProfileStep, Platform, ProfilePlanDetailCode, ProfilePlanError,
    profile_plan_config_error,
};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RowContext {
    pub(crate) host_selector: Option<Platform>,
    pub(crate) env_path_keys: Option<Vec<String>>,
    pub(crate) actual_host: Option<Platform>,
}

impl RowContext {
    pub(crate) fn declared(
        name: &str,
        host: Option<&toml::Value>,
        paths: Option<&toml::Value>,
    ) -> Result<Self, ProfilePlanError> {
        Ok(Self {
            host_selector: decode_host(name, host)?,
            env_path_keys: paths.map(|value| {
                value
                    .as_table()
                    .expect("validated env_paths table")
                    .keys()
                    .cloned()
                    .collect()
            }),
            actual_host: None,
        })
    }

    pub(crate) fn on_host(mut self, host: Platform) -> Self {
        self.actual_host = self.host_selector.map(|_| host);
        self
    }

    pub(crate) fn add_to(&self, row: &mut Value) {
        if let Some(selector) = self.host_selector {
            row["host_selector"] = json!(selector.as_str());
            row["host"] = json!(
                self.actual_host
                    .expect("planned host selector has actual host")
                    .as_str()
            );
        }
        if let Some(keys) = &self.env_path_keys {
            row["env_path_keys"] = json!(keys);
        }
    }
}

pub(crate) fn decode_host(
    name: &str,
    value: Option<&toml::Value>,
) -> Result<Option<Platform>, ProfilePlanError> {
    value
        .map(|value| {
            value.as_str().and_then(Platform::parse).ok_or_else(|| {
                ProfilePlanError::new(
                    ProfilePlanDetailCode::InvalidHost,
                    Some(name),
                    Some("host"),
                    format!("lint step `{name}` host must be linux, macos or windows"),
                    "Declare host as one of the exact strings linux, macos or windows.",
                )
            })
        })
        .transpose()
}

pub(crate) fn validate_selected_host(step: &ConsumerProfileStep) -> Result<(), ProfilePlanError> {
    if step.target().is_none()
        && let (Some(host), Some(platform)) = (step.context().host_selector, step.platform())
        && host != platform
    {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::InvalidHost,
            Some(step.name()),
            Some("host"),
            "a lint step without target cannot declare different host and platform values",
            "Use matching host and platform, omit host, or declare an explicit cross target.",
        ));
    }
    Ok(())
}

pub(crate) fn lint_selector<'a>(
    explicit: Option<&'a str>,
    default: Option<&'a str>,
    names: &[&str],
    path: &Path,
) -> Result<Option<&'a str>, CliError> {
    if explicit.is_some() || default.is_none() {
        return Ok(explicit);
    }
    let name = default.expect("default is present");
    if names.contains(&name) {
        return Ok(Some(name));
    }
    Err(profile_plan_config_error(ProfilePlanError::new(ProfilePlanDetailCode::UnknownDefaultLintProfile, None, Some("default_lint_profile"),
        format!("default_lint_profile `{name}` does not name a declared lint profile"),
        "Set default_lint_profile to an available lint profile name, or select an explicit --profile."), path)
        .with_detail("available", json!(names)))
}
