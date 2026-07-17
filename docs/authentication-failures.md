# 认证失败reason-code与可见性契约

本页定义`pggomtm`认证失败的版本化reason-code、PostgreSQL服务端日志级别和客户端
可见性。Reason-code只用于服务端诊断与自动化分类，不是授权结果，也不能被客户端
用作放宽或fallback依据。

## 使用闭集`pggomtm-auth/v1`

所有module自有reason-code都以`pggomtm-auth/v1/`开头。V1闭集如下：

| Reason-code | 类别 |
| --- | --- |
| `config-missing` | Validator config缺失 |
| `jwks-missing` | Public JWKS缺失 |
| `config-too-large` | Config超过大小上限 |
| `jwks-too-large` | JWKS超过大小上限 |
| `material-file-type-unsafe` | 材料不是批准的普通文件 |
| `material-permissions-unsafe` | 材料权限不安全 |
| `material-publication-layout-unsafe` | 材料不满足原子发布布局 |
| `config-invalid` | Config schema或内容无效 |
| `resources-invalid` | Issuer或audience资源无效 |
| `jwks-invalid` | JWKS schema、key或public参数无效 |
| `jwks-duplicate-kid` | JWKS含重复`kid` |
| `token-policy-invalid` | 内部token policy无效 |
| `token-invalid` | Compact token无法按契约解析 |
| `token-header-invalid` | JOSE header无效 |
| `token-kid-unknown` | `kid`不在当前snapshot |
| `token-signature-invalid` | ES256签名无效 |
| `token-claims-invalid` | Claims、资源、时间或actor组合无效 |
| `token-role-mismatch` | Requested role与signed role不一致 |
| `identity-invalid` | Identity字段或规范编码无效 |
| `callback-input-invalid` | Callback收到无效指针或文本输入 |
| `callback-state-invalid` | Callback state未初始化或不一致 |
| `postgres-major-unsupported` | PostgreSQL major不是18 |
| `internal-panic` | Rust内部panic被边界捕获 |
| `postgres-error` | PostgreSQL ERROR由FFI边界处理 |

完整字符串由`AuthenticationFailureReason::code()`唯一生成，例如
`pggomtm-auth/v1/token-signature-invalid`。同一V1 module不得改名、复用或动态扩展这些
字符串；需要改变分类语义时必须升级reason-code版本与module contract。

## 区分服务端与客户端可见性

| 失败类型 | Module/PostgreSQL服务端级别 | 客户端可见内容 |
| --- | --- | --- |
| Token、role或identity拒绝 | `LOG`，`pggomtm authentication rejected: reason=<code>` | PostgreSQL通用OAuth认证失败 |
| Startup config/JWKS错误 | `ERROR`，`pggomtm authentication failed: reason=<code>` | PostgreSQL通用OAuth认证失败 |
| 捕获的内部panic | `LOG`，`internal-panic` | PostgreSQL通用OAuth认证失败 |
| PostgreSQL ERROR | `ERROR`，`postgres-error` | 当前认证失败，不暴露module诊断code |

`LOG`用于预期的单次认证拒绝，默认只进入服务端日志。客户端不得看到
`pggomtm-auth/`、`reason=`、config/JWKS诊断或Rust内部信息。PostgreSQL拥有客户端通用
错误文案；调用方应按认证失败处理，不能解析自然语言来推断签名、`kid`、claims或role
细节。

## 禁止记录认证材料

任何失败类别都只能记录固定reason-code。Module、test client与workflow不得记录：

- 完整或部分JWT、OAuth bearer、API key、authorization code；
- JWKS内容、config原文、私钥、key coordinate或完整`kid`输入；
- connection string、数据库credential、session或真实identity材料；
- Rust文件路径、`panicked at`、完整stack/backtrace或动态底层错误文本。

服务端可按reason-code聚合计数，但不得加入token hash、用户输入、actor ID、role原文或
其他高基数字段。发生内部错误时保持fail closed；不得因为日志不可用而切换validator、
issuer或认证方式。
