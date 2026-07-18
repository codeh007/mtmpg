# pggomtm

`pggomtm` 是 PostgreSQL 18 OAuth validator Rust/pgrx module。本仓库是源码、测试、PostgreSQL image 和 release 的唯一权威；公开 `main` 是唯一 CI/CD 源码线，但不代表 stable。

## 产品边界

- PostgreSQL 通过 `oauth_validator_libraries` 加载 `pggomtm.so`。
- Module 导出 `PG_MODULE_MAGIC` 与 `_PG_oauth_validator_module_init`，不是 SQL extension，不需要 control、versioned SQL 或 `CREATE EXTENSION`。
- 每个新 OAuth backend 从 `/etc/pggomtm/validator.json` 和 `/etc/pggomtm/jwks.json` 建立只读离线 snapshot，不执行 HTTP、DNS、SQL、SPI 或在线 introspection。
- Validator 只接受 ES256 database JWT，校验唯一 issuer/audience、`database` scope、30 至 300 秒 TTL、closed profile-role 和版本化 identity。
- 认证失败保持 fail closed，服务端只记录稳定 reason-code，不记录 token、JWKS、配置或身份原文。

详细运行契约见：

- [Runtime 配置](docs/runtime-configuration.md)
- [认证失败可见性](docs/authentication-failures.md)
- [发布与兼容](docs/release-and-compatibility.md)

## 支持范围

源码面向 PostgreSQL 18 major。每次 GitHub Actions run 使用 PG18 稳定通道解析当前最新 minor，并让 ABI 生成、真实 PostgreSQL 测试和最终 image 使用同一解析结果。跨 PostgreSQL major 需要显式源码、feature、路径和 ABI 变更。

当前平台边界是 Linux amd64、Debian bookworm/glibc 和 Rust stable。`pg18` Cargo feature表示目标 major，不批准任何未经过当次远端测试的制品。

## GitHub Actions

`.github/workflows/ci.yml`是 PR、`main` push 与 release 调用复用的验证入口，负责依赖解析、Rustfmt、Clippy、Cargo tests、C/Rust ABI、真实 PG18 OAuth、production module 和最终 image。PR 与`main`只读运行且不上传release材料；只有SemVer tag workflow调用时才短暂传递同一run已验证的OCI archive。根`Dockerfile`只构建production image，不承载测试或扫描器。

维护者和 Agent 可以把范围明确的 commit 直接非 force 推送到 `main`。失败 commit 保留并通过后续 commit 修复；没有显式 SemVer tag 的 run 不得发布 image、GitHub Release 或 attestation。

本地工作区只用于源码/OpenSpec编辑、Git操作和只读调查。不得在本地运行 Cargo、原生编译、Docker build/run、临时 PostgreSQL 或 image 检查。

查询远端结果：

```bash
gh run list --repo codeh007/mtmpg --workflow ci.yml --branch main --limit 5
gh run view <run-id> --repo codeh007/mtmpg --log-failed
```

## 版本与发布

mtmpg 使用 SemVer 作为用户可见身份：

- Prerelease：`ghcr.io/codeh007/mtmpg:<prerelease>`，例如 `0.1.0-rc.1`
- Stable：`ghcr.io/codeh007/mtmpg:<stable>`
- `latest`：只在完整 stable release 成功后更新

`.github/workflows/release.yml`只响应严格校验通过的 `v<semver>` tag。它调用同一 `ci.yml`，下载该 run 已验证的 OCI archive后推送一次，不在publish job中运行Cargo或Docker build。Prerelease与stable分别从自己的不可变tag完整解析、测试、构建和发布。

每个release的长期权威是版本化公开GHCR image、GitHub Release、Cargo.lock、`resolved-inputs.json`、release manifest、checksums、SPDX SBOM、provenance与GitHub attestation。Actions artifact只用于同一次run内传递。当前stable见 [v0.1.0](https://github.com/codeh007/mtmpg/releases/tag/v0.1.0)，首个prerelease仍保留在 [v0.1.0-rc.1](https://github.com/codeh007/mtmpg/releases/tag/v0.1.0-rc.1)；发布进度由 [mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 和 active OpenSpec change 跟踪。

## 维护入口

- [维护规则](MAINTAINERS.md)
- [Agent 规则](AGENTS.md)
- [MIT License](LICENSE)
