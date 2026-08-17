## Context

gomtm issue #310 硬切方向已批准。executor 是独立 HTTP 二进制（axum 0.8 + axum-server TLS + libpq），与 PostgreSQL 内核无关，其铸币/HMAC/signer 职责上移到 gomtmui（better-auth 签名）与 gomtm（Go sql-relay 透传）。mtmpg 只保留必须驻留在数据库内核中的 validator（`pggomtm.so`）。

当前 database-token contract v2 携带铸造链字段（delegation_id/auth_method/authority_version/client_id/credential_id/db_role + db_profile/db_role 双字段），这些字段源于 executor 把 DelegatedPrincipal 转写进 token。硬切后数据库层只需要「真实 issuer 签给该用户、允许访问本库、在有效期内、profile 合法」的最小契约。

## Goals / Non-Goals

**Goals:**

- 删除 executor 产品全部源码、测试、Dockerfile、CI/release 入口与文档。
- database-token 契约最小化为 v1：iss/aud/sub/iat/exp/jti/scope/profile，deny unknown。
- system_user 身份编码改为 `oauth:<issuer-host>:v1;u=<userId>;p=<profile>`。
- validator package 升 0.3.0，保持离线 JWKS、双文件契约、fail-closed、reason-code 闭集、ES256/P-256、aud≠iss、TTL 30–300s、零网络/SQL/SPI 不变。
- 通过现有标准 CI 全绿，PR 交付。

**Non-Goals:**

- 不提供 executor 兼容层、过渡设计或第三 crate。
- 不保留旧 identity 解码、旧 claims 字段、旧 profile-role 映射、alias 或 fallback。
- 不物理删除他人 openspec change 文件；只标注被硬切取代。
- 不在本地运行 Cargo/Docker/PostgreSQL。

## Decisions

### 1. Executor 整个物理删除，不做软删除

executor 是独立二进制、无复用价值（职责已外置），按 AGENTS.md「无复用价值直接删除」处理，不保留 `--` 软删除文件。根 Dockerfile 保留，因为 CI final-image 测试依赖它构建并验证 production `pggomtm.so` 镜像。

### 2. 最小 claims 与 deny unknown

`DatabaseTokenClaims` 仅保留 iss/aud/sub/iat/exp/jti/scope/profile，`#[serde(deny_unknown_fields)]` 保持。删除的旧字段（delegation_id/auth_method/authority_version/client_id/credential_id/db_role/db_profile）在反序列化时作为 unknown 字段被拒绝为 InvalidToken，不新增显式字段白名单逻辑。

sub 采用 `^[A-Za-z0-9_-]{1,64}$`（复用 `is_valid_internal_id`，字节级等价约束）；jti 保持现有 internal-id 约束（alphanumeric + `_`/`-`，≤64 字节）。

### 3. profile 即角色

`DatabaseProfile` 保持三值 ordinary/business_admin/database_developer，`database_role()` 返回同名。删除 db_role 字段后，validate_claims 直接比较 requested_role 与 profile 的 role 名，不等返回 RequestedRoleMismatch。

### 4. 身份编码 `oauth:<issuer-host>:v1`

identity 字段收敛为 user_id + profile + issuer_host（issuer_host 取自 policy.issuer URL 的 host）。编码为 `<issuer-host>:v1;u=<userId>;p=<profile>`，system_user 前缀 `oauth:` 不变。解码按 `;` 切 3 段，首段用 `rsplit_once(':')` 分离 issuer-host 与版本 `v1`（兼容 IPv6 host 的方括号形式），版本不符或字段缺失均 InvalidIdentity。

### 5. reason-code 闭集不变

现有 24 个 reason-code 全部仍被新契约使用（旧 claims 字段此前只映射到 token-claims-invalid / identity-invalid 一类，无专用 code），因此 `auth_failure.rs` 与闭集字符串不变，不新增/删除/改名 code。

### 6. 版本与发布

validator `Cargo.toml` 升 0.3.0（契约变更 = 新 module+consumer 版本，按 docs/runtime-configuration.md 规则）。release 仅发布 `pggomtm.so` 宿主产物（validator image 仍由 CI final-image 验证）；`executor-v*` release 入口删除。

## Risks / Trade-offs

- 旧 v0.2.x 调用方（executor 铸的 v2 token/identity）全部失效——显式硬切，消费者同步切换。
- CI 结构较大改动（删除 executor 三个 job + release 分支）——保持 validator job 的解析/ABI/PG18/image 门禁不变，删除部分只触及 executor。
- 他人 deps PR（#10）可能先合并——push 前 rebase 最新 origin/main，冲突时按本任务优先删除 executor。

## Open Questions

无。
