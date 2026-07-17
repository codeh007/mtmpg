# Issue #116：Production identity与system_user往返证据

本证据于2026-07-17（UTC）采集，用于完成OpenSpec任务6.4。验证对象是无
`abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate`的production module；本次没有运行
本地完整Docker构建，任务结论只来自精确远端commit的GitHub Actions。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29594695377](https://github.com/codeh007/mtmpg/actions/runs/29594695377) |
| Remote head SHA | `9247e40613d5c9f95fd9ee7b0a9f8e4d9a2d0a72` |
| Event | `push` |
| Job | `Native authority`，ID `87932123353` |
| Job时间 | `2026-07-17T16:04:17Z`至`2026-07-17T16:13:40Z`，9分23秒 |
| 结论 | `success` |
| Actions artifact | 0 |

Checkout、clean source identity、Buildx与根Docker authority graph全部成功。最终
image同时要求ABI runtime、既有PGX OAuth和本次production identity三枚sentinel，
任一stage失败都不能生成成功结果。

## 无gate真实libpq链路

`production-identity-gate`只从`build` stage复制以`pg18` feature构建的
`libpggomtm.so`，再把它作为`pggomtm_identity_gate`装入精确
`postgres:18.4-bookworm` runtime。该module从只读`/etc/pggomtm` config/public JWKS
建立snapshot，并由真实HBA OAuth、`PQsetAuthDataHook`与libpq `OAUTHBEARER`连接调用
production callback。

每个成功连接执行以下完整往返：

1. Validator验证签名token、signed profile与startup requested role；
2. callback使用PostgreSQL allocator返回规范`authn_id`；
3. PostgreSQL把该值暴露为当前session的`system_user`；
4. C client把实际`system_user`写入仅本次stage可见的`0600`临时文件；
5. Rust helper调用正式`decode_system_user()`，比较全部归因字段，再重新编码并要求与
   原始`system_user`逐字节一致。

远端日志包含下列六个不同production场景的成功标记：

| Actor | Profile | Requested/current role |
| --- | --- | --- |
| OAuth client | `ordinary` | `gomtm_candidate_ordinary` |
| OAuth client | `business-admin` | `gomtm_candidate_business_admin` |
| OAuth client | `database-developer` | `gomtm_candidate_database_developer` |
| API-key credential | `ordinary` | `gomtm_candidate_ordinary` |
| API-key credential | `business-admin` | `gomtm_candidate_business_admin` |
| API-key credential | `database-developer` | `gomtm_candidate_database_developer` |

日志计数为7次允许连接与7次往返标记，其中额外1次是既有
`pgx-oauth-gate` ordinary smoke；production矩阵本身恰好覆盖上表六项。该链路证明
allocator返回值在PostgreSQL消费并形成`system_user`时仍有效，也证明client与
credential归因、delegation、method、authority version和profile没有歧义或丢失。

## 负向identity矩阵

Production真实libpq链路另外提交两枚签名有效但identity不合法的token：

- 65字节内部ID，超过64字节字段上限；
- 含保留分隔字符的delegation ID。

两次连接都只得到通用OAuth拒绝，未建立session或产生`system_user`。正式codec还在
同一runtime stage拒绝完整形状的未知`pggomtm:v2`、超过512字节的`authn_id`与含非法
分隔字符的identity。既有Rust unit矩阵同时覆盖两类actor、三个profile以及
`authn_id -> system_user -> decoded identity`。

首次候选run `29594035154`在production矩阵执行后，因直接callback probe的临时fixture
目录仍为`0700 root`而失败。提交`9247e40`只把该测试目录及JWT交给`postgres`并把文件
收紧为`0400`，没有修改production Rust或放宽正式config/JWKS权限；上述最终run随后
完整通过。

## 日志与边界

最终run的完整日志以不回显匹配内容的方式扫描邮箱、GitHub token、
`Authorization: Bearer`、private-key header、JWT、带credential的PostgreSQL URI和
`github_event_payload`，全部类别均为缺失。临时JWT、实际`system_user`文件、helper、
PostgreSQL data与测试module都只存在于gate stage并在成功后删除；final image只从该
stage复制成功sentinel，并立即删除sentinel。

本证据不完成脱敏reason、无gate module静态扫描、cold authority、release、consumer
或stable门禁；这些仍由任务6.5、6.6及后续供应链任务分别验证。
