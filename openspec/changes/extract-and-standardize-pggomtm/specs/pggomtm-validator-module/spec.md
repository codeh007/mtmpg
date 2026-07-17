## ADDED Requirements

### Requirement: pggomtm必须是独立的PostgreSQL OAuth validator shared module
`pggomtm` SHALL 构建为由PostgreSQL `oauth_validator_libraries`加载的Rust `cdylib`，并 SHALL 导出有效`PG_MODULE_MAGIC`与`_PG_oauth_validator_module_init`。它 MUST NOT要求control文件、versioned extension SQL、`CREATE EXTENSION`或`cargo pgrx install/package`才能运行。

#### Scenario: PostgreSQL按OAuth validator加载模块
- **WHEN** 已验证PG18 runtime把`pggomtm.so`放入真实`pg_config --pkglibdir`并配置`oauth_validator_libraries='pggomtm'`
- **THEN** server SHALL 通过module magic、OAuth magic和callback table加载模块，且无需安装任何SQL extension对象

#### Scenario: 缺少或损坏模块入口
- **WHEN** artifact缺少module magic、validator init symbol、正确OAuth magic或required callback
- **THEN** PostgreSQL SHALL 在接受OAuth连接前fail closed，且不得尝试第二validator或认证fallback

### Requirement: OAuth ABI必须由目标PostgreSQL官方header生成
构建 SHALL通过目标`pg_config --includedir-server/libpq/oauth.h`生成只包含OAuth magic、state/result/callback类型的allowlisted Rust bindings，并 SHALL用官方C compiler执行layout probe。Bindings SHALL在禁用外部formatter后单次materialize；被校验的精确字节 MUST原样写入`OUT_DIR`并成为编译输入，校验后 MUST NOT再次调用formatter、subprocess或二次序列化。`RUSTFMT`、`PATH/rustfmt`和其他ambient formatter MUST NOT被执行或改变最终字节。手写Rust struct、复制magic常量或其他生成结果 MUST NOT成为独立ABI权威。

#### Scenario: Header与Rust layout一致
- **WHEN** 构建使用批准的PG18 server-development headers
- **THEN** 生成bindings、C size/offset probe、header digest、最终`OUT_DIR`字节digest与callback调用布局 SHALL全部一致后才允许产生artifact

#### Scenario: Header缺失或ABI变化
- **WHEN** `oauth.h`缺失、digest未获批准、allowlisted符号不存在或C/Rust layout不一致
- **THEN** 构建 SHALL 失败且不得回退到仓库内手写ABI声明

#### Scenario: 恶意RUSTFMT尝试改写已校验magic
- **WHEN** 构建环境把`RUSTFMT`指向恶意formatter或把恶意`rustfmt`放到`PATH`首位，并尝试在校验后改写OAuth magic
- **THEN** formatter SHALL不被执行或最终`OUT_DIR`字节 SHALL保持与已校验字节完全一致，否则构建与发布门禁必须失败

#### Scenario: 校验后发生二次转换
- **WHEN** 生成链尝试用独立写文件API、formatter或其他后处理重新序列化已校验bindings
- **THEN** provenance门禁 SHALL拒绝该链路，且不得仅凭header digest或内存字符串校验声明最终编译输入可信

### Requirement: pgrx必须只承担已验证的PostgreSQL FFI安全职责
crate SHALL 直接使用固定完整`pgrx`的PG18 feature提供module magic、guard、PostgreSQL error和allocator接口，并 SHALL 通过`pgrx::pg_sys`访问所需raw symbol。没有源码直接使用的`pgrx-pg-sys`依赖 MUST被移除；任何移除完整pgrx的重构 MUST先证明panic、PostgreSQL ERROR、allocator和真实loader矩阵不退化。

#### Scenario: Callback返回authenticated identity
- **WHEN** validator授权一个合法token并返回`authn_id`
- **THEN** identity SHALL 使用PostgreSQL allocator分配，且panic或PostgreSQL ERROR不得越过FFI边界造成未定义行为

#### Scenario: Rust callback发生panic
- **WHEN** 测试gate在startup、validate或shutdown边界触发panic
- **THEN** module SHALL 把它转换为稳定内部失败并保持`authorized=false`、`authn_id=NULL`

### Requirement: Build minor与runtime stable-line兼容必须分责
每个artifact SHALL 精确记录并验证其Rust、pgrx、PostgreSQL source/header minor、runtime base digest、target和libc。Runtime SHALL 只接受PostgreSQL 18 major并依赖有效`PG_MODULE_MAGIC`与OAuth validator magic，MUST NOT以`sversion == 180004`或其他单一minor等式阻断同一PG18 stable line；消费者仍 MUST只部署manifest明确验证且按digest固定的minor变体。

#### Scenario: 在构建minor上运行
- **WHEN** artifact在manifest记录的精确PG18 minor和runtime base上加载
- **THEN** loader、callback、OAuth allow/deny和ABI矩阵 SHALL 全部通过

#### Scenario: 消费者尝试未验证minor
- **WHEN** 仅标记为PG18.4验证的artifact被计划用于PG18.5
- **THEN** 发布/消费门禁 SHALL 要求先由mtmpg对18.5重新构建和真实验证并发布新digest，而不是依赖runtime精确minor崩溃作为部署策略

#### Scenario: PostgreSQL major不匹配
- **WHEN** module被放入PG17、PG19或module magic不兼容的server
- **THEN** loader或startup SHALL fail closed且不得声称跨major兼容

### Requirement: Validator必须从只读本地材料建立离线验证snapshot
每个新OAuth backend SHALL 在validator startup从版本化只读config与public JWKS建立不可变验证snapshot，检查文件权限、大小、schema、唯一issuer、唯一database audience、public key数量和类型。Module MUST NOT读取signing private key、API key、OAuth bearer存储、连接串或其他secret，也 MUST NOT执行HTTP、DNS、SQL、SPI或在线introspection。

#### Scenario: 合法config与JWKS启动
- **WHEN** 新backend读取原子发布且权限正确的config和public JWKS
- **THEN** startup SHALL 建立只含public verifier与固定policy的snapshot，并在本次认证期间不再访问外部状态

#### Scenario: 材料缺失或损坏
- **WHEN** config/JWKS缺失、过大、权限不安全、schema错误、kid重复、包含private key或资源配置非法
- **THEN** startup SHALL fail closed且不得使用内置key、旧缓存、Web endpoint或备用issuer

#### Scenario: 原子轮换public JWKS
- **WHEN** 平台以原子替换发布包含active与retiring public key的新JWKS
- **THEN** 后续新backend SHALL 读取完整新snapshot，既有backend无需reload或重新认证，且不存在读取半写文件的状态

### Requirement: Database JWT必须按严格闭集验证
Validator SHALL 只接受固定ES256、稳定`kid`、唯一issuer/audience、database scope和30至300秒TTL的database JWT，并 SHALL 验证完整claims schema、`sub`、`jti`、`delegation_id`、`auth_method`、`authority_version`、`db_profile`、`db_role`以及恰好一个`client_id`或`credential_id`。未知字段、算法、资源、actor组合、时间或ID格式 MUST fail closed。

#### Scenario: OAuth client token有效
- **WHEN** 唯一issuer签发未过期、claims完整且只含`client_id`的OAuth database JWT
- **THEN** validator SHALL 验证签名和全部policy后生成保留user、client、delegation、method、authority version与profile的identity

#### Scenario: API-key-derived token有效
- **WHEN** 唯一issuer签发未过期、claims完整且只含`credential_id`的API-key-derived database JWT
- **THEN** validator SHALL 使用同一验证路径并生成保留credential归因且不包含API key/prefix的identity

#### Scenario: 外部凭据直达PostgreSQL
- **WHEN** client提交MCP OAuth access token、Supabase JWT、长期API key、opaque token或其他issuer token
- **THEN** validator SHALL 拒绝且不得尝试在线查询或另一认证器

### Requirement: Signed profile与requested role必须精确绑定
`db_profile` SHALL 只映射到版本化contract声明的closed PostgreSQL role集合，token中的`db_role` MUST与startup requested role和profile映射精确相等。Runtime config MUST NOT允许部署者添加任意role、算法、issuer fallback或越权profile映射。

#### Scenario: Ordinary请求ordinary role
- **WHEN** 合法ordinary token请求contract指定的ordinary role
- **THEN** validator SHALL 授权该role并返回ordinary identity

#### Scenario: Token请求更高或未知role
- **WHEN** ordinary token请求admin/developer/service/migration/cluster role，或token声明未知profile
- **THEN** validator SHALL 拒绝且不得依赖后续RLS、pg_ident或`SET ROLE`修正

### Requirement: Authenticated identity必须版本化、有界且无secret
授权结果 SHALL 使用显式版本的规范`authn_id`编码user、client-or-credential、delegation、auth method、authority version与profile，并 SHALL 能从PostgreSQL `system_user`无歧义解析。Identity MUST NOT包含JWT、API key、显示名称、key prefix或可注入分隔符，超长或未知版本 MUST拒绝而不是截断或散列降级。

#### Scenario: Identity往返成功
- **WHEN** 合法OAuth或API-key-derived token完成认证
- **THEN** `authn_id -> system_user -> decoded identity` SHALL 无损保留稳定归因字段且不包含secret

#### Scenario: Identity无法规范编码
- **WHEN** 任一ID超长、字符非法、字段组合矛盾或版本未知
- **THEN** validator SHALL 拒绝连接并只报告稳定脱敏reason类别

### Requirement: 正式artifact不得包含测试gate或活动撤销声明
Stable artifact MUST以无`abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate`的production feature构建，并 MUST扫描排除内置测试JWKS、signing key、gate token、probe module和fallback路径。Validator SHALL只在连接认证时检查JWT，MUST NOT声称token到期或credential撤销会终止已建立backend。

#### Scenario: 检查stable image
- **WHEN** release workflow检查最终filesystem、ELF symbol/string和SBOM
- **THEN** image SHALL 只包含正式module与公开build metadata，不包含gate能力、测试key、Rust target或secret

#### Scenario: Credential撤销时已有backend
- **WHEN** 外部delegation在一个直接PostgreSQL backend建立后被撤销
- **THEN** module MAY让该backend继续存在，文档 SHALL 明确撤销只阻止后续签发和新连接
