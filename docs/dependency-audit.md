# 依赖安全与许可证审计

本文定义 mtmpg 对完整 Cargo dependency graph 的 RustSec、许可证、来源和重复版本门禁。审计覆盖 production、build 与 test 所需依赖，因为 build script、原生 probe 和测试 gate 都属于发布供应链的一部分。

## 审计权威

- 工具：`cargo-deny 0.20.2`
- 官方发布包：`cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz`
- 发布包 SHA-256：`9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f`
- 目标：`x86_64-unknown-linux-gnu`
- Cargo feature：`--no-default-features --features pg18`
- Lock 语义：`--locked`
- 配置权威：[`deny.toml`](../deny.toml)

GitHub Actions中的固定native toolchain container从cargo-deny官方GitHub Release下载精确版本，先验证固定SHA-256，再由`scripts/native-test dependencies`执行审计。该入口在普通本地环境拒绝运行；不得使用未固定的action、系统包、`cargo install`最新版或开发者本机工具替代远端Actions结果。

## RustSec 例外

当前锁文件只命中一个 RustSec advisory：

| Advisory | Dependency path | 类型 | 当前结论 | 下次复核 |
| --- | --- | --- | --- | --- |
| `RUSTSEC-2021-0127` | `pggomtm -> pgrx 0.19.1 -> serde_cbor 0.11.2` | unmaintained | 临时接受精确 advisory | 2026-08-17，且不得晚于任何 pgrx 更新或首个 stable release |

接受理由：

- Advisory 表示 `serde_cbor` 已停止维护，不是已知内存安全漏洞或可利用漏洞公告。
- pggomtm 源码、build script 与测试不直接调用 `serde_cbor`；它只由锁定的完整 pgrx 传递引入。
- 该 advisory 没有安全升级版本。单独替换或 patch 传递依赖会形成未验证的 pgrx/FFI 变体，风险高于当前精确锁定状态。
- 正式 module 继续接受无网络/SQL、动态依赖、ELF symbol/string 与最终 filesystem 门禁；这些门禁不能消除 unmaintained 风险，但会限制其可达能力面。

`deny.toml` 只忽略精确 `RUSTSEC-2021-0127`，并在 reason 中记录复核期限。不得把全部 unmaintained、unsound、yanked 或 vulnerability 类别设为 allow，也不得增加无理由 advisory ID。复核时必须重新检查 pgrx 上游、可替代版本、实际 dependency path 与正式 artifact；条件变化后删除例外或通过单独审查升级 pgrx。

## 许可证策略

全图只允许当前实际需要的明确 SPDX 许可证：

- `MIT`
- `Apache-2.0`
- `BSD-3-Clause`
- `ISC`
- `Unicode-3.0`
- `Unlicense`
- `Zlib`

该列表不按“OSI 全部允许”或通配表达式放宽，production、build 与 dev dependency 都接受同一检查，未被当前依赖图使用的 allowlist 项也会使审计失败。依赖表达式包含多种可选许可证时，只要存在上述允许分支即可；例如 `r-efi` 的可选 `LGPL-2.1-or-later` 不会因此成为全局允许许可证。新增许可证必须单独核对 SPDX 文本、分发义务和最终 image/bundle 影响后修改配置。

## Dependency 与来源策略

- Workspace manifest 中的 wildcard version 一律拒绝。
- 重复 crate version 当前产生 warning 并显示 inclusion graph，不使用 `skip` 或 `skip-tree` 隐藏。
- 只允许 crates.io registry；未知 registry 与 Git dependency 一律拒绝。
- Cargo、RustSec、license 或 source 门禁失败时，CI 必须失败，不得通过全局 ignore、删除 lockfile、弱化 lint 或跳过完整 pgrx graph 获得通过。

审计结果只记录 advisory ID、crate、版本、dependency path、许可证标识与结论，不复制 secret、环境变量或连接信息。
