## Context

截至 2026-07-17，`pggomtm` 已从 `/workspace/gomtmui/native/pggomtm/` 迁入 mtmpg 根目录，gomtmui 中的本地副本和 gate 已删除。正式无 gate callback 已从外部只读 config/public JWKS 建立 per-backend snapshot，完成 ES256、actor、claims、closed profile-role、requested-role、allocator/identity、失败 reason 与 production artifact 能力门禁。当前 OpenSpec 进度为 31/54；剩余工作集中在依赖与许可证审计、公开状态修复、cold authority、final image/release 供应链、consumer contract 和跨仓库验收，而不是重写 validator 主路径。

远端 `issue-116-extract-pggomtm` 功能分支已经有连续成功的 Native CI 证据，且相对远端 `main` 包含全部迁移与实现提交；`main` 仍为只有 `.gitignore` 的初始基线。仓库已经由所有者设置为 public，但完整 public-readiness 尚未执行，默认分支尚未识别 README、MIT LICENSE、SECURITY、Cargo dependency manifest 或 workflow。GitHub 当前没有 PR、tag、Release 或 `mtmpg-postgres` package，也没有 branch protection/ruleset；auto-merge、secret scanning、push protection、dependency alerts 均未启用。Actions artifact 为零，但已存在 133 个 BuildKit cache entry，不能把仓库已公开倒推为公开前门禁已经通过。

根 `Dockerfile` 继续定义唯一 native build graph，GitHub Actions 继续是执行该 graph 并形成 task、consumer 与发布证据的唯一权威。本地命令只用于快速定位。普通 cached CI 已经证明可把完整首次构建后的重复反馈压缩到可接受时间；cold 与 trusted release lane 尚未建立。

仓库公开改变的是协作与分发状态，不改变产品职责：mtmpg 只拥有 Rust validator、native tests、PostgreSQL runtime image、release manifest 与供应链；gomtmui 继续拥有 issuer、delegation、database role/RLS、Go executor、MCP 和平台编排。当前没有建立第三个 mtmbase 源码仓库的必要。

## Goals / Non-Goals

**Goals:**

- 让 mtmpg 持续作为 `pggomtm` 源码、测试、native build、release manifest 与制品的唯一权威。
- 完成可实际加载的 PG18 离线 OAuth validator，而不把测试 gate、内置 key 或认证 fallback 带入正式 artifact。
- 以完整 pgrx 承担已验证的 PostgreSQL FFI 安全能力，同时从官方安装 header 生成最小 OAuth ABI bindings，并保持生成、校验、落盘与编译字节同一。
- 把“构建/测试针对精确 minor”与“runtime 拒绝同 major minor 升级”分开；部署仍只接受真实验证并按 digest 固定的 PG18 变体。
- 对已经发生的公开状态执行追溯式 public-readiness，处置真实 secret 和无法可信证明安全的旧 cache，再建立完整默认分支基线。
- 让受保护 `main` 表示可审计的公开开发主线，而不是 stable 发布状态；普通变更通过 Agent 管理的短期 PR 和必需检查自动合并。
- 以 cached PR/`main` CI、无缓存 cold authority 和最小权限 trusted release 分离快速反馈、独立复验与制品写入权限。
- 从受保护 `main` 的精确 source commit 只构建一次 stable candidate，以同一 OCI digest 完成 gomtmui 跨仓验收和 stable 晋级。
- 公开读取 `ghcr.io/codeh007/mtmpg-postgres`，同时保持 package 写权限只属于受信 release job。
- 区分 image 内 build manifest 与外部 release manifest，避免 OCI 制品记录自身 digest 的循环身份。

**Non-Goals:**

- 不把 pggomtm 改造成普通 SQL extension，不创建虚假的 control/extension SQL，也不使用 `cargo pgrx install/package` 作为生产交付。
- 不把 gomtmui 的 delegation 表、issuer 私钥、API-key 管理、数据库 role/RLS、MCP executor 或 Cloudflare 配置迁入 mtmpg。
- 不支持 PostgreSQL 17/19、musl、Windows、macOS 或多架构首发；第一阶段只发布 Linux amd64、PG18、Debian/glibc 变体。
- 不增加 HTTP、DNS、SQL、SPI、在线 introspection、active-grant 查询、私钥读取或认证 fallback。
- 不用 submodule、subtree、Cargo git dependency 或本地源码 override 维持双仓库开发路径。
- 不把进入 `main`、普通 CI 成功或 package 可公开拉取描述为 production-ready 或 stable。
- 不为单贡献者仓库设置必须由第二账号批准的不可满足门禁；高风险变化仍须留下显式技术审查证据。
- 不通过重新设为 private、删除日志或无理由改写 history 冒充公开泄漏处置；真实 secret 一律先吊销或轮换。
- 不在本 change 修改生产数据库、生产 Supabase、llm-gateway 或直接晋级生产流量。

## Decisions

### 1. mtmpg 根目录是单一 crate 与源码权威

仓库根目录直接包含 `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`Dockerfile`、`src/` 与 `tests/`。当前只有一个交付模块，不预建多 crate workspace 或 `native/pggomtm/` 嵌套。若未来确有第二个独立 PostgreSQL 模块，再通过单独 change 升级为 workspace。

迁移只携带源码、锁文件、Docker 定义和测试；`target/`、本地 image、secret、data 与临时证据不得复制。gomtmui 不提交本地 pggomtm 副本，也不加入 git submodule。跨仓库联调只消费明确 source commit 对应的 OCI digest。

保留源码在 gomtmui 并让 mtmpg 镜像发布，或在 gomtmui 添加 submodule，都会留下两个生命周期或源码级耦合，因此拒绝。

### 2. 保留完整 pgrx，OAuth ABI 从官方 header 生成

正式 crate 只直接依赖固定完整 `pgrx` 的 `pg18` feature，并通过 `pgrx::pg_sys` 使用 raw symbol。继续使用 `pg_module_magic!`、`pg_guard`、PostgreSQL error 和 allocator，避免自行重写 PostgreSQL longjmp/panic 与内存边界。

Build 从目标 `pg_config --includedir-server/libpq/oauth.h` 生成只包含 OAuth magic、三种 ABI struct 和 callback 类型的 bindings。生成过程关闭外部 formatter，单次 materialize 为内存字节；构建校验该精确字节后原样写入 `OUT_DIR`并比较最终 digest，不再调用 formatter、subprocess 或二次序列化。Header digest、官方 C size/offset/layout probe 与 Rust 输入继续交叉验证。

Provenance 门禁必须覆盖恶意 `RUSTFMT`、`PATH/rustfmt` 与验证后篡改尝试。只比较 header、内存字符串或 C layout 而不比较最终编译字节，不足以证明可信。

只使用 `pgrx-pg-sys`会失去完整 pgrx 已验证的 guard 与 module 能力；完全改成 C shim 会重写已经验证的边界。两者均拒绝。未来移除 pgrx 必须另开 change 并重新证明 panic、ERROR、allocator 与 loader 矩阵。

### 3. Build 精确，runtime 按 PG18 major 与 magic 兼容

每个 release 变体固定 Rust patch、pgrx、JOSE、PostgreSQL source/header 版本、官方 runtime image digest、target triple 与 libc，并在 build/release manifest 中记录。CI 必须在该精确 PostgreSQL minor 上执行 loader 和真实 OAuth 测试。

Module runtime 不要求 `sversion == 180004`。`PG_MODULE_MAGIC` 负责 PostgreSQL major ABI，OAuth callback table magic 负责 stable line 内的 OAuth ABI 变化；startup 只接受 PG18 major并依赖两种 magic 与生成 layout。消费者仍不得把一个仅验证过 18.4 的 artifact 自行装入 18.5；新 minor 必须重新构建、验证和发布新 digest。

### 4. 正式 validator 按新 backend 读取只读配置并离线验签

正式 artifact 不编译 `abi-gate`、`abi-runtime-gate` 或 `pgx-oauth-gate`。每个新 OAuth backend 在 startup 读取版本化只读 config 和 public JWKS snapshot，完成文件权限、大小、schema、issuer、audience、key 数量与类型检查；缺失、损坏或不匹配立即 fail closed。平台通过同文件系统原子替换发布材料，后续新 backend 读取新 snapshot，既有 backend 不 reload 或重新认证。

Validate callback 只执行 ES256、固定 issuer/audience/scope、iat/exp/TTL、完整 claims、actor 二选一、closed profile-role 与 requested role 检查。成功时使用 PostgreSQL allocator 返回版本化、有限长度且无 secret 的 `authn_id`；失败只给稳定脱敏 reason。已建立 backend 不重新验证，也不声称随 token 过期自动终止。

Config 只能改变批准的部署资源和 public 材料路径，不允许任意算法、claims、profile-role 映射、fallback issuer 或私钥路径。

### 5. 公开 GHCR PostgreSQL 派生 image 是主要部署物

Trusted candidate workflow 基于按 digest 固定的官方 `postgres:<minor>-bookworm` 构建最终 runtime image，只把正式 `libpggomtm.so`、MIT license 与非敏感 build manifest 放入目标 filesystem，保持官方 entrypoint，不内置 JWKS/config、数据库 data、gomtmui 源码、Rust toolchain、Cargo target 或测试 gate。

Package 固定为 `ghcr.io/codeh007/mtmpg-postgres` 并公开读取。所有写入只使用受保护 `main` 上受信 workflow 的最小 `packages: write`；公开读取不授予匿名写权限，也不改变 repository token 或 release 权限。所有构建可发布 source-discovery tag，只有 stable 额外增加 SemVer 与 `latest`；部署始终固定完整 OCI digest。

不使用通用 gomtm runtime base，也不把裸 `.so` 作为容器生产环境的主要安装路径，避免扩大攻击面或造成 arch/libc/PostgreSQL 错配。

### 6. Build manifest 与外部 release manifest 分离

Image 内的 `pggomtm-build-identity/v1` 和 build manifest 只记录在构建前可确定的 module version、features、toolchain、dependencies、PostgreSQL source/header/runtime base、target、arch、libc 与 `.so` digest。它们不得记录 image 自身尚未产生的 OCI digest，也不能冒充 release 身份。

Candidate image 产生完整 OCI digest 后，trusted workflow 从同一 build 结果生成外部 `release-manifest.json`，记录 source commit、module/contract version、PG build/test minor、header/base、target、`.so`/OCI digest与 native 验证矩阵，并把 manifest、SBOM 与 provenance 作为不可变 OCI 关联材料或 GitHub attestation 绑定到该 digest。gomtmui consumer gate只接受这些相互一致的材料。

Gomtmui E2E 另行产生绑定 release-manifest digest、OCI digest 与 consumer source 的验收证据，不改写 candidate manifest。Stable promotion 从精确 OCI digest 提取同一 `.so`，重新比较 module digest后只执行打包、checksum、tag/alias 和 GitHub Release 发布，不运行 Cargo 或重新构建 image。Release 包含原 candidate manifest、consumer evidence、tar.zst、`SHA256SUMS`、license、SBOM 与 provenance。

这样既保持 build-once/promote，也避免把包含 OCI digest 的同一 manifest 嵌入 image 形成自引用。

### 7. mtmpg 发布 consumer contract，gomtmui 只固定消费

mtmpg 在 release manifest 和测试向量中拥有 database-token contract、closed role/profile 与 `authn_id` 格式的 consumer-side 版本。gomtmui 仍拥有产品 OpenSpec、delegation authority 和 issuer，但 issuer 集成测试必须对固定 mtmpg candidate 的正负向向量通过。

gomtmui 的 Compose/platform 配置只引用 `ghcr.io/codeh007/mtmpg-postgres@sha256:<digest>`，只读挂载 config/JWKS，并在 candidate E2E 中验证 manifest、实际 server minor、OAuth 登录、identity 与 ACL/RLS。不得复制 release tarball 或在 gomtmui workflow 重建 Rust module。

跨仓库升级采用先发布 mtmpg candidate、再更新 gomtmui candidate、最后晋级同一 digest 的顺序。短期 database JWT 使 hard cut 不需要双 validator 或长期兼容 token。

### 8. 已公开状态必须追溯审计并建立一次性 main bootstrap

仓库已经公开，重新设为 private 不能撤销已公开的 ref、clone、日志或缓存暴露，因此 public-readiness 由前置门禁改为追溯处置。扫描必须覆盖全部 refs/history、当前工作树、Docker context、workflow 源码与日志、Actions artifact/cache、最终 image、GitHub Issue/PR 与将要公开的 package。Scanner 默认 redact，只输出 finding 元数据；合成 fixture 只允许按精确路径、模式和理由分类。真实 secret 先吊销或轮换，再按明确批准处理 history 和远端材料。

现有 BuildKit cache 无法逐项证明不含公开前上下文时，全部删除而不是抽样宣称安全。随后从 clean checkout 运行一次无缓存 bootstrap cold build，产生新的可信 cache 起点。Cache 删除不删除 Git history、Release 或本地源码，也不能替代 secret rotation。

默认分支尚无 workflow，常规 `workflow_dispatch` 在 bootstrap 前不能作为稳定入口。因此最终功能分支可以保留一次性、无 secret、无缓存的 feature-push cold 路径；该精确 remote HEAD 通过 public-readiness、cold CI、whole-branch review 和 source identity 核对后，使用非 force fast-forward 原样推进到 `main`。这次 bootstrap 不创建 tag、Release、package version alias 或 `latest`，也不声称 stable。

Fast-forward 完成后删除功能分支，确认默认分支识别 README、LICENSE、SECURITY、Cargo 与 workflow，然后启用仓库安全能力和 branch ruleset。一次性 feature trigger 可在后续普通 PR 中删除；它不成为永久第二 CI 实现。

### 9. 稳态 GitHub 治理由必需检查和 Agent auto-merge 驱动

公开后的 `main` 是开发主线。Ruleset 要求普通变更通过 PR、必需 Native CI、线性历史、禁止 force push 与 branch deletion，并要求讨论已解决；required approving review 数为 0，避免只有一个贡献者时形成不可满足门禁。仓库只允许 squash merge，启用 auto-merge 与合并后删分支。Agent 负责创建或更新 Issue 范围内的短期 PR、等待检查、处理失败并启用 auto-merge，用户无需手工维护 PR 生命周期。

高风险变化仍不自动放行：pgrx/JOSE、Rust toolchain、PostgreSQL minor、官方 base/header、Actions source/pin、release workflow 或权限变化必须在 PR/Issue 中记录上游 diff、风险与独立审查结论，Agent 只有在该证据存在后才能启用 auto-merge。GitHub 不要求第二账号批准不代表取消技术审查。

Dependabot 按 Cargo 与 GitHub Actions 两个生态分别分组并限制同时打开的 PR；不配置 Docker、Rust toolchain 或 PostgreSQL updater，不无条件自动合并 native 认证依赖。mtmpg 自有 Issue 跟踪后续 release 工作，并反向链接 gomtmui #116/#117。

GitHub Actions 保持批准来源、full-SHA action 引用和默认 read-only token。稳态 CI 分为三条 lane：

1. PR/`main` lane 使用 BuildKit/GitHub Actions cache、同 ref 并发取消和无 secret 的只读 token，运行唯一 Docker build graph；不登录 GHCR、不上传正式制品。
2. Cold authority 由 `schedule`、`workflow_dispatch` 和 release 前调用，从 clean checkout 无缓存验证固定 source/header/base、真实 loader/OAuth 与 final filesystem，并保存 source/run 摘要。
3. Trusted candidate/release workflow 只接受受保护 `main` ancestry 上的精确 commit，在 job 级申请 `packages: write`、`contents: write`、`id-token: write` 和 attestation 所需最小权限；PR 与 fork 代码永远不能取得这些权限。

稳态普通 CI 不再永久硬编码 Issue #116 分支。Public fork PR 使用 GitHub-hosted 临时 runner、read-only token且禁止 `pull_request_target`、release secret 和写权限。

### 10. Candidate 从 main 构建一次，stable 只晋级同一 digest

带 prerelease module version 的 alpha/RC 只验证 pipeline，不能改标为 stable，因为其 SemVer 与最终版本不同。准备 stable 时，最终 version 通过普通 PR 进入受保护 `main`，trusted candidate workflow 从该精确 main commit 只构建一次，并只发布 source-discovery tag、完整 OCI digest、candidate release manifest、SBOM 和 attestation。

mtmpg native/cold 门禁与 gomtmui candidate E2E 都验证该 source、manifest 和 OCI digest。验收期间 `main` 可以继续接收后续开发；stable tag 必须精确指向已验收 candidate source，并验证它仍是受保护 `main` 的未改写祖先，不要求发布时 `main` HEAD 仍停在该 commit。

全部门禁通过后，trusted promotion workflow为同一 OCI digest增加 SemVer/`latest` 发现别名，创建指向 candidate source commit 的 stable Git tag和 immutable GitHub Release，并从该 digest 取回已验证 bytes 生成辅助 bundle。Promotion 不重新运行 Cargo、不重建 image、不更换 attestation identity，也不把另一个构建结果解释为等价制品。

## Risks / Trade-offs

- [仓库在完整审计前已经公开] -> 不假装回到未公开状态；立即扫描全部公开表面，真实 secret 先轮换，按批准处理 history，并记录暴露窗口和处置结果。
- [旧 BuildKit cache 无法形成逐项安全证明] -> 删除现有 cache，从 clean bootstrap cold build 建立新的可信缓存起点。
- [`main` 被误解为 stable] -> README、release docs、metadata 与工具输出持续区分 development、candidate 和 stable；只有 immutable Release 与批准 digest 表示 stable。
- [单贡献者 auto-merge 缺少第二账号制衡] -> 必需远端检查、禁止 force/delete、显式高风险审查证据和完整 Issue/PR 轨迹替代不可满足的审批数，不允许 CI 失败时绕过。
- [公开 fork 可执行任意 Dockerfile 代码] -> 只使用临时 runner、read-only token、无 secret、无 package/Release/attestation写权限，并禁止 `pull_request_target`。
- [公开 GHCR 增加匿名下载但不应扩大写权限] -> Package read 公开，写入仍只来自受保护 `main` 的 trusted job；consumer继续固定 digest。
- [外部 manifest 与 image 身份可能循环] -> Image 内 build manifest不记录OCI digest；外部 release manifest在digest产生后绑定image与attestation。
- [Stable 验收期间 `main` 继续前进] -> Stable tag精确指向candidate SHA并验证其仍在main ancestry；promotion只引用已验收digest，不要求冻结整个开发主线。
- [跨仓库开发增加协调成本] -> 使用versioned manifest、测试向量、consumer evidence与digest pin，不增加submodule或本地fallback。
- [完整pgrx依赖树较大且含需持续复核的transitive advisory] -> 固定lock，运行依赖/许可证审计并逐项记录例外与复核期限，不以未验证手写FFI换取表面精简。
- [接受PG18同major可能被误解为任意minor已获部署支持] -> Runtime兼容与部署支持分离；manifest只列实际构建和验证的minor。
- [原型gate feature误入正式artifact] -> Candidate/release workflow显式禁止gate features并扫描symbol、string、内置key和final filesystem。
- [PostgreSQL module无法在既有session卸载] -> 不热替换`.so`；升级和rollback切换完整image digest并重建backend。

## Migration Plan

1. 更新本 change 的 proposal、design、release-supply-chain spec、validator spec 与 tasks，使 public 现状、main/stable 解耦、公开 GHCR 和 build-once/promote 使用同一权威语义；同步后续受影响文档。
2. 在现有远端功能分支执行追溯式 public-readiness，扫描全部 refs/history、工作树、Docker context、workflow/log、artifact/cache、GitHub 协作内容和候选 image。真实 secret 先轮换并完成批准处置；无法可信审计的旧 cache 全部删除。
3. 完成 Cargo/RustSec/license、final filesystem/ELF、secret 与 supply-chain gate，并让最终功能分支 remote HEAD 通过一次无缓存 bootstrap cold build；固定 source、run、image 与 finding 摘要。
4. 对该 exact remote HEAD 执行 whole-branch review、`git diff --check`与 strict OpenSpec validation，然后非 force fast-forward 到 `origin/main`。确认 default branch 内容完整后删除功能分支；不创建 tag、Release、package stable alias 或 `latest`。
5. 启用 secret scanning、push protection、dependency graph/alerts、private vulnerability reporting、branch ruleset、required checks 与 auto-merge；验证无第二人审批要求、禁止 force/delete、squash-only 和自动删分支。通过后用普通 PR 删除一次性 feature trigger并收敛 Dependabot。
6. 建立稳态 cached CI、scheduled/dispatch cold authority 与最小权限 trusted candidate/promotion workflow；生成外部 release manifest、versioned contract vectors、SBOM 和 attestation，并公开读取 GHCR package。
7. 将最终版本通过普通 PR 合并到 `main`，从精确 main commit 只构建一次 candidate image和外部材料。mtmpg完成 native/cold gates 后，gomtmui按同一digest运行真实 PG18 OAuth、identity、ACL/RLS和executor candidate E2E。
8. 跨仓门禁通过后验证candidate SHA仍在main ancestry，演练切换与rollback，再为同一OCI digest添加SemVer/`latest`、创建精确source tag和immutable Release；不得重新构建。
9. 汇总mtmpg source、OCI/release digest、manifest、SBOM/attestation、验证矩阵、已知限制与gomtmui consumer evidence，并分别回填mtmpg跟踪Issue和gomtmui #116/#117。

公开状态无法回滚为“从未暴露”。若追溯审计发现真实 secret，停止合并和发布，先轮换并修复前进。若 bootstrap 后 `main` CI 失败，保持无 stable 状态并通过受保护 PR 修复。若 candidate 或跨仓 E2E 失败，不创建 stable alias/Release，mtmpg发布新candidate修复。若已部署 stable失败，平台滚动切回上一已验证 OCI digest；任何 rollback 都不得恢复 gomtmui 第二份 Rust 源码或认证 fallback。

## Open Questions

- 首个 stable release 是否直接命名 `v0.1.0`，还是先用一个或多个不能晋级为 stable 的 alpha/RC 验证 pipeline；无论选择哪种，最终 stable candidate 都必须从冻结最终版本的受保护 `main` commit重新产生唯一digest并完成全部门禁。
