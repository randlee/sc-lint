"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const zlib = require("node:zlib");

const { ERROR, runAction } = require("../index.js");

function temporaryDirectory() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "sc-lint-action-test-"));
}

function tarGz(entries) {
  const blocks = [];
  for (const [name, contents] of entries) {
    const data = Buffer.from(contents);
    const header = Buffer.alloc(512);
    header.write(name);
    header.write("0000755\0", 100);
    header.write(data.length.toString(8).padStart(11, "0") + "\0", 124);
    header.write("0", 156);
    header.write("ustar\0", 257);
    for (let index = 148; index < 156; index += 1) header[index] = 32;
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    header.write(checksum.toString(8).padStart(6, "0") + "\0 ", 148);
    blocks.push(header, data, Buffer.alloc((512 - (data.length % 512)) % 512));
  }
  return zlib.gzipSync(Buffer.concat([...blocks, Buffer.alloc(1024)]));
}

function zip(entries) {
  const blocks = [];
  for (const [name, contents] of entries) {
    const data = Buffer.from(contents);
    const fileName = Buffer.from(name);
    const header = Buffer.alloc(30);
    header.writeUInt32LE(0x04034b50, 0);
    header.writeUInt16LE(20, 4);
    header.writeUInt32LE(data.length, 18);
    header.writeUInt32LE(data.length, 22);
    header.writeUInt16LE(fileName.length, 26);
    blocks.push(header, fileName, data);
  }
  return Buffer.concat(blocks);
}

function fixture(platform, architecture, operation, layout = "nested") {
  const target = {
    "linux/x64": ["x86_64-unknown-linux-gnu", "tar.gz"],
    "darwin/x64": ["x86_64-apple-darwin", "tar.gz"],
    "darwin/arm64": ["aarch64-apple-darwin", "tar.gz"],
    "win32/x64": ["x86_64-pc-windows-msvc", "zip"],
  }[`${platform}/${architecture}`];
  const binary = platform === "win32" ? "sc-lint.exe" : "sc-lint";
  const archiveRoot = `sc-lint_0.4.0_${target[0]}`;
  const entry = (name) => layout === "nested" ? `${archiveRoot}/${name}` : name;
  const binaryPath = layout === "nested" ? entry(`bin/${binary}`) : binary;
  const docsPath = layout === "nested"
    ? entry("share/sc-lint/sc-lint-docs/README.md")
    : "sc-lint-docs/README.md";
  const entries = [[binaryPath, "fixture binary"], [docsPath, "offline guide"]];
  const archive = target[1] === "zip" ? zip(entries) : tarGz(entries);
  const archiveName = `sc-lint_0.4.0_${target[0]}.${target[1]}`;
  const checksums = Buffer.from(`${crypto.createHash("sha256").update(archive).digest("hex")}  ${archiveName}\n`);
  const root = temporaryDirectory();
  const output = path.join(root, "output");
  const runnerPath = path.join(root, "path");
  fs.writeFileSync(output, "");
  fs.writeFileSync(runnerPath, "");
  const calls = [];
  return {
    archive,
    checksums,
    calls,
    options: {
      platform,
      arch: architecture,
      installRoot: path.join(root, "installed"),
      env: { GITHUB_OUTPUT: output, GITHUB_PATH: runnerPath, RUNNER_TEMP: root },
      inputs: { operation, version: "0.4.0", configPath: "sc-lint.toml", workingDirectory: root, artifactUrl: "https://fixture/archive", checksumsUrl: "https://fixture/checksums", releaseBaseUrl: "https://fixture" },
      download: async (url) => url.endsWith("archive") ? archive : checksums,
      execute(binaryPath, args, options) {
        calls.push({ binaryPath, args, options });
        return { status: 0, stdout: args[0] === "docs" ? path.join(path.dirname(binaryPath), "sc-lint-docs") : "ok", stderr: "" };
      },
    },
    output,
    runnerPath,
  };
}

test("runner-shaped hyphenated inputs drive action execution", async () => {
  const local = fixture("linux", "x64", "lint");
  const cwd = path.join(path.dirname(local.options.installRoot), "consumer");
  delete local.options.inputs;
  local.options.env = {
    ...local.options.env,
    "INPUT_OPERATION": "lint",
    "INPUT_VERSION": "0.4.0",
    "INPUT_WORKING-DIRECTORY": cwd,
    "INPUT_CONFIG-PATH": "consumer.toml",
    "INPUT_ARTIFACT-URL": "https://fixture/archive",
    "INPUT_CHECKSUMS-URL": "https://fixture/checksums",
  };
  await runAction(local.options);
  assert.equal(local.calls[0].options.cwd, cwd);
  assert.deepEqual(local.calls[0].args.slice(0, 2), ["--config", "consumer.toml"]);
  assert.deepEqual(local.calls[2].args.slice(-2), ["--config", "consumer.toml"]);
});

test("release-base-url builds artifact URLs when explicit URLs are absent", async () => {
  const local = fixture("linux", "x64", "setup");
  const urls = [];
  delete local.options.inputs;
  local.options.env = { ...local.options.env, "INPUT_OPERATION": "setup", "INPUT_VERSION": "0.4.0", "INPUT_RELEASE-BASE-URL": "https://mirror/releases/" };
  local.options.download = async (url) => {
    urls.push(url);
    return url.endsWith("checksums.txt") ? local.checksums : local.archive;
  };
  await runAction(local.options);
  assert.deepEqual(urls, ["https://mirror/releases/v0.4.0/sc-lint_0.4.0_x86_64-unknown-linux-gnu.tar.gz", "https://mirror/releases/v0.4.0/checksums.txt"]);
});

test("underscore-spelled input keys drive action execution", async () => {
  const local = fixture("linux", "x64", "lint");
  const cwd = path.join(path.dirname(local.options.installRoot), "consumer");
  const urls = [];
  delete local.options.inputs;
  local.options.env = {
    ...local.options.env,
    "INPUT_OPERATION": "lint",
    "INPUT_VERSION": "0.4.0",
    "INPUT_WORKING_DIRECTORY": cwd,
    "INPUT_CONFIG_PATH": "consumer.toml",
    "INPUT_RELEASE_BASE_URL": "https://underscore-mirror/releases/",
  };
  local.options.download = async (url) => {
    urls.push(url);
    return url.endsWith("checksums.txt") ? local.checksums : local.archive;
  };
  await runAction(local.options);
  assert.equal(local.calls[0].options.cwd, cwd);
  assert.deepEqual(local.calls[0].args.slice(0, 2), ["--config", "consumer.toml"]);
  assert.deepEqual(urls, ["https://underscore-mirror/releases/v0.4.0/sc-lint_0.4.0_x86_64-unknown-linux-gnu.tar.gz", "https://underscore-mirror/releases/v0.4.0/checksums.txt"]);
});

test("underscore-spelled explicit URLs are downloaded as given", async () => {
  const local = fixture("linux", "x64", "setup");
  const urls = [];
  delete local.options.inputs;
  local.options.env = { ...local.options.env, "INPUT_OPERATION": "setup", "INPUT_VERSION": "0.4.0", "INPUT_ARTIFACT_URL": "https://explicit/archive.tar.gz", "INPUT_CHECKSUMS_URL": "https://explicit/sums.txt" };
  local.options.download = async (url) => {
    urls.push(url);
    return url.endsWith("sums.txt") ? local.checksums : local.archive;
  };
  await runAction(local.options);
  assert.deepEqual(urls, ["https://explicit/archive.tar.gz", "https://explicit/sums.txt"]);
});

test("explicit artifact and checksum URLs override release-base-url", async () => {
  const local = fixture("linux", "x64", "setup");
  const urls = [];
  delete local.options.inputs;
  local.options.env = { ...local.options.env, "INPUT_OPERATION": "setup", "INPUT_VERSION": "0.4.0", "INPUT_RELEASE-BASE-URL": "https://mirror/releases", "INPUT_ARTIFACT-URL": "https://explicit/archive.tar.gz", "INPUT_CHECKSUMS-URL": "https://explicit/sums.txt" };
  local.options.download = async (url) => {
    urls.push(url);
    return url.endsWith("sums.txt") ? local.checksums : local.archive;
  };
  await runAction(local.options);
  assert.deepEqual(urls, ["https://explicit/archive.tar.gz", "https://explicit/sums.txt"]);
});

test("legacy flat archive remains accepted", async () => {
  const local = fixture("linux", "x64", "setup", "flat");
  const result = await runAction(local.options);
  assert.equal(result.binaryPath, path.join(local.options.installRoot, "sc-lint"));
  assert.equal(result.docsPath, path.join(local.options.installRoot, "sc-lint-docs"));
});

for (const [platform, architecture] of [["linux", "x64"], ["darwin", "x64"], ["win32", "x64"]]) {
  for (const operation of ["setup", "lint", "test"]) {
    test(`published-layout ${platform}/${architecture} fixture runs ${operation}`, async () => {
      const local = fixture(platform, architecture, operation);
      const result = await runAction(local.options);
      assert.equal(result.version, "0.4.0");
      assert.ok(fs.existsSync(result.binaryPath));
      assert.ok(fs.existsSync(result.docsPath));
      assert.deepEqual(local.calls[0].args.slice(0, 3), ["--config", "sc-lint.toml", "compatibility"]);
      assert.deepEqual(local.calls[1].args, ["docs", "--path"]);
      if (operation === "setup") assert.equal(local.calls.length, 2);
      if (operation === "lint") assert.deepEqual(local.calls[2].args.slice(0, 3), ["lint", "ci", "--consumer"]);
      if (operation === "test") assert.deepEqual(local.calls[2].args.slice(0, 1), ["test"]);
      assert.match(fs.readFileSync(local.output, "utf8"), /binary-path=.*\ndocs-path=.*sc-lint-docs\nversion=0\.4\.0/);
      assert.match(fs.readFileSync(local.runnerPath, "utf8"), /installed/);
    });
  }
}

test("checksum mismatch has stable recovery code", async () => {
  const local = fixture("linux", "x64", "setup");
  local.checksums = Buffer.from(`0000000000000000000000000000000000000000000000000000000000000000  sc-lint_0.4.0_x86_64-unknown-linux-gnu.tar.gz\n`);
  local.options.download = async (url) => url.endsWith("archive") ? local.archive : local.checksums;
  await assert.rejects(() => runAction(local.options), (error) => error.code === ERROR.checksum && error.recovery.includes("Discard"));
});

test("unavailable artifact, incompatible minimum, and command failure are distinct", async () => {
  const unavailable = fixture("linux", "x64", "setup");
  unavailable.options.download = async () => { throw new Error("offline"); };
  await assert.rejects(() => runAction(unavailable.options), (error) => error.code === ERROR.artifact && error.recovery.includes("mirror"));

  const incompatible = fixture("linux", "x64", "setup");
  incompatible.options.execute = () => ({ status: 7, stdout: "", stderr: "minimum not met" });
  await assert.rejects(() => runAction(incompatible.options), (error) => error.code === ERROR.compatibility && error.recovery.includes("minimum_version"));

  const failed = fixture("linux", "x64", "lint");
  failed.options.execute = (_binary, args) => ({ status: args[0] === "lint" ? 9 : 0, stdout: "", stderr: "profile failure" });
  await assert.rejects(() => runAction(failed.options), (error) => error.code === ERROR.command && error.recovery.includes("consumer profile"));
});

test("version input is an exact SemVer rather than a release-path fragment", async () => {
  const local = fixture("linux", "x64", "setup");
  local.options.inputs.version = "../untrusted";
  await assert.rejects(() => runAction(local.options), (error) => error.code === ERROR.artifact && error.message.includes("not SemVer"));
});

test("action source has no source or package fallback", () => {
  const source = fs.readFileSync(path.join(__dirname, "..", "index.js"), "utf8");
  assert.doesNotMatch(source, /cargo\s+run|sc-lint-boundary|sc-lint-portability|sc-lint-runtime/);
});

test("metadata declares the stable interface and required output contract", () => {
  const metadata = fs.readFileSync(path.join(__dirname, "..", "..", "action.yml"), "utf8");
  for (const name of ["operation:", "version:", "config-path:", "artifact-url:", "checksums-url:", "binary-path:", "docs-path:"]) {
    assert.match(metadata, new RegExp(`^  ${name}`, "m"));
  }
  assert.match(metadata, /using: node20/);
  assert.match(metadata, /main: action\/index\.js/);
});
