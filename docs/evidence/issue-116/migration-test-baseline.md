# Issue #116：pggomtm 迁移后原型门禁基线

本证据于 2026-07-16（UTC）从固定 source commit
`58d38f58bd6ba944ddef69f2a12a9c99445f81ba` 采集。它复验的是已迁移原型的
既有行为，不是 stable 或 production readiness 证明。当前无 gate 的正式构建
仍是 fail-closed 原型，尚未从外部只读 config/JWKS 提供生产验签能力。

## Clean checkout 与源码身份

测试对象由当前仓库创建为 detached 临时 worktree：

```sh
git worktree add --detach /tmp/mtmpg-task4-clean-58d38f58 \
  58d38f58bd6ba944ddef69f2a12a9c99445f81ba
```

构建上下文只来自该 checkout。构建前后
`git status --porcelain=v1 --untracked-files=all` 均为空，且 checkout 内不存在
`target/`。没有从当前实现 worktree 注入 ignored/untracked 文件。

受审查清单 `docs/evidence/issue-116/pggomtm-source.sha256` 的 SHA-256 为
`8ec180c6b82602fe1b943e75bf29166f2a0bdeefb62a86e57f82c861b1679ba9`，
共 13 条且无 `target/` 路径。复验结果如下：

- `/workspace/gomtmui/native/pggomtm` 在剪枝任意层级 `target/` 后仍恰有
  13 个普通文件，`sha256sum --check --strict` 为 13/13 `OK`；
- 原型迁移 commit `d4231226be6dc59599bce331d487b9b20039dcb6` 的对应
  13 个 blob 为 13/13 `OK`；
- 固定 source commit 中 12/13 个原型文件仍与清单逐字节相同；唯一差异是
  `.dockerignore`，其 SHA-256 从
  `c97ecfda4d205190b973232dcfdb0c29748521c2534dd866bcc782f30b086738`
  变为
  `ef752883f709971cf5faea021d4fc497b85778a6d06b3b4e1f8a5ad656393ab4`。
  该差异来自已审查的任务 1.3 commit
  `2c5fd7890c022f1ec8ec56881a68e700894df17c`，把 Docker context 收紧为
  deny-all 加最小 allowlist；Cargo、toolchain、Dockerfile、Rust 源码和测试
  均未改变。

最初还运行了一个额外的“当前 HEAD 13/13 全部等于迁移前清单”断言，exit 1
并在 `.dockerignore` 处停止。它不是需求定义的 Docker/Rust/C/PostgreSQL
行为门禁；调查确认是上述任务 1.3 的预期差异后，改用“原始目录 13/13、迁移
commit 13/13、当前权威实现 12/13 不变加 1 个已授权边界变更”的正确映射，
exit 0。该失败日志未删除或隐瞒。

## 固定构建身份

| 项目 | 固定值或实测值 |
| --- | --- |
| Source | `58d38f58bd6ba944ddef69f2a12a9c99445f81ba` |
| Rust builder base | `rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc` |
| 实际 Rust toolchain | `rustc 1.97.1 (8bab26f4f 2026-07-14)`；`cargo 1.97.1 (c980f4866 2026-06-30)` |
| Rust target | `x86_64-unknown-linux-gnu` |
| Native 依赖 | `pgrx 0.19.1`；JOSE 实现 `jaws 1.0.4`；均来自 locked Cargo graph |
| PostgreSQL source | `18.4`；tarball SHA-256 `81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094` |
| OAuth server header | `/opt/postgresql-18.4/include/server/libpq/oauth.h`；SHA-256 `be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf` |
| Runtime base | `postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296` |
| Runtime | PostgreSQL `18.4 (Debian 18.4-1.pgdg12+1)`，Linux amd64，Debian bookworm/glibc |

## 命令、exit code 与门禁映射

默认 final target 使用 daemon-backed BuildKit；普通 `docker build` 成功后会把
结果直接加载到本地 Docker daemon：

```sh
DOCKER_BUILDKIT=1 docker build \
  --no-cache --pull --progress=plain \
  --tag mtmpg-task4-final:58d38f5-local .
```

结果为 exit 0。固定 `FROM` image 层显示 `CACHED` 是 Docker 对已有 base
content 的正常复用；所有 `RUN` 门禁均重新执行。日志可见 Rust toolchain 再安装、
APT 与 crates 再下载、PostgreSQL 18.4 源码重新编译 495.6 秒，没有命中旧门禁
结果。

| Dockerfile 门禁 | 结果 |
| --- | --- |
| 官方 C OAuth layout probe 与 header checksum | exit 0；C 以 `-Wall -Wextra -Werror` 编译并运行，header SHA-256 匹配上表固定值 |
| Locked ABI/layout/fail-closed/panic 测试 | 4/4 passed，0 failed |
| ES256/JWKS/strict claims/role/identity 测试 | 10/10 passed，0 failed |
| `pgx-oauth-gate` integration test | 2/2 passed，0 failed；匹配 role/identity 成功，role mismatch 与 tamper 拒绝 |
| ABI runtime module 与 C probe 构建 | exit 0 |
| 无 gate release module 与 export symbol | exit 0；存在 `_PG_oauth_validator_module_init` |
| rustfmt | `cargo fmt --check` exit 0 |
| Clippy | 四组 locked feature 组合均以 `-D warnings` exit 0 |
| 真实 PostgreSQL 18.4 runtime SQL probe | exit 0；真实 `initdb`、server start、module load、allocator/panic probe 返回 `t`，随后正常 fast shutdown |
| Final artifact 隔离扫描 | exit 0；runtime probe `.so` 已删除；三个 gate marker 均无命中；`ldd` 无 `libcurl` |

另从同一 detached checkout 构建并直接加载 gate target：

```sh
DOCKER_BUILDKIT=1 docker build \
  --no-cache --pull --progress=plain \
  --target pgx-oauth-gate \
  --tag mtmpg-task4-pgx-oauth-gate:58d38f5-local .
```

结果为 exit 0。该构建也重新执行固定 PG source/header、C probe、locked Rust
测试、release symbol、rustfmt 与四组 Clippy；结果再次为 ABI 4/4、JWT/identity
10/10、pgx gate 2/2，全部 0 failed。Gate-specific release module symbol 与
无 `libcurl` 检查为 exit 0。两张 image 都只有本地 tag，`RepoDigests=[]`；没有
push、GHCR/Release 更新或 image/layer 导出。

## 迁移前后 image 与 module 对比

所有 inspect 只读取 ID、RepoTags、RepoDigests、Architecture、OS 与 Created；
module SHA-256 在 `--pull=never --network=none` 的临时容器内计算，没有导出
module 或 layer。

| 制品 | Image ID | Created（UTC） | Arch/OS | Module SHA-256 |
| --- | --- | --- | --- | --- |
| 迁移前 `gomtm-pggomtm:pgx-oauth-gate` | `sha256:ed5db167faa2bf91c408842d8845072f56e950d1326c594f8c4d5f99e01f554b` | `2026-07-16T16:42:44.170284071Z` | `amd64` / `linux` | `653e69305402a1887eff9ce0fdb9faa77a979281fa4a8a40c3d7c66c4869cee2` |
| 迁移后 `mtmpg-task4-pgx-oauth-gate:58d38f5-local` | `sha256:fbe2bbd25a328222ed37e32a8531eeb950df3672387544dbabe12bb6dfc2670a` | `2026-07-16T20:02:12.576424961Z` | `amd64` / `linux` | `653e69305402a1887eff9ce0fdb9faa77a979281fa4a8a40c3d7c66c4869cee2` |
| 迁移后 `mtmpg-task4-final:58d38f5-local` | `sha256:bee29ad8bcc0e36beccd88ffb8a01368f45de01951bf21eab7299ef4b45f7556` | `2026-07-16T19:45:01.20729722Z` | `amd64` / `linux` | `41019a9204fd61986a6a5a435544c86e89ff801d55715453cecf9e3d37dc73a1` |

旧、新 gate image ID 不同已调查：两者 `Created` 明确不同，且 Docker image ID
包含 image config，新的 `--no-cache` build 会生成新的构建时间元数据；仅这一项
已足以令 ID 不同。受限 inspect、固定 runtime base、相同 arch/OS 以及 gate
module SHA-256 完全相同，证明迁移前后 gate module 字节未改变。任务禁止导出
image/layer，且迁移前没有记录完整 OCI manifest/rootfs digest，因此本证据不把
module 字节相同扩大为“整个 image 逐字节相同”。

Final module SHA-256 与 gate module 不同是预期结果：final 以
`--features pg18` 构建，gate module 额外启用 `pgx-oauth-gate` 内置测试能力。
Final 的不同 digest 不是迁移回归，其 gate marker/libcurl 隔离扫描已通过。

## 完整日志与敏感扫描

完整 plain logs 只位于被 Git 忽略的 `.superpowers/sdd/`，不会提交：

| Log | 行数 | SHA-256 | Exit |
| --- | ---: | --- | ---: |
| `task-4-baseline.log` | 17 | `63173b80b0a2daad16ff88c775ee276204f2f0f0f1f25c4b46523aa396b4c324` | 1（上述额外过严断言） |
| `task-4-baseline-v2.log` | 47 | `2479ec4e748eeed8d82560b09564ab8bb7927f932f761b1dec8b208671418e12` | 0 |
| `task-4-final-build.log` | 5619 | `dd6f3032efdab1d7c78967b4c6251ae4064259770ad38cd8b1a7f7c8141c8258` | 0 |
| `task-4-gate-build.log` | 5558 | `3423c35039d3e6009cfd00bacc132519ff79aebc7d6a591193b45a744bc0018d` | 0 |
| `task-4-artifacts.log` | 10 | `83f220bb86e0375782057600687f822b842859c3940812ebec15410e869e37b8` | 0 |
| `task-4-log-secret-scan.log` | 9 | `a9de5df7d47a8063132eac157396f0a101106d19ffe52d54453597ba25eb1d5c` | 0 |

对前五个原始日志执行了仅输出命中文件名、不打印匹配内容的敏感扫描。PEM private
key header、JWT 三段格式、带凭据数据库 URI、Bearer header、常见云端/token
格式、secret/credential 赋值均为无命中（grep exit 1），扫描脚本整体 exit 0。
证据和日志未记录 private test key、完整 JWT、连接串、环境变量、Docker
credential 或未脱敏 secret。

## TDD 与已知限制

TDD 不适用于本任务：这是禁止修改源码、Cargo、Dockerfile 与 tests 的既有行为
基线复验。正确验证方式是从固定 clean checkout 运行现有测试和真实 runtime
门禁，而不是新增或修改测试。

已知限制如下：

- 当前 final artifact 仍是默认拒绝 token 的 fail-closed 原型，不能据此宣称
  production OAuth verifier 已完成；
- 本任务只复验 OpenSpec 1.4 指定的原型子集，不包含后续依赖/许可证审计、完整
  外部 config/JWKS 与真实 OAuth 正负矩阵、SBOM/provenance 或 stable release；
- 本地 image 只有可变的开发取证 tag 与 image ID，没有 registry digest，不能作为
  后续部署标识；
- 为遵守禁止导出约束，没有做旧、新完整 image layer 的逐字节比较。Gate module
  digest 完全相同是本次迁移前后 native 字节身份的直接证据。

## 交付范围复验

任务结束前安全执行了
`git worktree remove /tmp/mtmpg-task4-clean-58d38f58`，exit 0；临时 checkout
路径和 worktree 注册项均已不存在，没有使用 `git clean`。

最终 index 只新增本文件。源码、Cargo、Dockerfile、tests、OpenSpec checkbox 与
其他既有文档均未修改；`.superpowers/sdd/` 日志保持 ignored 且未暂存。
`git diff --check` 与 `git diff --cached --check` 均为 exit 0。对本文件执行与
完整日志相同的敏感模式扫描，所有类别均无命中，扫描整体 exit 0。
