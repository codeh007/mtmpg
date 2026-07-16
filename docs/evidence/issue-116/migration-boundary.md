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

以下 Bash 命令分别检查当前 index、原型迁移提交和所有可达历史 object path。
`grep` 返回 1 才表示无匹配；返回 0 被视为泄漏并令检查失败，返回大于 1
被视为工具错误并原样失败，不能冒充“无匹配”。

```bash
set -euo pipefail

forbidden_path_pattern='(^|/)(target|data|pgdata|postgres-data|postgres_data|postgresql-data|session|sessions)(/|$)|(^|/)\.env($|\.)|(^|/)(secrets?|credentials?|images|image-exports|oci-layout)(/|$)|(^|/)\.(secret|secrets|credentials)($|[./])|(^|/)\.(pgpass|netrc|git-credentials)$|\.(pem|key|p12|pfx|jks|keystore|kdbx|sqlite|sqlite3|db|session|oci|tar|tar\.gz|tgz|tar\.zst|zip|7z)$|(^|/)gomtmui(/|$)|(^|/)native/pggomtm(/|$)'

tracked_paths=$(git ls-files)
migration_paths=$(git diff-tree --no-commit-id --name-only -r \
  d4231226be6dc59599bce331d487b9b20039dcb6)
history_paths=$(git rev-list --objects --all \
  | sed -n -E 's/^[0-9a-f]{40} //p')

scan_paths() {
  label=$1
  paths=$2
  set +e
  hits=$(printf '%s\n' "$paths" \
    | LC_ALL=C grep -E -i -e "$forbidden_path_pattern" 2>&1)
  rc=$?
  set -e
  case "$rc" in
    0) printf '%s: MATCH\n%s\n' "$label" "$hits"; return 1 ;;
    1) printf '%s: NO_MATCH (grep exit 1)\n' "$label" ;;
    *) printf '%s: TOOL_ERROR (grep exit %s)\n%s\n' \
         "$label" "$rc" "$hits" >&2; return "$rc" ;;
  esac
}

scan_paths tracked "$tracked_paths"
scan_paths migration "$migration_paths"
scan_paths history "$history_paths"
```

复验结果：三个 scope 均为 `NO_MATCH (grep exit 1)`；迁移 payload 为 13
条路径，当前 index 为 24 条路径。`git rev-list --objects --all` 覆盖本地所有
可达 ref，而不是只检查当前工作树。

## 敏感内容扫描

扫描只输出命中的 object/path，不打印匹配行。模式覆盖真实 private-key PEM、
标准三段 compact JWT、带口令数据库 URI、Bearer、常见 credential/token/
secret 赋值和常见云端 token 格式。PEM 模式在命令中拆开 `KEY` 单词，避免
证据文档本身伪造一个 PEM header。

```bash
set -euo pipefail

declare -A patterns
key_word=KEY
patterns[pem]="-----BEGIN ((RSA|EC|DSA|OPENSSH) )?PRIVATE ${key_word}-----|-----BEGIN PGP PRIVATE ${key_word} BLOCK-----"
patterns[jwt]='eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{16,}'
patterns[db_uri]="(postgres|postgresql|mysql|mariadb|mongodb(\\+srv)?|redis)://[^[:space:]\"']+:[^@[:space:]\"']+@"
patterns[assignment]="(api[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|secret[_-]?key|authorization[_-]?code|database[_-]?(url|uri|dsn)|postgres(_ql)?[_-]?(url|uri|dsn)|password|passwd|credential)[[:space:]]*[:=][[:space:]]*[\"']?[A-Za-z0-9_./+@%-]{8,}"
patterns[bearer]='authorization[[:space:]]*:[[:space:]]*bearer[[:space:]]+[A-Za-z0-9._~+/-]{12,}'
patterns[token_format]='AKIA[0-9A-Z]{16}|github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|sk-(proj-)?[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{30,}|xox[baprs]-[A-Za-z0-9-]{10,}'

report_scan() {
  scope=$1
  label=$2
  output=$3
  rc=$4
  case "$rc" in
    0) printf '%s/%s: MATCH\n%s\n' "$scope" "$label" "$output"; return 1 ;;
    1) printf '%s/%s: NO_MATCH (git grep exit 1)\n' "$scope" "$label" ;;
    *) printf '%s/%s: TOOL_ERROR (git grep exit %s)\n%s\n' \
         "$scope" "$label" "$rc" "$output" >&2; return "$rc" ;;
  esac
}

commits_text=$(git rev-list --all)
mapfile -t commits <<<"$commits_text"
for label in pem jwt db_uri assignment bearer token_format; do
  pattern=${patterns[$label]}

  set +e
  output=$(git grep --cached -I -l -i -E -e "$pattern" -- . 2>&1)
  rc=$?
  set -e
  report_scan tracked "$label" "$output" "$rc"

  set +e
  output=$(git grep -I -l -i -E -e "$pattern" "${commits[@]}" -- 2>&1)
  rc=$?
  set -e
  report_scan history "$label" "$output" "$rc"
done
```

复验结果：tracked 与 history 的六类扫描全部是
`NO_MATCH (git grep exit 1)`，没有 private-key PEM、落盘 compact JWT、带口令
数据库 URI、凭据赋值或常见真实 token 格式命中。扫描过程没有通过删除日志、
排除 `src/`/`tests/` 或吞掉错误码获得通过。

## 已审查的确定性测试向量

原型有确定性但不具备任何真实权限的测试材料。它们不是上述扫描的全局豁免；
逐一检查如下：

| 文件 | 测试材料 | 唯一用途与静态边界 |
| --- | --- | --- |
| `tests/jwt_identity.rs` | `signing_key()` 从固定 `[7_u8; 32]` 构造 P-256 key | 只位于 Rust integration test；token 在测试进程内生成，用于 ES256/JWKS、claims、role 与 identity 正负测试，library/production 源码不引用该函数。 |
| `tests/pgx_oauth_gate.rs` | 同一固定测试标量在 `signed_gate_token()` 中生成 token | 文件首行 `#![cfg(feature = "pgx-oauth-gate")]`；只由该 gate 的 integration test 使用。 |
| `src/lib.rs` | `PGX_OAUTH_GATE_JWKS` 仅含与上述标量对应的 public verifying JWK | 常量和 `verify_pgx_gate_token()` 都有 `#[cfg(feature = "pgx-oauth-gate")]`；无该 feature 的正式构建不可达。public JWK 不是 secret。 |
| `tests/abi_layout.rs` | 三段 ASCII 占位 token | 只在 `callback_table_initializes_and_fails_closed_before_jwt_gate` 中验证 callback fail-closed；它不是 base64url JWT，也没有签名或权限。 |

精确入口可用以下只读命令复核：

```bash
git grep -n -F 'SigningKey::from_slice(&[7_u8; 32])' -- tests
git grep -n -E 'PGX_OAUTH_GATE_JWKS|verify_pgx_gate_token|#!\[cfg\(feature = "pgx-oauth-gate"\)\]' \
  -- src/lib.rs tests/pgx_oauth_gate.rs
git grep -n -F 'CString::new("header.payload.signature")' -- tests/abi_layout.rs
git grep -n -E 'pgx-oauth-gate|cargo build --locked --release' \
  -- Cargo.toml Dockerfile
```

结果仅落在表中三个测试文件、`src/lib.rs` 的 feature-gated public verifier 和
声明/调用 gate 的 Cargo/Docker 构建行。仓库没有固定 compact JWT 或 PEM；
固定标量不会授权任何真实 issuer、数据库或部署环境，也没有被误归类为真实
secret。Docker build context 必须携带 `tests/**` 以执行现有门禁，但最终
stage 的本地输入只来自前序 build stage 的正式 `.so`，不会复制 tests。

## `.gitignore` 复验

以下 `--no-index` 样例只是不存在的临时路径，位于 `.task-3-tmp/` 命名空间
或已忽略的 `.superpowers/`；命令不会创建文件。输出逐项指向实际命中的规则。

```bash
git check-ignore -v --no-index \
  .task-3-tmp/target/probe.o \
  .task-3-tmp/nested/.env.production \
  .task-3-tmp/nested/server.pem \
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

结果：八个禁止样例全部由对应规则或 `.superpowers/` 边界命中；
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
