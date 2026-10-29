#!/usr/bin/env bash
# Run CI-built Linux ARM artifacts on the existing Colima ARM VM.
set -euo pipefail
archive="$(realpath "$1")"
wheel="$(realpath "$2")"
archive_image=debian:13.4
wheel_image=python:3.9-slim
archive_was_present=true
wheel_was_present=true
cleanup() {
    if [[ "$archive_was_present" == false ]] && docker --context colima image inspect "$archive_image" >/dev/null 2>&1; then docker --context colima image rm "$archive_image"; fi
    if [[ "$wheel_was_present" == false ]] && docker --context colima image inspect "$wheel_image" >/dev/null 2>&1; then docker --context colima image rm "$wheel_image"; fi
    colima stop
    atm send fenix@atm-dev 'Finished lint-leim Colima validation; default profile stopped.'
}
trap cleanup EXIT
atm send fenix@atm-dev 'Starting Colima default profile for lint-leim Linux ARM release validation.'
colima start
docker --context colima image inspect "$archive_image" >/dev/null 2>&1 || archive_was_present=false
docker --context colima image inspect "$wheel_image" >/dev/null 2>&1 || wheel_was_present=false
docker --context colima run --rm --platform linux/arm64 --mount "type=bind,src=$archive,dst=/release.tar.gz,readonly" "$archive_image" sh -eu -c '
  apt-get update -qq && apt-get install -y -qq cargo >/dev/null
  mkdir -p /release /fixture/src
  tar -xzf /release.tar.gz -C /release --strip-components=1
  printf "[workspace]\nmembers = [\".\"]\nresolver = \"2\"\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2021\"\nauthors = [\"Fixture\"]\nlicense = \"MIT\"\nrust-version = \"1.80\"\nhomepage = \"https://example.com\"\nrepository = \"https://example.com\"\n[package]\nname = \"fixture\"\nversion.workspace = true\nedition.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrust-version.workspace = true\nhomepage.workspace = true\nrepository.workspace = true\n" > /fixture/Cargo.toml
  printf "pub fn fixture() -> u32 { 42 }\n" > /fixture/src/lib.rs
  printf "[tool.sc-lint.analyzers.boundary]\n[tool.sc-lint.analyzers.portability]\n[tool.sc-lint.analyzers.runtime]\n" > /fixture/sc-lint.toml
  export PATH=/release/bin:$PATH
  for bin in sc-lint sc-lint-boundary sc-lint-portability sc-lint-runtime; do "$bin" --version; done
  for bin in sc-lint-boundary sc-lint-portability sc-lint-runtime; do
    output=$("$bin" analyze --root /fixture --config /fixture/sc-lint.toml)
    printf "%s\n" "$output"
    printf "%s\n" "$output" | grep -q "status=pass"
  done
  sc-lint --root /fixture --config /fixture/sc-lint.toml lint sc-portability
'
docker --context colima run --rm --platform linux/arm64 --mount "type=bind,src=$wheel,dst=/release/$(basename "$wheel"),readonly" "$wheel_image" sh -eu -c '
  python -m pip install /release/*.whl
  python -c "import sc_lint; print(sc_lint.version_json())"
'
