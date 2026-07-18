#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly DOCKERFILE="${REPOSITORY_ROOT}/Dockerfile"
readonly DOCKERIGNORE="${REPOSITORY_ROOT}/.dockerignore"
readonly BUILD_BASE="rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc"
readonly RUNTIME_BASE="postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296"

fail() {
  printf 'Dockerfile responsibility gate failed: %s\n' "$1" >&2
  exit 1
}

assert_contains() {
  local file_path="$1"
  local expected="$2"

  grep --quiet --fixed-strings -- "${expected}" "${file_path}" || \
    fail "${file_path#"${REPOSITORY_ROOT}/"} omitted required production input: ${expected}"
}

assert_absent() {
  local file_path="$1"
  local forbidden="$2"

  if grep --ignore-case --quiet --fixed-strings -- "${forbidden}" "${file_path}"; then
    fail "${file_path#"${REPOSITORY_ROOT}/"} contains forbidden CI/test responsibility: ${forbidden}"
  fi
}

test -f "${DOCKERFILE}" && test ! -L "${DOCKERFILE}" || \
  fail "Dockerfile is unavailable"
test -f "${DOCKERIGNORE}" && test ! -L "${DOCKERIGNORE}" || \
  fail ".dockerignore is unavailable"

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT
readonly INSTRUCTIONS="${TEMP_ROOT}/instructions"

awk '
  function trim(value) {
    sub(/^[ \t]+/, "", value)
    sub(/[ \t\r]+$/, "", value)
    return value
  }
  function finish_instruction() {
    instruction = trim(instruction)
    gsub(/[ \t]+/, " ", instruction)
    print instruction
    instruction = ""
  }
  {
    current = trim($0)
    if (current == "" || substr(current, 1, 1) == "#") {
      next
    }
    continued = sub(/\\[ \t]*$/, "", current)
    if (instruction == "") {
      instruction = current
    } else {
      instruction = instruction " " current
    }
    if (!continued) {
      finish_instruction()
    }
  }
  END {
    if (instruction != "") {
      exit 1
    }
  }
' "${DOCKERFILE}" >"${INSTRUCTIONS}" || fail "Dockerfile instructions could not be parsed"

test "$(grep --count '^FROM ' "${INSTRUCTIONS}")" -eq 2 || \
  fail "Dockerfile must contain exactly one production build stage and one runtime stage"
test "$(grep '^FROM ' "${INSTRUCTIONS}" | sed -n '1p')" = \
  "FROM ${BUILD_BASE} AS build" || fail "production build stage is not pinned"
test "$(grep '^FROM ' "${INSTRUCTIONS}" | sed -n '2p')" = \
  "FROM ${RUNTIME_BASE}" || fail "official PostgreSQL runtime stage is not pinned"

readonly FINAL_STAGE="${TEMP_ROOT}/final-stage"
awk -v expected="FROM ${RUNTIME_BASE}" '
  $0 == expected { final = 1 }
  final { print }
' "${INSTRUCTIONS}" >"${FINAL_STAGE}"

cat >"${TEMP_ROOT}/expected-final-stage" <<EOF
FROM ${RUNTIME_BASE}
COPY --from=build --chown=0:0 --chmod=0644 /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm.so
COPY --from=build --chown=0:0 --chmod=0644 /src/LICENSE /usr/share/doc/pggomtm/LICENSE
COPY --from=build --chown=0:0 --chmod=0644 /tmp/pggomtm-build-manifest.json /usr/share/doc/pggomtm/build-manifest.json
EOF
cmp --silent -- "${FINAL_STAGE}" "${TEMP_ROOT}/expected-final-stage" || \
  fail "runtime stage must only add the production module, MIT license, and build metadata"

for required in \
  'ARG SOURCE_REVISION' \
  'rustup toolchain install 1.97.1' \
  'postgresql-18.4.tar.bz2' \
  '81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094' \
  "make -j\"\${build_jobs}\"" \
  'COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml LICENSE ./' \
  'COPY scripts/build-metadata ./scripts/build-metadata' \
  'COPY src ./src' \
  'cargo build --locked --release --lib --no-default-features --features pg18' \
  'scripts/build-metadata create'; do
  assert_contains "${DOCKERFILE}" "${required}"
done

for forbidden in \
  '.github' \
  'deny.toml' \
  'gitleaks.toml' \
  'COPY examples' \
  'COPY tests' \
  'fixture' \
  'scripts/artifact-readiness' \
  'scripts/native-test' \
  'scripts/public-readiness' \
  'cargo deny' \
  'cargo test' \
  'cargo fmt' \
  'cargo clippy' \
  'shellcheck' \
  'gitleaks' \
  'initdb' \
  'pg_ctl' \
  'psql' \
  'gate-passed' \
  'snapshot-filesystem' \
  'ENTRYPOINT' \
  'CMD [' \
  'VOLUME' \
  'STOPSIGNAL' \
  'USER '; do
  assert_absent "${DOCKERFILE}" "${forbidden}"
done

for required_context in \
  '!Cargo.toml' \
  '!Cargo.lock' \
  '!build.rs' \
  '!rust-toolchain.toml' \
  '!LICENSE' \
  '!scripts/build-metadata' \
  '!src/' \
  '!src/**'; do
  assert_contains "${DOCKERIGNORE}" "${required_context}"
done

for forbidden_context in \
  '!.github/' \
  '!deny.toml' \
  '!gitleaks.toml' \
  '!scripts/**' \
  '!examples/' \
  '!tests/'; do
  assert_absent "${DOCKERIGNORE}" "${forbidden_context}"
done

printf 'Dockerfile production responsibility policy passed\n'
