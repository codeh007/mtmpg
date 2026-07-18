# Issue #116：candidate artifact readiness 门禁证据

本证据于 2026-07-18（UTC）采集，用于完成 OpenSpec 任务 7.4。完成结论来自精确
远端 GitHub Actions source；本地 Docker image 只作为同源补充复验，不替代远端门禁。

## 远端执行身份

| 字段 | 值 |
| --- | --- |
| Run | [29630498602](https://github.com/codeh007/mtmpg/actions/runs/29630498602) |
| Job | [`Native authority`](https://github.com/codeh007/mtmpg/actions/runs/29630498602/job/88042992331)，ID `88042992331` |
| Remote head SHA | `a7560ddfbc6a287eb002dd07d2e7da54dc2bfe22` |
| Event | `push` |
| Job 时间 | `2026-07-18T04:24:22Z` 至 `2026-07-18T04:31:24Z`，7 分 02 秒 |
| 结论 | `success` |
| Actions artifact | 0 |

远端 run 先完成精确 checkout、clean source identity 与追溯式 public-readiness，再由根
`Dockerfile` 执行唯一 native build graph。固定 ShellCheck、artifact fixture、Gitleaks、
Cargo、官方 header、Rust、Clippy、真实 PostgreSQL 18.4 与最终 image 门禁全部成功。

## ELF 与运行时边界

正式 module 使用唯一 production feature `pg18` 构建。`verify-elf` 在生成内部 build
manifest 前强制以下闭集：

- ELF64、little-endian、`DYN`、x86-64；
- `DT_NEEDED` 只能是 `ld-linux-x86-64.so.2`、`libc.so.6`、`libgcc_s.so.1`；
- 动态 export 只能是 `Pg_magic_func` 与 `_PG_oauth_validator_module_init`。

远端 `candidate-runtime-gate` 成功证明：

- module 精确位于真实 `pg_config --pkglibdir` 的
  `/usr/lib/postgresql/18/lib/pggomtm.so`；
- runtime 是 `PostgreSQL 18.4 (Debian 18.4-1.pgdg12+1)`；
- machine、package architecture 与 libc 分别是 `x86_64`、`amd64` 与 glibc；
- `ldd` 没有 unresolved dependency，并包含全部批准依赖；
- `/usr/local/bin/docker-entrypoint.sh` 是可执行的官方 entrypoint；
- module、LICENSE、manifest 为 `0644 root:root`，文档目录为 `0755 root:root`。

## 最终 filesystem 与 Docker 配置

门禁分别从同一个 pinned official base 和 `candidate-content` 生成完整 root filesystem
inventory。每条记录包含逻辑路径、类型、mode、uid、gid，以及文件 SHA-256 或 symlink
target。任何 official base 记录被删除、改写、换类型、换权限或换所有者都会失败。

Candidate 相对 official base 只允许以下四条新增记录：

| 路径 | 类型 | Mode | Owner |
| --- | --- | --- | --- |
| `/usr/lib/postgresql/18/lib/pggomtm.so` | file | `0644` | `0:0` |
| `/usr/share/doc/pggomtm` | directory | `0755` | `0:0` |
| `/usr/share/doc/pggomtm/LICENSE` | file | `0644` | `0:0` |
| `/usr/share/doc/pggomtm/build-manifest.json` | file | `0644` | `0:0` |

`verify-dockerfile` 还把最终 stage 收敛为完整四指令闭集：从已验证的
`candidate-content` 开始，只运行只读 gate proof，随后显式保留
`ENTRYPOINT ["docker-entrypoint.sh"]` 与 `CMD ["postgres"]`。Fixture 已证明在 final
stage 增加额外 `COPY` 会失败，因此通过 gate 后不能再追加文件系统内容。

## Image 内 build manifest

内部 manifest schema 为 `pggomtm-build-manifest/v1`，只允许以下构建前事实：

- module 名称、SemVer、唯一 `pg18` feature、安装路径与实际 `.so` SHA-256；
- Rust target/version，固定 pgrx 与 JOSE 实现/version；
- PostgreSQL source/header/bindings/runtime base identity；
- Linux amd64 glibc platform；
- MIT SPDX、安装路径与仓库权威 LICENSE SHA-256。

Manifest 必须是 canonical compact JSON，必须与唯一 production build identity、实际
module bytes 和 MIT LICENSE bytes一致。Schema 拒绝额外字段、OCI/image digest 字段、
`sha256:<OCI digest>` 值、placeholder 与 `TBD`。因此内部 manifest 不包含自身尚未产生的
OCI digest；外部 release manifest 仍由后续任务在 image digest 产生后生成。

## 远端差异发现与处置

首次实现 run
[29629689316](https://github.com/codeh007/mtmpg/actions/runs/29629689316) 正确失败；诊断 run
[29629913391](https://github.com/codeh007/mtmpg/actions/runs/29629913391) 记录 GitHub Buildx
docker-container driver 把 `COPY --chmod=0644` 自动创建的
`/usr/share/doc/pggomtm` 父目录设为 `0644`。本地默认 builder 当时生成 `0755`，所以本地
成功不能替代远端结果。

根因修复在 candidate stage 先显式创建 `0755 root:root` 文档目录，再复制两个 `0644`
文件。最终成功 run 使用同一 Buildx driver 通过 runtime 与完整 filesystem diff，没有
放宽 mode 断言或增加兼容分支。

## 本地同源补充复验

提交前完整 Docker build 成功并生成本地 image ID
`sha256:f0bd01bf023b8f9043627ae2b3bcb1d5ebadccfcb999d4c6d4ae0e6f791f6004`。
该本地 image 中 module SHA-256 为
`f51d149eafb1d61610c4a44cb2324975a699113c2da667a696c75de1a5a995aa`，LICENSE SHA-256
为 `d00a4102edfa58b1d8c98e04635821b84ef7e86baad33d8888f61613322717a3`。Image 配置为
Linux amd64、`docker-entrypoint.sh` 与 `postgres`。这些值只用于本地复核，不是已发布
OCI distribution digest。

本证据只完成 candidate/stable 共用的 artifact readiness 门禁。该 run 使用普通 GitHub
Actions cache，`load: false`、`push: false`，没有发布 OCI image、tag、Release、SBOM、
provenance 或 attestation，也不是任务 7.7 或 10.1 的 cold authority。
