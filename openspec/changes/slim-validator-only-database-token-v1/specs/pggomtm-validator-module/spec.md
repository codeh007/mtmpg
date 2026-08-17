## MODIFIED Requirements

### Requirement: Database JWT必须按闭集验证
Validator SHALL只接受database-token contract v1：固定ES256、唯一issuer/audience、`database` scope、30至300秒TTL。Claims SHALL只允许 iss、aud、sub、iat、exp、jti、scope、profile 八个字段，`sub` MUST匹配 `^[A-Za-z0-9_-]{1,64}$`，`profile` MUST属于 ordinary/business_admin/database_developer；delegation_id、auth_method、authority_version、client_id、credential_id、db_role、db_profile 等旧字段及其他未知字段 MUST deny unknown 拒绝。外部 OAuth token、长期 API key、Supabase JWT、opaque token、未知字段或算法 MUST fail closed。

#### Scenario: 合法v1 database JWT
- **WHEN** token 签名有效、claims 完整且只含最小 v1 字段、profile 合法
- **THEN** validator SHALL 授权匹配 startup role 并生成不含 secret 的规范 v1 identity

#### Scenario: 外部凭据直达PostgreSQL
- **WHEN** client 提交非 database JWT 或其他 issuer token
- **THEN** validator SHALL 拒绝且不得调用在线认证器

#### Scenario: 旧铸造链字段进入 v0.3 validator
- **WHEN** token 含 delegation_id、auth_method、authority_version、client_id、credential_id、db_role 或 db_profile 任一旧字段
- **THEN** validator SHALL deny unknown 拒绝且不得 alias、重写或回退

### Requirement: Profile与requested role必须精确匹配
Database-token contract v1 的 `profile` SHALL 只允许 ordinary、business_admin、database_developer，并 SHALL 直接以同一字符串作为 closed PostgreSQL role。Token 的 `profile` 与 startup requested role MUST 精确相等。Runtime config MUST NOT 扩展算法、issuer、profile 或 role 集合。

#### Scenario: Token请求同名role
- **WHEN** 三个合法 profile 分别请求完全同名的 startup role
- **THEN** validator SHALL 通过 profile-role 检查并继续其余认证门禁

#### Scenario: Token请求越权或未知role
- **WHEN** ordinary token 请求 business_admin、database_developer、service、migration、cluster 或未知 role
- **THEN** validator SHALL 在认证阶段拒绝，不得依赖 RLS、alias 或 SET ROLE 修正

### Requirement: Authenticated identity必须版本化且无secret
V0.3.x 授权结果 SHALL 使用 `oauth:<issuer-host>:v1;u=<userId>;p=<profile>` 规范 system_user 编码（issuer-host 取自 config issuer URL 的 host），并 SHALL 能从 PostgreSQL system_user 无歧义解析。Encoder MUST 只产生 v1 identity，decoder MUST 只接受 v1。Identity MUST NOT 包含 JWT、API key、显示名称或 key prefix；旧 `oauth:pggomtm:v2;...`、非法、超长或未知版本 MUST 拒绝而非截断或兼容解码。

#### Scenario: V1 identity往返
- **WHEN** 合法 v1 token 完成认证
- **THEN** `authn_id -> system_user -> decoded identity` SHALL 无损保留 user/profile 与 issuer-host 且不含 secret

#### Scenario: 旧v2 identity进入v0.3 decoder
- **WHEN** system_user 含 `oauth:pggomtm:v2;...` 或未知版本
- **THEN** decoder SHALL 拒绝且不得转换为 v1 identity
