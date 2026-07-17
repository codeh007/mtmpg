# Issue #116：认证失败reason与脱敏可观察性证据

本证据于2026-07-17（UTC）采集，用于完成OpenSpec任务6.5。验证对象包含无gate
production callback、ABI runtime panic/PostgreSQL ERROR边界、真实libpq客户端和
PostgreSQL服务端日志。任务结论只来自精确远端commit的GitHub Actions；本次没有运行
本地完整Docker构建。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29597890311](https://github.com/codeh007/mtmpg/actions/runs/29597890311) |
| Remote head SHA | `71b0435dd2f310686bcdafb928d5c65976338174` |
| Event | `push` |
| Job | `Native authority`，ID `87942633412` |
| Job时间 | `2026-07-17T16:52:57Z`至`2026-07-17T17:02:26Z`，9分29秒 |
| 结论 | `success` |
| Actions artifact | 0 |

Checkout、clean source identity、Buildx与根Docker authority graph全部成功。最终
image继续要求ABI runtime、PGX OAuth与production identity三枚sentinel。

## 稳定reason-code闭集

`AuthenticationFailureReason`是24项`pggomtm-auth/v1/...`字符串的唯一源码权威。
远端locked Rust tests分别证明：

- 24项code与批准顺序逐字节一致且没有重复；
- 全部`JwtValidationError`都映射到闭集code；
- 全部`RuntimeConfigError`都映射到闭集code。

Module不再把`Display`、路径、JSON parser或底层I/O文本拼入认证日志。完整分类、级别与
客户端语义记录在[认证失败reason-code与可见性契约](../../authentication-failures.md)。

## 服务端级别与客户端可见性

Production stage把PostgreSQL日志写入临时独立文件，并实际验证：

| 场景 | 服务端 | libpq客户端 |
| --- | --- | --- |
| Config缺失的startup | `ERROR`与`pggomtm-auth/v1/config-missing` | 只见同一稳定startup code |
| Tampered signature | `LOG`与`pggomtm-auth/v1/token-signature-invalid` | 只见PostgreSQL通用OAuth失败 |
| 超长/非法identity | `LOG`与`pggomtm-auth/v1/identity-invalid` | 只见PostgreSQL通用OAuth失败 |
| 捕获的Rust panic | `LOG`与`pggomtm-auth/v1/internal-panic` | Callback fail closed |
| PostgreSQL ERROR | `ErrorData.elevel == ERROR`与`pggomtm-auth/v1/postgres-error` | 不含动态底层文本 |

Startup与PostgreSQL ERROR的C probe对`message`、`detail`、`detail_log`、`hint`和`context`
逐字段检查稳定code、1024字节上限及禁止模式。Token拒绝路径明确禁止客户端出现
`pggomtm-auth/`或`reason=`；startup路径只允许精确`config-missing`，仍禁止材料与内部
诊断。

成功日志包含7次真实允许连接和5次拒绝连接：既有PGX gate贡献1次允许/1次tampered
拒绝；production链贡献两类actor、三个profile的6次允许，以及startup、两种非法
identity和tampered signature共4次拒绝。所有client命令都不输出`PQerrorMessage()`原文。

## 不回显材料扫描

在删除临时文件前，Docker gate直接扫描完整PostgreSQL服务端日志并要求：

- 每一枚合成JWT都不能以完整值出现；
- `validator.json`与`jwks.json`完整内容不能出现；
- 不得出现Authorization bearer、带credential的PostgreSQL URI、private-key header；
- 不得出现`panicked at`、Rust源码路径、`RUST_BACKTRACE`或完整stack backtrace。

远端Action日志的额外不回显扫描确认邮箱、GitHub token、Authorization bearer、私钥、
JWT、带credential数据库URI和`github_event_payload`均缺失。排除Dockerfile中扫描规则
字面量及Cargo的`Running unittests src/lib.rs`固定文件名后，runtime Rust stack模式也
缺失。

首次候选run `29597100390`证明startup callback的稳定`ERROR` code会对当前失败连接
可见，而token拒绝由PostgreSQL归一化为通用OAuth失败。提交`71b0435`把两类可见性改为
两个显式测试分支，没有放宽任何材料或stack禁止规则；上述最终run随后完整通过。

本证据不完成任务6.6的无gate module级artifact扫描，也不完成cold、release、consumer
或stable供应链门禁。
