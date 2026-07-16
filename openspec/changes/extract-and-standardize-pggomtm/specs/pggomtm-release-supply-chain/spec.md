## ADDED Requirements

### Requirement: mtmpg必须是pggomtm唯一源码与构建权威
`codeh007/mtmpg` SHALL 在仓库根目录维护唯一`pggomtm` crate、lockfile、toolchain、Docker build、tests和release workflow。gomtmui或其他消费者 MUST NOT保留vendored源码、submodule、subtree、第二Docker build或可运行fallback；Cargo `target/`、本地image、secret和运行数据 MUST NOT进入迁移或Git历史。

#### Scenario: 从gomtmui迁移原型
- **WHEN** 迁移现有`gomtmui/native/pggomtm`原型
- **THEN** mtmpg SHALL 用文件清单和源码checksum证明所有权威源码/测试已迁入、`target/`已排除且现有门禁行为未改变

#### Scenario: 消费者需要pggomtm
- **WHEN** gomtmui构建或部署PostgreSQL candidate
- **THEN** 它 SHALL 消费mtmpg发布的固定OCI digest和versioned contract，不得从本地Rust目录重新构建

### Requirement: 仓库必须具备明确安全与维护契约
仓库 SHALL 提供准确README、MIT LICENSE、SECURITY、贡献与发布说明、支持矩阵、升级策略和非`CREATE EXTENSION`安装边界。GitHub设置 SHALL 在当前套餐允许范围内采用最小权限Actions、full-SHA action引用、read-only默认workflow token、批准的merge策略与合并后删分支；无法启用的branch protection/rulesets MUST如实记录而不得伪报。

#### Scenario: 新维护者检查仓库
- **WHEN** 维护者只读取tracked文档与GitHub设置
- **THEN** 其 SHALL 能确定模块职责、支持PG/runtime、构建/测试命令、release流程、安全报告方式、部署方法与当前治理限制

#### Scenario: Workflow请求权限
- **WHEN** 普通CI或release workflow运行
- **THEN** 普通CI SHALL只获得read权限，release job SHALL只显式获得写Release、GHCR和attestation所需的最小权限

### Requirement: CI必须从固定输入重复验证native安全边界
每个pull request与main push SHALL 对locked依赖运行Rustfmt、Clippy `-D warnings`、unit/integration tests、依赖与许可证审计、官方header binding/layout、真实PG18 loader/OAuth正负矩阵、动态依赖、secret和artifact隔离扫描。CI MUST使用固定Rust、pgrx、JOSE、PostgreSQL source/runtime digest和full-SHA actions；native认证依赖与PG minor更新 MUST只通过人工审查PR进入。

#### Scenario: 合法变更通过CI
- **WHEN** clean checkout在批准输入上完成全部静态与真实PostgreSQL门禁
- **THEN** CI SHALL 产生可关联source commit的成功证据，并且不得依赖开发者本地Cargo cache或未跟踪文件

#### Scenario: ABI或供应链门禁失败
- **WHEN** header/layout、OAuth矩阵、lock审计、动态依赖、secret扫描或artifact隔离任一失败
- **THEN** CI SHALL fail closed且不得发布image、release或成功attestation

### Requirement: GHCR派生PostgreSQL image必须是主要部署物
Release SHALL 发布基于精确官方`postgres:<minor>-bookworm@sha256:<digest>`的`ghcr.io/codeh007/mtmpg-postgres` image，只把正式`libpggomtm.so`、license和非敏感manifest加入真实`pg_config --pkglibdir`。Image MUST保持官方entrypoint，MUST NOT包含JWKS/config、私钥、token、数据库data、gomtmui源码、Rust toolchain、Cargo target或测试gate。

#### Scenario: 构建runtime image
- **WHEN** release workflow从同一clean checkout完成native与真实PG测试
- **THEN** 最终image SHALL 在固定PG/runtime上加载module，并 SHALL 发布短SHA与对应version标签及不可变OCI digest；只有stable release SHALL额外发布`latest`发现别名

#### Scenario: 正式环境选择image
- **WHEN** gomtmui candidate或后续环境部署pggomtm
- **THEN** 配置 SHALL 使用完整OCI digest，且不得只引用`latest`、release tag或本地`gomtm-pggomtm:*`标签

### Requirement: GitHub Release必须不可变且包含完整取证材料
每个正式Git tag SHALL 对应一个immutable GitHub Release，包含按target命名的`.so` bundle、`SHA256SUMS`、license、SBOM、build provenance/attestation和`release-manifest.json`。Actions临时artifact MUST NOT作为正式分发入口；release创建后tag、asset与manifest MUST NOT被覆盖或替换。

#### Scenario: 发布一个PG18.4 amd64变体
- **WHEN** release workflow发布已验证版本
- **THEN** Release SHALL 包含`pggomtm-<version>-pg18.4-linux-amd64-glibc.tar.zst`及可验证checksum、SBOM、provenance和与OCI digest一致的manifest

#### Scenario: 尝试覆盖既有release
- **WHEN** 相同tag或version请求上传不同二进制、manifest或image内容
- **THEN** 发布流程 SHALL 拒绝并要求新版本，而不是产生同名可变制品

### Requirement: Release manifest必须版本化跨仓库消费契约
`release-manifest.json` SHALL 至少记录source commit、module version、database-token contract version、authn-id version、Rust/pgrx/JOSE、PostgreSQL build/test minor与`PG_VERSION_NUM`、OAuth header digest、base image digest、target、arch、libc、`.so` digest、OCI digest和验证矩阵。gomtmui SHALL 在更新消费digest前验证这些字段与其issuer/profile/platform契约一致。

#### Scenario: gomtmui升级pggomtm
- **WHEN** gomtmui PR选择一个新的mtmpg release digest
- **THEN** consumer gate SHALL 校验manifest、contract version、PG variant、artifact digest和正负向token/identity向量后才允许candidate部署

#### Scenario: Contract或runtime不兼容
- **WHEN** token/authn contract、PG minor、arch、libc、base digest或验证状态与目标平台不匹配
- **THEN** gomtmui SHALL 拒绝该artifact且不得回退到本地build或旧协议适配器

### Requirement: Stable发布必须晚于正式runtime与跨仓库验收
源码迁移、CI或测试feature完成 MAY只产生short-SHA image或GitHub prerelease。首个stable release SHALL要求production feature读取外部只读config/JWKS、真实PG18 OAuth allow/deny、role/identity、无gate artifact扫描和gomtmui candidate集成全部通过；同一tag MUST只构建一次，后续环境 MUST晋级相同OCI digest。

#### Scenario: 仅ABI/JWT原型通过
- **WHEN** callback正常feature仍默认拒绝或依赖内置gate JWKS
- **THEN** workflow MAY从已推送功能分支commit发布明确alpha/sha制品，但 MUST NOT发布或更新`latest`、stable说明或生产可用声明

#### Scenario: Stable门禁全部通过
- **WHEN** mtmpg native矩阵和gomtmui candidate E2E都引用同一source与OCI digest并成功
- **THEN** 已验证功能分支 SHALL先以fast-forward把相同source commit推进到`main`，workflow SHALL 为已验证的同一OCI digest创建首个stable immutable Release与`latest`别名，且stable发布与后续晋级不得重新构建

### Requirement: 安装、轮换与rollback必须以不可变image为单位
容器环境 SHALL 通过切换到新的完整OCI digest并滚动重建PostgreSQL backend安装或升级pggomtm，JWKS/config SHALL作为运行时只读mount独立轮换。系统 MUST NOT热覆盖已加载`.so`、在目标主机现场编译或把release bundle持久化进gomtmui源码。Rollback SHALL切回上一已验证digest，不得恢复第二份源码或认证fallback。

#### Scenario: Candidate升级module
- **WHEN** 新release通过manifest和candidate smoke
- **THEN** 平台 SHALL 拉取固定digest、重建服务并验证实际server/module/OAuth身份后再完成切换

#### Scenario: 新release运行失败
- **WHEN** loader、OAuth、identity或platform smoke在切换后失败
- **THEN** 平台 SHALL停止新能力并滚动切回上一已验证digest，mtmpg SHALL通过新版本修复前进

### Requirement: 发布与部署不得泄漏secret或运行数据
仓库、Git history、workflow日志、BuildKit cache、image layer、Release asset、SBOM、provenance和manifest MUST NOT包含signing private key、API key、OAuth/database JWT、authorization code、数据库连接串、`.env`、PostgreSQL data、session或真实JWKS working copy。私有GHCR pull credential SHALL只存在于部署secret authority并只授予读取所需package的权限。

#### Scenario: 扫描发布物
- **WHEN** release workflow对source、history、log、image filesystem、bundle、SBOM和attestation执行泄漏扫描
- **THEN** 任一敏感材料命中 SHALL 阻止发布且不得用删除日志或弱化规则绕过

#### Scenario: 私有环境拉取GHCR image
- **WHEN** 部署主机需要读取private package
- **THEN** 它 SHALL 使用运行时注入的read-only package credential，且credential不得写入Compose、image、manifest或Release
