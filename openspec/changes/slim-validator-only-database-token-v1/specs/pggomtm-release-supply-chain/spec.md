## MODIFIED Requirements

### Requirement: mtmpg必须是精简的唯一源码与发布权威
`codeh007/mtmpg` SHALL只维护根 `pggomtm` validator crate、测试、CI 和 PostgreSQL image 定义，并 MUST删除 executor 产品全部：`executor/` 目录、根 Cargo workspace 成员、`executor-v*` release 入口与 executor CI 步骤、`docs/executor-runtime.md` 及 README/MAINTAINERS 中的 executor 引用。仓库 MUST NOT保留 executor 源码副本、第二 Dockerfile、executor image fallback 或 executor release 线。

#### Scenario: 精简为validator-only
- **WHEN** 维护者完成硬切
- **THEN** 仓库 SHALL只保留 pggomtm validator 及其唯一 Dockerfile/CI/release，executor 及其发布历史由 Git/Release 不可变记录保存

#### Scenario: gomtm/gomtmui消费pggomtm
- **WHEN** gomtm（Go sql-relay）与 gomtmui（签名上移）部署带 pggomtm 的 PostgreSQL
- **THEN** 它们 SHALL 引用 mtmpg 发布的版本化 image/宿主产物，不得重新构建 module 或维护第二套 native 矩阵

### Requirement: gomtmui必须最小化消费mtmpg release
Gomtmui SHALL在内测Compose中把PostgreSQL image设置为明确的`ghcr.io/codeh007/mtmpg:<semver>`，并 SHALL复用现有platform初始化、配置与运行契约。Gomtmui MUST NOT本地构建Rust module或mtmpg image，也 MUST NOT增加旧validator、认证fallback、private pull credential或第二份native测试矩阵。

Gomtmui SHALL删除专用mtmpg consumer workflow与测试harness。TLS、sub2api、pgAdmin、ACL/RLS、OAuth issuer和数据库SQL relay的真实集成 SHALL由gomtmui/gomtm对应领域change在功能启用时验证，不得作为mtmpg release前置条件。平台在pull、启动或备份时 SHALL记录实际resolved digest，但tracked配置 SHALL以mtmpg SemVer表达用户选择。

#### Scenario: 更新内测Compose版本
- **WHEN** gomtmui选择一个已发布mtmpg SemVer用于可重建内测平台
- **THEN** Compose与platform单一常量 SHALL引用该versioned image，且仓库不得新增专用native consumer workflow或测试目录

#### Scenario: 平台领域集成失败
- **WHEN** gomtmui后续启用TLS、profile role、ACL/RLS或数据库SQL relay时发现与某个mtmpg release不兼容
- **THEN** gomtmui SHALL在自身领域change中保持该能力停用并修复前进，不得复制mtmpg native矩阵或要求覆盖既有release
