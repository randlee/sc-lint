"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { fileURLToPath } = require("node:url");
const { inflateRawSync, gunzipSync } = require("node:zlib");
const { spawnSync } = require("node:child_process");

const RELEASE_BASE_URL = "https://github.com/randlee/sc-lint/releases/download";
const ERROR = Object.freeze({
  artifact: "ACTION.SC_LINT_ARTIFACT_UNAVAILABLE",
  checksum: "ACTION.SC_LINT_CHECKSUM_MISMATCH",
  compatibility: "ACTION.SC_LINT_COMPATIBILITY_FAILED",
  command: "ACTION.SC_LINT_COMMAND_FAILED",
});

class ActionError extends Error {
  constructor(code, message, recovery) {
    super(message);
    this.code = code;
    this.recovery = recovery;
  }
}

function releaseTarget(platform, architecture) {
  const targets = {
    "linux/x64": ["x86_64-unknown-linux-gnu", "tar.gz"],
    "darwin/x64": ["x86_64-apple-darwin", "tar.gz"],
    "darwin/arm64": ["aarch64-apple-darwin", "tar.gz"],
    "win32/x64": ["x86_64-pc-windows-msvc", "zip"],
  };
  const found = targets[`${platform}/${architecture}`];
  if (!found) {
    throw new ActionError(
      ERROR.artifact,
      `No verified sc-lint release is published for ${platform}/${architecture}.`,
      "Use a supported Linux x86_64, macOS x86_64/aarch64, or Windows x86_64 runner.",
    );
  }
  return { triple: found[0], extension: found[1] };
}

function requireInput(inputs, name) {
  const value = inputs[name];
  if (!value || !value.trim()) {
    throw new ActionError(ERROR.artifact, `Action input \`${name}\` is required.`, `Set \`${name}\` to an exact released sc-lint version.`);
  }
  return value.trim();
}

function actionInputs(env) {
  const read = (name, fallback = "") => (env[`INPUT_${name.toUpperCase().replace(/-/g, "_")}`] || fallback).trim();
  return {
    operation: read("operation", "lint"),
    version: read("version"),
    configPath: read("config-path", "sc-lint.toml"),
    artifactUrl: read("artifact-url"),
    checksumsUrl: read("checksums-url"),
    releaseBaseUrl: read("release-base-url", RELEASE_BASE_URL).replace(/\/$/, ""),
    workingDirectory: read("working-directory", "."),
  };
}

async function downloadBytes(url) {
  try {
    if (url.startsWith("file:")) return fs.promises.readFile(fileURLToPath(url));
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return Buffer.from(await response.arrayBuffer());
  } catch (error) {
    throw new ActionError(
      ERROR.artifact,
      `Unable to obtain verified release input from ${url}: ${error.message}`,
      "Check the release version and mirror URL, or make the verified release available to this runner.",
    );
  }
}

function verifyChecksum(archive, checksums, archiveName) {
  const match = checksums.toString("utf8").match(new RegExp(`^([a-fA-F0-9]{64})\\s+[*]?${escapeRegExp(archiveName)}$`, "m"));
  if (!match) {
    throw new ActionError(ERROR.checksum, `checksums.txt has no SHA-256 entry for ${archiveName}.`, "Use the checksum manifest published with the selected release.");
  }
  const actual = crypto.createHash("sha256").update(archive).digest("hex");
  if (actual !== match[1].toLowerCase()) {
    throw new ActionError(ERROR.checksum, `SHA-256 verification failed for ${archiveName}.`, "Discard the archive and retry from the trusted release or mirror.");
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function writeEntry(root, entryName, contents, mode) {
  const destination = path.resolve(root, entryName);
  if (!destination.startsWith(`${path.resolve(root)}${path.sep}`)) {
    throw new ActionError(ERROR.artifact, `Release archive contains unsafe path ${entryName}.`, "Use an official unmodified sc-lint release archive.");
  }
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, contents);
  if (mode) fs.chmodSync(destination, mode & 0o777);
}

function extractTarGz(archive, destination) {
  const bytes = gunzipSync(archive);
  for (let offset = 0; offset + 512 <= bytes.length;) {
    const header = bytes.subarray(offset, offset + 512);
    if (header.every((byte) => byte === 0)) break;
    const name = header.subarray(0, 100).toString("utf8").replace(/\0.*$/, "");
    const prefix = header.subarray(345, 500).toString("utf8").replace(/\0.*$/, "");
    const size = Number.parseInt(header.subarray(124, 136).toString("utf8").replace(/\0.*$/, "").trim() || "0", 8);
    const mode = Number.parseInt(header.subarray(100, 108).toString("utf8").replace(/\0.*$/, "").trim() || "644", 8);
    const type = String.fromCharCode(header[156] || 48);
    const entry = prefix ? `${prefix}/${name}` : name;
    offset += 512;
    if (type === "5") {
      const directory = path.resolve(destination, entry);
      if (!directory.startsWith(`${path.resolve(destination)}${path.sep}`)) {
        throw new ActionError(ERROR.artifact, `Release archive contains unsafe path ${entry}.`, "Use an official unmodified sc-lint release archive.");
      }
      fs.mkdirSync(directory, { recursive: true });
    }
    else if (type === "0" || type === "\0") writeEntry(destination, entry, bytes.subarray(offset, offset + size), mode);
    offset += Math.ceil(size / 512) * 512;
  }
}

function extractZip(archive, destination) {
  let offset = 0;
  while (offset + 30 <= archive.length && archive.readUInt32LE(offset) === 0x04034b50) {
    const flags = archive.readUInt16LE(offset + 6);
    const method = archive.readUInt16LE(offset + 8);
    const compressedSize = archive.readUInt32LE(offset + 18);
    const uncompressedSize = archive.readUInt32LE(offset + 22);
    const nameLength = archive.readUInt16LE(offset + 26);
    const extraLength = archive.readUInt16LE(offset + 28);
    if (flags & 0x08) throw new ActionError(ERROR.artifact, "ZIP data-descriptor archives are unsupported.", "Use the official release archive.");
    const name = archive.subarray(offset + 30, offset + 30 + nameLength).toString("utf8");
    const start = offset + 30 + nameLength + extraLength;
    const payload = archive.subarray(start, start + compressedSize);
    const contents = method === 0 ? payload : method === 8 ? inflateRawSync(payload) : null;
    if (!contents || contents.length !== uncompressedSize) throw new ActionError(ERROR.artifact, `Unsupported or malformed ZIP entry ${name}.`, "Use the official release archive.");
    if (name.endsWith("/")) {
      const directory = path.resolve(destination, name);
      if (!directory.startsWith(`${path.resolve(destination)}${path.sep}`)) {
        throw new ActionError(ERROR.artifact, `Release archive contains unsafe path ${name}.`, "Use an official unmodified sc-lint release archive.");
      }
      fs.mkdirSync(directory, { recursive: true });
    }
    else writeEntry(destination, name, contents, 0o755);
    offset = start + compressedSize;
  }
}

function invoke(binary, args, cwd, execute) {
  const result = execute(binary, args, { cwd, encoding: "utf8" });
  if (result.error) return { status: 1, stderr: result.error.message };
  return { status: result.status ?? 1, stdout: result.stdout || "", stderr: result.stderr || "" };
}

function writeActionOutput(env, name, value) {
  if (env.GITHUB_OUTPUT) fs.appendFileSync(env.GITHUB_OUTPUT, `${name}=${value}\n`);
}

function addActionPath(env, value) {
  if (env.GITHUB_PATH) fs.appendFileSync(env.GITHUB_PATH, `${value}\n`);
}

async function runAction(options = {}) {
  const env = options.env || process.env;
  const inputs = options.inputs || actionInputs(env);
  if (!["setup", "lint", "test"].includes(inputs.operation)) {
    throw new ActionError(ERROR.command, `Unsupported operation ${inputs.operation}.`, "Set operation to setup, lint, or test.");
  }
  const version = requireInput(inputs, "version");
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new ActionError(ERROR.artifact, `Action input \`version\` is not SemVer: ${version}.`, "Set version to an exact released sc-lint SemVer without a leading v.");
  }
  const target = releaseTarget(options.platform || process.platform, options.arch || process.arch);
  const archiveName = `sc-lint_${version}_${target.triple}.${target.extension}`;
  const tag = `v${version}`;
  const archiveUrl = inputs.artifactUrl || `${inputs.releaseBaseUrl}/${tag}/${archiveName}`;
  const checksumsUrl = inputs.checksumsUrl || `${inputs.releaseBaseUrl}/${tag}/checksums.txt`;
  const fetchBytes = options.download || downloadBytes;
  let archive;
  let checksums;
  try {
    archive = await fetchBytes(archiveUrl);
    checksums = await fetchBytes(checksumsUrl);
  } catch (error) {
    if (error instanceof ActionError) throw error;
    throw new ActionError(ERROR.artifact, `Unable to obtain verified release input: ${error.message}`, "Check the release version and mirror URL, or make the verified release available to this runner.");
  }
  verifyChecksum(archive, checksums, archiveName);

  const installRoot = options.installRoot || fs.mkdtempSync(path.join(env.RUNNER_TEMP || os.tmpdir(), "sc-lint-action-"));
  fs.mkdirSync(installRoot, { recursive: true });
  if (target.extension === "tar.gz") extractTarGz(archive, installRoot);
  else extractZip(archive, installRoot);
  const binary = path.join(installRoot, options.platform === "win32" || (!options.platform && process.platform === "win32") ? "sc-lint.exe" : "sc-lint");
  const docs = path.join(installRoot, "sc-lint-docs");
  if (!fs.existsSync(binary) || !fs.existsSync(docs)) {
    throw new ActionError(ERROR.artifact, "Verified release layout is missing sc-lint or sc-lint-docs.", "Use an E.5 release archive with the complete published layout.");
  }

  const execute = options.execute || spawnSync;
  const cwd = path.resolve(inputs.workingDirectory);
  const compatibility = invoke(binary, ["--config", inputs.configPath, "compatibility", "check", "--binary", binary], cwd, execute);
  if (compatibility.status !== 0) {
    throw new ActionError(ERROR.compatibility, `Compatibility preflight failed: ${compatibility.stderr || compatibility.stdout}`.trim(), "Align sc-lint.toml minimum_version with the selected release or select a newer release.");
  }
  const docsCheck = invoke(binary, ["docs", "--path"], cwd, execute);
  if (docsCheck.status !== 0) {
    throw new ActionError(ERROR.command, `Offline documentation discovery failed: ${docsCheck.stderr || docsCheck.stdout}`.trim(), "Use the complete verified release archive containing sc-lint-docs.");
  }
  if (inputs.operation !== "setup") {
    const command = inputs.operation === "lint" ? ["lint", "ci", "--consumer", "--config", inputs.configPath] : ["test", "--config", inputs.configPath];
    const operation = invoke(binary, command, cwd, execute);
    if (operation.status !== 0) {
      throw new ActionError(ERROR.command, `Consumer ${inputs.operation} failed: ${operation.stderr || operation.stdout}`.trim(), "Inspect the command output, fix the consumer profile, then rerun the Action.");
    }
  }
  writeActionOutput(env, "binary-path", binary);
  writeActionOutput(env, "docs-path", docs);
  writeActionOutput(env, "version", version);
  addActionPath(env, path.dirname(binary));
  return { binaryPath: binary, docsPath: docs, version };
}

async function main() {
  try {
    await runAction();
  } catch (error) {
    const failure = error instanceof ActionError ? error : new ActionError(ERROR.command, error.message, "Inspect the Action logs and retry.");
    console.error(`${failure.code}: ${failure.message}\nRecovery: ${failure.recovery}`);
    process.exitCode = 1;
  }
}

if (require.main === module) void main();

module.exports = { ActionError, ERROR, extractTarGz, extractZip, releaseTarget, runAction, verifyChecksum };
