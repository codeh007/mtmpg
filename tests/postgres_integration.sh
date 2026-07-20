#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

readonly POSTGRES_IMAGE="${PGGOMTM_POSTGRES_IMAGE:-postgres:18-bookworm}"
REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly CONTAINER_ENTRYPOINT="${REPOSITORY_ROOT}/tests/postgres_integration_container.sh"
readonly DOCKER_BIN="${PGGOMTM_DOCKER_BIN:-docker}"
TEST_CONTAINER=""

cleanup() {
  if test -n "${TEST_CONTAINER}"; then
    "${DOCKER_BIN}" rm --force "${TEST_CONTAINER}" >/dev/null 2>&1 || true
    TEST_CONTAINER=""
  fi
}

trap cleanup EXIT

fail() {
  printf 'PostgreSQL integration host harness: %s\n' "$1" >&2
  exit 2
}

require_github_actions() {
  test "${GITHUB_ACTIONS:-}" = "true" || \
    fail "recomputation is restricted to GitHub Actions; run the Native CI workflow"
}

validate_artifacts() {
  local artifact_root="$1"
  local artifact
  for artifact in \
    pggomtm_abi_gate.so \
    pggomtm_abi_runtime_probe.so \
    pggomtm.so \
    pggomtm_pgx_gate.so \
    pggomtm_oauth_smoke_client \
    pggomtm_oauth_smoke_fixture \
    oauth_runtime_probe.sql \
    runtime_config_missing_probe.sql \
    runtime_config_ready_probe.sql \
    runtime-config-fixture/validator.json \
    runtime-config-fixture/jwks.json; do
    if test ! -f "${artifact_root}/${artifact}" || test -L "${artifact_root}/${artifact}"; then
      fail "required integration artifact is unavailable: ${artifact}"
    fi
  done
}

validate_executor_artifacts() {
  local artifact_root="$1"
  local artifact
  for artifact in \
    mtmpg-executor \
    mtmpg_executor_fixture \
    mtmpg_executor_pg18_driver \
    pggomtm.so \
    executor_postgres_setup.sql \
    runtime/ca.crt \
    runtime/executor.crt \
    runtime/executor.key \
    runtime/hmac.secret \
    runtime/jwks.json \
    runtime/postgres.crt \
    runtime/postgres.key \
    runtime/signing-key.pem \
    runtime/validator.json; do
    if test ! -f "${artifact_root}/${artifact}" || test -L "${artifact_root}/${artifact}"; then
      fail "required executor integration artifact is unavailable: ${artifact}"
    fi
  done
}

run_integration() {
  test "$#" -eq 2 || fail "run requires a mode and one artifact directory"
  local mode="$1"
  shift
  command -v "${DOCKER_BIN}" >/dev/null || fail "Docker is unavailable"
  test -x "${CONTAINER_ENTRYPOINT}" || fail "container harness is unavailable"

  local artifact_root
  artifact_root="$(realpath -- "$1")" || fail "artifact directory cannot be resolved"
  test -d "${artifact_root}" || fail "artifact directory is unavailable"
  case "${mode}" in
    run) validate_artifacts "${artifact_root}" ;;
    run-executor) validate_executor_artifacts "${artifact_root}" ;;
    *) fail "unknown integration mode" ;;
  esac

  local container_name="pggomtm-integration-${GITHUB_RUN_ID:-local}-$$"
  "${DOCKER_BIN}" pull "${POSTGRES_IMAGE}" >/dev/null
  TEST_CONTAINER="$(
    "${DOCKER_BIN}" create \
      --name "${container_name}" \
      --hostname postgres \
      --platform linux/amd64 \
      --entrypoint sleep \
      "${POSTGRES_IMAGE}" \
      infinity
  )"
  test -n "${TEST_CONTAINER}" || fail "Docker did not create the integration container"
  "${DOCKER_BIN}" start "${TEST_CONTAINER}" >/dev/null
  "${DOCKER_BIN}" cp "${artifact_root}/." "${TEST_CONTAINER}:/test-artifacts"
  "${DOCKER_BIN}" cp \
    "${CONTAINER_ENTRYPOINT}" \
    "${TEST_CONTAINER}:/usr/local/bin/postgres_integration_container.sh"
  "${DOCKER_BIN}" exec \
    --env GITHUB_ACTIONS=true \
    "${TEST_CONTAINER}" \
    /usr/local/bin/postgres_integration_container.sh \
    "${mode}" \
    /test-artifacts
  cleanup
}

usage() {
  printf '%s\n' \
    'usage: tests/postgres_integration.sh run ARTIFACT_DIRECTORY' \
    '       tests/postgres_integration.sh run-executor ARTIFACT_DIRECTORY' \
    '' \
    "runtime: ${POSTGRES_IMAGE}"
}

case "${1:-}" in
  help|--help|-h)
    usage
    ;;
  run)
    require_github_actions
    shift
    run_integration run "$@"
    ;;
  run-executor)
    require_github_actions
    shift
    run_integration run-executor "$@"
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
