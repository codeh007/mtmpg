## Context

mtmpg 是单一 Rust crate。正式 `pggomtm` module 已实现 PostgreSQL 18 OAuth callback、离线 JWT/JWKS 验证、closed profile-role、版本化 `authn_id` 与 fail-closed 错误边界，已有 Rust tests、C probe、libpq smoke fixture 和 SQL runtime probe。

完整开发基线已经非force fast-forward到远端`main`，本地工作线已切换到同一main，临时功能分支也已删除。迁移前的规划曾把bootstrap、受保护分支、PR/auto-merge、cold cache仪式与stable发布耦合在一起，同时允许本地Docker诊断；本change已用main-first、Actions-only模型替代这些流程。

## Goals / Non-Goals

**Goals:**

- 立即把现有完整源码非 force 推进到 `main`，以后由 `main` 直接承载持续开发。
- 把 Rust、ABI、PostgreSQL integration、image build 和 image readiness 全部放到 GitHub Actions runner，不消耗共享本地工作区计算资源。
- 允许 `main` 暂时失败，但确保任何失败或未验证 commit 都不能生成用户可消费的 candidate/stable 制品。
- 交付行为上继承官方 `postgres:18.4-bookworm`、只预装正式 `pggomtm` module 的标准 PostgreSQL image。
- 从成功的精确 `main` commit 构建一次 candidate，让 gomtmui 按完整 digest 验收，再晋级同一 digest。

**Non-Goals:**

- 不要求 PR、branch protection、required review、squash-only 或 auto-merge 才能更新 `main`。
- 不保证 `main` 的每个瞬间都可发布，也不通过回退源码隐藏 CI 失败。
- 不在本地执行 Docker build/run、原生重编译、临时 PostgreSQL cluster 或最终 image 检查。
- 不把 `pggomtm` 改成需要 control/SQL/`CREATE EXTENSION` 的普通 SQL extension。
- 不把 Go executor、HTTP API、Cloudflare Tunnel、issuer 私钥、业务授权或数据库数据放入 image。
- 不修改生产数据库、生产配置或生产流量。

## Decisions

### 1. Main是源码集成线，制品状态独立

临时功能分支已经非force fast-forward到`main`，对应workflow trigger和远端分支已经删除。后续维护者与Agent可以直接向`main`提交小范围、可追踪的commit；公开PR仍可触发只读检查，但不是仓库唯一贡献者推进工作的前置条件。

`main` commit 与可发布制品使用不同状态机：

```text
main commit
    |
    +-- Actions失败 --> 保留源码，记录失败，不发布
    |
    `-- Actions成功 --> 构建一次candidate --> consumer验收 --> 同digest晋级stable
```

Git 历史不得 force rewrite。Main 失败时通过后续 commit 修复前进；最后一个已验证 candidate/stable digest 保持不变。

### 2. GitHub Actions承担全部重计算

`.github/workflows/native-ci.yml` 只以 `main` push 为权威 CI/CD 输入；可选 `pull_request` lane 始终只读。Workflow 不包含 Issue 编号、临时分支 trigger 或 scheduled cold lane。

Actions 依次直接运行：

1. Source/secret policy、依赖和许可证审计。
2. Rustfmt、Clippy `-D warnings` 与 locked Cargo feature matrix。
3. 官方 PostgreSQL 18.4 OAuth header、C layout 与 bindings 最终字节 provenance。
4. 专用临时 PostgreSQL 18.4 harness 的 loader、startup/config、OAuth allow/deny、identity 与失败脱敏矩阵。
5. Production artifact 的 feature、ELF、动态依赖和 capability 检查。
6. Final PostgreSQL image build及独立 image readiness。

这些入口可以使用固定 container 在 GitHub-hosted runner 内提供工具链，但执行入口必须拒绝普通本地环境。GitHub Actions cache可以加速相同内容输入；可信度来自固定输入、内容寻址、精确 source metadata、provenance 和最终 digest，不再维护独立 cold/cache 清理仪式。

### 3. Dockerfile只定义production image

Dockerfile 只包含两个职责明确的阶段：

1. 固定 Rust/toolchain/lock 与 PostgreSQL 18.4 development inputs，执行 locked production build并生成 source-bound build metadata。
2. 从固定 digest 的官方 `postgres:18.4-bookworm` 开始，只复制 `pggomtm.so`、MIT license 和 metadata。

Dockerfile 不复制 tests、fixture、workflow、scanner 或 test scripts，不启动 PostgreSQL，不产生测试 marker，也不重声明官方 entrypoint、command、volume、stop signal 或 user。Image build完成后，Actions 在外部比较官方 base config/filesystem，检查 module、ELF、metadata和真实官方entrypoint启动。

### 4. 成功main commit只发布一次candidate

Native CI 的测试 jobs 使用 `contents: read` 且无 secret。只有仓库自身 `main` push 在全部前置 jobs 成功后，candidate job 才取得最小 `packages: write`、`id-token: write` 和 attestation 权限。

Candidate job 从该精确 `GITHUB_SHA` 构建并推送一次 `ghcr.io/codeh007/mtmpg-postgres`。完整 OCI digest 是部署身份；`sha-<commit>` 只用于发现。失败、PR、fork 或非 `main` 事件不得写 package、Release 或 attestation。

Image 内 metadata记录source、toolchain、PostgreSQL/base、module和 `.so` digest，但不记录尚未产生的自身 OCI digest。Push产生digest后，同一 job从该构建结果生成外部release manifest、SBOM、provenance和attestation，不重建image。

### 5. Gomtmui只消费已发布digest

Gomtmui 将 `docker-compose.yml` 的 PostgreSQL image改为candidate完整digest，不保留本地Rust build或fallback。Consumer验证由gomtmui GitHub Actions在远端运行，覆盖官方initdb/volume/healthcheck、TLS、sub2api/pgAdmin连接、module加载、真实OAuth、`system_user` identity、ACL/RLS和rollback。

Consumer evidence绑定mtmpg source、internal/external manifest、OCI digest与gomtmui source。失败时gomtmui保持上一已验证digest，mtmpg以新main commit发布新candidate；不覆盖旧制品。

### 6. Stable只重标已验收digest

Promotion workflow从 `main` 上的精确source、candidate digest和consumer evidence开始，只为同一digest增加SemVer/`latest` alias并创建精确Git tag和immutable GitHub Release。Promotion不得运行Cargo、Docker build或生成不同module/image bytes。

## Risks / Trade-offs

- **Main可能暂时失败** -> candidate job依赖全部验证结果，失败只影响新制品，上一已验证digest不变。
- **所有重计算依赖GitHub Actions可用性** -> 保持测试入口显式、固定runner/tool版本和可重跑日志；Actions不可用时暂停构建发布，不回退到本地Docker。
- **每个成功main commit可能产生candidate** -> 使用source唯一tag和不可变digest，按明确保留策略清理未被consumer接受的发现tag，不删除已引用digest或证据。
- **专用OAuth harness不是`cargo pgrx test`默认路径** -> 这是非SQL-extension加载协议决定的，仍使用官方PostgreSQL工具和真实libpq验证。

## Migration Plan

1. 验证本次规划工件一致性。
2. 已将远端功能分支非force fast-forward到`main`，本地切换到同一commit并保留现有工作树修改。
3. 已删除临时分支trigger和分支，并精确清理既有本地诊断container/image；此后禁止本地Docker build/run。
4. 在`main`完成测试/Dockerfile/image职责分离和维护文档对齐，直接push并只读取远端Actions结果。
5. 建立成功main commit的candidate、manifest、SBOM/provenance/attestation发布。
6. 在gomtmui按digest完成远端consumer E2E和rollback。
7. 晋级同一digest并回填mtmpg #1、gomtmui #116/#117。

如果main CI失败，保留该commit并用后续commit修复，不发布candidate。若gomtmui消费失败，保持上一image并发布新candidate，不引入本地fallback。

## Open Questions

无。Main-first、Actions-only、candidate fail-closed和same-digest promotion已由用户确认。
