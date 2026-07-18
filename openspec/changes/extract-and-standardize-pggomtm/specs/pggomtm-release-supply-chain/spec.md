## ADDED Requirements

### Requirement: mtmpg必须是精简的唯一源码与发布权威
`codeh007/mtmpg` SHALL维护唯一`pggomtm` crate、测试、CI和PostgreSQL image定义。仓库 MUST删除`SECURITY.md`、`CONTRIBUTING.md`、`examples/`、历史性`docs/evidence/`及失效引用；仍被真实测试使用的最小fixture SHALL位于`tests/support/`，不得伪装成用户示例。

仓库 MUST NOT保留历史回溯collector、脚本入口自测、Dockerfile/workflow字面量门禁或重复的阶段性说明。生产`runtime_config` MUST保留，但其测试 SHALL按真实风险裁剪并采用不产生误导目录的组织方式。Gomtmui与其他消费者 MUST NOT保留Rust源码副本、submodule、第二Dockerfile、本地image fallback或现场编译路径。

#### Scenario: 精简仓库
- **WHEN** 维护者完成仓库清理
- **THEN** 每个保留的文档、测试、fixture和辅助入口 SHALL直接服务当前产品、远端CI或发布契约，历史过程只由Git、Actions和Release记录保存

#### Scenario: gomtmui消费pggomtm
- **WHEN** gomtmui部署带有pggomtm的PostgreSQL
- **THEN** 它 SHALL引用mtmpg发布的版本化image，不得重新构建module

### Requirement: Main必须是唯一持续集成与交付来源
远端`main` SHALL作为源码持续集成线，并 MAY由维护者或Agent直接非force推进。仓库 MUST NOT要求临时Issue分支、required Pull Request、branch protection、approving review、squash-only或auto-merge作为更新`main`的前置条件。Workflow MUST NOT包含临时分支名称或Issue编号trigger。

`main` MAY暂时处于CI失败状态；失败commit MUST保留在历史中并由后续commit修复，MUST NOT发布candidate、更新stable alias或覆盖既有制品。Git历史和已发布引用 MUST NOT force rewrite。

#### Scenario: 直接更新main
- **WHEN** 维护者或Agent向`main`非force推送一个源码commit
- **THEN** GitHub Actions SHALL验证该精确commit，且仓库不得要求先创建或人工维护PR

#### Scenario: Main验证失败
- **WHEN** `main`上的resolve、领域测试、ABI、真实PostgreSQL、image或evidence任一门禁失败
- **THEN** 该commit SHALL继续作为源码历史存在，但新package、tag、Release、attestation和stable更新 SHALL fail closed

#### Scenario: 外部贡献者提交PR
- **WHEN** 公开Pull Request面向`main`
- **THEN** workflow MAY执行只读验证，但 MUST NOT取得package、Release、attestation或跨仓写权限

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
- **THEN** 下一次CI SHALL自动解析该稳定更新并以完整行为门禁决定是否产生新mtmpg candidate

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
- **WHEN** `main` CI开始验证一个source commit
- **THEN** resolve step SHALL物化Cargo.lock和所有浮动image的实际身份，并把它们传递给全部后续只读与发布job

#### Scenario: 后续job重新解析出不同输入
- **WHEN** build、test或publish尝试使用不同lockfile、PostgreSQL minor或base digest
- **THEN** workflow SHALL失败且不得发布candidate

### Requirement: CI必须验证产品行为而不是实现字面量
每个`main` push SHALL运行Rust领域规则、当前PG18 C/Rust ABI layout、真实临时PostgreSQL OAuth矩阵、production module检查和最终image启动测试。测试 MUST覆盖JWT/role/identity/runtime config/fail-closed、module加载、OAuth allow/deny、`system_user`和错误脱敏。

测试 MUST NOT要求Dockerfile/workflow/脚本包含或不包含特定字符串，也 MUST NOT比较精确版本、archive hash、base layer数量或完整Docker `.Config`。仓库 MUST NOT维护为这些辅助入口伪造Docker、Cargo、GitHub CLI或scanner的大型自测。

#### Scenario: 领域或真实运行回归
- **WHEN** JWT、ABI、module加载、OAuth、identity或fail-closed行为偏离规格
- **THEN** CI SHALL给出对应行为门禁失败并阻止candidate

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
- **THEN** candidate发布 SHALL fail closed

### Requirement: 成功main commit必须只构建并发布一次SemVer candidate
只读验证job SHALL从精确`GITHUB_SHA`和本次resolved inputs构建一次production image，验证后物化为可传递OCI archive。只有仓库自身成功`main` push的最小写权限publish job MAY下载该archive并推送公开`ghcr.io/codeh007/mtmpg-postgres`，publish job MUST NOT运行Cargo或Docker build。

Candidate SHALL使用不可覆盖的mtmpg SemVer prerelease tag。完整OCI digest SHALL在push后记录为证据身份；candidate阶段 MUST NOT创建稳定SemVer、`latest`或stable GitHub Release。

Candidate的Cargo.lock、resolved inputs、manifest、SBOM、provenance、attestation和checksums SHALL同时发布为同一公开GHCR repository内按candidate SemVer派生的不可覆盖OCI evidence artifact。Actions artifact MAY用于job传递和短期诊断，但consumer与promotion MUST NOT依赖会在同run重试时被删除的Actions artifact。

#### Scenario: Main全部门禁成功
- **WHEN** 精确main commit的resolve、领域、ABI、PG18和final-image验证全部通过
- **THEN** publish job SHALL推送已验证OCI archive一次，输出SemVer candidate和完整OCI digest

#### Scenario: 发布条件不满足
- **WHEN** event不是仓库自身`main` push、任一前置job失败或candidate version已经存在
- **THEN** workflow SHALL不写入GHCR image/evidence、tag、Release或attestation

### Requirement: Release材料必须记录实际制品与输入
Candidate evidence SHALL包含本次Cargo.lock、`resolved-inputs.json`、release manifest、SBOM、provenance、attestation和checksums，并 SHALL绑定mtmpg SemVer、source SHA、module digest、image OCI digest、evidence OCI digest及实际解析的toolchain/dependency/PostgreSQL/base身份。

Evidence MUST描述实际结果，不得把某个上游patch或digest作为下一次构建的预批准输入。Workflow MUST NOT重建image来补写metadata；所有材料 MUST从同一已验证OCI archive生成且不含credential。

#### Scenario: 生成candidate evidence
- **WHEN** candidate OCI digest已经产生
- **THEN** workflow SHALL发布并匿名复验不可覆盖OCI evidence，使SemVer能够无歧义解析到source、lockfile、module、image digest与evidence digest，且后续run重试不能删除该发布证据

#### Scenario: 供应链身份不一致
- **WHEN** source、lockfile、module、OCI archive、registry digest、SBOM或attestation任一不匹配
- **THEN** candidate SHALL标记为不可消费且stable promotion SHALL失败

### Requirement: gomtmui必须按mtmpg版本远端验收
Gomtmui SHALL在candidate环境中把PostgreSQL image设置为明确的mtmpg SemVer prerelease tag，并通过GitHub Actions解析和记录其完整OCI digest。验收 SHALL覆盖官方initdb/volume/healthcheck、现有TLS与command、sub2api/pgAdmin连接、module加载、真实OAuth登录、`system_user`identity、ACL/RLS和rollback。

Consumer evidence SHALL绑定mtmpg version/source/manifest/module/OCI digest与gomtmui source。Gomtmui MUST NOT本地构建Rust module或mtmpg image，也 MUST NOT增加旧validator、认证fallback或private pull credential。

#### Scenario: Candidate通过gomtmui验收
- **WHEN** gomtmui远端workflow对版本化candidate完成完整consumer E2E
- **THEN** evidence SHALL证明实际解析的digest与mtmpg manifest一致，且没有本地Rust构建或认证fallback

#### Scenario: Candidate不兼容现有Compose
- **WHEN** initdb、volume、TLS、healthcheck、依赖服务、OAuth或rollback矩阵失败
- **THEN** gomtmui SHALL保持上一已验证mtmpg version，mtmpg SHALL通过新main commit发布新candidate，不得覆盖旧版本

### Requirement: Stable发布必须复用已验收candidate
Stable promotion SHALL只为通过mtmpg gates与gomtmui E2E/rollback的同一OCI digest增加稳定SemVer和`latest`alias，并创建精确source tag和immutable GitHub Release。Promotion MUST NOT运行Cargo、解析新依赖或构建image，也 MUST NOT覆盖既有tag、manifest、asset、Release或image内容。

#### Scenario: 晋级stable
- **WHEN** native、supply-chain、consumer与rollback证据全部绑定同一candidate version和digest
- **THEN** promotion SHALL发布同一digest的稳定mtmpg SemVer及其manifest、lockfile、SBOM、provenance、checksums和consumer evidence

#### Scenario: Promotion尝试重建或覆盖
- **WHEN** stable workflow运行Cargo/Docker build、重新解析上游或发现目标SemVer已经存在
- **THEN** 发布 SHALL失败并要求新main commit产生新candidate重新验收
