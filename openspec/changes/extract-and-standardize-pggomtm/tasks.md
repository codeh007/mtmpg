## 1. 源码权威硬切与基线保护

- [x] 1.1 记录`gomtmui/native/pggomtm`除`target/`外的完整文件清单、源码checksum、现有feature矩阵和本地image身份，并确认mtmpg工作树、remote与默认分支状态
- [x] 1.2 把Cargo manifest/lock、toolchain、Dockerfile、Rust源码和测试迁入mtmpg仓库根目录，保持原型行为与测试向量不变且不引入workspace嵌套
- [x] 1.3 完善`.gitignore`与`.dockerignore`，静态证明`target/`、本地image、secret、`.env`、data、session和gomtmui源码没有进入迁移内容或Git history
- [ ] 1.4 在mtmpg clean checkout上重新运行现有ABI layout/runtime、JWT/JWKS、identity与pgx OAuth gate，比较迁移前后结果并保存不含secret的基线证据

## 2. 独立仓库规范与GitHub治理

- [ ] 2.1 增加README、MIT LICENSE、SECURITY、CONTRIBUTING/维护说明和仓库级AGENTS规则，准确说明非`CREATE EXTENSION`边界、支持矩阵、构建、测试、安装、升级与安全报告方式
- [ ] 2.2 增加发布与兼容文档，定义SemVer、PG/runtime变体、database-token/authn-id contract版本、prerelease/stable门禁、digest消费和rollback语义
- [ ] 2.3 配置Dependabot只创建Cargo与GitHub Actions更新PR，禁止native认证依赖、Rust patch或PostgreSQL minor自动合并
- [ ] 2.4 收紧GitHub Actions为批准来源和full-SHA pin，保持默认workflow token只读，设置合并后删分支与批准的merge策略，并记录private套餐无法启用branch protection/rulesets的真实限制
- [ ] 2.5 检查仓库描述、topics、issue/security/release设置与私有可见性，保持private默认且不擅自升级套餐或公开package

## 3. 官方OAuth ABI与pgrx边界

- [ ] 3.1 先增加失败测试，要求ABI类型与magic来自目标`pg_config --includedir-server/libpq/oauth.h`生成结果而不是手写Rust声明
- [ ] 3.2 实现最小build-time allowlist bindings，固定并记录官方header digest，拒绝缺失symbol、未知layout或非批准server headers
- [ ] 3.3 保留官方C size/offset/layout probe并与生成Rust bindings交叉验证，覆盖state/result/callback、magic和init signature
- [ ] 3.4 删除源码中的手写OAuth ABI struct/magic权威与冗余direct `pgrx-pg-sys`依赖，只通过完整pgrx的`pg_sys`、module magic、guard、error和allocator接口实现FFI
- [ ] 3.5 覆盖startup/validate/shutdown的null、panic、PostgreSQL ERROR、allocator、错误magic与缺失callback负向矩阵，证明任一异常在真实backend中fail closed

## 4. PG18 stable-line兼容与artifact身份

- [ ] 4.1 先修改版本门禁测试，要求`180003`、`180004`和未来PG18 numeric minor通过major检查，PG17/PG19拒绝，并删除精确`sversion == 180004`成功条件
- [ ] 4.2 实现runtime PG18 major检查并继续依赖`PG_MODULE_MAGIC`与OAuth validator magic，确保不同major和ABI变化在加载/启动前失败
- [ ] 4.3 固定每个build变体的Rust、pgrx、JOSE、PostgreSQL source/header、runtime base digest、target、arch与libc，并生成可比较artifact identity
- [ ] 4.4 在精确PG18.4 runtime完成loader、allocator、callback和OAuth smoke，记录“已验证minor”而不宣称未运行minor已获准部署

## 5. 正式只读config/JWKS runtime

- [ ] 5.1 定义版本化config schema与唯一只读文件路径契约，只允许issuer、audience、public JWKS路径及批准的公开部署参数，不允许算法、role映射、fallback issuer或私钥配置
- [ ] 5.2 先增加config/JWKS snapshot负向测试，覆盖缺失、过大、权限不安全、未知字段、非法HTTPS资源、empty/duplicate/unknown kid、private JWK和非ES256 key
- [ ] 5.3 实现每个新OAuth backend在startup读取并验证不可变config/public-JWKS snapshot，缺失或损坏时fail closed且不保留旧缓存或内置key
- [ ] 5.4 实现同文件系统原子替换轮换语义，证明后续backend读取完整active+retiring key集合、既有backend不reload且不会观察半写文件
- [ ] 5.5 增加静态与ELF依赖门禁，证明正式module不存在HTTP、DNS、libcurl、SQL、SPI、私钥、service credential、在线introspection或第二issuer/fallback

## 6. JWT、role与identity接入正式callback

- [ ] 6.1 把已验证的ES256/JWKS verifier接入无gate feature的正式validate callback，保持固定issuer/audience/database scope、30至300秒TTL和deny-unknown完整claims契约
- [ ] 6.2 覆盖OAuth client与API-key credential actor二选一、authority version、profile、role、ID字符/长度、time、algorithm、audience/scope和tampered signature正负矩阵
- [ ] 6.3 实现closed ordinary/business-admin/database-developer profile-role映射与startup requested role精确匹配，拒绝service/migration/cluster/未知role及配置扩权
- [ ] 6.4 把版本化`authn_id`接入PostgreSQL allocator，覆盖`authn_id -> system_user -> decoded identity`无歧义往返和超长/非法/未知版本拒绝
- [ ] 6.5 为认证失败建立稳定脱敏reason类别，验证日志、panic与错误不包含JWT、JWKS内容、connection string或完整内部堆栈
- [ ] 6.6 构建无`abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate`的production artifact，扫描排除内置测试JWKS/key/token、probe symbol/string和测试module

## 7. 可重复CI与供应链门禁

- [ ] 7.1 增加PR/main CI，使用full-SHA actions和read-only权限运行Rustfmt、Clippy `-D warnings`、locked unit/integration tests及所有ABI/JWT/runtime gates
- [ ] 7.2 增加Cargo依赖、RustSec与许可证审计，对完整pgrx transitive advisory逐项记录理由和复核期限，不使用全局ignore或自动放宽
- [ ] 7.3 用Docker Buildx和registry cache建立clean-checkout PG18.4 build，验证固定source/header checksum、base digest、真实loader/OAuth和最终runtime filesystem
- [ ] 7.4 增加Git history、workflow log、build context/cache、image、bundle、SBOM与manifest的secret/运行数据泄漏扫描，任一命中阻止发布
- [ ] 7.5 增加动态链接、ELF export、arch/libc、module位置、官方entrypoint与image内容门禁，证明final image只增加正式`.so`、license和公开manifest

## 8. GHCR、Release与consumer contract

- [ ] 8.1 定义并生成`release-manifest.json`，完整记录source、module/contract版本、toolchain/dependencies、PG build/test minor、header/base digest、target/libc、`.so`/OCI digest和验证结果
- [ ] 8.2 发布versioned正负向database-token、role和authn-id测试向量，让gomtmui issuer集成测试能固定消费且不包含任何真实key/token
- [ ] 8.3 建立最小权限release workflow，从同一clean checkout一次构建`ghcr.io/codeh007/mtmpg-postgres` version/short-SHA标签并输出不可变OCI digest；只有stable release额外更新`latest`
- [ ] 8.4 生成按PG/runtime target命名的tar.zst、`SHA256SUMS`、MIT license、SBOM和binary/container provenance/attestation，验证所有digest与manifest一致
- [ ] 8.5 建立immutable GitHub Release并拒绝覆盖既有tag/asset；Actions临时artifact只用于job传递且不得成为正式安装URL
- [ ] 8.6 文档化private GHCR的read-only pull credential边界，确保credential只由部署secret authority注入且不进入Compose、image、manifest或Release
- [ ] 8.7 在native、CI与prerelease门禁就绪后提交并推送`issue-116-extract-pggomtm`功能分支，确认远程commit与本地source identity一致且不合并到`main`

## 9. Prerelease、跨仓库验收与stable门禁

- [ ] 9.1 从已推送功能分支commit只发布明确short-SHA或`v0.1.0-alpha.*` prerelease，验证没有创建/更新`latest`且不存在stable/production可用误报
- [ ] 9.2 向gomtmui提供prerelease OCI digest、manifest与contract向量，等待其hard-cut change完成固定消费、真实PG18 OAuth、identity、ACL/RLS和executor candidate E2E
- [ ] 9.3 复核gomtmui验收使用与mtmpg相同的source/OCI digest，且gomtmui运行代码、build配置和active tests不再依赖本地Rust源码或`gomtm-pggomtm:*`标签
- [ ] 9.4 只有production feature、native矩阵、无gate扫描和gomtmui candidate E2E全部通过后才记录stable readiness；在功能分支尚未合并到`main`时不得创建stable Release或更新`latest`
- [ ] 9.5 演练切换新digest与滚动切回上一已验证digest，确认不热覆盖`.so`、不恢复第二源码或认证fallback，并记录既有backend不会随token撤销自动终止

## 10. 最终验证、推送与交付证据

- [ ] 10.1 从clean checkout运行全部Rust、C probe、Docker、真实PG18 OAuth、依赖/许可证、secret、SBOM/provenance和artifact隔离门禁
- [ ] 10.2 运行`git diff --check`与`openspec validate extract-and-standardize-pggomtm --strict`，确认tracked tree不含`target/`、secret、data、临时artifact或重复实现
- [ ] 10.3 完成whole-branch review并审查Git diff、release manifest与GitHub设置，把已完成跨仓库验收的功能分支以fast-forward原样推进并按Issue #116明确授权推送到`origin/main`，确认remote main与已验证branch HEAD为同一commit且不得force push
- [ ] 10.4 从该main commit为已经验证的同一OCI digest创建首个stable immutable Release与`latest`别名，确认没有重新构建、覆盖既有release或改变attestation identity
- [ ] 10.5 汇总mtmpg commit、OCI/release digest、SBOM/attestation、验证矩阵、已知限制和gomtmui consumer证据，供Issue #116在两个仓库工作全部结束后统一回填
