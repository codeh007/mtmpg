## Why

Gomtmui需要把已认证的MCP delegation转换为真实PostgreSQL 18 OAuth连接，但其TypeScript驱动和常规Rust PostgreSQL驱动都不支持所需的OAUTHBEARER token注入。在gomtmui内保留Go生产服务会引入第二技术栈并割裂mtmpg认证契约，因此executor应由mtmpg以Rust/libpq 18 companion artifact统一实现和发布。

## What Changes

- **BREAKING**：将mtmpg从“只发布validator image”的仓库边界扩展为同一Rust权威下两个隔离制品：现有`pggomtm` PostgreSQL image与新的私网SQL executor image；已发布`v0.2.0` validator及contract v2保持不可变。
- 新增私网Rust HTTPS executor：验证HMAC request envelope，把严格`DelegatedPrincipal`转换为30秒ES256 database JWT，并只通过PostgreSQL 18 libpq auth-data hook把token交给当前`PGconn*`。
- 使用并发安全、一次性的`PGconn* -> token` registry隔离不同请求；连接成功、失败、取消或关闭时清零，未知、重复和清理后的连接全部fail closed。
- 通过libpq extended protocol执行一个带结构化binds的顶层statement，提供read-only/confirmed-change事务、预算、取消、结构化结果、稳定SQLSTATE分类和不记录SQL/credential的审计。
- 新增独立`ghcr.io/codeh007/mtmpg-executor:0.1.0`非root image与版本化release入口；标准GitHub CI统一解析Rust/PG18/libpq输入，运行Rust、并发、真实PG18、TLS、OAuth、取消和final-image门禁并生成SBOM、provenance与attestation。
- 修改仓库规则、README与维护文档，允许职责明确的companion crate、HTTP服务和executor image定义，同时继续禁止消费者源码副本、本地image fallback和validator/executor契约复制。

## Capabilities

### New Capabilities

- `delegated-sql-executor`: 定义HMAC私网服务、database JWT issuer、per-`PGconn` libpq OAuth、单statement事务、预算、取消、响应与审计契约。

### Modified Capabilities

- `pggomtm-release-supply-chain`: 将唯一源码/CI/发布权威从单一validator image扩展为彼此隔离的validator与executor制品，并定义独立版本、image、测试和供应链材料。

## Impact

- Rust源码与依赖：Cargo package布局、libpq 18 bindings、TLS/HMAC/JWT/HTTP runtime及executor领域测试。
- 真实系统验证：不同principal并发OAuth连接、extended protocol、rollback、budget、cancel和PG18 validator集成。
- 交付：executor image定义、CI/release workflow、manifest、SBOM、provenance、attestation、README、AGENTS与维护文档。
- 消费边界：gomtmui只消费`mtmpg:0.2.0`与versioned executor image，不复制Rust源码、Cargo工具链或native测试。
