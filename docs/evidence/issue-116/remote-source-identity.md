# Issue #116：远端功能 ref 源码身份

本证据于 2026-07-17T06:00:02Z（UTC）采集。目标是解除 mtmpg 迁移源码只存在于
本地 `main` 的状态，同时保持远端默认分支不变，为后续远端 CI 与候选制品提供
可审计的 source identity。

## 推送前状态

- 本地 `main` 的已审查提交为
  `d6ffbf7658febe0027a2b30e6d535c9bc85719a9`。
- `refs/heads/issue-116-extract-pggomtm` 在远端不存在。
- `refs/heads/main` 在远端指向
  `453b2a71c8f98b8824278f2d469683626c6d7e71`。
- 本地 `main` 继续跟踪 `origin/main`；没有把 upstream 改成新的功能 ref。

## 创建方式

使用普通、非 force push 从精确本地提交创建远端功能 ref：

```sh
git push origin HEAD:refs/heads/issue-116-extract-pggomtm
```

命令 exit 0，Git 报告创建新分支。未使用 `--force`、`--force-with-lease`、
`--set-upstream`，也未创建 tag、Release、OCI 制品或推进默认分支。

## 远端复验

```sh
git ls-remote --heads origin main issue-116-extract-pggomtm
```

复验结果：

| Ref | Remote commit |
| --- | --- |
| `refs/heads/issue-116-extract-pggomtm` | `d6ffbf7658febe0027a2b30e6d535c9bc85719a9` |
| `refs/heads/main` | `453b2a71c8f98b8824278f2d469683626c6d7e71` |

功能 ref 与受审查本地提交完全一致；`origin/main` 在推送前后保持同一 commit。
后续 CI、prerelease 或 short-SHA candidate 必须关联远端可达的精确 source commit，
不得把本证据扩大为 stable 或 production readiness 证明。
