## Why

`pggomtm` 已经形成独立的 Rust/PostgreSQL 18 OAuth validator 原型，却仍以未跟踪目录存在于 `gomtmui/native/pggomtm/`，导致源码权威、原生构建、产品应用和发布生命周期耦合在同一仓库。当前 `mtmpg` 只有初始提交，且 gomtmui 尚未正式消费该模块，因此现在是建立唯一独立源码权威、纠正 ABI 边界并固定可审计交付方式的最低风险窗口。原生 PostgreSQL、Rust 与多阶段 Docker 门禁又使本机完整构建耗时较长；验收若继续依赖本地 image 和终端状态，将缺少远端 commit、可复验日志与统一执行环境。

## What Changes

- **BREAKING**：把 `pggomtm` 的唯一源码、测试、Cargo lock、toolchain 和 Docker build authority 从 `gomtmui/native/pggomtm/` 硬切到 `mtmpg` 仓库根目录；不保留 submodule、subtree、vendored copy、镜像 fallback 或第二构建实现。
- 保留完整 `pgrx` 作为 PostgreSQL module magic、panic/error guard 和 allocator 安全层，删除冗余的直接 `pgrx-pg-sys` 依赖；OAuth callback ABI 改为从目标 PostgreSQL 官方 `libpq/oauth.h` 生成的最小 allowlist bindings，而不是手写结构体权威。Bindings必须单次materialize，校验的精确字节必须原样写入`OUT_DIR`并成为编译输入，禁用验证后的外部formatter或二次序列化，恶意`RUSTFMT`与`PATH/rustfmt`不得改变产物。
- 完成唯一离线 validator runtime：每个新OAuth backend从只读本地public JWKS/config建立不可变startup snapshot，严格验证ES256 database JWT、requested role与版本化`authn_id`并fail closed；轮换不依赖SIGHUP、网络fetch、既有backend reload或跨backend共享缓存，也不增加HTTP、SQL、SPI、私钥、在线introspection或认证fallback。
- 纠正 PostgreSQL 兼容门禁：构建、测试和发布元数据精确记录实际 minor 与 headers，但 runtime 依赖 PostgreSQL major module magic 和 OAuth validator magic，不以 `sversion == 180004` 阻断同一 PG18 stable line 的安全升级；每个拟部署 minor 仍必须先完成独立真实验证并由消费者固定镜像 digest。
- 建立仓库规范：README、MIT LICENSE、SECURITY、贡献/发布说明、最小权限 GitHub Actions、依赖更新策略，以及当前 private 阶段和后续 public 阶段各自可执行的默认分支与发布治理。
- 保持根`Dockerfile`为唯一构建图权威，并把远端 GitHub Actions 设为构建、测试和验收证据的唯一执行权威；本地命令只用于可选的快速诊断，不能完成OpenSpec task、consumer gate或发布门禁。
- 建立三条远端CI/CD lane：feature push/PR使用BuildKit缓存运行完整门禁，定时/人工冷门禁从clean checkout执行无缓存复验，受信发布lane只从已验证commit生成GHCR、manifest、SBOM与attestation；任一lane都必须关联精确远端source commit。
- 在远端CI、prerelease与GHCR门禁运行前，把精确已审查commit非force push到远端功能分支；首次workflow通过该分支的push或PR事件启动，`origin/main`在跨仓库验收前不接收产品实现。
- 在仓库公开前执行public-readiness门禁，覆盖全部Git refs与历史、当前工作树、Docker context、workflow日志/artifact、最终image及GitHub协作内容；真实secret必须先吊销或轮换并完成批准的历史处置，合成fixture只能按精确位置和理由分类。仓库visibility由所有者在门禁通过后手动切换，随后立即复核公开仓库的secret scanning、依赖安全与branch protection/ruleset。
- 以 GHCR 中基于精确官方 PostgreSQL image 的派生 runtime image 作为主要部署物；以 immutable GitHub Release tarball、checksum、release manifest、SBOM 和 provenance/attestation 作为辅助交付与取证材料。正式消费者必须按 digest 安装，不得依赖可变 tag 或在运行容器中热覆盖 `.so`。
- 发布版本化 consumer contract 与兼容元数据，让 gomtmui 只负责 issuer、delegation、数据库 role/RLS、executor 和平台编排，并通过固定 release/digest 消费 `pggomtm`。

## Capabilities

### New Capabilities

- `pggomtm-validator-module`: 定义独立 Rust PostgreSQL 18 OAuth validator 的官方 ABI 来源与最终字节同一性、离线 JWT/JWKS startup snapshot、role/identity、fail-closed 与 PG18 stable-line 兼容边界。
- `pggomtm-release-supply-chain`: 定义独立仓库治理、可重复 CI、GHCR runtime image、immutable Release、manifest、SBOM、provenance、版本兼容和消费者按 digest 安装契约。

### Modified Capabilities

无。

## Impact

- 仓库：`codeh007/mtmpg` 成为唯一源码与 release authority；`codeh007/gomtmui` 删除本地第二实现并改为制品消费者。
- Rust/PostgreSQL：保留固定 Rust、`pgrx`、JOSE 与 PG18 build baseline，引入官方 `oauth.h` 生成边界并修订 minor runtime gate。
- 交付：新增以远端commit为证据权威的 GitHub Actions、GHCR package、immutable GitHub Release、SBOM/provenance 和 release manifest；目标主机不安装 Rust/cargo，也不现场编译。
- 部署：测试和后续生产平台使用与目标 PostgreSQL 完全匹配、按 digest 固定的派生 image；JWKS/config 继续作为运行时只读材料，不进入发布物。
- GitHub：仓库当前保持private；公开是已经确认的后续方向，但只能由所有者在完整public-readiness门禁通过后执行，公开后的保护设置与首个stable合并语义必须重新核对。
- 安全：Git history、仓库、workflow日志/artifact、镜像和 release 不得包含 signing private key、API key、OAuth/database JWT、连接串、运行数据或 Cargo target cache。
