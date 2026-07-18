## Why

`pggomtm` 的生产源码规模有限，但仓库已经累积了大量历史证据、流程脚本、策略自测和精确版本断言，维护成本反而超过业务实现。随后建立的每次 `main` 自动 candidate、定制 OCI evidence 和 gomtmui consumer workflow 又把阶段性探索固化进了 CI/CD；项目需要回到以真实 PostgreSQL/OAuth 行为、标准 GitHub 流程和可发布 mtmpg SemVer 为中心的最小模型。

## What Changes

- **BREAKING**：删除 `SECURITY.md`、`CONTRIBUTING.md`、`examples/` 和历史性 `docs/evidence/`；清理所有失效引用，并把仍被真实集成测试使用的最小 OAuth fixture 移入测试支持代码。
- **BREAKING**：源码不再固定 Rust/PostgreSQL patch、Docker base digest、Cargo 依赖精确版本、GitHub Action commit SHA 或扫描工具 archive hash，也不再用测试批准这些字面值。
- Rust 跟随 `stable`，PostgreSQL 跟随当前支持的 PG18 major 内最新稳定 minor，Cargo 使用兼容版本范围，Actions 使用稳定 major tag；PostgreSQL major 升级仍作为显式 ABI 兼容变更处理。
- PR 与 `main` push 复用一份只读 CI 定义，运行领域、ABI、真实 PostgreSQL 和最终 image 门禁；PR MAY 使用 GitHub 原生、受限的 auto-merge，维护者与 Agent 仍可直接非 force 推进 `main`。
- **BREAKING**：停止为每次 `main` push 生成 run-ID candidate，删除定制 `<version>.evidence` OCI bundle 和跨仓 same-digest promotion；只有显式 SemVer Git tag（含 prerelease）触发发布。
- Release 在一次 run 中解析实际工具链、依赖锁文件和 image digest，测试、production build 与发布复用该解析结果，并通过 GitHub Release、标准 OCI SBOM、provenance 与 attestation 记录实际值，而不是与源码内预设 hash 比较。
- 精简 `build.rs`、`scripts/`、`tests/`、Dockerfile、workflow 和维护文档；删除历史回溯、入口自测、实现字面量、精确 layer/config 相等和重复负向矩阵，只保留领域规则、ABI layout、真实 PostgreSQL OAuth、production module 与最终 image 启动验证。
- `main` 继续作为唯一源码集成线，所有构建、测试、临时 PostgreSQL 和 image 验证继续只在 GitHub Actions 执行；失败 commit 保留，且没有 SemVer tag 就不得产生 release。
- 公开 image 统一命名为 `ghcr.io/codeh007/mtmpg:<semver>`。mtmpg SemVer 是用户可见身份，OCI digest 是该 release 的机器证据，但不作为上游技术栈版本写死在源码中。
- **BREAKING**：gomtmui 删除专用 mtmpg consumer workflow 与测试 harness，只在 Compose/平台契约中消费版本化 image；TLS、sub2api、pgAdmin、ACL/RLS 与真实授权集成归 gomtmui 自身 change，不再阻塞 mtmpg release。

## Capabilities

### New Capabilities

- `pggomtm-validator-module`：PostgreSQL 18 OAuth validator 的 ABI、离线验证、role、identity、失败边界和最新 PG18 minor 兼容要求。
- `pggomtm-release-supply-chain`：精简仓库、PR/main 可复用 CI、SemVer tag release、版本化 PostgreSQL image、标准供应链证据和最小消费契约。

### Modified Capabilities

无。

## Impact

- `codeh007/mtmpg` 的 `Cargo.toml`、`Cargo.lock` 策略、`rust-toolchain.toml`、`build.rs`、Dockerfile、Actions、测试、脚本、文档和 OpenSpec 将统一到精简且 latest-compatible 的模型。
- 旧 `ghcr.io/codeh007/mtmpg-postgres` run-ID candidate 与 OCI evidence 只保留为历史探索结果，不得作为新 release 或 gomtmui 输入；首个合格版本必须从新 tag workflow 发布到 `ghcr.io/codeh007/mtmpg`。
- Gomtmui 不构建 Rust module，也不维护专用跨仓 consumer harness；它按 mtmpg SemVer 配置 Compose，并在真正实现数据库授权能力时由自己的领域 change 验证平台集成。
- 不修改生产数据库、生产配置或生产流量，也不恢复任何本地 Docker build/run 或原生重计算路径。
