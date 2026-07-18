## 1. 源码权威硬切与基线保护

- [x] 1.1 记录`gomtmui/native/pggomtm`除`target/`外的完整文件清单、源码checksum、现有feature矩阵和本地image身份，并确认mtmpg工作树、remote与默认分支状态
- [x] 1.2 把Cargo manifest/lock、toolchain、Dockerfile、Rust源码和测试迁入mtmpg仓库根目录，保持原型行为与测试向量不变且不引入workspace嵌套
- [x] 1.3 完善`.gitignore`与`.dockerignore`，静态证明`target/`、本地image、secret、`.env`、data、session和gomtmui源码没有进入迁移内容或Git history
- [x] 1.4 在mtmpg clean checkout上重新运行现有ABI layout/runtime、JWT/JWKS、identity与pgx OAuth gate，比较迁移前后结果并保存不含secret的基线证据

## 2. 独立仓库规范与阶段性GitHub治理

- [x] 2.1 增加README、MIT LICENSE、SECURITY、CONTRIBUTING/维护说明和仓库级AGENTS规则，准确说明非`CREATE EXTENSION`边界、支持矩阵、构建、测试、安装、升级与安全报告方式
- [x] 2.2 增加发布与兼容文档，定义SemVer、PG/runtime变体、database-token/authn-id contract版本、prerelease/stable门禁、digest消费和rollback语义
- [x] 2.3 配置Dependabot只创建Cargo与GitHub Actions更新PR，禁止native认证依赖、Rust patch或PostgreSQL minor自动合并
- [x] 2.4 收紧GitHub Actions为批准来源和full-SHA pin，保持默认workflow token只读，设置合并后删分支与批准merge策略，并如实记录当时private/free无法启用branch protection/ruleset的阶段性限制；该完成项不代表当前public治理已完成
- [x] 2.5 检查当时仓库描述、topics、issue/security/release设置与private可见性，记录所有者尚未公开package且未擅自改变visibility的基线；后续所有者已公开源码仓库，追溯处置与当前设置由7.x重新验证
- [x] 2.6 审查当前本地`main`领先`origin/main`且功能分支已删除的状态，从精确已审查commit非force push远端`issue-116-extract-pggomtm`功能ref，确认remote commit一致并保持`origin/main`不变，为远端CI建立可审计source identity

## 3. 官方OAuth ABI与pgrx边界

- [x] 3.1 先增加tracked RED门禁，真实证明恶意`RUSTFMT`与`PATH/rustfmt`可被检测，并要求ABI类型/magic来自目标官方`oauth.h`且被校验字节与最终`OUT_DIR`编译字节完全一致
- [x] 3.2 实现最小build-time allowlist bindings，禁用外部formatter并单次materialize生成结果，固定官方header与最终bindings digest，拒绝ambient formatter、缺失symbol、未知layout或非批准server headers
- [x] 3.3 对同一份materialized字节完成allowlist验证后直接原样写入`OUT_DIR`，保留官方C size/offset/layout probe并交叉验证state/result/callback、magic、init signature及最终文件digest，禁止校验后的二次序列化
- [x] 3.4 删除源码中的手写OAuth ABI struct/magic权威与冗余direct `pgrx-pg-sys`依赖，只通过完整pgrx的`pg_sys`、module magic、guard、error和allocator接口实现FFI
- [x] 3.5 覆盖startup/validate/shutdown的null、panic、PostgreSQL ERROR、allocator、错误magic与缺失callback负向矩阵，证明任一异常在真实backend中fail closed

## 4. PG18 stable-line兼容与artifact身份

- [x] 4.1 先修改版本门禁测试，要求`180003`、`180004`和未来PG18 numeric minor通过major检查，PG17/PG19拒绝，并删除精确`sversion == 180004`成功条件
- [x] 4.2 实现runtime PG18 major检查并继续依赖`PG_MODULE_MAGIC`与OAuth validator magic，确保不同major和ABI变化在加载/启动前失败
- [x] 4.3 固定每个build变体的Rust、pgrx、JOSE、PostgreSQL source/header、runtime base digest、target、arch与libc，并生成可比较artifact identity
- [x] 4.4 在精确PG18.4 runtime完成loader、allocator、callback和OAuth smoke，记录“已验证minor”而不宣称未运行minor已获准部署

## 5. 正式只读config/JWKS runtime

- [x] 5.1 定义版本化config schema与唯一只读文件路径契约，只允许issuer、audience、public JWKS路径及批准的公开部署参数，不允许算法、role映射、fallback issuer或私钥配置
- [x] 5.2 先增加config/JWKS snapshot负向测试，覆盖缺失、过大、权限不安全、未知字段、非法HTTPS资源、empty/duplicate/unknown kid、private JWK和非ES256 key
- [x] 5.3 实现每个新OAuth backend在startup读取并验证不可变config/public-JWKS snapshot，缺失或损坏时fail closed且不保留旧缓存或内置key
- [x] 5.4 实现同文件系统原子替换轮换语义，证明后续backend读取完整active+retiring key集合、既有backend不reload且不会观察半写文件
- [x] 5.5 增加静态与ELF依赖门禁，证明正式module不存在HTTP、DNS、libcurl、SQL、SPI、私钥、service credential、在线introspection或第二issuer/fallback

## 6. JWT、role、identity与production artifact

6.3至6.6已经分别由精确远端HEAD的Actions成功run完成；本组全部完成，后续不得用发布供应链工作重新打开或重复实现validator主路径。

- [x] 6.1 把已验证的ES256/JWKS verifier接入无gate feature的正式validate callback，保持固定issuer/audience/database scope、30至300秒TTL和deny-unknown完整claims契约
- [x] 6.2 覆盖OAuth client与API-key credential actor二选一、authority version、profile、role、ID字符/长度、time、algorithm、audience/scope和tampered signature正负矩阵
- [x] 6.3 为已存在的closed ordinary/business-admin/database-developer profile-role映射与startup requested role精确匹配补齐显式unit、config扩权和真实PostgreSQL forbidden-role门禁，拒绝service/migration/cluster/未知role；只有对应远端Actions run通过后才完成，本任务没有发现实现缺口时不得改production Rust逻辑
- [x] 6.4 以无gate production callback和真实libpq OAuth backend证明PostgreSQL allocator与`authn_id -> system_user -> decoded identity`无歧义往返，覆盖OAuth client/API-key actor、三个profile及超长/非法/未知版本拒绝；仅在门禁发现真实缺口时修改实现
- [x] 6.5 明确认证失败reason-code的稳定字符串、服务端日志级别与客户端可见性，验证token拒绝、startup错误、panic和PostgreSQL ERROR只产生脱敏类别且不包含JWT、JWKS内容、connection string或完整内部堆栈
- [x] 6.6 由远端Actions构建无`abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate`的production artifact，扫描排除内置测试JWKS/key/token、probe symbol/string和测试module，并把本任务限定为module级artifact gate而不重复后续CI/发布供应链门禁

## 7. 公开状态修复与main bootstrap

- [x] 7.1 在远端功能ref建立feature push/PR Native CI bootstrap：只用full-SHA批准actions、read-only token、无secret/无GHCR登录、同ref并发取消和BuildKit GitHub Actions cache运行唯一Docker build graph，并用精确remote HEAD的成功run替代本地完整Docker结果
- [x] 7.2 增加Cargo dependency、RustSec与license审计，对完整pgrx transitive advisory逐项记录理由和复核期限，不使用全局ignore或自动放宽
- [x] 7.3 实现默认redact的追溯式public-readiness门禁，覆盖全部refs/history、tracked/uncommitted文件、Docker context、workflow源码/log、Actions artifact/cache、GitHub Issue/PR和candidate image；合成fixture只允许精确路径/模式/理由分类
- [x] 7.4 增加动态链接、ELF export、arch/libc、module位置、官方entrypoint、image filesystem与build-manifest门禁，证明candidate/stable image只增加正式`.so`、MIT license和不含自身OCI digest的公开build metadata
- [x] 7.5 同步README、SECURITY、CONTRIBUTING、MAINTAINERS、AGENTS、GitHub治理与release文档到public、development-main、Agent auto-merge和公开GHCR语义；在mtmpg创建后续release跟踪Issue并反向链接gomtmui #116/#117
- [x] 7.6 在当前公开远端执行完整追溯扫描；真实secret命中时先吊销或轮换并经批准处置history/remote，只有重扫通过后才继续，且不得通过重新设为private或删除日志冒充处置
- [x] 7.7 删除全部无法逐项证明安全的公开前GitHub Actions/BuildKit cache，从clean checkout对最终remote HEAD运行一次无secret、无缓存bootstrap cold build，记录source、run、image和脱敏finding摘要
- [ ] 7.8 对最终remote HEAD完成whole-branch review、source identity、dependency/license、public-readiness、cold与artifact矩阵复核，并运行`git diff --check`和`openspec validate extract-and-standardize-pggomtm --strict`
- [ ] 7.9 按Issue #116授权把已审查功能分支以非force fast-forward原样推进到`origin/main`，确认默认分支识别README/LICENSE/SECURITY/Cargo/workflow后删除功能分支，并证明没有创建tag、Release、package version alias或`latest`
- [ ] 7.10 启用并复核secret scanning、push protection、dependency graph/alerts、private vulnerability reporting与main ruleset；要求PR、Native CI、线性历史和讨论解决，禁止force/delete，required approvals为0，启用squash-only、auto-merge与合并后删分支
- [ ] 7.11 通过受保护main上的普通PR删除一次性Issue #116 feature trigger，将稳态CI收敛为PR/main cached lane和schedule/dispatch cold mode，并把Dependabot按Cargo/Actions分组、限制并发且保持高风险变化不自动合并

## 8. Manifest、公开GHCR与发布流水线

- [ ] 8.1 定义并生成image内build manifest与外部`release-manifest.json` schema：前者不得包含自身OCI digest，后者在digest产生后绑定source、module/contract、PG build/test minor、header/base、target、`.so`/OCI digest与native验证矩阵
- [ ] 8.2 发布versioned正负向database-token、role和authn-id测试向量，让gomtmui issuer/consumer集成测试固定消费且不包含任何真实key/token
- [ ] 8.3 建立最小权限trusted candidate workflow，只接受受保护`main` ancestry上的精确commit，job级申请必要package/id-token/attestation权限，从最终版本commit只构建一次并仅发布source-discovery tag与不可变OCI digest
- [ ] 8.4 为同一candidate生成并验证外部release manifest、SPDX或CycloneDX SBOM、binary/container provenance与GitHub attestation，证明所有source、`.so`、OCI和manifest digest一致且final image通过7.4门禁
- [ ] 8.5 建立trusted stable promotion workflow，只从已验收OCI digest提取并复核相同`.so`，生成tar.zst、`SHA256SUMS`和Release资产，增加SemVer/`latest` alias且不得运行Cargo或Docker rebuild
- [ ] 8.6 建立拒绝覆盖既有tag、asset、manifest、evidence和image alias的immutable GitHub Release机制；Actions临时artifact只用于job传递且不得成为正式安装URL
- [ ] 8.7 文档化cached/cold/trusted lane、公开GHCR匿名读取、完整digest消费、manifest/attestation验证、candidate/stable区别、升级和rollback；文档不得要求private pull credential

## 9. Candidate与跨仓库验收

- [ ] 9.1 若需要alpha/RC，使用与prerelease module version一致的tag、manifest和OCI digest只验证pipeline，证明它不能增加stable tag、更新`latest`或晋级为最终版本；若不需要则记录跳过
- [ ] 9.2 通过普通PR把冻结最终`MAJOR.MINOR.PATCH`的Cargo、contract、manifest与文档版本合并到受保护`main`，记录精确source commit且不得直接创建stable tag或Release
- [ ] 9.3 从该精确main commit只构建一次stable candidate，发布source-discovery tag、完整OCI digest、manifest、vectors、SBOM与attestation，并把`mtmpg-postgres`设置为公开读取后验证匿名pull与写权限隔离
- [ ] 9.4 向gomtmui提供同一candidate source、OCI digest、release manifest、contract vectors与attestation，等待其hard-cut change完成固定消费、真实PG18 OAuth、identity、ACL/RLS和executor candidate E2E
- [ ] 9.5 取得gomtmui绑定candidate source、manifest digest、OCI digest与consumer source的不可变consumer evidence，确认其没有改写candidate manifest或从tag重建image
- [ ] 9.6 复核gomtmui运行代码、build配置与active tests不再依赖本地Rust源码、release tarball、`gomtm-pggomtm:*`标签、private pull credential或认证fallback
- [ ] 9.7 演练切换candidate digest与滚动切回上一已验证digest，确认不热覆盖`.so`、不恢复第二源码或认证fallback，并记录既有backend不会随token撤销自动终止

## 10. 最终验证与stable晋级

- [ ] 10.1 对已验收candidate source触发远端cold authority，从clean checkout运行全部Rust、C probe、Docker、真实PG18 OAuth、dependency/license、secret、SBOM/provenance和artifact隔离门禁；普通cached或本地run不得替代
- [ ] 10.2 运行`git diff --check`与`openspec validate extract-and-standardize-pggomtm --strict`，确认tracked tree不含`target/`、secret、data、临时artifact、重复实现或与public/main/release语义冲突的文档
- [ ] 10.3 完成whole-release review，验证candidate source仍是受保护`main`的未改写祖先，release manifest、consumer evidence、OCI digest、SBOM/attestation与rollback结果精确一致
- [ ] 10.4 只调用trusted promotion workflow为同一已验收OCI digest增加SemVer/`latest` alias，创建精确指向candidate source的stable Git tag和immutable GitHub Release，不得重新构建或改变attestation identity
- [ ] 10.5 验证远端tag、Release assets、manifest、consumer evidence、公开GHCR alias与匿名digest pull相互一致，Actions记录证明promotion没有运行Cargo或产生第二image digest
- [ ] 10.6 汇总mtmpg source、OCI/release digest、manifest、SBOM/attestation、验证矩阵、已知限制、GitHub治理状态和gomtmui consumer evidence，回填mtmpg跟踪Issue与gomtmui #116/#117
