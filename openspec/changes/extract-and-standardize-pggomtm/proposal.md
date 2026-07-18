## Why

`pggomtm` 已经具备 PostgreSQL 18.4 OAuth validator 的核心实现，但开发与发布流程仍围绕临时功能分支、本地 Docker 构建、Dockerfile 内嵌测试以及过度的 PR/branch-protection 仪式展开。这些流程消耗本地计算资源，也没有把维护者真正关心的边界表达清楚：`main` 可以作为持续演进的源码集成线，只有通过远端验证的不可变构建物才能交付给用户。

## What Changes

- **BREAKING**：`main` 改为唯一持续集成与交付来源，允许维护者和 Agent 直接非 force 推送；不再要求 required PR、branch protection、squash-only、approving review 或 auto-merge 才能推进源码。
- 将 `issue-116-extract-pggomtm` 非 force fast-forward 到 `main`，删除 workflow 中的临时分支 trigger，并在迁移完成后删除该临时分支。
- 开发、测试和制品检查不再在共享本地工作区执行 `docker build`、`docker run` 或等价重计算；固定工具链、Rust/ABI/PostgreSQL 18.4 测试和 image 检查全部由 GitHub Actions 执行。
- `main` 的暂时失败不会回退或改写源码，但失败的 commit 不得发布 candidate、更新 stable alias 或覆盖既有制品。
- 根 `Dockerfile` 只构建 production module 并组装基于固定官方 PostgreSQL 18.4 digest 的 runtime image；测试、扫描、临时 cluster 和 CI policy 由 Actions 直接编排。
- 每个成功的 `main` commit 只构建并发布一次公开 GHCR candidate，同时生成 source-bound metadata、release manifest、SBOM、provenance 和 attestation。
- Gomtmui 只按完整 OCI digest 消费 candidate，并在远端环境验证 PostgreSQL、OAuth、identity、ACL/RLS、依赖服务和 rollback。
- Stable promotion 只给同一已验收 candidate digest 增加 SemVer/`latest` alias并创建 immutable GitHub Release，不运行 Cargo 或 Docker build。
- 重写 proposal、design、delta specs、tasks 与维护文档，删除临时分支、本地 Docker、独立 cold authority 和强制 PR 治理的过时描述。

## Capabilities

### New Capabilities

- `pggomtm-validator-module`：PostgreSQL 18 OAuth validator 的 ABI、离线验证、role、identity 和失败边界。
- `pggomtm-release-supply-chain`：main-first Actions CI/CD、标准 PostgreSQL 18.4 派生镜像、公开 GHCR、不可变发布和 gomtmui 消费契约。

### Modified Capabilities

无。

## Impact

- `codeh007/mtmpg` 的远端 `main`、GitHub Actions、Dockerfile、测试入口、供应链脚本和维护文档将统一到 Actions-only 模型。
- 本地工作区只承担源码与规划编辑、Git/OpenSpec 操作和只读调查；既有本地诊断 container/image 将按精确名称清理，不执行宽泛 prune。
- `codeh007/mtmpg` 仍是 Rust module、测试和 image 的唯一权威；gomtmui 不保留 Rust 副本、第二 Dockerfile、本地 image fallback 或现场编译路径。
- Gomtmui 最终只替换 `docker-compose.yml` 中 PostgreSQL image 的完整 digest；真实 E2E 由远端 workflow 执行，不修改生产数据库、生产配置或生产流量。
