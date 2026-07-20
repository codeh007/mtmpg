## Why

`mtmpg v0.1.0`的database-token contract v1把通用database profile映射到带项目阶段前缀的PostgreSQL role，迫使消费者维护第二套命名。Gomtmui现已选择以通用同名profile-role完成单数据库激活，因此需要一个不可变的新contract与SemVer release，而不能原地修改v0.1.0。

## What Changes

- **BREAKING**：database-token contract从v1升级到v2，只接受`ordinary`、`business_admin`和`database_developer`三个`db_profile`值，并要求`db_role`与startup requested role使用完全相同的名称。
- **BREAKING**：`authn_id` contract从`pggomtm:v1`升级为`pggomtm:v2`，identity中的profile使用v2通用名称；v1 identity、连字符profile和带项目或阶段前缀的role全部fail closed。
- 更新Rust领域、真实PG18与final-image最小矩阵，覆盖三个v2 profile-role的允许、错配与旧contract拒绝，不保留alias、role membership或兼容解码器。
- 更新运行与兼容文档，把v1标记为仅属于不可变v0.1.x release line，并把v2定义为v0.2.x唯一contract。
- 将package version提升为`0.2.0`；源码进入`main`并通过精确SHA的完整远端CI后，创建不可变`v0.2.0` tag，由现有标准workflow发布GHCR image、GitHub Release、manifest、SBOM、provenance与attestation。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `pggomtm-validator-module`：把closed profile-role与版本化identity升级为无项目、无阶段前缀的contract v2，并明确拒绝v1输入。

## Impact

- Rust领域契约：`src/database_auth.rs`及相关runtime验证与identity codec。
- 测试：Rust JWT/identity矩阵、共享OAuth fixture、真实PG18与final-image smoke。
- 发布与文档：`Cargo.toml`、README、runtime/release compatibility文档、OpenSpec与现有SemVer tag workflow。
- 消费者：gomtmui必须整体切换到`ghcr.io/codeh007/mtmpg:0.2.0`和contract v2，不得把v0.1.x token、identity或role与v0.2.0混用。
