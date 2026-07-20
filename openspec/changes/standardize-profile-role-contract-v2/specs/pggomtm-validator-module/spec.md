## MODIFIED Requirements

### Requirement: Database JWT必须按闭集验证
Validator SHALL只接受database-token contract v2：固定ES256、唯一issuer/audience、database scope、30至300秒TTL、完整claims、actor二选一、正authority version，以及`ordinary`、`business_admin`或`database_developer`三个closed profile。`db_role` SHALL与`db_profile`精确同名。外部OAuth token、长期API key、Supabase JWT、opaque token、v1 profile/role、未知字段或算法 MUST fail closed。

#### Scenario: 合法v2 database JWT
- **WHEN** token签名有效、claims完整、只含`client_id`或`credential_id`之一，且profile与role使用同一v2通用名称
- **THEN** validator SHALL授权匹配startup role并生成不含secret的规范v2 identity

#### Scenario: 外部凭据直达PostgreSQL
- **WHEN** client提交非database JWT或其他issuer token
- **THEN** validator SHALL拒绝且不得调用在线认证器

#### Scenario: V1 token提交给v0.2 validator
- **WHEN** token使用`business-admin`、`database-developer`、任一`gomtm_candidate_*`、`gomtm_*`或其他旧profile/role名称
- **THEN** validator SHALL fail closed且不得alias、重写、继承或回退到v1 contract

### Requirement: Profile与requested role必须精确匹配
Database-token contract v2的`db_profile` SHALL只允许`ordinary`、`business_admin`和`database_developer`，并 SHALL直接使用同一字符串作为closed PostgreSQL role。Token中的`db_role`、startup requested role和profile MUST三者精确相等。Runtime config MUST NOT扩展算法、issuer、profile或role集合。

#### Scenario: Token请求同名role
- **WHEN** 三个合法v2 profile分别请求其完全同名的startup role
- **THEN** validator SHALL通过profile-role检查并继续执行其余认证门禁

#### Scenario: Token请求越权或旧role
- **WHEN** ordinary token请求`business_admin`、`database_developer`、service、migration、cluster、带项目/阶段前缀或未知role
- **THEN** validator SHALL在认证阶段拒绝，不得依赖后续RLS、alias或`SET ROLE`修正

### Requirement: Authenticated identity必须版本化且无secret
V0.2.x授权结果 SHALL使用`pggomtm:v2`规范`authn_id`编码user、client-or-credential、delegation、auth method、authority version与v2 profile，并 SHALL能从PostgreSQL `system_user`无歧义解析。Encoder MUST只产生v2 identity，decoder MUST只接受v2。Identity MUST NOT包含JWT、API key、显示名称或key prefix；v1、非法、超长或未知版本 MUST拒绝而不是截断或兼容解码。

#### Scenario: V2 identity往返
- **WHEN** 合法v2 token完成认证
- **THEN** `authn_id -> system_user -> decoded identity` SHALL无损保留归因字段与下划线profile且不包含secret

#### Scenario: V1 identity进入v0.2 decoder
- **WHEN** `system_user`包含`pggomtm:v1` identity或连字符profile
- **THEN** decoder SHALL拒绝且不得转换为v2 identity
