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

use semver::Version;
use serde_json::{Value, json};

use crate::CliError;
use crate::MinimumVersion;
use crate::config::{LoadedConfig, VERSION_PROBE_SCHEMA};

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
    ActivationFailed,
}

impl InstallerErrorCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "CLI.SC_LINT_INSTALL_UNSUPPORTED_PLATFORM",
            Self::ReleaseUnavailable => "CLI.SC_LINT_RELEASE_UNAVAILABLE",
            Self::ChecksumMismatch => "CLI.SC_LINT_RELEASE_CHECKSUM_MISMATCH",
            Self::PermissionDenied => "CLI.SC_LINT_INSTALL_PERMISSION_DENIED",
            Self::PostInstallVersionFailed => "CLI.SC_LINT_POST_INSTALL_VERSION_FAILED",
            Self::ActivationFailed => "CLI.SC_LINT_INSTALL_ACTIVATION_FAILED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ReleaseTarget {
    triple: &'static str,
    archive_extension: &'static str,
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

    fn archive_name(self, version: &MinimumVersion) -> String {
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
    let target = ReleaseTarget::detect()?;
    let install_dir = managed_install_dir()?;
    let managed_binary = install_dir.join(ReleaseTarget::binary_name());
    let had_previous_installation = std::iter::once(managed_binary.clone())
        .chain(path_binary_candidates())
        .any(|binary| binary.is_file());
    let existing = find_compatible_binary(&managed_binary, minimum_version);

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

    let archive_name = target.archive_name(minimum_version);
    let archive_url = release_url(minimum_version, &archive_name);
    let checksums_url = release_url(minimum_version, "checksums.txt");
    if check_only || dry_run {
        return Ok(json!({
            "status": if check_only { "update_required" } else { "dry_run" },
            "action": if check_only { "check" } else { "install" },
            "minimum_version": minimum_version.to_string(),
            "archive": archive_name,
            "release_url": archive_url,
            "checksums_url": checksums_url,
            "install_dir": install_dir.display().to_string(),
            "config_path": config_path.display().to_string(),
            "summary": "a verified sc-lint release would be installed",
        }));
    }

    let installed_version = install_verified_release(
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
    // `dirs` resolves XDG data directories on Unix and LocalAppData on Windows
    // without coupling this product boundary to platform-specific environment keys.
    if let Some(data_dir) = dirs::data_dir() {
        return Ok(data_dir.join("sc-lint").join("bin"));
    }
    dirs::home_dir()
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
        })
}

fn release_url(version: &MinimumVersion, filename: &str) -> String {
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
) -> Option<(PathBuf, Version)> {
    std::iter::once(managed_binary.to_path_buf())
        .chain(path_binary_candidates())
        .find_map(|binary| {
            let version = probe_version(&binary).ok()?;
            (version >= *minimum.as_semver()).then_some((binary, version))
        })
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
    minimum_version: &MinimumVersion,
    target: ReleaseTarget,
    managed_binary: &Path,
    archive_url: &str,
    checksums_url: &str,
    staging: &Path,
) -> Result<Version, CliError> {
    let archive_name = target.archive_name(minimum_version);
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
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("could not retrieve verified sc-lint release data from `{url}`"),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Check network access or the release version, then rerun `sc-lint setup`.",
        ))
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
        ));
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

fn sha256(path: &Path) -> Result<String, CliError> {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("shasum", vec!["-a", "256"])
    } else if cfg!(windows) {
        ("certutil", vec!["-hashfile"])
    } else {
        ("sha256sum", Vec::new())
    };
    let mut command = Command::new(program);
    command.args(args).arg(path);
    if cfg!(windows) {
        command.arg("SHA256");
    }
    let output = command.output().map_err(|error| {
        install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("could not start `{program}` to verify the release checksum"),
            error.to_string(),
            "Install a SHA-256 utility and rerun `sc-lint setup`.",
        )
    })?;
    if !output.status.success() {
        return Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("`{program}` failed while verifying `{}`", path.display()),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Repair the local SHA-256 utility and rerun `sc-lint setup`.",
        ));
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
                    "`{program}` did not return a SHA-256 digest for `{}`",
                    path.display()
                ),
                output.trim().to_string(),
                "Repair the local SHA-256 utility and rerun `sc-lint setup`.",
            )
        })
}

fn extract_archive(archive: &Path, destination: &Path) -> Result<(), CliError> {
    let output = Command::new("tar")
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
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(install_error(
            InstallerErrorCode::ReleaseUnavailable,
            format!("could not extract release archive `{}`", archive.display()),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "Download a complete release archive and rerun `sc-lint setup`.",
        ))
    }
}

fn activate_candidate(
    candidate: &Path,
    target: &Path,
    minimum: &MinimumVersion,
) -> Result<Version, CliError> {
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
        fs::remove_file(&backup).map_err(|error| permission_error(&backup, error))?;
    }
    let had_previous = target.exists();
    if had_previous {
        fs::rename(target, &backup).map_err(|error| activation_error(target, error))?;
    }
    if let Err(error) = fs::rename(candidate, target) {
        if had_previous {
            let _ = fs::rename(&backup, target);
        }
        return Err(activation_error(target, error));
    }
    let verification = probe_version(target).and_then(|version| {
        if version >= *minimum.as_semver() {
            Ok(version)
        } else {
            Err(format!(
                "activated version `{version}` is below required `{minimum}"
            ))
        }
    });
    let version = match verification {
        Ok(version) => version,
        Err(cause) => {
            let _ = fs::remove_file(target);
            if had_previous {
                let _ = fs::rename(&backup, target);
            }
            return Err(install_error(
                InstallerErrorCode::PostInstallVersionFailed,
                format!(
                    "activated sc-lint binary `{}` failed post-install verification",
                    target.display()
                ),
                cause,
                "The previous managed installation was restored. Repair the release source and rerun `sc-lint setup`.",
            ));
        }
    };
    if had_previous {
        let _ = fs::remove_file(&backup);
    }
    Ok(version)
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
    use std::io::Write;
    #[cfg(unix)]
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
        let archive_name = release_target.archive_name(&minimum);
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
