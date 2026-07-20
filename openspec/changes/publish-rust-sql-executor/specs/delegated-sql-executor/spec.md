## ADDED Requirements

### Requirement: Executor必须只接受HMAC认证的私网HTTPS请求
Executor SHALL只在固定versioned HTTPS path接受请求，并 SHALL在JSON解析和业务处理前限制body大小、验证method/path/version、Unix timestamp、128-bit nonce、原始body digest与HMAC-SHA256。签名比较 MUST constant-time，timestamp窗口 MUST固定30秒，nonce MUST进入有界TTL replay store；任何缺失、篡改、过期或重放 MUST返回同一未授权结果且不得签发JWT或连接PostgreSQL。

#### Scenario: 合法私网请求
- **WHEN** gomtmui Worker向固定path发送时间窗内、nonce未使用且HMAC覆盖完整原始body的请求
- **THEN** executor SHALL继续严格schema验证且不得把私网可达性当作认证替代

#### Scenario: 请求被篡改或重放
- **WHEN** method、path、version、timestamp、nonce或body任一变化、过期或重复使用
- **THEN** executor SHALL返回统一未授权结果，且日志不得暴露签名、principal或字段差异

### Requirement: Executor请求必须是无原始credential的严格单statement schema
请求 SHALL只包含严格`DelegatedPrincipal`、一个非空`statement`、结构化`binds`、`read|change` intent、change confirmation和correlation ID。Principal SHALL包含user、恰好一个client或credential、delegation、auth method、authority version、database scope、profile和可空credential expiry；OAuth expiry MUST为有效Unix时间，API key MAY以`null`表示不设过期时间。未知字段、`statements[]`、外部Bearer、API key、database JWT、password、connection string、role、issuer、audience、claims或expiry覆盖 MUST拒绝整个请求，且不得使用超大时间戳哨兵替代`null`。

#### Scenario: OAuth和API-key principal进入同一路径
- **WHEN** 两类principal分别满足同一strict schema
- **THEN** executor SHALL使用同一issuer、libpq和statement路径，只按client/credential身份分支归因

#### Scenario: API key不设过期时间
- **WHEN** API-key principal的credential expiry为`null`
- **THEN** strict decoder SHALL接受该principal并保留`null`，而OAuth principal缺少expiry MUST拒绝

#### Scenario: 调用方提交role或token
- **WHEN** 请求包含role覆盖、claims对象、外部credential、database JWT或第二statement
- **THEN** strict decoder SHALL拒绝且不得忽略字段、选择首项或调用issuer

### Requirement: Database JWT必须由executor内唯一issuer签发
Executor SHALL只在HMAC、principal、scope、profile、intent和适用的credential剩余有效期全部通过后签发JWT。OAuth及设置了expiry的API key MUST覆盖完整30秒TTL；未设置expiry的API key不执行该时间比较。Token SHALL固定使用ES256、active `kid`、唯一issuer/audience、`database` scope和精确30秒TTL，并 SHALL包含`sub`、`iat`、`exp`、`jti`、`delegation_id`、`auth_method`、`authority_version`、`db_profile`、`db_role`以及恰好一个`client_id`或`credential_id`。Profile和role SHALL只允许完全同名的`ordinary`、`business_admin`和`database_developer`；调用方不得覆盖任何claim。

#### Scenario: Credential剩余时间充足
- **WHEN** 合法principal的credential expiry至少覆盖完整30秒TTL
- **THEN** issuer SHALL按共享contract生成一个短期JWT且profile、role和startup user完全同名

#### Scenario: Credential即将过期
- **WHEN** credential剩余有效期不足30秒
- **THEN** executor SHALL拒绝而不得截断、延长TTL或使用其他issuer

#### Scenario: API key永不过期
- **WHEN** API-key principal的credential expiry为`null`
- **THEN** issuer SHALL签发固定30秒database JWT，且不得把`null`改写为哨兵时间戳

### Requirement: Signer私钥与database JWT必须保持进程内隔离
ES256 private key SHALL只从运行时只读private mount加载并使用zeroizing内存，MUST NOT进入validator、image、source、argv、environment、日志或响应。Database JWT SHALL只从issuer传到同进程当前`PGconn*`的libpq auth-data hook；它 MUST NOT进入HTTP、Worker、MCP响应、connection string、文件或审计。连接成功、失败、取消或关闭后所有token material MUST清零。

#### Scenario: 检查service输出与mount
- **WHEN** CI和gomtmui platform扫描executor image、mount、environment、日志和响应
- **THEN** 只有运行进程可读取private key，任何private key或database JWT输出命中 SHALL阻止发布或激活

### Requirement: Libpq auth hook必须按PGconn一次性隔离token
Executor SHALL使用本次CI解析的PostgreSQL 18 `libpq-fe.h`生成最小bindings并只注册一个进程级auth-data hook。每次请求 SHALL先通过`PQconnectStartParams`取得唯一`PGconn*`，在首次poll前登记对应zeroizing token；hook SHALL只为`PQAUTHDATA_OAUTH_BEARER_TOKEN`原子移除当前`PGconn*` entry并安装cleanup。Registry MUST NOT使用thread-local current token、user、role或全局单值；未知、重复、NULL、错误type和清理后的连接 MUST fail closed。

#### Scenario: 不同principal并发连接
- **WHEN** 多个不同user/profile/token的`PGconn*`同时推进OAuth认证
- **THEN** 每个backend SHALL得到自身token并产生匹配`system_user`，任何串接或共享 SHALL使真实PG18门禁失败

#### Scenario: 连接在hook前失败
- **WHEN** TLS、socket、deadline或配置错误使连接在token取用前失败
- **THEN** RAII owner SHALL移除并清零残留entry，后续连接不得取得该token

### Requirement: OAuth连接必须固定且每请求新建
Executor SHALL为每次合法请求建立一个新的PostgreSQL 18 libpq连接，固定host `postgres`、database `gomtm`、profile同名user、`require_auth=oauth`、可信CA与TLS `verify-full`。请求结束 SHALL无条件`PQfinish`。Executor MUST NOT提供password、SCRAM、备用host/database、Hyperdrive、`SET ROLE`、跨用户pool或connection reuse。

#### Scenario: TLS与OAuth匹配
- **WHEN** `DNS:postgres`证书、JWT、startup user和database全部有效
- **THEN** libpq SHALL建立由`pggomtm`认证的真实backend并在本次请求后关闭

#### Scenario: OAuth或TLS失败
- **WHEN** hostname、chain、database、role、JWT或auth method任一不匹配
- **THEN** 连接 SHALL fail closed且不得改用SCRAM、旧SAN、其他database或共享service identity

### Requirement: 用户SQL必须只通过extended protocol执行一个顶层statement
Executor SHALL只使用`PQsendQueryParams`或`PQexecParams`提交一次用户statement，即使bind为空也 MUST NOT调用`PQsendQuery`或`PQexec` simple query。Bind SHALL以libpq参数数组传递且不得插值SQL。PostgreSQL prepared-statement parser SHALL成为多顶层命令唯一权威；executor MUST NOT使用分号切割、正则、自建parser或调用方声明判断statement数量。

#### Scenario: 多顶层命令
- **WHEN** statement包含`SELECT ...; INSERT ...`等两个顶层命令
- **THEN** PostgreSQL extended protocol SHALL在业务执行前拒绝整体且executor不得拆分或部分执行

#### Scenario: 合法内部语句
- **WHEN** 一个合法CTE、`CALL`、`DO`、routine body、literal或comment内部包含分号
- **THEN** executor SHALL原样参数化提交并由PostgreSQL解析，不得误拆分

### Requirement: Read与change必须由服务端事务强制
Executor SHALL在service-owned transaction中执行一个用户statement，并只在完整结果通过预算后commit。`read` SHALL设置PostgreSQL read-only transaction；`change` SHALL要求当前调用显式确认。Parse、bind、ACL、RLS、constraint、结果预算、timeout、取消或commit任一失败 SHALL rollback或关闭backend并丢弃全部缓冲结果。Intent与确认只能收窄请求，不得替代数据库授权。

#### Scenario: Read intent尝试写入
- **WHEN** read请求提交DML、DDL或写routine
- **THEN** PostgreSQL SHALL拒绝，executor SHALL rollback且不得返回部分结果

#### Scenario: Change结果超过预算
- **WHEN** confirmed change执行后产生超限`RETURNING`结果
- **THEN** executor SHALL在commit前取消或rollback，且数据库事务性效果不得提交

### Requirement: 所有执行必须有固定预算和可终止取消
Executor SHALL固定限制并发、request body、statement、bind count/单值、connection、lock、statement、transaction、rows、serialized bytes、single value与总deadline。输出上限 SHALL为1000 rows、1 MiB serialized result和256 KiB single value。HTTP取消、deadline或任一预算超限时，connection owner SHALL使用PG18 libpq cancel API终止query、rollback或关闭并在总deadline内结束；任务 MUST NOT detached到后台或返回部分结果。

#### Scenario: 结果超过预算
- **WHEN** row、serialized byte或single-value任一上限被超过
- **THEN** executor SHALL丢弃已缓冲内容、rollback并返回稳定budget错误

#### Scenario: 调用方取消
- **WHEN** HTTP request context在connect或query期间取消
- **THEN** cancellation SHALL到达拥有`PGconn`的task并终止backend，executor不得继续执行或异步返回结果

### Requirement: 响应与审计必须结构化且不记录敏感内容
成功响应 SHALL只在commit成功后包含columns、有限rows、command tag、affected rows、duration和correlation ID。失败 SHALL只包含稳定类别、允许的SQLSTATE class、脱敏消息和correlation ID。审计 MAY记录principal IDs、profile、intent、query fingerprint、阶段、SQLSTATE、command tag、affected rows和duration，但 MUST NOT记录HMAC secret、外部credential、database JWT、private key、connection string、完整SQL、bind、结果或内部堆栈。

#### Scenario: 数据库权限拒绝
- **WHEN** statement被ACL、RLS、constraint或read-only transaction拒绝
- **THEN** executor SHALL返回稳定脱敏错误并记录关联信息，且不得以其他role重试

#### Scenario: DML无返回行
- **WHEN** confirmed change成功执行不带`RETURNING`的DML
- **THEN** 响应 SHALL返回command tag和affected rows而不是伪造result set
