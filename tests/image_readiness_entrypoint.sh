#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly ENTRYPOINT="${REPOSITORY_ROOT}/scripts/image-readiness"
readonly OFFICIAL_IMAGE="postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296"

fail() {
  printf 'image-readiness entrypoint policy failed: %s\n' "$1" >&2
  exit 1
}

test -x "${ENTRYPOINT}" || fail "scripts/image-readiness is not executable"
help_output="$("${ENTRYPOINT}" help)"
grep --quiet --fixed-strings -- 'verify IMAGE SOURCE_REVISION' <<<"${help_output}" || \
  fail "help omitted the image/source contract"
grep --quiet --fixed-strings -- "${OFFICIAL_IMAGE}" <<<"${help_output}" || \
  fail "help omitted the approved official base"

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT
readonly DOCKER_MARKER="${TEMP_ROOT}/docker-called"
cat >"${TEMP_ROOT}/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
touch "${PGGOMTM_TEST_DOCKER_MARKER}"
exit 17
EOF
chmod 0755 "${TEMP_ROOT}/docker"

if env -u GITHUB_ACTIONS \
  PGGOMTM_DOCKER_BIN="${TEMP_ROOT}/docker" \
  PGGOMTM_TEST_DOCKER_MARKER="${DOCKER_MARKER}" \
  "${ENTRYPOINT}" verify candidate:not-a-digest invalid-source \
  >"${TEMP_ROOT}/local.out" 2>&1; then
  fail "entrypoint accepted local image recomputation"
fi
test ! -e "${DOCKER_MARKER}" || \
  fail "entrypoint touched Docker before rejecting local use"
grep --quiet --fixed-strings -- 'recomputation is restricted to GitHub Actions' \
  "${TEMP_ROOT}/local.out" || fail "entrypoint returned an unstable local rejection"

if GITHUB_ACTIONS=true \
  PGGOMTM_DOCKER_BIN="${TEMP_ROOT}/docker" \
  PGGOMTM_TEST_DOCKER_MARKER="${DOCKER_MARKER}" \
  "${ENTRYPOINT}" verify candidate:not-a-digest invalid-source \
  >"${TEMP_ROOT}/invalid.out" 2>&1; then
  fail "entrypoint accepted an invalid source revision"
fi
test ! -e "${DOCKER_MARKER}" || \
  fail "entrypoint mutated Docker state before validating source identity"
grep --quiet --fixed-strings -- 'source revision must be a full lowercase Git commit' \
  "${TEMP_ROOT}/invalid.out" || fail "entrypoint returned an unstable validation error"

for required in \
  "\$official.Config == \$image.Config" \
  "((\$base_layers | length) + 3)" \
  'verify-filesystem' \
  'verify-elf' \
  'verify-image' \
  'verify-runtime' \
  'pggomtm_abi_runtime_probe' \
  'pggomtm_identity_gate' \
  'synthetic-private-key' \
  'oauth_validator_libraries=pggomtm' \
  'POSTGRES_HOST_AUTH_METHOD=trust' \
  '--tmpfs /var/lib/postgresql' \
  'rm --force'; do
  grep --quiet --fixed-strings -- "${required}" "${ENTRYPOINT}" || \
    fail "entrypoint omitted readiness operation: ${required}"
done

printf 'image-readiness entrypoint policy passed\n'
