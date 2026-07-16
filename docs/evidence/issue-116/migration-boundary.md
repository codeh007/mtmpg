# Issue #116：pggomtm 迁移边界静态证据

本证据于 2026-07-16（UTC）采集，任务起点为
`98611e84f24f5e429d3d4109a3070aba601174ef`，原型迁移提交为
`d4231226be6dc59599bce331d487b9b20039dcb6`。检查只读取 Git index、
所有本地可达 ref 的 object path/内容以及 Dockerfile；没有读取环境变量、
连接串或外部 secret，没有构建 Rust、Docker image 或导出 layer。

## 范围与禁止类别

以下内容不得出现在当前 tracked tree、原型迁移 payload 或 mtmpg 可达 Git
历史中：

- Cargo `target/`、编译/发布输出、本地 OCI/image/archive 导出；
- 根级或嵌套 `.env`、private-key PEM、key container、credential、token、
  secret；
- PostgreSQL `data`/`pgdata`、其他数据库文件和 session 运行数据；
- `gomtmui/` 路径或 `native/pggomtm/` 嵌套源码副本。

迁移提交的 13 个路径仅为 `.dockerignore`、Cargo manifest/lock、toolchain、
Dockerfile、两个 `src/` 文件和七个 `tests/` 文件。它没有携带目录嵌套、
缓存、运行数据或 image。当前 index 在加入本证据后共有 24 个 tracked 路径。

## Git 路径边界

以下 Bash 命令分别检查当前 HEAD tree、index、原型迁移提交以及每一个可达
commit 的完整 tree。路径始终由 NUL 分隔；逐 tree 枚举不会因相同 blob object
ID 去重而漏掉旧路径。任何枚举器错误或禁止路径命中都会失败。

```bash
set -euo pipefail

forbidden_path_pattern='(^|/)(target|data|pgdata|postgres-data|postgres_data|postgresql-data|session|sessions|images|image-exports|oci-layout|build|dist|out|artifacts|release|releases|coverage|tmp|temp|logs|\.superpowers|\.worktree|\.worktrees)(/|$)|(^|/)\.env($|\.)|(^|/)\.?(secret|secrets|credential|credentials)(/|$)|(^|/)\.(pgpass|netrc|git-credentials)$|\.(pem|key|p12|pfx|jks|keystore|kdbx|sqlite|sqlite3|db|db-wal|db-shm|session|session\.json|session\.lock|oci|tar|tar\.gz|tgz|tar\.zst|zip|7z|gz|zst|o|a|so|dylib|dll|exe|pdb|deb|rpm|tmp|temp|log|swp|swo)$|(^|/)gomtmui(/|$)|(^|/)native/pggomtm(/|$)'

scan_path_stream() {
  scope=$1
  found=0
  while IFS= read -r -d '' path; do
    if [[ $path =~ $forbidden_path_pattern ]]; then
      printf '%s: MATCH %q\n' "$scope" "$path" >&2
      found=1
    fi
  done
  test "$found" -eq 0
}

run_path_scan() {
  scope=$1
  shift
  set +e
  "$@" | scan_path_stream "$scope"
  status=("${PIPESTATUS[@]}")
  set -e
  test "${status[0]}" -eq 0 || {
    printf '%s: ENUMERATOR_ERROR (exit %s)\n' "$scope" "${status[0]}" >&2
    return "${status[0]}"
  }
  test "${status[1]}" -eq 0 || return 1
  printf '%s: PASS\n' "$scope"
}

run_path_scan head git ls-tree -rz --name-only HEAD
run_path_scan index git ls-files -z --cached
run_path_scan migration git ls-tree -rz --name-only \
  d4231226be6dc59599bce331d487b9b20039dcb6

commits_text=$(git rev-list --all)
mapfile -t commits <<<"$commits_text"
for commit in "${commits[@]}"; do
  run_path_scan "commit:$commit" git ls-tree -rz --name-only "$commit"
done
printf 'commit_count=%s\n' "${#commits[@]}"
```

复验结果：HEAD、index、迁移提交及逐个可达 commit tree 全部 `PASS`。
模式显式覆盖独立 `.gz`、`.zst` 以及 `.gitignore` 中的 image/archive 类别；
没有使用 `git rev-list --objects --all` 的单一路径提示作为历史路径证明。

## 敏感内容扫描

扫描只输出命中的 object/path，不打印匹配内容。current worktree、index 和
每个可达 commit tree 都用 `git grep -a -z -l`：`-a` 强制把 binary blob
作为文本扫描，`-z` 保持命中路径 NUL-safe，没有使用会跳过 binary 的 `-I`。
所有 commit message 和 annotated tag message 另行逐对象扫描。PEM 模式覆盖
encrypted/private-key header；赋值模式另外覆盖裸大写 token/secret 环境变量。

```bash
set -euo pipefail

declare -A patterns insensitive
key_word=KEY
patterns[pem]="-----BEGIN (([A-Z0-9]+[[:space:]]+)*PRIVATE[[:space:]]+${key_word}|PGP PRIVATE ${key_word} BLOCK)-----"
patterns[jwt]='eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{16,}'
patterns[db_uri]="(postgres|postgresql|mysql|mariadb|mongodb(\\+srv)?|redis)://[^[:space:]\"']+:[^@[:space:]\"']+@"
patterns[assignment]="(api[_-]?key|api[_-]?token|oauth[_-]?token|access[_-]?token|refresh[_-]?token|client[_-]?secret|secret[_-]?key|authorization[_-]?code|database[_-]?(url|uri|dsn)|postgres(_ql)?[_-]?(url|uri|dsn)|password|passwd|credential)[[:space:]]*[:=][[:space:]]*[\"']?[A-Za-z0-9_./+@%:-]{8,}"
patterns[bare_assignment]="(^|[^A-Za-z0-9_])(TOKEN|SECRET|API_TOKEN|OAUTH_TOKEN)[[:space:]]*[:=][[:space:]]*[\"']?[A-Za-z0-9_./+@%:-]{8,}"
patterns[bearer]='authorization[[:space:]]*:[[:space:]]*bearer[[:space:]]+[A-Za-z0-9._~+/-]{12,}'
patterns[token_format]='AKIA[0-9A-Z]{16}|github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|sk-(proj-)?[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{30,}|xox[baprs]-[A-Za-z0-9-]{10,}'
for label in pem jwt db_uri assignment bearer token_format; do insensitive[$label]=1; done
insensitive[bare_assignment]=0

format_nul_hits() {
  scope=$1
  while IFS= read -r -d '' hit; do
    printf '%s: MATCH %q\n' "$scope" "$hit" >&2
  done
}

run_blob_scan() {
  scope=$1 label=$2 pattern=$3 fold_case=$4 tree=$5
  cmd=(git grep)
  test "$tree" = index && cmd+=(--cached)
  cmd+=(-a -z -l -E)
  test "$fold_case" -eq 1 && cmd+=(-i)
  cmd+=(-e "$pattern")
  case "$tree" in
    current|index) cmd+=(-- .) ;;
    *) cmd+=("$tree" --) ;;
  esac

  set +e
  "${cmd[@]}" | format_nul_hits "$scope/$label"
  status=("${PIPESTATUS[@]}")
  set -e
  test "${status[1]}" -eq 0 || return "${status[1]}"
  case "${status[0]}" in
    0) return 1 ;;
    1) printf '%s/%s: PASS (git grep exit 1)\n' "$scope" "$label" ;;
    *) printf '%s/%s: TOOL_ERROR (git grep exit %s)\n' \
         "$scope" "$label" "${status[0]}" >&2; return "${status[0]}" ;;
  esac
}

run_message_scan() {
  scope=$1 label=$2 pattern=$3 fold_case=$4
  shift 4
  grep_cmd=(grep -a -E -q -e "$pattern")
  test "$fold_case" -eq 1 && grep_cmd=(grep -a -E -i -q -e "$pattern")
  set +e
  "$@" | LC_ALL=C "${grep_cmd[@]}"
  status=("${PIPESTATUS[@]}")
  set -e
  test "${status[0]}" -eq 0 || {
    printf '%s/%s: PRODUCER_ERROR (exit %s)\n' \
      "$scope" "$label" "${status[0]}" >&2
    return "${status[0]}"
  }
  case "${status[1]}" in
    0) printf '%s/%s: MATCH\n' "$scope" "$label" >&2; return 1 ;;
    1) printf '%s/%s: PASS (grep exit 1)\n' "$scope" "$label" ;;
    *) printf '%s/%s: TOOL_ERROR (grep exit %s)\n' \
         "$scope" "$label" "${status[1]}" >&2; return "${status[1]}" ;;
  esac
}

commits_text=$(git rev-list --all)
mapfile -t commits <<<"$commits_text"
tags_text=$(git for-each-ref --format='%(refname)' refs/tags)
mapfile -t tags <<<"$tags_text"

for label in pem jwt db_uri assignment bare_assignment bearer token_format; do
  pattern=${patterns[$label]}
  fold_case=${insensitive[$label]}
  run_blob_scan current "$label" "$pattern" "$fold_case" current
  run_blob_scan index "$label" "$pattern" "$fold_case" index
  for commit in "${commits[@]}"; do
    run_blob_scan "commit:$commit" "$label" "$pattern" "$fold_case" "$commit"
    run_message_scan "commit-message:$commit" "$label" "$pattern" "$fold_case" \
      git show -s --format=%B "$commit"
  done
  for tag in "${tags[@]}"; do
    test -n "$tag" || continue
    test "$(git cat-file -t "$tag")" = tag || continue
    run_message_scan "tag-message:$tag" "$label" "$pattern" "$fold_case" \
      git for-each-ref --format='%(contents)' "$tag"
  done
done
```

复验结果：current、index、逐 commit blob、逐 commit message 和 annotated tag
message 的七类模式全部 `PASS`；没有 encrypted/private-key PEM、落盘 compact
JWT、带口令数据库 URI、凭据赋值、Bearer 或常见真实 token 格式命中。扫描
没有排除 `src/`/`tests/`，binary blob 也没有被跳过；任何 producer、formatter、
`git grep` 或 `grep` 错误都会保持非零并失败。

## 已审查的确定性测试向量

原型有确定性但不具备任何真实权限的测试材料。复核从完整 tracked index 搜索，
不先限定文件，再把实际 `file:line` 集合与硬编码允许集合逐项比较；任何新增
production 入口都会导致集合不等而失败。模式在文档源码中拆分，避免证据自身
成为 fixture 命中。

```bash
set -euo pipefail

collect_fixture_lines() {
  mode=$1 pattern=$2
  set +e
  if test "$mode" = fixed; then
    output=$(git grep --cached -n -F -e "$pattern" -- 2>&1)
  else
    output=$(git grep --cached -n -E -e "$pattern" -- 2>&1)
  fi
  rc=$?
  set -e
  case "$rc" in
    0) printf '%s\n' "$output" \
         | sed -E 's/^([^:]+:[0-9]+):.*$/\1/' \
         | LC_ALL=C sort -u ;;
    1) return 0 ;;
    *) printf 'fixture search TOOL_ERROR (exit %s)\n%s\n' \
         "$rc" "$output" >&2; return "$rc" ;;
  esac
}

compare_fixture_set() {
  label=$1 actual=$2 expected=$3
  if test "$actual" != "$expected"; then
    printf '%s fixture set mismatch\nexpected:\n%s\nactual:\n%s\n' \
      "$label" "$expected" "$actual" >&2
    return 1
  fi
  printf '%s fixture set: PASS\n' "$label"
}

scalar_pattern='SigningKey::from_slice(&[7_u8; 3''2])'
placeholder_pattern='header.pay''load.signature'
gate_constant='PGX_OAUTH_GATE_JW''KS'
gate_verify='verify_pgx_gate_''token'
gate_kid='candidate-es256-pgx-''gate'
gate_pattern="$gate_constant|$gate_verify|$gate_kid"
key_word=KEY
pem_pattern="-----BEGIN (([A-Z0-9]+[[:space:]]+)*PRIVATE[[:space:]]+${key_word}|PGP PRIVATE ${key_word} BLOCK)-----"
jwt_pattern='eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{16,}'

expected_scalar=$(printf '%s\n' \
  'tests/jwt_identity.rs:21' \
  'tests/pgx_oauth_gate.rs:29')
expected_placeholder=$(printf '%s\n' \
  'tests/abi_layout.rs:42' \
  'tests/oauth_runtime_probe.c:52')
expected_gate=$(printf '%s\n' \
  'Dockerfile:166' \
  'src/lib.rs:165' \
  'src/lib.rs:17' \
  'src/lib.rs:20' \
  'src/lib.rs:29' \
  'tests/pgx_oauth_gate.rs:31' \
  'tests/pgx_oauth_gate.rs:44' \
  'tests/pgx_oauth_gate.rs:53' \
  'tests/pgx_oauth_gate.rs:66' \
  'tests/pgx_oauth_gate.rs:8')

compare_fixture_set scalar \
  "$(collect_fixture_lines fixed "$scalar_pattern")" "$expected_scalar"
compare_fixture_set placeholder \
  "$(collect_fixture_lines fixed "$placeholder_pattern")" "$expected_placeholder"
compare_fixture_set gate \
  "$(collect_fixture_lines regex "$gate_pattern")" "$expected_gate"
compare_fixture_set pem "$(collect_fixture_lines regex "$pem_pattern")" ''
compare_fixture_set compact-jwt "$(collect_fixture_lines regex "$jwt_pattern")" ''
```

复验结果：五个集合全部 `PASS`。固定测试标量只在两个 integration test；三段
占位值只在 Rust ABI test 和 C runtime probe；gate 标识只在 feature-gated
源码/测试入口及 Dockerfile 的排除扫描。仓库没有 test PEM 或落盘 compact
JWT。public verifying JWK 不是 secret；确定性标量不会授权任何真实 issuer、
数据库或部署环境。Docker build context 携带 `tests/**` 只为执行现有门禁，
最终 stage 不复制 tests。

## `.gitignore` 复验

以下 `--no-index` 样例只是不存在的临时路径，位于 `.task-3-tmp/` 命名空间
或已忽略的 `.superpowers/`；命令不会创建文件。输出逐项指向实际命中的规则。

```bash
git check-ignore -v --no-index \
  .task-3-tmp/target/probe.o \
  .task-3-tmp/nested/.env.production \
  .task-3-tmp/nested/server.pem \
  .task-3-tmp/a/.credentials/probe \
  .task-3-tmp/b/credential/probe \
  .task-3-tmp/c/credentials/probe \
  .task-3-tmp/d/secret/probe \
  .task-3-tmp/e/secrets/probe \
  .task-3-tmp/pgdata/PG_VERSION \
  .task-3-tmp/nested/sessions/run.session \
  .task-3-tmp/image-export.tar \
  dist/libpggomtm.so \
  .superpowers/sdd/task-3-fixtures/probe

# -v 显示精确反例；quiet 必须返回 1，证明模板未被忽略。
git check-ignore -v --no-index .task-3-tmp/nested/.env.example
set +e
git check-ignore -q --no-index .task-3-tmp/nested/.env.example
example_rc=$?
set -e
test "$example_rc" -eq 1
test ! -e .task-3-tmp
```

结果：十三个禁止样例全部由对应规则或 `.superpowers/` 边界命中，包括任意
层级 `.credentials/`、`credential(s)/` 和 `secret(s)/`；
`.env.example` 显示 `!.env.example`，quiet 返回 1。没有创建样例，因此清理后
不存在临时样例残留。

## Docker build context 最小化

根 `.dockerignore` 第一条有效规则为 `**`，其后唯一 allowlist 为
`Dockerfile`、三个 Cargo/toolchain 根文件以及 `src/**`、`tests/**`（目录
自身也显式解除，保证后代反例生效）。以下静态检查同时固定规则和 Dockerfile
的三条本地 `COPY`：

```bash
actual_rules=$(sed '/^[[:space:]]*#/d; /^[[:space:]]*$/d' .dockerignore)
expected_rules=$(printf '%s\n' \
  '**' \
  '!Dockerfile' \
  '!Cargo.toml' \
  '!Cargo.lock' \
  '!rust-toolchain.toml' \
  '!src/' \
  '!src/**' \
  '!tests/' \
  '!tests/**')
test "$actual_rules" = "$expected_rules"

actual_copy=$(awk '$1 == "COPY" && $0 !~ /--from=/ { print }' Dockerfile)
expected_copy=$(printf '%s\n' \
  'COPY Cargo.toml Cargo.lock rust-toolchain.toml ./' \
  'COPY src ./src' \
  'COPY tests ./tests')
test "$actual_copy" = "$expected_copy"
```

结果：两项比较均成功。所有 `COPY --from=...` 都从前序 stage 取构建结果，
不扩大客户端 context。因此 `.git`、`.gitignore`、`.dockerignore`、OpenSpec、
docs/evidence、`.env`、secret、data/session、target、artifact、gomtmui 其他源码
均被 deny-all 拦截，不发送给 BuildKit；allowlist 恰好覆盖 Dockerfile 的本地
`COPY` 输入和 Dockerfile 自身。
