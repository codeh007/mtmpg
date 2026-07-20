## Context

`mtmpg v0.1.0`发布的database-token contract v1使用`ordinary`、`business-admin`、`database-developer`三个profile，并把它们映射到`gomtm_candidate_*` PostgreSQL role。该映射编译在validator中，正确地不能由runtime config扩展，但名称把消费者项目和阶段泄漏进了通用module contract。Gomtmui的新单数据库设计要求profile、signed role与startup role使用同一组通用名称。

V0.1.0 tag、image和contract必须保持不可变。Mtmpg本地又禁止Cargo、Docker和临时PostgreSQL重计算，因此行为变更只能通过测试先行的远端`main` CI验证，再由新的SemVer tag进入现有Release workflow。

## Goals / Non-Goals

**Goals:**

- 只保留`ordinary`、`business_admin`和`database_developer`三个profile，并让PostgreSQL role与profile精确同名。
- 把database-token contract与authn-id contract同时升级到v2，明确拒绝全部v1名称和identity。
- 保持算法、claims字段、TTL、actor、runtime config、OAuth ABI、reason-code和零网络边界不变。
- 通过现有标准CI与SemVer tag workflow发布并核验`v0.2.0`。

**Non-Goals:**

- 不修改或覆盖`v0.1.0` tag、image、Release或`latest`历史。
- 不提供旧名称alias、role membership、双contract decoder、runtime可配置role映射或认证fallback。
- 不增加JWT字段、config字段、HTTP/SQL依赖、第二validator或消费者专用测试矩阵。
- 不在本地运行Cargo、原生编译、Docker或PostgreSQL。

## Decisions

### 1. Profile和role使用完全相同的下划线名称

Contract v2的闭集固定为`ordinary`、`business_admin`和`database_developer`。`DatabaseProfile`的Serde值、规范字符串与`database_role()`结果必须一致。下划线形式是未加引号的合法PostgreSQL identifier，也适合JSON和`authn_id`，避免连字符role所需的quoted identifier。

不采用`mtmpg_*`、`gomtm_*`或`*_candidate_*`前缀，因为module是跨消费者发布物，项目和阶段属于部署边界。也不保留v1名称到v2名称的映射，因为alias会让旧token继续通过并形成第二套权限入口。

### 2. Token policy和identity一起提升为v2

Database-token contract是发布文档声明的closed policy，不新增调用方可覆盖的version claim。V0.2.x只接受v2名称；使用v1 profile或role的token在严格claims反序列化或closed mapping检查中失败。

`AUTHN_ID_PREFIX`提升为`pggomtm:v2`，encoder只产生v2，decoder只接受v2。Identity的字段、顺序和长度约束保持不变，但profile值使用下划线名称。Runtime config仍使用`pggomtm-validator-config/v1`，因为issuer、audience和JWKS文件结构没有变化；配置schema版本不冒充token或identity contract版本。

### 3. 其他安全与ABI契约保持字节级边界不变

JWT仍使用ES256、唯一issuer/audience、`database` scope、30至300秒TTL、deny-unknown claims和actor二选一。OAuth callback ABI、module magic、snapshot读取、reason-code与production feature集合不变。本change不借版本升级扩大权限或增加配置开关。

### 4. 远端RED/GREEN证明breaking行为

先只修改Rust领域测试、共享fixture和真实PG18/final-image输入，使它们要求v2名称、v2 identity并拒绝v1；该精确SHA推送到`main`后，GitHub CI必须因旧production实现产生预期RED。随后修改最小production代码和文档，再由下一精确SHA取得完整GREEN。完整profile/role/identity矩阵继续只位于Rust领域测试；PG18 harness验证真实startup role与`system_user`，final-image只保留最小allow/deny smoke。

### 5. V0.2.0只由既有SemVer workflow发布

Production实现GREEN后把`Cargo.toml` version设为`0.2.0`并进入`main`。只有精确main SHA的完整CI成功且OpenSpec、文档和secret扫描通过后才创建不可变`v0.2.0` tag。Release workflow必须复用同一CI定义，发布`ghcr.io/codeh007/mtmpg:0.2.0`、GitHub Release、manifest、Cargo.lock、SBOM、provenance和attestation；发布后匿名核对source、version、module/image digest和标准证明。

## Risks / Trade-offs

- **V1调用方全部失效** -> 这是显式breaking release；消费者必须一次性切换issuer claims、PostgreSQL role/HBA和identity helper，不提供混合运行。
- **Profile改为下划线会影响持久delegation值** -> Gomtmui当前database可重建，migration与TypeScript契约在消费v0.2.0时同步硬切。
- **测试先行会让main短暂失败** -> 仓库规则明确允许main失败并要求后续前进修复；RED SHA不得打tag或发布。
- **Tag workflow在写入阶段失败** -> `v0.2.0` tag不得移动或复用；先以main完整CI降低风险，若仍形成不可消费release则按现有供应链规范精确清理孤立对象并发布更高patch，不覆盖历史。

## Migration Plan

1. 提交并验证本change完整OpenSpec工件。
2. 提交只含v2期望的测试与fixture，推送`main`并记录精确RED run。
3. 修改`DatabaseProfile`、role闭集、`pggomtm:v2` identity、package version和当前文档，推送`main`并取得精确GREEN run。
4. 创建并推送不可变`v0.2.0` tag，等待Release workflow全部完成。
5. 匿名核对GitHub Release、GHCR image、manifest、SBOM、provenance、attestation及`latest`，再把结果交给gomtmui消费change。
6. 消费失败时保持gomtmui SQL能力停用；需要回滚时排空backend并整体切回v0.1.0，同时恢复v1 issuer、role和identity helper，不做跨contract热切换。

## Open Questions

无。用户已经明确选择v0.2.0、通用名称和无`gomtm`/`candidate`前缀的单一实现。
