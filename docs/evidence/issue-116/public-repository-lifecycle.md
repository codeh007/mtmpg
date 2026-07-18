# Issue #116：公开仓库 lifecycle 对齐证据

本证据于 2026-07-18（UTC）采集，用于完成 OpenSpec 任务 7.5。结论绑定精确远端 GitHub Actions source，并记录 mtmpg 自有 release 跟踪 Issue 与 gomtmui 反向链接。本文不把文档目标解释为已经执行的 GitHub mutation 或发布能力。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29631276987](https://github.com/codeh007/mtmpg/actions/runs/29631276987) |
| Job | [`Native authority`](https://github.com/codeh007/mtmpg/actions/runs/29631276987/job/88045211492)，ID `88045211492` |
| Remote head SHA | `c15c89c49037fb5c20ff51bc9101a6e57c83d53e` |
| Event | `push` |
| Job 时间 | `2026-07-18T04:52:29Z` 至 `2026-07-18T04:52:57Z`，28 秒 |
| 结论 | `success` |
| Actions artifact | 0 |

远端 run 从 clean exact source 执行 public-readiness 与唯一 Docker build graph。七份 lifecycle 文档不在 Docker context，因此 build graph 复用前一精确 context 的内容寻址 cache；source scanner 仍扫描本次全部 tracked 文档。

## 同步的权威语义

本任务更新以下文件：

- `README.md`
- `SECURITY.md`
- `CONTRIBUTING.md`
- `MAINTAINERS.md`
- `AGENTS.md`
- `docs/github-governance.md`
- `docs/release-and-compatibility.md`

七份文档统一以下边界：

1. 仓库已经 public；默认 `main` 是 development baseline，不表示 candidate、stable 或 production-ready。
2. 当前功能 ref 只用于一次性 bootstrap。它通过追溯审计、cold build 与 whole-branch review 后非 force fast-forward 到 `main`，且不创建 tag、Release 或 `latest`。
3. 稳态普通变更由 Agent 管理 Issue、短期 PR、required `Native CI`、失败处置与 squash auto-merge；required approving review 数为 `0`，高风险变化仍需显式技术审查。
4. 目标 `ghcr.io/codeh007/mtmpg-postgres` 公开读取，写权限只属于受保护 `main` 的 trusted job。消费者不保存 private pull credential，并固定完整 OCI digest。
5. 最终版本先通过普通 PR 进入受保护 `main`，trusted workflow 再从精确 commit 只构建一次 candidate。Gomtmui 验收同一 source/manifest/digest 后，promotion 只晋级同一 digest，不重建 image。
6. Image 内 `pggomtm-build-manifest/v1` 不含自身 OCI digest；外部 `release-manifest.json` 在 digest 产生后绑定 source、`.so`、image、软件物料清单（SBOM）与 attestation。

## GitHub 服务端当前状态

任务完成时再次只读复验远端，得到：

| 设置或 surface | 当前结果 |
| --- | --- |
| Repository visibility | `public` |
| Default branch | `main`，仍为初始 bootstrap baseline |
| Squash / merge commit / rebase | `true` / `false` / `false` |
| Delete branch on merge | `true` |
| Auto-merge | `false` |
| Repository rulesets | `[]` |
| Secret scanning / push protection | `disabled` / `disabled` |
| Private vulnerability reporting | `enabled=false` |
| Pull Requests | 0 |
| Git tags / Releases | 0 / 0 |
| `mtmpg-postgres` package | 0 |

OpenSpec 7.10 负责启用 auto-merge、ruleset、安全扫描、依赖告警与私密漏洞报告。8.x 与 9.x 负责 trusted workflow、public package、candidate 与 stable。7.5 没有修改这些服务端设置，也没有创建 PR、tag、Release、package alias 或 `latest`。

## Release 跟踪 Issue 与反向链接

[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 于 `2026-07-18T04:40:26Z` 创建，标题为“完成 mtmpg 公开主线、可信 candidate 与首个 stable release”。Issue 明确记录：

- OpenSpec 7.6–10.6 的目标、当前基线、范围与非目标
- public-readiness、main 治理、manifest/SBOM/attestation、public GHCR、gomtmui consumer evidence 与 stable promotion 验收
- 不提前创建 stable、不恢复第二源码、不在 promotion 重建、不修改生产系统的边界
- 用户在 gomtmui #117 的既有继续实施授权

两个跨仓库总线 Issue 已添加反向链接：

- [gomtmui #116 反向链接](https://github.com/codeh007/gomtmui/issues/116#issuecomment-5009934543)
- [gomtmui #117 反向链接](https://github.com/codeh007/gomtmui/issues/117#issuecomment-5009934538)

两条评论只建立 mtmpg #1 的跟踪关系，并明确不代表 candidate、Release 或 stable 已完成。Mtmpg Issue 负责本仓库协作范围与远端证据，OpenSpec 继续负责行为契约和 task 状态。

## 文档聚焦验证

提交前完成以下检查：

- 获取最新 Writing Guidelines，审查七份文件的语气、结构、术语与格式
- 禁用词、破折号、未标注 opening code fence、硬换行段落与失效相对链接均为 0 findings
- README、CONTRIBUTING 与 GitHub governance 的全部 `bash` 围栏通过 `bash -n`
- `git diff --check` 通过
- `openspec validate extract-and-standardize-pggomtm --strict` 通过

本证据不完成追溯式远端扫描、cache 删除、cold bootstrap、`main` fast-forward、GitHub 安全 mutation、public GHCR、candidate 或 stable。后续状态由 [mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 与 OpenSpec 7.6–10.6 继续跟踪。
