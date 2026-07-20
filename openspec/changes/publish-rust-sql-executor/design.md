## Context

`pggomtm v0.2.0`已经发布并以PostgreSQL 18 server-side validator离线验证contract v2 JWT。Gomtmui需要一个私网runtime把`DelegatedPrincipal + statement + binds + intent`转换为短期JWT和真实OAuth backend；其`pg`驱动没有OAUTHBEARER token provider，常规Rust PostgreSQL驱动也没有该机制。PostgreSQL 18 libpq提供`PQsetAuthDataHook`、`PQAUTHDATA_OAUTH_BEARER_TOKEN`、extended query和异步cancel API，但auth hook是进程级回调，必须额外证明并发连接不会串token。

当前mtmpg规则、Cargo package和workflow只允许validator module、一个Dockerfile和一个SemVer image。Gomtmui第一轮实现的Go executor未发布且已被最新评论否决；该代码不能迁移为Rust完成证据。新executor必须从测试先行重新实现，并通过mtmpg只允许远端GitHub Actions重计算的现有边界取得证据。

## Goals / Non-Goals

**Goals:**

- 在mtmpg中建立职责隔离但共享contract权威的Rust executor package。
- 以PostgreSQL 18本次解析的`libpq-fe.h`生成最小client bindings，并通过每`PGconn*`一次性token registry安全注入30秒JWT。
- 实现HMAC/TLS私网服务、单extended-protocol statement、事务、预算、取消、结构化结果和脱敏审计。
- 由标准远端CI验证不同principal并发连接和真实PG18全矩阵，并发布独立versioned executor image与供应链材料。
- 保持`pggomtm v0.2.0`、validator ABI/runtime config、离线验证和既有Release不可变。

**Non-Goals:**

- 不提供公开database-token endpoint、通用JWT signer、直接用户PostgreSQL客户端或外部OAuth flow。
- 不提供`statements[]`、SQL脚本、跨请求transaction/session/pool、SQL parser、AST授权器、`SET ROLE`或SCRAM fallback。
- 不在gomtmui复制Rust/Go源码、Cargo工具链、native测试、Docker build或本地image fallback。
- 不把executor编进PostgreSQL image、validator dynamic library、Worker或Next.js bundle。
- 不修改production Supabase、production MCP、sub2api或llm-gateway。

## Decisions

### 1. 一个workspace包含两个独立产品package

根Cargo package继续命名`pggomtm`并保持validator版本线；新增`executor/` package，历史初始版本为`0.1.0`，当前前向修复版本为`0.1.1`。Workspace只包含这两个产品package并共享一次解析的`Cargo.lock`。Validator中与pgrx无关的profile、role、claims schema和identity输入约束抽成纯Rust contract模块；executor以关闭validator默认feature的path dependency复用该模块，不能复制claim struct、profile闭集或role映射。

Validator的pgrx、server OAuth ABI与runtime config保持在根package并只由`pg18` feature构建。Executor的HTTP、TLS、HMAC、signer和libpq client依赖只存在于executor package，不能进入`pggomtm.so`。相比把binary塞进validator package，该布局允许独立版本和依赖；相比两个无workspace crate，它避免第二lockfile和第二contract实现。

### 2. Validator与executor使用独立不可变发布身份

现有`v<semver>` tag、`ghcr.io/codeh007/mtmpg:<semver>`和`latest`只属于validator，`v0.2.0`不移动、不重建。Executor使用`executor-v<semver>` annotated tag、`ghcr.io/codeh007/mtmpg-executor:<semver>`和独立GitHub Release。`executor-v0.1.0`已经是不可变但无附件的失败历史；修复后的当前稳定消费身份是`executor-v0.1.1`/`0.1.1`。Executor stable MAY维护自己的`latest`，但gomtmui只消费明确SemVer。

0.1.1发布必须使用精确的main GREEN SHA。只读CI先验证tag、package version、共享lockfile、PG18/libpq输入和final image，再在draft Release中上传manifest、checksums、lockfile、resolved inputs、SBOM、provenance与attestation；全部附件核验通过后才将同一个Release发布并冻结。已冻结的Release不再接受后续上传，因此先正式发布、后上传附件的顺序属于发布错误，必须通过递增patch前向修复。

PR/main运行同一只读CI并验证两个package。Release workflow向CI传入明确product，CI只物化该product已经验证的OCI archive；publish job不重新Cargo resolve、build或Docker build。每个product的manifest、SBOM、provenance和attestation包含source、共享lockfile、实际PG18/libpq、image digest和对应package version。

### 3. libpq client ABI从当前PG18 header生成

Executor build从CI解析的PostgreSQL 18 `pg_config --includedir/libpq-fe.h`使用bindgen生成最小allowlist，只包含连接、auth hook、extended query、result、transaction状态、socket poll与async cancel所需类型/函数/常量。官方C compiler验证`PGauthData`、`PGoauthBearerRequest`与回调签名；源码不提交手写struct、magic常量或固定minor bindings。Builder通过`pkg-config`链接本次PG18 libpq，final image安装匹配PG18通道的最小libpq runtime与CA证书。

直接生成bindings优于依赖可能尚未暴露PG18 auth-data API的通用driver；所有unsafe集中在一个窄适配模块，其上层只暴露所有权明确的连接、query、cancel和result类型。

### 4. 全局auth hook只按当前PGconn一次性取token

进程启动时只注册一次auth-data hook。每个请求签发JWT后调用`PQconnectStartParams`取得`PGconn*`，再在首次`PQconnectPoll`前把zeroizing token登记到并发安全registry。Hook只处理`PQAUTHDATA_OAUTH_BEARER_TOKEN`，以当前`PGconn*`原子移除对应entry，把token所有权放入`PGoauthBearerRequest.user/token`，并安装cleanup callback；未知连接、重复hook、错误type、NULL或panic全部返回失败。

连接成功、失败、deadline、取消和`PQfinish`路径都有单一RAII owner，负责从registry移除残留entry并清零token。Registry不按thread、user、role或全局“current token”取值。真实PG18门禁同时启动多个不同user/profile/token连接，并验证各自`system_user`；任何token串接都必须让测试失败。该行为是发布阻断项，不能以串行测试替代。

### 5. HTTPS/HMAC边界保持严格versioned schema

Executor使用成熟Rust HTTP/TLS runtime提供唯一versioned HTTPS path。请求在JSON解析前限制body大小，并验证version、method、固定path、Unix timestamp、128-bit nonce和原始body SHA-256的HMAC-SHA256；签名使用constant-time比较，nonce进入有界30秒TTL replay store。认证失败返回统一未授权响应。

Strict request只含`DelegatedPrincipal`、一个statement、结构化binds、`read|change` intent、change confirmation与server接受的correlation ID。OAuth principal必须携带有效credential expiry；API-key principal可用`null`表示永不过期，wire保持`null`且不使用时间戳哨兵。未知字段、外部credential、database JWT、role、issuer、audience、claims、connection string或`statements[]`整体拒绝。初版只允许运行一个instance；扩容前必须把nonce store替换为共享原子authority。

### 6. 每请求一个libpq连接和一个服务端事务

合法请求取得并发semaphore后在专用blocking task中拥有一个`PGconn`。连接固定`host=postgres`、`dbname=gomtm`、profile同名user、`require_auth=oauth`、TLS `verify-full`和受控CA，不提供password或备用host。Connection不进入pool，不跨请求复用。

执行路径用`PQsendQueryParams` extended protocol，即使无bind也不调用simple query。Executor先开启service-owned transaction并设置local statement/lock/idle-in-transaction预算；`read`设置read-only，`change`要求本次明确确认。用户statement完成并在预算内缓冲结果后才commit；所有其他路径rollback或关闭连接。PostgreSQL负责拒绝多个顶层statement并裁决ACL/RLS/routine/constraint，executor不切分SQL也不复制授权。

### 7. 取消由同一connection task驱动

Blocking task以libpq socket与短poll interval推进connect/query，同时读取deadline/cancellation flag。取消或预算超限时由该owner创建`PGcancelConn`并运行PG18 async/blocking cancel API，随后消费或关闭结果、rollback并`PQfinish`。HTTP future drop guard只发送取消信号，不跨线程直接操作`PGconn`。任务必须在总deadline内结束，不能detach到后台。

### 8. 预算、响应和审计均在commit前收敛

Versioned常量限制request body、statement bytes、bind count/单值、connection、lock、statement、transaction、rows、serialized bytes、单值和总deadline。初始输出上限固定1000 rows、1 MiB serialized result和256 KiB single value；其他输入与时间预算在实现测试中以公开常量固化并由超限矩阵覆盖。

成功响应只包含columns、有限rows、command tag、affected rows、duration和correlation ID。失败只返回稳定类别、允许的SQLSTATE class、脱敏消息和correlation ID。审计可记录principal IDs、profile、intent、query fingerprint、阶段、SQLSTATE、command tag、affected rows和duration，但不得记录HMAC secret、Bearer、API key、database JWT、private key、connection string、完整SQL、bind、结果或内部堆栈。

### 9. Executor image是最小非root独立runtime

Executor image定义与validator根Dockerfile分开归属executor package，只复制release binary、MIT license、匹配libpq runtime与CA material，并以固定非root UID/GID运行。Image不包含Rust/C compiler、Cargo、source、test fixture、JWT/JWKS/private key、PGDATA或PostgreSQL server。TLS证书、HMAC和signer private key只由运行时只读mount提供。

CI在构建前运行Rust领域、FFI layout、真实PG18、并发token、TLS/HMAC、extended protocol、rollback、budget与cancel门禁；final-image只验证非root启动、HTTPS readiness、动态依赖和最小真实allow/deny路径，不复制完整领域矩阵。

## Risks / Trade-offs

- [libpq auth hook全局可变] -> 启动时只注册一次，以`PGconn*`原子一次性registry隔离；并发不同主体真实PG18门禁阻止发布。
- [Raw FFI可能产生所有权错误] -> bindings来自当前header，unsafe集中在单一模块，RAII/zeroize覆盖全部退出路径，并启用Clippy、sanitizer可用门禁和并发压力测试。
- [Blocking libpq占用线程] -> semaphore限制并发，每请求一个短寿命blocking task；deadline/cancel在同一owner内推进，不引入共享pool。
- [Workspace扩大validator依赖解析面] -> executor依赖只属于自身package，validator library依赖图和final image门禁继续证明不包含HTTP/libpq client runtime。
- [两个release line增加供应链分支] -> 使用明确product输入、独立tag namespace/image/manifest，复用一次CI定义且publish不重建。
- [单实例nonce store不能水平扩展] -> Compose和activation gate固定replica=1；扩容作为后续change并要求共享原子nonce store。

## Migration Plan

1. 更新OpenSpec、AGENTS、README和Cargo workspace边界；先提交只包含Rust RED测试与最小fixture的精确main SHA，远端CI必须因executor尚未实现而产生预期失败。
2. 抽取pgrx无关的contract模块并保持validator完整GREEN，再实现HMAC/protocol/issuer与libpq FFI单元边界。
3. 实现真实PG18并发OAuth、extended protocol、transaction、budget和cancel harness，取得精确main GREEN run。
4. 增加executor image与统一CI/release product分派，验证validator image与既有v0.2.0未变化，并取得final-image GREEN。
5. 保留`executor-v0.1.0`的不可变失败历史，修复workflow的draft发布顺序并将executor package递增到`0.1.1`。
6. 在精确main GREEN SHA创建annotated `executor-v0.1.1` tag，由标准workflow一次发布`0.1.1` image、Release和供应链材料；匿名核对tag、source、digest、SBOM、provenance与attestation，并核对validator `v0.2.0`身份未变化。
7. Gomtmui只在resolved identity与全部platform gate通过后选择该executor image；失败时保持SQL tool未注册。

Rollback不修改或删除已发布validator/executor身份。Candidate只需取消SQL tool注册并停止executor；若executor release不可消费，则修复源码并发布更高patch，不移动旧tag或回退到Go/Worker signer/SCRAM路径。

## Open Questions

当前没有阻塞实现的产品问题。具体HTTP/TLS crate选择以Rust stable兼容性、无不必要功能和远端CI行为为准；它不得改变固定wire、HMAC、token、libpq或发布契约。
