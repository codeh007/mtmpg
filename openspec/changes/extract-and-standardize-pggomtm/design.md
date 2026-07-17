## Context

截至2026-07-17，`pggomtm` 已从 `/workspace/gomtmui/native/pggomtm/` 迁入mtmpg根目录并通过迁移基线，gomtmui中的本地副本和gate已经删除。远端`issue-116-extract-pggomtm`功能ref已经建立并指向`2b77107`，`origin/main`仍为初始commit；当前本地功能分支HEAD为`da45f7d`、领先远端功能ref 17个commit，并保留任务6.3的未提交测试与文档改动。仓库尚无workflow、Actions run、artifact、Release或`mtmpg-postgres` package，因此这些本地提交和image仍不是可审计的远端验收证据。

独立审查曾证明binding生成存在critical provenance缺口：`bindings.to_string()`被校验后，`write_to_file()`可再次通过bindgen默认formatter读取`RUSTFMT`或`PATH/rustfmt`并改写最终文件。当前本地实现已经禁用外部formatter、单次materialize并直接写入被校验的相同字节，任务3.x的本地门禁已完成；但在精确远端commit通过GitHub Actions前，这些结果仍不得作为release provenance或gomtmui consumer输入。

正式无gate callback当前已经从外部只读config/public JWKS建立每backend snapshot并执行严格ES256、actor、claims、profile-role和requested-role验证。任务6.3的本地改动只补充forbidden-role、配置扩权和真实PostgreSQL probe，没有修改production Rust逻辑；任务6.4至6.6仍需完成production `system_user`往返、稳定脱敏reason和最终无gate artifact门禁。

原生 PostgreSQL、Rust、pgrx与多阶段Docker build在本机首次编译耗时较长，且中断的终端输出难以形成持久证据。根`Dockerfile`继续定义唯一build graph，但GitHub Actions必须成为执行该graph、保存日志并判定任务完成的唯一证据权威。本地命令只保留为可选快速诊断，不能完成OpenSpec task、consumer gate或发布门禁。

仓库当前仍为private/free，Actions已经限制为GitHub-owned及批准的Docker actions、强制full-SHA引用、默认token只读，immutable releases已启用；但尚无workflow。所有者已经确认后续会把源码仓库设为public，因此本change必须先完成全历史、工作树、Docker context、workflow与artifact的public-readiness门禁，再由所有者手动改变visibility并复核公开仓库保护能力。

PostgreSQL 官方契约把 OAuth validator 定义为 `oauth_validator_libraries` 动态加载的 server module：它需要 `PG_MODULE_MAGIC`和`_PG_oauth_validator_module_init`，但不需要 control 文件、SQL schema或`CREATE EXTENSION`。`pgrx 0.19.1`支持PG18并提供module magic、guard、error与allocator接口，但其PG18 bindgen输入没有包含`libpq/oauth.h`，因此不是OAuth ABI的权威包装。

gomtmui 的 `hard-cut-pggomtm-delegation-auth` change 同时定义issuer、delegation、MCP SQL executor、PostgreSQL role/RLS和validator行为。抽离后必须保持职责清晰：mtmpg拥有native module及其制品；gomtmui拥有产品身份、签发与平台消费，不得形成第二份Rust源码或第二发布链。

## Goals / Non-Goals

**Goals:**

- 让mtmpg成为`pggomtm`源码、测试、native build、release manifest与制品的唯一权威。
- 完成可实际加载的PG18离线OAuth validator，而不把测试gate或内置key带入正式artifact。
- 以完整pgrx承担已验证的PostgreSQL FFI安全能力，同时从官方安装header生成最小OAuth ABI bindings。
- 保证生成、验证、落盘与编译使用同一份精确bindings字节，任何ambient formatter或验证后转换都不能改变ABI输入。
- 把“构建/测试针对精确minor”与“runtime拒绝同major minor升级”分开，允许PG18 stable line按官方ABI语义升级，但部署前仍要求真实验证与digest pin。
- 发布可被gomtmui按digest消费的PostgreSQL runtime image和可取证Release附件。
- 保持`Dockerfile`为唯一构建图，并建立以远端commit为身份、GitHub Actions为执行与证据权威的最小权限CI、依赖治理和发布流程。
- 以缓存PR/push CI、无缓存冷门禁和受信发布lane分离快速反馈、独立复验与制品写入权限。
- 在公开前完成不回显敏感值的public-readiness审计，并在公开后启用和验证实际可用的secret scanning、依赖安全与分支保护。
- 在远端CI前先建立指向已审查source commit的远端功能分支，同时保持`origin/main`不变直到跨仓库candidate验收完成。

**Non-Goals:**

- 不把pggomtm改造成普通SQL extension，不创建虚假的control/extension SQL，也不使用`cargo pgrx install/package`作为生产交付。
- 不把gomtmui的delegation表、issuer私钥、API-key管理、数据库role/RLS、MCP executor或Cloudflare配置迁入mtmpg。
- 不支持PostgreSQL 17/19、musl、Windows、macOS或多架构首发；第一阶段仅发布已验证的Linux amd64、PG18、Debian/glibc变体。
- 不增加HTTP、DNS、SQL、SPI、在线introspection、active-grant查询、私钥读取或认证fallback。
- 不用submodule、subtree、Cargo git dependency或本地源码override维持双仓库开发路径。
- 不删除本地复现命令或禁止维护者做聚焦诊断；只是不把本地结果当成任务完成、consumer或发布证据。
- 不由自动化Agent擅自公开仓库、公开GHCR package、放宽Actions来源或在日志中输出疑似secret原文。
- 不在本change修改生产数据库、生产Supabase、llm-gateway或直接晋级生产流量。

## Decisions

### 1. mtmpg根目录是单一crate与源码权威

迁移后的仓库根目录直接包含`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`Dockerfile`、`src/`与`tests/`。当前只有一个交付模块，不预建多crate workspace或`native/pggomtm/`嵌套。若未来确有第二个独立PostgreSQL模块，再通过单独change升级为workspace。

迁移只携带源码、锁文件、Docker定义和测试；`target/`、本地image、secret、data与临时证据不得复制。迁移完成后gomtmui不提交本地pggomtm副本，也不加入git submodule。跨仓库本地联调通过构建/拉取明确commit对应的OCI image完成。

备选方案是保留源码在gomtmui并让mtmpg只镜像发布，或在gomtmui添加submodule。两者都会留下两个生命周期或源码级耦合，拒绝。

### 2. 保留完整pgrx，OAuth ABI从官方header生成

正式crate只直接依赖`pgrx = 0.19.1`的`pg18` feature；`pgrx::pg_sys`已经重导出`pgrx-pg-sys`，因此删除未直接使用的`pgrx-pg-sys`依赖。继续使用`pg_module_magic!`、`pg_guard`、PostgreSQL error和`pstrdup`，避免自行重写PostgreSQL longjmp/panic与allocator边界。

新增最小build-time binding步骤，从目标`pg_config --includedir-server/libpq/oauth.h`读取官方header，只allowlist OAuth magic、三种ABI struct和callback类型。生成结果必须关闭外部formatter并单次materialize为内存字节；构建对这份精确字节执行allowlist/constant/layout相关校验后，直接把相同字节写入Cargo `OUT_DIR`并记录digest，不能再次调用formatter、subprocess或`Bindings::write_to_file`式二次序列化。构建同时记录header digest，并保留用官方C compiler执行的size/offset/layout probe。Rust源码不再手写这些struct、字段布局或magic常量。

Provenance门禁必须把`RUSTFMT`指向恶意formatter、把恶意`rustfmt`放到`PATH`首位并覆盖验证后改写magic的真实复现；两者都必须无法执行或无法改变最终编译字节。只验证header digest、内存字符串或C layout而不比较最终`OUT_DIR`字节，不足以关闭该门禁。

只使用`pgrx-pg-sys`会失去pgrx明确提供的guard与module能力，而且上游声明该crate不应独立使用；完全改成C shim又会在抽离时重写已经验证的边界。二者均拒绝。若未来要移除pgrx，必须另开以真实panic/ERROR/allocator矩阵为前置的change。

### 3. Build精确，runtime按PG18 major与magic兼容

每个release变体固定Rust patch、pgrx、JOSE、PostgreSQL source/header版本、官方runtime image digest、target triple与libc，并在release manifest中记录。CI必须在该精确PostgreSQL minor上执行loader和真实OAuth测试。

module runtime不再要求`sversion == 180004`。`PG_MODULE_MAGIC`负责PostgreSQL major ABI，OAuth callback table的magic负责stable line内紧急OAuth ABI变化；startup只接受PG18 major并依赖两种magic与生成layout。消费者仍不得把一个仅验证过18.4的artifact自行装入18.5：新minor先由mtmpg CI重新构建/验证并发布新manifest/image，gomtmui再更新digest。

精确minor runtime gate会把正常安全升级变成代码故障，与PostgreSQL官方ABI指南和样例相悖；完全不记录minor又无法证明构建身份。上述两层策略同时保留安全升级能力和部署可审计性。

### 4. 正式validator按新backend读取只读配置并离线验签

正式artifact不编译`abi-gate`、`abi-runtime-gate`或`pgx-oauth-gate`能力。每个新OAuth backend在validator startup读取版本化只读config和public JWKS snapshot，完成文件权限、大小、schema、issuer、audience、key数量与key类型检查；缺失、损坏或不匹配立即fail closed。平台通过同文件系统内原子替换发布JWKS/config，后续新backend读取新snapshot，既有backend不reload或重新认证；第一阶段不增加SIGHUP/signal handler、网络fetch或跨backend shared cache。

validate callback只执行ES256签名、固定issuer/audience/scope、iat/exp/最大TTL、完整claims、actor二选一、closed profile-role映射和startup requested role检查。成功时用PostgreSQL allocator返回版本化、有限长度且不含secret的`authn_id`；失败只给稳定reason类别。已建立backend不重新验证，也不声称随token过期自动终止。

配置只允许改变部署资源值和public材料路径，不允许调用方配置任意算法、claims、profile-role映射、fallback issuer或私钥路径。gomtmui提供实际issuer/JWKS，但不能扩大validator的闭集安全契约。

### 5. OCI PostgreSQL派生image是主要部署物

release workflow基于与目标平台相同且按digest固定的官方`postgres:<minor>-bookworm`构建最终runtime image，只把`libpggomtm.so`、必要license与非敏感build manifest放入真实`pg_config --pkglibdir`。镜像保持官方entrypoint，不内置JWKS、config、数据库data或gomtmui源码。

主要package命名固定为`ghcr.io/codeh007/mtmpg-postgres`。所有构建可发布短SHA tag，prerelease只发布明确alpha/rc版本与短SHA；只有stable release额外发布`latest`发现别名。PG/runtime变体使用例如`0.1.0-pg18.4-bookworm`。gomtmui和任何部署契约始终固定OCI digest，不消费`latest`。

不使用通用gomtm runtime base，因为该镜像需要精确继承PostgreSQL ABI/runtime，而Node、Go、Python、VNC等通用agent能力只会扩大体积与攻击面。也不把裸`.so`作为容器生产环境的主安装路径，避免arch/libc/PostgreSQL错配。

### 6. Immutable Release提供辅助二进制与供应链证据

同一Git tag建立immutable GitHub Release，并附加：

- `pggomtm-<version>-pg18.4-linux-amd64-glibc.tar.zst`，只包含`.so`、license和manifest；
- `SHA256SUMS`；
- SPDX或CycloneDX SBOM；
- build provenance/attestation；
- `release-manifest.json`。

manifest至少记录source commit、pggomtm version、database-token contract version、authn-id version、Rust/pgrx/JOSE版本、PostgreSQL build/test minor与`PG_VERSION_NUM`、header digest、base image digest、target、arch、libc、`.so` digest、OCI digest和验证矩阵结果。GitHub Actions临时artifact只用于job间传递，不作为正式下载渠道。

目标主机不安装Rust/cargo、不现场编译、不热覆盖已加载`.so`。容器部署通过切换到新OCI digest并重建backend完成；非容器手工安装只允许在manifest完全匹配时把bundle内容放入`pg_config --pkglibdir`并重启/滚动backend。

### 7. mtmpg发布consumer contract，gomtmui只固定消费

mtmpg在release manifest和测试向量中拥有database-token contract、closed role/profile与`authn_id`格式的consumer-side版本。gomtmui仍拥有产品OpenSpec、delegation authority和issuer实现，但其issuer集成测试必须对固定mtmpg release的正负向向量通过。

gomtmui的Compose/platform配置只引用`ghcr.io/codeh007/mtmpg-postgres@sha256:<digest>`，只读挂载config/JWKS，并在candidate E2E中验证artifact manifest、实际server minor、OAuth登录、identity与ACL/RLS。不得把release tarball复制进gomtmui仓库，也不得在gomtmui workflow重建Rust模块。

跨仓库契约升级采用先发布兼容mtmpg prerelease、再更新gomtmui candidate、最后发布stable并固定digest的顺序。120秒database JWT使hard cut不需要双validator或长期兼容token。

### 8. CI与GitHub设置采用最小权限和不可变发布

根`Dockerfile`是唯一build graph authority；`.github/workflows/`中的远端workflow只负责编排该graph、权限、触发条件、缓存、证据与发布，不复制Rust、C或PostgreSQL测试实现。只有关联远端可达commit的GitHub Actions成功run可以完成OpenSpec task、mtmpg release gate或gomtmui consumer gate。本地`docker build`、本地tag和终端输出只用于排障。

CI分为三条lane：

1. Feature push/PR lane使用Docker Buildx和GitHub Actions cache运行Rustfmt、Clippy `-D warnings`、locked tests、依赖/许可证审计、header generation与layout probe、真实PG loader/OAuth负向矩阵、动态依赖和secret/产物隔离扫描。它允许缓存、启用同ref并发取消，不上传正式制品、不登录GHCR且不读取发布secret。
2. Cold authority lane通过`workflow_dispatch`、定时或发布前调用，从clean checkout对固定输入执行无缓存完整build graph并保存commit、job、image与门禁摘要。正常开发CI不得默认`--no-cache`，否则远端执行只转移了等待位置而没有解决反馈周期。
3. Trusted release lane只在明确受信的最终commit上申请最小写权限，构建或晋级GHCR、manifest、SBOM与attestation；PR代码不得接触写package、Release或attestation所需的credential和权限。

首次workflow尚未存在于默认分支，因此以远端功能分支push或PR事件完成bootstrap；后续使用`gh run view`、`gh run watch`与`gh run rerun`读取或复验已有run，workflow被默认分支识别后再把`gh workflow run --ref ...`作为稳定人工入口。不得为了获得`workflow_dispatch`而临时切换默认分支、force push或复制workflow到第二条实现。

workflow引用固定full commit SHA；仓库Actions限制为批准来源并要求SHA pin。默认`GITHUB_TOKEN`保持read-only，release job只显式获得`contents: write`、`packages: write`及attestation需要的最小权限。Dependabot只创建精确依赖升级PR，不自动合并native认证依赖或PostgreSQL minor。

仓库在private/free阶段继续如实记录无法启用branch protection/rulesets的限制，以批准来源、full-SHA、只读token、合并后删分支、人工复核和远端CI补偿。公开前必须对全部refs与历史、工作树、Docker context、workflow日志/artifact、最终image、Release/package及GitHub Issue/PR内容执行不回显敏感值的审计；真实secret先吊销或轮换，再经过明确批准处置历史，合成测试fixture只允许精确位置、精确模式和理由的窄分类，禁止全局ignore。

仓库visibility只由所有者在public-readiness门禁通过后手动切换。公开后立即复核并启用实际可用的secret scanning、dependency graph/alerts与branch protection/ruleset；GHCR package visibility仍是独立决策。公开后的保护规则不得悄悄破坏首个stable的source identity，必须先明确选择“公开前完成首次exact fast-forward”或“让已验证source commit成为受保护main的未改写祖先并由stable tag精确指向它”。

### 9. Prerelease先于stable，禁止把fail-closed原型冒充可安装版本

迁移基线审查后立即建立远端`issue-116-extract-pggomtm`source ref，使后续CI和release都有可审计commit。Alpha/RC与stable candidate是不同lane：带prerelease module version的alpha/RC只验证pipeline，不能增加stable tag或晋级同一digest；可晋级candidate必须先在功能分支冻结最终`MAJOR.MINOR.PATCH`版本，从该commit只构建一次并仅发布short-SHA身份。只有正式callback消费外部只读JWKS/config、无gate feature、真实OAuth allow/deny矩阵和gomtmui对同一source/OCI digest的candidate集成全部通过后，才能fast-forward该remote branch HEAD到`main`，再让stable release与`latest`引用已经验证的同一OCI digest而不重建。

同一source/tag只构建一次。candidate到后续production晋级必须复用同一OCI digest和attestation，不能重新构建一个同tag但不同内容的image。

## Risks / Trade-offs

- [跨仓库开发增加协调成本] -> 用版本化manifest、测试向量、prerelease和digest pin替代源码级联调，不增加submodule或本地fallback。
- [完整pgrx依赖树较大且含需持续复核的transitive advisory] -> 固定lock、运行依赖/许可证审计、记录逐项例外；移除冗余direct `pgrx-pg-sys`，但不以未验证的手写FFI换取表面精简。
- [生成bindings依赖目标server headers] -> build只接受`pg_config`解析的官方header，固定digest并运行C/Rust layout probe；不提交手写生成结果作为第二权威。
- [外部formatter可在校验后改写bindings] -> 禁用bindgen formatter，单次materialize并校验/写入相同字节，记录最终输入digest，使用恶意`RUSTFMT`与`PATH/rustfmt` tracked RED门禁证明验证后无二次转换。
- [接受PG18同major可能被误解为任意minor已获准部署] -> runtime兼容与部署支持分离；manifest只列已构建/验证minor，gomtmui按对应image digest消费。
- [私有仓库当前不能启用branch protection/rulesets] -> 如实记录限制，先用远端CI和人工控制补偿；public-readiness通过并由所有者公开后立即复核和启用服务端保护。
- [私有GHCR增加部署认证] -> 只使用read-only package credential并留在部署secret authority；不得写入Compose、image或release。
- [本地提交和image无法形成可审计验收证据] -> 先提交并非force push到既有远端功能ref，只由对应GitHub Actions run完成任务与consumer门禁。
- [每次远端构建都禁用缓存仍会导致反馈过慢] -> PR/push lane使用内容寻址的BuildKit/GitHub Actions cache和并发取消，定时/人工cold lane及release前门禁再执行无缓存复验。
- [公共仓库的非受信PR可以执行任意Dockerfile代码] -> PR lane使用GitHub-hosted临时runner、只读token、无secret、无GHCR登录且禁止`pull_request_target`；release lane不消费非受信PR权限上下文。
- [secret scanner误报合成fixture或把真实secret打印到日志] -> scanner只输出finding元数据并默认redact；fixture按精确位置分类，真实命中转入私密处置且不得上传含原文报告。
- [公开后的branch protection与首次exact fast-forward冲突] -> 在visibility改变前确定source commit、stable tag和main ancestry规则，不以admin临时绕过掩盖未决治理语义。
- [原型gate feature误入正式artifact] -> stable workflow显式禁止gate features，并扫描符号、字符串、内置key和最终filesystem。
- [PostgreSQL module无法在现有session卸载] -> 不热替换`.so`，通过新image和backend滚动重建；rollback切回上一个已验证digest。

## Migration Plan

1. 记录gomtmui原型文件清单和源码checksum，排除`target/`、image、secret与运行数据；在mtmpg根目录建立唯一crate、文档和仓库规则。
2. 先原样迁入已验证原型并运行现有ABI/JWT/pgx gates，证明迁移没有改变行为；随后在mtmpg内完成pgrx依赖收敛、官方header bindings、PG18 major runtime gate和正式config/JWKS callback。
3. 保留既有远端`issue-116-extract-pggomtm`ref与`origin/main`边界；先更新OpenSpec和流程文档，把当前6.3测试改动与CI bootstrap拆成可审查commit，再非force push到功能ref。首次push/PR直接触发远端workflow，不在本机复跑完整Docker authority来完成6.3。
4. 建立缓存feature push/PR lane、人工/定时cold lane与最小权限release lane；由精确远端HEAD的Actions run验证6.3，再继续6.4至6.6。每个任务只以远端run作为完成证据，本地结果只帮助定位失败。
5. 在private状态运行全refs/history、工作树、Docker context、workflow与artifact的public-readiness扫描和依赖/许可证审查；真实secret先轮换并完成批准处置。门禁通过后等待所有者手动公开，再验证secret scanning、依赖安全、branch protection/ruleset与首次stable合并策略。
6. 若需要alpha/RC，只把它用于pipeline验证且不晋级；正式跨仓库验收前在功能分支冻结最终version commit，从该commit只构建一次short-SHA stable candidate image和manifest，使用真实PG18完成loader、OAuth、role、identity与负向矩阵；不得提前更新`latest`、发布stable或生产说明。
7. 修订gomtmui hard-cut change，删除本地源码/build任务，改为固定prerelease digest和contract向量；更新executor gate、Compose/platform与candidate E2E。
8. mtmpg stable门禁与gomtmui candidate E2E全部通过后，完成rollback演练、远端cold验证与whole-branch review，但此时仍不得创建stable release或更新`latest`。
9. 确认gomtmui运行代码、build配置和active测试不再引用`native/pggomtm`或本地`gomtm-pggomtm:*`标签后，按公开前已确定的main ancestry策略让已验证source进入`main`；stable tag必须精确指向manifest记录的已验证source commit。随后复用同一OCI digest创建首个stable immutable Release与`latest`别名，不得重新构建，最后按Issue要求回填跨仓库commit、release和验证证据。

Rollback不恢复gomtmui中的第二份Rust源码。若prerelease或candidate失败，gomtmui继续固定上一已验证PostgreSQL image/digest或保持candidate OAuth禁用，mtmpg修复前进并发布新版本。若已部署的新stable出现问题，平台滚动切回上一已验证OCI digest；任何已经加载旧`.so`的backend通过连接重建清除。

## Open Questions

- 所有者应在首次stable exact source进入`main`之前还是之后公开仓库；若之前公开，受保护main如何在不改写已验证source commit的前提下完成首次交付？
- 源码仓库公开后，`mtmpg-postgres` GHCR package是否同时公开仍需独立决定；源码visibility不得自动扩大package写权限或部署credential范围。
- 首个stable release是否直接命名`v0.1.0`，还是在gomtmui完整candidate E2E前保留多个alpha/rc；无论命名如何，stable门禁不得降低。
