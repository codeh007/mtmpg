#!/usr/bin/env bash
set -euo pipefail

umask 077

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPOSITORY_ROOT
readonly GATE="${REPOSITORY_ROOT}/scripts/public-readiness"
readonly FIXTURE="${REPOSITORY_ROOT}/tests/fixtures/public-readiness/synthetic-private-key.pem"
readonly GITLEAKS_BIN="${1:-${PGGOMTM_GITLEAKS_BIN:-gitleaks}}"

TEMP_ROOT="$(mktemp --directory)"
readonly TEMP_ROOT
trap 'rm -rf "${TEMP_ROOT}"' EXIT

fail() {
  printf 'public-readiness gate test failed: %s\n' "$1" >&2
  exit 1
}

assert_contains() {
  local file_path="$1"
  local expected="$2"

  grep --quiet --fixed-strings -- "${expected}" "${file_path}" || \
    fail "${file_path#"${REPOSITORY_ROOT}/"} does not contain required gate wiring"
}

assert_absent() {
  local file_path="$1"
  local forbidden="$2"

  if grep --ignore-case --quiet --fixed-strings -- "${forbidden}" "${file_path}"; then
    fail "${file_path#"${REPOSITORY_ROOT}/"} contains forbidden workflow authority: ${forbidden}"
  fi
}

assert_redacted() {
  local output_path="$1"

  if grep --quiet --fixed-strings --file="${FIXTURE}" "${output_path}"; then
    fail "scanner output disclosed the synthetic sentinel"
  fi
}

initialize_repository() {
  local repository="$1"

  git init --quiet --initial-branch=main "${repository}"
  git -C "${repository}" config user.name "Public Readiness Test"
  git -C "${repository}" config user.email "public-readiness@example.test"
  install -d "${repository}/tests/fixtures/public-readiness"
  install -m 0600 "${FIXTURE}" \
    "${repository}/tests/fixtures/public-readiness/synthetic-private-key.pem"
  git -C "${repository}" add tests/fixtures/public-readiness/synthetic-private-key.pem
  git -C "${repository}" commit --quiet --message "test: add approved sentinel"
}

initialize_surface_bundle() {
  local bundle_root="$1"
  local cache_status="$2"
  local cache_count="$3"

  local surface
  for surface in \
    git_refs_history \
    tracked_uncommitted \
    docker_context \
    workflow_source \
    workflow_logs \
    actions_artifacts \
    actions_caches \
    releases_packages \
    github_issues \
    github_pull_requests \
    candidate_image; do
    install -d "${bundle_root}/surfaces/${surface}"
  done

  printf '%s\n' \
    '{' \
    '  "schema": "pggomtm-public-readiness-bundle/v1",' \
    '  "surfaces": {' \
    '    "git_refs_history": {"status": "materialized", "count": 1},' \
    '    "tracked_uncommitted": {"status": "materialized", "count": 1},' \
    '    "docker_context": {"status": "materialized", "count": 1},' \
    '    "workflow_source": {"status": "materialized", "count": 1},' \
    '    "workflow_logs": {"status": "absent", "count": 0},' \
    '    "actions_artifacts": {"status": "absent", "count": 0},' \
    "    \"actions_caches\": {\"status\": \"${cache_status}\", \"count\": ${cache_count}}," \
    '    "releases_packages": {"status": "absent", "count": 0},' \
    '    "github_issues": {"status": "absent", "count": 0},' \
    '    "github_pull_requests": {"status": "absent", "count": 0},' \
    '    "candidate_image": {"status": "absent", "count": 0}' \
    '  }' \
    '}' \
    >"${bundle_root}/manifest.json"
}

test -x "${GITLEAKS_BIN}" || fail "pinned gitleaks binary is not executable"
test -x "${GATE}" || fail "scripts/public-readiness is not executable"

export PGGOMTM_GITLEAKS_BIN="${GITLEAKS_BIN}"

readonly ALLOWED_ROOT="${TEMP_ROOT}/allowed"
install -d "${ALLOWED_ROOT}/tests/fixtures/public-readiness"
install -m 0600 "${FIXTURE}" \
  "${ALLOWED_ROOT}/tests/fixtures/public-readiness/synthetic-private-key.pem"
"${GATE}" scan-path "${ALLOWED_ROOT}" >"${TEMP_ROOT}/allowed.out"

readonly WRONG_PATH_ROOT="${TEMP_ROOT}/wrong-path"
install -d "${WRONG_PATH_ROOT}/tests/fixtures/public-readiness-copy"
install -m 0600 "${FIXTURE}" \
  "${WRONG_PATH_ROOT}/tests/fixtures/public-readiness-copy/synthetic-private-key.pem"
if "${GATE}" scan-path "${WRONG_PATH_ROOT}" >"${TEMP_ROOT}/wrong-path.out" 2>&1; then
  fail "synthetic sentinel was allowed outside its exact path"
fi
assert_redacted "${TEMP_ROOT}/wrong-path.out"

readonly WRONG_PATTERN_ROOT="${TEMP_ROOT}/wrong-pattern"
install -d "${WRONG_PATTERN_ROOT}/tests/fixtures/public-readiness"
sed '2s/.$/A/' "${FIXTURE}" \
  >"${WRONG_PATTERN_ROOT}/tests/fixtures/public-readiness/synthetic-private-key.pem"
chmod 0600 \
  "${WRONG_PATTERN_ROOT}/tests/fixtures/public-readiness/synthetic-private-key.pem"
if "${GATE}" scan-path "${WRONG_PATTERN_ROOT}" >"${TEMP_ROOT}/wrong-pattern.out" 2>&1; then
  fail "modified sentinel was allowed by an imprecise pattern"
fi
assert_redacted "${TEMP_ROOT}/wrong-pattern.out"

readonly INLINE_ALLOW_ROOT="${TEMP_ROOT}/inline-allow"
install -d "${INLINE_ALLOW_ROOT}"
INLINE_TOKEN="ghp_$(
  printf '%s' 'public-readiness-inline-sentinel' | sha256sum | cut -c1-36
)"
readonly INLINE_TOKEN
printf 'github_token = "%s" # gitleaks:allow\n' "${INLINE_TOKEN}" \
  >"${INLINE_ALLOW_ROOT}/inline-allow.txt"
if "${GATE}" scan-path "${INLINE_ALLOW_ROOT}" >"${TEMP_ROOT}/inline-allow.out" 2>&1; then
  fail "inline gitleaks allow comment bypassed the repository policy"
fi
if grep --quiet --fixed-strings "${INLINE_TOKEN}" \
  "${TEMP_ROOT}/inline-allow.out"; then
  fail "scanner output disclosed the inline-allow sentinel"
fi

PUBLIC_HEADER_DIGEST="$(
  printf '%s%s' \
    'be015ae68deef28a906c8739bc653ca9' \
    '0a4c6966c10f0efd3bd926efb4958bcf'
)"
readonly PUBLIC_HEADER_DIGEST
readonly APPROVED_DIGEST_ROOT="${TEMP_ROOT}/approved-public-digest"
initialize_repository "${APPROVED_DIGEST_ROOT}"
install -d "${APPROVED_DIGEST_ROOT}/scripts"
printf 'readonly %s="%s"\n' \
  OAUTH_HEADER_SHA256 \
  "${PUBLIC_HEADER_DIGEST}" \
  >"${APPROVED_DIGEST_ROOT}/scripts/native-test"
git -C "${APPROVED_DIGEST_ROOT}" add scripts/native-test
git -C "${APPROVED_DIGEST_ROOT}" commit --quiet --message "test: add public header digest"
"${GATE}" scan-source "${APPROVED_DIGEST_ROOT}" \
  >"${TEMP_ROOT}/approved-public-digest.out"

readonly WRONG_DIGEST_PATH_ROOT="${TEMP_ROOT}/wrong-public-digest-path"
initialize_repository "${WRONG_DIGEST_PATH_ROOT}"
install -d "${WRONG_DIGEST_PATH_ROOT}/scripts/native-test-copy"
printf 'readonly %s="%s"\n' \
  OAUTH_HEADER_SHA256 \
  "${PUBLIC_HEADER_DIGEST}" \
  >"${WRONG_DIGEST_PATH_ROOT}/scripts/native-test-copy/config"
git -C "${WRONG_DIGEST_PATH_ROOT}" add scripts/native-test-copy/config
git -C "${WRONG_DIGEST_PATH_ROOT}" commit --quiet --message "test: move public header digest"
if "${GATE}" scan-source "${WRONG_DIGEST_PATH_ROOT}" \
  >"${TEMP_ROOT}/wrong-public-digest-path.out" 2>&1; then
  fail "public digest allowlist accepted the wrong path"
fi
assert_redacted "${TEMP_ROOT}/wrong-public-digest-path.out"

readonly WRONG_DIGEST_VALUE_ROOT="${TEMP_ROOT}/wrong-public-digest-value"
initialize_repository "${WRONG_DIGEST_VALUE_ROOT}"
install -d "${WRONG_DIGEST_VALUE_ROOT}/scripts"
printf 'readonly %s="%s"\n' \
  OAUTH_HEADER_SHA256 \
  "${PUBLIC_HEADER_DIGEST%?}a" \
  >"${WRONG_DIGEST_VALUE_ROOT}/scripts/native-test"
git -C "${WRONG_DIGEST_VALUE_ROOT}" add scripts/native-test
git -C "${WRONG_DIGEST_VALUE_ROOT}" commit --quiet --message "test: change public header digest"
if "${GATE}" scan-source "${WRONG_DIGEST_VALUE_ROOT}" \
  >"${TEMP_ROOT}/wrong-public-digest-value.out" 2>&1; then
  fail "public digest allowlist accepted a different value"
fi
assert_redacted "${TEMP_ROOT}/wrong-public-digest-value.out"

readonly CLEAN_SOURCE_ROOT="${TEMP_ROOT}/clean-source"
initialize_repository "${CLEAN_SOURCE_ROOT}"
"${GATE}" scan-source "${CLEAN_SOURCE_ROOT}" >"${TEMP_ROOT}/clean-source.out"

readonly HISTORY_SOURCE_ROOT="${TEMP_ROOT}/history-source"
initialize_repository "${HISTORY_SOURCE_ROOT}"
install -m 0600 "${FIXTURE}" "${HISTORY_SOURCE_ROOT}/unclassified-private-key.pem"
git -C "${HISTORY_SOURCE_ROOT}" add unclassified-private-key.pem
git -C "${HISTORY_SOURCE_ROOT}" commit --quiet --message "test: add unclassified sentinel"
git -C "${HISTORY_SOURCE_ROOT}" rm --quiet unclassified-private-key.pem
git -C "${HISTORY_SOURCE_ROOT}" commit --quiet --message "test: remove unclassified sentinel"
if "${GATE}" scan-source "${HISTORY_SOURCE_ROOT}" \
  >"${TEMP_ROOT}/history-source.out" 2>&1; then
  fail "a secret removed from HEAD was not rejected from Git history"
fi
assert_redacted "${TEMP_ROOT}/history-source.out"

readonly WORKTREE_SOURCE_ROOT="${TEMP_ROOT}/worktree-source"
initialize_repository "${WORKTREE_SOURCE_ROOT}"
install -m 0600 "${FIXTURE}" "${WORKTREE_SOURCE_ROOT}/uncommitted-private-key.pem"
if "${GATE}" scan-source "${WORKTREE_SOURCE_ROOT}" \
  >"${TEMP_ROOT}/worktree-source.out" 2>&1; then
  fail "an uncommitted secret was not rejected from the worktree"
fi
assert_redacted "${TEMP_ROOT}/worktree-source.out"

readonly INSTALLED_GITLEAKS_ROOT="${TEMP_ROOT}/installed-gitleaks"
"${GATE}" install-gitleaks "${INSTALLED_GITLEAKS_ROOT}"
test "$("${INSTALLED_GITLEAKS_ROOT}/gitleaks" version)" = "8.30.1" || \
  fail "gate did not install the approved gitleaks release"
env -u PGGOMTM_GITLEAKS_BIN \
  PATH="${INSTALLED_GITLEAKS_ROOT}:${PATH}" \
  "${GATE}" scan-path "${ALLOWED_ROOT}" \
  >"${TEMP_ROOT}/path-resolved-gitleaks.out"

readonly NATIVE_WORKFLOW="${REPOSITORY_ROOT}/.github/workflows/native-ci.yml"
assert_contains "${NATIVE_WORKFLOW}" "contents: read"
assert_contains "${NATIVE_WORKFLOW}" "fetch-depth: 0"
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/public-readiness install-gitleaks \"\$RUNNER_TEMP/public-readiness-bin\""
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/public-readiness scan-source \"\$GITHUB_WORKSPACE\""
for entrypoint_fixture in \
  tests/native_container_entrypoint.sh \
  tests/native_test_entrypoint.sh \
  tests/postgres_integration_entrypoint.sh \
  tests/image_readiness_entrypoint.sh; do
  assert_contains "${NATIVE_WORKFLOW}" "run: ${entrypoint_fixture}"
done
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/native-container create"
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/native-container prepare"
for native_command in \
  policy \
  dependencies \
  abi \
  cargo-tests \
  quality \
  production-artifact \
  stage-integration; do
  assert_contains "${NATIVE_WORKFLOW}" \
    "scripts/native-container exec ${native_command}"
done
assert_contains "${NATIVE_WORKFLOW}" \
  "tests/postgres_integration.sh run target/native-integration"
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/native-container destroy"
assert_contains "${NATIVE_WORKFLOW}" \
  "SOURCE_REVISION=\${{ github.sha }}"
assert_contains "${NATIVE_WORKFLOW}" \
  "scripts/image-readiness verify \"mtmpg-native-ci:\${GITHUB_SHA}\" \"\$GITHUB_SHA\""
for forbidden_workflow in \
  'issue-116-extract-pggomtm' \
  'pull_request_target' \
  'schedule:' \
  'no-cache:' \
  'inputs.cold' \
  'packages: write' \
  'contents: write' \
  'id-token: write' \
  'attestations: write'; do
  assert_absent "${NATIVE_WORKFLOW}" "${forbidden_workflow}"
done

readonly CLEAN_BUNDLE_ROOT="${TEMP_ROOT}/clean-bundle"
initialize_surface_bundle "${CLEAN_BUNDLE_ROOT}" "absent" 0
"${GATE}" scan-bundle "${CLEAN_BUNDLE_ROOT}" >"${TEMP_ROOT}/clean-bundle.out"

readonly LOG_BUNDLE_ROOT="${TEMP_ROOT}/log-bundle"
initialize_surface_bundle "${LOG_BUNDLE_ROOT}" "absent" 0
install -m 0600 "${FIXTURE}" \
  "${LOG_BUNDLE_ROOT}/surfaces/workflow_logs/run.log"
if "${GATE}" scan-bundle "${LOG_BUNDLE_ROOT}" >"${TEMP_ROOT}/log-bundle.out" 2>&1; then
  fail "workflow log secret was not rejected from a materialized bundle"
fi
assert_redacted "${TEMP_ROOT}/log-bundle.out"

readonly CACHE_BUNDLE_ROOT="${TEMP_ROOT}/cache-bundle"
initialize_surface_bundle "${CACHE_BUNDLE_ROOT}" "unresolved" 1
printf '{"id":1,"reason":"cache content has no download API"}\n' \
  >"${CACHE_BUNDLE_ROOT}/surfaces/actions_caches/metadata.json"
if "${GATE}" scan-bundle "${CACHE_BUNDLE_ROOT}" \
  >"${TEMP_ROOT}/cache-bundle.out" 2>&1; then
  fail "unresolved Actions cache content was reported as scanned"
fi
assert_contains "${TEMP_ROOT}/cache-bundle.out" '"surface":"actions_caches"'

if "${GATE}" retrospective "codeh007/mtmpg/extra" \
  >"${TEMP_ROOT}/invalid-repository.out" 2>&1; then
  fail "retrospective collector accepted an invalid repository identity"
fi
assert_contains "${TEMP_ROOT}/invalid-repository.out" \
  "repository must use an owner/name identity"

if "${GATE}" retrospective "codeh007/mtmpg" "ghcr.io/codeh007/mtmpg-postgres:latest" \
  >"${TEMP_ROOT}/invalid-image.out" 2>&1; then
  fail "retrospective collector accepted a mutable image tag"
fi
assert_contains "${TEMP_ROOT}/invalid-image.out" \
  "candidate image must use the repository owner and a full sha256 digest"

readonly REMOTE_SOURCE_ROOT="${TEMP_ROOT}/remote-source"
initialize_repository "${REMOTE_SOURCE_ROOT}"
readonly COLLECTOR_SOURCE_ROOT="${TEMP_ROOT}/collector-source"
initialize_repository "${COLLECTOR_SOURCE_ROOT}"
install -d \
  "${COLLECTOR_SOURCE_ROOT}/scripts" \
  "${COLLECTOR_SOURCE_ROOT}/.github/workflows"
install -m 0755 "${GATE}" \
  "${COLLECTOR_SOURCE_ROOT}/scripts/public-readiness"
install -m 0600 \
  "${REPOSITORY_ROOT}/gitleaks.toml" \
  "${REPOSITORY_ROOT}/.dockerignore" \
  "${REPOSITORY_ROOT}/Dockerfile" \
  "${COLLECTOR_SOURCE_ROOT}"
install -m 0600 "${NATIVE_WORKFLOW}" \
  "${COLLECTOR_SOURCE_ROOT}/.github/workflows/native-ci.yml"
git -C "${COLLECTOR_SOURCE_ROOT}" add .
git -C "${COLLECTOR_SOURCE_ROOT}" commit --quiet \
  --message "test: prepare retrospective collector source"
readonly COLLECTOR_GATE="${COLLECTOR_SOURCE_ROOT}/scripts/public-readiness"
export FAKE_GH_REPOSITORY="${REMOTE_SOURCE_ROOT}"
export PGGOMTM_GH_BIN="${REPOSITORY_ROOT}/tests/fixtures/public-readiness/fake-gh"
export FAKE_GH_CALL_LOG="${TEMP_ROOT}/fake-gh-calls.log"
"${COLLECTOR_GATE}" retrospective "codeh007/mtmpg" \
  >"${TEMP_ROOT}/retrospective.out"
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"surface":"surface-bundle","status":"complete","unresolved":0'
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"workflow_logs":{"status":"materialized","count":1}'
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"actions_artifacts":{"status":"materialized","count":1}'
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"github_issues":{"status":"materialized","count":1}'
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"github_pull_requests":{"status":"materialized","count":1}'
assert_contains "${TEMP_ROOT}/retrospective.out" \
  '"releases_packages":{"status":"materialized","count":2}'
assert_contains "${FAKE_GH_CALL_LOG}" \
  'api repos/codeh007/mtmpg/issues/2/comments?per_page=100'
assert_contains "${FAKE_GH_CALL_LOG}" \
  'api repos/codeh007/mtmpg/pulls/2/files?per_page=100'
assert_contains "${FAKE_GH_CALL_LOG}" \
  'api repos/codeh007/mtmpg/releases/assets/302 -H Accept: application/octet-stream'

printf 'public-readiness gate fixture policy passed\n'
