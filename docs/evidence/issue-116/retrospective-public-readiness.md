# Issue #116：公开远端追溯扫描证据

本证据于 2026-07-18（UTC）采集，用于完成 OpenSpec 任务 7.6。扫描覆盖当前公开远端与本地精确 source；所有已物化 surface 均为 0 findings，唯一未解析项是 207 个无法下载内容的 GitHub Actions cache entry。任务 7.7 必须先删除这些 cache，再从 clean checkout 完成无缓存 bootstrap cold build。

## Source identity

| 字段 | 值 |
| --- | --- |
| Repository | [`codeh007/mtmpg`](https://github.com/codeh007/mtmpg) |
| Visibility | `public` |
| Default branch | `main` |
| Local HEAD | `27fa602dddb8ce152a3e71610ec179bb4449e322` |
| Remote feature HEAD | `27fa602dddb8ce152a3e71610ec179bb4449e322` |
| Remote `main` HEAD | `453b2a71c8f98b8824278f2d469683626c6d7e71` |
| Candidate image | 未提供，远端 package 不存在 |

扫描时本地工作树为 clean，且本地 HEAD 与 `refs/heads/issue-116-extract-pggomtm` 完全一致。远端 mirror 物化了两个 ref，扫描没有用默认分支范围代替全部远端 history。

## 固定扫描器与命令

Gitleaks 固定为 `8.30.1`。安装器验证官方 Linux x64 归档 SHA-256 `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb`，扫描强制 `--redact=100`、`--gitleaks-ignore-path /dev/null` 与 `--ignore-gitleaks-allow`。报告仅包含脱敏状态、计数和 finding 元数据，不保存或回显 secret 值。

```bash
scan_root="$(mktemp -d)"
scripts/public-readiness install-gitleaks "${scan_root}/bin"
PGGOMTM_GITLEAKS_BIN="${scan_root}/bin/gitleaks" \
  scripts/public-readiness retrospective codeh007/mtmpg
```

## Surface inventory

| Surface | 状态 | 数量 |
| --- | --- | ---: |
| Remote Git refs/history | `materialized` | 2 |
| Tracked、ignored 与 uncommitted 工作树 | `materialized` | 1 |
| Docker context | `materialized` | 1 |
| Workflow source | `materialized` | 1 |
| Workflow logs | `materialized` | 23 |
| Actions artifacts | `absent` | 0 |
| Actions caches | `unresolved` | 207 |
| Releases/packages | `absent` | 0 |
| GitHub Issues 与评论 | `materialized` | 1 |
| GitHub Pull Requests、reviews、files 与评论 | `absent` | 0 |
| Candidate image | `absent` | 0 |

Scanner 分别报告本地全部 Git history、当前工作树、远端全部 refs/history 和已物化 surface bundle 为 `clean`，每项均为 0 findings。Workflow logs 全部成功下载；Actions artifact、Release/package、Pull Request 和 candidate image 没有缺失下载造成的未解析状态。

## 结论与后续边界

追溯命令以状态 `1` 退出，原因仅为 Actions caches 的 207 个 `unresolved` 记录。GitHub API 只公开 cache metadata，不提供逐项下载内容，因此本任务没有把 metadata 扫描解释为 cache 内容已安全，也没有增加 allowlist 或删除日志来获得成功状态。

本次没有真实 secret finding，因此不需要触发 credential 吊销、轮换或 history 处置。任务 7.7 仍须删除全部 207 个旧 cache entry，并在 cache inventory 为零后触发精确最终 remote HEAD 的无缓存 cold run。该 cold run 成功前，本证据不代表 cold authority、`main` bootstrap、candidate、Release 或 stable readiness。
