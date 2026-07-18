# pggomtm 维护规则

本页定义公开仓库的源码与构建权威、维护者职责、main-first开发流程、技术审查门禁和候选发布前证据。公开的`main`是唯一CI/CD源码线，不表示stable；版本与发布门禁以[发布与兼容契约](docs/release-and-compatibility.md)为准，GitHub服务端状态见[GitHub Actions治理](docs/github-governance.md)。

## 唯一源码与构建权威

本仓库根目录是 `pggomtm` 的唯一源码权威来源。维护者必须保持以下边界：

- `Cargo.toml`、`Cargo.lock` 与 `rust-toolchain.toml` 定义单 crate 和 locked Rust 输入。
- `src/` 是唯一实现，不在消费仓库、部署仓库或运维目录维护副本。
- 根级`Dockerfile`只执行locked production build并组装固定PostgreSQL 18.4 runtime image。
- `.github/workflows/native-ci.yml`直接编排source、security、Rust、ABI、PostgreSQL integration、production artifact与image readiness；任务、consumer和发布证据必须来自可识别远端commit的clean checkout及成功Actions run。
- 本地不得运行Docker build/run、Cargo或原生编译、临时PostgreSQL和image检查；重计算入口必须在普通环境先拒绝。
- [mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)跟踪公开主线、candidate、跨仓库验收与首个 stable release；具体行为与完成状态仍以 OpenSpec 为权威。

消费方只能使用经过验证的 artifact，不得复制源码后自行形成第二条构建链。

## 维护者职责

维护者对变更的技术结论和证据负责：

- 审查 Issue、OpenSpec 范围、源码、测试与文档是否一致。
- 审查 PostgreSQL OAuth 应用程序二进制接口（ABI）、pgrx 边界、allocator、panic 和 fail-closed 行为。
- 人工核对依赖、Rust toolchain、PostgreSQL minor、runtime 与架构变化。
- 保护私密安全报告，并确保公开材料不含 secret。
- 在候选交付前核对 clean build、测试结果、artifact 身份和已知限制。
- 让Agent把范围明确的commit直接非force推进到`main`，等待精确SHA的CI；失败时保留历史并追加修复commit。

维护者不得把持续集成（CI）通过、进入 `main` 或 package 可公开拉取等同于生产就绪。当前 validator、closed role/identity、失败脱敏与 artifact readiness 已通过远端门禁，但 trusted release、软件物料清单（SBOM）、attestation、gomtmui consumer evidence 与 rollback 尚未完成，仓库没有稳定发布版。

## Main-first 与技术审查

普通变更可以由维护者或Agent直接非force推送到`main`。仓库不要求required PR、ruleset、branch protection、approving review、squash-only或auto-merge；公开PR仍可作为外部贡献入口，但只能运行read-only验证。

pgrx、JOSE、Rust toolchain、PostgreSQL minor、官方base/header、Actions source/pin、release workflow或写权限变化仍需显式技术审查。没有服务端强制审批不表示自动放行，审查结论必须绑定Issue、OpenSpec和精确SHA。

## 人工依赖与 PostgreSQL 审查

原生认证依赖和 runtime 变化必须由维护者逐项批准：

1. 阅读上游 changelog、安全公告、许可证与 feature diff。
2. 检查 `Cargo.lock` 的完整变化和重复依赖。
3. 由GitHub Actions在精确PostgreSQL source、header与runtime minor上重新构建。
4. 在同一远端SHA运行C layout、Rust、真实loader、callback和artifact隔离门禁。
5. 只把实际通过的组合加入支持矩阵。

不得在缺少显式审查时推进pgrx、pgrx-pg-sys、JSON Object Signing and Encryption（JOSE）、Rust补丁版本或PostgreSQL minor更新。其他PostgreSQL 18 minor通过相同门禁前，当前支持范围仍只有18.4。

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
- `ghcr.io/codeh007/mtmpg-postgres`匿名读取与仓库自身成功`main` candidate job写权限隔离；部署配置只引用完整OCI digest

证据缺失或组合不匹配时停止交付。不要在本地生成tag/image或用可变文件名、终端输出、未经验证的运行结果替代远端CI与制品身份。

## 禁止的维护操作

以下操作会破坏源码或运行时权威，维护者不得执行：

- 在其他仓库保留第二份 `pggomtm` 源码或构建定义
- 在生产主机现场安装 Rust 后编译 module
- 在仍有 PostgreSQL backend 运行时热覆盖已加载的 `.so`
- 为认证失败增加旧实现、备用 issuer、网络 fallback 或宽松解析路径
- 对共享分支执行 `force push`、覆盖既有交付物或掩盖失败证据

升级和回退都应切换到已验证候选，并重建或重启全部受影响 backend。贡献与安全问题分别按[贡献指南](CONTRIBUTING.md)和[安全政策](SECURITY.md)处理。
