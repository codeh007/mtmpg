#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly REPOSITORY_ROOT

fail() {
  printf 'executor stage-integration: %s\n' "$1" >&2
  exit 2
}

test "${GITHUB_ACTIONS:-}" = "true" || fail "run this harness through GitHub Actions"
test -x "${PGRX_PG_CONFIG_PATH:-}" || fail "PGRX_PG_CONFIG_PATH is unavailable"
[[ "$("${PGRX_PG_CONFIG_PATH}" --version)" =~ ^PostgreSQL\ 18\. ]] || \
  fail "PGRX_PG_CONFIG_PATH must identify PostgreSQL 18"
command -v openssl >/dev/null || fail "OpenSSL is unavailable"

test "$#" -eq 2 || fail "expected validator artifact and output directories"
VALIDATOR_ROOT="$(realpath -- "$1")" || fail "validator artifacts cannot be resolved"
test -f "${VALIDATOR_ROOT}/pggomtm.so" || fail "production validator module is unavailable"

TARGET_ROOT="${CARGO_TARGET_DIR:-${REPOSITORY_ROOT}/target}"
if [[ "${TARGET_ROOT}" != /* ]]; then
  TARGET_ROOT="${REPOSITORY_ROOT}/${TARGET_ROOT}"
fi
OUTPUT_ROOT="$2"
if [[ "${OUTPUT_ROOT}" != /* ]]; then
  OUTPUT_ROOT="${REPOSITORY_ROOT}/${OUTPUT_ROOT}"
fi
case "${OUTPUT_ROOT}" in
  "${TARGET_ROOT}"/*) ;;
  *) fail "output directory must be inside CARGO_TARGET_DIR" ;;
esac

install -d -m 0755 "${TARGET_ROOT}"
STAGING_ROOT="$(mktemp --directory "${TARGET_ROOT}/.executor-integration.XXXXXX")"
cleanup() {
  if test -n "${STAGING_ROOT:-}" && test -d "${STAGING_ROOT}"; then
    rm -rf -- "${STAGING_ROOT}"
  fi
}
trap cleanup EXIT

cd "${REPOSITORY_ROOT}"
cargo build --locked --release --package mtmpg-executor
cargo build --locked --release --package mtmpg-executor --examples

install -m 0755 \
  "${TARGET_ROOT}/release/mtmpg-executor" \
  "${STAGING_ROOT}/mtmpg-executor"
install -m 0755 \
  "${TARGET_ROOT}/release/examples/mtmpg_executor_fixture" \
  "${STAGING_ROOT}/mtmpg_executor_fixture"
install -m 0755 \
  "${TARGET_ROOT}/release/examples/mtmpg_executor_pg18_driver" \
  "${STAGING_ROOT}/mtmpg_executor_pg18_driver"
install -m 0644 "${VALIDATOR_ROOT}/pggomtm.so" "${STAGING_ROOT}/pggomtm.so"
install -m 0644 \
  executor/tests/postgres_setup.sql \
  "${STAGING_ROOT}/executor_postgres_setup.sql"

install -d -m 0700 "${STAGING_ROOT}/runtime"
"${STAGING_ROOT}/mtmpg_executor_fixture" generate "${STAGING_ROOT}/runtime"

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
  -subj '/CN=Executor integration CA' \
  -keyout "${STAGING_ROOT}/runtime/ca.key" \
  -out "${STAGING_ROOT}/runtime/ca.crt" >/dev/null 2>&1

generate_leaf() {
  local name="$1"
  openssl req -newkey rsa:2048 -sha256 -nodes \
    -subj "/CN=${name}" \
    -addext "subjectAltName=DNS:${name}" \
    -keyout "${STAGING_ROOT}/runtime/${name}.key" \
    -out "${STAGING_ROOT}/runtime/${name}.csr" >/dev/null 2>&1
  openssl x509 -req -sha256 -days 1 \
    -in "${STAGING_ROOT}/runtime/${name}.csr" \
    -CA "${STAGING_ROOT}/runtime/ca.crt" \
    -CAkey "${STAGING_ROOT}/runtime/ca.key" \
    -CAcreateserial \
    -copy_extensions copy \
    -out "${STAGING_ROOT}/runtime/${name}.crt" >/dev/null 2>&1
  rm -f -- "${STAGING_ROOT}/runtime/${name}.csr"
}

generate_leaf postgres
generate_leaf executor
rm -f -- "${STAGING_ROOT}/runtime/ca.key" "${STAGING_ROOT}/runtime/ca.srl"
chmod 0400 \
  "${STAGING_ROOT}/runtime/hmac.secret" \
  "${STAGING_ROOT}/runtime/signing-key.pem" \
  "${STAGING_ROOT}/runtime/postgres.key" \
  "${STAGING_ROOT}/runtime/executor.key"
chmod 0444 \
  "${STAGING_ROOT}/runtime/ca.crt" \
  "${STAGING_ROOT}/runtime/postgres.crt" \
  "${STAGING_ROOT}/runtime/executor.crt" \
  "${STAGING_ROOT}/runtime/jwks.json" \
  "${STAGING_ROOT}/runtime/validator.json"

chmod 0755 "${STAGING_ROOT}"
rm -rf -- "${OUTPUT_ROOT}"
mv -- "${STAGING_ROOT}" "${OUTPUT_ROOT}"
STAGING_ROOT=""
printf '%s\n' "${OUTPUT_ROOT}"
