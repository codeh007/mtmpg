# Issue #116：pggomtm 迁移前基线

本证据于 2026-07-16T17:36:03Z（UTC）只读采集。源码范围是
`/workspace/gomtmui/native/pggomtm`；采集过程中未复制或修改原型，未读取、
哈希或导出任何 `target/` 内容，也未导出 Docker image 或 layer。

## 源码清单

- 清单：`docs/evidence/issue-116/pggomtm-source.sha256`
- 普通文件数量：13
- 清单 SHA-256：
  `8ec180c6b82602fe1b943e75bf29166f2a0bdeefb62a86e57f82c861b1679ba9`
- 排除规则：枚举时剪枝任意层级名为 `target` 的目录；本次发现并排除
  `native/pggomtm/target/`。
- 清单按 `LC_ALL=C` 的相对路径稳定排序；每个路径都以
  `native/pggomtm/` 开头，可从 `/workspace/gomtmui` 直接执行
  `sha256sum --check`。

枚举与哈希使用以下只读命令：

```sh
cd /workspace/gomtmui
find native/pggomtm -type d -name target -prune -o -type f -print0 \
  | LC_ALL=C sort -z \
  | xargs -0 --no-run-if-empty sha256sum
```

## Cargo feature 与门禁基线

原型 `Cargo.toml` 的 feature 定义和源码条件编译如下。三个名称含 `gate`
的 feature 都只用于测试或取证，不是生产能力。

| Feature | 默认启用 | 当前用途 |
| --- | --- | --- |
| `pg18` | 是（`default = ["pg18"]`） | 启用 `pgrx/pg18` 与 `pgrx-pg-sys/pg18`，是当前所有构建组合的 PostgreSQL 绑定基线。 |
| `abi-gate` | 否 | 供进程内 ABI/layout 与 panic 边界测试使用：测试构建不加 `pg_guard`，把 startup 错误转换为可捕获 panic，并暴露 panic gate。 |
| `abi-runtime-gate` | 否 | 启用只供真实 PostgreSQL runtime probe 使用的 panic 与 allocator 特殊输入；Docker 中构建独立 probe module，验证后从最终 image 删除并扫描其标记字符串。 |
| `pgx-oauth-gate` | 否 | 启用内置测试 JWKS、测试 verifier、对应 Rust integration test 和本地 `pgx-oauth-gate` image；只验证候选 JWT/role/identity 路径，不是正式 verifier。 |

现有构建和测试入口：

- 正式 Docker artifact 使用
  `cargo build --locked --release --no-default-features --features pg18`；最终
  image 只复制该无 gate 的 `libpggomtm.so`。
- 无 `pgx-oauth-gate` 时，validate callback 先把结果复位为
  `authorized=false`、`authn_id=NULL`，随后仅报告 callback 已处理，仍拒绝
  token。因此当前默认/正式构建是 fail-closed 原型，不具备生产验签能力。
- `tests/abi_layout.rs` 覆盖 Rust ABI layout、callback 初始化与 fail-closed、
  当前精确 PG 18.4 minor gate；启用 `abi-gate` 时追加 panic 边界测试。
- `tests/jwt_identity.rs` 覆盖 ES256/JWKS、严格 claims、closed role/profile 与
  版本化 identity；`tests/pgx_oauth_gate.rs` 整个文件受
  `pgx-oauth-gate` 条件编译，覆盖匹配 role 成功及 role/tamper 拒绝。
- `tests/oauth_layout_probe.c` 是官方 header 的 C layout probe；
  `tests/oauth_runtime_probe.c` 与 `tests/oauth_runtime_probe.sql` 是真实
  PostgreSQL loader、allocator 与 panic runtime gate。

## 本地 image 身份

只读取所需 inspect 字段：

```sh
docker image inspect gomtm-pggomtm:pgx-oauth-gate \
  --format '{{json .Id}}|{{json .RepoTags}}|{{json .RepoDigests}}|{{json .Architecture}}|{{json .Os}}|{{json .Created}}'
```

| 字段 | 采集值 |
| --- | --- |
| Image ID | `sha256:ed5db167faa2bf91c408842d8845072f56e950d1326c594f8c4d5f99e01f554b` |
| RepoTags | `["gomtm-pggomtm:pgx-oauth-gate"]` |
| RepoDigests | `[]`（无） |
| Architecture | `amd64` |
| OS | `linux` |
| Created | `2026-07-16T16:42:44.170284071Z` |

该本地 tag/image 只作为迁移前身份记录，不属于源码清单或迁移内容。

## mtmpg 仓库基线

| 项目 | 采集值 |
| --- | --- |
| Worktree | `/workspace/mtmpg/.worktrees/issue-116-extract-pggomtm` |
| 分支 | `issue-116-extract-pggomtm` |
| HEAD | `c062f5512eb20d324694e8cee6370da9bc5f5dc7` |
| 取证开始时状态 | `clean` |
| `origin` fetch URL | `https://github.com/codeh007/mtmpg` |
| `origin` push URL | `https://github.com/codeh007/mtmpg` |
| 远程默认分支 | `main` |
| 远程默认分支 HEAD | `453b2a71c8f98b8824278f2d469683626c6d7e71` |
| GitHub visibility | `private`（`isPrivate=true`） |

默认分支由 `git ls-remote --symref origin HEAD` 确认；仓库可见性和 GitHub
默认分支由
`gh repo view codeh007/mtmpg --json nameWithOwner,isPrivate,defaultBranchRef`
只读确认。未读取或输出凭据、环境变量、连接串或私密配置。

## 复验方法

```sh
cd /workspace/gomtmui
manifest=/workspace/mtmpg/.worktrees/issue-116-extract-pggomtm/docs/evidence/issue-116/pggomtm-source.sha256

! grep -E '(^|/)target/' "$manifest"
source_count=$(find native/pggomtm -type d -name target -prune -o -type f -print0 \
  | awk -v RS='\0' 'END { print NR }')
manifest_count=$(wc -l < "$manifest")
test "$source_count" -eq "$manifest_count"
sha256sum --check "$manifest"
sha256sum "$manifest"
```

上述复验预期得到 13 个源文件、13 条清单记录、全部 checksum 成功，以及清单
自身 SHA-256 与本文记录一致。本基线不要求 Rust 或 Docker 构建测试。
