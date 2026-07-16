# pggomtm 维护规则

本页定义当前源码与构建权威、维护者职责、人工审查门禁和候选发布前证据。它不建立尚未实现的 stable release；未来版本与发布门禁以[发布与兼容契约](docs/release-and-compatibility.md)为准。

## 唯一源码与构建权威

本仓库根目录是 `pggomtm` 的唯一源码权威来源。维护者必须保持以下边界：

- `Cargo.toml`、`Cargo.lock` 与 `rust-toolchain.toml` 定义单 crate 和 locked Rust 输入。
- `src/` 是唯一实现，不在消费仓库、部署仓库或运维目录维护副本。
- 根级 `Dockerfile` 是当前 clean build、测试 gate 与 final image 组装的唯一权威入口。
- 构建和证据必须来自可识别 commit 的 clean checkout。

消费方只能使用经过验证的 artifact，不得复制源码后自行形成第二条构建链。

## 维护者职责

维护者对变更的技术结论和证据负责：

- 审查 Issue、OpenSpec 范围、源码、测试与文档是否一致。
- 审查 PostgreSQL OAuth 应用程序二进制接口（ABI）、pgrx 边界、allocator、panic 和 fail-closed 行为。
- 人工核对依赖、Rust toolchain、PostgreSQL minor、runtime 与架构变化。
- 保护私密安全报告，并确保公开材料不含 secret。
- 在候选交付前核对 clean build、测试结果、artifact 身份和已知限制。

维护者不得把持续集成（CI）通过等同于生产就绪。当前最终制品仍拒绝所有 token，仓库没有稳定发布版。

## 人工依赖与 PostgreSQL 审查

原生认证依赖和 runtime 变化必须由维护者逐项批准：

1. 阅读上游 changelog、安全公告、许可证与 feature diff。
2. 检查 `Cargo.lock` 的完整变化和重复依赖。
3. 在精确 PostgreSQL source、header 与 runtime minor 上重新构建。
4. 运行 C layout、Rust、真实 loader、callback 和 artifact 隔离门禁。
5. 只把实际通过的组合加入支持矩阵。

不得自动合并 pgrx、pgrx-pg-sys、JSON Object Signing and Encryption（JOSE）、Rust 补丁版本或 PostgreSQL minor 更新。其他 PostgreSQL 18 minor 通过相同门禁前，当前支持范围仍只有 18.4。

## 候选交付前证据

任何候选制品对外提供前都需要可复验记录：

- 精确 source commit 与 clean worktree 状态
- Rust、Cargo dependencies、PostgreSQL source/header、runtime、target、架构和 libc 身份
- Rustfmt、Clippy `-D warnings`、locked tests、C probe 与真实 PostgreSQL runtime 结果
- `PG_MODULE_MAGIC`、`_PG_oauth_validator_module_init` 和动态链接检查
- 无 gate 最终制品对测试 key、JWKS、token、probe 和 secret 的隔离扫描
- 安装、回退、限制和消费方候选验证结果

证据缺失或组合不匹配时停止交付。不要用本地 tag、可变文件名或未经验证的运行结果替代制品身份。

## 禁止的维护操作

以下操作会破坏源码或运行时权威，维护者不得执行：

- 在其他仓库保留第二份 `pggomtm` 源码或构建定义
- 在生产主机现场安装 Rust 后编译 module
- 在仍有 PostgreSQL backend 运行时热覆盖已加载的 `.so`
- 为认证失败增加旧实现、备用 issuer、网络 fallback 或宽松解析路径
- 对共享分支执行 `force push`、覆盖既有交付物或掩盖失败证据

升级和回退都应切换到已验证候选，并重建或重启全部受影响 backend。贡献与安全问题分别按[贡献指南](CONTRIBUTING.md)和[安全政策](SECURITY.md)处理。
