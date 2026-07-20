# SQL executor 运行契约

`mtmpg-executor`是只供受控私网调用的PostgreSQL 18 OAuth companion service。它把已经由调用方认证并授权的`DelegatedPrincipal`转换为30秒database JWT，并只通过同一进程的libpq auth-data hook把token交给当前`PGconn`。它不是公开SQL API、通用token service或直接数据库登录入口。

## 固定入口与HMAC

Service只监听配置的TLS socket，并提供两个路径：

- `GET /ready`：进程和TLS readiness。
- `POST /v1/sql/execute`：唯一SQL执行入口。

执行请求必须包含`x-executor-version`、`x-executor-timestamp`、`x-executor-nonce`和`x-executor-signature`。Version固定为`v1`；timestamp窗口为30秒；nonce是16字节的小写hex；signature是HMAC-SHA256的小写hex。Canonical input固定为：

```text
v1\nPOST\n/v1/sql/execute\n<unix-seconds>\n<nonce>\n<sha256(raw-body)>
```

HMAC secret必须是32个原始字节，只从只读文件加载。Service在JSON解析前验证body上限和HMAC，并使用constant-time比较。Nonce保存在单进程有界TTL store中；当前release只允许运行一个replica。水平扩容前必须先引入共享原子replay authority，不能用多个本地store绕过重放边界。

## Strict request

Request body只允许以下字段：

- `principal`：`user_id`、恰好一个`client_id|credential_id`、`delegation_id`、`auth_method`、`authority_version`、固定`database_scope`、`profile`和必需的`credential_expires_at`字段。API key允许该字段显式为`null`，OAuth必须是正整数时间戳；字段缺失和时间戳哨兵值均非法。
- `statement`：一个非空PostgreSQL顶层statement。
- `binds`：`null|text|int64|boolean|json`结构化参数数组。
- `intent`：`read|change`。
- `change_confirmed`：`change`必须为`true`，`read`必须为`false`。
- `correlation_id`：受限的调用方关联ID。

所有对象deny unknown fields。请求不能提交Bearer、API key、password、database JWT、connection string、issuer、audience、role、claims或`statements[]`。Profile和startup role只允许完全同名的`ordinary`、`business_admin`、`database_developer`；不存在alias、映射或阶段前缀。

## Runtime mount

Image以固定`10001:10001`身份运行。以下路径由部署平台通过只读mount提供，不能写入image、environment value或argv：

| 环境变量 | 文件或值 |
| --- | --- |
| `MTMPG_EXECUTOR_HMAC_SECRET_PATH` | 32字节HMAC secret文件 |
| `MTMPG_EXECUTOR_SIGNING_KEY_PATH` | ES256 PKCS#8 private key文件 |
| `MTMPG_EXECUTOR_POSTGRES_CA_PATH` | PostgreSQL TLS CA文件 |
| `MTMPG_EXECUTOR_TLS_CERT_PATH` | Executor HTTPS certificate文件 |
| `MTMPG_EXECUTOR_TLS_KEY_PATH` | Executor HTTPS private key文件 |
| `MTMPG_EXECUTOR_ISSUER` | 唯一database-token issuer |
| `MTMPG_EXECUTOR_AUDIENCE` | 唯一database-token audience |
| `MTMPG_EXECUTOR_KEY_ID` | active signing `kid` |
| `MTMPG_EXECUTOR_LISTEN` | 私网listen address |

Signer private key只进入executor。PostgreSQL validator只接收对应public config/JWKS投影。Database JWT不进入HTTP响应、connection string、文件或日志；连接成功、失败、取消和关闭都会清理registry与token内存。

## PostgreSQL连接与SQL

每个合法请求新建一个连接，固定使用host `postgres`、database `gomtm`、profile同名user、`sslmode=verify-full`、`require_auth=oauth`、唯一issuer和通用client ID `sql-executor`。请求不能覆盖这些参数。Service不提供password、SCRAM、备用host/database、`SET ROLE`、Hyperdrive、pool或connection reuse。

用户statement始终通过`PQsendQueryParams` extended protocol提交一次；bind不插值，executor不按分号切割或自行解析SQL。PostgreSQL负责拒绝多个顶层statement，并最终裁决ACL、RLS、routine和constraint。

`read`使用service-owned read-only transaction；`change`只接受本次明确确认。结果完全缓冲并通过预算后才commit。Parse、bind、授权、约束、预算、timeout、cancel或commit任一失败都会rollback或关闭backend，且不返回部分结果。

## 预算、取消与失败

固定上限包括256 KiB request body、64 KiB statement、64个bind、单bind 64 KiB、1000 rows、1 MiB serialized result和256 KiB single value。连接、lock、statement、transaction和总请求均有deadline；HTTP future被丢弃时只向connection owner发送cancel flag，由owner使用PG18 libpq cancel API终止query并在总deadline内结束，不保留后台task。

成功响应只含columns、rows、command tag、affected rows、duration和correlation ID。失败响应只含稳定category、允许的SQLSTATE class、固定消息和correlation ID。日志可以记录闭集阶段，但不能记录HMAC、credential、database JWT、private key、connection string、完整SQL、bind、结果、panic文本或stack。

认证、TLS、OAuth、授权或执行失败全部fail closed。任何部署回滚都应停止注册上层SQL tool并切回另一个已发布executor SemVer；不得改用本地build、旧issuer、SCRAM或第二executor实现。
