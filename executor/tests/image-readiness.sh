#!/usr/bin/env bash
set -euo pipefail

umask 077
export LC_ALL=C

fail() {
  printf 'executor image readiness: %s\n' "$1" >&2
  exit 2
}

test "${GITHUB_ACTIONS:-}" = "true" || fail "run this gate through GitHub Actions"
test "$#" -eq 4 || fail "expected image, source, version, and integration artifacts"
command -v curl >/dev/null || fail "curl is unavailable"
command -v docker >/dev/null || fail "Docker is unavailable"

readonly IMAGE="$1"
readonly SOURCE="$2"
readonly VERSION="$3"
ARTIFACT_ROOT="$(realpath -- "$4")" || fail "integration artifacts cannot be resolved"
readonly ARTIFACT_ROOT
test -d "${ARTIFACT_ROOT}" || fail "integration artifacts are unavailable"
test -n "${PGGOMTM_POSTGRES_IMAGE:-}" || fail "resolved PostgreSQL runtime is unavailable"

readonly RUNNER_TEMP_ROOT="${RUNNER_TEMP:?RUNNER_TEMP is unavailable}"
RUNTIME_ROOT="$(mktemp --directory "${RUNNER_TEMP_ROOT}/executor-image-runtime.XXXXXX")"
INSPECTION_CONTAINER=""
SERVICE_CONTAINER=""

cleanup() {
  if test -n "${SERVICE_CONTAINER}"; then
    docker rm --force "${SERVICE_CONTAINER}" >/dev/null 2>&1 || true
  fi
  if test -n "${INSPECTION_CONTAINER}"; then
    docker rm --force "${INSPECTION_CONTAINER}" >/dev/null 2>&1 || true
  fi
  if test -d "${RUNTIME_ROOT}"; then
    sudo rm -rf -- "${RUNTIME_ROOT}"
  fi
}
trap cleanup EXIT

test "$(docker image inspect --format '{{.Config.User}}' "${IMAGE}")" = "10001:10001" || \
  fail "image does not use the fixed non-root identity"
test "$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "${IMAGE}")" = "${SOURCE}" || \
  fail "image source label does not match"
test "$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.version" }}' "${IMAGE}")" = "${VERSION}" || \
  fail "image version label does not match"

if docker image inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "${IMAGE}" \
  | grep --extended-regexp --quiet 'MTMPG_EXECUTOR|SECRET|TOKEN|PRIVATE_KEY|DATABASE_URL'; then
  fail "image config contains runtime credential material"
fi

docker run --rm --user 0:0 --entrypoint /bin/sh "${IMAGE}" -ec '
  test -x /usr/local/bin/mtmpg-executor
  test -f /usr/share/doc/mtmpg-executor/LICENSE
  test ! -e /usr/local/cargo
  test ! -e /src
  test ! -e /tests
  test ! -e /var/lib/postgresql
  test ! -e /usr/lib/postgresql/18/lib/pggomtm.so
  ! command -v cargo >/dev/null 2>&1
  ! command -v rustc >/dev/null 2>&1
  ! command -v cc >/dev/null 2>&1
  ! command -v clang >/dev/null 2>&1
  ! command -v pg_config >/dev/null 2>&1
  ! command -v postgres >/dev/null 2>&1
  ! command -v initdb >/dev/null 2>&1
  ! command -v pg_ctl >/dev/null 2>&1
  test ! -e /run/executor/hmac.secret
  test ! -e /run/executor/signing-key.pem
  test ! -e /run/executor/jwks.json
' || fail "image contains build, source, test, server, validator, or secret material"

linkage="$(docker run --rm --entrypoint /usr/bin/ldd "${IMAGE}" /usr/local/bin/mtmpg-executor 2>&1)" || \
  fail "executor dynamic linkage cannot be inspected"
if grep --quiet 'not found' <<<"${linkage}"; then
  fail "executor has unresolved dynamic dependencies"
fi
if ! grep --quiet 'libpq[.]so[.]5' <<<"${linkage}"; then
  fail "executor is not linked to the required libpq runtime"
fi

install -d -m 0700 "${RUNTIME_ROOT}/mount"
install -m 0400 \
  "${ARTIFACT_ROOT}/runtime/hmac.secret" \
  "${ARTIFACT_ROOT}/runtime/signing-key.pem" \
  "${ARTIFACT_ROOT}/runtime/executor.key" \
  "${RUNTIME_ROOT}/mount"
install -m 0444 \
  "${ARTIFACT_ROOT}/runtime/ca.crt" \
  "${ARTIFACT_ROOT}/runtime/executor.crt" \
  "${RUNTIME_ROOT}/mount"
sudo chown -R 10001:10001 "${RUNTIME_ROOT}/mount"
sudo chmod 0500 "${RUNTIME_ROOT}/mount"

SERVICE_CONTAINER="$(docker run --detach \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  --pids-limit 64 \
  --mount "type=bind,source=${RUNTIME_ROOT}/mount,target=/run/executor,readonly" \
  --publish 127.0.0.1::8443 \
  --env MTMPG_EXECUTOR_AUDIENCE=https://postgres.example.test/database/main \
  --env MTMPG_EXECUTOR_HMAC_SECRET_PATH=/run/executor/hmac.secret \
  --env MTMPG_EXECUTOR_ISSUER=https://auth.example.test/database \
  --env MTMPG_EXECUTOR_KEY_ID=executor-es256-test \
  --env MTMPG_EXECUTOR_LISTEN=0.0.0.0:8443 \
  --env MTMPG_EXECUTOR_POSTGRES_CA_PATH=/run/executor/ca.crt \
  --env MTMPG_EXECUTOR_SIGNING_KEY_PATH=/run/executor/signing-key.pem \
  --env MTMPG_EXECUTOR_TLS_CERT_PATH=/run/executor/executor.crt \
  --env MTMPG_EXECUTOR_TLS_KEY_PATH=/run/executor/executor.key \
  "${IMAGE}")" || fail "image did not start"

host_port="$(docker inspect --format '{{(index (index .NetworkSettings.Ports "8443/tcp") 0).HostPort}}' "${SERVICE_CONTAINER}")"
test -n "${host_port}" || fail "HTTPS port was not published"
ready=0
curl_status=0
for _ in $(seq 1 80); do
  if curl --fail --silent --show-error \
    --noproxy '*' \
    --cacert "${ARTIFACT_ROOT}/runtime/ca.crt" \
    --resolve "executor:${host_port}:127.0.0.1" \
    "https://executor:${host_port}/ready" >/dev/null 2>&1; then
    ready=1
    break
  else
    curl_status=$?
  fi
  if ! docker inspect --format '{{.State.Running}}' "${SERVICE_CONTAINER}" | grep --quiet '^true$'; then
    break
  fi
  sleep 0.25
done
docker logs "${SERVICE_CONTAINER}" >"${RUNTIME_ROOT}/service.log" 2>&1
if test "${ready}" -ne 1; then
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
    if grep --quiet "^executor startup failed: ${startup_stage}$" "${RUNTIME_ROOT}/service.log"; then
      fail "image exited during ${startup_stage} startup"
    fi
  done
  fail "image HTTPS readiness failed with curl status ${curl_status}"
fi

hmac_secret="$(tr -d '\n' <"${ARTIFACT_ROOT}/runtime/hmac.secret")"
if test -n "${hmac_secret}" && grep --fixed-strings --quiet "${hmac_secret}" "${RUNTIME_ROOT}/service.log"; then
  fail "service log disclosed the HMAC secret"
fi
if grep --quiet 'BEGIN .*PRIVATE KEY' "${RUNTIME_ROOT}/service.log"; then
  fail "service log disclosed the signing key"
fi
docker rm --force "${SERVICE_CONTAINER}" >/dev/null
SERVICE_CONTAINER=""

INSPECTION_CONTAINER="$(docker create "${IMAGE}")"
docker cp \
  "${INSPECTION_CONTAINER}:/usr/local/bin/mtmpg-executor" \
  "${ARTIFACT_ROOT}/mtmpg-executor"
chmod 0755 "${ARTIFACT_ROOT}/mtmpg-executor"

GITHUB_ACTIONS=true \
PGGOMTM_POSTGRES_IMAGE="${PGGOMTM_POSTGRES_IMAGE}" \
  tests/postgres_integration.sh run-executor "${ARTIFACT_ROOT}"

printf 'Executor final image readiness and PG18 matrix passed\n'
