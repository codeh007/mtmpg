# pggomtm 发布与兼容契约

公开 `main` 是开发主线，不表示已发布。可消费版本只由不可变SemVer Git tag、公开GHCR image、GitHub Release和对应标准attestation共同定义。

## 版本域

四个版本域相互独立：

| 版本 | 当前形式 | 作用 |
| --- | --- | --- |
| mtmpg SemVer | `MAJOR.MINOR.PATCH`或prerelease | 用户选择的module/image release |
| Executor SemVer | `MAJOR.MINOR.PATCH`或prerelease | 用户选择的私网executor image release |
| Database token contract | integer `2` | JWT字段和验证语义 |
| Authn ID contract | `pggomtm:v2` | `authn_id`编码与解析 |

Database token contract 2固定ES256、唯一issuer/audience、`database` scope、30至300秒TTL、deny-unknown claims、actor二选一以及closed profile-role。三个名称同时用于`db_profile`、`db_role`和startup requested role：

| `db_profile` | `db_role`与PostgreSQL role |
| --- | --- |
| `ordinary` | `ordinary` |
| `business_admin` | `business_admin` |
| `database_developer` | `database_developer` |

V0.1.x只实现database-token contract 1与`pggomtm:v1` identity，其连字符profile和`gomtm_candidate_*` role属于不可变历史。V0.2.x只实现contract 2与`pggomtm:v2`，必须拒绝v1 token、identity和role；不得提供alias、role membership、兼容decoder或fallback。改变token字段、算法、profile-role或identity编码时必须再次提升对应contract，不得原地改变已发布版本语义。

## 平台兼容

当前支持PostgreSQL 18 major、Linux amd64、Debian bookworm/glibc和Rust stable。每次CI解析PG18稳定通道的当前minor，并让development header、ABI、真实PG integration、production build和最终runtime使用同一结果。

Release材料记录实际Rust、Cargo依赖、pgrx、PostgreSQL minor/header、builder/runtime digest、module和OCI digest。这些值是该release的观测事实，不是下一次构建的预批准pin。

PostgreSQL major、架构、libc或runtime发行版变化需要显式源码变更和完整真实验证。消费者不得把PG18 artifact用于其他major。

## CI与发布

`.github/workflows/ci.yml`同时服务Pull Request、`main` push和validator/executor release调用。PR与main只运行只读门禁：

1. 生成一次Cargo.lock并解析Rust、PG18和builder/runtime digest。
2. 运行validator Rust/C ABI/真实PG18门禁与executor Rust/libpq ABI/并发OAuth/SQL门禁。
3. 按明确product只构建一次对应production OCI archive。
4. 没有SemVer tag时不写GHCR、Release或attestation。

`.github/workflows/release.yml`只由validator `v<semver>` tag进入；executor release入口只由`executor-v<semver>`进入。去除各自前缀后的version必须与目标Cargo package version相等。Release调用同一CI并上传本次run已验证的目标archive；最小写权限publish job只下载和推送该archive，不运行Cargo、不重新解析依赖，也不执行第二次Docker build。

目标version或GitHub Release已存在、tag/source/version不一致、任一门禁失败时，publish必须fail closed。失败tag不得移动、删除或复用；修复后提升SemVer并创建新tag。

## Prerelease与stable

Prerelease和stable是两个独立release：

- Prerelease例如 `v0.1.0-rc.1`，发布 `ghcr.io/codeh007/mtmpg:0.1.0-rc.1`和GitHub prerelease，不更新 `latest`。
- Stable例如 `v0.2.0`，从自己的tag重新解析、完整测试、构建和发布；全部成功后才把 `latest`更新为该stable digest。

Stable从自己的tag运行完整release，不复用prerelease制品，也不依赖gomtmui跨仓证据。不同tag即使指向相近源码，也分别保存自己的lockfile、resolved inputs、module和OCI身份。

## Release材料

每个GitHub Release至少包含：

- `Cargo.lock`
- `resolved-inputs.json`
- `verified-image.json`
- `release-manifest.json`
- `checksums.txt`
- SPDX JSON SBOM
- provenance与SBOM attestation bundle

Release manifest绑定SemVer、tag、source SHA、Cargo.lock、实际toolchain/PG18输入、module SHA-256、OCI archive hash和registry digest。对应image digest同时具有GitHub attestation与OCI registry referrer。

Actions artifact只在同一次workflow run内传递已验证archive和manifest输入，并使用短保留期。长期消费不得依赖Actions artifact，也不得发布自定义 `<version>.evidence` OCI tag。

## 消费与rollback

消费者使用明确的`ghcr.io/codeh007/mtmpg:<semver>`与按需使用`ghcr.io/codeh007/mtmpg-executor:<semver>`，不得现场编译、复制native测试矩阵或增加本地image fallback。Gomtmui的TLS、sub2api、pgAdmin、ACL/RLS、OAuth issuer和SQL tool activation由gomtmui自身领域change验证，不阻塞mtmpg release。

每次切换以完整PostgreSQL image为单位：

1. 停止接收新连接并排空旧backend。
2. 切换到目标mtmpg SemVer。
3. 启动全部backend，记录实际resolved digest并验证PG18、module、OAuth、role和identity。
4. 失败时排空backend并切回上一已验证version。

不得热覆盖 `.so`、现场编译或增加旧认证fallback。Validator只在连接认证时检查token；token到期或授权撤销不会自动终止既有backend。

发布工作由[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)和active OpenSpec change跟踪。
