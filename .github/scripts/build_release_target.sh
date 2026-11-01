#!/usr/bin/env bash
# One build recipe shared by stable and prerelease workflows.
set -euo pipefail

case "$1" in
  binaries)
    bins="$(python3 .github/scripts/release_artifacts.py cargo-build-bin-args --manifest "$RELEASE_ARTIFACT_MANIFEST")"
    if [[ "$BUILD_TARGET" == 'aarch64-unknown-linux-gnu' ]]; then
      build_venv="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/release-zigbuild.XXXXXXXX")"
      python3 -m venv "$build_venv"
      "$build_venv/bin/python" -m pip install cargo-zigbuild==0.20.1 ziglang==0.12.1
      export PATH="$build_venv/bin:$PATH"
      cargo zigbuild --release --target aarch64-unknown-linux-gnu.2.28 $bins
    else
      cargo build --release --target "$BUILD_TARGET" $bins
    fi
    ;;
  wheel)
    [[ -n "${MACOSX_DEPLOYMENT_TARGET:-}" ]] || unset MACOSX_DEPLOYMENT_TARGET
    if [[ "$BUILD_SYSTEM" == 'setuptools' ]]; then
      python -m pip install build==1.3.0
      python -m build --wheel --outdir dist "$BUILD_SOURCE"
    elif [[ -z "$BUILD_TARGET" ]]; then
      args=(--release --manifest-path "$BUILD_CARGO_MANIFEST" --out dist)
      if [[ "$BUILD_RUNNER" == 'ubuntu-24.04-arm' ]]; then
        args+=(--compatibility manylinux2014 --zig)
      fi
      maturin build "${args[@]}"
    else
      rustup target add "$BUILD_TARGET"
      args=(--release --locked --manifest-path "$BUILD_CARGO_MANIFEST" --out dist --target "$BUILD_TARGET")
      if [[ "$BUILD_TARGET" == 'aarch64-pc-windows-msvc' ]]; then
        export PYO3_CROSS=1 PYO3_CROSS_PYTHON_VERSION=3.11
        args+=(--features pyo3/generate-import-lib)
      fi
      if [[ "$BUILD_MANYLINUX" != 'off' ]]; then
        args+=(--compatibility "manylinux_${BUILD_MANYLINUX}" --zig)
      fi
      maturin build "${args[@]}"
    fi
    ;;
  *) echo "expected binaries or wheel" >&2; exit 2 ;;
esac
