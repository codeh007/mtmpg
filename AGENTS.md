# pggomtm 仓库级 Agent 规则

本文件约束在本仓库工作的自动化开发 Agent。Agent 必须先理解 OpenSpec、PostgreSQL OAuth 应用程序二进制接口（ABI）与当前 fail-closed 原型边界，再修改源码、测试、构建或文档。

## 适用范围与权威

本文件适用于整个仓库。仓库保持单一 Rust crate、单一实现和单一构建权威：

- `Cargo.toml`、`Cargo.lock` 与 `rust-toolchain.toml` 定义 crate、依赖和工具链。
- `src/lib.rs` 承载 PostgreSQL module 入口、OAuth callback 与原生边界。
- `src/database_auth.rs` 承载离线 JSON Web Token（JWT）、JSON Web Key Set（JWKS）、role 与 identity 领域逻辑。
- `tests/` 承载 Rust 测试、官方 C layout probe 与真实 runtime probe。
- `Dockerfile` 组装精确 PostgreSQL 18.4 环境并运行完整门禁。
- `openspec/` 记录需求、设计、delta spec 与 task 状态。
- `docs/evidence/` 只保存可复验且不含 secret 的历史证据。

不要引入 Cargo workspace、嵌套 crate、消费仓库源码副本或第二套 Docker 构建定义。

## 修改前读取契约

Agent 必须按当前任务范围核对现有契约：

1. 读取关联 GitHub Issue 和 active OpenSpec proposal、design、spec 与 tasks。
2. 读取 `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml` 和 `Dockerfile`。
3. 追踪 `_PG_oauth_validator_module_init`、startup、validate 与 shutdown 调用边界。
4. 阅读相关 Rust、C、SQL 测试和既有 evidence。
5. 明确当前行为与未来目标，不能把未完成的 OpenSpec task 写成现有能力。

当前无 gate 最终制品是拒绝所有 token 的 fail-closed 原型。它不是 production-ready，也没有稳定发布版或已发布开放容器计划（OCI）摘要。

## 保持完整 pgrx 与官方 OAuth 边界

完整 `pgrx` 是 PostgreSQL 集成边界。Agent 必须保留 `pgrx` 提供的 module magic、guard、PostgreSQL error 与 allocator 语义，不能用自行实现的替代层绕开这些边界。

PostgreSQL OAuth ABI 的目标权威是目标 `pg_config --includedir-server` 下的 `libpq/oauth.h`。当前 pgrx 0.19.1 的 PostgreSQL 18 bindgen 输入没有完整覆盖该 header，因此不得把缺失的 pgrx 生成类型误当成 OAuth ABI 权威。

涉及 OAuth struct、magic、callback 或 layout 时，Agent 必须以精确官方 header、C probe 和受控生成结果交叉验证。不要新增第二套手写声明；生成文件只能通过仓库提供的生成命令更新，不得手写。

该模块由 `oauth_validator_libraries` 加载，不是 SQL extension。不要新增 control 文件、versioned extension SQL、`CREATE EXTENSION` 流程，也不要把 `cargo pgrx install/package` 作为生产交付方式。

## 保护认证与生产数据

源码、测试、Git history、日志、image 和 evidence 都不得包含以下数据：

- signing private key、API key、OAuth token、database JWT 或 authorization code
- 数据库连接串、`.env`、credential、session、PostgreSQL data 或真实 JWKS working copy
- 生产配置、用户身份数据或未脱敏请求日志

测试只能使用确定性合成 fixture，并通过 feature 把 fixture 限定在测试 gate。除非任务明确授权，否则对生产数据库、配置和部署只允许只读检查；任何输出都必须脱敏。

认证失败必须 fail closed。不要增加备用 issuer、旧 verifier、网络 fetch、SQL/SPI 查询、宽松 claims 解析或其他 fallback。

## 测试与 Docker 门禁

行为修复和功能变更先增加能证明缺口的失败测试，再实现最小根因修复。纯文档变更不适用测试驱动开发（TDD），应运行文档聚焦检查。

Agent 必须按改动风险运行门禁：

- 先运行相关 locked Rust test、C probe 或 SQL runtime probe。
- 运行 `cargo fmt --check` 和对应 feature 组合的 Clippy `-D warnings`。
- 原生 ABI、pgrx、PostgreSQL minor 或 artifact 变化必须运行 Docker clean build。
- 交付前运行完整 Docker 门禁、secret 扫描和 `git diff --check`。

测试失败时先定位根因。不要删除测试、弱化断言、降低 lint、关闭 warning 或扩大兼容分支来获得通过结果。

## 文档与 OpenSpec

所有正文、分析、计划和仓库文档使用中文。完整且未改写的标准 MIT 英文法律文本是唯一例外；代码、命令、标识符和上游专有名称保留原文。

文档必须区分已验证事实与未来目标。三项以上内容使用列表，步骤使用有序列表，代码围栏标注语言，相对链接必须存在。

OpenSpec 是需求范围的权威来源：

- 行为变化先更新或确认对应 proposal、design 与 delta spec。
- 实现和验证完成后才能更新 task checkbox。
- 不手写生成文件，不增加备用实现，不用包装函数掩盖根因。
- 每次提交只包含当前 task 要求的源码、测试、文档与 evidence。

## GitHub Issue 与交付

任务来自 GitHub Issue 时，Agent 先使用 `gh` 阅读 Issue。实施期间保留可复验的 commit、命令、结果和限制，等该 Issue 涉及的仓库工作全部结束后再统一回填。

Agent 不得执行 `force push`、覆盖他人改动、修改无关文件或把 ignored 构建产物加入 Git。交付前检查 tracked diff，确认没有 secret、运行数据、重复源码或超出 OpenSpec 的声明。
