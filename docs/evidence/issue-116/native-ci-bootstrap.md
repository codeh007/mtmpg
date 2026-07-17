# Issue #116：Native CI bootstrap 远端证据

本证据于2026-07-17（UTC）采集，用于完成OpenSpec任务6.3与7.1。根
`Dockerfile`仍是唯一build graph；本次没有运行本地完整Docker构建，也没有把
本地image或终端结果作为验收依据。

## 远端source与首轮完整执行

任务6.3的closed profile-role候选提交为
`c7720c4`，workflow与流程文档提交为`716b189`。两者以普通、非force push进入
`issue-116-extract-pggomtm`，由该push首次启动`Native CI`：

| 字段 | 值 |
| --- | --- |
| Run | [29591036227](https://github.com/codeh007/mtmpg/actions/runs/29591036227) |
| Remote head SHA | `716b1891094d100de2b50c37219ad29f67a0c32d` |
| Event | `push` |
| Job | `Native authority`，ID `87919904275` |
| Job时间 | `2026-07-17T15:10:45Z`至`2026-07-17T15:25:02Z`，14分17秒 |
| 结论 | `success` |
| Actions artifact | 0 |

Checkout、clean source identity、Buildx初始化和根Docker authority graph全部成功。
该远端SHA包含任务6.3的unit、config扩权与真实PostgreSQL forbidden-role门禁，
因此该run完成6.3所要求的远端验证；production Rust逻辑没有为该任务改动。

## 最终bootstrap状态

首轮日志审查发现Docker Buildx的metadata file会默认生成最小
`buildx.build.provenance`，其中包含push event payload。`--provenance=false`只禁用
image attestation，`DOCKER_BUILD_SUMMARY=false`只禁用job summary，二者都不会单独
删除该metadata。普通CI随后显式设置
`BUILDX_METADATA_PROVENANCE=disabled`；精确source identity继续由checkout后的独立
SHA与clean tree检查保证。

最终workflow状态由以下远端缓存run验证：

| 字段 | 值 |
| --- | --- |
| Run | [29592650829](https://github.com/codeh007/mtmpg/actions/runs/29592650829) |
| Remote head SHA | `55d1cec763816990a9524109c41a9e1f0850b0d7` |
| Event | `push` |
| Job | `Native authority`，ID `87925284020` |
| Job时间 | `2026-07-17T15:34:12Z`至`2026-07-17T15:34:38Z`，26秒 |
| 结论 | `success` |
| Actions artifact | 0 |

该run复用内容寻址的GitHub Actions cache并成功解析完整Docker graph。缓存命中符合
日常feature push/PR lane契约；它不是任务7.3或10.1要求的无缓存cold authority证据。

## Workflow边界复验

最终`.github/workflows/native-ci.yml`满足以下边界：

- 由批准功能分支push、面向`main`的pull request、`main` push和人工dispatch触发；
- 同workflow/ref并发取消，使用GitHub-hosted `ubuntu-24.04`与120分钟job上限；
- workflow只声明`contents: read`，仓库默认workflow权限也是`read`且不能批准PR；
- checkout不持久化credential；checkout、Buildx和build-push action均固定完整SHA；
- 只运行根`Dockerfile`，使用BuildKit GitHub Actions cache，不登录或push GHCR，
  不load image，不生成SBOM或image provenance；
- 禁用build record artifact、build summary和build-record metadata provenance。

最终run的完整日志以不回显匹配内容的扫描检查以下类别：邮箱格式、GitHub token
前缀、`Authorization: Bearer`、private-key header、JWT、携带credential的PostgreSQL
URI及`github_event_payload`。全部类别均为缺失；GitHub API同时确认该run的artifact
数量为0。

先前run `29591036227`与`29592305440`在metadata开关落地前已经产生，日志中仍有
push event metadata。它们没有上传artifact，也没有发现token、JWT、私钥或数据库
credential模式，但仍必须纳入任务7.4的全workflow日志public-readiness审计；不得用
删除日志或弱化扫描规则冒充门禁通过。

## 分支边界

采集时远端refs为：

| Ref | Remote commit |
| --- | --- |
| `refs/heads/issue-116-extract-pggomtm` | `55d1cec763816990a9524109c41a9e1f0850b0d7` |
| `refs/heads/main` | `453b2a71c8f98b8824278f2d469683626c6d7e71` |

`origin/main`没有移动，没有创建tag、Release或GHCR package。本证据只完成任务6.3
与日常CI bootstrap 7.1，不代表cold、release、public-readiness、consumer或stable
门禁已经完成。
