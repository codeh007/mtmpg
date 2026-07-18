## Why

`pggomtm` 的生产源码规模有限，但仓库已经累积了大量历史证据、流程脚本、策略自测和精确版本断言，维护成本反而超过业务实现。项目需要回到以最新兼容稳定技术栈、真实 PostgreSQL/OAuth 行为和可发布 mtmpg 版本为中心的最小开发与发布模型。

## What Changes

- **BREAKING**：删除 `SECURITY.md`、`CONTRIBUTING.md`、`examples/` 和历史性 `docs/evidence/`；清理所有失效引用，并把仍被真实集成测试使用的最小 OAuth fixture 移入测试支持代码。
- **BREAKING**：源码不再固定 Rust/PostgreSQL patch、Docker base digest、Cargo 依赖精确版本、GitHub Action commit SHA 或扫描工具 archive hash，也不再用测试批准这些字面值。
- Rust 跟随 `stable`，PostgreSQL 跟随当前支持的 PG18 major 内最新稳定 minor，Cargo 使用兼容版本范围，Actions 使用稳定 major tag；PostgreSQL major 升级仍作为显式 ABI 兼容变更处理。
- 每次 `main` CI 在开始时只解析一次实际工具链、依赖锁文件和 image digest；测试、production build 和 candidate 发布复用该解析结果，并把实际值记录到 release manifest、SBOM、provenance 与 attestation，而不是与源码内预设 hash 比较。
- 精简 `build.rs`、`scripts/`、`tests/`、Dockerfile、workflow 和维护文档；删除历史回溯、入口自测、实现字面量、精确 layer/config 相等和重复负向矩阵，只保留领域规则、ABI layout、真实 PostgreSQL OAuth、production module 与最终 image 启动验证。
- `main` 继续作为唯一源码集成线，所有构建、测试、临时 PostgreSQL 和 image 验证继续只在 GitHub Actions 执行；失败 commit 保留，但不得产生可消费 candidate 或 stable release。
- mtmpg SemVer 成为用户可见的发布与消费身份；OCI digest 仍由 CI 解析并绑定到证据，用于证明 candidate、gomtmui 验收和 stable promotion 指向同一不可变制品，但不作为上游技术栈版本写死在源码中。

## Capabilities

### New Capabilities

- `pggomtm-validator-module`：PostgreSQL 18 OAuth validator 的 ABI、离线验证、role、identity、失败边界和最新 PG18 minor 兼容要求。
- `pggomtm-release-supply-chain`：精简仓库、latest-compatible Actions CI/CD、版本化 PostgreSQL image、标准供应链证据和 gomtmui 消费契约。

### Modified Capabilities

无。

## Impact

- `codeh007/mtmpg` 的 `Cargo.toml`、`Cargo.lock` 策略、`rust-toolchain.toml`、`build.rs`、Dockerfile、Actions、测试、脚本、文档和 OpenSpec 将统一到精简且 latest-compatible 的模型。
- 已完成的固定 PG18.4/digest image 和严格 image readiness 工作不再代表目标状态；必须按新契约重新实现并取得远端 Actions 结果。
- Gomtmui 不构建 Rust module，按 mtmpg SemVer 选择 candidate/stable；验收证据在运行时记录其 OCI digest、mtmpg source 和 gomtmui source。
- 不修改生产数据库、生产配置或生产流量，也不恢复任何本地 Docker build/run 或原生重计算路径。
