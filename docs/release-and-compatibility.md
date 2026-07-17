# pggomtm 发布与兼容契约

本页定义 `pggomtm` 未来发布必须遵守的版本、制品、兼容、晋级和回退契约。当前仓库尚无 stable、GitHub Release或GitHub Container Registry（GHCR）摘要。当前代码已在PostgreSQL 18.4接入无gate production verifier并通过首个valid/tampered token smoke，但完整认证、制品与发布门禁仍未完成；后续条款不代表对应能力已经实现。

## 当前状态与本文效力

当前仓库只具备原型构建与验证基线。本页是未来发布的规范，不是已发布制品清单：

- `Cargo.toml` 中的 `0.1.0` 只是当前 crate 元数据，不代表已经发布 `v0.1.0`。
- 仓库尚未创建 stable tag、GitHub Release 或可供部署固定的 GHCR 开放容器计划（Open Container Initiative，OCI）摘要。
- 正常构建在外部材料和claims全部合规时授权并返回PostgreSQL allocator分配的版本化`authn_id`，tampered或不合规token保持未授权。
- 当前无gate startup已经从外部只读 config 和 JSON Web Key Set（JWKS）建立每backend verifier snapshot，验证同文件系统原子轮换与既有snapshot隔离，并由正式validate callback消费该snapshot。
- Production build已经验证normal dependency tree、production源码、ELF动态依赖/未解析符号与敏感字符串不包含HTTP/DNS、libcurl/libpq、SQL/SPI、私钥加载、service credential、在线introspection或issuer fallback能力。
- Runtime只接受PostgreSQL 18 major；当前build/test identity仍精确固定18.4，尚未独立构建或验证18.5，因此不得把现有artifact部署到18.5。
- PG18.4的loader、allocator、callback和真实libpq `OAUTHBEARER`正负向smoke已通过；复验范围与限制见[PG18.4 runtime/OAuth证据](evidence/issue-116/pg18.4-runtime-oauth-smoke.md)。
- 每个Cargo feature组合已生成并嵌入`pggomtm-build-identity/v1`规范JSON及其SHA-256；该build identity不包含source commit、`.so`或OCI digest，不能替代发布manifest。
- `release-manifest.json` 与 release workflow 尚未实现。

未来候选和 stable 发布必须满足本文全部适用门禁。任何缺失、未知或不匹配的发布事实都按不兼容处理。

正式runtime必须遵循[固定路径与版本化配置契约](runtime-configuration.md)。Release不得通过环境变量、PostgreSQL GUC或启动参数增加第二个配置入口。

## 三种版本不得混用

发布同时携带三个独立版本域。每个版本只描述自己的契约：

| 版本域 | 表示方式 | 描述对象 |
| --- | --- | --- |
| Module 语义化版本（Semantic Versioning，SemVer） | `MAJOR.MINOR.PATCH`，Git tag 增加 `v` 前缀 | crate、源码与整个发布 |
| `database-token` contract | manifest 中的整数，初始值为 `1` | Database JSON Web Token（JWT）的字段和验证语义 |
| `authn-id` contract | identity 编码前缀，当前为 `pggomtm:v1` | `authn_id` 的编码与解析 |

`authority_version` 不是上述任一 contract version。它是 token 和 identity 中单个授权状态的版本，用于归因与授权状态演进。

### Module SemVer

Module SemVer 标识一次完整发布。Git、crate 和 Release 必须使用同一个版本：

- Stable Git tag 使用 `vMAJOR.MINOR.PATCH`，例如 `v1.2.3`。
- `Cargo.toml` crate version、manifest module version 和 GitHub Release version 必须去除或增加 `v` 后精确对应。
- Prerelease 使用 `v0.1.0-alpha.N` 或 `v0.1.0-rc.N`，其中 `N` 是递增正整数。
- `1.0.0` 前，任何 contract breaking 变更至少提升 minor，并显式协调所有 consumer。
- `1.0.0` 前后，patch 都不得破坏已经发布的 contract。
- `1.0.0` 后，任何 contract breaking 变更必须提升 major。

Module SemVer 变化不能替代下述 contract version 变化。一次发布可以同时提升 module、`database-token` 和 `authn-id` 版本。

带 prerelease module version 的 artifact 永远不能改标为 stable。例如，`0.1.0-alpha.1` 与 `0.1.0` 是不同 module version，二者必须使用不同构建和 OCI digest。

### Database token contract

首个未来 `release-manifest.json` 必须声明 `database-token` contract integer `1`。Contract `1` 固定以下验证语义：

- Claims schema 使用 deny-unknown，并要求完整字段集合。
- 签名算法固定为 `ES256`，不得接受替代算法。
- `issuer` 和 `audience` 必须分别精确匹配配置的唯一超文本传输安全协议（HTTPS）资源。
- `scope` 必须精确等于 `database`。
- Token 生存时间不得少于 `30s`，且不得超过 `300s`。
- Actor 必须恰好选择一种：OAuth 的 `client_id` 或应用程序编程接口（API）key 的 `credential_id`。
- `auth_method`、actor 字段和 identity 归因必须一致。
- `db_profile` 和 `db_role` 必须使用下表的闭集映射。

Contract `1` 的 profile 与 PostgreSQL role 映射如下：

| `db_profile` | `db_role` |
| --- | --- |
| `ordinary` | `gomtm_candidate_ordinary` |
| `business-admin` | `gomtm_candidate_business_admin` |
| `database-developer` | `gomtm_candidate_database_developer` |

任何 token 字段、字段语义、profile-role 映射、算法或 deny-unknown schema 变化都必须提升 contract integer。新 contract 必须随发布提供版本化正向和负向测试向量，不得原地改变整数 `1` 的含义。

### Authn ID contract

当前 identity codec 使用 `pggomtm:v1` 前缀。该前缀标识 `authn_id` 的字段、顺序、分隔、规范编码和解析规则，不代表 production verifier 已经完成。

任何编码或解析变化都必须使用新前缀，例如 `pggomtm:v2`。发布不得静默改变 `pggomtm:v1`，也不得把未知前缀解释为 `v1`。

## 首发 PostgreSQL 与 runtime 变体

首个可发布变体只覆盖一个精确构建和运行组合。它不授权同一 PostgreSQL major 的其他 minor：

| 维度 | 首发值 |
| --- | --- |
| PostgreSQL build 与 test minor | `18.4` |
| `PG_VERSION_NUM` | `180004` |
| Runtime 发行版 | Debian bookworm |
| 操作系统 | Linux |
| 架构 | amd64 |
| C library | glibc |
| Rust target | `x86_64-unknown-linux-gnu` |

每次构建和真实测试必须精确记录 PostgreSQL minor、OAuth header digest 与 runtime base image digest。Manifest 还必须把这些值绑定到对应 source commit 和 OCI digest。

未来 PostgreSQL 18 minor 只有经过独立构建、真实 loader 与 OAuth 验证后，才能作为新变体发布。Consumer 不得把只验证 PostgreSQL 18.4 的 artifact 自行用于 PostgreSQL 18.5，即使未来 module runtime 改为接受 PostgreSQL 18 major。

## 命名与不可变制品身份

名称只帮助发现制品，不能作为部署身份。Bundle 模式固定为 `pggomtm-<version>-pg18.4-linux-amd64-glibc.tar.zst`：

| 对象 | 示例 |
| --- | --- |
| Prerelease Git tag | `v0.1.0-alpha.1` |
| Prerelease OCI version tag | `ghcr.io/codeh007/mtmpg-postgres:0.1.0-alpha.1-pg18.4-bookworm` |
| Stable candidate 的缩短 source commit hash tag | `ghcr.io/codeh007/mtmpg-postgres:sha-1a2b3c4-pg18.4-bookworm` |
| Stable OCI version tag | `ghcr.io/codeh007/mtmpg-postgres:0.1.0-pg18.4-bookworm` |
| Stable bundle | `pggomtm-0.1.0-pg18.4-linux-amd64-glibc.tar.zst` |

`sha-1a2b3c4` 表示一个缩短的 source commit hash。Stable candidate 只发布该 tag，但其 crate、module 和 manifest 已使用冻结的最终 `0.1.0` version。

Git tag、OCI tag 和 `latest` 都只能用于发现。部署必须使用完整 OCI digest，例如 `ghcr.io/codeh007/mtmpg-postgres@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`。

上述 64 位十六进制值是明显非真实的格式占位，不对应任何已发布 image。Release、manifest 和部署配置不得包含 registry 凭据。

## Release manifest 契约

当前build script已经生成较小的`pggomtm-build-identity/v1`，记录module version、feature组合、Rust/pgrx/JOSE、PostgreSQL source/header/bindings/runtime base以及target、OS、architecture和libc。它用于在release pipeline前比较build变体；正式`release-manifest.json`仍必须在后续发布阶段加入remote source commit、test minor、最终`.so`与OCI digest、验证矩阵、SBOM和attestation，且不得把build identity冒充已发布制品身份。

未来 `release-manifest.json` 必须提供不可变、可比较的发布事实。它至少记录以下字段：

- Source commit、module version、`database-token` contract integer 和 `authn-id` version。
- Rust、pgrx 与 JSON Object Signing and Encryption（JOSE）版本。
- PostgreSQL build minor、test minor 与 `PG_VERSION_NUM`。
- OAuth header digest 与 runtime base image digest。
- Rust target、操作系统、架构与 libc。
- `.so` digest、OCI digest 与完整验证矩阵。

GitHub Release 还必须提供与 manifest 一致的 `SHA256SUMS`、MIT license、软件物料清单（software bill of materials，SBOM）和 provenance。Manifest、checksum、Release asset 与 attestation 创建后不得覆盖。

gomtmui 在更新消费 digest 前必须验证全部 manifest 字段。它还必须验证 issuer、profile、platform 和版本化正负向 token、role、identity 测试向量。

## Prerelease 与 stable candidate 的独立顺序

带 prerelease module version 的 alpha 或 release candidate（RC）只能验证 pipeline 和早期 contract。它永远不能直接增加 stable version tag、stable GitHub Release 或 `latest`，也不能作为 stable digest 晋级。

Alpha 或 RC 使用与其 prerelease module version 一致的 crate、manifest、Git tag 和 OCI version tag。准备 stable 时，维护者必须创建冻结最终 version 的新 commit，并产生新的构建与 OCI digest。

Stable candidate 必须按以下顺序从最终 version commit 晋级：

1. 在功能分支冻结最终 `MAJOR.MINOR.PATCH`，让 crate、module 和 manifest version 精确一致，再推送该 commit。
2. 只从该 commit 构建一次，并只发布 `sha-<short_commit>` 形式的缩短 source commit hash candidate 身份。
3. Candidate 阶段不得发布 stable Git tag、OCI stable version tag、stable GitHub Release 或 `latest`。
4. mtmpg 完成候选门禁后，gomtmui 使用相同 source commit 和 OCI digest 完成端到端（end-to-end，E2E）验证。
5. 全部门禁通过后，以 fast-forward 让 `main` 指向同一个已验证 commit。
6. `main` 指向该 commit 后，才创建 stable Git tag、GitHub Release、OCI stable version tag 与 `latest`，并让它们引用同一个已验证 OCI digest。

创建 stable tag 和 Release 不得触发重建。后续环境必须晋级同一 OCI digest，不能从 tag 重建等价制品。

## Stable 发布门禁

Stable 表示生产发布门禁全部通过，不表示单项原型测试成功。当前原型不满足以下门禁，因此禁止发布 stable：

- Production feature 从外部只读 config 和 public JWKS 建立并在正式validate callback中使用 verifier，不依赖内置材料。
- 最终 artifact 不包含 `abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate`、测试 key、token、probe 或认证 fallback。
- 精确首发变体通过 native build、应用程序二进制接口（ABI）、loader、allocator 和 callback 门禁。
- 真实 PostgreSQL 18 OAuth allow 与 deny、role 和 identity 正负向矩阵全部通过。
- Dependency、license、动态链接和 secret 扫描全部通过。
- SBOM、binary 与 container provenance 和 attestation 全部生成并验证。
- gomtmui 对相同 source commit 和 OCI digest 的 candidate E2E 全部通过。
- Immutable manifest、bundle、checksum、OCI digest 和验证矩阵完全一致。
- 新 digest 与上一已验证 digest 的升级和 rollback 演练通过。

任一门禁缺失或失败都必须阻止 stable Release 和 `latest` 更新。流程不得通过弱化扫描、移除失败证据或改用另一构建结果继续发布。

## 升级、环境晋级与 rollback

升级和 rollback 都以完整 immutable OCI digest 为最小单位。每次切换按以下顺序执行：

1. 停止接收新连接并排空受影响的 PostgreSQL backend，等待旧 backend 全部退出。
2. 把整个 runtime image 引用切换到目标完整 OCI digest。
3. 启动或重建全部 backend，再验证实际 PostgreSQL、module、OAuth、role 和 identity。
4. 后续环境重复相同步骤，并晋级同一个 OCI digest。

切换失败时，平台先停止或排空 backend，再切回上一已验证 digest，最后重启并验证。mtmpg 必须通过新版本修复前进，不能覆盖失败版本。

发布与部署禁止以下操作：

- 在仍有 backend 运行时热覆盖 `.so`。
- 在目标主机安装 Rust 或现场编译 module。
- 在消费仓库恢复第二份源码或本地 rebuild 路径。
- 增加旧 verifier、旧协议适配器、备用 issuer 或其他认证 fallback。
- 使用 `latest`、version tag 或缩短的 source commit hash tag 代替完整部署 digest。

Validator 只在连接认证时检查 token。Token 到期或授权撤销不会自动终止已经建立的 backend，平台必须通过连接排空或其他会话生命周期控制结束它。

## Consumer 兼容决策

Consumer 只接受与目标环境和自身契约完全匹配的 manifest。以下任一维度缺失、未知或不匹配时都必须 fail closed：

| 维度 | 必须满足的条件 | 不匹配时的动作 |
| --- | --- | --- |
| Module SemVer | 与所选 Release、manifest 和批准版本精确一致 | 拒绝 candidate |
| `database-token` contract | 与 issuer 支持的整数精确一致 | 拒绝 token 与 artifact |
| `authn-id` version | 与 identity consumer 支持的前缀精确一致 | 拒绝 artifact |
| PostgreSQL minor | 与 manifest 的 build 和 test minor 精确一致 | 拒绝部署 |
| Rust target 与架构 | 与目标 Linux amd64 平台精确一致 | 拒绝部署 |
| libc | 与目标 glibc runtime 精确一致 | 拒绝部署 |
| Runtime base digest | 与已验证 base image digest 精确一致 | 拒绝部署 |
| OCI digest 与验证矩阵 | Digest、manifest 和成功矩阵相互一致 | 拒绝晋级 |

Consumer 不得用本地 rebuild、旧协议适配或可变 tag 修复不匹配。缺少兼容变体时，mtmpg 必须先发布新版本、新 manifest、新测试向量和新 OCI digest。
