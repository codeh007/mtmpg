#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

ARTIFACT_ROOT=""
ACTIVE_PGDATA=""
EXECUTOR_PID=""
PKGLIBDIR=""

fail() {
  printf 'PostgreSQL integration container harness: %s\n' "$1" >&2
  exit 2
}

require_github_actions() {
  test "${GITHUB_ACTIONS:-}" = "true" || \
    fail "recomputation is restricted to GitHub Actions; run the Native CI workflow"
}

assert_no_extended_match() {
  local pattern="$1"
  local file_path="$2"
  local reason="$3"
  local status

  set +e
  grep --extended-regexp --quiet "${pattern}" "${file_path}"
  status=$?
  set -e
  case "${status}" in
    0) fail "${reason}" ;;
    1) ;;
    *) fail "negative log scan could not read ${file_path}" ;;
  esac
}

assert_no_fixed_string() {
  local pattern="$1"
  local file_path="$2"
  local reason="$3"
  local status

  set +e
  grep --fixed-strings --quiet "${pattern}" "${file_path}"
  status=$?
  set -e
  case "${status}" in
    0) fail "${reason}" ;;
    1) ;;
    *) fail "negative fixed-string scan could not read ${file_path}" ;;
  esac
}

assert_file_contents_absent() {
  local pattern_file="$1"
  local file_path="$2"
  local reason="$3"
  local status

  test -s "${pattern_file}" || fail "negative scan material is empty: ${pattern_file}"
  set +e
  grep --fixed-strings --file="${pattern_file}" "${file_path}" >/dev/null
  status=$?
  set -e
  case "${status}" in
    0) fail "${reason}" ;;
    1) ;;
    *) fail "negative material scan could not read its inputs" ;;
  esac
}

stop_active_cluster() {
  if test -n "${ACTIVE_PGDATA}" && test -f "${ACTIVE_PGDATA}/postmaster.pid"; then
    gosu postgres pg_ctl \
      --pgdata="${ACTIVE_PGDATA}" \
      --mode=immediate \
      --wait stop >/dev/null 2>&1 || true
  fi
  ACTIVE_PGDATA=""
}

stop_executor() {
  if test -n "${EXECUTOR_PID}" && kill -0 "${EXECUTOR_PID}" >/dev/null 2>&1; then
    kill -TERM "${EXECUTOR_PID}" >/dev/null 2>&1 || true
    wait "${EXECUTOR_PID}" >/dev/null 2>&1 || true
  fi
  EXECUTOR_PID=""
}

cleanup() {
  stop_executor
  stop_active_cluster
  rm -rf \
    /etc/pggomtm \
    /tmp/pggomtm-abi-pgdata \
    /tmp/pggomtm-abi-server.log \
    /tmp/pggomtm-oauth-pgdata \
    /tmp/pggomtm-oauth-ldd \
    /tmp/pggomtm-oauth-server.log \
    /tmp/pggomtm-oauth-fixtures \
    /tmp/pggomtm-production-backend-pgdata \
    /tmp/pggomtm-production-backend-server.log \
    /tmp/pggomtm-production-backend-fixtures \
    /tmp/mtmpg-executor-pgdata \
    /tmp/mtmpg-executor-runtime \
    /tmp/mtmpg-executor-server.log \
    /tmp/mtmpg-executor-service.log

  if test -n "${PKGLIBDIR}"; then
    rm -f \
      "${PKGLIBDIR}/pggomtm_abi_gate.so" \
      "${PKGLIBDIR}/pggomtm_abi_runtime_probe.so" \
      "${PKGLIBDIR}/pggomtm.so" \
      "${PKGLIBDIR}/pggomtm_pgx_gate.so"
  fi
}

trap cleanup EXIT INT TERM

usage() {
  printf '%s\n' \
    'usage: tests/postgres_integration_container.sh run ARTIFACT_DIRECTORY' \
    '       tests/postgres_integration_container.sh run-executor ARTIFACT_DIRECTORY' \
    '' \
    'matrices:' \
    '  abi-runtime' \
    '  oauth-gate' \
    '  production-backend' \
    '  executor-oauth-sql'
}

require_artifact() {
  local path="$1"
  if test ! -f "${ARTIFACT_ROOT}/${path}" || test -L "${ARTIFACT_ROOT}/${path}"; then
    fail "required integration artifact is unavailable: ${path}"
  fi
}

verify_runtime() {
  test "$(id -u)" -eq 0 || fail "container harness must run as root"
  command -v gosu >/dev/null || fail "official postgres image does not provide gosu"
  command -v pg_config >/dev/null || fail "official postgres image does not provide pg_config"
  [[ "$(pg_config --version)" =~ ^PostgreSQL\ 18\. ]] || \
    fail "container runtime is not PostgreSQL 18"
  PKGLIBDIR="$(pg_config --pkglibdir)"
  test "${PKGLIBDIR}" = "/usr/lib/postgresql/18/lib" || \
    fail "container runtime has an unexpected module directory"

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
    require_artifact "${artifact}"
  done
  chmod 0755 \
    "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture"
}

verify_executor_runtime() {
  test "$(id -u)" -eq 0 || fail "container harness must run as root"
  command -v gosu >/dev/null || fail "official postgres image does not provide gosu"
  command -v pg_config >/dev/null || fail "official postgres image does not provide pg_config"
  [[ "$(pg_config --version)" =~ ^PostgreSQL\ 18\. ]] || \
    fail "container runtime is not PostgreSQL 18"
  PKGLIBDIR="$(pg_config --pkglibdir)"
  test "${PKGLIBDIR}" = "/usr/lib/postgresql/18/lib" || \
    fail "container runtime has an unexpected module directory"

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
    require_artifact "${artifact}"
  done
  chmod 0755 \
    "${ARTIFACT_ROOT}/mtmpg-executor" \
    "${ARTIFACT_ROOT}/mtmpg_executor_fixture" \
    "${ARTIFACT_ROOT}/mtmpg_executor_pg18_driver"

  local executor_linkage
  if ! executor_linkage="$(ldd "${ARTIFACT_ROOT}/mtmpg-executor" 2>&1)"; then
    fail "executor binary is incompatible with the PG18 runtime"
  fi
  if grep --quiet 'not found' <<<"${executor_linkage}"; then
    fail "executor binary is incompatible with the PG18 runtime"
  fi
}

install_module() {
  local source_name="$1"
  local module_name="$2"
  install -m 0644 \
    "${ARTIFACT_ROOT}/${source_name}" \
    "${PKGLIBDIR}/${module_name}"
}

start_cluster() {
  local pgdata="$1"
  local log_file="$2"
  local options="$3"

  test -z "${ACTIVE_PGDATA}" || fail "another temporary cluster is still active"
  install -d -m 0700 -o postgres -g postgres "${pgdata}"
  gosu postgres initdb \
    --pgdata="${pgdata}" \
    --encoding=UTF8 \
    --no-locale \
    --auth-local=trust \
    --auth-host=reject >/dev/null
  ACTIVE_PGDATA="${pgdata}"
  gosu postgres pg_ctl \
    --pgdata="${pgdata}" \
    --log="${log_file}" \
    --options="${options}" \
    --wait start >/dev/null
}

stop_cluster() {
  test -n "${ACTIVE_PGDATA}" || fail "no temporary cluster is active"
  gosu postgres pg_ctl \
    --pgdata="${ACTIVE_PGDATA}" \
    --mode=fast \
    --wait stop >/dev/null
  ACTIVE_PGDATA=""
}

psql_file() {
  gosu postgres psql \
    --host=/tmp \
    --username=postgres \
    --dbname=postgres \
    --file="$1" >/dev/null
}

psql_command() {
  gosu postgres psql \
    --host=/tmp \
    --username=postgres \
    --dbname=postgres \
    --command="$1" >/dev/null
}

psql_scalar() {
  gosu postgres psql \
    --host=/tmp \
    --username=postgres \
    --dbname=postgres \
    --tuples-only \
    --no-align \
    --command="$1"
}

install_runtime_config() {
  rm -rf /etc/pggomtm
  install -d -m 0555 /etc/pggomtm
  install -m 0444 \
    "${ARTIFACT_ROOT}/runtime-config-fixture/validator.json" \
    /etc/pggomtm/validator.json
  install -m 0444 \
    "${ARTIFACT_ROOT}/runtime-config-fixture/jwks.json" \
    /etc/pggomtm/jwks.json
}

generate_fixtures() {
  local fixture_root="$1"
  install -d -m 0700 "${fixture_root}"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture" generate "${fixture_root}"
}

run_abi_runtime_matrix() {
  local pgdata="/tmp/pggomtm-abi-pgdata"
  local log_file="/tmp/pggomtm-abi-server.log"

  install_module pggomtm_abi_gate.so pggomtm_abi_gate.so
  install_module pggomtm_abi_runtime_probe.so pggomtm_abi_runtime_probe.so
  install_module pggomtm.so pggomtm.so
  start_cluster \
    "${pgdata}" \
    "${log_file}" \
    "-c listen_addresses='' -k /tmp -c log_min_messages=log"

  psql_file "${ARTIFACT_ROOT}/oauth_runtime_probe.sql"
  psql_file "${ARTIFACT_ROOT}/runtime_config_missing_probe.sql"
  install_runtime_config
  psql_file "${ARTIFACT_ROOT}/runtime_config_ready_probe.sql"

  grep --extended-regexp --quiet \
    'LOG:.*pggomtm authentication rejected: reason=pggomtm-auth/v1/internal-panic' \
    "${log_file}"
  assert_no_extended_match \
    'stack backtrace|panicked at|RUST_BACKTRACE|src/lib\.rs|eyJ[A-Za-z0-9_-]+\.' \
    "${log_file}" \
    "ABI runtime log disclosed sensitive failure details"
  stop_cluster

  rm -rf "${pgdata}" "${log_file}" /etc/pggomtm
  rm -f \
    "${PKGLIBDIR}/pggomtm_abi_gate.so" \
    "${PKGLIBDIR}/pggomtm_abi_runtime_probe.so" \
    "${PKGLIBDIR}/pggomtm.so"
  printf 'PG18 ABI runtime matrix passed\n'
}

run_oauth_gate_matrix() {
  local pgdata="/tmp/pggomtm-oauth-pgdata"
  local log_file="/tmp/pggomtm-oauth-server.log"
  local fixture_root="/tmp/pggomtm-oauth-fixtures"

  install_module pggomtm_pgx_gate.so pggomtm_pgx_gate.so
  local ldd_output="/tmp/pggomtm-oauth-ldd"
  ldd "${PKGLIBDIR}/pggomtm_pgx_gate.so" >"${ldd_output}"
  assert_no_fixed_string \
    libcurl \
    "${ldd_output}" \
    "OAuth gate module unexpectedly depends on libcurl"
  install -d -m 0700 -o postgres -g postgres "${pgdata}"
  gosu postgres initdb \
    --pgdata="${pgdata}" \
    --encoding=UTF8 \
    --no-locale \
    --auth-local=trust \
    --auth-host=reject >/dev/null
  sed -i \
    '1ilocal all ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm_pgx_gate delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  ACTIVE_PGDATA="${pgdata}"
  gosu postgres pg_ctl \
    --pgdata="${pgdata}" \
    --log="${log_file}" \
    --options="-c listen_addresses='' -k /tmp -c oauth_validator_libraries=pggomtm_pgx_gate" \
    --wait start >/dev/null

  psql_command 'CREATE ROLE ordinary LOGIN'
  generate_fixtures "${fixture_root}"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    --expect-allowed \
    "${fixture_root}/oauth-ordinary.jwt" \
    ordinary \
    "${fixture_root}/oauth-ordinary.system-user"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture" \
    verify-system-user \
    oauth-ordinary \
    "${fixture_root}/oauth-ordinary.system-user"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    --expect-rejected \
    "${fixture_root}/tampered.jwt" \
    ordinary
  stop_cluster

  rm -rf "${pgdata}" "${log_file}" "${fixture_root}" "${ldd_output}"
  rm -f "${PKGLIBDIR}/pggomtm_pgx_gate.so"
  printf 'PG18 OAuth gate matrix passed\n'
}

verify_system_user() {
  local fixture_root="$1"
  local scenario="$2"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture" \
    verify-system-user \
    "${scenario}" \
    "${fixture_root}/${scenario}.system-user"
}

expect_allowed() {
  local fixture_root="$1"
  local scenario="$2"
  local role="$3"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    --expect-allowed \
    "${fixture_root}/${scenario}.jwt" \
    "${role}" \
    "${fixture_root}/${scenario}.system-user"
  verify_system_user "${fixture_root}" "${scenario}"
}

expect_rejected() {
  local fixture_root="$1"
  local scenario="$2"
  local role="$3"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    --expect-rejected \
    "${fixture_root}/${scenario}.jwt" \
    "${role}"
}

verify_production_log() {
  local log_file="$1"
  local fixture_root="$2"

  grep --extended-regexp --quiet \
    '(ERROR|FATAL):.*pggomtm authentication failed: reason=pggomtm-auth/v1/config-missing' \
    "${log_file}"
  grep --extended-regexp --quiet \
    'LOG:.*pggomtm authentication rejected: reason=pggomtm-auth/v1/token-signature-invalid' \
    "${log_file}"
  grep --extended-regexp --quiet \
    'LOG:.*pggomtm authentication rejected: reason=pggomtm-auth/v1/token-role-mismatch' \
    "${log_file}"

  local fixture
  for fixture in "${fixture_root}"/*.jwt; do
    assert_file_contents_absent \
      "${fixture}" \
      "${log_file}" \
      "production log disclosed a database JWT fixture"
  done
  assert_file_contents_absent \
    /etc/pggomtm/validator.json \
    "${log_file}" \
    "production log disclosed validator config"
  assert_file_contents_absent \
    /etc/pggomtm/jwks.json \
    "${log_file}" \
    "production log disclosed public JWKS content"
  assert_no_extended_match \
    'Authorization: Bearer|postgres(ql)?://|stack backtrace|panicked at|RUST_BACKTRACE|src/lib\.rs|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY' \
    "${log_file}" \
    "production log disclosed sensitive failure details"
}

run_production_backend_smoke() {
  local pgdata="/tmp/pggomtm-production-backend-pgdata"
  local log_file="/tmp/pggomtm-production-backend-server.log"
  local fixture_root="/tmp/pggomtm-production-backend-fixtures"

  install_module pggomtm.so pggomtm.so
  install -d -m 0700 -o postgres -g postgres "${pgdata}"
  gosu postgres initdb \
    --pgdata="${pgdata}" \
    --encoding=UTF8 \
    --no-locale \
    --auth-local=trust \
    --auth-host=reject >/dev/null
  sed -i \
    '1ilocal all ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ilocal all business_admin oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ilocal all gomtm_candidate_business_admin oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ilocal all gomtm_candidate_ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ilocal all gomtm_ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  ACTIVE_PGDATA="${pgdata}"
  gosu postgres pg_ctl \
    --pgdata="${pgdata}" \
    --log="${log_file}" \
    --options="-c listen_addresses='' -k /tmp -c log_min_messages=log -c oauth_validator_libraries=pggomtm" \
    --wait start >/dev/null

  psql_command \
    'CREATE ROLE ordinary LOGIN; CREATE ROLE business_admin LOGIN; CREATE ROLE gomtm_candidate_ordinary LOGIN; CREATE ROLE gomtm_candidate_business_admin LOGIN; CREATE ROLE gomtm_ordinary LOGIN'
  generate_fixtures "${fixture_root}"
  "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" \
    --expect-startup-rejected \
    "${fixture_root}/oauth-ordinary.jwt" \
    ordinary
  install_runtime_config

  expect_allowed "${fixture_root}" oauth-ordinary ordinary
  expect_rejected "${fixture_root}" oauth-ordinary business_admin
  expect_rejected "${fixture_root}" tampered ordinary
  expect_rejected "${fixture_root}" oauth-v1-profile gomtm_candidate_business_admin
  expect_rejected "${fixture_root}" oauth-project-role gomtm_ordinary
  expect_rejected "${fixture_root}" oauth-stage-role gomtm_candidate_ordinary

  stop_cluster
  verify_production_log "${log_file}" "${fixture_root}"
  rm -rf "${pgdata}" "${log_file}" "${fixture_root}" /etc/pggomtm
  rm -f "${PKGLIBDIR}/pggomtm.so"
  printf 'PG18 production backend smoke passed\n'
}

install_executor_runtime() {
  local runtime_root="$1"
  rm -rf /etc/pggomtm "${runtime_root}"
  install -d -m 0555 /etc/pggomtm
  install -m 0444 "${ARTIFACT_ROOT}/runtime/validator.json" /etc/pggomtm/validator.json
  install -m 0444 "${ARTIFACT_ROOT}/runtime/jwks.json" /etc/pggomtm/jwks.json

  install -d -m 0700 -o postgres -g postgres "${runtime_root}"
  install -m 0444 -o postgres -g postgres \
    "${ARTIFACT_ROOT}/runtime/ca.crt" \
    "${ARTIFACT_ROOT}/runtime/executor.crt" \
    "${runtime_root}"
  install -m 0400 -o postgres -g postgres \
    "${ARTIFACT_ROOT}/runtime/executor.key" \
    "${ARTIFACT_ROOT}/runtime/hmac.secret" \
    "${ARTIFACT_ROOT}/runtime/signing-key.pem" \
    "${runtime_root}"
}

run_executor_oauth_sql_matrix() {
  local pgdata="/tmp/mtmpg-executor-pgdata"
  local postgres_log="/tmp/mtmpg-executor-server.log"
  local executor_log="/tmp/mtmpg-executor-service.log"
  local runtime_root="/tmp/mtmpg-executor-runtime"

  install_module pggomtm.so pggomtm.so
  install_executor_runtime "${runtime_root}"
  install -d -m 0700 -o postgres -g postgres "${pgdata}"
  gosu postgres initdb \
    --pgdata="${pgdata}" \
    --encoding=UTF8 \
    --no-locale \
    --auth-local=trust \
    --auth-host=reject >/dev/null
  install -m 0600 -o postgres -g postgres \
    "${ARTIFACT_ROOT}/runtime/postgres.key" \
    "${pgdata}/server.key"
  install -m 0644 -o postgres -g postgres \
    "${ARTIFACT_ROOT}/runtime/postgres.crt" \
    "${pgdata}/server.crt"
  sed -i \
    '1ihostssl gomtm database_developer 0.0.0.0/0 oauth issuer="https://auth.example.test/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ihostssl gomtm business_admin 0.0.0.0/0 oauth issuer="https://auth.example.test/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  sed -i \
    '1ihostssl gomtm ordinary 0.0.0.0/0 oauth issuer="https://auth.example.test/database" scope="database" validator=pggomtm delegate_ident_mapping=1' \
    "${pgdata}/pg_hba.conf"
  ACTIVE_PGDATA="${pgdata}"
  gosu postgres pg_ctl \
    --pgdata="${pgdata}" \
    --log="${postgres_log}" \
    --options="-c listen_addresses='*' -k /tmp -c ssl=on -c log_min_messages=log -c oauth_validator_libraries=pggomtm" \
    --wait start >/dev/null
  psql_file "${ARTIFACT_ROOT}/executor_postgres_setup.sql"

  grep --quiet ' executor$' /etc/hosts || printf '127.0.0.1 executor\n' >>/etc/hosts
  gosu postgres env \
    MTMPG_EXECUTOR_AUDIENCE=https://postgres.example.test/database/main \
    MTMPG_EXECUTOR_HMAC_SECRET_PATH="${runtime_root}/hmac.secret" \
    MTMPG_EXECUTOR_ISSUER=https://auth.example.test/database \
    MTMPG_EXECUTOR_KEY_ID=executor-es256-test \
    MTMPG_EXECUTOR_LISTEN=0.0.0.0:8443 \
    MTMPG_EXECUTOR_POSTGRES_CA_PATH="${runtime_root}/ca.crt" \
    MTMPG_EXECUTOR_SIGNING_KEY_PATH="${runtime_root}/signing-key.pem" \
    MTMPG_EXECUTOR_TLS_CERT_PATH="${runtime_root}/executor.crt" \
    MTMPG_EXECUTOR_TLS_KEY_PATH="${runtime_root}/executor.key" \
    "${ARTIFACT_ROOT}/mtmpg-executor" >"${executor_log}" 2>&1 &
  EXECUTOR_PID=$!

  sleep 1
  if ! kill -0 "${EXECUTOR_PID}" >/dev/null 2>&1; then
    local startup_stage
    for startup_stage in \
      hmac \
      signing_key \
      issuer \
      token_registry \
      libpq \
      database_tls \
      listen \
      https_tls \
      https_server; do
      if grep --quiet "^executor startup failed: ${startup_stage}$" "${executor_log}"; then
        fail "executor service exited during ${startup_stage} startup"
      fi
    done
    fail "executor service exited before readiness"
  fi
  MTMPG_EXECUTOR_CA_PATH="${runtime_root}/ca.crt" \
  MTMPG_EXECUTOR_HMAC_PATH="${runtime_root}/hmac.secret" \
  MTMPG_EXECUTOR_URL=https://executor:8443 \
    "${ARTIFACT_ROOT}/mtmpg_executor_pg18_driver"

  local active_sleep
  active_sleep="$(psql_scalar "SELECT count(*) FROM pg_stat_activity WHERE state = 'active' AND query LIKE 'SELECT pg_sleep(%'")"
  test "${active_sleep}" = "0" || fail "cancelled executor query remained active"

  stop_executor
  stop_cluster
  assert_file_contents_absent \
    "${runtime_root}/hmac.secret" \
    "${executor_log}" \
    "executor log disclosed the HMAC secret"
  assert_file_contents_absent \
    "${runtime_root}/signing-key.pem" \
    "${executor_log}" \
    "executor log disclosed the signing key"
  assert_no_extended_match \
    'Authorization: Bearer|postgres(ql)?://|eyJ[A-Za-z0-9_-]+\.|BEGIN (EC )?PRIVATE KEY|SELECT pg_sleep|INSERT INTO app\.executor_probe|panicked at|stack backtrace' \
    "${executor_log}" \
    "executor log disclosed sensitive request content"
  assert_no_extended_match \
    'eyJ[A-Za-z0-9_-]+\.|BEGIN (EC )?PRIVATE KEY' \
    "${postgres_log}" \
    "PostgreSQL log disclosed executor token material"

  rm -rf "${pgdata}" "${runtime_root}" /etc/pggomtm
  rm -f "${postgres_log}" "${executor_log}" "${PKGLIBDIR}/pggomtm.so"
  printf 'PG18 executor OAuth and SQL integration matrix passed\n'
}

run_executor() {
  test "$#" -eq 1 || fail "run-executor requires exactly one artifact directory"
  ARTIFACT_ROOT="$(realpath -- "$1" 2>/dev/null)" || \
    fail "artifact directory is unavailable"
  test -d "${ARTIFACT_ROOT}" || fail "artifact directory is unavailable"
  verify_executor_runtime
  run_executor_oauth_sql_matrix
  cleanup

  local leaked_path
  for leaked_path in \
    /etc/pggomtm \
    /tmp/mtmpg-executor-pgdata \
    /tmp/mtmpg-executor-runtime; do
    test ! -e "${leaked_path}" || fail "executor cleanup left runtime state: ${leaked_path}"
  done
}

run_all() {
  test "$#" -eq 1 || fail "run requires exactly one artifact directory"
  ARTIFACT_ROOT="$(realpath -- "$1" 2>/dev/null)" || \
    fail "artifact directory is unavailable"
  test -d "${ARTIFACT_ROOT}" || fail "artifact directory is unavailable"

  verify_runtime
  run_abi_runtime_matrix
  run_oauth_gate_matrix
  run_production_backend_smoke
  cleanup

  local leaked_path
  for leaked_path in \
    /etc/pggomtm \
    /tmp/pggomtm-abi-pgdata \
    /tmp/pggomtm-oauth-pgdata \
    /tmp/pggomtm-production-backend-pgdata; do
    test ! -e "${leaked_path}" || fail "integration cleanup left runtime state: ${leaked_path}"
  done
  printf 'PG18 PostgreSQL integration harness passed\n'
}

case "${1:-}" in
  help|--help|-h)
    usage
    ;;
  run)
    require_github_actions
    shift
    run_all "$@"
    ;;
  run-executor)
    require_github_actions
    shift
    run_executor "$@"
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
