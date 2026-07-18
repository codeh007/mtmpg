# Issue #116：bootstrap cold authority 证据

本证据于 2026-07-18（UTC）采集，用于完成 OpenSpec 任务 7.7。全部旧 GitHub Actions 与 BuildKit cache 已从 259 项删除到 0 项；随后，精确远端 source 从 clean checkout 执行无 secret、无缓存的唯一 Docker build graph并成功。该 run 生成的新 cache 成为可信缓存起点。

## Cache 清理边界

任务 7.6 的追溯扫描报告 207 个旧 cache entry 无法下载内容。后续 source 扫描与 workflow 验证运行使删除前 inventory 增至 259 项。清理开始前没有 queued 或 in-progress run，本地工作树为 clean，且本地 HEAD 与远端功能分支一致。

| 字段 | 值 |
| --- | --- |
| 删除前 cache | 259 |
| 成功删除 | 259 |
| 删除后 cache | 0 |
| 独立复核 | `total_count=0`，返回列表为 0 |
| 完成时间 | `2026-07-18T05:43:20Z` |

清理逐项调用 GitHub Actions cache 删除 API，并循环读取 inventory，直到服务端返回 0。它没有删除或改写 Git refs/history、workflow logs、Actions run、Release、package、Issue 或本地源码，也没有替代 secret rotation。

## Cold workflow 证明

功能分支 push 在同一 `docker/build-push-action` 步骤设置 `no-cache: true`，仍只执行根 `Dockerfile`。现有 policy gate 对该精确表达式建立 tracked 断言。首次远端验证 run [29632227438](https://github.com/codeh007/mtmpg/actions/runs/29632227438) 因断言字面量触发固定 ShellCheck 0.11.0 的 `SC2016` 而失败；修复只调整 shell 引号，没有修改 cold 条件或降低 ShellCheck。修复后的预验证 run [29632399183](https://github.com/codeh007/mtmpg/actions/runs/29632399183) 在删除 cache 前成功，因此不作为清理后 authority。

清理后使用空树差异提交 `449215707492e1eb322d23702a8f16d0975063fa` 触发权威 run。该 commit 只建立审计型 source identity，不改变已验证文件树。

| 字段 | 值 |
| --- | --- |
| Run | [29632760924](https://github.com/codeh007/mtmpg/actions/runs/29632760924) |
| Job | [`Native authority`](https://github.com/codeh007/mtmpg/actions/runs/29632760924/job/88049560034)，ID `88049560034` |
| Remote source SHA | `449215707492e1eb322d23702a8f16d0975063fa` |
| Event | `push` |
| Job 时间 | `2026-07-18T05:44:03Z` 至 `2026-07-18T05:57:22Z`，13 分 19 秒 |
| 结论 | `success` |

Buildx 的实际命令同时包含 `--no-cache` 与 `--pull`，标签为 `mtmpg-native-ci:449215707492e1eb322d23702a8f16d0975063fa`。完整日志没有 `CACHED` build step。Graph 从固定 Rust 1.96.0 与 PostgreSQL 18.4 base digest拉取输入，重新运行 ShellCheck、Gitleaks、Cargo、Rustfmt、Clippy、依赖/许可证、官方 OAuth header/layout、production artifact、真实 PG18.4 OAuth/identity、ELF、runtime、filesystem 与 build-manifest 门禁。

## Image 与发布边界

最终 image 仍基于 `postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296`。Candidate runtime、完整 official-base filesystem diff、正式 module、MIT LICENSE、内部 build manifest、官方 entrypoint 和 `postgres` CMD 门禁全部成功。

本次 workflow 设置 `push:false`、`load:false`、provenance false 与 SBOM false。Build 结果只保留在 Buildx cache，ref 为 `builder-0f28f0f4-b266-4ec5-8be9-29c9ed351382/builder-0f28f0f4-b266-4ec5-8be9-29c9ed3513820/y6bour0qna3nojbv5vm1cnvsa`；没有 OCI distribution digest，不得把验证标签或 Buildx ref解释为已发布 candidate。Actions artifact、Release、mtmpg package、GHCR tag 与 `latest` 均为 0。

## Finding 摘要与可信 cache 起点

Cold run 的固定 Gitleaks source gate报告 Git history 与 worktree 各 0 findings，Docker context gate报告 0 findings。Run 结束后的完整 retrospective 再次物化本地/远端全部 history、工作树、Docker context、workflow source、27 份 workflow logs、Issue 与其他 GitHub surface；本地 history、工作树、远端 history和 surface bundle均为 `clean`，没有 rejected finding。

Post-cold inventory 包含 69 个新 cache entry，全部绑定 `refs/heads/issue-116-extract-pggomtm`，创建时间为 `2026-07-18T05:53:03Z` 至 `2026-07-18T05:57:15Z`。这些 entry 在零 inventory 后由唯一 cold run 的 `cache-to: type=gha,mode=max,scope=mtmpg-native-ci` 产生，合计 2,089,814,937 bytes。GitHub 不提供 cache 内容下载 API，因此 retrospective 仍如实标记它们为 `unresolved`；可信结论来自已证明的零起点、精确 source、单一 cold producer、0 findings 与完整成功 gate，而不是声称逐项扫描了 cache 内容。

本证据完成 bootstrap cache 清理与 cold authority，不代表 candidate、公开 GHCR、软件物料清单（SBOM）、attestation、Release 或 stable readiness。任务 7.8 仍须对最终功能分支完成 whole-branch review、source identity 和全部矩阵复核。
