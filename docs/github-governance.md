# 如何复验 GitHub 仓库治理设置

本页记录 `codeh007/mtmpg` 在 2026-07-16至2026-07-17验证的仓库元数据、协作入口、安全状态、不可变发布、GitHub Actions、合并设置、套餐限制和人工补偿控制。维护者可以使用下方只读命令复验状态，但不能把文档当成 GitHub 服务端强制门禁。

## 核对仓库元数据与协作入口

远程仓库当前保持 private，默认分支仍为 `main`。元数据只描述已经实现的 PostgreSQL 18 离线开放授权（OAuth）validator 模块，不声明 SQL extension、production-ready 或 stable 状态。

| 设置 | 远程当前状态 |
| --- | --- |
| Visibility | `private` |
| Default branch | `main` |
| Description | `PostgreSQL 18 离线 OAuth validator 模块（Rust/pgrx）` |
| Topics | `authentication`、`oauth`、`pgrx`、`postgresql`、`postgresql-18`、`rust` |
| Homepage | 空 |
| Issues | 启用 |
| Blank issues | 允许；`main` 没有覆盖默认行为的 `.github/ISSUE_TEMPLATE/config.yml` |
| Projects | 启用 |
| Wiki | 关闭 |
| Discussions | 关闭 |

Topics 是闭集。维护者不得加入营销词、`gomtmui`、Cloudflare、Supabase 或尚未支持的平台标签，也不得为 homepage 填写不存在的文档站点。

## 区分本地安全策略与远程安全功能

安全状态必须按证据来源分别解释。[本地安全策略](../SECURITY.md) 已在当前功能分支中跟踪，但尚未推送到远程默认分支。GraphQL 返回 `isSecurityPolicyEnabled=false`，读取 `main` 的 `SECURITY.md` 也返回超文本传输协议（HTTP）`404`，因此远程当前没有已识别的 security policy。

| 信号 | 当前结果 | 结论 |
| --- | --- | --- |
| GraphQL `hasVulnerabilityAlertsEnabled` | `false` | Dependabot vulnerability alerts 未启用 |
| Dependabot alerts 端点 | HTTP `403`，明确报告 alerts disabled | 与 GraphQL 结果一致 |
| GraphQL `dependencyGraphManifests.totalCount` | `0` | 当前未识别 dependency manifest，不据此扩大能力声明 |
| Code security configuration 端点 | HTTP `204`，无响应体 | 当前没有可读取的附加 code security configuration |
| Repository `security_and_analysis` | Owner/name repository 路由持续 HTTP `503` | 托管端当前不可读取，状态未知 |
| Automated security fixes | HTTP `503` | 托管端当前不可读取，状态未知 |
| Secret scanning alerts | HTTP `503` | 托管端当前不可读取，不能声称 secret scanning 已启用 |
| Private vulnerability reporting | HTTP `404` | 当前 private 仓库类型下不可用，不声称已启用 |

本次治理没有执行安全设置 mutation，也没有启用 GitHub Advanced Security（GHAS）、secret scanning 或付费能力。维护者不得把 `503`、空响应或本地文件解释成远程能力已启用。

## 保持 GitHub Release 不可变

仓库当前设置 `immutable releases` 为 `enabled=true`。该设置只固定后续 GitHub Release 的不可变基线，不会创建 tag、Release、asset 或 GitHub Container Registry（GHCR）package。

当前 GitHub Release 数和 Git tag 数均为 `0`。精确查询 `mtmpg-postgres` container package 返回 HTTP `404 Package not found.`，因此当前不存在该目标 package，也不存在 stable Release。后续每个正式 Git tag 必须对应一个 immutable GitHub Release；Actions 临时 artifact 不能作为正式分发入口，已创建 Release 的 tag、asset 与 manifest 不能被覆盖或替换。

## 核对 Actions 与合并设置变更

本次治理变更保持仓库为 private、默认分支为 `main`，并收紧 GitHub Actions、完整提交安全散列算法（Secure Hash Algorithm，SHA）引用与拉取请求（Pull Request，PR）合并设置：

| 设置 | 变更前 | 变更后 |
| --- | --- | --- |
| 仓库可见性与账户套餐 | private、free | private、free，未改变 visibility 或 plan |
| 默认分支 | `main` | `main` |
| Actions 总开关 | `enabled=true` | `enabled=true` |
| Actions 来源 | `allowed_actions=all` | `allowed_actions=selected` |
| 完整提交 SHA 固定 | `sha_pinning_required=false` | `sha_pinning_required=true` |
| Selected actions | 顶层允许全部来源，读取端点返回 `409 Conflict` | 仅 GitHub-owned actions 与四个批准的 Docker action |
| 默认 workflow token | `read`，不能批准 PR review | `read`，不能批准 PR review，已显式重设 |
| 网页合并方式 | squash、merge commit、rebase 都启用 | 仅 squash 启用 |
| 自动合并 | 关闭 | 关闭 |
| 合并后删除分支 | 关闭 | 开启 |
| Squash commit 标题 | `COMMIT_OR_PR_TITLE` | `PR_TITLE` |
| Repository rulesets | 超文本传输协议（HTTP）`403` 套餐限制 | 未创建，仍返回相同 `403` |
| `main` branch protection | HTTP `403` 套餐限制 | 未启用，仍返回相同 `403` |

完整 SHA 固定要求 workflow 使用 action 的完整 commit SHA。它拒绝 tag、branch 和缩短 SHA 等可变或不完整引用。

## 限制 Actions 来源

Actions 设置采用闭集来源，并通过全局完整 SHA 要求固定每个实际引用：

- **GitHub-owned actions**: 允许 GitHub 拥有的 actions
- **Docker Buildx**: 允许 `docker/setup-buildx-action@*`
- **Docker 登录**: 允许 `docker/login-action@*`
- **Docker 元数据**: 允许 `docker/metadata-action@*`
- **Docker 构建与推送**: 允许 `docker/build-push-action@*`

`verified_allowed=false` 禁止任意 verified creator。列表不包含 `docker/*` 或其他第三方通配来源。Selected actions 中的 `@*` 只描述批准仓库，`sha_pinning_required=true` 仍要求 workflow 使用完整 commit SHA。

新增 action 来源前，维护者必须核对上游身份、所需权限和精确 commit，再单独修改仓库设置。不得通过扩大到组织级通配模式绕过审查。

## 保持默认 workflow token 只读

仓库默认 `GITHUB_TOKEN` 权限为 `read`，且 `can_approve_pull_request_reviews=false`。普通持续集成（Continuous Integration，CI）不得依赖仓库级默认写权限，也不能让 workflow 批准 PR review。

未来 release job 只有在对应 workflow 任务实现时，才能在 job 内显式申请发布所需的最小权限。仓库默认权限不得改为 `write`。

## 以远端Native CI作为验证证据

根`Dockerfile`继续定义唯一native build graph；`.github/workflows/native-ci.yml`只负责编排远端checkout、Buildx、缓存、权限和日志，不复制Rust、C或PostgreSQL门禁实现。普通CI具有以下边界：

- 由`issue-116-extract-pggomtm`分支push、面向`main`的PR、`main` push或后续人工dispatch触发
- 使用GitHub-hosted `ubuntu-24.04`临时runner与120分钟job上限
- `permissions`只包含`contents: read`，checkout不持久化credential
- `actions/checkout@v7.0.0`、`docker/setup-buildx-action@v4.2.0`与`docker/build-push-action@v7.3.0`均固定到已验证的完整commit SHA
- 使用BuildKit GitHub Actions cache与同ref并发取消，不登录GHCR、不读取发布secret、不push或load image、不生成SBOM/image provenance
- 禁用build record artifact、build summary和build-record metadata provenance，避免普通CI日志记录完整push event payload
- 禁止`pull_request_target`；未来公开仓库的非受信PR仍只能在无secret、只读权限上下文运行

首次workflow不在默认分支时，由功能分支push或PR事件bootstrap。维护者使用以下命令查看结果；本地Docker结果不得代替远端run：

```bash
gh run list \
  --repo codeh007/mtmpg \
  --workflow native-ci.yml \
  --branch issue-116-extract-pggomtm \
  --limit 5
gh run watch <run-id> --repo codeh007/mtmpg --exit-status
gh run view <run-id> --repo codeh007/mtmpg --log-failed
gh run rerun <run-id> --repo codeh007/mtmpg
```

首次feature push bootstrap与最终日志最小化run均已成功；精确run、SHA、job、权限、artifact数量和非cold限制见[Native CI bootstrap证据](evidence/issue-116/native-ci-bootstrap.md)。

后续cold authority任务才会为人工、定时和发布前复验增加无缓存模式；普通开发CI不得默认禁用缓存。

## 对普通 PR 使用 squash-only

普通未来 PR 只能通过网页 squash 合并，合并后 GitHub 删除源分支。Merge commit、rebase merge 和 auto-merge 均关闭，squash commit 标题使用 PR 标题。

Squash-only 是仓库策略，不等同于强制 branch protection。维护者仍需在合并前人工确认评审、CI 结果、提交范围和发布影响。

## 保留 Issue #116 的一次性快进例外

Issue #116 的首次跨仓库 stable 交付必须保持已验证 source commit 不变。全部 mtmpg 门禁与 gomtmui 跨仓库验收通过后，维护者使用本地 `git merge --ff-only` 把已验证分支原样推进到 `main`，再执行非 force push。

这次操作不能使用网页 squash，因为 squash 会生成不同 commit。维护者必须在推进前确认已验证 branch `HEAD`、candidate 证据和预期 `main` 目标一致；推进后再确认远程 `main` 指向同一完整 commit。该例外只适用于 Issue #116 的首次 stable 交付，普通未来 PR 继续遵守 squash-only。

## 在套餐限制下执行人工控制

仓库保持 private，所有者账户为 free。GitHub 表述性状态转移（Representational State Transfer，REST）应用程序编程接口（Application Programming Interface，API）对 `/rulesets` 与 `/branches/main/protection` 返回 `403`，消息为 `Upgrade to GitHub Pro or make this repository public to enable this feature.`。

本次工作没有升级套餐、公开仓库或尝试创建付费治理功能。当前不存在可由 GitHub 强制执行的 ruleset 或 branch protection，维护者必须执行以下补偿控制：

- 合并前人工检查变更范围、review 和对应 CI 证据
- 保持 Actions 来源闭集、完整 SHA 固定和默认 token 只读
- 保持 auto-merge 关闭，并对普通 PR 使用 squash-only
- 创建 tag 或 release 前核对 OpenSpec 门禁、source commit 和跨仓库验证证据
- 推进 `main` 前确认 ancestry；Issue #116 只允许上述 `--ff-only` 流程
- 在 Issue、OpenSpec、commit 和发布证据中保留可复验身份，不用文档冒充服务端保护

所有者已经确认后续会公开源码仓库，但visibility仍只能在public-readiness门禁通过后由所有者手动改变。公开前必须扫描全部refs/history、工作树、Docker context、workflow日志/artifact、最终image及GitHub协作内容；真实secret先吊销或轮换，合成fixture只能精确分类。公开后立即复核secret scanning、依赖安全与branch protection/ruleset，并在改变visibility前解决它们与Issue #116首次精确source交付的关系。GHCR package visibility仍是独立决策。

## 使用只读命令复验元数据与协作入口

Owner/name repository 路由当前持续返回 `503`。以下 GraphQL 查询只读取仓库元数据、协作入口、tag 和 Release 数量：

```bash
gh api graphql \
  -f query='
    query {
      repository(owner: "codeh007", name: "mtmpg") {
        isPrivate
        defaultBranchRef { name }
        description
        homepageUrl
        hasIssuesEnabled
        hasProjectsEnabled
        hasWikiEnabled
        hasDiscussionsEnabled
        repositoryTopics(first: 20) { nodes { topic { name } } }
        refs(refPrefix: "refs/tags/", first: 1) { totalCount }
        releases(first: 1) { totalCount }
      }
    }
  ' \
  --jq '.data.repository'
gh api \
  'repos/codeh007/mtmpg/contents/.github/ISSUE_TEMPLATE/config.yml?ref=main'
```

GraphQL 返回协作入口状态；issue template config 当前返回 HTTP `404`，确认没有关闭 blank issues 的远程配置。

以下只读查询分别检查安全核心状态和 `main` 是否已识别安全策略文件：

```bash
gh api graphql \
  -f query='
    query {
      repository(owner: "codeh007", name: "mtmpg") {
        isSecurityPolicyEnabled
        hasVulnerabilityAlertsEnabled
        dependencyGraphManifests(first: 1) { totalCount }
      }
    }
  ' \
  --jq '.data.repository'
gh api repos/codeh007/mtmpg/contents/SECURITY.md?ref=main
```

以下命令交叉检查托管安全端点。当前依次返回 HTTP `503`、`204`、`403`、`503`、`503` 和 `404`；后续返回值变化时，维护者必须按实际响应更新判断，不能把不可读取状态记为启用：

```bash
gh api repos/codeh007/mtmpg --jq '{security_and_analysis}'
gh api repos/codeh007/mtmpg/code-security-configuration
gh api 'repos/codeh007/mtmpg/dependabot/alerts?per_page=1'
gh api repos/codeh007/mtmpg/automated-security-fixes
gh api 'repos/codeh007/mtmpg/secret-scanning/alerts?per_page=1'
gh api repos/codeh007/mtmpg/private-vulnerability-reporting
```

以下命令确认 immutable releases 已启用、Release 列表为空，且只检查批准的目标 package 名：

```bash
gh api repos/codeh007/mtmpg/immutable-releases --jq '{enabled}'
gh api 'repos/codeh007/mtmpg/releases?per_page=1' --jq 'length'
gh api users/codeh007/packages/container/mtmpg-postgres
```

## 使用只读命令复验 Actions 与合并设置

以下 `gh api` 命令只读取仓库、可见性、默认分支和 merge 字段。它们不会输出认证 header 或 credential：

```bash
gh api repos/codeh007/mtmpg \
  --jq '{
    private,
    visibility,
    default_branch,
    allow_squash_merge,
    allow_merge_commit,
    allow_rebase_merge,
    allow_auto_merge,
    delete_branch_on_merge,
    squash_merge_commit_title,
    squash_merge_commit_message
  }'
gh api user --jq '{login, plan: .plan.name}'
```

以下命令分别读取 Actions 顶层权限、批准来源和默认 workflow token：

```bash
gh api repos/codeh007/mtmpg/actions/permissions \
  --jq '{enabled, allowed_actions, sha_pinning_required}'
gh api repos/codeh007/mtmpg/actions/permissions/selected-actions \
  --jq '{
    github_owned_allowed,
    verified_allowed,
    patterns_allowed
  }'
gh api repos/codeh007/mtmpg/actions/permissions/workflow \
  --jq '{
    default_workflow_permissions,
    can_approve_pull_request_reviews
  }'
```

分别运行以下只读命令，确认两个付费治理端点仍返回预期 `403` 和相同升级提示：

```bash
gh api repos/codeh007/mtmpg/rulesets
```

```bash
gh api repos/codeh007/mtmpg/branches/main/protection
```

如果任一可配置字段偏离本页的变更后状态，维护者必须停止合并或发布并修正远程设置。不要用更新本页代替恢复 GitHub 服务端目标状态。
