# pggomtm 发布与兼容契约

公开`main`是开发主线，不表示stable。可消费candidate与stable的状态以GitHub Release、GHCR和对应attestation为准。

## 版本域

三个版本域相互独立：

| 版本 | 当前形式 | 作用 |
| --- | --- | --- |
| mtmpg SemVer | `MAJOR.MINOR.PATCH` | 用户选择的module/image release |
| Database token contract | integer `1` | JWT字段和验证语义 |
| Authn ID contract | `pggomtm:v1` | `authn_id`编码与解析 |

Database token contract 1 固定ES256、唯一issuer/audience、`database` scope、30至300秒TTL、deny-unknown claims、actor二选一以及closed profile-role：

| `db_profile` | PostgreSQL role |
| --- | --- |
| `ordinary` | `gomtm_candidate_ordinary` |
| `business-admin` | `gomtm_candidate_business_admin` |
| `database-developer` | `gomtm_candidate_database_developer` |

改变token字段、算法、profile-role或identity编码时必须提升对应contract；不得原地改变已发布版本语义。

## 平台兼容

当前支持边界是PostgreSQL 18 major、Linux amd64、Debian bookworm/glibc和Rust stable。每次CI解析PG18稳定通道的当前minor，并让development header、ABI、真实PG integration和最终runtime使用同一结果。

Release材料必须记录实际Rust、Cargo依赖、pgrx、PostgreSQL minor/header、builder/runtime digest、target、架构、libc、module和OCI digest。这些值是该release的观测事实，不是源码中下一次构建的预批准pin。

PostgreSQL major、架构、libc或runtime发行版变化需要新代码和完整真实验证。消费者不得把PG18 artifact用于其他major。

## Candidate

Candidate使用不可覆盖的SemVer prerelease tag，例如：

```text
ghcr.io/codeh007/mtmpg-postgres:0.1.0-rc.123456
```

Native CI按以下顺序生成candidate：

1. 从精确main source解析一次Cargo.lock、toolchain、PG18和浮动image digest。
2. 复用该解析结果运行领域、ABI、真实PG18和final-image门禁。
3. 只构建一次production image并物化OCI archive。
4. 只读job验证archive并上传Cargo.lock、`resolved-inputs.json`和测试结果。
5. 最小写权限publish job推送同一archive，不运行Cargo或Docker build。
6. Push后生成release manifest、SBOM、provenance、attestation和checksums。

Candidate tag、source和OCI digest必须一一对应，已有tag不得覆盖。PR、fork、失败main和重复version不得写package或发布材料。

## Consumer验收

Gomtmui按candidate SemVer配置PostgreSQL image，并在远端解析实际OCI digest。Consumer evidence必须绑定mtmpg version/source/manifest/module/OCI digest与gomtmui source，并覆盖：

- 官方initdb、volume、healthcheck与PG18启动
- TLS、sub2api与pgAdmin连接
- Production module从真实`pkglibdir`加载
- Database JWT allow/deny、profile-role和`system_user`
- Ordinary、business-admin、database-developer的ACL/RLS
- 切回上一已验证version的rollback

失败时保持上一version，mtmpg通过新main commit发布新candidate；不得覆盖失败candidate或增加本地fallback。

## Stable promotion

Stable promotion只接受已经通过mtmpg和gomtmui全部证据的candidate digest。Promotion不得解析新依赖、运行Cargo或构建image，只能：

1. 为同一digest增加稳定SemVer与`latest`tag。
2. 创建指向candidate source的精确Git tag和immutable GitHub Release。
3. 发布同一Cargo.lock、resolved inputs、manifest、SBOM、provenance、checksums和consumer evidence。

用户使用mtmpg SemVer交流和升级；OCI digest用于机器校验candidate、consumer evidence和stable是否为同一bytes。Tag、Release和asset创建后不得覆盖。

## 升级与rollback

每次切换都以完整mtmpg image为单位：

1. 停止接收新连接并排空旧PostgreSQL backend。
2. 切换到目标mtmpg SemVer。
3. 启动全部backend，验证实际digest、PG18、module、OAuth、role和identity。
4. 失败时排空backend并切回上一已验证version。

不得热覆盖`.so`、现场编译、恢复第二份源码或增加旧认证fallback。Validator只在连接认证时检查token；token到期或授权撤销不会自动终止既有backend。

发布工作由[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)和active OpenSpec change跟踪。
