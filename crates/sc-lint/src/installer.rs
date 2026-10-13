//! Product-owned release installation for consumer repositories.
//!
//! The release workflow publishes a versioned archive and `checksums.txt`.
//! This module deliberately stages both before changing the managed binary, so
//! a failed download, digest check, extraction, or post-install probe cannot
//! replace a working installation.

#![expect(
    clippy::result_large_err,
    reason = "CliError is the stable RBP-001 top-level error contract for every installer boundary."
)]

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use etcetera::{BaseStrategy, choose_base_strategy};
use semver::Version;
use serde_json::{Value, json};

use crate::CliError;
use crate::MinimumVersion;
use crate::config::{LoadedConfig, VERSION_PROBE_SCHEMA};
use crate::error::ErrorCode;

mod activation;
mod probe;

use activation::*;
use probe::*;
pub(crate) use probe::{probe_backend_version, probe_version};

const DEFAULT_RELEASE_BASE_URL: &str = "https://github.com/randlee/sc-lint/releases/download";
const RELEASE_BASE_URL_ENV: &str = "SC_LINT_RELEASE_BASE_URL";
const INSTALL_DIR_ENV: &str = "SC_LINT_INSTALL_DIR";
const BACKEND_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const BACKEND_PROBE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const BACKEND_PROBE_OUTPUT_LIMIT: usize = 64 * 1024;
#[cfg(test)]
const PUBLISH_ARTIFACTS: &str = include_str!("../../../release/publish-artifacts.toml");
const MANAGED_BACKENDS: &[&str] = &[
    crate::consts::TOOL_BOUNDARY,
    crate::consts::TOOL_PORTABILITY,
    crate::consts::TOOL_RUNTIME,
];

/// The only executable helper E.3 may place in a consumer repository.
///
/// It delegates behavior to the installed product instead of copying source
/// checkout scripts; E.3 owns rendering it into `.sc-lint/bootstrap`.
pub(crate) const CONSUMER_BOOTSTRAP_ASSET: &str = include_str!("../assets/bootstrap");
/// Windows companion for the product-owned consumer bootstrap.
pub(crate) const CONSUMER_BOOTSTRAP_POWERSHELL_ASSET: &str =
    include_str!("../assets/bootstrap.ps1");

#[derive(Debug, Clone, Copy)]
enum InstallerErrorCode {
    UnsupportedPlatform,
    ReleaseUnavailable,
    ChecksumMismatch,
    PermissionDenied,
    PostInstallVersionFailed,
    RollbackFailed,
    ActivationFailed,
}

impl ErrorCode for InstallerErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "CLI.SC_LINT_INSTALL_UNSUPPORTED_PLATFORM",
            Self::ReleaseUnavailable => "CLI.SC_LINT_RELEASE_UNAVAILABLE",
            Self::ChecksumMismatch => "CLI.SC_LINT_RELEASE_CHECKSUM_MISMATCH",
            Self::PermissionDenied => "CLI.SC_LINT_INSTALL_PERMISSION_DENIED",
            Self::PostInstallVersionFailed => "CLI.SC_LINT_POST_INSTALL_VERSION_FAILED",
            Self::RollbackFailed => "CLI.SC_LINT_INSTALL_ROLLBACK_FAILED",
            Self::ActivationFailed => "CLI.SC_LINT_INSTALL_ACTIVATION_FAILED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ReleaseTarget {
    triple: &'static str,
    archive_extension: &'static str,
}

/// The exact immutable release selected for download after applying a
/// repository's [`MinimumVersion`] floor.
///
/// The current release index resolves a floor to that floor's published
/// release. Keeping the selected artifact version distinct prevents download
/// code from accidentally treating a compatibility floor as an exact pin.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedReleaseVersion(Version);

impl ResolvedReleaseVersion {
    fn for_minimum(minimum: &MinimumVersion) -> Self {
        Self(minimum.as_semver().clone())
    }
}

impl std::fmt::Display for ResolvedReleaseVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl ReleaseTarget {
    fn detect() -> Result<Self, CliError> {
        let target = match (env::consts::OS, env::consts::ARCH) {
            ("linux", "x86_64") => Self {
                triple: "x86_64-unknown-linux-gnu",
                archive_extension: "tar.gz",
            },
            ("macos", "x86_64") => Self {
                triple: "x86_64-apple-darwin",
                archive_extension: "tar.gz",
            },
            ("macos", "aarch64") => Self {
                triple: "aarch64-apple-darwin",
                archive_extension: "tar.gz",
            },
            ("windows", "x86_64") => Self {
                triple: "x86_64-pc-windows-msvc",
                archive_extension: "zip",
            },
            (os, arch) => {
                return Err(install_error(
                    InstallerErrorCode::UnsupportedPlatform,
                    format!("sc-lint has no verified release for host platform `{os}/{arch}`"),
                    "The release workflow currently publishes Linux x86_64, macOS x86_64/aarch64, and Windows x86_64 artifacts.",
                    "Install a supported release manually, or use a supported platform and rerun `sc-lint setup`.",
                )
                .with_detail("host_os", json!(os))
                .with_detail("host_arch", json!(arch)));
            }
        };
        Ok(target)
    }

    fn archive_name(self, version: &ResolvedReleaseVersion) -> String {
        format!(
            "sc-lint_{version}_{}.{}",
            self.triple, self.archive_extension
        )
    }

    fn binary_name() -> &'static str {
        if cfg!(windows) {
            "sc-lint.exe"
        } else {
            "sc-lint"
        }
    }
}

pub(crate) fn run_setup(loaded: &LoadedConfig, dry_run: bool) -> Result<Value, CliError> {
    debug_assert!(
        !CONSUMER_BOOTSTRAP_ASSET.is_empty(),
        "the E.3 consumer bootstrap source asset must be packaged with the product"
    );
    run(loaded, false, dry_run)
}

pub(crate) fn run_upgrade(
    loaded: &LoadedConfig,
    check_only: bool,
    dry_run: bool,
) -> Result<Value, CliError> {
    run(loaded, check_only, dry_run)
}

fn run(loaded: &LoadedConfig, check_only: bool, dry_run: bool) -> Result<Value, CliError> {
    let (minimum_version, config_path) = loaded.compatibility_requirement()?;
    let release_version = ResolvedReleaseVersion::for_minimum(minimum_version);
    let target = ReleaseTarget::detect()?;
    let install_dir = managed_install_dir()?;
    let managed_binary = install_dir.join(ReleaseTarget::binary_name());
    let use_path_candidates = env::var_os(INSTALL_DIR_ENV).is_none();
    let had_previous_installation = std::iter::once(managed_binary.clone())
        .chain(path_binary_candidates_if(use_path_candidates))
        .any(|binary| binary.is_file());
    let managed_existing =
        find_compatible_binary_with(&managed_binary, minimum_version, false, |binary| {
            let version = probe_version(binary)?;
            managed_payload_complete(binary)
                .then_some(version)
                .ok_or_else(|| "managed release bundle is incomplete".to_string())
        });
    let existing = managed_existing.or_else(|| {
        // PATH is a supported development fallback when no managed launcher
        // exists. A partial managed install must be repaired from the release,
        // even if an unrelated PATH executable happens to be compatible.
        (!managed_binary.exists() && use_path_candidates)
            .then(|| {
                find_compatible_binary_with(&managed_binary, minimum_version, true, probe_version)
            })
            .flatten()
    });

    if let Some((binary, version)) = existing {
        return Ok(json!({
            "status": "current",
            "action": "found",
            "minimum_version": minimum_version.to_string(),
            "installed_version": version.to_string(),
            "binary_path": binary.display().to_string(),
            "install_dir": install_dir.display().to_string(),
            "config_path": config_path.display().to_string(),
            "summary": "a compatible sc-lint installation is already available",
        }));
    }

    let archive_name = target.archive_name(&release_version);
    let archive_url = release_url(&release_version, &archive_name);
    let checksums_url = release_url(&release_version, "checksums.txt");
    if check_only || dry_run {
        return Ok(json!({
            "status": if check_only { "update_required" } else { "dry_run" },
            "action": if check_only { "check" } else { "install" },
            "minimum_version": minimum_version.to_string(),
            "selected_release_version": release_version.to_string(),
            "archive": archive_name,
            "release_url": archive_url,
            "checksums_url": checksums_url,
            "install_dir": install_dir.display().to_string(),
            "config_path": config_path.display().to_string(),
            "summary": "a verified sc-lint release would be installed",
        }));
    }

    let installed_version = install_verified_release(
        &release_version,
        minimum_version,
        target,
        &install_dir,
        &managed_binary,
        &archive_url,
        &checksums_url,
    )?;
    Ok(json!({
        "status": if had_previous_installation { "upgraded" } else { "installed" },
        "action": if had_previous_installation { "upgraded" } else { "installed" },
        "minimum_version": minimum_version.to_string(),
        "selected_release_version": release_version.to_string(),
        "installed_version": installed_version.to_string(),
        "binary_path": managed_binary.display().to_string(),
        "install_dir": install_dir.display().to_string(),
        "config_path": config_path.display().to_string(),
        "summary": if had_previous_installation {
            "verified sc-lint release upgraded"
        } else {
            "verified sc-lint release activated"
        },
    }))
}

fn managed_install_dir() -> Result<PathBuf, CliError> {
    if let Some(path) = env::var_os(INSTALL_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }
    // The platform strategy resolves XDG data directories on Unix and
    // LocalAppData on Windows without coupling this product boundary to
    // platform-specific environment keys.
    if let Ok(strategy) = choose_base_strategy() {
        return Ok(strategy.data_dir().join("sc-lint").join("bin"));
    }
    etcetera::home_dir()
        .ok()
        .map(|home| {
            home.join(".local")
                .join("share")
                .join("sc-lint")
                .join("bin")
        })
        .ok_or_else(|| {
            install_error(
                InstallerErrorCode::PermissionDenied,
                "could not determine a managed sc-lint install directory",
                "Neither SC_LINT_INSTALL_DIR nor a platform data or home directory was available.",
                "Set SC_LINT_INSTALL_DIR to a writable directory and rerun `sc-lint setup`.",
            )
            .with_detail("install_directory_source", json!("unavailable"))
        })
}

fn release_url(version: &ResolvedReleaseVersion, filename: &str) -> String {
    let base =
        env::var(RELEASE_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_RELEASE_BASE_URL.to_string());
    format!(
        "{}/{}/{}",
        base.trim_end_matches('/'),
        format_args!("v{version}"),
        filename
    )
}

fn find_compatible_binary_with<F>(
    managed_binary: &Path,
    minimum: &MinimumVersion,
    use_path_candidates: bool,
    probe: F,
) -> Option<(PathBuf, Version)>
where
    F: Fn(&Path) -> Result<Version, String>,
{
    std::iter::once(managed_binary.to_path_buf())
        .chain(path_binary_candidates_if(use_path_candidates))
        .find_map(|binary| {
            let version = probe(&binary).ok()?;
            (version >= *minimum.as_semver()).then_some((binary, version))
        })
}

fn path_binary_candidates_if(use_path_candidates: bool) -> impl Iterator<Item = PathBuf> {
    use_path_candidates
        .then(path_binary_candidates)
        .into_iter()
        .flatten()
}

fn path_binary_candidates() -> impl Iterator<Item = PathBuf> {
    let binary_name = ReleaseTarget::binary_name();
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths)
                .map(|directory| directory.join(binary_name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
}

fn install_verified_release(
    release_version: &ResolvedReleaseVersion,
    minimum_version: &MinimumVersion,
    target: ReleaseTarget,
    install_dir: &Path,
    managed_binary: &Path,
    archive_url: &str,
    checksums_url: &str,
) -> Result<Version, CliError> {
    fs::create_dir_all(install_dir).map_err(|error| permission_error(install_dir, error))?;
    let staging = install_dir.join(format!(
        ".sc-lint-stage-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir(&staging).map_err(|error| permission_error(&staging, error))?;
    let result = install_into_staging(
        release_version,
        minimum_version,
        target,
        managed_binary,
        archive_url,
        checksums_url,
        &staging,
    );
    let _ = fs::remove_dir_all(&staging);
    result
}

fn install_into_staging(
    release_version: &ResolvedReleaseVersion,
    minimum_version: &MinimumVersion,
    target: ReleaseTarget,
    managed_binary: &Path,
    archive_url: &str,
    checksums_url: &str,
    staging: &Path,
) -> Result<Version, CliError> {
    let archive_name = target.archive_name(release_version);
    let checksum_file = staging.join("checksums.txt");
    let archive = staging.join(&archive_name);
    download(checksums_url, &checksum_file)?;
    download(archive_url, &archive)?;
    verify_checksum(&archive, &checksum_file, &archive_name)?;

    let extract_dir = staging.join("extracted");
    fs::create_dir(&extract_dir).map_err(|error| permission_error(&extract_dir, error))?;
    extract_archive(&archive, &extract_dir)?;
    let candidate = release_binary_candidate(&extract_dir, release_version, target);
    let payload = release_payload_candidates(&candidate, managed_binary);
    if let Some(problem) = payload_validation_issue(&payload) {
        return Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("release archive `{archive_name}` failed payload validation: {problem}"),
            "The release artifact must contain the launcher, healthy manifest-declared backend binaries, and the documentation bundle.",
            "Download a complete supported release again or report the malformed release artifact.",
        )
        .with_detail("archive_name", json!(archive_name))
        .with_detail("payload_validation_problem", json!(problem))
        .with_detail("expected_binaries", json!([ReleaseTarget::binary_name(), MANAGED_BACKENDS[0], MANAGED_BACKENDS[1], MANAGED_BACKENDS[2]]))
        .with_detail(
            "extract_directory",
            json!(extract_dir.display().to_string()),
        ));
    }
    activate_payload(&payload, managed_binary, minimum_version)
}

#[derive(Debug, Clone)]
struct PayloadEntry {
    source: PathBuf,
    target: PathBuf,
}

type PayloadChange = (PathBuf, PathBuf, bool, bool);
type ProbeOutputReader = thread::JoinHandle<io::Result<Vec<u8>>>;

fn release_payload_candidates(binary: &Path, managed_binary: &Path) -> Vec<PayloadEntry> {
    let bin_dir = binary.parent().unwrap_or_else(|| Path::new("."));
    let managed_bin_dir = managed_binary.parent().unwrap_or_else(|| Path::new("."));
    let mut entries = vec![PayloadEntry {
        source: binary.to_path_buf(),
        target: managed_binary.to_path_buf(),
    }];
    for backend in MANAGED_BACKENDS {
        let name = if cfg!(windows) {
            format!("{backend}.exe")
        } else {
            (*backend).to_string()
        };
        entries.push(PayloadEntry {
            source: bin_dir.join(&name),
            target: managed_bin_dir.join(name),
        });
    }
    let prefix = if bin_dir.file_name().is_some_and(|name| name == "bin") {
        bin_dir.parent().unwrap_or(bin_dir)
    } else {
        bin_dir
    };
    let docs_candidates = [
        bin_dir.join("docs-bundle"),
        bin_dir.join(crate::consts::DOCS_BUNDLE_DIR),
        prefix.join("share/sc-lint/sc-lint-docs"),
        prefix.join("share/sc-lint/docs-bundle"),
    ];
    let docs_source = docs_candidates
        .iter()
        .find(|path| docs_bundle_complete(path))
        .cloned()
        .unwrap_or_else(|| docs_candidates[0].clone());
    let managed_prefix = managed_bin_dir.parent().unwrap_or(managed_bin_dir);
    entries.push(PayloadEntry {
        source: docs_source,
        target: managed_prefix.join("share/sc-lint/sc-lint-docs"),
    });
    entries
}

fn docs_bundle_complete(path: &Path) -> bool {
    let Ok(manifest) = fs::read_to_string(path.join("manifest.toml")) else {
        return false;
    };
    let Ok(manifest) = toml::from_str::<toml::Value>(&manifest) else {
        return false;
    };
    if manifest
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        != Some(1)
    {
        return false;
    }
    let Some(guides) = manifest.get("guides").and_then(toml::Value::as_array) else {
        return false;
    };
    let mut has_overview = false;
    for guide in guides {
        let Some(relative) = guide.get("path").and_then(toml::Value::as_str) else {
            return false;
        };
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
            || !path.join(relative).is_file()
        {
            return false;
        }
        has_overview |= relative == Path::new("README.md");
    }
    has_overview
}

fn payload_validation_issue(entries: &[PayloadEntry]) -> Option<String> {
    payload_validation_issue_with_timeout(entries, BACKEND_PROBE_TIMEOUT)
}

fn payload_validation_issue_with_timeout(
    entries: &[PayloadEntry],
    probe_timeout: Duration,
) -> Option<String> {
    for entry in entries {
        if entry.source.is_dir() {
            if !docs_bundle_complete(&entry.source) {
                return Some(format!(
                    "documentation bundle at `{}` is incomplete",
                    entry.source.display()
                ));
            }
        } else if let Some(backend) = entry
            .source
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| {
                MANAGED_BACKENDS.iter().find(|backend| {
                    name == if cfg!(windows) {
                        format!("{backend}.exe")
                    } else {
                        (*backend).to_string()
                    }
                })
            })
        {
            if let Some(problem) = backend_cli_failure(&entry.source, backend, probe_timeout) {
                return Some(problem);
            }
        } else if !entry.source.is_file() {
            return Some(format!(
                "required release file `{}` is missing",
                entry.source.display()
            ));
        }
    }
    None
}

fn managed_payload_complete(binary: &Path) -> bool {
    managed_payload_validation_issue(binary).is_none()
}

fn managed_payload_validation_issue(binary: &Path) -> Option<String> {
    managed_payload_validation_issue_with_timeout(binary, BACKEND_PROBE_TIMEOUT)
}

fn managed_payload_validation_issue_with_timeout(
    binary: &Path,
    probe_timeout: Duration,
) -> Option<String> {
    let parent = binary.parent().unwrap_or_else(|| Path::new("."));
    for backend in MANAGED_BACKENDS {
        let name = if cfg!(windows) {
            format!("{backend}.exe")
        } else {
            (*backend).to_string()
        };
        let path = parent.join(name);
        if let Some(problem) = backend_cli_failure(&path, backend, probe_timeout) {
            return Some(problem);
        }
    }
    let prefix = parent.parent().unwrap_or(parent);
    if docs_bundle_complete(&parent.join("docs-bundle"))
        || docs_bundle_complete(&parent.join(crate::consts::DOCS_BUNDLE_DIR))
        || docs_bundle_complete(&prefix.join("share/sc-lint/sc-lint-docs"))
        || docs_bundle_complete(&prefix.join("share/sc-lint/docs-bundle"))
    {
        None
    } else {
        Some(format!(
            "documentation bundle is missing or incomplete beside `{}`",
            binary.display()
        ))
    }
}

fn release_binary_candidate(
    extract_dir: &Path,
    release_version: &ResolvedReleaseVersion,
    target: ReleaseTarget,
) -> PathBuf {
    let flat = extract_dir.join(ReleaseTarget::binary_name());
    if flat.is_file() {
        return flat;
    }
    extract_dir
        .join(format!("sc-lint_{release_version}_{}", target.triple))
        .join("bin")
        .join(ReleaseTarget::binary_name())
}

fn download(url: &str, destination: &Path) -> Result<(), CliError> {
    let output = Command::new("curl")
        .args(["--fail", "--location", "--retry", "3", "--output"])
        .arg(destination)
        .arg(url)
        .output()
        .map_err(|error| {
            install_error(
                InstallerErrorCode::ReleaseUnavailable,
                format!("could not start curl while retrieving `{url}`"),
                error.to_string(),
                "Install curl or provide a reachable verified sc-lint release source, then rerun `sc-lint setup`.",
            )
            .with_detail("release_url", json!(url))
            .with_detail("destination", json!(destination.display().to_string()))
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("could not retrieve verified sc-lint release data from `{url}`"),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Check network access or the release version, then rerun `sc-lint setup`.",
        )
        .with_detail("release_url", json!(url))
        .with_detail("destination", json!(destination.display().to_string())))
    }
}

fn verify_checksum(
    archive: &Path,
    checksum_file: &Path,
    archive_name: &str,
) -> Result<(), CliError> {
    let checksums = fs::read_to_string(checksum_file).map_err(|error| {
        install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!(
                "could not read downloaded checksum manifest `{}`",
                checksum_file.display()
            ),
            error.to_string(),
            "Download the release checksum manifest again and rerun `sc-lint setup`.",
        )
        .with_detail(
            "checksum_manifest",
            json!(checksum_file.display().to_string()),
        )
        .with_detail("archive_name", json!(archive_name))
    })?;
    let expected = checksums.lines().find_map(|line| {
        let (digest, filename) = line.split_once(char::is_whitespace)?;
        (filename.trim_start_matches('*').trim() == archive_name).then_some(digest)
    });
    let Some(expected) = expected else {
        return Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("checksum manifest did not contain `{archive_name}`"),
            "The release checksum manifest is incomplete for the selected platform artifact.",
            "Select a supported release or report the incomplete checksum manifest.",
        )
        .with_detail(
            "checksum_manifest",
            json!(checksum_file.display().to_string()),
        )
        .with_detail("archive_name", json!(archive_name)));
    };
    let actual = sha256(archive)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(install_error(
            InstallerErrorCode::ChecksumMismatch,
            format!("SHA-256 verification failed for `{archive_name}`"),
            format!("expected {expected}, observed {actual}"),
            "Do not activate this artifact. Retry the download from an official release and rerun `sc-lint setup`.",
        )
        .with_detail("archive", json!(archive_name))
        .with_detail("expected_sha256", json!(expected))
        .with_detail("actual_sha256", json!(actual)))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ChecksumCommand {
    program: &'static str,
    prefix_args: &'static [&'static str],
    suffix_args: &'static [&'static str],
}

fn sha256_command() -> ChecksumCommand {
    if cfg!(target_os = "macos") {
        ChecksumCommand {
            program: "shasum",
            prefix_args: &["-a", "256"],
            suffix_args: &[],
        }
    } else if cfg!(windows) {
        ChecksumCommand {
            program: "certutil",
            prefix_args: &["-hashfile"],
            suffix_args: &["SHA256"],
        }
    } else {
        ChecksumCommand {
            program: "sha256sum",
            prefix_args: &[],
            suffix_args: &[],
        }
    }
}

fn sha256(path: &Path) -> Result<String, CliError> {
    let command_spec = sha256_command();
    let mut command = Command::new(command_spec.program);
    command
        .args(command_spec.prefix_args)
        .arg(path)
        .args(command_spec.suffix_args);
    let output = command.output().map_err(|error| {
        install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!(
                "could not start `{}` to verify the release checksum",
                command_spec.program
            ),
            error.to_string(),
            "Install a SHA-256 utility and rerun `sc-lint setup`.",
        )
        .with_detail("checksum_utility", json!(command_spec.program))
        .with_detail("archive_path", json!(path.display().to_string()))
    })?;
    if !output.status.success() {
        return Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!(
                "`{}` failed while verifying `{}`",
                command_spec.program,
                path.display()
            ),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Repair the local SHA-256 utility and rerun `sc-lint setup`.",
        )
        .with_detail("checksum_utility", json!(command_spec.program))
        .with_detail("archive_path", json!(path.display().to_string())));
    }
    let output = String::from_utf8_lossy(&output.stdout);
    output
        .split_whitespace()
        .find(|word| {
            word.len() == 64 && word.chars().all(|character| character.is_ascii_hexdigit())
        })
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            install_error(
                InstallerErrorCode::ReleaseUnavailable,
                format!(
                    "`{}` did not return a SHA-256 digest for `{}`",
                    command_spec.program,
                    path.display(),
                ),
                output.trim().to_string(),
                "Repair the local SHA-256 utility and rerun `sc-lint setup`.",
            )
            .with_detail("checksum_utility", json!(command_spec.program))
            .with_detail("archive_path", json!(path.display().to_string()))
        })
}

fn extract_archive(archive: &Path, destination: &Path) -> Result<(), CliError> {
    let output =
        Command::new("tar")
            .args(["-xf"])
            .arg(archive)
            .args(["-C"])
            .arg(destination)
            .output()
            .map_err(|error| {
                install_error(
                InstallerErrorCode::ReleaseUnavailable,
                format!("could not start tar to extract `{}`", archive.display()),
                error.to_string(),
                "Install tar or download the release archive manually, then rerun `sc-lint setup`.",
            )
            .with_detail("archive_path", json!(archive.display().to_string()))
            .with_detail("extract_directory", json!(destination.display().to_string()))
            })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("could not extract release archive `{}`", archive.display()),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Download a complete release archive and rerun `sc-lint setup`.",
        )
        .with_detail("archive_path", json!(archive.display().to_string()))
        .with_detail(
            "extract_directory",
            json!(destination.display().to_string()),
        ))
    }
}

fn reject_running_managed_binary(target: &Path) -> Result<(), CliError> {
    #[cfg(not(windows))]
    let _ = target;
    #[cfg(windows)]
    {
        let current_exe = std::env::current_exe()
            .ok()
            .and_then(|path| fs::canonicalize(path).ok());
        let target = fs::canonicalize(target).ok();
        if current_exe.as_ref() == target.as_ref() {
            return Err(install_error(
                InstallerErrorCode::ActivationFailed,
                "cannot replace the running managed sc-lint executable on Windows",
                "Windows may retain an executable handle while the process is running.",
                "Run `sc-lint setup` from a separately downloaded sc-lint executable, then retry the managed upgrade.",
            )
            .with_detail("managed_binary", json!(target.map(|path| path.display().to_string())))
            .with_detail("platform", json!("windows")));
        }
    }
    Ok(())
}

fn permission_error(path: &Path, error: io::Error) -> CliError {
    let code = if error.kind() == io::ErrorKind::PermissionDenied {
        InstallerErrorCode::PermissionDenied
    } else {
        InstallerErrorCode::ActivationFailed
    };
    install_error(
        code,
        format!("could not write managed sc-lint path `{}`", path.display()),
        error.to_string(),
        "Choose a writable SC_LINT_INSTALL_DIR and rerun `sc-lint setup`.",
    )
    .with_detail("managed_path", json!(path.display().to_string()))
    .with_detail("io_error_kind", json!(format!("{:?}", error.kind())))
}

fn activation_error(path: &Path, error: io::Error) -> CliError {
    let code = if error.kind() == io::ErrorKind::PermissionDenied {
        InstallerErrorCode::PermissionDenied
    } else {
        InstallerErrorCode::ActivationFailed
    };
    install_error(
        code,
        format!(
            "could not atomically activate managed sc-lint binary `{}`",
            path.display()
        ),
        error.to_string(),
        "The previous managed installation was retained. Choose a writable directory and rerun `sc-lint setup`.",
    )
    .with_detail("managed_binary", json!(path.display().to_string()))
    .with_detail("io_error_kind", json!(format!("{:?}", error.kind())))
}

fn install_error(
    code: InstallerErrorCode,
    message: impl Into<String>,
    cause: impl Into<String>,
    suggested_action: impl Into<String>,
) -> CliError {
    CliError::backend_failure(message)
        .with_code(code.as_str())
        .with_cause(cause)
        .with_suggested_action(suggested_action)
        .with_documentation("sc-lint docs installation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serial_test::serial;
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::io::Write;
    use std::str::FromStr;

    use tempfile::TempDir;

    const MINIMAL_DOCS_MANIFEST: &str = "schema_version = 1\n\n[[guides]]\npath = \"README.md\"\n";
    #[cfg(unix)]
    const BACKEND_PROBE_TEST_WATCHDOG: Duration = Duration::from_secs(30);

    #[test]
    fn release_target_matches_the_published_release_matrix() {
        let target = ReleaseTarget::detect().expect("current test host is supported");
        assert!(
            [
                "x86_64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
                "x86_64-pc-windows-msvc",
            ]
            .contains(&target.triple)
        );
    }

    #[test]
    fn managed_payload_contract_matches_the_publish_artifact_manifest() {
        let manifest: toml::Value = toml::from_str(PUBLISH_ARTIFACTS).expect("publish manifest");
        let binaries = manifest
            .get("release_binaries")
            .and_then(toml::Value::as_array)
            .expect("release binaries");
        let names = binaries
            .iter()
            .filter_map(|binary| binary.get("name").and_then(toml::Value::as_str))
            .collect::<Vec<_>>();
        assert!(names.contains(&"sc-lint"));
        for backend in MANAGED_BACKENDS {
            assert!(
                names.contains(backend),
                "missing {backend} in publish manifest"
            );
        }
        let launcher = binaries
            .iter()
            .find(|binary| binary.get("name").and_then(toml::Value::as_str) == Some("sc-lint"))
            .expect("launcher manifest entry");
        let bundled_paths = launcher
            .get("bundled_paths")
            .and_then(toml::Value::as_array)
            .expect("launcher bundled paths");
        assert!(bundled_paths.iter().any(|bundle| {
            bundle.get("source").and_then(toml::Value::as_str) == Some("docs-bundle")
                && bundle.get("destination").and_then(toml::Value::as_str)
                    == Some("share/sc-lint/sc-lint-docs")
        }));
    }

    #[test]
    fn checksum_mismatch_is_structured_and_does_not_require_activation() {
        let fixture = TempDir::new().expect("fixture");
        let archive = fixture.path().join("artifact.tar.gz");
        fs::write(&archive, "untrusted artifact").expect("archive");
        let checksums = fixture.path().join("checksums.txt");
        fs::write(
            &checksums,
            "0000000000000000000000000000000000000000000000000000000000000000 artifact.tar.gz\n",
        )
        .expect("checksums");
        let error = verify_checksum(&archive, &checksums, "artifact.tar.gz").expect_err("mismatch");
        assert_eq!(error.code(), "CLI.SC_LINT_RELEASE_CHECKSUM_MISMATCH");
        assert!(error.cause.is_some());
        assert!(error.suggested_action.is_some());
    }

    #[test]
    fn checksum_command_selects_sha256_on_every_supported_platform() {
        let command = sha256_command();
        #[cfg(windows)]
        assert_eq!(
            command,
            ChecksumCommand {
                program: "certutil",
                prefix_args: &["-hashfile"],
                suffix_args: &["SHA256"],
            }
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            command,
            ChecksumCommand {
                program: "shasum",
                prefix_args: &["-a", "256"],
                suffix_args: &[],
            }
        );
        #[cfg(all(not(windows), not(target_os = "macos")))]
        assert_eq!(
            command,
            ChecksumCommand {
                program: "sha256sum",
                prefix_args: &[],
                suffix_args: &[],
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_binary_candidate_accepts_nested_and_prefers_legacy_flat_layouts() {
        let fixture = TempDir::new().expect("fixture");
        let minimum = MinimumVersion::from_str("0.7.0").expect("minimum");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let target = ReleaseTarget::detect().expect("current test host is supported");
        let nested = fixture
            .path()
            .join(format!("sc-lint_{release_version}_{}", target.triple))
            .join("bin")
            .join(ReleaseTarget::binary_name());
        fs::create_dir_all(nested.parent().expect("nested parent")).expect("nested directory");
        fs::write(&nested, "nested release launcher").expect("nested launcher");

        assert_eq!(
            release_binary_candidate(fixture.path(), &release_version, target),
            nested
        );

        let flat = fixture.path().join(ReleaseTarget::binary_name());
        fs::write(&flat, "legacy flat release launcher").expect("flat launcher");
        assert_eq!(
            release_binary_candidate(fixture.path(), &release_version, target),
            flat
        );
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&fixture.path().join(backend), backend);
        }
        let legacy_docs = fixture.path().join("docs-bundle");
        fs::create_dir_all(&legacy_docs).expect("legacy docs");
        fs::write(legacy_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("legacy docs manifest");
        fs::write(legacy_docs.join("README.md"), "# legacy docs\n").expect("legacy README");
        let flat_payload =
            release_payload_candidates(&flat, &fixture.path().join("managed/bin/sc-lint"));
        assert!(
            payload_validation_issue(&flat_payload).is_none(),
            "legacy flat layout is accepted"
        );
    }

    #[test]
    fn consumer_bootstrap_exposes_only_product_owned_operations() {
        for operation in ["setup", "lint", "test", "upgrade"] {
            assert!(
                CONSUMER_BOOTSTRAP_ASSET.contains(operation),
                "bootstrap does not expose {operation}"
            );
        }
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains(r#"[ "$operation" = exec ]"#));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains(".sc-lint/bootstrap exec"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("compatibility check"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("--config"));
        assert!(!CONSUMER_BOOTSTRAP_ASSET.contains("cargo run"));
        for operation in ["setup", "lint", "test", "upgrade"] {
            assert!(
                CONSUMER_BOOTSTRAP_POWERSHELL_ASSET.contains(operation),
                "Windows bootstrap does not expose {operation}"
            );
        }
        assert!(CONSUMER_BOOTSTRAP_POWERSHELL_ASSET.contains(r#"$Operation -eq "exec""#));
        assert!(CONSUMER_BOOTSTRAP_POWERSHELL_ASSET.contains(".sc-lint/bootstrap.ps1 exec"));
        assert!(CONSUMER_BOOTSTRAP_POWERSHELL_ASSET.contains("CLI.SC_LINT_RELEASE_UNAVAILABLE"));
        assert!(!CONSUMER_BOOTSTRAP_POWERSHELL_ASSET.contains("cargo run"));
    }

    #[cfg(unix)]
    #[test]
    fn local_release_fixture_installs_only_after_checksum_verification() {
        let fixture = TempDir::new().expect("fixture");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let release_target = ReleaseTarget::detect().expect("supported fixture host");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let archive_name = release_target.archive_name(&release_version);
        let release_dir = fixture.path().join("release").join("v0.4.1");
        let payload_dir = fixture.path().join("payload");
        let install_dir = fixture.path().join("managed");
        fs::create_dir_all(&release_dir).expect("release directory");
        let archive_root = payload_dir.join(format!(
            "sc-lint_{release_version}_{}",
            release_target.triple
        ));
        fs::create_dir_all(archive_root.join("bin")).expect("payload directory");
        fs::create_dir(&install_dir).expect("managed directory");
        write_probe(
            &archive_root.join("bin").join(ReleaseTarget::binary_name()),
            "0.4.1",
        );
        add_fake_release_companions(&archive_root);
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(".")
            .status()
            .expect("tar starts");
        assert!(tar_status.success(), "tar produced fixture archive");
        let digest = sha256(&archive).expect("fixture checksum");
        fs::write(
            release_dir.join("checksums.txt"),
            format!("{digest} {archive_name}\n"),
        )
        .expect("checksums");

        let target = install_dir.join(ReleaseTarget::binary_name());
        install_verified_release(
            &release_version,
            &minimum,
            release_target,
            &install_dir,
            &target,
            &format!("file://{}", archive.display()),
            &format!("file://{}", release_dir.join("checksums.txt").display()),
        )
        .expect("verified fixture installs");
        assert_eq!(
            probe_version(&target).expect("installed probe"),
            Version::parse("0.4.1").expect("version")
        );
    }

    #[cfg(unix)]
    #[test]
    fn checksum_valid_release_with_nonfunctional_backend_preserves_previous_bundle() {
        let fixture = TempDir::new().expect("fixture");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let target = ReleaseTarget::detect().expect("supported fixture host");
        let archive_name = target.archive_name(&release_version);
        let release_dir = fixture.path().join("release").join("v0.4.1");
        let payload_dir = fixture.path().join("payload");
        let prefix = format!("sc-lint_{release_version}_{}", target.triple);
        let archive_root = payload_dir.join(&prefix);
        let bin_dir = archive_root.join("bin");
        fs::create_dir_all(&bin_dir).expect("payload bin");
        fs::create_dir_all(&release_dir).expect("release directory");
        write_probe(&bin_dir.join(ReleaseTarget::binary_name()), "0.4.1");
        add_fake_release_companions(&archive_root);
        write_failing_stub(&bin_dir.join(crate::consts::TOOL_RUNTIME));
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(".")
            .status()
            .expect("tar starts");
        assert!(
            tar_status.success(),
            "tar produced malformed fixture archive"
        );
        let digest = sha256(&archive).expect("archive checksum");
        fs::write(
            release_dir.join("checksums.txt"),
            format!("{digest} {archive_name}\n"),
        )
        .expect("checksum manifest");

        let install_dir = fixture.path().join("managed/bin");
        let managed_binary = install_dir.join(ReleaseTarget::binary_name());
        let managed_docs = fixture.path().join("managed/share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&install_dir).expect("managed bin");
        fs::create_dir_all(&managed_docs).expect("managed docs");
        write_probe(&managed_binary, "0.4.1");
        let previous_launcher = fs::read(&managed_binary).expect("previous launcher");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&install_dir.join(backend), backend);
        }
        fs::write(managed_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("managed docs manifest");
        fs::write(managed_docs.join("README.md"), "previous complete bundle\n")
            .expect("managed docs README");
        let previous_backends = MANAGED_BACKENDS
            .iter()
            .map(|backend| fs::read(install_dir.join(backend)).expect("previous backend"))
            .collect::<Vec<_>>();

        let error = install_verified_release(
            &release_version,
            &minimum,
            target,
            &install_dir,
            &managed_binary,
            &format!("file://{}", archive.display()),
            &format!("file://{}", release_dir.join("checksums.txt").display()),
        )
        .expect_err("nonfunctional backend is rejected");
        assert_eq!(error.code(), "CLI.SC_LINT_RELEASE_UNAVAILABLE");
        assert_eq!(
            fs::read(&managed_binary).expect("previous launcher remains"),
            previous_launcher
        );
        assert_eq!(
            probe_version(&managed_binary).expect("previous launcher remains"),
            Version::parse("0.4.1").expect("version")
        );
        for (backend, previous) in MANAGED_BACKENDS.iter().zip(previous_backends) {
            let path = install_dir.join(backend);
            assert_eq!(fs::read(&path).expect("previous backend remains"), previous);
            assert!(backend_cli_failure(&path, backend, BACKEND_PROBE_TIMEOUT).is_none());
        }
        assert_eq!(
            fs::read_to_string(managed_docs.join("README.md")).expect("previous docs remain"),
            "previous complete bundle\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn failed_post_install_probe_restores_the_previous_binary() {
        let fixture = TempDir::new().expect("fixture");
        let target = fixture.path().join("sc-lint");
        let candidate = fixture.path().join("candidate");
        write_probe(&target, "0.4.1");
        write_probe(&candidate, "0.3.0");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");

        let error = activate_payload(
            &[PayloadEntry {
                source: candidate,
                target: target.clone(),
            }],
            &target,
            &minimum,
        )
        .expect_err("old candidate rolls back");
        assert_eq!(error.code(), "CLI.SC_LINT_POST_INSTALL_VERSION_FAILED");
        assert_eq!(
            probe_version(&target).expect("old binary restored"),
            Version::parse("0.4.1").expect("version")
        );
    }

    #[cfg(unix)]
    #[test]
    fn failed_bundle_activation_restores_every_previous_managed_payload() {
        let fixture = TempDir::new().expect("fixture");
        let install = fixture.path().join("managed/bin");
        let old_docs = fixture.path().join("managed/share/sc-lint/sc-lint-docs");
        let staged = fixture.path().join("candidate");
        let staged_bin = staged.join("bin");
        let staged_docs = staged.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&install).expect("install directory");
        fs::create_dir_all(&old_docs).expect("old docs directory");
        fs::create_dir_all(&staged_bin).expect("candidate bin directory");
        fs::create_dir_all(&staged_docs).expect("candidate docs directory");
        write_probe(&install.join(ReleaseTarget::binary_name()), "0.4.1");
        write_probe(&staged_bin.join(ReleaseTarget::binary_name()), "0.3.0");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&install.join(backend), backend);
            write_backend_probe(&staged_bin.join(backend), backend);
        }
        let previous_backends = MANAGED_BACKENDS
            .iter()
            .map(|backend| fs::read(install.join(backend)).expect("old backend"))
            .collect::<Vec<_>>();
        fs::write(old_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("old manifest");
        fs::write(old_docs.join("README.md"), "old docs\n").expect("old README");
        fs::write(staged_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("new manifest");
        fs::write(staged_docs.join("README.md"), "new docs\n").expect("new README");

        let managed_binary = install.join(ReleaseTarget::binary_name());
        let mut entries = vec![PayloadEntry {
            source: staged_bin.join(ReleaseTarget::binary_name()),
            target: managed_binary.clone(),
        }];
        entries.extend(MANAGED_BACKENDS.iter().map(|backend| PayloadEntry {
            source: staged_bin.join(backend),
            target: install.join(backend),
        }));
        entries.push(PayloadEntry {
            source: staged_docs,
            target: old_docs.clone(),
        });

        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let error = activate_payload(&entries, &managed_binary, &minimum)
            .expect_err("candidate below floor must roll back entire bundle");
        assert_eq!(error.code(), "CLI.SC_LINT_POST_INSTALL_VERSION_FAILED");
        assert_eq!(
            probe_version(&managed_binary).expect("restored launcher"),
            Version::parse("0.4.1").unwrap()
        );
        for (backend, previous) in MANAGED_BACKENDS.iter().zip(previous_backends) {
            assert_eq!(
                fs::read(install.join(backend)).expect("old backend restored"),
                previous
            );
            assert!(
                backend_cli_failure(&install.join(backend), backend, BACKEND_PROBE_TIMEOUT)
                    .is_none()
            );
        }
        assert_eq!(
            fs::read_to_string(old_docs.join("README.md")).expect("old docs"),
            "old docs\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_payload_validation_rejects_missing_backend_and_corrupt_docs() {
        let fixture = TempDir::new().expect("fixture");
        let version = ResolvedReleaseVersion(Version::new(0, 7, 1));
        let target = ReleaseTarget::detect().expect("supported target");
        let archive_root = fixture
            .path()
            .join(format!("sc-lint_{version}_{}", target.triple));
        let bin = archive_root.join("bin");
        let managed = fixture.path().join("managed/bin/sc-lint");
        fs::create_dir_all(&bin).expect("archive bin");
        fs::write(bin.join(ReleaseTarget::binary_name()), "launcher").expect("launcher");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&bin.join(backend), backend);
        }
        fs::remove_file(bin.join(crate::consts::TOOL_RUNTIME)).expect("omit runtime");
        let docs = archive_root.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&docs).expect("docs directory");
        fs::write(docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("manifest");
        let mut entries =
            release_payload_candidates(&bin.join(ReleaseTarget::binary_name()), &managed);
        assert!(
            payload_validation_issue(&entries).is_some(),
            "missing backend is rejected"
        );
        write_backend_probe(
            &bin.join(crate::consts::TOOL_RUNTIME),
            crate::consts::TOOL_RUNTIME,
        );
        assert!(
            payload_validation_issue(&entries).is_some(),
            "docs without overview are rejected"
        );
        fs::write(docs.join("README.md"), "# docs\n").expect("overview");
        entries = release_payload_candidates(&bin.join(ReleaseTarget::binary_name()), &managed);
        assert!(
            payload_validation_issue(&entries).is_none(),
            "complete release payload is accepted"
        );
    }

    #[cfg(unix)]
    #[test]
    fn swapped_real_backend_is_rejected_from_staged_and_installed_bundles() {
        build_release_binaries();
        let fixture = TempDir::new().expect("fixture");
        let staged_root = fixture.path().join("staged");
        let staged_bin = staged_root.join("bin");
        let staged_docs = staged_root.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&staged_bin).expect("staged bin");
        fs::create_dir_all(&staged_docs).expect("staged docs");
        fs::write(staged_bin.join(ReleaseTarget::binary_name()), "launcher")
            .expect("staged launcher");
        for backend in MANAGED_BACKENDS {
            fs::copy(built_backend_binary(backend), staged_bin.join(backend))
                .expect("copy staged real backend");
        }
        fs::copy(
            built_backend_binary(crate::consts::TOOL_RUNTIME),
            staged_bin.join(crate::consts::TOOL_PORTABILITY),
        )
        .expect("swap staged portability backend");
        fs::write(staged_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("staged docs manifest");
        fs::write(staged_docs.join("README.md"), "# docs\n").expect("staged docs README");
        let staged_entries = release_payload_candidates(
            &staged_bin.join(ReleaseTarget::binary_name()),
            &fixture.path().join("managed/bin/sc-lint"),
        );
        assert!(
            payload_validation_issue(&staged_entries).is_some(),
            "runtime binary renamed as portability is rejected in staging"
        );

        let installed_root = fixture.path().join("installed");
        let installed_bin = installed_root.join("bin");
        let installed_docs = installed_root.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&installed_bin).expect("installed bin");
        fs::create_dir_all(&installed_docs).expect("installed docs");
        fs::write(installed_bin.join(ReleaseTarget::binary_name()), "launcher")
            .expect("installed launcher");
        for backend in MANAGED_BACKENDS {
            fs::copy(built_backend_binary(backend), installed_bin.join(backend))
                .expect("copy installed real backend");
        }
        fs::copy(
            built_backend_binary(crate::consts::TOOL_RUNTIME),
            installed_bin.join(crate::consts::TOOL_PORTABILITY),
        )
        .expect("swap installed portability backend");
        fs::write(installed_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("installed docs manifest");
        fs::write(installed_docs.join("README.md"), "# docs\n").expect("installed docs README");
        assert!(
            !managed_payload_complete(&installed_bin.join(ReleaseTarget::binary_name())),
            "runtime binary renamed as portability is rejected in an installed bundle"
        );
    }

    #[cfg(unix)]
    #[test]
    fn backend_probe_accepts_a_healthy_companion_after_the_old_two_second_limit() {
        let fixture = TempDir::new().expect("fixture");
        let backend = crate::consts::TOOL_RUNTIME;
        let path = fixture.path().join(backend);
        write_slow_backend_probe(&path, backend, "2.25");

        assert_eq!(
            backend_cli_failure(&path, backend, BACKEND_PROBE_TIMEOUT),
            None,
            "healthy companion should pass when its first help response exceeds the old deadline"
        );
    }

    #[cfg(unix)]
    #[test]
    fn hanging_noisy_backend_is_bounded_and_cleaned_up_in_staged_and_installed_bundles() {
        let fixture = TempDir::new().expect("fixture");

        let staged_root = fixture.path().join("staged");
        let staged_bin = staged_root.join("bin");
        let staged_docs = staged_root.join("share/sc-lint/sc-lint-docs");
        let staged_pid_file = fixture.path().join("staged-child-pids");
        fs::create_dir_all(&staged_bin).expect("staged bin");
        fs::create_dir_all(&staged_docs).expect("staged docs");
        fs::write(staged_bin.join(ReleaseTarget::binary_name()), "launcher")
            .expect("staged launcher");
        for backend in [
            crate::consts::TOOL_BOUNDARY,
            crate::consts::TOOL_PORTABILITY,
        ] {
            write_backend_probe(&staged_bin.join(backend), backend);
        }
        write_hanging_noisy_stub(
            &staged_bin.join(crate::consts::TOOL_RUNTIME),
            &staged_pid_file,
        );
        fs::write(staged_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("staged docs manifest");
        fs::write(staged_docs.join("README.md"), "# docs\n").expect("staged docs README");
        let staged_launcher = staged_bin.join(ReleaseTarget::binary_name());
        let staged_managed = fixture.path().join("managed/bin/sc-lint");
        let probe_timeout = Duration::from_secs(5);
        let staged_is_complete = run_probe_with_watchdog(&staged_pid_file, move || {
            let entries = release_payload_candidates(&staged_launcher, &staged_managed);
            payload_validation_issue_with_timeout(&entries, probe_timeout)
        });
        let staged_problem = staged_is_complete.expect("hanging staged companion is rejected");
        assert!(
            staged_problem.contains(crate::consts::TOOL_RUNTIME),
            "unexpected staged probe failure: {staged_problem}"
        );
        assert!(
            staged_problem.contains("timed out after 5000 ms"),
            "unexpected staged probe failure: {staged_problem}"
        );
        assert_probe_children_stopped(&staged_pid_file);

        let installed_root = fixture.path().join("installed");
        let installed_bin = installed_root.join("bin");
        let installed_docs = installed_root.join("share/sc-lint/sc-lint-docs");
        let installed_pid_file = fixture.path().join("installed-child-pids");
        fs::create_dir_all(&installed_bin).expect("installed bin");
        fs::create_dir_all(&installed_docs).expect("installed docs");
        fs::write(installed_bin.join(ReleaseTarget::binary_name()), "launcher")
            .expect("installed launcher");
        for backend in [
            crate::consts::TOOL_BOUNDARY,
            crate::consts::TOOL_PORTABILITY,
        ] {
            write_backend_probe(&installed_bin.join(backend), backend);
        }
        write_hanging_noisy_stub(
            &installed_bin.join(crate::consts::TOOL_RUNTIME),
            &installed_pid_file,
        );
        fs::write(installed_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("installed docs manifest");
        fs::write(installed_docs.join("README.md"), "# docs\n").expect("installed docs README");
        let installed_launcher = installed_bin.join(ReleaseTarget::binary_name());
        let installed_is_complete = run_probe_with_watchdog(&installed_pid_file, move || {
            managed_payload_validation_issue_with_timeout(&installed_launcher, probe_timeout)
        });
        let installed_problem =
            installed_is_complete.expect("hanging installed companion is rejected");
        assert!(installed_problem.contains(crate::consts::TOOL_RUNTIME));
        assert!(installed_problem.contains("timed out after 5000 ms"));
        assert_probe_children_stopped(&installed_pid_file);
    }

    #[test]
    fn rollback_failure_never_claims_the_previous_binary_was_restored() {
        let fixture = TempDir::new().expect("fixture");
        let target = fixture.path().join(ReleaseTarget::binary_name());
        let candidate = fixture.path().join("candidate");
        fs::write(&target, "known-good").expect("target");
        fs::write(&candidate, "candidate").expect("candidate");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let file_ops = FailRollbackRemove {
            target: target.clone(),
        };

        let error = activate_payload_with(
            &[PayloadEntry {
                source: candidate,
                target: target.clone(),
            }],
            &target,
            &minimum,
            &file_ops,
        )
        .expect_err("a failed rollback must surface separately");

        assert_eq!(error.code(), "CLI.SC_LINT_INSTALL_ROLLBACK_FAILED");
        assert!(
            error
                .suggested_action
                .as_deref()
                .is_some_and(|action| action.contains("known-good backup manually"))
        );
        assert_eq!(
            error.details.get("backup_path"),
            Some(&json!(
                fixture
                    .path()
                    .join(format!(".{}-sc-lint-backup", ReleaseTarget::binary_name()))
                    .display()
                    .to_string()
            ))
        );
        assert!(
            fixture
                .path()
                .join(format!(".{}-sc-lint-backup", ReleaseTarget::binary_name()))
                .is_file()
        );
    }

    #[test]
    #[serial]
    fn setup_and_upgrade_command_dispatch_covers_all_installation_states_on_every_platform() {
        let fixture = TempDir::new().expect("fixture");
        let config_path = fixture.path().join("sc-lint.toml");
        let built_binary = built_cli_binary();
        let built_binary_dir = built_binary.parent().expect("built CLI directory");
        let install_fixture = TempDir::new_in(built_binary_dir).expect("same-filesystem fixture");
        let install_dir = install_fixture.path().join("managed");
        fs::create_dir(&install_dir).expect("managed directory");
        let _environment = InstallerEnvironment::set(&[(INSTALL_DIR_ENV, install_dir.as_os_str())]);
        let package_version = Version::parse(env!("CARGO_PKG_VERSION")).expect("package version");
        // No managed binary: setup's dry-run follows the real dispatch path,
        // and compatibility check proves the structured missing-binary path.
        write_consumer_config(&config_path, &package_version);
        let missing = execute_install_command(&config_path, &["setup", "--dry-run"]);
        assert_eq!(missing["status"], "dry_run");
        let missing_path = fixture.path().join(ReleaseTarget::binary_name());
        let missing_error = execute_command(
            &config_path,
            &[
                "compatibility",
                "check",
                "--binary",
                missing_path.to_str().unwrap(),
            ],
        )
        .expect_err("missing binary must use command dispatch");
        assert_eq!(missing_error.code(), "CLI.SC_LINT_BINARY_NOT_FOUND");

        // The built CLI is a native executable on each CI host, so all three
        // version states exercise the actual installer command path without a
        // Unix shell fixture or platform-specific permission assumptions. CI
        // may reuse a cached executable from a preceding workspace build, so
        // derive the compatible floor from the binary we actually link
        // rather than assuming it is this test crate's package version.
        let managed_binary = install_dir.join(ReleaseTarget::binary_name());
        // Keep the test process from holding a write fd for the executable it probes.
        fs::hard_link(&built_binary, &managed_binary).expect("hard link native CLI probe");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&install_dir.join(backend), backend);
        }
        let docs = install_fixture.path().join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&docs).expect("managed docs");
        fs::write(docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("manifest");
        fs::write(docs.join("README.md"), "# docs\n").expect("docs overview");
        let current = probe_version(&managed_binary).expect("probe linked native CLI");

        let mut old_floor = current.clone();
        old_floor.patch += 1;
        write_consumer_config(&config_path, &old_floor);
        let old = execute_install_command(&config_path, &["upgrade", "--check"]);
        assert_eq!(old["status"], "update_required");

        write_consumer_config(&config_path, &current);
        let current_result = execute_install_command(&config_path, &["setup"]);
        assert_eq!(current_result["status"], "current");
        assert_eq!(current_result["installed_version"], current.to_string());

        write_consumer_config(&config_path, &Version::new(0, 0, 0));
        let newer = execute_install_command(&config_path, &["upgrade", "--check"]);
        assert_eq!(newer["status"], "current");
        assert_eq!(newer["installed_version"], current.to_string());
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn setup_and_upgrade_cover_missing_old_current_and_newer_versions_without_touching_consumer_files()
     {
        let fixture = TempDir::new().expect("fixture");
        let release_base = write_local_release_fixture(fixture.path(), "0.4.1");
        let config_path = fixture.path().join("consumer").join("sc-lint.toml");
        let consumer_readme = fixture.path().join("consumer").join("README.md");
        fs::create_dir_all(config_path.parent().expect("consumer directory")).expect("consumer");
        fs::write(
            &config_path,
            "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
        )
        .expect("config");
        fs::write(&consumer_readme, "consumer-owned README\n").expect("readme");
        let install_dir = fixture.path().join("managed");
        let _environment = InstallerEnvironment::set(&[
            (INSTALL_DIR_ENV, install_dir.as_os_str()),
            (RELEASE_BASE_URL_ENV, release_base.as_os_str()),
        ]);

        let missing = execute_install_command(&config_path, &["setup"]);
        assert_eq!(missing["status"], "installed");
        assert_eq!(missing["selected_release_version"], "0.4.1");
        assert_eq!(
            fs::read_to_string(&consumer_readme).expect("readme"),
            "consumer-owned README\n"
        );

        write_probe(&install_dir.join(ReleaseTarget::binary_name()), "0.4.0");
        let old = execute_install_command(&config_path, &["upgrade"]);
        assert_eq!(old["status"], "upgraded");
        assert_eq!(old["installed_version"], "0.4.1");

        let current = execute_install_command(&config_path, &["setup"]);
        assert_eq!(current["status"], "current");
        assert_eq!(current["installed_version"], "0.4.1");

        // A matching launcher version does not make an incomplete managed
        // bundle current. setup repairs missing required release payloads.
        fs::write(install_dir.join(crate::consts::TOOL_RUNTIME), "")
            .expect("truncate runtime companion");
        fs::remove_file(install_dir.join("../share/sc-lint/sc-lint-docs/README.md"))
            .expect("remove docs overview");
        let repaired = execute_install_command(&config_path, &["setup"]);
        assert_eq!(repaired["status"], "upgraded");
        assert!(install_dir.join(crate::consts::TOOL_RUNTIME).is_file());
        assert!(
            backend_cli_failure(
                &install_dir.join(crate::consts::TOOL_RUNTIME),
                crate::consts::TOOL_RUNTIME,
                BACKEND_PROBE_TIMEOUT,
            )
            .is_none()
        );
        assert!(
            install_dir
                .join("../share/sc-lint/sc-lint-docs/README.md")
                .is_file()
        );

        write_probe(&install_dir.join(ReleaseTarget::binary_name()), "0.5.0");
        let newer = execute_install_command(&config_path, &["upgrade"]);
        assert_eq!(newer["status"], "current");
        assert_eq!(newer["installed_version"], "0.5.0");
        assert_eq!(
            fs::read_to_string(&consumer_readme).expect("readme"),
            "consumer-owned README\n"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn real_release_archive_installs_self_contained_bundle_ahead_of_stale_path_helpers() {
        let fixture = TempDir::new().expect("fixture");
        let version = env!("CARGO_PKG_VERSION");
        let minimum = MinimumVersion::from_str(version).expect("package version");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let target = ReleaseTarget::detect().expect("supported target");
        let archive_name = target.archive_name(&release_version);
        let release_dir = fixture.path().join("release").join(format!("v{version}"));
        let payload_dir = fixture.path().join("payload");
        let archive_root = payload_dir.join(format!("sc-lint_{version}_{}", target.triple));
        let bin_dir = archive_root.join("bin");
        build_release_binaries();
        fs::create_dir_all(&release_dir).expect("release directory");
        fs::create_dir_all(&bin_dir).expect("archive bin directory");
        let debug_dir = built_cli_binary()
            .parent()
            .expect("debug directory")
            .to_path_buf();
        for name in
            std::iter::once(ReleaseTarget::binary_name()).chain(MANAGED_BACKENDS.iter().copied())
        {
            fs::copy(debug_dir.join(name), bin_dir.join(name)).expect("copy release binary");
        }
        copy_directory(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs-bundle"),
            &archive_root.join("share/sc-lint/sc-lint-docs"),
        );
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(".")
            .status()
            .expect("tar starts");
        assert!(tar_status.success(), "tar produced release-shaped archive");
        let digest = sha256(&archive).expect("archive checksum");
        fs::write(
            release_dir.join("checksums.txt"),
            format!("{digest} {archive_name}\n"),
        )
        .expect("checksum manifest");

        let install_dir = fixture.path().join("managed/bin");
        let config_path = fixture.path().join("consumer/sc-lint.toml");
        let consumer = config_path.parent().expect("consumer dir");
        fs::create_dir_all(consumer).expect("consumer directory");
        fs::write(
            &config_path,
            format!("[tool.sc-lint]\nminimum_version = \"{version}\"\n"),
        )
        .expect("consumer config");
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

        let stale_path = fixture.path().join("stale-path");
        fs::create_dir_all(&stale_path).expect("stale PATH directory");
        for backend in [crate::consts::TOOL_PORTABILITY, crate::consts::TOOL_RUNTIME] {
            write_failing_stub(&stale_path.join(backend));
        }
        let stale_and_original_path = env::join_paths(
            std::iter::once(stale_path.as_path().as_os_str().to_os_string()).chain(
                env::split_paths(&env::var_os("PATH").expect("PATH"))
                    .map(|path| path.into_os_string()),
            ),
        )
        .expect("joined PATH");
        let release_base: OsString =
            format!("file://{}", fixture.path().join("release").display()).into();
        let _environment = InstallerEnvironment::set(&[
            (INSTALL_DIR_ENV, install_dir.as_os_str()),
            (RELEASE_BASE_URL_ENV, release_base.as_os_str()),
            ("PATH", stale_and_original_path.as_os_str()),
        ]);

        let installed = execute_install_command(&config_path, &["setup"]);
        assert_eq!(installed["status"], "installed");
        let managed_binary = install_dir.join(ReleaseTarget::binary_name());
        for args in [
            vec![
                "--json",
                "--config",
                config_path.to_str().unwrap(),
                "--root",
                workspace.to_str().unwrap(),
                "lint",
                "sc-boundary",
            ],
            vec![
                "--json",
                "--config",
                config_path.to_str().unwrap(),
                "--root",
                workspace.to_str().unwrap(),
                "lint",
                "sc-runtime",
            ],
            vec![
                "--json",
                "--config",
                config_path.to_str().unwrap(),
                "--root",
                workspace.to_str().unwrap(),
                "lint",
                "sc-portability",
            ],
            vec!["--json", "docs"],
            vec!["--json", "version"],
        ] {
            let output = Command::new(&managed_binary)
                .args(&args)
                .current_dir(&workspace)
                .output()
                .expect("managed command starts");
            assert!(
                output.status.success(),
                "managed command {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
            let envelope: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
            assert_eq!(envelope["ok"], true, "command {:?}", args);
            if args.last() == Some(&"version") {
                assert_eq!(envelope["data"]["self_contained"], true);
            }
        }
    }

    #[test]
    fn version_comparison_fixture_runs_on_every_supported_platform() {
        let fixture = TempDir::new().expect("fixture");
        let managed = fixture.path().join(ReleaseTarget::binary_name());
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let probe = |path: &Path| {
            let version = fs::read_to_string(path).map_err(|error| error.to_string())?;
            Version::parse(version.trim()).map_err(|error| error.to_string())
        };

        for (version, expected) in [
            ("0.4.0", None),
            ("0.4.1", Some("0.4.1")),
            ("0.5.0", Some("0.5.0")),
        ] {
            fs::write(&managed, version).expect("version fixture");
            let found = find_compatible_binary_with(&managed, &minimum, false, probe);
            assert_eq!(
                found.map(|(_, version)| version.to_string()),
                expected.map(str::to_owned)
            );
        }
    }

    struct FailRollbackRemove {
        target: PathBuf,
    }

    impl InstallerFileOps for FailRollbackRemove {
        fn remove_file(&self, path: &Path) -> io::Result<()> {
            if path == self.target {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected rollback removal failure",
                ));
            }
            fs::remove_file(path)
        }

        fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
            fs::rename(source, destination)
        }
    }

    struct InstallerEnvironment {
        original: Vec<(&'static str, Option<OsString>)>,
    }

    impl InstallerEnvironment {
        fn set(values: &[(&'static str, &std::ffi::OsStr)]) -> Self {
            let original = values
                .iter()
                .map(|(name, _)| (*name, env::var_os(name)))
                .collect();
            for (name, value) in values {
                // Tests are serialized because process environment is global.
                unsafe { env::set_var(name, value) };
            }
            Self { original }
        }
    }

    impl Drop for InstallerEnvironment {
        fn drop(&mut self) {
            for (name, value) in &self.original {
                match value {
                    Some(value) => unsafe { env::set_var(name, value) },
                    None => unsafe { env::remove_var(name) },
                }
            }
        }
    }

    fn execute_command(config_path: &Path, command_args: &[&str]) -> Result<Value, CliError> {
        let mut args = vec![
            "sc-lint",
            "--config",
            config_path.to_str().expect("UTF-8 config path"),
        ];
        args.extend_from_slice(command_args);
        let cli = crate::Cli::parse_from(args);
        let context = crate::CommandContext::from_cli(&cli).expect("command context");
        let loaded = crate::LoadedConfig::load(&cli, &context).expect("consumer config");
        crate::command::execute(&context, &loaded).map(|success| success.data)
    }

    fn execute_install_command(config_path: &Path, command_args: &[&str]) -> Value {
        execute_command(config_path, command_args).expect("installer command")
    }

    fn write_consumer_config(path: &Path, minimum: &Version) {
        fs::write(
            path,
            format!("[tool.sc-lint]\nminimum_version = \"{minimum}\"\n"),
        )
        .expect("consumer config");
    }

    fn built_cli_binary() -> PathBuf {
        built_artifact_directory().join(ReleaseTarget::binary_name())
    }

    #[cfg(unix)]
    fn built_backend_binary(name: &str) -> PathBuf {
        let name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        };
        built_artifact_directory().join(name)
    }

    fn built_artifact_directory() -> PathBuf {
        env::current_exe()
            .expect("test executable")
            .parent()
            .and_then(Path::parent)
            .expect("target debug directory")
            .to_path_buf()
    }

    #[cfg(unix)]
    fn build_release_binaries() {
        // `cargo test -p sc-lint --lib` builds the test executable but not the
        // companion backend executables this fixture copies from target/debug.
        // Derive Cargo's target directory from the test executable so an
        // explicit `cargo test --target-dir` is honored by this nested build.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let artifact_directory = built_artifact_directory();
        let target_dir = artifact_directory.parent().expect("Cargo target directory");
        let build_status = Command::new("cargo")
            .arg("build")
            .arg("--quiet")
            .arg("--target-dir")
            .arg(target_dir)
            .args([
                "-p",
                "sc-lint",
                "-p",
                "sc-lint-boundary",
                "-p",
                "sc-lint-portability",
                "-p",
                "sc-lint-runtime",
            ])
            .current_dir(&workspace_root)
            .status()
            .expect("build release backend binaries");
        assert!(build_status.success(), "backend binaries build");
    }

    #[cfg(unix)]
    fn write_local_release_fixture(root: &Path, version: &str) -> OsString {
        let minimum = MinimumVersion::from_str(version).expect("fixture version");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let release_target = ReleaseTarget::detect().expect("supported fixture host");
        let archive_name = release_target.archive_name(&release_version);
        let release_dir = root.join("release").join(format!("v{version}"));
        let payload_dir = root.join("payload");
        fs::create_dir_all(&release_dir).expect("release directory");
        let archive_root = payload_dir.join(format!("sc-lint_{version}_{}", release_target.triple));
        fs::create_dir_all(archive_root.join("bin")).expect("published archive bin directory");
        write_probe(
            &archive_root.join("bin").join(ReleaseTarget::binary_name()),
            version,
        );
        add_fake_release_companions(&archive_root);
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(".")
            .status()
            .expect("tar starts");
        assert!(tar_status.success(), "tar produced fixture archive");
        let digest = sha256(&archive).expect("fixture checksum");
        fs::write(
            release_dir.join("checksums.txt"),
            format!("{digest} {archive_name}\n"),
        )
        .expect("checksums");
        format!("file://{}", root.join("release").display()).into()
    }

    #[cfg(unix)]
    fn add_fake_release_companions(root: &Path) {
        let bin_dir = root.join("bin");
        let docs = root.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&bin_dir).expect("release bin");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&bin_dir.join(backend), backend);
        }
        fs::create_dir_all(&docs).expect("release docs");
        fs::write(docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("docs manifest");
        fs::write(docs.join("README.md"), "# sc-lint\n").expect("docs README");
    }

    #[cfg(unix)]
    fn copy_directory(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).expect("destination directory");
        for entry in fs::read_dir(source).expect("source directory") {
            let entry = entry.expect("directory entry");
            let target = destination.join(entry.file_name());
            if entry.file_type().expect("entry type").is_dir() {
                copy_directory(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).expect("copy docs file");
            }
        }
    }

    #[cfg(unix)]
    fn write_failing_stub(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, "#!/bin/sh\nexit 97\n").expect("stale helper");
        let mut permissions = fs::metadata(path).expect("stub metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("make stub executable");
    }

    #[cfg(unix)]
    fn write_probe(path: &Path, version: &str) {
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' '{{\"ok\":true,\"command\":\"version\",\"data\":{{\"tool\":\"sc-lint\",\"version\":\"{version}\",\"contract_schema\":\"sc-lint-version-v1\"}}}}'"
        );
        // Write the executable in a child so the test process never owns its write fd.
        let mut child = Command::new("sh")
            .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "write-probe"])
            .arg(path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("probe writer starts");
        child
            .stdin
            .take()
            .expect("probe writer stdin")
            .write_all(script.as_bytes())
            .expect("script text");
        assert!(child.wait().expect("probe writer waits").success());
    }

    fn write_backend_probe(path: &Path, name: &str) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let about = backend_about(name).expect("known backend identity");
            fs::write(
            path,
            format!(
                    "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '%s\\n' 'Usage: {name} <COMMAND>' '{about}' 'Commands:' '  analyze  Analyze a workspace'\n  exit 0\nfi\nexit 97\n"
            ),
        )
        .expect("backend probe");
            let mut permissions = fs::metadata(path).expect("probe metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("make backend probe executable");
        }
        #[cfg(not(unix))]
        {
            let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let executable = format!("{name}.exe");
            let destination = path.with_file_name(format!("{name}.exe"));
            fs::copy(
                workspace_root.join("target/debug").join(executable),
                destination,
            )
            .expect("copy built backend executable");
        }
    }

    #[cfg(unix)]
    fn write_slow_backend_probe(path: &Path, name: &str, delay_seconds: &str) {
        use std::os::unix::fs::PermissionsExt;

        let about = backend_about(name).expect("known backend identity");
        fs::write(
            path,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  sleep {delay_seconds}\n  printf '%s\\n' 'Usage: {name} <COMMAND>' '{about}' 'Commands:' '  analyze  Analyze a workspace'\n  exit 0\nfi\nexit 97\n"
            ),
        )
        .expect("slow backend probe");
        let mut permissions = fs::metadata(path).expect("probe metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("make slow backend probe executable");
    }

    #[cfg(unix)]
    fn write_hanging_noisy_stub(path: &Path, child_pid_file: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let child_pid_file = child_pid_file.to_string_lossy().replace('\'', "'\\''");
        fs::write(
            path,
            format!(
                "#!/bin/sh\nsleep 600 &\nsleeper=$!\nyes noisy-child &\nnoiser=$!\nprintf '%s %s %s\\n' \"$$\" \"$sleeper\" \"$noiser\" > '{child_pid_file}'\nwait\n"
            ),
        )
        .expect("write hanging noisy backend");
        let mut permissions = fs::metadata(path)
            .expect("hanging backend metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("make hanging backend executable");
    }

    #[cfg(unix)]
    fn run_probe_with_watchdog<T: Send + 'static>(
        child_pid_file: &Path,
        probe: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = sender.send(probe());
        });
        match receiver.recv_timeout(BACKEND_PROBE_TEST_WATCHDOG) {
            Ok(result) => {
                worker.join().expect("probe worker");
                result
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                terminate_test_probe_processes(child_pid_file);
                panic!(
                    "backend probe exceeded the {}-second test deadlock watchdog",
                    BACKEND_PROBE_TEST_WATCHDOG.as_secs()
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let _ = worker.join();
                panic!("probe worker exited without returning a result");
            }
        }
    }

    #[cfg(unix)]
    fn terminate_test_probe_processes(child_pid_file: &Path) {
        let Ok(pids) = fs::read_to_string(child_pid_file) else {
            return;
        };
        let pids = pids
            .split_whitespace()
            .filter_map(|pid| pid.parse::<u32>().ok())
            .collect::<Vec<_>>();
        if let Some(group_id) = pids.first() {
            let process_group = format!("-{group_id}");
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &process_group])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        for pid in pids {
            let _ = Command::new("/bin/kill")
                .args(["-KILL", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    #[cfg(unix)]
    fn assert_probe_children_stopped(child_pid_file: &Path) {
        let child_pids = fs::read_to_string(child_pid_file)
            .expect("probe recorded child pids")
            .split_whitespace()
            .map(|pid| pid.parse::<u32>().expect("child pid"))
            .collect::<Vec<_>>();
        assert_eq!(
            child_pids.len(),
            3,
            "probe recorded parent and both child pids"
        );
        for pid in child_pids {
            let mut details = probe_process_details(pid);
            for _ in 0..100 {
                if details
                    .as_deref()
                    .is_none_or(|details| matches!(details.as_bytes().first(), Some(b'Z' | b'X')))
                {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
                details = probe_process_details(pid);
            }
            if details
                .as_deref()
                .is_some_and(|details| matches!(details.as_bytes().first(), Some(b'Z' | b'X')))
            {
                eprintln!("backend probe child {pid} terminated; process details {details:?}");
            }
            assert!(
                details
                    .as_deref()
                    .is_none_or(|details| matches!(details.as_bytes().first(), Some(b'Z' | b'X'))),
                "backend probe child {pid} remained live with process details {details:?}"
            );
        }
    }

    #[cfg(unix)]
    fn probe_process_details(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-o", "stat=,ppid=,pgid=,command=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|state| !state.is_empty())
    }
}
