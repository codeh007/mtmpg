# Issue #116：Cargo 依赖与许可证审计证据

本证据于 2026-07-17（UTC）采集，用于完成 OpenSpec 任务 7.2。结论来自精确远端
GitHub Actions source；本地 Docker image 和终端输出只用于提交前定位，不作为任务
完成依据。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29608897724](https://github.com/codeh007/mtmpg/actions/runs/29608897724) |
| Job | `Native authority`，ID `87978635435` |
| Remote head SHA | `135889852d5473e68fca36a1e1a774c0e8c14b0e` |
| Event | `push` |
| Job 时间 | `2026-07-17T19:48:35Z` 至 `2026-07-17T19:56:18Z`，7 分 43 秒 |
| 结论 | `success` |
| Actions artifact | 0 |

远端 run 从根 `Dockerfile` 下载 `cargo-deny 0.20.2` 官方 Linux x64 musl 包，先验证
固定 SHA-256
`9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f`，再对
`Cargo.lock`、`x86_64-unknown-linux-gnu` 和 `pg18` feature 执行 locked advisory、
license、ban 与 source 检查。工具包 checksum、审计命令及后续完整 native build graph
均在该 run 中实际执行成功。

## 审计结果

远端脱敏汇总为：

| 检查 | Errors | Warnings | Notes |
| --- | ---: | ---: | ---: |
| Advisories | 0 | 0 | 2 |
| Bans | 0 | 6 | 0 |
| Licenses | 0 | 0 | 151 |
| Sources | 0 | 0 | 0 |

六项 ban warning 是完整 inclusion graph 中可见的重复版本：`getrandom`、`hashbrown`、
`shlex`、`toml`、`toml_datetime` 与 `winnow`。配置没有用 `skip` 或 `skip-tree`
隐藏它们；wildcard version、未知 registry 与 Git dependency 仍为 deny。

许可证检查显式覆盖 production、build 与 dev dependency，只允许当前依赖图实际使用的
七种 SPDX 许可证，并把未使用 allowlist 项设为 error。远端结果没有未知、未批准或未
遇到的许可证 warning。

## RustSec 逐项处置

当前完整 pgrx 传递依赖只存在一个已接受 advisory：

| Advisory | Dependency path | 处置 | 复核期限 |
| --- | --- | --- | --- |
| `RUSTSEC-2021-0127` | `pggomtm -> pgrx 0.19.1 -> serde_cbor 0.11.2` | 精确 ID 临时接受；unmaintained、无安全升级，不用未验证 patch 改写 pgrx FFI 图 | 2026-08-17，且不得晚于任何 pgrx 更新或首个 stable release |

完整理由和许可证策略记录在
[`docs/dependency-audit.md`](../../dependency-audit.md)。`deny.toml` 没有全局 advisory
ignore、类别放宽或无理由例外；条件变化时必须删除该例外或通过单独技术审查升级
pgrx。

本证据只完成依赖、RustSec、许可证与来源审计。该 run 使用普通 GitHub Actions cache，
不是任务 7.7 或 10.1 的 cold authority，也不代表 public-readiness、candidate image、
SBOM、provenance 或 stable 发布已经完成。
