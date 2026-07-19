# pggomtm-release-supply-chain Specification

## Purpose
定义mtmpg作为pggomtm唯一源码与发布权威的仓库边界，以及只读CI、SemVer发布、标准供应链证明、稳定输入解析和gomtmui最小消费契约。

## Requirements
### Requirement: mtmpg必须是精简的唯一源码与发布权威
`codeh007/mtmpg` SHALL维护唯一`pggomtm` crate、测试、CI和PostgreSQL image定义。仓库 MUST删除`SECURITY.md`、`CONTRIBUTING.md`、`examples/`、历史性`docs/evidence/`及失效引用；仍被真实测试使用的最小fixture SHALL位于`tests/support/`，不得伪装成用户示例。

仓库 MUST NOT保留历史回溯collector、脚本入口自测、Dockerfile/workflow字面量门禁或重复的阶段性说明。生产`runtime_config` MUST保留，但其测试 SHALL按真实风险裁剪并采用不产生误导目录的组织方式。Gomtmui与其他消费者 MUST NOT保留Rust源码副本、submodule、第二Dockerfile、本地image fallback或现场编译路径。

#### Scenario: 精简仓库
- **WHEN** 维护者完成仓库清理
- **THEN** 每个保留的文档、测试、fixture和辅助入口 SHALL直接服务当前产品、远端CI或发布契约，历史过程只由Git、Actions和Release记录保存

#### Scenario: gomtmui消费pggomtm
- **WHEN** gomtmui部署带有pggomtm的PostgreSQL
- **THEN** 它 SHALL引用mtmpg发布的版本化image，不得重新构建module

### Requirement: PR、Main与release必须分离
远端`main` SHALL作为唯一源码持续集成线，并 SHALL允许维护者或Agent直接非force推进。Pull Request与`main` push SHALL复用同一只读CI定义；workflow MUST NOT包含临时分支名称或Issue编号trigger。没有显式SemVer Git tag的commit MUST NOT发布image、GitHub Release或attestation。

仓库 SHALL以required CI和GitHub原生auto-merge处理明确受信任的owner、Agent或批准的Dependabot PR，并 MUST要求外部PR经过人工批准。仓库 MUST NOT使用`pull_request_target`或其他高权限自定义脚本自动合并任意外部代码。指定维护者/Agent SHALL保留直接推进`main`的规则绕过能力。

`main` SHALL允许暂时处于CI失败状态；失败commit MUST保留在历史中并由后续commit修复。Git历史、SemVer tag和已发布引用 MUST NOT force rewrite。

#### Scenario: 直接更新main
- **WHEN** 维护者或Agent向`main`非force推送一个源码commit
- **THEN** GitHub Actions SHALL只读验证该精确commit，且没有SemVer tag时不得发布任何release制品

#### Scenario: 受信任PR自动合并
- **WHEN** owner、Agent或批准的Dependabot PR通过required CI并启用GitHub原生auto-merge
- **THEN** GitHub SHALL按仓库规则合并该PR，workflow不得自行checkout并以高权限执行合并

#### Scenario: Main验证失败
- **WHEN** `main`上的resolve、领域测试、ABI、真实PostgreSQL或image任一门禁失败
- **THEN** 该commit SHALL继续作为源码历史存在，但不得创建package、tag、Release或attestation

#### Scenario: 外部贡献者提交PR
- **WHEN** 公开Pull Request面向`main`
- **THEN** workflow SHALL执行只读验证且 MUST NOT取得package、Release、attestation、自动合并或跨仓写权限

### Requirement: 重计算必须只在GitHub Actions执行
mtmpg的依赖解析、开发测试、原生编译、临时PostgreSQL cluster、Docker build/run和最终image检查 SHALL只在GitHub-hosted Actions runner执行。共享本地工作区 MUST NOT执行这些重计算；本地只允许源码/规划编辑、Git/OpenSpec操作、只读调查和对已知对象的精确清理。

仓库 MUST NOT以本地image、container、tag或终端日志作为task、consumer或release证据，也 MUST NOT使用宽泛Docker prune清理共享主机。

#### Scenario: Agent处理实现任务
- **WHEN** Agent需要验证mtmpg源码、PostgreSQL或image变更
- **THEN** Agent SHALL提交到`main`并读取精确Actions run，不得用本地构建结果完成任务

#### Scenario: Actions运行验证
- **WHEN** `main` push触发远端workflow
- **THEN** runner SHALL创建并清理本次run所需的Cargo、container、PGDATA、config和fixture资源

### Requirement: 源码必须声明最新兼容稳定输入
mtmpg源码 SHALL使用Rust `stable`、PG18 major内最新稳定minor、Cargo兼容版本范围和GitHub Actions稳定major tag。Dockerfile、toolchain、Cargo manifest、workflow、scanner安装和测试 MUST NOT固定上游patch、Docker base digest、Cargo精确`=`版本、Action commit SHA、手工下载archive hash或对应的预批准常量。

Release用`Cargo.lock` MUST由每次CI重新解析并作为证据保存，不得作为长期上游快照提交到源码。PostgreSQL major、Cargo不兼容major和产品SemVer升级仍 MUST通过显式源码变更处理，不得被浮动通道静默跨越。

#### Scenario: 兼容上游发布新版本
- **WHEN** Rust stable、PG18 minor、兼容Cargo依赖、Action major内版本或标准工具发布更新
- **THEN** 下一次CI SHALL自动解析该稳定更新并运行完整行为门禁，但只有后续显式SemVer tag才能产生release

#### Scenario: 上游更新不兼容
- **WHEN** 最新兼容输入无法构建或未通过真实行为测试
- **THEN** 当前main run SHALL失败且上一mtmpg release保持不变，维护者 SHALL通过后续源码修复适配

#### Scenario: PostgreSQL发布新major
- **WHEN** `postgres:latest`或其他通道会从PG18切换到后续major
- **THEN** mtmpg SHALL继续使用PG18稳定通道，直到显式实现新major feature、ABI、路径和运行验证

### Requirement: CI必须只解析一次并复用实际输入
每个验证run SHALL先生成唯一Cargo lockfile、解析builder/runtime浮动tag的完整digest，并记录实际Rust、Cargo、PostgreSQL、pgrx、关键依赖和工具版本。Native tests、ABI生成、真实PG18 integration、production build和final-image验证 MUST复用同一lockfile与解析结果。

CI SHALL生成`resolved-inputs.json`并记录source SHA、workflow run、Cargo.lock digest、实际版本和临时base digest。源码与测试 MUST把这些值作为观测结果验证一致性，不得要求它们等于预先写死的patch或hash。

#### Scenario: 单次run解析输入
- **WHEN** PR、`main`或SemVer tag CI开始验证一个source commit
- **THEN** resolve step SHALL物化Cargo.lock和所有浮动image的实际身份，并把它们传递给该run的全部后续验证步骤；只有tag release可把结果传给发布job

#### Scenario: 后续job重新解析出不同输入
- **WHEN** build、test或publish尝试使用不同lockfile、PostgreSQL minor或base digest
- **THEN** workflow SHALL失败且不得发布release

### Requirement: CI必须验证产品行为而不是实现字面量
每个PR、`main` push与SemVer tag SHALL通过同一可复用CI定义运行Rust领域规则、当前PG18 C/Rust ABI layout、真实临时PostgreSQL OAuth矩阵、production module检查和最终image启动测试。测试 MUST覆盖JWT/role/identity/runtime config/fail-closed、module加载、OAuth allow/deny、`system_user`和错误脱敏。

测试 MUST NOT要求Dockerfile/workflow/脚本包含或不包含特定字符串，也 MUST NOT比较精确版本、archive hash、base layer数量或完整Docker `.Config`。仓库 MUST NOT维护为这些辅助入口伪造Docker、Cargo、GitHub CLI或scanner的大型自测。

完整JWT/profile/role/identity矩阵 SHALL只由Rust领域测试维护，真实backend矩阵 SHALL只由一个PG18 harness维护，最终image SHALL只运行证明打包与启动边界所需的最小allow/deny smoke。共享fixture、client和staging入口 SHALL复用；gomtmui或其他消费者 MUST NOT复制mtmpg native矩阵。

#### Scenario: 领域或真实运行回归
- **WHEN** JWT、ABI、module加载、OAuth、identity或fail-closed行为偏离规格
- **THEN** CI SHALL给出对应行为门禁失败并阻止release

#### Scenario: 同一风险存在重复矩阵
- **WHEN** 两个harness完整覆盖同一JWT、profile、role或identity风险而第二个边界没有新增行为
- **THEN** 测试 SHALL合并到对应单一权威，final-image只保留证明production打包有效的最小smoke

#### Scenario: 上游metadata发生非行为变化
- **WHEN** 当前PG18 base只改变不影响官方entrypoint、module加载和OAuth行为的metadata、layer或默认环境表示
- **THEN** final-image gate SHALL依据真实运行结果继续验证，不得仅因与上一digest逐字段不等而失败

### Requirement: Dockerfile必须只构建标准PG18 production image
根`Dockerfile` SHALL使用浮动稳定builder/runtime tag作为人类可维护默认值，并 SHALL接受CI resolve step提供的本次临时完整digest。它 SHALL构建production`pggomtm.so`并从官方PG18 image只增加module、MIT license和最小非敏感版本信息。

最终image SHALL保留官方entrypoint、default command、volume、stop signal、initdb和postgres用户行为。Dockerfile MUST NOT运行单元测试、集成测试、lint、scanner、临时cluster或测试marker，也 MUST NOT复制test fixture、源码、Cargo target、JWKS/config、PGDATA或credential到最终image。

#### Scenario: 构建production image
- **WHEN** 本次run的领域、ABI和真实PG18前置测试成功
- **THEN** Dockerfile SHALL使用已解析的同一builder/runtime身份生成可按官方方式启动的production image

#### Scenario: 构建上下文包含测试材料
- **WHEN** production stage尝试复制fixture、private material、测试feature或构建工具
- **THEN** final-image内容与运行门禁 SHALL失败并阻止发布

### Requirement: 最终image必须通过真实启动和OAuth验证
Actions SHALL对本次构建的最终image验证实际PostgreSQL属于PG18、官方entrypoint能够完成initdb并启动、`pggomtm.so`位于真实`pkglibdir`、module能够加载、动态依赖可用且OAuth allow/deny与identity smoke通过。

检查 SHALL确认最终image不含private key、JWT fixture、测试feature、源码或compiler，但 MUST使用精简内容检查、标准SBOM policy和真实运行结果，不得通过复制整个base filesystem清单或要求完整config/layer相等实现。

#### Scenario: Final image可用
- **WHEN** Actions启动本次production image并运行OAuth smoke
- **THEN** PostgreSQL SHALL通过官方启动路径ready，加载production module并返回符合规格的授权、拒绝和`system_user`结果

#### Scenario: Final image只有静态形状正确
- **WHEN** image文件存在但官方entrypoint、module加载或OAuth行为失败
- **THEN** release SHALL fail closed

### Requirement: SemVer tag release必须只发布一次已验证image
只有指向仓库source且符合SemVer的Git tag SHALL触发release；去除前导`v`后的tag version MUST与`Cargo.toml` package version精确一致。只读CI SHALL从该tag的精确source和本次resolved inputs构建一次production image，验证后物化为同一run内可传递OCI archive。

最小写权限publish job SHALL只下载并推送该已验证archive到公开`ghcr.io/codeh007/mtmpg:<semver>`，MUST NOT运行Cargo、重新解析依赖或执行第二次Docker build。Prerelease SHALL只创建自身version tag和GitHub prerelease；stable SHALL创建自身version tag、stable GitHub Release并更新`latest`。

目标image version、Git tag和GitHub Release MUST不可覆盖。Actions artifact SHALL只用于同一run内传递archive与release材料，不得作为长期发布权威。

#### Scenario: SemVer tag全部门禁成功
- **WHEN** tag version匹配package且resolve、领域、ABI、PG18和final-image验证全部通过
- **THEN** publish job SHALL推送已验证OCI archive一次，并创建对应version类型的GitHub Release与标准供应链证明

#### Scenario: PR或main验证成功
- **WHEN** 没有SemVer tag的PR或`main` commit通过全部CI门禁
- **THEN** workflow SHALL不写入GHCR image、GitHub Release或attestation

#### Scenario: Tag或目标version不可发布
- **WHEN** tag不匹配package version、任一前置job失败或目标image/Release已经存在
- **THEN** workflow SHALL fail closed且不得覆盖或移动任何既有发布身份

### Requirement: Release材料必须记录实际制品与输入
每个GitHub Release SHALL包含本次Cargo.lock、`resolved-inputs.json`、精简release manifest及其checksums；对应OCI image SHALL具有标准SBOM、provenance与GitHub attestation。材料 SHALL绑定mtmpg SemVer、source SHA、module digest、image OCI digest及实际解析的toolchain/dependency/PostgreSQL/base身份。

Release材料 MUST描述实际结果，不得把某个上游patch或digest作为下一次构建的预批准输入。Workflow MUST NOT重建image来补写metadata；所有材料 MUST从同一已验证OCI archive生成且不含credential。仓库 MUST NOT发布自定义`<version>.evidence` OCI tag、ORAS evidence bundle或跨仓consumer evidence。

#### Scenario: 生成标准release材料
- **WHEN** versioned OCI image已经推送并得到registry digest
- **THEN** workflow SHALL生成并验证GitHub Release assets与标准OCI/GitHub证明，使SemVer能够无歧义解析到source、lockfile、module和image digest

#### Scenario: 供应链身份不一致
- **WHEN** source、lockfile、module、OCI archive、registry digest、SBOM或attestation任一不匹配
- **THEN** release SHALL失败且不得创建或更新`latest`

### Requirement: gomtmui必须最小化消费mtmpg release
Gomtmui SHALL在内测Compose中把PostgreSQL image设置为明确的`ghcr.io/codeh007/mtmpg:<semver>`，并 SHALL复用现有platform初始化、配置与运行契约。Gomtmui MUST NOT本地构建Rust module或mtmpg image，也 MUST NOT增加旧validator、认证fallback、private pull credential或第二份native测试矩阵。

Gomtmui SHALL删除专用mtmpg consumer workflow与测试harness。TLS、sub2api、pgAdmin、ACL/RLS、OAuth issuer和SQL executor的真实集成 SHALL由gomtmui对应领域change在功能启用时验证，不得作为mtmpg release前置条件。平台在pull、启动或备份时 SHALL记录实际resolved digest，但tracked配置 SHALL以mtmpg SemVer表达用户选择。

#### Scenario: 更新内测Compose版本
- **WHEN** gomtmui选择一个已发布mtmpg SemVer用于可重建内测平台
- **THEN** Compose与platform单一常量 SHALL引用该versioned image，且仓库不得新增专用native consumer workflow或测试目录

#### Scenario: 平台领域集成失败
- **WHEN** gomtmui后续启用TLS、profile role、ACL/RLS或SQL executor时发现与某个mtmpg release不兼容
- **THEN** gomtmui SHALL在自身领域change中保持该能力停用并修复前进，不得复制mtmpg native矩阵或要求覆盖既有release

### Requirement: Prerelease与stable必须是独立SemVer release
Prerelease与stable SHALL分别从各自不可变Git tag执行同一完整release门禁。Stable MUST NOT依赖gomtmui consumer evidence或复用某个prerelease digest；若source或解析输入不同，它 SHALL作为新的独立release接受完整测试。只有stable release SHALL更新`latest`。

#### Scenario: 发布prerelease
- **WHEN** `v0.1.0-rc.1`等合法prerelease tag通过完整门禁
- **THEN** workflow SHALL发布对应versioned image与GitHub prerelease，且不得更新`latest`

#### Scenario: 发布stable
- **WHEN** `v0.1.0`等合法stable tag通过完整门禁
- **THEN** workflow SHALL发布该stable version、GitHub Release与标准供应链材料，并把`latest`更新为该release的digest

#### Scenario: 失败tag需要修复
- **WHEN** tag release任一门禁失败或写入后形成没有完整Release的孤立version
- **THEN** 原tag MUST NOT移动或重用，维护者 SHALL修复源码、提升SemVer并精确清理孤立version后重新发布
