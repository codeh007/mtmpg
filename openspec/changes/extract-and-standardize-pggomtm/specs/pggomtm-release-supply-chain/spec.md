## ADDED Requirements

### Requirement: mtmpg必须是唯一源码与镜像权威
`codeh007/mtmpg` SHALL维护唯一`pggomtm` crate、lockfile、toolchain、tests、CI和PostgreSQL image定义。Gomtmui与其他消费者MUST NOT保留Rust源码副本、submodule、第二Dockerfile、本地image fallback或现场编译路径。

#### Scenario: gomtmui消费pggomtm
- **WHEN** gomtmui部署带有pggomtm的PostgreSQL
- **THEN** 它 SHALL引用mtmpg发布的完整OCI digest和versioned contract，不得重新构建module

### Requirement: Main必须是唯一持续集成与交付来源
远端`main` SHALL作为源码持续集成线，并MAY由维护者或Agent直接非force推进。仓库MUST NOT要求临时Issue分支、required Pull Request、branch protection、approving review、squash-only或auto-merge作为更新`main`的前置条件。Workflow MUST NOT包含临时分支名称或Issue编号trigger。

`main` MAY暂时处于CI失败状态；失败commit MUST保留在历史中并由后续commit修复，MUST NOT发布candidate、更新stable alias或覆盖既有制品。Git历史和已发布引用MUST NOT force rewrite。

#### Scenario: 直接更新main
- **WHEN** 维护者或Agent向`main`非force推送一个源码commit
- **THEN** GitHub Actions SHALL验证该精确commit，且仓库不得要求先创建或人工维护PR

#### Scenario: Main验证失败
- **WHEN** `main`上的任一必需测试或image检查失败
- **THEN** 该commit SHALL继续作为源码历史存在，但所有package、attestation、tag、Release和stable更新 SHALL fail closed

#### Scenario: 外部贡献者提交PR
- **WHEN** 公开Pull Request面向`main`
- **THEN** workflow MAY执行只读验证，但MUST NOT取得package、Release、attestation或跨仓写权限

### Requirement: 重计算必须只在GitHub Actions执行
mtmpg的开发测试、原生编译、临时PostgreSQL cluster、Docker build/run和最终image检查 SHALL只在GitHub-hosted Actions runner执行。共享本地工作区MUST NOT执行这些重计算；本地只允许源码/规划编辑、Git/OpenSpec操作、只读调查和对既有诊断对象的精确清理。

承载重计算的脚本 SHALL在非GitHub Actions环境拒绝执行，帮助和纯fixture policy命令除外。仓库MUST NOT以本地image、container、tag或终端日志作为task、consumer或release证据，也MUST NOT使用宽泛Docker prune清理共享主机。

#### Scenario: Agent尝试本地构建
- **WHEN** Agent在普通本地工作区调用mtmpg Docker build、Docker run、原生编译或PostgreSQL integration入口
- **THEN** 入口 SHALL在消耗重计算资源前拒绝，并指向GitHub Actions workflow

#### Scenario: Actions执行相同入口
- **WHEN** `GITHUB_ACTIONS=true`的固定runner运行CI
- **THEN** 入口 SHALL执行完整测试和制品检查，并在结束后清理container、PGDATA、config和fixture

### Requirement: Dockerfile必须只构建标准PostgreSQL runtime image
根`Dockerfile` SHALL使用固定Rust/toolchain/lock与PostgreSQL 18.4 development inputs构建production`pggomtm.so`，并 SHALL从固定digest的官方`postgres:18.4-bookworm`组装最终image。最终image SHALL保留官方entrypoint、default command、volume、stop signal、initdb与postgres用户语义，只增加正式module、MIT license和非敏感build metadata。

Dockerfile MUST NOT运行单元测试、集成测试、PostgreSQL test cluster、lint、依赖/许可证审计、secret扫描或CI policy；MUST NOT复制tests、fixture、gate module、Rust toolchain、Cargo target、源码、JWKS/config、PGDATA或secret到最终image。

#### Scenario: 构建最终image
- **WHEN** 成功的`main` CI在全部外部测试通过后构建Dockerfile
- **THEN** 结果 SHALL是可按官方方式启动的PostgreSQL 18.4 image，且`pggomtm.so`位于真实`pg_config --pkglibdir`

#### Scenario: Dockerfile尝试执行测试
- **WHEN** Dockerfile新增Cargo test、test fixture、临时cluster、scanner或测试成功marker
- **THEN** source policy SHALL失败并要求把该逻辑放回Actions调用的测试入口

### Requirement: Native CI必须直接运行Rust与PostgreSQL测试
每个`main` push SHALL由GitHub Actions直接运行固定版本的Rustfmt、Clippy`-D warnings`、locked Cargo tests、依赖/许可证/secret检查、官方OAuth header/layout与bindings最终字节同一性测试，以及临时PostgreSQL 18.4上的loader、startup、OAuth allow/deny、identity、失败脱敏和production artifact测试。可选PR lane SHALL运行同一只读验证。

测试逻辑 SHALL位于Rust tests、C/SQL probe或专用测试脚本，不得编码在Dockerfile layer中。Workflow MAY使用内容寻址cache，但MUST NOT以独立scheduled cold/cache清理仪式作为发布前提。

#### Scenario: Main CI验证变更
- **WHEN** `main` push触发Native CI
- **THEN** 每类门禁 SHALL有独立可定位结果，任一失败 SHALL阻止candidate job

#### Scenario: 运行PostgreSQL集成测试
- **WHEN** Actions验证OAuth validator runtime
- **THEN** harness SHALL创建隔离的临时PG18.4 cluster、安装对应gate module、运行正负矩阵并在结束后清理PGDATA

#### Scenario: 使用pgrx测试工具
- **WHEN** 测试需要PostgreSQL进程内能力
- **THEN** 项目 MAY使用兼容非SQL-extension加载协议的pgrx工具，但 MUST NOT伪造control/SQL/`CREATE EXTENSION`契约来满足`cargo pgrx test`

### Requirement: 最终image必须经过独立制品检查
Actions SHALL在Docker build完成后独立检查官方base layer/config、PostgreSQL 18.4版本、entrypoint/command/volume/stop signal/user、module位置与加载、动态依赖、ELF identity、filesystem增量和build metadata。检查 MUST证明最终image不含测试feature、fixture、credential、运行数据、源码或构建工具。

#### Scenario: Image包含测试或构建内容
- **WHEN** filesystem、ELF、string或SBOM扫描发现gate feature、fixture、Cargo target、compiler、source、JWKS working copy或PGDATA
- **THEN** candidate发布 SHALL fail closed

#### Scenario: 官方runtime语义被覆盖
- **WHEN** candidate的entrypoint、command、volume、stop signal、user、环境或base layer前缀与固定官方image不一致
- **THEN** image readiness SHALL失败且不得发布candidate

### Requirement: 成功main commit必须只发布一次公开candidate
只有仓库自身成功的`main` push MAY取得job级最小package、id-token与attestation写权限。Candidate job SHALL从精确`GITHUB_SHA`构建并推送一次`ghcr.io/codeh007/mtmpg-postgres`，产生不可变完整OCI digest；MUST NOT从PR、fork、临时分支、失败job或本地image发布。

Package SHALL公开读取。完整OCI digest SHALL作为部署身份；source tag只能用于发现。Candidate阶段MUST NOT创建stable SemVer alias、`latest`或GitHub Release。

#### Scenario: Main全部门禁成功
- **WHEN** `main`精确commit的native、security、integration与image gates全部通过
- **THEN** candidate job SHALL构建并推送一次image，输出公开可读取的完整digest和source identity

#### Scenario: 发布条件不满足
- **WHEN** event不是仓库自身`main` push或任一前置job失败
- **THEN** workflow SHALL不登录或写入GHCR，也不得请求Release或attestation写权限

#### Scenario: 消费者选择image
- **WHEN** gomtmui更新PostgreSQL service
- **THEN** Compose SHALL使用`ghcr.io/codeh007/mtmpg-postgres@sha256:<digest>`，不得使用本地tag或浮动tag

### Requirement: Release材料必须绑定同一制品
Image内build metadata SHALL记录source、toolchain、PostgreSQL/base、module和`.so` digest，但 MUST NOT记录尚未产生的自身OCI digest。OCI digest产生后，同一candidate job SHALL从同一build result生成外部`release-manifest.json`、SBOM、provenance和attestation，并 SHALL将source、`.so`、OCI digest与native matrix绑定。

Workflow MUST NOT重建image来补写metadata。Manifest、SBOM、attestation和checksum中的source/module/image身份 MUST精确一致且不含credential。

#### Scenario: 生成release manifest
- **WHEN** candidate OCI digest已经产生
- **THEN** workflow SHALL从同一build result生成外部材料并证明它们绑定该digest，不得重建image

#### Scenario: 供应链身份不一致
- **WHEN** source、module digest、OCI digest、SBOM、provenance或attestation任一不匹配
- **THEN** candidate SHALL标记为不可消费且stable promotion SHALL失败

### Requirement: gomtmui必须远端验证真实消费契约
Gomtmui SHALL在candidate环境中只替换`docker-compose.yml`的PostgreSQL image完整digest，并通过GitHub Actions验证官方initdb/volume/healthcheck语义、现有TLS与command配置、sub2api/pgAdmin连接、真实OAuth登录、`system_user`identity、ACL/RLS和rollback。Consumer evidence SHALL绑定mtmpg source、manifest、OCI digest和gomtmui source。

Gomtmui MUST NOT本地构建Rust module或mtmpg image，也 MUST NOT增加旧validator、认证fallback或private pull credential。

#### Scenario: Candidate通过gomtmui验收
- **WHEN** gomtmui远端workflow对固定digest完成完整consumer E2E
- **THEN** evidence SHALL证明运行的是manifest声明的PostgreSQL/module bytes，且没有本地Rust构建或认证fallback

#### Scenario: Candidate不兼容现有Compose
- **WHEN** initdb、volume、TLS、healthcheck、依赖服务或OAuth矩阵失败
- **THEN** gomtmui SHALL保持上一已验证image，mtmpg SHALL通过新main commit发布新candidate，不得修改旧digest

### Requirement: Stable发布必须复用已验收candidate
Stable promotion SHALL只为通过mtmpg gates与gomtmui E2E/rollback的同一OCI digest增加SemVer/`latest` alias，并创建精确source tag和immutable GitHub Release。Promotion MUST NOT运行Cargo或Docker build，也 MUST NOT覆盖既有tag、manifest、asset或image内容。

#### Scenario: 晋级首个stable
- **WHEN** native、supply-chain、consumer与rollback证据全部绑定同一candidate digest
- **THEN** promotion SHALL发布同一digest及其manifest、SBOM、provenance、checksums和consumer evidence

#### Scenario: Promotion尝试重建
- **WHEN** stable workflow运行Cargo/Docker build或产生新的module/image digest
- **THEN** 发布 SHALL失败并要求新main commit产生新candidate重新验收
