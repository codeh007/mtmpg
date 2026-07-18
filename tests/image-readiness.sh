#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly DOCKER_BIN="${PGGOMTM_DOCKER_BIN:-docker}"

fail() {
  printf 'image-readiness: %s\n' "$1" >&2
  exit 2
}

test "${GITHUB_ACTIONS:-}" = "true" || fail "run this harness through GitHub Actions"
test "$#" -eq 4 || fail "usage: tests/image-readiness.sh IMAGE SOURCE VERSION ARTIFACT_DIR"

readonly IMAGE="$1"
readonly SOURCE_REVISION="$2"
readonly VERSION="$3"
ARTIFACT_ROOT="$(realpath -- "$4")" || fail "artifact directory cannot be resolved"
readonly ARTIFACT_ROOT

[[ "${SOURCE_REVISION}" =~ ^[0-9a-f]{40}$ ]] || fail "source must be a full Git commit"
[[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+(\.[0-9]+)*)?$ ]] || \
  fail "version is not an mtmpg SemVer candidate"
test -x "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_client" || fail "OAuth client is unavailable"
test -x "${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture" || fail "OAuth fixture is unavailable"
command -v "${DOCKER_BIN}" >/dev/null || fail "Docker is unavailable"

SESSION_ROOT="$(mktemp --directory)"
RUNTIME_CONTAINER=""
cleanup() {
  if test -n "${RUNTIME_CONTAINER}"; then
    "${DOCKER_BIN}" rm --force "${RUNTIME_CONTAINER}" >/dev/null 2>&1 || true
  fi
  chmod -R u+w -- "${SESSION_ROOT}" 2>/dev/null || true
  rm -rf -- "${SESSION_ROOT}"
}
trap cleanup EXIT INT TERM

test "$(
  "${DOCKER_BIN}" image inspect \
    --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' \
    "${IMAGE}"
)" = "${SOURCE_REVISION}" || fail "image source label does not match"
test "$(
  "${DOCKER_BIN}" image inspect \
    --format '{{ index .Config.Labels "org.opencontainers.image.version" }}' \
    "${IMAGE}"
)" = "${VERSION}" || fail "image version label does not match"

# shellcheck disable=SC2016
"${DOCKER_BIN}" run --rm --platform linux/amd64 --entrypoint sh "${IMAGE}" -ceu '
  test -f /usr/lib/postgresql/18/lib/pggomtm.so
  test -f /usr/share/doc/pggomtm/LICENSE
  test ! -e /src
  test ! -e /test-artifacts
  for tool in cargo rustc cc clang; do
    ! command -v "$tool" >/dev/null
  done
  pg_config --version | grep -Eq "^PostgreSQL 18\."
  ! ldd /usr/lib/postgresql/18/lib/pggomtm.so | grep -q "not found"
'

CONTENT_CONTAINER="$("${DOCKER_BIN}" create --platform linux/amd64 "${IMAGE}")"
"${DOCKER_BIN}" cp \
  "${CONTENT_CONTAINER}:/usr/lib/postgresql/18/lib/pggomtm.so" \
  "${SESSION_ROOT}/pggomtm.so"
"${DOCKER_BIN}" rm "${CONTENT_CONTAINER}" >/dev/null
strings --all "${SESSION_ROOT}/pggomtm.so" >"${SESSION_ROOT}/module.strings"
for forbidden in \
  pggomtm_abi_runtime_probe \
  pggomtm_pgx_gate \
  oauth-ordinary.jwt \
  candidate.example.test \
  'BEGIN PRIVATE KEY' \
  'BEGIN RSA PRIVATE KEY' \
  'BEGIN EC PRIVATE KEY'; do
  if grep --quiet --fixed-strings -- "${forbidden}" "${SESSION_ROOT}/module.strings"; then
    fail "production module contains test or private material: ${forbidden}"
  fi
done

install -d -m 0755 "${SESSION_ROOT}/config"
install -m 0444 \
  "${REPOSITORY_ROOT}/tests/fixtures/runtime-config/validator.json" \
  "${SESSION_ROOT}/config/validator.json"
install -m 0444 \
  "${REPOSITORY_ROOT}/tests/fixtures/runtime-config/jwks.json" \
  "${SESSION_ROOT}/config/jwks.json"
chmod 0555 "${SESSION_ROOT}/config"
install -d -m 0700 "${SESSION_ROOT}/fixtures"
"${ARTIFACT_ROOT}/pggomtm_oauth_smoke_fixture" generate "${SESSION_ROOT}/fixtures"

cat >"${SESSION_ROOT}/init.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
psql --host=/tmp --set ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<'SQL'
CREATE ROLE gomtm_candidate_ordinary LOGIN;
SQL
cat >"$PGDATA/pg_hba.conf" <<'HBA'
local all postgres trust
local all gomtm_candidate_ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm delegate_ident_mapping=1
local all all reject
HBA
EOF
chmod 0555 "${SESSION_ROOT}/init.sh"

RUNTIME_CONTAINER="pggomtm-image-${GITHUB_RUN_ID:-ci}-$$"
"${DOCKER_BIN}" run \
  --detach \
  --name "${RUNTIME_CONTAINER}" \
  --platform linux/amd64 \
  --env POSTGRES_HOST_AUTH_METHOD=trust \
  --mount "type=bind,src=${SESSION_ROOT}/init.sh,dst=/docker-entrypoint-initdb.d/10-pggomtm.sh,readonly" \
  --mount "type=bind,src=${SESSION_ROOT}/config,dst=/etc/pggomtm,readonly" \
  --mount "type=bind,src=${ARTIFACT_ROOT},dst=/test-artifacts,readonly" \
  --mount "type=bind,src=${SESSION_ROOT}/fixtures,dst=/fixtures" \
  --tmpfs /var/lib/postgresql:rw,nosuid,nodev,size=512m \
  "${IMAGE}" \
  -c oauth_validator_libraries=pggomtm \
  -c unix_socket_directories=/tmp \
  -c log_min_messages=log >/dev/null

ready=0
for _ in $(seq 1 60); do
  if "${DOCKER_BIN}" exec "${RUNTIME_CONTAINER}" \
    pg_isready --host=/tmp --username=postgres --dbname=postgres >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if test "${ready}" -ne 1; then
  "${DOCKER_BIN}" logs "${RUNTIME_CONTAINER}" >&2 || true
  fail "official entrypoint did not start PostgreSQL"
fi

test "$(
  "${DOCKER_BIN}" exec "${RUNTIME_CONTAINER}" \
    psql --host=/tmp --username=postgres --dbname=postgres --tuples-only --no-align \
      --command='SHOW oauth_validator_libraries'
)" = "pggomtm" || fail "production module was not loaded"

"${DOCKER_BIN}" exec "${RUNTIME_CONTAINER}" \
  /test-artifacts/pggomtm_oauth_smoke_client \
  --expect-allowed \
  /fixtures/oauth-ordinary.jwt \
  gomtm_candidate_ordinary \
  /fixtures/oauth-ordinary.system-user
"${DOCKER_BIN}" exec "${RUNTIME_CONTAINER}" \
  /test-artifacts/pggomtm_oauth_smoke_fixture \
  verify-system-user \
  oauth-ordinary \
  /fixtures/oauth-ordinary.system-user
"${DOCKER_BIN}" exec "${RUNTIME_CONTAINER}" \
  /test-artifacts/pggomtm_oauth_smoke_client \
  --expect-rejected \
  /fixtures/tampered.jwt \
  gomtm_candidate_ordinary

"${DOCKER_BIN}" stop --time 10 "${RUNTIME_CONTAINER}" >/dev/null
"${DOCKER_BIN}" rm "${RUNTIME_CONTAINER}" >/dev/null
RUNTIME_CONTAINER=""
printf 'final PG18 image behavior passed: %s\n' "${IMAGE}"
