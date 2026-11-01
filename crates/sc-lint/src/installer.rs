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
pub(crate) use probe::{ProbeFailure, probe_backend_version, probe_version, run_version_command};

const DEFAULT_RELEASE_BASE_URL: &str = "https://github.com/randlee/sc-lint/releases/download";
const RELEASE_BASE_URL_ENV: &str = "SC_LINT_RELEASE_BASE_URL";
const INSTALL_DIR_ENV: &str = "SC_LINT_INSTALL_DIR";
pub(crate) const BACKEND_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
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
/// checkout scripts; E.3 owns rendering it into `.sc-lint/bootstrap.py`.
pub(crate) const CONSUMER_BOOTSTRAP_ASSET: &str = include_str!("../assets/bootstrap.py");

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

struct ReleaseUrls<'a> {
    archive: &'a str,
    checksums: &'a str,
}

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
        Self::detect_for(env::consts::OS, env::consts::ARCH)
    }

    /// Selects the published release target for one host `os`/`arch` pair.
    ///
    /// `detect` passes the compile-time host; tests pass unpublished pairs so
    /// the real unsupported-platform failure runs on every CI host.
    fn detect_for(os: &str, arch: &str) -> Result<Self, CliError> {
        let target = match (os, arch) {
            ("linux", "x86_64") => Self {
                triple: "x86_64-unknown-linux-gnu",
                archive_extension: "tar.gz",
            },
            ("linux", "aarch64") => Self {
                triple: "aarch64-unknown-linux-gnu",
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
            ("windows", "aarch64") => Self {
                triple: "aarch64-pc-windows-msvc",
                archive_extension: "zip",
            },
            (os, arch) => {
                return Err(install_error(
                    InstallerErrorCode::UnsupportedPlatform,
                    format!("sc-lint has no verified release for host platform `{os}/{arch}`"),
                    "The release workflow publishes Linux, macOS, and Windows artifacts for x86_64 and aarch64.",
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

pub(crate) fn run_with_probes(
    loaded: &LoadedConfig,
    check_only: bool,
    dry_run: bool,
    version_probe: impl Fn(&Path) -> Option<Version>,
    payload_complete: impl Fn(&Path) -> bool,
) -> Result<Value, CliError> {
    debug_assert!(
        !CONSUMER_BOOTSTRAP_ASSET.is_empty(),
        "the E.3 consumer bootstrap source asset must be packaged with the product"
    );
    let (minimum_version, config_path) = loaded.compatibility_requirement()?;
    let release_version = ResolvedReleaseVersion::for_minimum(minimum_version);
    let target = ReleaseTarget::detect()?;
    let install_dir = managed_install_dir()?;
    let managed_binary = install_dir.join(ReleaseTarget::binary_name());
    let use_path_candidates = env::var_os(INSTALL_DIR_ENV).is_none();
    let had_previous_installation = std::iter::once(managed_binary.clone())
        .chain(path_binary_candidates_if(use_path_candidates))
        .any(|binary| binary.is_file());
    let existing = find_existing_installation(
        &managed_binary,
        minimum_version,
        version_probe,
        payload_complete,
    );

    if let Some((binary, version)) = existing {
        return Ok(existing_installation_result(
            minimum_version,
            &version,
            &binary,
            &install_dir,
            config_path,
        ));
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

fn existing_installation_result(
    minimum_version: &MinimumVersion,
    version: &Version,
    binary: &Path,
    install_dir: &Path,
    config_path: &Path,
) -> Value {
    json!({
        "status": "current",
        "action": "found",
        "minimum_version": minimum_version.to_string(),
        "installed_version": version.to_string(),
        "binary_path": binary.display().to_string(),
        "install_dir": install_dir.display().to_string(),
        "config_path": config_path.display().to_string(),
        "summary": "a compatible sc-lint installation is already available",
    })
}

fn find_existing_installation(
    managed_binary: &Path,
    minimum_version: &MinimumVersion,
    version_probe: impl Fn(&Path) -> Option<Version>,
    payload_complete: impl Fn(&Path) -> bool,
) -> Option<(PathBuf, Version)> {
    let use_path_candidates = env::var_os(INSTALL_DIR_ENV).is_none();
    let managed_existing =
        find_compatible_binary_with(managed_binary, minimum_version, false, |binary| {
            let version = version_probe(binary)?;
            payload_complete(binary).then_some(version)
        });
    managed_existing.or_else(|| {
        // PATH remains a development fallback only when there is no managed
        // launcher; partial managed installations must be repaired.
        (!managed_binary.exists() && use_path_candidates)
            .then(|| {
                find_compatible_binary_with(managed_binary, minimum_version, true, version_probe)
            })
            .flatten()
    })
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
    F: Fn(&Path) -> Option<Version>,
{
    std::iter::once(managed_binary.to_path_buf())
        .chain(path_binary_candidates_if(use_path_candidates))
        .find_map(|binary| {
            let version = probe(&binary)?;
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
    install_verified_release_with_file_ops(
        release_version,
        minimum_version,
        target,
        install_dir,
        managed_binary,
        &ReleaseUrls {
            archive: archive_url,
            checksums: checksums_url,
        },
        &SystemFileOps,
    )
}

fn install_verified_release_with_file_ops<F: InstallerFileOps>(
    release_version: &ResolvedReleaseVersion,
    minimum_version: &MinimumVersion,
    target: ReleaseTarget,
    install_dir: &Path,
    managed_binary: &Path,
    release_urls: &ReleaseUrls<'_>,
    file_ops: &F,
) -> Result<Version, CliError> {
    file_ops
        .create_dir_all(install_dir)
        .map_err(|error| permission_error(install_dir, error))?;
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
        release_urls,
        &staging,
        file_ops,
    );
    let _ = fs::remove_dir_all(&staging);
    result
}

fn install_into_staging<F: InstallerFileOps>(
    release_version: &ResolvedReleaseVersion,
    minimum_version: &MinimumVersion,
    target: ReleaseTarget,
    managed_binary: &Path,
    release_urls: &ReleaseUrls<'_>,
    staging: &Path,
    file_ops: &F,
) -> Result<Version, CliError> {
    let archive_name = target.archive_name(release_version);
    let checksum_file = staging.join("checksums.txt");
    let archive = staging.join(&archive_name);
    download(release_urls.checksums, &checksum_file)?;
    download(release_urls.archive, &archive)?;
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
    activate_payload_with(&payload, managed_binary, minimum_version, file_ops)
}

#[derive(Debug, Clone)]
struct PayloadEntry {
    source: PathBuf,
    target: PathBuf,
}

type PayloadChange = (PathBuf, PathBuf, bool, bool);
type ProbeOutputReader = std::sync::mpsc::Receiver<io::Result<Vec<u8>>>;

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
    payload_validation_issue_with_probe(entries, probe_timeout, backend_cli_failure)
}

fn payload_validation_issue_with_probe(
    entries: &[PayloadEntry],
    probe_timeout: Duration,
    mut probe: impl FnMut(&Path, &str, Duration) -> Option<String>,
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
            if let Some(problem) = probe(&entry.source, backend, probe_timeout) {
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

pub(crate) fn managed_payload_complete(binary: &Path) -> bool {
    managed_payload_validation_issue(binary).is_none()
}

fn managed_payload_validation_issue(binary: &Path) -> Option<String> {
    managed_payload_validation_issue_with_timeout(binary, BACKEND_PROBE_TIMEOUT)
}

fn managed_payload_validation_issue_with_timeout(
    binary: &Path,
    probe_timeout: Duration,
) -> Option<String> {
    managed_payload_validation_issue_with_probe(binary, probe_timeout, backend_cli_failure)
}

fn managed_payload_validation_issue_with_probe(
    binary: &Path,
    probe_timeout: Duration,
    mut probe: impl FnMut(&Path, &str, Duration) -> Option<String>,
) -> Option<String> {
    let parent = binary.parent().unwrap_or_else(|| Path::new("."));
    for backend in MANAGED_BACKENDS {
        let name = if cfg!(windows) {
            format!("{backend}.exe")
        } else {
            (*backend).to_string()
        };
        let path = parent.join(name);
        if let Some(problem) = probe(&path, backend, probe_timeout) {
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
    use std::io::{Read, Write};
    use std::str::FromStr;

    use tempfile::TempDir;

    const MINIMAL_DOCS_MANIFEST: &str = "schema_version = 1\n\n[[guides]]\npath = \"README.md\"\n";
    #[cfg(unix)]
    const BACKEND_PROBE_TEST_WATCHDOG: Duration = Duration::from_secs(30);
    #[cfg(unix)]
    const BACKEND_OUTPUT_TEST_WAIT: Duration = Duration::from_secs(5);
    /// Failure ceiling for the probe's processes to release the liveness FIFO
    /// after the test kills them. It does not bound the probe under test.
    #[cfg(unix)]
    const PROBE_HOLDER_EXIT_CEILING: Duration = Duration::from_secs(30);

    #[test]
    fn release_target_matches_the_published_release_matrix() {
        let target = ReleaseTarget::detect().expect("current test host is supported");
        assert!(
            [
                "x86_64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
                "x86_64-pc-windows-msvc",
                "aarch64-pc-windows-msvc",
            ]
            .contains(&target.triple)
        );
    }

    #[test]
    fn unsupported_host_platform_is_a_structured_installer_failure() {
        for (os, arch) in [("freebsd", "x86_64"), ("linux", "riscv64")] {
            let error = ReleaseTarget::detect_for(os, arch)
                .expect_err("an unpublished platform must not select a release target");
            assert_eq!(
                error.code(),
                "CLI.SC_LINT_INSTALL_UNSUPPORTED_PLATFORM",
                "{os}/{arch}"
            );
            assert_eq!(error.exit_code(), crate::consts::EXIT_BACKEND_FAILURE);
            assert!(
                error.message.contains(&format!("`{os}/{arch}`")),
                "{error:?}"
            );
            assert_eq!(error.details.get("host_os"), Some(&json!(os)));
            assert_eq!(error.details.get("host_arch"), Some(&json!(arch)));
            assert_installer_failure_is_recoverable(&error, "sc-lint setup");
        }
    }

    #[test]
    fn release_targets_select_both_architectures() {
        for (os, arch, triple, extension) in [
            ("linux", "x86_64", "x86_64-unknown-linux-gnu", "tar.gz"),
            ("linux", "aarch64", "aarch64-unknown-linux-gnu", "tar.gz"),
            ("windows", "x86_64", "x86_64-pc-windows-msvc", "zip"),
            ("windows", "aarch64", "aarch64-pc-windows-msvc", "zip"),
        ] {
            let target = ReleaseTarget::detect_for(os, arch).unwrap();
            assert_eq!(
                (target.triple, target.archive_extension),
                (triple, extension)
            );
        }
    }

    struct DenyManagedDirectoryCreation {
        denied_path: PathBuf,
    }

    impl InstallerFileOps for DenyManagedDirectoryCreation {
        fn remove_file(&self, path: &Path) -> io::Result<()> {
            fs::remove_file(path)
        }

        fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
            fs::rename(source, destination)
        }

        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            if path == self.denied_path {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "controlled installer filesystem denial",
                ));
            }
            fs::create_dir_all(path)
        }
    }

    #[test]
    fn installer_file_ops_permission_denied_is_a_recoverable_install_failure() {
        let fixture = TempDir::new().expect("fixture");
        let install_dir = fixture.path().join("managed");
        let managed_binary = install_dir.join(ReleaseTarget::binary_name());
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum version");
        let release_version = ResolvedReleaseVersion::for_minimum(&minimum);
        let target = ReleaseTarget::detect().expect("supported test target");
        let file_ops = DenyManagedDirectoryCreation {
            denied_path: install_dir.clone(),
        };

        let error = install_verified_release_with_file_ops(
            &release_version,
            &minimum,
            target,
            &install_dir,
            &managed_binary,
            &ReleaseUrls {
                archive: "file:///unused/archive",
                checksums: "file:///unused/checksums",
            },
            &file_ops,
        )
        .expect_err("injected filesystem denial");

        assert_eq!(error.code(), "CLI.SC_LINT_INSTALL_PERMISSION_DENIED");
        assert_eq!(error.exit_code(), crate::consts::EXIT_BACKEND_FAILURE);
        assert_eq!(
            error.details.get("managed_path"),
            Some(&json!(install_dir.display().to_string()))
        );
        assert_eq!(
            error.details.get("io_error_kind"),
            Some(&json!("PermissionDenied"))
        );
        assert_installer_failure_is_recoverable(&error, "SC_LINT_INSTALL_DIR");
        assert!(
            !install_dir.exists(),
            "denied directory must not be created"
        );
    }

    /// REQ-CLI-020: every installer failure names a cause, a recovery action,
    /// and the `sc-lint docs installation` reference.
    fn assert_installer_failure_is_recoverable(error: &CliError, recovery_hint: &str) {
        assert!(
            error
                .cause
                .as_deref()
                .is_some_and(|cause| !cause.is_empty()),
            "{error:?}"
        );
        assert!(
            error
                .suggested_action
                .as_deref()
                .is_some_and(|action| action.contains(recovery_hint)),
            "{error:?}"
        );
        assert_eq!(
            error.documentation.as_deref(),
            Some("sc-lint docs installation")
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
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains(".sc-lint/bootstrap.py exec"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("compatibility"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("--config"));
        assert!(!CONSUMER_BOOTSTRAP_ASSET.contains("cargo run"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("CLI.SC_LINT_RELEASE_UNAVAILABLE"));
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

        let error = activate_payload_with(
            &[PayloadEntry {
                source: candidate,
                target: target.clone(),
            }],
            &target,
            &minimum,
            &SystemFileOps,
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
        let error = activate_payload_with(&entries, &managed_binary, &minimum, &SystemFileOps)
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
        let fixture = TempDir::new().expect("fixture");
        let built_binaries = crate::test_support::release_binaries();
        let staged_root = fixture.path().join("staged");
        let staged_bin = staged_root.join("bin");
        let staged_docs = staged_root.join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&staged_bin).expect("staged bin");
        fs::create_dir_all(&staged_docs).expect("staged docs");
        fs::write(staged_bin.join(ReleaseTarget::binary_name()), "launcher")
            .expect("staged launcher");
        for backend in MANAGED_BACKENDS {
            fs::copy(&built_binaries[*backend], staged_bin.join(backend))
                .expect("copy staged real backend");
        }
        fs::copy(
            &built_binaries[crate::consts::TOOL_RUNTIME],
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
            fs::copy(&built_binaries[*backend], installed_bin.join(backend))
                .expect("copy installed real backend");
        }
        fs::copy(
            &built_binaries[crate::consts::TOOL_RUNTIME],
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
        let staged_ready_fifo = fixture.path().join("staged-probe-ready");
        fs::create_dir_all(&staged_bin).expect("staged bin");
        fs::create_dir_all(&staged_docs).expect("staged docs");
        create_probe_ready_fifo(&staged_ready_fifo);
        let staged_holders_fifo = fixture.path().join("staged-probe-holders");
        let staged_holders = watch_probe_holders(&staged_holders_fifo);
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
            &staged_ready_fifo,
            &staged_holders_fifo,
        );
        fs::write(staged_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("staged docs manifest");
        fs::write(staged_docs.join("README.md"), "# docs\n").expect("staged docs README");
        let staged_launcher = staged_bin.join(ReleaseTarget::binary_name());
        let staged_managed = fixture.path().join("managed/bin/sc-lint");
        let probe_timeout = Duration::from_secs(5);
        let staged_pid_file_for_probe = staged_pid_file.clone();
        let staged_ready_fifo_for_probe = staged_ready_fifo.clone();
        let staged_is_complete = run_probe_with_watchdog(&staged_pid_file, move || {
            let entries = release_payload_candidates(&staged_launcher, &staged_managed);
            payload_validation_issue_with_probe(
                &entries,
                probe_timeout,
                |path, backend, timeout| {
                    if backend == crate::consts::TOOL_RUNTIME {
                        hanging_backend_cli_failure_after_startup(
                            path,
                            backend,
                            timeout,
                            &staged_pid_file_for_probe,
                            &staged_ready_fifo_for_probe,
                        )
                    } else {
                        backend_cli_failure(path, backend, timeout)
                    }
                },
            )
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
        assert_probe_holders_exited(&staged_holders, &staged_pid_file);

        let installed_root = fixture.path().join("installed");
        let installed_bin = installed_root.join("bin");
        let installed_docs = installed_root.join("share/sc-lint/sc-lint-docs");
        let installed_pid_file = fixture.path().join("installed-child-pids");
        let installed_ready_fifo = fixture.path().join("installed-probe-ready");
        fs::create_dir_all(&installed_bin).expect("installed bin");
        fs::create_dir_all(&installed_docs).expect("installed docs");
        create_probe_ready_fifo(&installed_ready_fifo);
        let installed_holders_fifo = fixture.path().join("installed-probe-holders");
        let installed_holders = watch_probe_holders(&installed_holders_fifo);
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
            &installed_ready_fifo,
            &installed_holders_fifo,
        );
        fs::write(installed_docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST)
            .expect("installed docs manifest");
        fs::write(installed_docs.join("README.md"), "# docs\n").expect("installed docs README");
        let installed_launcher = installed_bin.join(ReleaseTarget::binary_name());
        let installed_pid_file_for_probe = installed_pid_file.clone();
        let installed_ready_fifo_for_probe = installed_ready_fifo.clone();
        let installed_is_complete = run_probe_with_watchdog(&installed_pid_file, move || {
            managed_payload_validation_issue_with_probe(
                &installed_launcher,
                probe_timeout,
                |path, backend, timeout| {
                    if backend == crate::consts::TOOL_RUNTIME {
                        hanging_backend_cli_failure_after_startup(
                            path,
                            backend,
                            timeout,
                            &installed_pid_file_for_probe,
                            &installed_ready_fifo_for_probe,
                        )
                    } else {
                        backend_cli_failure(path, backend, timeout)
                    }
                },
            )
        });
        let installed_problem =
            installed_is_complete.expect("hanging installed companion is rejected");
        assert!(installed_problem.contains(crate::consts::TOOL_RUNTIME));
        assert!(installed_problem.contains("timed out after 5000 ms"));
        assert_probe_holders_exited(&installed_holders, &installed_pid_file);
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
        let install_fixture = TempDir::new().expect("install fixture");
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

        // A placeholder path plus an injected version probe keeps this unit
        // test independent of whether Cargo built the product executable.
        let managed_binary = install_dir.join(ReleaseTarget::binary_name());
        fs::write(&managed_binary, "controlled probe fixture").expect("managed CLI fixture");
        for backend in MANAGED_BACKENDS {
            write_backend_probe(&install_dir.join(backend), backend);
        }
        let docs = install_fixture.path().join("share/sc-lint/sc-lint-docs");
        fs::create_dir_all(&docs).expect("managed docs");
        fs::write(docs.join("manifest.toml"), MINIMAL_DOCS_MANIFEST).expect("manifest");
        fs::write(docs.join("README.md"), "# docs\n").expect("docs overview");
        let current = package_version.clone();
        let version_probe = |_: &Path| Some(current.clone());

        let mut old_floor = current.clone();
        old_floor.patch += 1;
        write_consumer_config(&config_path, &old_floor);
        let old = execute_install_command_with_probe(
            &config_path,
            &["upgrade", "--check"],
            version_probe,
        );
        assert_eq!(old["status"], "update_required");

        write_consumer_config(&config_path, &current);
        let current_result =
            execute_install_command_with_probe(&config_path, &["setup"], version_probe);
        assert_eq!(current_result["status"], "current");
        assert_eq!(current_result["installed_version"], current.to_string());

        write_consumer_config(&config_path, &Version::new(0, 0, 0));
        let newer = execute_install_command_with_probe(
            &config_path,
            &["upgrade", "--check"],
            version_probe,
        );
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
    fn real_release_archive_installs_self_contained_bundle() {
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
        fs::create_dir_all(&release_dir).expect("release directory");
        fs::create_dir_all(&bin_dir).expect("archive bin directory");
        let built_binaries = crate::test_support::release_binaries();
        for name in
            std::iter::once(ReleaseTarget::binary_name()).chain(MANAGED_BACKENDS.iter().copied())
        {
            let binary = built_binaries
                .get(name)
                .unwrap_or_else(|| panic!("Cargo build did not produce release binary `{name}`"));
            fs::copy(binary, bin_dir.join(name)).unwrap_or_else(|error| {
                panic!("copy release binary `{}`: {error}", binary.display())
            });
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
        let original_path = env::var_os("PATH").expect("PATH");
        let controlled_search_path = env::join_paths(
            std::iter::once(stale_path.as_os_str().to_os_string())
                .chain(env::split_paths(&original_path).map(|path| path.into_os_string())),
        )
        .expect("joined controlled search path");

        let release_base: OsString =
            format!("file://{}", fixture.path().join("release").display()).into();
        let _environment = InstallerEnvironment::set(&[
            (INSTALL_DIR_ENV, install_dir.as_os_str()),
            (RELEASE_BASE_URL_ENV, release_base.as_os_str()),
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
                .env("PATH", &controlled_search_path)
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
            fs::read_to_string(path)
                .ok()
                .and_then(|version| Version::parse(version.trim()).ok())
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

    fn execute_install_command_with_probe(
        config_path: &Path,
        command_args: &[&str],
        version_probe: impl Fn(&Path) -> Option<Version>,
    ) -> Value {
        let mut args = vec![
            "sc-lint",
            "--config",
            config_path.to_str().expect("UTF-8 config path"),
        ];
        args.extend_from_slice(command_args);
        let cli = crate::Cli::parse_from(args);
        let context = crate::CommandContext::from_cli(&cli).expect("command context");
        let loaded = crate::LoadedConfig::load(&cli, &context).expect("consumer config");
        crate::command::execute_installer_with_test_probes(&context, &loaded, version_probe, |_| {
            true
        })
        .map(|success| success.data)
        .expect("installer command")
    }

    fn write_consumer_config(path: &Path, minimum: &Version) {
        fs::write(
            path,
            format!("[tool.sc-lint]\nminimum_version = \"{minimum}\"\n"),
        )
        .expect("consumer config");
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
            let script = format!(
                "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '%s\\n' 'Usage: {name} <COMMAND>' '{about}' 'Commands:' '  analyze  Analyze a workspace'\n  exit 0\nfi\nexit 97\n"
            );
            let mut child = Command::new("sh")
                .args([
                    "-c",
                    "cat > \"$1\" && chmod 755 \"$1\"",
                    "write-backend-probe",
                ])
                .arg(path)
                .stdin(Stdio::piped())
                .spawn()
                .expect("backend probe writer starts");
            child
                .stdin
                .take()
                .expect("backend probe stdin")
                .write_all(script.as_bytes())
                .expect("backend probe script");
            assert!(child.wait().expect("backend probe writer waits").success());
            let mut permissions = fs::metadata(path).expect("probe metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("make backend probe executable");
        }
        #[cfg(not(unix))]
        {
            let _ = name;
            fs::write(path.with_extension("exe"), "controlled backend fixture")
                .expect("controlled backend placeholder");
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
    fn write_hanging_noisy_stub(path: &Path, ready_fifo: &Path, holder_fifo: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let ready_fifo = ready_fifo.to_string_lossy().replace('\'', "'\\''");
        let holder_fifo = holder_fifo.to_string_lossy().replace('\'', "'\\''");
        // The shell holds the liveness FIFO open for writing before it forks, so
        // both children inherit it. Only process exit releases it.
        fs::write(
            path,
            format!(
                "#!/bin/sh\nexec 8>'{holder_fifo}'\nsleep 600 &\nsleeper=$!\nyes noisy-child &\nnoiser=$!\nprintf '%s %s %s\\n' \"$$\" \"$sleeper\" \"$noiser\" > '{ready_fifo}'\nwait\n"
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
    fn create_probe_ready_fifo(path: &Path) {
        let status = Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("start mkfifo for backend startup handshake");
        assert!(status.success(), "mkfifo creates startup handshake");
    }

    /// Creates the liveness FIFO and starts a watcher that reads it to EOF.
    /// The watcher's result arrives only after every process holding the
    /// write end has exited, which does not depend on how fast it is reaped.
    /// Call before starting the probe, because opening the read end blocks
    /// until the probe opens the write end.
    #[cfg(unix)]
    fn watch_probe_holders(path: &Path) -> std::sync::mpsc::Receiver<io::Result<Vec<u8>>> {
        create_probe_ready_fifo(path);
        let (sender, receiver) = std::sync::mpsc::channel();
        let path = path.to_path_buf();
        thread::spawn(move || {
            let result = fs::File::open(&path).and_then(|mut holder| {
                let mut released = Vec::new();
                holder.read_to_end(&mut released)?;
                Ok(released)
            });
            let _ = sender.send(result);
        });
        receiver
    }

    #[cfg(unix)]
    fn hanging_backend_cli_failure_after_startup(
        binary: &Path,
        expected_name: &str,
        timeout: Duration,
        child_pid_file: &Path,
        ready_fifo: &Path,
    ) -> Option<String> {
        let (mut child, reader) =
            spawn_probe(binary, expected_name, &["--help"], "--help").expect("start backend probe");
        let process_id = child.id();
        fs::write(child_pid_file, process_id.to_string()).expect("record probe process group");

        // The stub writes its parent and child pids to this FIFO only after
        // both descendants exist. Opening it blocks until that point, so the
        // production timeout starts after startup is established without
        // adding time to the timeout being tested.
        let ready_pids = fs::read_to_string(ready_fifo).expect("read backend startup handshake");
        fs::write(child_pid_file, &ready_pids).expect("record backend process tree");
        let pid_count = ready_pids.split_whitespace().count();
        assert_eq!(pid_count, 3, "startup handshake includes both children");

        let problem =
            match wait_for_backend_probe(&mut child, binary, expected_name, "--help", timeout) {
                Ok(status) => {
                    terminate_probe_processes(&mut child);
                    let _ = finish_backend_output_after_stop(
                        reader,
                        BACKEND_OUTPUT_TEST_WAIT,
                        BACKEND_OUTPUT_TEST_WAIT,
                        Some(|| terminate_probe_process_tree(process_id)),
                    );
                    panic!("hanging backend unexpectedly exited with {status}");
                }
                Err(problem) => problem,
            };
        let output = finish_backend_output_after_stop(
            reader,
            BACKEND_OUTPUT_TEST_WAIT,
            BACKEND_OUTPUT_TEST_WAIT,
            Some(|| terminate_probe_process_tree(process_id)),
        )
        .expect("drain noisy output after process-tree cleanup");
        assert!(
            String::from_utf8_lossy(&output).contains("noisy-child"),
            "the backend emitted noisy output before timeout"
        );
        Some(problem)
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
                terminate_test_probe_processes(child_pid_file);
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

    /// Waits for the probe's processes to release the liveness FIFO. A process
    /// holds it only while alive, so this proves the kill took effect without
    /// depending on how quickly the killed processes are reaped.
    #[cfg(unix)]
    fn assert_probe_holders_exited(
        holders: &std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
        child_pid_file: &Path,
    ) {
        match holders.recv_timeout(PROBE_HOLDER_EXIT_CEILING) {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => panic!("backend probe liveness FIFO failed: {error}"),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                terminate_test_probe_processes(child_pid_file);
                panic!(
                    "backend probe processes still hold the liveness FIFO after {PROBE_HOLDER_EXIT_CEILING:?}"
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                panic!("backend probe liveness watcher stopped without reporting");
            }
        }
    }
}
