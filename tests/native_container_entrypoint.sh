#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly ENTRYPOINT="${REPOSITORY_ROOT}/scripts/native-container"
readonly BUILDER_IMAGE="rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc"

fail() {
  printf 'native container entrypoint policy failed: %s\n' "$1" >&2
  exit 1
}

test -x "${ENTRYPOINT}" || fail "scripts/native-container is not executable"
grep --quiet --fixed-strings -- "${BUILDER_IMAGE}" "${ENTRYPOINT}" || \
  fail "native container does not pin the approved builder digest"

help_output="$("${ENTRYPOINT}" help)"
for command in create prepare exec destroy; do
  grep --quiet --fixed-strings -- "${command}" <<<"${help_output}" || \
    fail "native container help omitted command: ${command}"
done

if grep --quiet --fixed-strings 'docker build' "${ENTRYPOINT}"; then
  fail "native container delegates test setup to Dockerfile"
fi

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT
readonly DOCKER_LOG="${TEMP_ROOT}/docker.log"
readonly REMOVE_MARKER="${TEMP_ROOT}/removed"
readonly LOCAL_REJECTION="${TEMP_ROOT}/local-rejection.out"
cat >"${TEMP_ROOT}/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${PGGOMTM_TEST_DOCKER_LOG}"
case "${1:-}" in
  inspect) exit 1 ;;
  create) printf '%s\n' 'synthetic-native-container' ;;
  rm) touch "${PGGOMTM_TEST_REMOVE_MARKER}" ;;
esac
EOF
chmod 0755 "${TEMP_ROOT}/docker"

export PGGOMTM_DOCKER_BIN="${TEMP_ROOT}/docker"
export PGGOMTM_NATIVE_CONTAINER="pggomtm-native-test"
export PGGOMTM_TEST_DOCKER_LOG="${DOCKER_LOG}"
export PGGOMTM_TEST_REMOVE_MARKER="${REMOVE_MARKER}"

if env -u GITHUB_ACTIONS "${ENTRYPOINT}" create >"${LOCAL_REJECTION}" 2>&1; then
  fail "native container accepted local recomputation"
fi
test ! -e "${DOCKER_LOG}" || fail "native container touched Docker before rejecting local use"
grep --quiet --fixed-strings -- 'recomputation is restricted to GitHub Actions' \
  "${LOCAL_REJECTION}" || fail "native container returned an unstable local rejection"

export GITHUB_ACTIONS=true
"${ENTRYPOINT}" create
"${ENTRYPOINT}" prepare
"${ENTRYPOINT}" exec cargo-tests
"${ENTRYPOINT}" destroy

test -f "${REMOVE_MARKER}" || fail "native container destroy did not remove the container"
grep --quiet --fixed-strings -- "pull ${BUILDER_IMAGE}" "${DOCKER_LOG}" || \
  fail "native container did not pull the pinned builder"
grep --quiet --fixed-strings -- \
  "type=bind,src=${REPOSITORY_ROOT},dst=/workspace" "${DOCKER_LOG}" || \
  fail "native container did not mount the exact repository"
grep --quiet --fixed-strings -- '--env GITHUB_ACTIONS=true' "${DOCKER_LOG}" || \
  fail "native container did not propagate the Actions-only guard"
grep --quiet --fixed-strings -- 'scripts/native-test prepare' "${DOCKER_LOG}" || \
  fail "native container prepare did not call the direct toolchain entrypoint"
grep --quiet --fixed-strings -- 'scripts/native-test cargo-tests' "${DOCKER_LOG}" || \
  fail "native container exec did not call the requested direct gate"

printf 'native container entrypoint policy passed\n'
