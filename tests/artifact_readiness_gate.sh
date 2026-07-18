#!/usr/bin/env bash
set -euo pipefail

umask 077

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly GATE="${REPOSITORY_ROOT}/scripts/artifact-readiness"
readonly LICENSE_FILE="${REPOSITORY_ROOT}/LICENSE"

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT

fail() {
  printf 'artifact-readiness gate test failed: %s\n' "$1" >&2
  exit 1
}

assert_contains() {
  local file_path="$1"
  local expected="$2"

  grep --quiet --fixed-strings -- "${expected}" "${file_path}" || \
    fail "${file_path#"${REPOSITORY_ROOT}/"} does not contain required artifact gate wiring"
}

expect_failure() {
  local reason="$1"
  shift

  if "$@" >"${TEMP_ROOT}/expected-failure.out" 2>&1; then
    fail "${reason}"
  fi
}

test -x "${GATE}" || fail "scripts/artifact-readiness is not executable"
test -r "${LICENSE_FILE}" || fail "repository MIT license is unavailable"
command -v jq >/dev/null || fail "jq is required by the artifact fixture"

readonly IDENTITY_ROOT="${TEMP_ROOT}/identity"
install -d "${IDENTITY_ROOT}/release/build/pggomtm-approved/out"
FIXTURE_SOURCE_SHA256="$(printf '%064d' 1)"
readonly FIXTURE_SOURCE_SHA256
FIXTURE_HEADER_SHA256="$(printf '%064d' 2)"
readonly FIXTURE_HEADER_SHA256
FIXTURE_BINDINGS_SHA256="$(printf '%064d' 3)"
readonly FIXTURE_BINDINGS_SHA256
FIXTURE_RUNTIME_SHA256="$(printf '%064d' 4)"
readonly FIXTURE_RUNTIME_SHA256
jq --null-input --compact-output \
  --arg source_sha256 "${FIXTURE_SOURCE_SHA256}" \
  --arg header_sha256 "${FIXTURE_HEADER_SHA256}" \
  --arg bindings_sha256 "${FIXTURE_BINDINGS_SHA256}" \
  --arg runtime_sha256 "${FIXTURE_RUNTIME_SHA256}" '
    {
      schema: "pggomtm-build-identity/v1",
      module_version: "0.1.0",
      features: ["pg18"],
      rust: {
        version: "1.97.1",
        target: "x86_64-unknown-linux-gnu"
      },
      dependencies: {
        pgrx: "0.19.1",
        jose_implementation: "jaws",
        jose_version: "1.0.4"
      },
      postgresql: {
        source_version: "18.4",
        pg_version_num: 180004,
        source_sha256: $source_sha256,
        oauth_header_sha256: $header_sha256,
        oauth_bindings_sha256: $bindings_sha256,
        runtime_base: "postgres:18.4-bookworm",
        runtime_base_sha256: $runtime_sha256
      },
      platform: {
        os: "linux",
        arch: "amd64",
        libc: "glibc"
      }
    }
  ' >"${IDENTITY_ROOT}/release/build/pggomtm-approved/out/pggomtm_build_identity.json"

readonly MODULE_FILE="${TEMP_ROOT}/pggomtm.so"
readonly MANIFEST_FILE="${TEMP_ROOT}/build-manifest.json"
printf 'deterministic synthetic module bytes\n' >"${MODULE_FILE}"
chmod 0644 "${MODULE_FILE}"

"${GATE}" create-build-manifest \
  "${IDENTITY_ROOT}" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${MANIFEST_FILE}"
"${GATE}" verify-build-manifest \
  "${MANIFEST_FILE}" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${IDENTITY_ROOT}"

jq --exit-status '
  .schema == "pggomtm-build-manifest/v1"
  and .module.name == "pggomtm"
  and .module.version == "0.1.0"
  and .module.features == ["pg18"]
  and .module.path == "/usr/lib/postgresql/18/lib/pggomtm.so"
  and .license.spdx == "MIT"
  and .license.path == "/usr/share/doc/pggomtm/LICENSE"
  and .postgresql.runtime_base == "postgres:18.4-bookworm"
  and .platform == {"arch":"amd64","libc":"glibc","os":"linux"}
' "${MANIFEST_FILE}" >/dev/null || fail "generated build manifest omitted approved public identity"

install -d "${IDENTITY_ROOT}/release/build/pggomtm-duplicate/out"
install -m 0600 \
  "${IDENTITY_ROOT}/release/build/pggomtm-approved/out/pggomtm_build_identity.json" \
  "${IDENTITY_ROOT}/release/build/pggomtm-duplicate/out/pggomtm_build_identity.json"
expect_failure \
  "multiple production build identities were accepted" \
  "${GATE}" create-build-manifest \
  "${IDENTITY_ROOT}" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${TEMP_ROOT}/duplicate-manifest.json"
rm -rf "${IDENTITY_ROOT}/release/build/pggomtm-duplicate"

jq '.image_digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' \
  "${MANIFEST_FILE}" >"${TEMP_ROOT}/self-referential-manifest.json"
expect_failure \
  "image self-digest was accepted in the internal build manifest" \
  "${GATE}" verify-build-manifest \
  "${TEMP_ROOT}/self-referential-manifest.json" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${IDENTITY_ROOT}"

jq '.module.sha256 = "<digest>"' \
  "${MANIFEST_FILE}" >"${TEMP_ROOT}/placeholder-manifest.json"
expect_failure \
  "placeholder digest was accepted in the internal build manifest" \
  "${GATE}" verify-build-manifest \
  "${TEMP_ROOT}/placeholder-manifest.json" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${IDENTITY_ROOT}"

jq --arg changed "${FIXTURE_BINDINGS_SHA256}" \
  '.postgresql.oauth_header_sha256 = $changed' \
  "${MANIFEST_FILE}" >"${TEMP_ROOT}/identity-mismatch-manifest.json"
expect_failure \
  "manifest build facts did not have to match the production identity" \
  "${GATE}" verify-build-manifest \
  "${TEMP_ROOT}/identity-mismatch-manifest.json" \
  "${MODULE_FILE}" \
  "${LICENSE_FILE}" \
  "${IDENTITY_ROOT}"

cp "${MODULE_FILE}" "${TEMP_ROOT}/changed-pggomtm.so"
printf 'changed\n' >>"${TEMP_ROOT}/changed-pggomtm.so"
expect_failure \
  "module bytes did not have to match the manifest digest" \
  "${GATE}" verify-build-manifest \
  "${MANIFEST_FILE}" \
  "${TEMP_ROOT}/changed-pggomtm.so" \
  "${LICENSE_FILE}" \
  "${IDENTITY_ROOT}"

readonly SNAPSHOT_ROOT="${TEMP_ROOT}/snapshot-root"
install -d "${SNAPSHOT_ROOT}/etc"
printf 'baseline\n' >"${SNAPSHOT_ROOT}/etc/base.conf"
chmod 0644 "${SNAPSHOT_ROOT}/etc/base.conf"
"${GATE}" snapshot-filesystem \
  "${SNAPSHOT_ROOT}" \
  "${TEMP_ROOT}/snapshot.tsv"
SNAPSHOT_HASH="$(sha256sum "${SNAPSHOT_ROOT}/etc/base.conf" | cut -d' ' -f1)"
readonly SNAPSHOT_HASH
grep --quiet --fixed-strings \
  "${SNAPSHOT_ROOT}/etc/base.conf" \
  "${TEMP_ROOT}/snapshot.tsv" && \
  fail "filesystem snapshot exposed host paths instead of logical paths"
grep --quiet --fixed-strings \
  "$(printf '/etc/base.conf\tf\t644\t%s\t%s\t%s' \
    "$(id -u)" "$(id -g)" "${SNAPSHOT_HASH}")" \
  "${TEMP_ROOT}/snapshot.tsv" || fail "filesystem snapshot omitted file identity"

MODULE_SHA256="$(sha256sum "${MODULE_FILE}" | cut -d' ' -f1)"
readonly MODULE_SHA256
LICENSE_SHA256="$(sha256sum "${LICENSE_FILE}" | cut -d' ' -f1)"
readonly LICENSE_SHA256
MANIFEST_SHA256="$(sha256sum "${MANIFEST_FILE}" | cut -d' ' -f1)"
readonly MANIFEST_SHA256
readonly BASE_INVENTORY="${TEMP_ROOT}/base.tsv"
readonly CANDIDATE_INVENTORY="${TEMP_ROOT}/candidate.tsv"
printf '/etc/base.conf\tf\t644\t0\t0\t%s\n' "${SNAPSHOT_HASH}" \
  >"${BASE_INVENTORY}"
{
  cat "${BASE_INVENTORY}"
  printf '/usr/lib/postgresql/18/lib/pggomtm.so\tf\t644\t0\t0\t%s\n' \
    "${MODULE_SHA256}"
  printf '/usr/share/doc/pggomtm\td\t755\t0\t0\t-\n'
  printf '/usr/share/doc/pggomtm/LICENSE\tf\t644\t0\t0\t%s\n' \
    "${LICENSE_SHA256}"
  printf '/usr/share/doc/pggomtm/build-manifest.json\tf\t644\t0\t0\t%s\n' \
    "${MANIFEST_SHA256}"
} | LC_ALL=C sort >"${CANDIDATE_INVENTORY}"
"${GATE}" verify-filesystem "${BASE_INVENTORY}" "${CANDIDATE_INVENTORY}"

cp "${CANDIDATE_INVENTORY}" "${TEMP_ROOT}/extra-file.tsv"
printf '/usr/local/bin/unapproved\tf\t755\t0\t0\t%s\n' "${MODULE_SHA256}" \
  >>"${TEMP_ROOT}/extra-file.tsv"
LC_ALL=C sort -o "${TEMP_ROOT}/extra-file.tsv" "${TEMP_ROOT}/extra-file.tsv"
expect_failure \
  "an unapproved final-image file was accepted" \
  "${GATE}" verify-filesystem \
  "${BASE_INVENTORY}" \
  "${TEMP_ROOT}/extra-file.tsv"

sed "s#${SNAPSHOT_HASH}#${MODULE_SHA256}#" \
  "${CANDIDATE_INVENTORY}" >"${TEMP_ROOT}/modified-base.tsv"
expect_failure \
  "a modified official-base file was accepted" \
  "${GATE}" verify-filesystem \
  "${BASE_INVENTORY}" \
  "${TEMP_ROOT}/modified-base.tsv"

expect_failure \
  "a non-ELF artifact passed the ELF policy" \
  "${GATE}" verify-elf "${MODULE_FILE}"

"${GATE}" verify-dockerfile "${REPOSITORY_ROOT}/Dockerfile"
awk '
  { print }
  $0 == "FROM candidate-content" {
    print "COPY LICENSE /tmp/unapproved-final-file"
  }
' "${REPOSITORY_ROOT}/Dockerfile" >"${TEMP_ROOT}/modified-final-stage.Dockerfile"
expect_failure \
  "an unapproved final-stage filesystem mutation was accepted" \
  "${GATE}" verify-dockerfile \
  "${TEMP_ROOT}/modified-final-stage.Dockerfile"
assert_contains "${REPOSITORY_ROOT}/.dockerignore" "!LICENSE"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "tests/artifact_readiness_gate.sh"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "scripts/artifact-readiness create-build-manifest"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "scripts/artifact-readiness verify-elf"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "AS runtime-base-inventory"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "AS candidate-content"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "AS candidate-runtime-gate"
assert_contains "${REPOSITORY_ROOT}/Dockerfile" \
  "AS candidate-artifact-gate"

printf 'artifact-readiness gate fixture policy passed\n'
