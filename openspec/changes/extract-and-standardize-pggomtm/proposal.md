## Why

`pggomtm` 已经从 `gomtmui/native/pggomtm/` 硬切到独立 `mtmpg` 仓库，并形成可由远端 Native CI 验证的 PostgreSQL 18 离线 OAuth validator 主路径；但仓库在原计划的 public-readiness 门禁完成前已经公开，远端默认 `main` 仍只有初始基线，README、LICENSE、SECURITY、源码与 workflow 均停留在长期功能分支。现有计划又把“进入 `main`”与“成为 stable release”绑定，导致公开开发主线、仓库治理、跨仓库验收和不可变发布无法按常见开源流程独立推进。现在需要在不削弱原生认证与供应链门禁的前提下，校正公开状态、建立受保护的开发主线，并把一次构建、跨仓验收和同一 digest 晋级形成完整闭环。

## What Changes

- **BREAKING**：保持 `pggomtm` 唯一源码、测试、Cargo lock、toolchain 和 Docker build authority 位于 `mtmpg` 仓库根目录；`gomtmui` 及其他消费者不得恢复 submodule、subtree、vendored copy、本地镜像 fallback 或第二构建实现。
- 保留完整 `pgrx` 作为 PostgreSQL module magic、panic/error guard 和 allocator 安全层；OAuth callback ABI 继续只从目标 PostgreSQL 官方 `libpq/oauth.h` 生成最小 allowlist bindings，并保证单次 materialize、校验字节与最终 `OUT_DIR` 编译字节完全一致且不受外部 formatter 影响。
- 完成唯一离线 validator runtime 与正式 artifact 门禁：每个新 OAuth backend 从只读本地 public JWKS/config 建立不可变 startup snapshot，严格验证 ES256 database JWT、requested role 与版本化 `authn_id` 并 fail closed；不得增加网络、SQL/SPI、私钥、在线 introspection 或认证 fallback。
- 保持 PostgreSQL build/test minor 精确可取证，同时让 runtime 依赖 PostgreSQL major module magic 与 OAuth validator magic；每个拟部署 PG18 minor 仍须独立真实验证并由消费者固定完整 OCI digest。
- **PUBLIC-STATE CORRECTION**：把仓库 public 作为已经发生的外部状态，立即执行追溯式 public-readiness，覆盖全部 Git refs/history、tracked 与 uncommitted 文件、Docker context、workflow 源码与日志、Actions artifact/cache、最终 image 及 GitHub Issue/PR 内容。任何真实 secret 命中必须先吊销或轮换；无法形成可信审计证据的公开前 cache 必须清理后从 clean cold build 重建，不得倒推声称公开前门禁已经通过。
- 将公开开发主线与 stable 发布解耦：追溯审计、whole-branch review 和远端 cold CI 通过后，一次性把已审查远端功能分支 fast-forward 到 `main`，使 README、LICENSE、SECURITY、OpenSpec、源码与 workflow 成为默认分支基线；该推进不创建 stable tag、Release、version alias 或 `latest`。
- 首次基线之后，所有普通变更通过 Issue 范围内的短期 PR 进入受保护 `main`。Agent 负责创建/更新 PR、等待必需检查和启用 auto-merge，合并后自动删除分支；单贡献者仓库不设置无法满足的第二人批准门禁，但 native 认证依赖、Rust toolchain、PostgreSQL minor 与 release workflow 变化仍须显式技术审查。
- 建立三条职责隔离的远端 CI/CD lane：PR/`main` 使用 BuildKit cache 提供快速反馈；定时、人工和发布前 cold authority 从 clean checkout 无缓存复验；trusted release 只从受保护 `main` 的精确 commit 取得最小写权限并生成 GHCR、manifest、SBOM 与 attestation。
- Stable candidate 只从冻结最终版本的精确 `main` commit 构建一次，先按 source identity 发现、按 OCI digest 消费；mtmpg 门禁与 gomtmui 跨仓库 E2E 都验证同一 source/digest 后，才为同一 digest 增加 SemVer/`latest` 发现别名并创建 immutable GitHub Release，不得重建。
- 公开发布 `ghcr.io/codeh007/mtmpg-postgres` 的读取权限，写权限仍只授予受信 release job；消费者无需 private pull credential，但仍必须按完整 digest 部署。
- 区分 image 内不含自身 OCI digest 的 build manifest 与外部 `release-manifest.json`。外部 manifest 绑定 source、module/contract、PG/runtime、`.so`、OCI digest、SBOM、验证矩阵与 attestation，避免形成自引用制品身份。
- 发布版本化 consumer contract 与正负向测试向量，让 gomtmui 只负责 issuer、delegation、数据库 role/RLS、executor 和平台编排，并通过固定 release contract 与 OCI digest 消费 `pggomtm`。

## Capabilities

### New Capabilities

- `pggomtm-validator-module`: 定义独立 Rust PostgreSQL 18 OAuth validator 的官方 ABI 来源与最终字节同一性、离线 JWT/JWKS startup snapshot、role/identity、fail-closed 与 PG18 stable-line 兼容边界。
- `pggomtm-release-supply-chain`: 定义公开独立仓库治理、可重复 CI、公开读取的 GHCR runtime image、immutable Release、manifest、SBOM、provenance、版本兼容和消费者按 digest 安装契约。

### Modified Capabilities

无。

## Impact

- 仓库：`codeh007/mtmpg` 继续作为唯一源码与 release authority；`codeh007/gomtmui` 只消费制品与 contract，不恢复本地第二实现。
- GitHub：仓库已经公开；默认分支将取得完整开发基线，并启用实际可用的 secret scanning、push protection、dependency graph/alerts、branch ruleset、必需检查与 Agent auto-merge。公开源码与公开读取 GHCR 不扩大 package 写权限。
- Rust/PostgreSQL：保留固定 Rust、`pgrx`、JOSE 与 PG18 build baseline、官方 `oauth.h` 生成边界、正式离线 verifier 及精确 minor 验证策略。
- 交付：远端 GitHub Actions 成为唯一任务、consumer 与发布证据权威；从精确 `main` commit 只构建一次 candidate，并以同一 digest 完成跨仓验收、stable 晋级、SBOM/provenance 和 immutable Release。
- 部署：测试与后续生产平台使用和目标 PostgreSQL 完全匹配、按 digest 固定的派生 image；JWKS/config 继续作为运行时只读材料，不进入发布物，目标主机不安装 Rust/cargo 或现场编译。
- 安全：Git history、仓库、workflow 日志/artifact/cache、镜像和 Release 不得包含 signing private key、API key、OAuth/database JWT、连接串、运行数据或 Cargo target cache；公开前状态不能替代追溯审计证据。
