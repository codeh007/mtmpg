# pggomtm 维护规则

本页定义公开仓库的源码与构建权威、维护者职责、Agent 管理的拉取请求（Pull Request，PR）流程、技术审查门禁和候选发布前证据。公开的 `main` 是开发主线，不表示 stable；版本与发布门禁以[发布与兼容契约](docs/release-and-compatibility.md)为准，GitHub 服务端状态见[GitHub Actions 与合并治理](docs/github-governance.md)。

## 唯一源码与构建权威

本仓库根目录是 `pggomtm` 的唯一源码权威来源。维护者必须保持以下边界：

- `Cargo.toml`、`Cargo.lock` 与 `rust-toolchain.toml` 定义单 crate 和 locked Rust 输入。
- `src/` 是唯一实现，不在消费仓库、部署仓库或运维目录维护副本。
- 根级 `Dockerfile` 是clean build、测试gate与final image组装的唯一build graph authority。
- `.github/workflows/native-ci.yml`是执行该graph的远端CI authority；任务、consumer和发布证据必须来自可识别远端commit的clean checkout及成功Actions run。
- 本地测试和image只用于诊断，不能替代远端run或发布身份。
- [mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)跟踪公开主线、candidate、跨仓库验收与首个 stable release；具体行为与完成状态仍以 OpenSpec 为权威。

消费方只能使用经过验证的 artifact，不得复制源码后自行形成第二条构建链。

## 维护者职责

维护者对变更的技术结论和证据负责：

- 审查 Issue、OpenSpec 范围、源码、测试与文档是否一致。
- 审查 PostgreSQL OAuth 应用程序二进制接口（ABI）、pgrx 边界、allocator、panic 和 fail-closed 行为。
- 人工核对依赖、Rust toolchain、PostgreSQL minor、runtime 与架构变化。
- 保护私密安全报告，并确保公开材料不含 secret。
- 在候选交付前核对 clean build、测试结果、artifact 身份和已知限制。
- 让 Agent 为普通变更创建或更新短期 PR、等待 required CI、处理失败并在条件满足后启用 auto-merge，避免把 PR 生命周期转交给单一贡献者手工维护。

维护者不得把持续集成（CI）通过、进入 `main` 或 package 可公开拉取等同于生产就绪。当前 validator、closed role/identity、失败脱敏与 artifact readiness 已通过远端门禁，但 trusted release、软件物料清单（SBOM）、attestation、gomtmui consumer evidence 与 rollback 尚未完成，仓库没有稳定发布版。

## Agent auto-merge 与技术审查

首次 bootstrap 完成后，普通变更通过受保护 `main` 的 Issue 范围内短期 PR 进入开发主线。GitHub ruleset 要求 PR、`Native CI`、线性历史和讨论解决，required approving review 数为 `0`；仓库保持 squash-only、auto-merge 与合并后删除分支。

Agent 只能在 required checks 成功、讨论已解决且风险证据齐全后启用 auto-merge。pgrx、JOSE、Rust toolchain、PostgreSQL minor、官方 base/header、Actions source/pin、release workflow 或写权限变化仍需显式技术审查；零强制审批不是自动放行。

当前 auto-merge、ruleset 与 branch protection 尚未启用，OpenSpec 任务 7.10 负责实际 mutation 与复核。在该任务完成前，文档目标不能替代 GitHub 服务端门禁。

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
- GitHub Actions workflow、run ID、job结论与远端commit的一致性
- Rust、Cargo dependencies、PostgreSQL source/header、runtime、target、架构和 libc 身份
- Rustfmt、Clippy `-D warnings`、locked tests、C probe 与真实 PostgreSQL runtime 结果
- `PG_MODULE_MAGIC`、`_PG_oauth_validator_module_init` 和动态链接检查
- 无 gate 最终制品对测试 key、JWKS、token、probe 和 secret 的隔离扫描
- 安装、回退、限制和消费方候选验证结果
- 内部 build manifest 不含自身 OCI digest，外部 release manifest 在 digest 产生后绑定 source、`.so`、image、SBOM 与 attestation
- `ghcr.io/codeh007/mtmpg-postgres` 匿名读取与 trusted job 写权限隔离；部署配置只引用完整 OCI digest

证据缺失或组合不匹配时停止交付。不要用本地tag、可变文件名、终端输出或未经验证的运行结果替代远端CI与制品身份。

## 禁止的维护操作

以下操作会破坏源码或运行时权威，维护者不得执行：

- 在其他仓库保留第二份 `pggomtm` 源码或构建定义
- 在生产主机现场安装 Rust 后编译 module
- 在仍有 PostgreSQL backend 运行时热覆盖已加载的 `.so`
- 为认证失败增加旧实现、备用 issuer、网络 fallback 或宽松解析路径
- 对共享分支执行 `force push`、覆盖既有交付物或掩盖失败证据

升级和回退都应切换到已验证候选，并重建或重启全部受影响 backend。贡献与安全问题分别按[贡献指南](CONTRIBUTING.md)和[安全政策](SECURITY.md)处理。
