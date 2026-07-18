#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly ENTRYPOINT="${REPOSITORY_ROOT}/scripts/native-test"

fail() {
  printf 'native test entrypoint policy failed: %s\n' "$1" >&2
  exit 1
}

test -x "${ENTRYPOINT}" || fail "scripts/native-test is not executable"

help_output="$("${ENTRYPOINT}" help)"
for command in \
  prepare \
  policy \
  dependencies \
  abi \
  cargo-tests \
  quality \
  production-artifact \
  stage-integration; do
  grep --quiet --fixed-strings -- "${command}" <<<"${help_output}" || \
    fail "help omitted direct gate command: ${command}"
done

if local_error="$(env -u GITHUB_ACTIONS \
  "${ENTRYPOINT}" cargo-tests 2>&1)"; then
  fail "native test entrypoint accepted local recomputation"
fi
grep --quiet --fixed-strings -- 'recomputation is restricted to GitHub Actions' \
  <<<"${local_error}" || fail "native test entrypoint returned an unstable local rejection"

export GITHUB_ACTIONS=true

if grep --quiet --fixed-strings 'docker build' "${ENTRYPOINT}"; then
  fail "native test entrypoint delegates test execution to Dockerfile"
fi

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT
install -d \
  "${TEMP_ROOT}/bin" \
  "${TEMP_ROOT}/include/server/libpq" \
  "${TEMP_ROOT}/lib"

cat >"${TEMP_ROOT}/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${PGGOMTM_TEST_COMMAND_LOG}"
if test "${1:-}" = '--version'; then
  printf '%s\n' 'cargo 1.97.1 (c980f4866 2026-06-30)'
  exit 0
fi
if test "${1:-}" = 'deny' && test "${2:-}" = '--version'; then
  printf '%s\n' 'cargo-deny 0.20.2'
  exit 0
fi
if test "${1:-}" = 'tree'; then
  printf '%s\n' 'pggomtm v0.1.0'
fi
if test "${1:-}" = 'build' && [[ " $* " == *' --release '* ]]; then
  target_dir="${CARGO_TARGET_DIR:-target}"
  install -d "${target_dir}/release/build/pggomtm-test/out"
  printf '%s\n' 'synthetic module' >"${target_dir}/release/libpggomtm.so"
  printf '%s\n' '{}' \
    >"${target_dir}/release/build/pggomtm-test/out/pggomtm_build_identity.json"
  if [[ " $* " == *' --example pggomtm_oauth_smoke_fixture '* ]]; then
    install -d "${target_dir}/release/examples"
    printf '%s\n' '#!/usr/bin/env bash' \
      >"${target_dir}/release/examples/pggomtm_oauth_smoke_fixture"
    chmod 0755 "${target_dir}/release/examples/pggomtm_oauth_smoke_fixture"
  fi
fi
EOF
chmod 0755 "${TEMP_ROOT}/bin/cargo"

cat >"${TEMP_ROOT}/bin/rustc" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
test "${1:-}" = '--version'
printf '%s\n' 'rustc 1.97.1 (8bab26f4f 2026-07-14)'
EOF
chmod 0755 "${TEMP_ROOT}/bin/rustc"

cat >"${TEMP_ROOT}/bin/cc" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${PGGOMTM_TEST_COMMAND_LOG}"
output=
while test "$#" -gt 0; do
  if test "$1" = '-o'; then
    output="$2"
    shift 2
  else
    shift
  fi
done
test -n "${output}"
printf '%s\n' '#!/usr/bin/env bash' 'touch "${PGGOMTM_TEST_PROBE_MARKER}"' >"${output}"
chmod 0700 "${output}"
EOF
chmod 0755 "${TEMP_ROOT}/bin/cc"

cat >"${TEMP_ROOT}/bin/sha256sum" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
test "${1:-}" = '--check'
cat >/dev/null
EOF
chmod 0755 "${TEMP_ROOT}/bin/sha256sum"

cat >"${TEMP_ROOT}/bin/shellcheck" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if test "${1:-}" = '--version'; then
  printf '%s\n' 'ShellCheck - shell script analysis tool' 'version: 0.11.0'
  exit 0
fi
printf 'shellcheck %s\n' "$*" >>"${PGGOMTM_TEST_COMMAND_LOG}"
EOF
chmod 0755 "${TEMP_ROOT}/bin/shellcheck"

cat >"${TEMP_ROOT}/bin/gitleaks" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' '8.30.1'
EOF
chmod 0755 "${TEMP_ROOT}/bin/gitleaks"

cat >"${TEMP_ROOT}/pg_config" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\${1:-}" in
  --version) printf '%s\n' 'PostgreSQL 18.4' ;;
  --includedir-server) printf '%s\n' '${TEMP_ROOT}/include/server' ;;
  --includedir) printf '%s\n' '${TEMP_ROOT}/include' ;;
  --libdir) printf '%s\n' '${TEMP_ROOT}/lib' ;;
  --cppflags) printf '%s\n' '-DFAKE_PG_CPPFLAG=1' ;;
  *) exit 2 ;;
esac
EOF
chmod 0755 "${TEMP_ROOT}/pg_config"
printf '%s\n' 'synthetic oauth header' >"${TEMP_ROOT}/include/server/libpq/oauth.h"

cat >"${TEMP_ROOT}/artifact-readiness" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'artifact-readiness %s\n' "$*" >>"${PGGOMTM_TEST_COMMAND_LOG}"
if test "${1:-}" = 'create-build-manifest'; then
  printf '%s\n' '{}' >"$5"
fi
EOF
chmod 0755 "${TEMP_ROOT}/artifact-readiness"

cat >"${TEMP_ROOT}/artifact-gate-test" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
touch "${PGGOMTM_TEST_ARTIFACT_POLICY_MARKER}"
EOF
chmod 0755 "${TEMP_ROOT}/artifact-gate-test"

cat >"${TEMP_ROOT}/public-gate-test" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
test "$1" = "${PGGOMTM_TEST_GITLEAKS_BIN}"
touch "${PGGOMTM_TEST_PUBLIC_POLICY_MARKER}"
EOF
chmod 0755 "${TEMP_ROOT}/public-gate-test"

readonly COMMAND_LOG="${TEMP_ROOT}/commands.log"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" cargo-tests

for invocation in \
  'test --locked --no-default-features --features pg18,abi-gate' \
  'test --locked --no-default-features --features pg18,abi-gate,pgx-oauth-gate --test artifact_identity --test pgx_oauth_gate' \
  'test --locked --no-default-features --features pg18,abi-runtime-gate --test artifact_identity' \
  'test --locked --no-default-features --features pg18 --test artifact_identity'; do
  grep --quiet --fixed-strings --line-regexp -- "${invocation}" "${COMMAND_LOG}" || \
    fail "cargo-tests omitted direct invocation: ${invocation}"
done

: >"${COMMAND_LOG}"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" dependencies
grep --quiet --fixed-strings --line-regexp -- \
  'deny --locked --no-default-features --features pg18 check --show-stats advisories licenses bans sources' \
  "${COMMAND_LOG}" || fail "dependencies omitted the locked cargo-deny gate"

: >"${COMMAND_LOG}"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" quality
for invocation in \
  'fmt --check' \
  'clippy --locked --all-targets --no-default-features --features pg18,abi-gate -- -D warnings' \
  'clippy --locked --all-targets --no-default-features --features pg18,abi-gate,pgx-oauth-gate -- -D warnings' \
  'clippy --locked --lib --no-default-features --features pg18,abi-runtime-gate -- -D warnings' \
  'clippy --locked --lib --no-default-features --features pg18 -- -D warnings'; do
  grep --quiet --fixed-strings --line-regexp -- "${invocation}" "${COMMAND_LOG}" || \
    fail "quality omitted direct invocation: ${invocation}"
done

: >"${COMMAND_LOG}"
readonly PROBE_MARKER="${TEMP_ROOT}/probe-ran"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_TEST_PROBE_MARKER="${PROBE_MARKER}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" abi
test -f "${PROBE_MARKER}" || fail "ABI command did not execute the compiled C layout probe"
grep --quiet --fixed-strings -- \
  '-std=c11 -Wall -Wextra -Werror -DFAKE_PG_CPPFLAG=1' \
  "${COMMAND_LOG}" || fail "ABI command omitted the target PostgreSQL C flags"
grep --quiet --fixed-strings --line-regexp -- \
  'test --locked --no-default-features --features pg18,abi-gate --test oauth_build_provenance -- --ignored --exact real_generator_rejects_unapproved_provenance_inputs' \
  "${COMMAND_LOG}" || fail "ABI command omitted the bindings provenance test"

: >"${COMMAND_LOG}"
readonly TEST_TARGET_DIR="${TEMP_ROOT}/target"
readonly TEST_ARTIFACT_DIR="${TEMP_ROOT}/artifacts"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_ARTIFACT_READINESS_BIN="${TEMP_ROOT}/artifact-readiness" \
PGGOMTM_NATIVE_ARTIFACT_DIR="${TEST_ARTIFACT_DIR}" \
CARGO_TARGET_DIR="${TEST_TARGET_DIR}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" production-artifact
test -f "${TEST_ARTIFACT_DIR}/pggomtm.so" || \
  fail "production-artifact did not stage the production module"
test -f "${TEST_ARTIFACT_DIR}/build-manifest.json" || \
  fail "production-artifact did not stage the build manifest"
grep --quiet --fixed-strings --line-regexp -- \
  'build --locked --release --lib --no-default-features --features pg18' \
  "${COMMAND_LOG}" || fail "production-artifact omitted the locked production build"
grep --quiet --fixed-strings -- \
  'artifact-readiness verify-elf' "${COMMAND_LOG}" || \
  fail "production-artifact omitted direct ELF verification"
grep --quiet --fixed-strings --line-regexp -- \
  'test --locked --no-default-features --features pg18,abi-gate --test production_capability_gate -- --ignored' \
  "${COMMAND_LOG}" || fail "production-artifact omitted the production capability gate"

: >"${COMMAND_LOG}"
readonly ARTIFACT_POLICY_MARKER="${TEMP_ROOT}/artifact-policy-ran"
readonly PUBLIC_POLICY_MARKER="${TEMP_ROOT}/public-policy-ran"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_TEST_ARTIFACT_POLICY_MARKER="${ARTIFACT_POLICY_MARKER}" \
PGGOMTM_TEST_PUBLIC_POLICY_MARKER="${PUBLIC_POLICY_MARKER}" \
PGGOMTM_TEST_GITLEAKS_BIN="${TEMP_ROOT}/bin/gitleaks" \
PGGOMTM_SHELLCHECK_BIN="${TEMP_ROOT}/bin/shellcheck" \
PGGOMTM_GITLEAKS_BIN="${TEMP_ROOT}/bin/gitleaks" \
PGGOMTM_ARTIFACT_GATE_TEST="${TEMP_ROOT}/artifact-gate-test" \
PGGOMTM_PUBLIC_GATE_TEST="${TEMP_ROOT}/public-gate-test" \
  "${ENTRYPOINT}" policy
test -f "${ARTIFACT_POLICY_MARKER}" || fail "policy omitted the artifact gate fixture"
test -f "${PUBLIC_POLICY_MARKER}" || fail "policy omitted the public-readiness fixture"
grep --quiet --fixed-strings -- 'scripts/native-test' "${COMMAND_LOG}" || \
  fail "policy omitted ShellCheck for the direct native entrypoint"

rm -f "${ARTIFACT_POLICY_MARKER}" "${PUBLIC_POLICY_MARKER}"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_TEST_ARTIFACT_POLICY_MARKER="${ARTIFACT_POLICY_MARKER}" \
PGGOMTM_TEST_PUBLIC_POLICY_MARKER="${PUBLIC_POLICY_MARKER}" \
PGGOMTM_TEST_GITLEAKS_BIN=gitleaks \
PGGOMTM_SHELLCHECK_BIN=shellcheck \
PGGOMTM_GITLEAKS_BIN=gitleaks \
PGGOMTM_ARTIFACT_GATE_TEST="${TEMP_ROOT}/artifact-gate-test" \
PGGOMTM_PUBLIC_GATE_TEST="${TEMP_ROOT}/public-gate-test" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" policy
test -f "${ARTIFACT_POLICY_MARKER}" && test -f "${PUBLIC_POLICY_MARKER}" || \
  fail "policy did not resolve pinned tools from PATH"

: >"${COMMAND_LOG}"
readonly TEST_INTEGRATION_DIR="${TEST_TARGET_DIR}/integration-artifacts"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_TEST_PROBE_MARKER="${PROBE_MARKER}" \
PGGOMTM_NATIVE_INTEGRATION_DIR="${TEST_INTEGRATION_DIR}" \
CARGO_TARGET_DIR="${TEST_TARGET_DIR}" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" stage-integration
test "$(stat --format='%a' "${TEST_INTEGRATION_DIR}")" = '755' || \
  fail "stage-integration directory is not readable by the host harness"
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
  runtime_config_validate_probe.sql \
  runtime-config-fixture/validator.json \
  runtime-config-fixture/jwks.json; do
  test -f "${TEST_INTEGRATION_DIR}/${artifact}" || \
    fail "stage-integration omitted artifact: ${artifact}"
done
test "$(stat --format='%a' "${TEST_INTEGRATION_DIR}/pggomtm_abi_runtime_probe.so")" = '644' || \
  fail "stage-integration probe mode is not host-readable"
test "$(stat --format='%a' "${TEST_INTEGRATION_DIR}/pggomtm_oauth_smoke_client")" = '755' || \
  fail "stage-integration smoke client mode is not host-executable"
test "$(stat --format='%a' "${TEST_INTEGRATION_DIR}/pggomtm_oauth_smoke_fixture")" = '755' || \
  fail "stage-integration fixture mode is not host-executable"
grep --quiet --fixed-strings --line-regexp -- \
  'build --locked --release --no-default-features --features pg18,abi-runtime-gate' \
  "${COMMAND_LOG}" || fail "stage-integration omitted the ABI runtime module"
grep --quiet --fixed-strings --line-regexp -- \
  'build --locked --release --no-default-features --features pg18,pgx-oauth-gate' \
  "${COMMAND_LOG}" || fail "stage-integration omitted the OAuth gate module"

readonly PREPARED_MARKER="${TEMP_ROOT}/native-prepared"
touch "${PREPARED_MARKER}"
PGGOMTM_TEST_COMMAND_LOG="${COMMAND_LOG}" \
PGGOMTM_NATIVE_PREPARED_MARKER="${PREPARED_MARKER}" \
PGGOMTM_SHELLCHECK_BIN="${TEMP_ROOT}/bin/shellcheck" \
PGGOMTM_GITLEAKS_BIN="${TEMP_ROOT}/bin/gitleaks" \
PGRX_PG_CONFIG_PATH="${TEMP_ROOT}/pg_config" \
PATH="${TEMP_ROOT}/bin:${PATH}" \
  "${ENTRYPOINT}" prepare

printf 'native test entrypoint policy passed\n'
