#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly HOST_ENTRYPOINT="${REPOSITORY_ROOT}/tests/postgres_integration.sh"
readonly CONTAINER_ENTRYPOINT="${REPOSITORY_ROOT}/tests/postgres_integration_container.sh"
readonly POSTGRES_IMAGE="postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296"

fail() {
  printf 'PostgreSQL integration entrypoint policy failed: %s\n' "$1" >&2
  exit 1
}

test -x "${HOST_ENTRYPOINT}" || fail "host harness is not executable"
test -x "${CONTAINER_ENTRYPOINT}" || fail "container harness is not executable"

host_help="$("${HOST_ENTRYPOINT}" help)"
grep --quiet --fixed-strings -- 'run ARTIFACT_DIRECTORY' <<<"${host_help}" || \
  fail "host harness help omitted the artifact-directory contract"
grep --quiet --fixed-strings -- "${POSTGRES_IMAGE}" "${HOST_ENTRYPOINT}" || \
  fail "host harness does not pin the approved PostgreSQL runtime digest"

container_help="$("${CONTAINER_ENTRYPOINT}" help)"
for matrix in abi-runtime oauth-gate production-identity; do
  grep --quiet --fixed-strings -- "${matrix}" <<<"${container_help}" || \
    fail "container harness help omitted matrix: ${matrix}"
done

if host_local_error="$(env -u GITHUB_ACTIONS \
  "${HOST_ENTRYPOINT}" run /path/that/does/not/exist 2>&1)"; then
  fail "host harness accepted local recomputation"
fi
grep --quiet --fixed-strings -- 'recomputation is restricted to GitHub Actions' \
  <<<"${host_local_error}" || fail "host harness returned an unstable local rejection"

if container_local_error="$(env -u GITHUB_ACTIONS \
  "${CONTAINER_ENTRYPOINT}" run /path/that/does/not/exist 2>&1)"; then
  fail "container harness accepted local recomputation"
fi
grep --quiet --fixed-strings -- 'recomputation is restricted to GitHub Actions' \
  <<<"${container_local_error}" || \
  fail "container harness returned an unstable local rejection"

if container_error="$(
  GITHUB_ACTIONS=true \
    "${CONTAINER_ENTRYPOINT}" run /path/that/does/not/exist 2>&1
)"; then
  fail "container harness accepted a missing artifact directory"
fi
grep --quiet --fixed-strings -- 'artifact directory is unavailable' \
  <<<"${container_error}" || fail "container harness returned an unstable input error"

if grep --quiet --fixed-strings 'docker build' \
  "${HOST_ENTRYPOINT}" "${CONTAINER_ENTRYPOINT}"; then
  fail "PostgreSQL integration harness delegates tests to Dockerfile"
fi

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT
readonly ARTIFACT_ROOT="${TEMP_ROOT}/artifacts"
install -d "${ARTIFACT_ROOT}/runtime-config-fixture"
for artifact in \
  pggomtm_abi_gate.so \
  pggomtm_abi_runtime_probe.so \
  pggomtm_config_gate.so \
  pggomtm_pgx_gate.so \
  pggomtm_identity_gate.so \
  pggomtm_oauth_smoke_client \
  pggomtm_oauth_smoke_fixture \
  oauth_runtime_probe.sql \
  runtime_config_missing_probe.sql \
  runtime_config_ready_probe.sql \
  runtime_config_validate_probe.sql; do
  printf '%s\n' 'synthetic artifact' >"${ARTIFACT_ROOT}/${artifact}"
done
printf '%s\n' '{}' >"${ARTIFACT_ROOT}/runtime-config-fixture/validator.json"
printf '%s\n' '{"keys":[]}' >"${ARTIFACT_ROOT}/runtime-config-fixture/jwks.json"

readonly DOCKER_LOG="${TEMP_ROOT}/docker.log"
readonly REMOVE_MARKER="${TEMP_ROOT}/removed"
cat >"${TEMP_ROOT}/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${PGGOMTM_TEST_DOCKER_LOG}"
case "${1:-}" in
  create) printf '%s\n' 'synthetic-container-id' ;;
  exec)
    if test "${PGGOMTM_TEST_DOCKER_EXEC_FAIL:-0}" = '1'; then
      exit 17
    fi
    ;;
  rm) touch "${PGGOMTM_TEST_REMOVE_MARKER}" ;;
esac
EOF
chmod 0755 "${TEMP_ROOT}/docker"

PGGOMTM_DOCKER_BIN="${TEMP_ROOT}/docker" \
PGGOMTM_TEST_DOCKER_LOG="${DOCKER_LOG}" \
PGGOMTM_TEST_REMOVE_MARKER="${REMOVE_MARKER}" \
GITHUB_ACTIONS=true \
  "${HOST_ENTRYPOINT}" run "${ARTIFACT_ROOT}"
test -f "${REMOVE_MARKER}" || fail "host harness did not remove its successful test container"
grep --quiet --fixed-strings -- "pull ${POSTGRES_IMAGE}" "${DOCKER_LOG}" || \
  fail "host harness did not pull the pinned PostgreSQL image"
grep --quiet --fixed-strings -- 'postgres_integration_container.sh run' \
  "${DOCKER_LOG}" || fail "host harness did not execute the container matrix"

rm -f "${REMOVE_MARKER}"
if PGGOMTM_DOCKER_BIN="${TEMP_ROOT}/docker" \
  PGGOMTM_TEST_DOCKER_LOG="${DOCKER_LOG}" \
  PGGOMTM_TEST_REMOVE_MARKER="${REMOVE_MARKER}" \
  PGGOMTM_TEST_DOCKER_EXEC_FAIL=1 \
  GITHUB_ACTIONS=true \
    "${HOST_ENTRYPOINT}" run "${ARTIFACT_ROOT}"; then
  fail "host harness ignored a failed container matrix"
fi
test -f "${REMOVE_MARKER}" || fail "host harness leaked its failed test container"

printf 'PostgreSQL integration entrypoint policy passed\n'
