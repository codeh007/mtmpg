# GitHub Actions 与仓库治理

本页记录`codeh007/mtmpg`在2026-07-18（UTC）的远端状态，以及main-first、Actions-only的持续集成与交付边界。GitHub服务端和精确workflow源码是执行事实；OpenSpec定义行为与完成条件。

## 当前远端状态

| 设置 | 当前状态 |
| --- | --- |
| Visibility | `public` |
| Default branch | `main`，已包含完整开发基线 |
| Repository rulesets | `[]` |
| Classic branch protection | 未配置 |
| 默认`GITHUB_TOKEN` | `contents: read`，不能批准PR |
| Auto-merge | 禁用 |
| Squash / merge commit / rebase | 启用 / 禁用 / 禁用 |
| 合并后删除分支 | 启用 |
| Secret scanning / push protection | 禁用 / 禁用 |
| Dependabot vulnerability alerts | 禁用 |
| Private vulnerability reporting | 禁用 |
| Git tags / Releases | 0 / 0 |
| `mtmpg-postgres` package | 尚未发布 |

Branch protection、required Pull Request（PR）、approving review、squash-only和auto-merge都不是更新`main`的前置条件。仓库仍禁止force push；失败commit通过后续commit修复，不通过历史改写隐藏。

## Main 与制品状态分离

`main`是唯一持续集成与交付源码来源，也是development branch，不是stable alias。维护者和Agent可以直接非force推进范围明确的commit。

每个`main` SHA独立进入以下状态机：

1. GitHub Actions验证精确SHA。
2. 任一必需门禁失败时保留源码历史，不发布package、attestation、tag或Release。
3. 全部门禁成功后，从同一SHA只构建并push一次candidate。
4. Gomtmui按完整OCI digest验收并演练rollback。
5. Promotion只为同一已验收digest增加SemVer与`latest` alias，并创建immutable Release。

上一已验证candidate或stable digest不因后续`main`失败而改变。

## Native CI 权限与触发

`.github/workflows/native-ci.yml`的workflow级权限固定为`contents: read`。它包含以下入口：

- `main` push：唯一具有candidate资格的权威输入。
- 面向`main`的PR：可选、只读、无secret、无发布权限。
- 人工dispatch：只用于只读复验，不取得candidate资格。

Workflow不得包含临时Issue分支、Issue编号、schedule cold lane或`pull_request_target`。可信度来自固定输入、精确source metadata、内容寻址cache、远端完整门禁和最终digest，不依赖独立无缓存仪式。

Native CI直接编排source/secret、依赖/许可证、Rustfmt、Clippy、locked Cargo tests、官方OAuth ABI provenance、production artifact、专用PostgreSQL 18.4 integration和最终image readiness。根`Dockerfile`只构建production module并组装标准PostgreSQL image。

## Candidate 最小写权限

测试jobs始终只读。只有满足以下全部条件的candidate job才能请求最小写权限：

- 事件来自`codeh007/mtmpg`自身的`main` push。
- 当前ref为`refs/heads/main`，source为精确`GITHUB_SHA`。
- 所有native、security、integration与image jobs成功。
- Job仅取得所需的`packages: write`、`id-token: write`和attestation权限，不取得Release写权限。

PR、fork、dispatch、失败`main`和非`main` ref不得登录或写入GHCR，也不得创建attestation、tag或Release。

## 可选 PR 与外部贡献

PR保留为外部贡献与网页讨论入口，但不是维护者推进源码的必需流程。公开fork只能使用GitHub-hosted临时runner、read-only token和无secret上下文。

pgrx、JSON Object Signing and Encryption（JOSE）、Rust toolchain、PostgreSQL minor、官方base/header、Actions source/pin、release workflow或写权限变化仍需显式技术审查。没有required review不取消这一要求；结论必须记录在Issue/OpenSpec并绑定精确SHA。

## GHCR 与 Release

目标package为`ghcr.io/codeh007/mtmpg-postgres`，发布后必须允许匿名读取。消费者不保存private pull credential，并始终使用完整Open Container Initiative（OCI）digest：

```text
ghcr.io/codeh007/mtmpg-postgres@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

该值只展示格式。Source tag、SemVer tag和`latest`只用于发现。Candidate阶段不创建stable alias或GitHub Release；promotion不得运行Cargo或Docker build，只能重标同一已验收digest并发布与其一致的不可变材料。

## 只读复验

以下命令读取仓库、Actions与治理状态，不输出credential：

```bash
gh api repos/codeh007/mtmpg \
  --jq '{
    visibility, default_branch,
    allow_squash_merge, allow_merge_commit,
    allow_rebase_merge, allow_auto_merge,
    delete_branch_on_merge, security_and_analysis
  }'
gh api repos/codeh007/mtmpg/actions/permissions/workflow
gh api repos/codeh007/mtmpg/rulesets
gh api repos/codeh007/mtmpg/branches/main/protection
gh api 'repos/codeh007/mtmpg/releases?per_page=1' --jq 'length'
gh api users/codeh007/packages/container/mtmpg-postgres
```

未配置branch protection或尚未创建package时，对应API返回HTTP`404`。状态变化后必须同时更新workflow、OpenSpec和本文，不得用文档声明代替服务端事实。

发布闭环由[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)跟踪，并反向链接[gomtmui #116](https://github.com/codeh007/gomtmui/issues/116)与[gomtmui #117](https://github.com/codeh007/gomtmui/issues/117)。
