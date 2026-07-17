# pggomtm

`pggomtm` 是 PostgreSQL 18 OAuth validator 的 Rust 原型模块。本页说明当前能力边界、已验证环境、构建门禁，以及只用于候选验证的安装与回退步骤。

## 当前状态与加载边界

当前代码只证明模块加载、原生应用程序二进制接口（ABI）边界和离线 JSON Web Token（JWT）验证组件可行，尚未提供可用于生产的完整 validator：

- PostgreSQL 通过 `oauth_validator_libraries` 加载该模块。
- Cargo 把可部署模块构建为 `cdylib` 共享模块，并导出 `PG_MODULE_MAGIC` 与 `_PG_oauth_validator_module_init`。
- 该模块不是 SQL extension，不需要 control 文件、versioned SQL 或 `CREATE EXTENSION`。
- 生产交付不得使用 `cargo pgrx install` 或 `cargo pgrx package`。
- 当前无 gate 的最终制品会保持 `authorized=false` 且不返回 `authn_id`，因此拒绝所有 token。
- `abi-gate`、`abi-runtime-gate` 与 `pgx-oauth-gate` 只用于测试。内置的确定性 key、公开 JSON Web Key Set（JWKS）和 token fixture 不得用于生产。

当前状态不是 `production-ready`。仓库没有已发布的 `stable` 发布版、生产支持版本或可供部署固定的 GitHub Container Registry（GHCR）开放容器计划（OCI）摘要。

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

每个 Cargo feature 组合都会生成规范的 `pggomtm-build-identity/v1` JSON及其 SHA-256，并把两者嵌入对应module。Identity固定Rust、pgrx、JOSE、PostgreSQL source/header/runtime base、target、architecture与libc，可用于比较build变体；它不包含source commit、最终`.so`或OCI digest，因此不是发布用`release-manifest.json`。

## 从仓库根目录构建和测试

`Dockerfile` 是当前 clean build 与原型门禁的权威入口。以下命令构建 Linux amd64 本地候选镜像，并重新运行 Rust、C、真实 PostgreSQL runtime 与最终制品隔离检查：

```bash
DOCKER_BUILDKIT=1 docker build \
  --platform linux/amd64 \
  --no-cache \
  --pull \
  --progress=plain \
  --tag pggomtm-candidate:local \
  .
```

以下命令额外构建只供测试的 `pgx-oauth-gate` target。不要部署该镜像或其中的 gate module：

```bash
DOCKER_BUILDKIT=1 docker build \
  --platform linux/amd64 \
  --no-cache \
  --pull \
  --progress=plain \
  --target pgx-oauth-gate \
  --tag pggomtm-pgx-oauth-gate:local \
  .
```

构建阶段先在 `/src/target/release/libpggomtm.so` 生成无 gate module。最终镜像把它重命名并安装到 `/usr/lib/postgresql/18/lib/pggomtm.so`。该路径来自当前 `postgres:18.4-bookworm` 镜像；非容器环境必须以目标系统的 `pg_config --pkglibdir` 为准。

## 只用于候选验证的安装

当前仓库没有生产支持的稳定制品。以下步骤只适用于与支持矩阵完全相同的隔离候选环境，且安装后仍会拒绝所有 token：

1. 确认本页构建命令已经生成 `pggomtm-candidate:local` final image。
2. 停止整个候选 PostgreSQL 实例，确保没有 backend 已加载旧 `.so`。
3. 从 final image 创建临时 container，并从当前 final 路径复制 `pggomtm.so`。
4. 删除临时 container，验证目标 `pg_config` 报告 PostgreSQL 18.4，再把 module 安装到真实 `pkglibdir`。
5. 在 `postgresql.conf` 中配置 validator library。
6. 启动或重建全部 backend，再运行候选加载与拒绝路径验证。

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

candidate_container="$(docker create pggomtm-candidate:local)"
docker cp \
  "${candidate_container}:/usr/lib/postgresql/18/lib/pggomtm.so" \
  "$candidate_dir/pggomtm.so"
docker rm "$candidate_container" >/dev/null
candidate_container=
test "$(pg_config --version)" = "PostgreSQL 18.4"
pkglibdir="$(pg_config --pkglibdir)"
install -m 0755 "$candidate_dir/pggomtm.so" "$pkglibdir/pggomtm.so"
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
- 阅读[发布与兼容契约](docs/release-and-compatibility.md)了解未来 SemVer、manifest、stable 门禁和 digest 回退规则。
- 按[安全政策](SECURITY.md)私密报告可能泄露 secret 或影响认证边界的问题。
- 自动化开发代理必须遵守[仓库级 AGENTS 规则](AGENTS.md)。
