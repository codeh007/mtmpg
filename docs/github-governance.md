# 如何复验 GitHub 仓库治理设置

本页记录 `codeh007/mtmpg` 在 2026-07-18（UTC）只读复验的 GitHub 状态、公开开发主线目标、Agent 管理的拉取请求（Pull Request，PR）流程、Actions 权限、GitHub Container Registry（GHCR）与发布边界。GitHub 服务端设置是执行权威；文档、OpenSpec checkbox 或本地 Git 状态不能替代远端强制门禁。

## 当前公开仓库状态

仓库已经 public，但默认 `main` 仍是一次性 bootstrap 前的初始基线。完整源码、OpenSpec、维护文档与 workflow 暂时位于 `issue-116-extract-pggomtm`；这不表示源码仍为 private，也不表示该功能 ref 已经 stable。

| 设置 | 远端当前状态 |
| --- | --- |
| Visibility | `public` |
| Default branch | `main` |
| Description | `PostgreSQL 18 离线 OAuth validator 模块（Rust/pgrx）` |
| Topics | `authentication`、`oauth`、`pgrx`、`postgresql`、`postgresql-18`、`rust` |
| Homepage | 空 |
| Issues | 启用；[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 跟踪后续 release 闭环 |
| Blank issues | 允许；`main` 尚无自定义 issue template config |
| Pull Requests | 0 |
| Projects | 启用 |
| Wiki | 关闭 |
| Discussions | 关闭 |
| Git tags / Releases | 0 / 0 |
| `mtmpg-postgres` package | 不存在 |

Topics 是当前闭集。维护者不得加入尚未支持的平台、产品或营销标签，也不得把 homepage 指向不存在的文档站点。

## 把 main 作为开发主线

公开 `main` 表示经过远端检查的 development baseline，不表示 candidate、stable 或 production-ready。一次性 bootstrap 必须先完成追溯式 public-readiness、旧 cache 处置、无缓存 cold build、whole-branch review 与 source identity 核对，再把已审查功能 ref 非 force fast-forward 到 `main`。

该 fast-forward 只建立默认分支基线，不创建 Git tag、GitHub Release、package version alias 或 `latest`。推进后删除功能 ref，并通过普通 PR 删除 workflow 中的一次性 Issue #116 trigger。

首次 bootstrap 之后，普通变更只能从短期 Issue 分支通过 PR 进入 `main`。Stable candidate 从受保护 `main` 上冻结最终版本的精确 commit 构建一次；验收期间 `main` 可以继续前进，只要 candidate commit 仍是未改写祖先。

## 区分当前设置与目标门禁

下表同时记录服务端现状和 OpenSpec 目标，避免把 7.10 尚未执行的 mutation 写成已启用能力：

| 设置 | 当前状态 | 目标状态 |
| --- | --- | --- |
| Squash merge | 启用 | 保持唯一网页合并方式 |
| Merge commit / rebase merge | 禁用 / 禁用 | 保持禁用 |
| 合并后删除分支 | 启用 | 保持启用 |
| Auto-merge | 禁用 | 7.10 启用 |
| Repository rulesets | `[]` | 7.10 创建 `main` ruleset |
| Classic branch protection | HTTP `404`，未保护 | 由 `main` ruleset 提供目标约束 |
| Required approving reviews | 无服务端要求 | 固定为 `0` |
| Required checks | 无 | 要求 `Native CI` |
| 线性历史与讨论解决 | 未强制 | 由 ruleset 强制 |
| Force push / branch deletion | 未由 ruleset 禁止 | 对 `main` 禁止 |
| Secret scanning | 禁用 | 7.10 启用并复核 |
| Push protection | 禁用 | 7.10 启用并复核 |
| Dependabot vulnerability alerts | 禁用 | 7.10 启用并复核 |
| Private vulnerability reporting | 禁用 | 7.10 启用并复核 |
| Immutable releases | 启用 | 保持启用 |

Required approving review 数为 `0`，因为仓库当前只有一个贡献者。零审批不取消技术审查：高风险 PR 仍需在 Issue/PR 中记录上游 diff、风险、精确验证与独立技术结论。

## 由 Agent 管理普通 PR

完成 7.10 后，Agent 负责 Issue 范围内普通 PR 的生命周期：

1. 从受保护 `main` 创建短期分支和 PR。
2. 保持 Issue、OpenSpec task、diff 与验证证据一致。
3. 等待 required `Native CI`，读取失败日志并修复根因。
4. 确认讨论解决，且高风险变化具有显式技术审查证据。
5. 启用 squash auto-merge，让 GitHub 合并并删除源分支。

pgrx、JSON Object Signing and Encryption（JOSE）、Rust toolchain、PostgreSQL minor、官方 base/header、Actions source/pin、release workflow 或写权限变化不得无条件 auto-merge。Auto-merge 当前仍为禁用状态；在 7.10 完成前，维护者不能用直接合并冒充上述流程。

Dependabot 只维护 Cargo 与 GitHub Actions。任务 7.11 将按生态分组并限制并发 PR；native 认证依赖、Rust toolchain、PostgreSQL minor 与 release workflow 不自动合并。

## 保持 Actions 权限隔离

仓库已经启用 Actions 来源闭集与完整提交安全散列算法（Secure Hash Algorithm，SHA）固定：

- `allowed_actions=selected`，允许 GitHub-owned actions
- 第三方只允许 `docker/setup-buildx-action@*`、`docker/login-action@*`、`docker/metadata-action@*` 与 `docker/build-push-action@*`
- `verified_allowed=false`，不允许任意 verified creator
- `sha_pinning_required=true`，workflow 中每个 action 必须引用完整 commit SHA
- 默认 `GITHUB_TOKEN` 为 `read`，且不能批准 PR review

持续集成与交付（CI/CD）按职责拆成三条 lane：

| Lane | 触发与缓存 | 权限 | 当前状态 |
| --- | --- | --- | --- |
| PR/`main` cached | PR、`main` push；BuildKit GitHub Actions cache | `contents: read`，无 secret | 已实现；仍含一次性 feature trigger |
| Cold authority | schedule、workflow dispatch、发布前调用；无缓存 | read-only，不发布 | 尚未实现 |
| Trusted candidate/promotion | 受保护 `main` ancestry 上的精确 commit | job 级最小 package、Release、id-token 与 attestation 权限 | 尚未实现 |

公开 fork PR 只使用 GitHub-hosted 临时 runner、read-only token 和无 secret 上下文。Workflow 禁止 `pull_request_target`，普通 CI 不登录 GHCR、不上传正式制品，也不取得 package、Release 或 attestation 写权限。

## 公开读取 GHCR package

目标 package 是 `ghcr.io/codeh007/mtmpg-postgres`。它当前不存在；后续 trusted candidate workflow 创建 package 后，维护者把读取权限设为 public，同时把写权限限制在受保护 `main` 的 trusted job。

匿名读取不授予上传、删除、改标、attestation 或 Release 权限。Gomtmui 和其他消费者不保存 private pull credential，并始终使用完整 Open Container Initiative（OCI）digest：

```text
ghcr.io/codeh007/mtmpg-postgres@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

该值只展示 digest 格式。Source tag、SemVer tag 与 `latest` 只用于发现；candidate 先按 digest 验收，stable promotion 只给同一已验收 digest 增加 alias，不重建 image。

## 保持 Release 不可变

Repository immutable releases 当前为 `enabled=true`，但仓库没有 tag、Release 或 package。该设置只固定未来 Release 的不可变基线，不代表任何版本已经发布。

Candidate 从受保护 `main` 的精确 commit 只构建一次。Trusted promotion 从已验收 OCI digest 提取并复核相同 `.so`，然后创建 stable tag、alias 与 GitHub Release；promotion 不运行 Cargo 或 Docker rebuild。Actions 临时 artifact 不能成为正式安装入口，既有 tag、asset、manifest、evidence 与 image alias 不能覆盖。

## 追溯公开状态与安全功能

仓库在完整 public-readiness 之前已经公开。任务 7.6 必须扫描全部 refs/history、工作树、Docker context、workflow/log、Actions artifact/cache、GitHub 协作内容与 candidate image；真实 secret 先撤销或轮换，再按批准范围处置。任务 7.7 必须删除无法逐项证明安全的旧 cache，并从 clean checkout 建立无缓存 cold authority 起点。

当前远端安全状态为：

| 信号 | 当前结果 |
| --- | --- |
| `isSecurityPolicyEnabled` | `false`，因为默认 `main` 尚未识别 `SECURITY.md` |
| Dependency graph manifests | `0`，因为默认 `main` 尚未识别 `Cargo.toml` |
| Dependabot vulnerability alerts | `false` |
| Secret scanning | `disabled` |
| Secret scanning push protection | `disabled` |
| Private vulnerability reporting | `enabled=false` |

7.10 负责在完整基线进入 `main` 后启用并复核这些能力。更新本文不能替代 GitHub mutation，也不能把公开前状态倒推为已经通过追溯审计。

## 跟踪 release 闭环

[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 跟踪 OpenSpec 7.6–10.6，并反向链接 [gomtmui #116](https://github.com/codeh007/gomtmui/issues/116) 与 [gomtmui #117](https://github.com/codeh007/gomtmui/issues/117)。Issue 记录协作范围和远端证据，OpenSpec 继续定义行为契约与 task 完成状态。

## 使用只读命令复验状态

以下命令读取 repository、merge、Actions 与安全状态，不输出认证 header 或 credential：

```bash
gh api repos/codeh007/mtmpg \
  --jq '{
    private, visibility, default_branch,
    allow_squash_merge, allow_merge_commit,
    allow_rebase_merge, allow_auto_merge,
    delete_branch_on_merge, security_and_analysis
  }'
gh api repos/codeh007/mtmpg/actions/permissions/workflow
gh api repos/codeh007/mtmpg/private-vulnerability-reporting
```

以下命令分别读取 ruleset、classic protection 与依赖告警。未配置时，后两项可能以 HTTP `404` 表示关闭：

```bash
gh api repos/codeh007/mtmpg/rulesets
gh api repos/codeh007/mtmpg/branches/main/protection
gh api -i repos/codeh007/mtmpg/vulnerability-alerts
```

以下命令读取 tag、Release、目标 package 与当前协作项：

```bash
gh api repos/codeh007/mtmpg/immutable-releases --jq '{enabled}'
gh api 'repos/codeh007/mtmpg/releases?per_page=1' --jq 'length'
gh api users/codeh007/packages/container/mtmpg-postgres
gh issue list --repo codeh007/mtmpg --state all
gh pr list --repo codeh007/mtmpg --state all
```

任一字段偏离本页的当前状态时，维护者先判断对应 OpenSpec task 是否已经执行，再更新远端设置与文档。不要用文档改写代替服务端恢复。
