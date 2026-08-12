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
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use etcetera::{BaseStrategy, choose_base_strategy};
use semver::Version;
use serde_json::{Value, json};

use crate::CliError;
use crate::MinimumVersion;
use crate::config::{LoadedConfig, VERSION_PROBE_SCHEMA};
use crate::error::ErrorCode;

const DEFAULT_RELEASE_BASE_URL: &str = "https://github.com/randlee/sc-lint/releases/download";
const RELEASE_BASE_URL_ENV: &str = "SC_LINT_RELEASE_BASE_URL";
const INSTALL_DIR_ENV: &str = "SC_LINT_INSTALL_DIR";

/// The only executable helper E.3 may place in a consumer repository.
///
/// It delegates behavior to the installed product instead of copying source
/// checkout scripts; E.3 owns rendering it into `.sc-lint/bootstrap`.
pub(crate) const CONSUMER_BOOTSTRAP_ASSET: &str = include_str!("../assets/bootstrap");

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
    let existing = find_compatible_binary(&managed_binary, minimum_version, use_path_candidates);

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

fn find_compatible_binary(
    managed_binary: &Path,
    minimum: &MinimumVersion,
    use_path_candidates: bool,
) -> Option<(PathBuf, Version)> {
    find_compatible_binary_with(managed_binary, minimum, use_path_candidates, probe_version)
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
    let candidate = extract_dir.join(ReleaseTarget::binary_name());
    if !candidate.is_file() {
        return Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!(
                "release archive `{archive_name}` did not contain `{}`",
                ReleaseTarget::binary_name()
            ),
            "The release artifact did not match the published sc-lint archive layout.",
            "Download a supported release again or report the malformed release artifact.",
        )
        .with_detail("archive_name", json!(archive_name))
        .with_detail("expected_binary", json!(ReleaseTarget::binary_name()))
        .with_detail(
            "extract_directory",
            json!(extract_dir.display().to_string()),
        ));
    }
    activate_candidate(&candidate, managed_binary, minimum_version)
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

trait InstallerFileOps {
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()>;
}

struct SystemFileOps;

impl InstallerFileOps for SystemFileOps {
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }
}

#[derive(Debug)]
struct PostInstallVerificationFailure {
    cause: String,
    observed_version: Option<Version>,
}

fn activate_candidate(
    candidate: &Path,
    target: &Path,
    minimum: &MinimumVersion,
) -> Result<Version, CliError> {
    activate_candidate_with(candidate, target, minimum, &SystemFileOps, probe_version)
}

fn activate_candidate_with<F, P>(
    candidate: &Path,
    target: &Path,
    minimum: &MinimumVersion,
    file_ops: &F,
    probe: P,
) -> Result<Version, CliError>
where
    F: InstallerFileOps,
    P: FnOnce(&Path) -> Result<Version, String>,
{
    reject_running_managed_binary(target)?;
    let parent = target.parent().ok_or_else(|| {
        install_error(
            InstallerErrorCode::ActivationFailed,
            format!(
                "managed binary path `{}` has no parent directory",
                target.display()
            ),
            "The managed installation path is invalid.",
            "Set SC_LINT_INSTALL_DIR to a writable directory and rerun `sc-lint setup`.",
        )
    })?;
    let backup = parent.join(format!(".{}-backup", ReleaseTarget::binary_name()));
    if backup.exists() {
        file_ops
            .remove_file(&backup)
            .map_err(|error| permission_error(&backup, error))?;
    }
    let had_previous = target.exists();
    if had_previous {
        file_ops
            .rename(target, &backup)
            .map_err(|error| activation_error(target, error))?;
    }
    if let Err(error) = file_ops.rename(candidate, target) {
        if had_previous && let Err(rollback_error) = file_ops.rename(&backup, target) {
            return Err(rollback_failure_error(
                target,
                &backup,
                rollback_error,
                Some(error),
            ));
        }
        return Err(activation_error(target, error));
    }
    let verification = probe(target)
        .map_err(|cause| PostInstallVerificationFailure {
            cause,
            observed_version: None,
        })
        .and_then(|version| {
            (version >= *minimum.as_semver())
                .then_some(version.clone())
                .ok_or_else(|| PostInstallVerificationFailure {
                    cause: format!("activated version `{version}` is below required `{minimum}`"),
                    observed_version: Some(version),
                })
        });
    let version = match verification {
        Ok(version) => version,
        Err(failure) => {
            if let Err(rollback_error) = rollback_candidate(target, &backup, had_previous, file_ops)
            {
                return Err(
                    rollback_failure_error(target, &backup, rollback_error, None)
                        .with_detail("post_install_verification_cause", json!(failure.cause)),
                );
            }
            return Err(post_install_verification_error(
                target,
                minimum,
                had_previous,
                failure,
            ));
        }
    };
    if had_previous {
        let _ = file_ops.remove_file(&backup);
    }
    Ok(version)
}

fn rollback_candidate<F: InstallerFileOps>(
    target: &Path,
    backup: &Path,
    had_previous: bool,
    file_ops: &F,
) -> io::Result<()> {
    file_ops.remove_file(target)?;
    if had_previous {
        file_ops.rename(backup, target)?;
    }
    Ok(())
}

fn post_install_verification_error(
    target: &Path,
    minimum: &MinimumVersion,
    had_previous: bool,
    failure: PostInstallVerificationFailure,
) -> CliError {
    let recovery = if had_previous {
        "The previous managed installation was restored. Repair the release source and rerun `sc-lint setup`."
    } else {
        "The failed candidate was removed. Repair the release source and rerun `sc-lint setup`."
    };
    let mut error = install_error(
        InstallerErrorCode::PostInstallVersionFailed,
        format!(
            "activated sc-lint binary `{}` failed post-install verification",
            target.display()
        ),
        failure.cause,
        recovery,
    )
    .with_detail("managed_binary", json!(target.display().to_string()))
    .with_detail("minimum_version", json!(minimum.to_string()))
    .with_detail("had_previous_installation", json!(had_previous));
    if let Some(observed_version) = failure.observed_version {
        error = error.with_detail("observed_version", json!(observed_version.to_string()));
    }
    error
}

fn rollback_failure_error(
    target: &Path,
    backup: &Path,
    rollback_error: io::Error,
    activation_error: Option<io::Error>,
) -> CliError {
    let mut error = install_error(
        InstallerErrorCode::RollbackFailed,
        format!(
            "could not verify rollback of managed sc-lint binary `{}`",
            target.display()
        ),
        rollback_error.to_string(),
        format!(
            "Inspect `{}` and `{}`; recover the known-good backup manually, then rerun `sc-lint setup`.",
            target.display(),
            backup.display()
        ),
    )
    .with_detail("managed_binary", json!(target.display().to_string()))
    .with_detail("backup_path", json!(backup.display().to_string()))
    .with_detail(
        "rollback_error_kind",
        json!(format!("{:?}", rollback_error.kind())),
    );
    if let Some(activation_error) = activation_error {
        error = error.with_detail("activation_error", json!(activation_error.to_string()));
    }
    error
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

fn probe_version(binary: &Path) -> Result<Version, String> {
    if !binary.is_file() {
        return Err("binary was not found".to_string());
    }
    let output = Command::new(binary)
        .args(["--json", "version"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("version probe exited with {}", output.status));
    }
    let envelope: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let data = envelope
        .get("data")
        .ok_or_else(|| "version probe did not include data".to_string())?;
    if envelope.get("ok").and_then(Value::as_bool) != Some(true)
        || envelope.get("command").and_then(Value::as_str) != Some("version")
        || data.get("tool").and_then(Value::as_str) != Some(crate::consts::SERVICE_NAME)
        || data.get("contract_schema").and_then(Value::as_str) != Some(VERSION_PROBE_SCHEMA)
    {
        return Err("version probe did not implement sc-lint-version-v1".to_string());
    }
    let version = data
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "version probe did not include a version".to_string())?;
    Version::parse(version).map_err(|error| error.to_string())
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
    #[cfg(unix)]
    use clap::Parser;
    #[cfg(unix)]
    use serial_test::serial;
    #[cfg(unix)]
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::io::Write;
    use std::str::FromStr;

    use tempfile::TempDir;

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

    #[test]
    fn consumer_bootstrap_exposes_only_product_owned_operations() {
        for operation in ["ensure", "setup", "upgrade"] {
            assert!(
                CONSUMER_BOOTSTRAP_ASSET.contains(operation),
                "bootstrap does not expose {operation}"
            );
        }
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("compatibility check"));
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains("--config"));
        assert!(!CONSUMER_BOOTSTRAP_ASSET.contains("cargo run"));
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
        fs::create_dir(&payload_dir).expect("payload directory");
        fs::create_dir(&install_dir).expect("managed directory");
        write_probe(&payload_dir.join(ReleaseTarget::binary_name()), "0.4.1");
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(ReleaseTarget::binary_name())
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
    fn failed_post_install_probe_restores_the_previous_binary() {
        let fixture = TempDir::new().expect("fixture");
        let target = fixture.path().join("sc-lint");
        let candidate = fixture.path().join("candidate");
        write_probe(&target, "0.4.1");
        write_probe(&candidate, "0.3.0");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");

        let error = activate_candidate(&candidate, &target, &minimum)
            .expect_err("old candidate rolls back");
        assert_eq!(error.code(), "CLI.SC_LINT_POST_INSTALL_VERSION_FAILED");
        assert_eq!(
            probe_version(&target).expect("old binary restored"),
            Version::parse("0.4.1").expect("version")
        );
    }

    #[test]
    fn rollback_failure_never_claims_the_previous_binary_was_restored() {
        let fixture = TempDir::new().expect("fixture");
        let target = fixture.path().join("sc-lint");
        let candidate = fixture.path().join("candidate");
        fs::write(&target, "known-good").expect("target");
        fs::write(&candidate, "candidate").expect("candidate");
        let minimum = MinimumVersion::from_str("0.4.1").expect("minimum");
        let file_ops = FailRollbackRemove {
            target: target.clone(),
        };

        let error = activate_candidate_with(&candidate, &target, &minimum, &file_ops, |_| {
            Err("post-install version probe rejected candidate".to_string())
        })
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
                    .join(format!(".{}-backup", ReleaseTarget::binary_name()))
                    .display()
                    .to_string()
            ))
        );
        assert!(
            fixture
                .path()
                .join(format!(".{}-backup", ReleaseTarget::binary_name()))
                .is_file()
        );
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

        let missing = execute_install_command(&config_path, "setup");
        assert_eq!(missing["status"], "installed");
        assert_eq!(missing["selected_release_version"], "0.4.1");
        assert_eq!(
            fs::read_to_string(&consumer_readme).expect("readme"),
            "consumer-owned README\n"
        );

        write_probe(&install_dir.join(ReleaseTarget::binary_name()), "0.4.0");
        let old = execute_install_command(&config_path, "upgrade");
        assert_eq!(old["status"], "upgraded");
        assert_eq!(old["installed_version"], "0.4.1");

        let current = execute_install_command(&config_path, "setup");
        assert_eq!(current["status"], "current");
        assert_eq!(current["installed_version"], "0.4.1");

        write_probe(&install_dir.join(ReleaseTarget::binary_name()), "0.5.0");
        let newer = execute_install_command(&config_path, "upgrade");
        assert_eq!(newer["status"], "current");
        assert_eq!(newer["installed_version"], "0.5.0");
        assert_eq!(
            fs::read_to_string(&consumer_readme).expect("readme"),
            "consumer-owned README\n"
        );
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

    #[cfg(unix)]
    struct InstallerEnvironment {
        original: Vec<(&'static str, Option<OsString>)>,
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
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

    #[cfg(unix)]
    fn execute_install_command(config_path: &Path, command: &str) -> Value {
        let cli = crate::Cli::parse_from([
            "sc-lint",
            "--config",
            config_path.to_str().expect("UTF-8 config path"),
            command,
        ]);
        let context = crate::CommandContext::from_cli(&cli).expect("command context");
        let loaded = crate::LoadedConfig::load(&cli, &context).expect("consumer config");
        crate::command::execute(&context, &loaded)
            .expect("installer command")
            .data
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
        fs::create_dir(&payload_dir).expect("payload directory");
        write_probe(&payload_dir.join(ReleaseTarget::binary_name()), version);
        let archive = release_dir.join(&archive_name);
        let tar_status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload_dir)
            .arg(ReleaseTarget::binary_name())
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
    fn write_probe(path: &Path, version: &str) {
        use std::os::unix::fs::PermissionsExt;

        let mut file = fs::File::create(path).expect("probe script");
        writeln!(
            file,
            "#!/bin/sh\nprintf '%s\\n' '{{\"ok\":true,\"command\":\"version\",\"data\":{{\"tool\":\"sc-lint\",\"version\":\"{version}\",\"contract_schema\":\"sc-lint-version-v1\"}}}}'"
        )
        .expect("script text");
        let mut permissions = file.metadata().expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("executable");
    }
}
