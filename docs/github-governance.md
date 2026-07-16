# 如何复验 GitHub Actions 与合并设置

本页记录 `codeh007/mtmpg` 在 2026-07-16 验证的 GitHub 仓库设置、当前套餐限制和人工补偿控制。维护者可以使用下方只读命令复验状态，但不能把文档当成 GitHub 服务端强制门禁。

## 核对变更前后状态

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

是否升级套餐或公开仓库需要单独决策。不得创建会阻断已批准 Issue #116 快进交付的伪替代规则。

## 使用只读命令复验设置

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
