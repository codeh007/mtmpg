# pggomtm

`pggomtm` 是公开维护的 PostgreSQL 18 OAuth validator Rust/pgrx 模块。本仓库是源码、原生测试、Docker build graph 与后续发布物的唯一权威；公开的 `main` 表示开发主线，不表示 stable 或 production-ready。本页说明当前能力边界、已验证环境、构建门禁与候选诊断方式。

## 当前状态与加载边界

当前代码已经把模块加载、原生应用程序二进制接口（ABI）边界、离线 JSON Web Token（JWT）验证、closed role/identity 与 artifact readiness 接入正式构建。发布 workflow、外部 release manifest、公开 candidate、跨仓库验收与 stable promotion 尚未完成：

- PostgreSQL 通过 `oauth_validator_libraries` 加载该模块。
- Cargo 把可部署模块构建为 `cdylib` 共享模块，并导出 `PG_MODULE_MAGIC` 与 `_PG_oauth_validator_module_init`。
- 该模块不是 SQL extension，不需要 control 文件、versioned SQL 或 `CREATE EXTENSION`。
- 生产交付不得使用 `cargo pgrx install` 或 `cargo pgrx package`。
- 当前无 gate 的最终制品要求每个新 OAuth backend 在 startup 从固定只读路径建立独立 config/public JWKS snapshot；材料缺失、损坏、可写或不满足同目录同文件系统的原子发布布局时 startup fail closed。
- 正式 validate callback 消费该 snapshot，严格验证ES256签名、唯一issuer/audience、database scope、30至300秒TTL与deny-unknown claims；合法候选token返回PostgreSQL allocator分配的版本化`authn_id`，tampered或不合规token保持未授权。
- Production build对normal dependency tree、production源码、ELF `DT_NEEDED`、未解析符号和敏感字符串执行离线能力门禁，拒绝HTTP/DNS、libcurl/libpq、SQL/SPI、私钥加载、service credential、在线introspection和issuer fallback入口。
- 最终 image 相对 pinned official PostgreSQL base 只允许增加正式 `.so`、MIT LICENSE、内部 build manifest 及其目录；ELF、arch/libc、module位置、权限、官方entrypoint和内部manifest均已取得[远端artifact readiness证据](docs/evidence/issue-116/artifact-readiness-gate.md)。
- 认证失败遵循[版本化reason-code与可见性契约](docs/authentication-failures.md)：服务端只记录稳定脱敏类别；普通token拒绝对客户端保持通用，startup失败最多暴露稳定code。
- `abi-gate`、`abi-runtime-gate` 与 `pgx-oauth-gate` 只用于测试。内置的确定性 key、公开 JSON Web Key Set（JWKS）和 token fixture 不得用于生产。

仓库已经 public，但当前状态不是 `production-ready`。默认 `main` 仍是一次性 bootstrap 前的初始基线；完整开发基线暂时位于 `issue-116-extract-pggomtm`，必须先完成追溯审计、无缓存 cold build 与 whole-branch review，再按 OpenSpec 非 force fast-forward 到 `main`。仓库没有 stable 发布版、生产支持版本或可供部署固定的 GitHub Container Registry（GHCR）开放容器计划（OCI）摘要。

## 已验证支持矩阵

下表只记录[迁移后原型门禁基线](docs/evidence/issue-116/migration-test-baseline.md)已经验证的组合：

| 维度 | 已验证值 | 当前限制 |
| --- | --- | --- |
| 操作系统与架构 | Linux amd64 | 未验证其他操作系统或架构 |
| 运行环境 | Debian bookworm、glibc | 未验证其他发行版或 libc |
| PostgreSQL | 18.4 | Runtime 只接受 PostgreSQL 18 major；其他 PG18 minor 尚未经过独立构建与真实验证，不得部署 |
| Rust | 1.97.1 | 由 `rust-toolchain.toml` 与 Docker 构建固定 |
| Rust 目标平台 | `x86_64-unknown-linux-gnu` | 未验证其他目标平台 |
| pgrx | 0.19.1 | `Cargo.toml` 与 `Cargo.lock` 使用精确版本 |

`pg18` 是当前 Cargo feature 名称，不代表所有 PostgreSQL 18 minor 都已获部署支持。Runtime major gate允许PG18 stable line，但只有 PostgreSQL 18.4 通过了当前源码、头文件与真实运行门禁。

PG18.4的loader、allocator、callback及真实libpq `OAUTHBEARER`正负向smoke记录在[Issue #116 PG18.4验证证据](docs/evidence/issue-116/pg18.4-runtime-oauth-smoke.md)。该证据不得外推为PG18.5部署批准。

每个 Cargo feature 组合都会生成规范的 `pggomtm-build-identity/v1` JSON及其 SHA-256，并把两者嵌入对应module。正式 image 还包含 `pggomtm-build-manifest/v1`，把唯一 production identity 绑定到实际 `.so` 与 MIT LICENSE checksum。内部 manifest 不包含 source commit 或 image 自身 OCI digest；后续 trusted workflow 必须在 digest 产生后生成独立 `release-manifest.json`。

正式validator只允许读取[固定路径下的版本化runtime配置](docs/runtime-configuration.md)。当前已实现每个新backend的只读config/public JWKS加载、严格校验、同文件系统原子替换布局、独立snapshot持有与shutdown释放；后续backend读取新snapshot，既有backend不reload。正式validate callback已经接入snapshot并通过真实PG18.4 valid/tampered token smoke，以及OAuth client/API-key actor、三种profile、authority、ID、time、algorithm、audience/scope和signature矩阵。Closed profile-role与forbidden-role门禁、allocator/identity往返、稳定reason与日志脱敏及无gate artifact能力边界均已取得远端证据；完整索引见[`docs/evidence/issue-116/`](docs/evidence/issue-116/)。后续工作不重新实现 validator 主路径，而是完成公开治理、cold authority、发布供应链与 gomtmui consumer evidence。

## 通过 GitHub Actions 构建和测试

根 `Dockerfile` 是 Rust、C、production 静态/ELF 离线能力、真实 PostgreSQL runtime 与最终制品隔离检查的唯一 build graph。`.github/workflows/native-ci.yml` 从精确远端 commit 执行该 graph；只有成功的 `Native CI` run 可以完成 OpenSpec task、gomtmui consumer gate 或发布门禁。本地命令只用于诊断。

一次性 bootstrap 期间，workflow 接受 `issue-116-extract-pggomtm` push。进入 `main` 后，普通开发只接受面向 `main` 的 Pull Request（PR）和 `main` push；Agent 创建或更新短期 PR、等待 required `Native CI`、处理失败，并在 GitHub 治理启用后设置 auto-merge。公开 fork PR 始终使用无 secret、read-only token，workflow 禁止 `pull_request_target`。

使用当前分支定位并等待对应 run：

```bash
branch="$(git branch --show-current)"
gh run list \
  --repo codeh007/mtmpg \
  --workflow native-ci.yml \
  --branch "$branch" \
  --limit 5
run_id=1234567890123
gh run watch "$run_id" --repo codeh007/mtmpg --exit-status
gh run view "$run_id" --repo codeh007/mtmpg --log-failed
```

常规 PR/`main` run 使用内容寻址的 GitHub Actions cache 和同 ref 并发取消，不登录 GHCR、不读取发布 secret、不上传正式制品。后续 cold authority mode 负责无缓存复验；trusted candidate/promotion workflow 只从受保护 `main` 的精确 commit 取得最小写权限。

### 可选的本地诊断

需要定位Docker或runner差异时，可以构建本地诊断image。它不能替代远端run：

```bash
DOCKER_BUILDKIT=1 docker build \
  --platform linux/amd64 \
  --pull \
  --progress=plain \
  --tag pggomtm-diagnostic:local \
  .
```

需要单独定位测试gate时，追加`--target pgx-oauth-gate`并使用明显的diagnostic tag。不要部署该image或其中的gate module。

构建阶段先在 `/src/target/release/libpggomtm.so` 生成无 gate module。最终镜像把它重命名并安装到 `/usr/lib/postgresql/18/lib/pggomtm.so`。该路径来自当前 `postgres:18.4-bookworm` 镜像；非容器环境必须以目标系统的 `pg_config --pkglibdir` 为准。

## 公开 GHCR 与部署身份

后续 trusted candidate workflow 将发布公开读取的 `ghcr.io/codeh007/mtmpg-postgres`。该 package 当前尚不存在；可匿名读取不表示 stable，也不授予上传、删除、改标或 Release 权限。

Package 发布后，消费者不需要 private pull credential，但必须使用完整 OCI digest：

```text
ghcr.io/codeh007/mtmpg-postgres@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

该 64 位值只是格式示例。`latest`、SemVer tag 与 source tag 只用于发现，不能作为部署身份。Candidate 先从受保护 `main` 的精确 commit 构建一次；gomtmui 验收同一 digest 后，promotion 只能给该 digest 增加 stable alias 和 immutable Release，不得重建。

## 只用于本地候选诊断的安装

当前仓库没有生产支持的稳定制品。以下步骤只用于与支持矩阵完全相同的隔离本地诊断，不是未来 GHCR consumer 的安装路径；它要求先按[运行时配置契约](docs/runtime-configuration.md)提供只读材料，并只验证当前正式callback的候选allow/deny行为：

1. 确认对应远端commit的`Native CI`已经成功；若要执行本地安装演练，再从同一commit生成`pggomtm-diagnostic:local` final image。
2. 停止整个候选 PostgreSQL 实例，确保没有 backend 已加载旧 `.so`。
3. 从 final image 创建临时 container，并从当前 final 路径复制 `pggomtm.so`。
4. 删除临时 container，验证目标 `pg_config` 报告 PostgreSQL 18.4，再把 module 安装到真实 `pkglibdir`。
5. 在固定路径提供权限正确的候选config/public JWKS，并在 `postgresql.conf` 中配置 validator library。
6. 启动或重建全部 backend，确认startup建立snapshot，再运行候选allow/deny路径验证。

在实例停止后执行以下提取与安装命令。清理函数会在命令失败时删除临时 container 和目录：

```bash
set -euo pipefail
candidate_dir="$(mktemp -d)"
candidate_container=
cleanup_candidate() {
  if test -n "$candidate_container"; then
    docker rm --force "$candidate_container" >/dev/null 2>&1 || true
  fi
  rm -rf "$candidate_dir"
}
trap cleanup_candidate EXIT

candidate_container="$(docker create pggomtm-diagnostic:local)"
docker cp \
  "${candidate_container}:/usr/lib/postgresql/18/lib/pggomtm.so" \
  "$candidate_dir/pggomtm.so"
docker rm "$candidate_container" >/dev/null
candidate_container=
test "$(pg_config --version)" = "PostgreSQL 18.4"
pkglibdir="$(pg_config --pkglibdir)"
install -m 0644 "$candidate_dir/pggomtm.so" "$pkglibdir/pggomtm.so"
cleanup_candidate
trap - EXIT
```

在 `postgresql.conf` 中使用以下配置：

```ini
oauth_validator_libraries = 'pggomtm'
```

这里不运行 `CREATE EXTENSION`。不要在仍有 backend 运行时覆盖已加载的 `.so`，也不要把测试 gate feature 或内置测试 JWKS 带入候选配置。

## 候选升级与回退

候选切换必须以已验证构建为单位，并通过重建或重启 backend 生效：

1. 记录当前源 commit、PostgreSQL 18.4 环境和候选 module 身份。
2. 从 clean checkout 构建并验证替代候选。
3. 停止所有受影响的 PostgreSQL backend；若平台先排空连接，必须等待旧 backend 全部退出。
4. 在 backend 全部停止后，切换到已经验证且身份固定的候选镜像或 module。
5. 启动或重建全部 backend，再运行 loader、拒绝路径和 ABI 门禁。

回退时先停止或排空全部 backend，再切回上一份已验证候选，最后启动或重建并验证。不要热覆盖 `.so`，不要现场编译，也不要恢复第二份源码或认证 fallback。

## 参与维护与报告问题

仓库维护入口说明了不同类型问题的处理方式：

- 阅读[贡献指南](CONTRIBUTING.md)后再提交代码或文档变更。
- 阅读[维护规则](MAINTAINERS.md)了解源码权威与人工审查门禁。
- 阅读[GitHub 治理状态](docs/github-governance.md)了解 development `main`、Agent auto-merge 与远端服务端门禁。
- 阅读[发布与兼容契约](docs/release-and-compatibility.md)了解公开 GHCR、SemVer、manifest、stable 门禁和 digest 回退规则。
- 使用[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)跟踪公开主线、candidate、跨仓库验收与首个 stable release。
- 按[安全政策](SECURITY.md)私密报告可能泄露 secret 或影响认证边界的问题。
- 自动化开发代理必须遵守[仓库级 AGENTS 规则](AGENTS.md)。
