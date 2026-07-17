# Issue #116：追溯式 public-readiness 门禁证据

本证据于 2026-07-17（UTC）采集，用于完成 OpenSpec 任务 7.3。结论来自精确远端
GitHub Actions source；本地完整 Docker 构建只用于提交前诊断，不作为任务完成依据。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29615379490](https://github.com/codeh007/mtmpg/actions/runs/29615379490) |
| Job | `Native authority`，ID `87999221380` |
| Remote head SHA | `e6e932d7cb0dbfa73287eca51e484bf2be0451b4` |
| Event | `push` |
| Job 时间 | `2026-07-17T21:38:25Z` 至 `2026-07-17T21:46:13Z`，7 分 48 秒 |
| 结论 | `success` |
| Actions artifact | 0 |

远端 run 在 Buildx 前完成精确 source identity、固定 scanner 安装和全部历史与工作树
扫描，再由根 `Dockerfile` 执行同一门禁的 fixture、Docker context、Cargo、ABI、Rust、
Clippy 和真实 PostgreSQL 18.4 矩阵。所有步骤均成功。

## 固定工具与脱敏边界

- Gitleaks 固定为 `8.30.1`，官方 Linux x64 归档 SHA-256 为
  `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb`。
- ShellCheck 固定为 `0.11.0`，官方 Linux x86_64 归档 SHA-256 为
  `8c3be12b05d5c177a04c29e3c78ce89ac86f1595681cab149b65b97c4e227198`。
- Scanner 强制 `--redact=100`，并在输出前验证每个 finding 的 secret 字段已经变为
  `REDACTED`。失败输出只保留 rule、路径、行号、commit 和 fingerprint。
- Scanner 强制把 ignore 文件指向 `/dev/null`，并禁用 `gitleaks:allow` 行内旁路；
  fixture 已证明行内注释不能绕过门禁。
- Git history 使用 `--all`，当前目录扫描覆盖 tracked、ignored 与 uncommitted 文件；
  workflow 使用 `fetch-depth: 0`，Docker 内扫描实际复制到 `/src` 的最小构建输入。

## 精确 allowlist

仓库没有全局 ignore。仅存在以下两个同时约束 rule、路径、内容或 commit 的例外：

| 类别 | 精确边界 | 理由 |
| --- | --- | --- |
| 合成哨兵 | `private-key`；`tests/fixtures/public-readiness/synthetic-private-key.pem`；完整三行合成内容 | 只用于证明门禁可检测 private-key，内容不是凭据 |
| 历史误报 | `private-key`；commit `0563a822817c107cfb078b501002c035e8b42ee8`；`tests/production_capability_gate.rs`；完整跨行模式 | 该历史测试只拒绝 private-key marker，没有包含 PEM key |

Fixture 同时证明哨兵换路径、修改内容、只从 HEAD 删除、留在未提交工作树或进入 workflow
log bundle 时均被拒绝且不会回显原值。

## 追溯 surface 契约

`pggomtm-public-readiness-bundle/v1` 固定要求以下 11 个 surface 全部声明状态与数量：

1. 全部远端 Git refs 与 history；
2. tracked、ignored 与 uncommitted 工作树；
3. Docker context；
4. workflow source；
5. workflow logs；
6. Actions artifacts；
7. Actions caches；
8. Releases 与 packages；
9. GitHub Issues 及评论；
10. Pull Requests、普通讨论、review comments、reviews 与 files；
11. 按完整 OCI digest 指定的 candidate image。

Release asset 使用 `Accept: application/octet-stream` 下载实际 bytes。Candidate image 只接受
`ghcr.io/<owner>/mtmpg-postgres@sha256:<64 hex>`，随后执行 pull、save 和嵌套 archive
扫描；若 package 已存在但没有提供精确 digest，surface 必须为 `unresolved`。Actions
cache 没有内容下载 API，只能物化 metadata，因此只要存在就必须为 `unresolved`，不能
声称 cache 内容已经扫描。

## 当前公开状态只读复核

远端实现 run 成功后再次执行真实只读 retrospective，得到：

| Surface | 状态 | 数量 |
| --- | --- | ---: |
| Git refs/history | `materialized` | 2 |
| tracked/uncommitted | `materialized` | 1 |
| Docker context | `materialized` | 1 |
| workflow source | `materialized` | 1 |
| workflow logs | `materialized` | 16 |
| Actions artifacts | `absent` | 0 |
| Actions caches | `unresolved` | 176 |
| Releases/packages | `absent` | 0 |
| GitHub Issues | `absent` | 0 |
| GitHub Pull Requests | `absent` | 0 |
| candidate image | `absent` | 0 |

本地 history、工作树、远端 history 和已物化 surface bundle 均为 `0 findings`。命令最终
以非零状态退出，只因为 176 个 cache 无法下载并被正确标记为 `unresolved`。

本证据只完成可持续追溯门禁的实现与远端验证。任务 7.6 仍须在最终功能 ref 上执行完整
追溯扫描；任务 7.7 仍须删除无法证明安全的旧 cache，并从 clean checkout 完成无缓存
cold build。当前 run 使用普通 Actions cache，也没有 candidate image，因此不代表 cold、
candidate、release 或 stable readiness。
