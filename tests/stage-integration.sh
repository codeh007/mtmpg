#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT

fail() {
  printf 'stage-integration: %s\n' "$1" >&2
  exit 2
}

test "${GITHUB_ACTIONS:-}" = "true" || fail "run this harness through GitHub Actions"
test -x "${PGRX_PG_CONFIG_PATH:-}" || fail "PGRX_PG_CONFIG_PATH is unavailable"
[[ "$("${PGRX_PG_CONFIG_PATH}" --version)" =~ ^PostgreSQL\ 18\. ]] || \
  fail "PGRX_PG_CONFIG_PATH must identify PostgreSQL 18"

TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
if [[ "${TARGET_ROOT}" != /* ]]; then
  TARGET_ROOT="${REPOSITORY_ROOT}/${TARGET_ROOT}"
fi
OUTPUT_ROOT="${1:-${TARGET_ROOT}/native-integration}"
if [[ "${OUTPUT_ROOT}" != /* ]]; then
  OUTPUT_ROOT="${REPOSITORY_ROOT}/${OUTPUT_ROOT}"
fi
case "${OUTPUT_ROOT}" in
  "${TARGET_ROOT}"/*) ;;
  *) fail "output directory must be inside CARGO_TARGET_DIR" ;;
esac

install -d -m 0755 "${TARGET_ROOT}"
STAGING_ROOT="$(mktemp --directory "${TARGET_ROOT}/.native-integration.XXXXXX")"
cleanup() {
  if test -n "${STAGING_ROOT:-}" && test -d "${STAGING_ROOT}"; then
    rm -rf -- "${STAGING_ROOT}"
  fi
}
trap cleanup EXIT

cd "${REPOSITORY_ROOT}"
readonly RELEASE_MODULE="${TARGET_ROOT}/release/libpggomtm.so"

cargo build --locked --release --no-default-features --features pg18,abi-runtime-gate
install -m 0644 "${RELEASE_MODULE}" "${STAGING_ROOT}/pggomtm_abi_gate.so"

cargo build --locked --release --no-default-features --features pg18,pgx-oauth-gate
install -m 0644 "${RELEASE_MODULE}" "${STAGING_ROOT}/pggomtm_pgx_gate.so"

cargo build --locked --release --lib --no-default-features --features pg18
install -m 0644 "${RELEASE_MODULE}" "${STAGING_ROOT}/pggomtm.so"

cargo build --locked --release --bin pggomtm_oauth_smoke_fixture \
  --no-default-features \
  --features pg18,pgx-oauth-gate
install -m 0755 \
  "${TARGET_ROOT}/release/pggomtm_oauth_smoke_fixture" \
  "${STAGING_ROOT}/pggomtm_oauth_smoke_fixture"

INCLUDE_DIR="$("${PGRX_PG_CONFIG_PATH}" --includedir-server)"
CLIENT_INCLUDE_DIR="$("${PGRX_PG_CONFIG_PATH}" --includedir)"
LIB_DIR="$("${PGRX_PG_CONFIG_PATH}" --libdir)"
read -r -a CPPFLAGS <<<"$("${PGRX_PG_CONFIG_PATH}" --cppflags)"

cc -std=c11 -Wall -Wextra -Werror -fPIC -shared \
  "${CPPFLAGS[@]}" \
  -I"${INCLUDE_DIR}" \
  tests/oauth_runtime_probe.c \
  -o "${STAGING_ROOT}/pggomtm_abi_runtime_probe.so"
cc -std=c11 -Wall -Wextra -Werror \
  -I"${CLIENT_INCLUDE_DIR}" \
  tests/oauth_smoke_client.c \
  -L"${LIB_DIR}" \
  -lpq \
  -o "${STAGING_ROOT}/pggomtm_oauth_smoke_client"
chmod 0644 "${STAGING_ROOT}/pggomtm_abi_runtime_probe.so"
chmod 0755 "${STAGING_ROOT}/pggomtm_oauth_smoke_client"

install -m 0644 \
  tests/oauth_runtime_probe.sql \
  tests/runtime_config_missing_probe.sql \
  tests/runtime_config_ready_probe.sql \
  "${STAGING_ROOT}"
install -d -m 0755 "${STAGING_ROOT}/runtime-config-fixture"
install -m 0644 \
  tests/fixtures/runtime-config/validator.json \
  tests/fixtures/runtime-config/jwks.json \
  "${STAGING_ROOT}/runtime-config-fixture"

chmod 0755 "${STAGING_ROOT}"
rm -rf -- "${OUTPUT_ROOT}"
mv -- "${STAGING_ROOT}" "${OUTPUT_ROOT}"
STAGING_ROOT=""
printf '%s\n' "${OUTPUT_ROOT}"
