## ADDED Requirements

### Requirement: mtmpg必须隔离维护validator与executor两个产品package
`codeh007/mtmpg` SHALL在一个Cargo workspace中维护根`pggomtm` validator package与唯一`executor/` companion package，并 SHALL共享一个CI解析的Cargo.lock和一个纯Rust database-token contract权威。Pgrx/server ABI依赖 MUST只进入validator；HTTP/TLS/libpq client依赖 MUST只进入executor。消费者 MUST NOT复制任一源码、crate、Dockerfile、native测试或现场构建路径。

#### Scenario: 构建validator与executor
- **WHEN** CI分别构建两个product
- **THEN** validator image SHALL只包含PG18 module，executor image SHALL只包含service binary/libpq runtime，且两者不得包含另一产品的runtime、source或private material

#### Scenario: Contract发生变化
- **WHEN** database-token profile、role或claims schema需要修改
- **THEN** 变更 SHALL只修改共享纯Rust contract并同时运行validator/executor门禁，不得维护第二struct、mapping或decoder

### Requirement: Executor CI必须复用当前稳定输入并验证真实行为
PR、`main`与executor release SHALL复用仓库标准只读CI，一次解析Rust stable、共享Cargo.lock、PG18 development/runtime、libpq和builder/runtime image identity。Executor门禁 SHALL运行Rustfmt、Clippy、Rust领域、当前libpq C/Rust layout、HMAC/TLS、不同principal并发OAuth、真实PG18 extended protocol/rollback/budget/cancel及最小final-image验证。测试 MUST验证行为，不得断言Dockerfile/workflow字面量、精确上游patch/hash、layer或完整image config。

#### Scenario: Main提交executor变更
- **WHEN** 一个无release tag的main commit修改executor、共享contract或CI
- **THEN** 标准CI SHALL验证validator与executor且不得发布image、Release或attestation

#### Scenario: Libpq hook隔离失败
- **WHEN** 并发真实PG18测试观察到token串接、registry残留、未知连接放行或取消后继续执行
- **THEN** CI SHALL失败并阻止全部executor发布

### Requirement: Executor release必须使用独立不可变版本身份
Executor SHALL使用自身Cargo package version、annotated `executor-v<semver>` tag、`ghcr.io/codeh007/mtmpg-executor:<semver>` image和独立GitHub Release。Tag version MUST与executor package精确一致，且 MUST NOT触发、移动、覆盖或更新validator的`v<semver>` tag、`ghcr.io/codeh007/mtmpg:<semver>`、Release或`latest`。初始`executor-v0.1.0`历史身份 SHALL保持不可变；若该发布因供应链材料不完整而不可消费，修复 SHALL递增patch并使用新的tag、image与Release，当前前向修复目标为`executor-v0.1.1`。

只读CI SHALL从tag精确source物化一次已验证executor OCI archive；最小写权限publish job SHALL只推送该archive并生成manifest、checksums、Cargo.lock、resolved inputs、SPDX SBOM、provenance和GitHub attestation，不得重新resolve、Cargo build或Docker build。全部Release附件 MUST在draft状态上传并核验，随后同一Release才可发布并冻结；正式发布后 MUST NOT再上传或替换附件。Gomtmui SHALL只消费明确SemVer和匹配resolved digest，不使用executor `latest`或本地fallback。

#### Scenario: 前向发布可消费的executor stable
- **WHEN** `executor-v0.1.0`已是不可变但无附件的失败历史，且annotated `executor-v0.1.1`指向精确main GREEN ancestry、version匹配、目标身份不存在并且全部门禁通过
- **THEN** workflow SHALL一次发布`mtmpg-executor:0.1.1`及具有完整附件的独立immutable Release，validator v0.2.0 tag、image、Release与latest身份保持不变

#### Scenario: Draft附件上传失败
- **WHEN** 任一manifest、checksums、Cargo.lock、resolved inputs、SBOM、provenance或attestation在draft阶段缺失或核验失败
- **THEN** workflow SHALL不得发布或冻结该Release，也不得把不完整身份交给消费者

#### Scenario: 目标version已存在
- **WHEN** tag、GHCR version或GitHub Release任一目标身份已经存在或材料不一致
- **THEN** release SHALL fail closed且不得覆盖、移动tag、重建image或更新任一product的latest

### Requirement: Executor final image必须最小、非root且无构建与secret材料
Executor image SHALL只包含release binary、匹配PG18 libpq runtime、CA certificates与MIT license，并 SHALL以固定非root UID/GID启动versioned HTTPS service。Image MUST NOT包含Rust/C toolchain、Cargo、source、tests、fixture、PostgreSQL server、PGDATA、validator module、JWT/JWKS、private key、HMAC secret或credential。运行时secret和TLS material SHALL只通过只读mount提供。

#### Scenario: 验证最终executor image
- **WHEN** CI启动本次OCI image并运行最小HTTPS/HMAC/OAuth allow-deny smoke
- **THEN** service SHALL以非root身份ready、动态链接匹配libpq且不含构建/secret材料，否则发布失败
