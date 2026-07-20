# pggomtm

`mtmpg`维护两个隔离Rust产品：PostgreSQL 18 OAuth validator `pggomtm`，以及把受信delegation转换为短期database JWT与libpq OAuth连接的私网executor。本仓库是两者源码、测试、image和release的唯一权威；公开`main`是唯一CI/CD源码线，但不代表stable。

## 产品边界

- PostgreSQL 通过 `oauth_validator_libraries` 加载 `pggomtm.so`。
- Module 导出 `PG_MODULE_MAGIC` 与 `_PG_oauth_validator_module_init`，不是 SQL extension，不需要 control、versioned SQL 或 `CREATE EXTENSION`。
- 每个新 OAuth backend 从 `/etc/pggomtm/validator.json` 和 `/etc/pggomtm/jwks.json` 建立只读离线 snapshot，不执行 HTTP、DNS、SQL、SPI 或在线 introspection。
- Validator 只接受 ES256 database JWT，校验唯一 issuer/audience、`database` scope、30 至 300 秒 TTL、closed profile-role 和版本化 identity。V0.2.x只允许`ordinary`、`business_admin`和`database_developer`，且`db_profile`、`db_role`与startup role必须精确同名。
- 认证失败保持 fail closed，服务端只记录稳定 reason-code，不记录 token、JWKS、配置或身份原文。
- `executor/`只接受versioned HTTPS/HMAC单statement请求，以per-`PGconn` auth hook隔离一次性JWT，并使用extended protocol、service-owned transaction、预算和取消；它不提供公开token endpoint或认证fallback。
- Validator与executor共享唯一database-token contract，但保持独立package、image和版本：validator使用`v<semver>`，executor使用`executor-v<semver>`。

详细运行契约见：

- [Runtime 配置](docs/runtime-configuration.md)
- [认证失败可见性](docs/authentication-failures.md)
- [发布与兼容](docs/release-and-compatibility.md)

## 支持范围

源码面向 PostgreSQL 18 major。每次 GitHub Actions run 使用 PG18 稳定通道解析当前最新 minor，并让 ABI 生成、真实 PostgreSQL 测试和最终 image 使用同一解析结果。跨 PostgreSQL major 需要显式源码、feature、路径和 ABI 变更。

当前平台边界是 Linux amd64、Debian bookworm/glibc 和 Rust stable。`pg18` Cargo feature表示目标 major，不批准任何未经过当次远端测试的制品。

## GitHub Actions

`.github/workflows/ci.yml`是PR、`main` push与两个release入口复用的验证权威，负责一次依赖解析、Rustfmt、Clippy、Cargo tests、C/Rust ABI、真实PG18 OAuth、executor并发隔离与各自final image。PR与`main`只读运行且不上传release材料；只有明确product的tag workflow调用时才短暂传递同一run已验证的OCI archive。两个Dockerfile只构建各自production image，不承载测试或扫描器。

维护者和 Agent 可以把范围明确的 commit 直接非 force 推送到 `main`。失败 commit 保留并通过后续 commit 修复；没有显式 SemVer tag 的 run 不得发布 image、GitHub Release 或 attestation。

本地工作区只用于源码/OpenSpec编辑、Git操作和只读调查。不得在本地运行 Cargo、原生编译、Docker build/run、临时 PostgreSQL 或 image 检查。

查询远端结果：

```bash
gh run list --repo codeh007/mtmpg --workflow ci.yml --branch main --limit 5
gh run view <run-id> --repo codeh007/mtmpg --log-failed
```

## 版本与发布

mtmpg 使用 SemVer 作为用户可见身份：

- Validator prerelease/stable：`ghcr.io/codeh007/mtmpg:<semver>`，由`v<semver>` tag发布。
- Executor prerelease/stable：`ghcr.io/codeh007/mtmpg-executor:<semver>`，由`executor-v<semver>` tag发布。
- 两个product的`latest`只在各自完整stable release成功后更新；消费者使用明确SemVer。

`.github/workflows/release.yml`只响应严格校验通过的 `v<semver>` tag。它调用同一 `ci.yml`，下载该 run 已验证的 OCI archive后推送一次，不在publish job中运行Cargo或Docker build。Prerelease与stable分别从自己的不可变tag完整解析、测试、构建和发布。

每个release的长期权威是版本化公开GHCR image、GitHub Release、Cargo.lock、`resolved-inputs.json`、release manifest、checksums、SPDX SBOM、provenance与GitHub attestation。Actions artifact只用于同一次run内传递。当前stable见 [v0.2.0](https://github.com/codeh007/mtmpg/releases/tag/v0.2.0)；[v0.1.0](https://github.com/codeh007/mtmpg/releases/tag/v0.1.0)与首个prerelease [v0.1.0-rc.1](https://github.com/codeh007/mtmpg/releases/tag/v0.1.0-rc.1)作为不可变历史保留。发布进度由 [mtmpg #1](https://github.com/codeh007/mtmpg/issues/1) 和 active OpenSpec change 跟踪。

## 维护入口

- [维护规则](MAINTAINERS.md)
- [Agent 规则](AGENTS.md)
- [MIT License](LICENSE)
