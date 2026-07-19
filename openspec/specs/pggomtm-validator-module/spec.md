# pggomtm-validator-module Specification

## Purpose
定义pggomtm作为PostgreSQL 18 OAuth validator module的ABI、离线验证、安全边界与发布兼容性契约。

## Requirements
### Requirement: pggomtm必须是PostgreSQL OAuth validator module
`pggomtm` SHALL构建为由PostgreSQL `oauth_validator_libraries`加载的Rust `cdylib`，并 SHALL导出有效`PG_MODULE_MAGIC`与`_PG_oauth_validator_module_init`。它 MUST NOT依赖control文件、versioned SQL、`CREATE EXTENSION`或HTTP服务。

#### Scenario: PostgreSQL加载module
- **WHEN** 当前受支持的PG18 runtime把`pggomtm.so`放入真实`pg_config --pkglibdir`并配置`oauth_validator_libraries='pggomtm'`
- **THEN** server SHALL通过module magic、OAuth magic和callback table加载module，无需创建SQL对象

### Requirement: OAuth ABI必须来自本次目标PG18 header
构建 SHALL从本次CI解析的目标`pg_config --includedir-server/libpq/oauth.h`生成最小allowlist Rust bindings，并 SHALL以官方C compiler验证size、offset和callback layout。构建 MUST NOT依赖手写struct、复制magic常量、预批准header/bindings hash或某个PostgreSQL minor的固定bindings文本。

#### Scenario: 当前PG18 header生成有效ABI
- **WHEN** CI使用当前PG18 development package构建module
- **THEN** bindgen SHALL只生成所需OAuth类型和常量，且C/Rust layout、callback签名与最终编译输入 SHALL一致

#### Scenario: Header或layout不兼容
- **WHEN** 当前PG18 header缺失所需symbol、bindgen无法生成allowlist或C/Rust layout不一致
- **THEN** build SHALL失败且不得回退到旧header digest、缓存bindings或手写ABI

### Requirement: pgrx必须保护PostgreSQL FFI边界
Crate SHALL使用与当前稳定Rust和PG18兼容的`pgrx`提供module magic、guard、PostgreSQL error和allocator语义，并 SHALL通过`pgrx::pg_sys`访问raw symbol。Panic、PostgreSQL ERROR、NULL或allocator失败 MUST在callback边界fail closed，不得产生未定义行为。

#### Scenario: Callback异常
- **WHEN** startup、validate或shutdown发生panic、ERROR或非法输入
- **THEN** module SHALL拒绝认证并保持`authorized=false`、`authn_id=NULL`

### Requirement: Validator必须建立只读离线snapshot
每个新OAuth backend SHALL在startup从版本化只读config与public JWKS建立不可变snapshot，并验证权限、大小、schema、issuer、audience、key数量和ES256 public key类型。Module MUST NOT读取private key、API key、连接串或其他secret，也 MUST NOT执行HTTP、DNS、SQL、SPI或在线introspection。

#### Scenario: Config或JWKS无效
- **WHEN** 文件缺失、权限不安全、schema错误、kid重复、包含private key或资源配置非法
- **THEN** startup SHALL fail closed且不得使用内置key、旧cache、Web endpoint或备用issuer

#### Scenario: 原子轮换public JWKS
- **WHEN** 平台原子替换active与retiring public keys
- **THEN** 后续新backend SHALL读取完整新snapshot，既有backend保持原snapshot且不得观察半写文件

### Requirement: Database JWT必须按闭集验证
Validator SHALL只接受固定ES256、唯一issuer/audience、database scope和30至300秒TTL的database JWT，并 SHALL验证完整claims、actor二选一、authority version、profile、role、ID格式和时间。外部OAuth token、长期API key、Supabase JWT、opaque token、未知字段或算法 MUST fail closed。

#### Scenario: 合法database JWT
- **WHEN** token签名有效、claims完整且只含`client_id`或`credential_id`之一
- **THEN** validator SHALL授权匹配role并生成不含secret的规范identity

#### Scenario: 外部凭据直达PostgreSQL
- **WHEN** client提交非database JWT或其他issuer token
- **THEN** validator SHALL拒绝且不得调用在线认证器

### Requirement: Profile与requested role必须精确匹配
`db_profile` SHALL只映射到versioned contract声明的closed PostgreSQL role集合，token中的`db_role` MUST与startup requested role及profile映射精确相等。Runtime config MUST NOT扩展算法、issuer或role映射。

#### Scenario: Token请求越权role
- **WHEN** ordinary token请求admin、developer、service、migration、cluster或未知role
- **THEN** validator SHALL在认证阶段拒绝，不得依赖后续RLS或`SET ROLE`修正

### Requirement: Authenticated identity必须版本化且无secret
授权结果 SHALL使用显式版本的规范`authn_id`编码user、client-or-credential、delegation、auth method、authority version与profile，并 SHALL能从PostgreSQL `system_user`无歧义解析。Identity MUST NOT包含JWT、API key、显示名称或key prefix；非法、超长或未知版本 MUST拒绝而不是截断。

#### Scenario: Identity往返
- **WHEN** 合法token完成认证
- **THEN** `authn_id -> system_user -> decoded identity` SHALL无损保留归因字段且不包含secret

### Requirement: Production artifact不得包含测试能力
Prerelease与stable module SHALL只启用production features，并 MUST排除gate callbacks、test fixtures、内置JWKS/key/token、probe symbols和fallback路径。Validator只在连接认证时检查token，不声称token过期或credential撤销会终止已建立backend。

#### Scenario: 检查production module
- **WHEN** CI检查最终module并在真实PG18 image中加载它
- **THEN** artifact SHALL只包含正式validator能力，任何测试symbol、private material或fixture命中 SHALL阻止发布

### Requirement: 部署支持必须跟随PG18最新稳定minor
每次CI与SemVer release构建 SHALL解析并使用PG18 major内当前最新稳定development/runtime，且构建、ABI测试、真实PostgreSQL测试和最终image MUST使用该次run记录的同一minor。Release材料 SHALL记录实际Rust、pgrx、PostgreSQL、header、module和runtime版本/digest，但源码与测试 MUST NOT预先批准某个PG18 minor或对应hash。

#### Scenario: PG18发布新稳定minor
- **WHEN** 浮动PG18通道解析到比上一release更新的稳定minor
- **THEN** CI SHALL基于新header重新生成ABI并运行完整真实测试，只有显式的新SemVer tag通过后才发布对应mtmpg version

#### Scenario: 消费者尝试其他major
- **WHEN** artifact被计划用于PG17、PG19或其他未显式支持的major
- **THEN** 发布/消费门禁 SHALL拒绝，并要求mtmpg增加对应feature、路径、ABI和真实运行验证
