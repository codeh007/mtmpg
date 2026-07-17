## ADDED Requirements

### Requirement: mtmpg必须是pggomtm唯一源码与构建权威
`codeh007/mtmpg` SHALL 在仓库根目录维护唯一`pggomtm` crate、lockfile、toolchain、Docker build graph、tests和release workflow。根`Dockerfile` SHALL是唯一构建图权威，GitHub Actions SHALL是执行该构建图并形成任务、consumer与发布证据的唯一权威。gomtmui或其他消费者 MUST NOT保留vendored源码、submodule、subtree、第二Docker build或可运行fallback；Cargo `target/`、本地image、secret和运行数据 MUST NOT进入迁移或Git历史。

#### Scenario: 从gomtmui迁移原型
- **WHEN** 迁移现有`gomtmui/native/pggomtm`原型
- **THEN** mtmpg SHALL 用文件清单和源码checksum证明所有权威源码/测试已迁入、`target/`已排除且现有门禁行为未改变

#### Scenario: 消费者需要pggomtm
- **WHEN** gomtmui构建或部署PostgreSQL candidate
- **THEN** 它 SHALL 消费mtmpg发布的固定OCI digest和versioned contract，不得从本地Rust目录重新构建

#### Scenario: 远端CI需要可审计source
- **WHEN** 本地功能分支包含尚未被远端验证的实现、测试或workflow变更
- **THEN** 维护者 SHALL在运行CI或prerelease前把精确已审查commit非force push到远端功能ref，并让后续Actions证据关联该remote commit

#### Scenario: 本地构建成功
- **WHEN** 开发者本地`docker build`、测试命令或image tag成功
- **THEN** 结果 MAY用于定位问题，但 MUST NOT完成OpenSpec task、gomtmui consumer gate、release readiness或发布证据

### Requirement: 仓库必须具备明确安全与维护契约
仓库 SHALL 提供准确README、MIT LICENSE、SECURITY、贡献与发布说明、支持矩阵、升级策略和非`CREATE EXTENSION`安装边界。GitHub设置 SHALL 在当前visibility与套餐允许范围内采用最小权限Actions、full-SHA action引用、read-only默认workflow token、批准的merge策略与合并后删分支；无法启用的branch protection/rulesets MUST如实记录而不得伪报。仓库公开后 SHALL重新核对并启用实际可用的服务端安全与分支保护能力。

#### Scenario: 新维护者检查仓库
- **WHEN** 维护者只读取tracked文档与GitHub设置
- **THEN** 其 SHALL 能确定模块职责、支持PG/runtime、构建/测试命令、release流程、安全报告方式、部署方法与当前治理限制

#### Scenario: Workflow请求权限
- **WHEN** 普通CI或release workflow运行
- **THEN** 普通CI SHALL只获得read权限，release job SHALL只显式获得写Release、GHCR和attestation所需的最小权限

### Requirement: 仓库公开必须先通过public-readiness门禁
源码仓库在从private切换为public前 SHALL完成不回显敏感值的public-readiness审计，覆盖全部Git refs与历史、当前tracked与uncommitted文件、Docker build context、workflow源码和日志、Actions artifact、最终image、Release/package以及GitHub Issue/PR内容。真实secret MUST先吊销或轮换，再按明确批准处置历史；合成测试fixture只允许按精确路径、精确模式与理由分类，MUST NOT使用全局ignore。Visibility SHALL只由所有者在门禁通过后手动切换；源码公开 MUST NOT自动改变GHCR package visibility。

#### Scenario: 公开前扫描只有合成哨兵命中
- **WHEN** scanner只命中被测试用于证明secret门禁有效的确定性哨兵或公开fixture
- **THEN** 维护者 SHALL记录精确路径、模式和理由后继续门禁，且不得放宽其他文件、历史或secret类别

#### Scenario: 公开前发现真实secret
- **WHEN** 任一ref、历史、工作树、日志、artifact、image或协作内容包含真实credential或私密材料
- **THEN** 仓库 MUST保持private，相关credential SHALL先被吊销或轮换，并在明确批准的历史与远端处置完成后重新运行完整门禁

#### Scenario: 所有者完成公开切换
- **WHEN** public-readiness已经通过且所有者手动把源码仓库设为public
- **THEN** 维护者 SHALL立即复核secret scanning、dependency graph/alerts、branch protection/ruleset及首个stable source进入`main`的策略，并 SHALL独立决定GHCR package visibility

### Requirement: CI必须从固定输入重复验证native安全边界
每个批准的功能分支push、pull request与main push SHALL 对locked依赖运行Rustfmt、Clippy `-D warnings`、unit/integration tests、依赖与许可证审计、官方header binding/layout与最终字节同一性、真实PG18 loader/OAuth正负矩阵、动态依赖、secret和artifact隔离扫描。CI MUST从远端可达的精确source commit运行并使用固定Rust、pgrx、JOSE、PostgreSQL source/runtime digest和full-SHA actions；native认证依赖与PG minor更新 MUST只通过人工审查PR进入。首次workflow SHALL通过功能分支push或PR事件bootstrap，不得为获得人工dispatch而切换默认分支或复制第二workflow实现。

#### Scenario: 合法变更通过CI
- **WHEN** clean checkout在批准输入上完成全部静态与真实PostgreSQL门禁
- **THEN** CI SHALL 产生可关联远端source commit的成功证据，并且不得依赖开发者本地Cargo cache、仅本地分支或未跟踪文件

#### Scenario: 日常CI使用缓存
- **WHEN** 功能分支push或PR触发常规验证
- **THEN** workflow SHALL使用内容寻址的BuildKit/GitHub Actions cache和同ref并发取消运行唯一Docker build graph，且不得登录GHCR、读取发布secret或上传正式制品

#### Scenario: 冷门禁复验固定commit
- **WHEN** 人工dispatch、定时门禁或发布前门禁验证一个批准的远端commit
- **THEN** workflow SHALL从clean checkout对固定输入执行无缓存完整build graph并保存commit、run与验证摘要，且不得用常规缓存run冒充该cold authority证据

#### Scenario: Public fork提交PR
- **WHEN** 非受信fork代码触发公开仓库PR验证
- **THEN** workflow SHALL只使用GitHub-hosted临时runner和read-only token，不得使用`pull_request_target`、发布secret、GHCR写权限、Release写权限或attestation写权限

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
源码迁移、CI或测试feature完成 MAY产生带prerelease module version的alpha/RC，但该artifact MUST NOT直接增加stable tag或晋级同一digest。可晋级stable candidate SHALL来自远端功能分支上已经冻结最终`MAJOR.MINOR.PATCH`的commit，从该commit只构建一次并先只发布short-SHA身份。首个stable release SHALL要求production feature读取外部只读config/JWKS、真实PG18 OAuth allow/deny、role/identity、无gate artifact扫描和gomtmui对同一source/OCI digest的candidate集成全部通过；stable tag、Release与`latest` MUST引用已经验证的digest且不得触发重建。

#### Scenario: 仅ABI/JWT原型通过
- **WHEN** callback正常feature仍默认拒绝或依赖内置gate JWKS
- **THEN** workflow MAY从已推送功能分支commit发布明确alpha/RC制品验证pipeline，但 MUST NOT把该prerelease digest改标为stable、发布或更新`latest`、stable说明或生产可用声明

#### Scenario: 最终版本SHA candidate进入跨仓库验收
- **WHEN** 功能分支已冻结最终module version且正式runtime与native门禁通过
- **THEN** workflow SHALL从该远端commit只构建一次并只发布short-SHA candidate身份，gomtmui SHALL使用同一source与OCI digest完成E2E

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

#### Scenario: 源码仓库已经公开但package仍为private
- **WHEN** 所有者只改变mtmpg源码仓库visibility而没有独立批准GHCR package公开
- **THEN** package SHALL继续使用private pull边界，且workflow与文档不得把源码公开误报为package公开
