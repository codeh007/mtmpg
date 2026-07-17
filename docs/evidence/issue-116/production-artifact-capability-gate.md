# Issue #116：production artifact capability gate 证据

本证据于 2026-07-17（UTC）采集，用于完成 OpenSpec 任务 6.6。结论只来自远端
GitHub Actions；本地没有用完整 Docker 构建替代远端证据。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29601404193](https://github.com/codeh007/mtmpg/actions/runs/29601404193) |
| Job | `Native authority`，ID `87954125914` |
| Remote head SHA | `70358dc8a084ab35b4c25d46943a2a9bf5396fa4` |
| 结论 | `success` |
| Actions artifact | 0 |

## Module 级边界

远端 production 构建使用无 `abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate` 的正式
feature，且只允许 ELF 导出符号 `Pg_magic_func` 与
`_PG_oauth_validator_module_init`。ELF 还必须是 little-endian ELF64、amd64、DYN。

DT_NEEDED 只允许运行时基础依赖（`libgcc_s.so.1`、`libc.so.6` 与 loader），不包含
HTTP、DNS、libcurl、SQL、SPI 或第二认证实现。artifact gate 同时扫描并拒绝测试
JWKS、raw signing key、compact JWT、probe symbol/string、gate marker、测试 module
以及不批准的动态依赖。

Dockerfile 的 final stage 不再重复 module gate；正式 artifact 的 capability gate 是
本任务唯一权威。后续 7.x CI、SBOM、发布和 cold authority 门禁不由本证据提前宣称。
