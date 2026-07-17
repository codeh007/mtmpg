## ADDED Requirements

### Requirement: mtmpg必须是pggomtm唯一源码与构建权威
`codeh007/mtmpg` SHALL 在仓库根目录维护唯一 `pggomtm` crate、lockfile、toolchain、Docker build graph、tests 和 release workflow。根 `Dockerfile` SHALL 是唯一构建图权威，GitHub Actions SHALL 是执行该构建图并形成 task、consumer 与发布证据的唯一权威。gomtmui 或其他消费者 MUST NOT 保留 vendored 源码、submodule、subtree、第二 Docker build 或可运行 fallback；Cargo `target/`、本地 image、secret 和运行数据 MUST NOT 进入迁移或 Git history。

#### Scenario: 从gomtmui迁移原型
- **WHEN** 迁移现有 `gomtmui/native/pggomtm` 原型
- **THEN** mtmpg SHALL 用文件清单和源码 checksum 证明所有权威源码与测试已迁入、`target/` 已排除且既有门禁行为未改变

#### Scenario: 消费者需要pggomtm
- **WHEN** gomtmui 构建或部署 PostgreSQL candidate
- **THEN** 它 SHALL 消费 mtmpg 发布的固定 OCI digest 和 versioned contract，不得从本地 Rust 目录重新构建

#### Scenario: 本地构建成功
- **WHEN** 开发者本地 `docker build`、测试命令或 image tag 成功
- **THEN** 结果 MAY 用于定位问题，但 MUST NOT 完成 OpenSpec task、gomtmui consumer gate、release readiness 或发布证据

### Requirement: 公开仓库必须具备可由Agent维护的受保护开发主线
公开仓库的默认 `main` SHALL 包含 README、MIT LICENSE、SECURITY、CONTRIBUTING、维护/发布说明、Cargo manifest、OpenSpec、源码和唯一 workflow。首次 bootstrap 后，branch ruleset SHALL 要求普通变更经 Pull Request、必需 Native CI、线性历史和讨论解决后进入 `main`，并 SHALL 禁止 force push 与 branch deletion。Required approving review 数 SHALL 为零，仓库 SHALL 启用 squash-only、auto-merge 和合并后删分支，使单贡献者场景中的 Agent 能创建、更新、验证和自动合并 Issue 范围内的 PR。Native 认证依赖、Rust toolchain、PostgreSQL minor、Actions source/pin 与 release workflow 变化仍 MUST 有显式技术审查证据，不得无条件自动合并。

#### Scenario: 建立公开默认分支基线
- **WHEN** 追溯 public-readiness、bootstrap cold CI、whole-branch review 和 source identity 核对全部通过
- **THEN** 已审查远端功能分支 SHALL 以非 force fast-forward 原样进入 `main`，随后删除功能分支并启用 ruleset，且该推进 MUST NOT 创建 stable tag、Release、version alias 或 `latest`

#### Scenario: Agent维护普通PR
- **WHEN** Agent 为一个已限定 Issue 创建普通短期 PR 且所有必需检查通过
- **THEN** Agent SHALL 启用 auto-merge，GitHub SHALL squash 合并并删除源分支，不要求不存在的第二贡献者批准

#### Scenario: 高风险依赖或发布权限变化
- **WHEN** PR 修改 pgrx、JOSE、Rust toolchain、PostgreSQL minor、批准 action、release workflow 或 workflow 写权限
- **THEN** PR SHALL 记录上游 diff、风险、精确验证与独立技术审查结论，缺少任一证据时 MUST NOT 启用 auto-merge

#### Scenario: Workflow请求权限
- **WHEN** 普通 CI 或 trusted candidate/release workflow 运行
- **THEN** 普通 CI SHALL 只有 read 权限，trusted job SHALL 只在 job 级取得写 GHCR、Release 和 attestation 所需的最小权限

### Requirement: 已公开仓库必须完成追溯式public-readiness并持续防泄漏
仓库已经公开，因此系统 SHALL 立即执行不回显敏感值的追溯式 public-readiness，覆盖全部 Git refs/history、tracked 与 uncommitted 文件、Docker build context、workflow 源码与日志、Actions artifact/cache、最终 image、Release/package 及 GitHub Issue/PR 内容。真实 secret MUST 先吊销或轮换，再按明确批准处置 history 和远端材料；重新设为 private、删除日志或补写文档 MUST NOT 被解释为撤销既有暴露。合成 fixture 只允许按精确路径、精确模式和理由分类，MUST NOT 使用全局 ignore。首次处置后，PR、cold 与 release lane SHALL 持续执行相同类别的脱敏扫描。

#### Scenario: 仓库已在门禁前公开
- **WHEN** public visibility 已经发生但没有完整 public-readiness 证据
- **THEN** change SHALL 把状态记录为待追溯处置，不得倒推声称公开前门禁通过，也不得仅因默认 `main` 尚未包含源码而跳过其他公开 ref

#### Scenario: 追溯扫描只有合成哨兵命中
- **WHEN** scanner 只命中用于证明门禁有效的确定性哨兵或公开 fixture
- **THEN** 维护者 SHALL 记录精确路径、模式和理由后继续门禁，且不得放宽其他文件、历史或 secret 类别

#### Scenario: 追溯扫描发现真实secret
- **WHEN** 任一 ref、history、工作树、日志、artifact、cache、image 或协作内容包含真实 credential 或私密材料
- **THEN** 相关 credential SHALL 先被吊销或轮换，合并与发布 MUST 暂停，只有批准的 history/remote 处置和完整重扫通过后才能继续

#### Scenario: 旧Actions cache无法可信审计
- **WHEN** 公开前产生的 BuildKit cache 不能逐项证明只含批准的公开输入
- **THEN** 全部相关 cache SHALL 被删除，并 SHALL 从 clean checkout 的无缓存 cold build 建立新的可信 cache 起点

### Requirement: CI必须从固定输入重复验证native安全边界
每个 Pull Request 与 `main` push SHALL 对 locked dependencies 运行 Rustfmt、Clippy `-D warnings`、unit/integration tests、依赖与许可证审计、官方 header binding/layout 与最终字节同一性、真实 PG18 loader/OAuth 正负矩阵、动态依赖、secret 和 artifact 隔离扫描。CI MUST 从远端可达的精确 source commit 运行并使用固定 Rust、pgrx、JOSE、PostgreSQL source/runtime digest 和 full-SHA actions。普通 PR/`main` lane SHALL 使用内容寻址的 BuildKit/GitHub Actions cache；cold authority SHALL 从 clean checkout 无缓存复验；trusted candidate/release SHALL 只接受受保护 `main` ancestry 上的精确 commit。

#### Scenario: Bootstrap前运行最终cold门禁
- **WHEN** workflow 尚未存在于默认分支且最终功能分支准备进入 `main`
- **THEN** 唯一 workflow MAY 通过一次性 feature-push cold 路径在无 secret、无缓存上下文验证 exact remote HEAD，且该路径不得成为永久第二 CI 实现

#### Scenario: 日常PR或main验证
- **WHEN** Pull Request 或 `main` push 触发普通验证
- **THEN** workflow SHALL 使用 GitHub-hosted runner、read-only token、BuildKit cache 和同 ref 并发取消运行唯一 Docker build graph，且不得登录 GHCR、读取 release credential 或上传正式制品

#### Scenario: 冷门禁复验固定commit
- **WHEN** schedule、workflow dispatch 或发布前门禁验证一个批准的远端 commit
- **THEN** workflow SHALL 从 clean checkout 对固定输入执行无缓存完整 build graph并保存 source、run 与验证摘要，且不得用普通 cached run 冒充 cold authority 证据

#### Scenario: Public fork提交PR
- **WHEN** 非受信 fork 代码触发公开仓库 PR 验证
- **THEN** workflow SHALL 只使用 GitHub-hosted 临时 runner 和 read-only token，不得使用 `pull_request_target`、package/Release 写权限、attestation 写权限或任何 secret

#### Scenario: ABI或供应链门禁失败
- **WHEN** header/layout、OAuth 矩阵、lock 审计、动态依赖、secret 扫描或 artifact 隔离任一失败
- **THEN** CI SHALL fail closed且不得发布 image、Release 或成功 attestation

### Requirement: GHCR派生PostgreSQL image必须公开读取且按digest消费
Trusted candidate workflow SHALL 发布基于精确官方 `postgres:<minor>-bookworm@sha256:<digest>` 的 `ghcr.io/codeh007/mtmpg-postgres` image，只把正式 `libpggomtm.so`、MIT license 和非敏感 build manifest 加入真实 `pg_config --pkglibdir`。Image MUST 保持官方 entrypoint，MUST NOT 包含 JWKS/config、私钥、token、数据库 data、gomtmui 源码、Rust toolchain、Cargo target 或测试 gate。Package SHALL 公开读取，但写权限 SHALL 只授予受保护 `main` 上的 trusted job。消费者 MUST 始终按完整 OCI digest 部署。

#### Scenario: 构建candidate runtime image
- **WHEN** trusted candidate workflow 从受保护 `main` 的精确 commit完成 native 与真实 PG 测试
- **THEN** workflow SHALL 只构建一次并发布 source-discovery tag、不可变 OCI digest 与关联供应链材料，不得提前创建 stable SemVer alias 或 `latest`

#### Scenario: 匿名读取公开package
- **WHEN** 未携带 registry credential 的消费者拉取已公开 `mtmpg-postgres` package
- **THEN** registry MAY 允许读取 image，但任何上传、删除、改标或 Release 操作 MUST 继续要求 trusted workflow 的最小写权限

#### Scenario: 正式环境选择image
- **WHEN** gomtmui candidate 或后续环境部署 pggomtm
- **THEN** 配置 SHALL 使用完整 OCI digest，且不得只引用 `latest`、version tag、source tag 或本地 `gomtm-pggomtm:*` tag

### Requirement: Build manifest与release manifest必须避免循环身份
Image 内 build manifest SHALL 只记录在 OCI digest 产生前可确定的 module version、features、toolchain、dependencies、PostgreSQL source/header/runtime base、target、arch、libc 与 `.so` digest，MUST NOT 记录 image 自身 OCI digest。Image digest 产生后，trusted workflow SHALL 生成外部 `release-manifest.json`，绑定 remote source commit、module/contract、PG build/test minor、header/base、target、`.so`/OCI digest 与 native 验证矩阵，并 SHALL 用不可变 OCI 关联材料或 GitHub attestation 绑定其身份。

#### Scenario: 生成image内build manifest
- **WHEN** Docker build 尚未产生最终 OCI digest
- **THEN** build manifest SHALL 提供可比较构建事实但 MUST NOT 包含占位、自引用或预计算的 image digest

#### Scenario: 生成外部release manifest
- **WHEN** candidate image 已经产生完整 OCI digest
- **THEN** workflow SHALL 从同一 build result 生成外部 manifest并验证 source、`.so` 与 OCI digest 一致，且不得重建 image 来补写其内部文件

### Requirement: Release manifest必须版本化跨仓库消费契约
`release-manifest.json` SHALL 至少记录 source commit、module version、database-token contract version、authn-id version、Rust/pgrx/JOSE、PostgreSQL build/test minor 与 `PG_VERSION_NUM`、OAuth header digest、base image digest、target、arch、libc、`.so` digest、OCI digest 和 native 验证矩阵。Manifest 创建后 MUST 不可变。Gomtmui SHALL 在更新消费 digest 前验证全部字段与正负向 contract vectors，并 SHALL 产生单独绑定 manifest digest、OCI digest 和 consumer source 的 E2E evidence；consumer evidence MUST NOT 改写 candidate manifest。

#### Scenario: gomtmui升级pggomtm
- **WHEN** gomtmui PR 选择新的 mtmpg candidate digest
- **THEN** consumer gate SHALL 校验 manifest、contract version、PG variant、artifact digest、attestation 和正负向 token/identity vectors 后才允许 candidate 部署

#### Scenario: Contract或runtime不兼容
- **WHEN** token/authn contract、PG minor、arch、libc、base digest、artifact digest 或验证状态与目标平台不匹配
- **THEN** gomtmui SHALL 拒绝该 artifact且不得回退本地 build 或旧协议适配器

#### Scenario: 跨仓库E2E完成
- **WHEN** gomtmui 对 candidate source/manifest/OCI digest 完成真实 PG18 与产品 E2E
- **THEN** consumer evidence SHALL 精确绑定三者及 gomtmui source，且原 candidate manifest 和 image MUST 保持字节不变

### Requirement: GitHub Release必须不可变且复用已验证candidate bytes
每个 stable Git tag SHALL 对应一个 immutable GitHub Release，包含按 target 命名的 `.so` bundle、`SHA256SUMS`、MIT license、candidate release manifest、consumer evidence、SBOM 与 provenance/attestation。Stable promotion SHALL 从已验收 OCI digest 提取相同 `.so`并比较其 digest后打包，MUST NOT 重新运行 Cargo、重新构建 image 或改变 attestation identity。Actions 临时 artifact MUST NOT 成为正式分发入口；Release 创建后 tag、asset 与 manifest MUST NOT 被覆盖或替换。

#### Scenario: 发布PG18.4 amd64 stable变体
- **WHEN** 同一 candidate digest 的 native、cold、consumer E2E 与 rollback 门禁全部通过
- **THEN** Release SHALL 包含 `pggomtm-<version>-pg18.4-linux-amd64-glibc.tar.zst`、checksum、manifest、consumer evidence、SBOM 与 provenance，且 bundle 中 `.so` digest SHALL 与 candidate manifest一致

#### Scenario: Promotion尝试重新编译
- **WHEN** stable workflow 尝试运行 Cargo、Docker rebuild 或使用另一个 image digest生成 Release asset
- **THEN** 发布流程 SHALL 拒绝，必须只从已验收 candidate digest 提取和验证 bytes

#### Scenario: 尝试覆盖既有release
- **WHEN** 相同 tag 或 version 请求上传不同 binary、manifest、evidence 或 image 内容
- **THEN** 发布流程 SHALL 拒绝并要求新版本，而不是产生同名可变制品

### Requirement: Stable发布必须晚于正式runtime与跨仓库验收
带 prerelease module version 的 alpha/RC MAY 用于 pipeline 验证，但 MUST NOT 直接晋级为 stable。可晋级 stable candidate SHALL 来自受保护 `main` 上冻结最终 `MAJOR.MINOR.PATCH` 的精确 commit，从该 commit 只构建一次并先只发布 source identity 与 OCI digest。首个 stable SHALL 要求 production feature、真实 PG18 OAuth allow/deny、role/identity、无 gate artifact、依赖/许可证/secret、SBOM/provenance、gomtmui 对同一 source/manifest/digest 的 E2E 和 rollback 全部通过。验收期间 `main` MAY 继续前进；stable tag SHALL 精确指向 candidate commit并证明它仍属于 `main` ancestry。SemVer/`latest` 与 immutable Release MUST 只引用已经验证的同一 OCI digest。

#### Scenario: Alpha或RC完成pipeline验证
- **WHEN** prerelease module version 的 artifact 通过自身门禁
- **THEN** 它 MAY 发布对应 prerelease identity，但 MUST NOT 被重新标记为 stable、更新 `latest` 或作为最终版本 digest晋级

#### Scenario: 最终版本candidate进入跨仓库验收
- **WHEN** 最终 module version 已通过普通 PR 进入 `main`
- **THEN** trusted workflow SHALL 从该精确 main commit只构建一次，并只发布 source-discovery identity、OCI digest、manifest、SBOM 与 attestation供 gomtmui验收

#### Scenario: 验收期间main继续前进
- **WHEN** candidate E2E 进行期间后续普通 PR 已进入 `main`
- **THEN** candidate MAY 继续晋级，只要其 source commit仍是 `main` 未改写祖先、全部证据仍精确绑定该source/digest且没有重建

#### Scenario: Stable门禁全部通过
- **WHEN** mtmpg native/cold 矩阵与 gomtmui consumer evidence 都引用同一 source、manifest 和 OCI digest并成功
- **THEN** promotion SHALL 为同一 digest增加 stable version/`latest` alias，创建指向 candidate source commit 的 tag 与 immutable Release，且不得重新构建

### Requirement: 安装、轮换与rollback必须以不可变image为单位
容器环境 SHALL 通过切换完整 OCI digest并滚动重建 PostgreSQL backend 安装或升级 pggomtm，JWKS/config SHALL 作为运行时只读 mount 独立轮换。系统 MUST NOT 热覆盖已加载 `.so`、在目标主机现场编译或把 release bundle持久化进 gomtmui源码。Rollback SHALL 切回上一已验证 digest，不得恢复第二份源码或认证 fallback。

#### Scenario: Candidate升级module
- **WHEN** 新 candidate通过 manifest与candidate smoke
- **THEN** 平台 SHALL 拉取固定 digest、重建服务并验证实际 server/module/OAuth身份后再完成切换

#### Scenario: 新release运行失败
- **WHEN** loader、OAuth、identity或platform smoke在切换后失败
- **THEN** 平台 SHALL 停止新能力并滚动切回上一已验证 digest，mtmpg SHALL 通过新版本修复前进

### Requirement: 发布与部署不得泄漏secret或扩大package写权限
仓库、Git history、workflow日志、BuildKit cache、image layer、Release asset、SBOM、provenance和manifest MUST NOT包含signing private key、API key、OAuth/database JWT、authorization code、数据库连接串、`.env`、PostgreSQL data、session或真实JWKS working copy。公开GHCR读取 MUST NOT需要部署credential；package、tag、Release与attestation写权限 SHALL只存在于受保护`main`的trusted job运行期，且MUST NOT进入Compose、image、manifest或Release。

#### Scenario: 扫描发布物
- **WHEN** trusted workflow对source、history、log、cache、image filesystem、bundle、SBOM和attestation执行泄漏扫描
- **THEN** 任一真实敏感材料命中 SHALL 阻止发布且不得用删除日志、设为private或弱化规则绕过

#### Scenario: 公开环境按digest拉取image
- **WHEN** 部署主机读取公开 `mtmpg-postgres@sha256:<digest>`
- **THEN** 它 SHALL 不需要private pull credential，且匿名读取能力不得授权上传、删除、改标或发布Release

#### Scenario: 非受信PR尝试取得写权限
- **WHEN** fork或普通PR修改workflow并尝试访问package、Release或attestation写权限
- **THEN** GitHub Actions SHALL 在read-only、无secret上下文运行并拒绝任何写入
