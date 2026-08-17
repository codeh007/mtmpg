## Why

gomtm issue #310 已批准硬切：mtmpg 只保留数据库内核部分（`pggomtm.so` validator），Rust SQL executor 整个删除，不做任何兼容/过渡设计。executor 是独立 HTTP 二进制（axum + libpq），与 PostgreSQL 内核无关，其唯一 DB 接点是 libpq 客户端；其 HTTP 端点、HMAC 验签、ES256 signer key 与 libpq OAuth hook 等职责全部外置到 gomtm（Go sql-relay 透传）与 gomtmui（better-auth 签名上移）。删除后 mtmpg 只保留「数据库层按用户身份限权」所需的最小内核干预：validator 校验真实 issuer 签发的短令牌 + 3 个 profile 角色 + per-user RLS。

同时把 database-token 契约最小化到 v1：数据库层只需要「真实 issuer 签给这个用户、允许访问本库、在有效期内、profile 合法」，删除 delegation_id、auth_method、authority_version、client_id、credential_id、db_role 等铸造链字段，`profile` 即数据库角色。

## What Changes

- **BREAKING**：删除 executor 产品全部——`executor/` 源码/测试/Dockerfile、根 Cargo workspace 成员、`executor-v*` release 入口与 executor CI 步骤、`docs/executor-runtime.md`、README/MAINTAINERS 中 executor 引用、共享 PG18 harness 的 run-executor 矩阵。
- **BREAKING**：database-token contract 收敛为 v1 最小 claims（iss/aud/sub/iat/exp/jti/scope/profile），deny unknown；删除 delegation_id/auth_method/authority_version/client_id/credential_id/db_role，`profile` 即数据库角色（startup role 必须与 profile 精确同名）。
- **BREAKING**：system_user 身份编码改为 `oauth:<issuer-host>:v1;u=<userId>;p=<profile>`，彻底移除 `oauth:pggomtm:v2;u=...;actor=...;d=...;m=...;a=...;p=...` 编码及前缀常量。
- 不变项：离线 JWKS 快照、`/etc/pggomtm` 双文件契约（schema `pggomtm-validator-config/v1`）、fail-closed、reason-code 脱敏闭集、ES256/P-256/`use=sig`/`key_ops=verify`、aud≠iss、TTL 30–300s、拒绝网络/SQL/SPI。
- validator `Cargo.toml` 升 `0.3.0`（契约变更 = 新 module+consumer 版本）。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `pggomtm-validator-module`：database-token 与 identity 契约收敛为最小 v1。
- `pggomtm-release-supply-chain`：仓库边界收敛为 validator-only（删除 executor 产品、CI 与 release 线）。

## Impact

- Rust 源码：`src/database_auth.rs`（claims、identity 编码、枚举与错误路径）、`src/runtime_config.rs`（测试 fixture）。
- 测试：Rust JWT/identity 领域矩阵、共享 OAuth fixture、真实 PG18 harness（system_user 断言）与 final-image smoke；ABI 测试不动；删除 executor 全部测试。
- 交付：根 Cargo.toml、Dockerfile、`.dockerignore`、`.github/workflows/ci.yml` 与 `release.yml`、README、MAINTAINERS、`docs/runtime-configuration.md`、`docs/authentication-failures.md`、`docs/release-and-compatibility.md`、删除 `docs/executor-runtime.md`。
- OpenSpec：`publish-rust-sql-executor` 与 `release-host-artifacts` 的 executor 相关任务标注「被 issue-310 硬切取代」后保留为历史；`standardize-profile-role-contract-v2` 与本任务重叠，同样标注取代。
- 消费者：gomtm（M2 sql-relay）与 gomtmui（M3 签名上移）必须按本契约签发/解析最小 v1 令牌与 `oauth:<issuer-host>:v1` identity。
