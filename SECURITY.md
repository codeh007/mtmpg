# pggomtm 安全政策

本页说明公开仓库的安全支持范围、私密报告渠道、最小复现信息和协同披露原则。当前没有生产支持的稳定发布版，但维护者仍接受对公开源码、持续集成（CI）与候选门禁的安全报告。

## 当前安全支持范围

当前没有任何版本获得生产安全支持：

| 对象 | 安全支持状态 |
| --- | --- |
| 稳定发布版 | 尚不存在，不提供生产支持 |
| 公开 `main`、功能分支与本地候选镜像 | 仅供开发、测试和安全评审，不是生产发布物 |
| `ghcr.io/codeh007/mtmpg-postgres` | 当前尚未发布；未来公开读取不自动获得生产安全支持 |
| `abi-gate`、`abi-runtime-gate`、`pgx-oauth-gate` | 仅供测试，不属于受支持的生产能力 |

安全报告可以覆盖以下问题：

- 认证绕过
- 原生应用程序二进制接口（ABI）或内存安全
- panic 边界
- JSON Web Token（JWT）与 JSON Web Key Set（JWKS）验证
- secret 泄露
- 构建链污染
- 测试 gate 进入无 gate 制品

请同时说明问题是否只影响测试 feature。

## 私密报告安全问题

认证或 secret 问题必须通过已经可用的私密渠道报告。GitHub 私密漏洞报告当前尚未启用，OpenSpec 任务 7.10 将在公开开发基线进入 `main` 后启用并复核该能力：

1. 如果仓库的 Security 页面显示 GitHub 私密漏洞报告入口，请通过该入口创建私密报告或 GitHub Security Advisory 草稿。
2. 当前入口不可用时，请使用仓库所有者已经向你提供的既有私密渠道。
3. 如果没有可用私密渠道，请不要把漏洞细节或敏感材料发布到普通 Issue；先创建不含漏洞细节的联系请求，让仓库所有者提供私密接收方式。

本文不提供未经验证的邮箱或固定响应时限。普通功能缺陷可以使用公开 Issue；认证绕过、secret、未修复内存安全问题或可利用供应链缺口必须保持私密。

## 公开仓库与 package 边界

仓库、Git refs、Actions 日志和协作内容都是公开面。发现真实 secret 后，先撤销或轮换，再按批准范围处置 history、日志、cache 或其他远端材料；把仓库改回 private 或删除日志不能撤销既有暴露。

后续 `mtmpg-postgres` package 将允许匿名读取，部署端不保存 private pull credential。Package 写入、删除、改标、attestation 与 GitHub Release 权限只能在受保护 `main` 的 trusted job 运行期存在，不能进入 image、manifest、Compose、文档示例或 Release asset。

## 不要提交 secret

普通 Issue、Pull Request（PR）、讨论、测试日志和构建日志不得包含以下材料：

- 私钥、API key、OAuth token、database JWT 或 authorization code
- 数据库连接串、`.env` 内容、session、credential 或 PostgreSQL data
- 未脱敏的真实 JWKS 工作副本、请求头或用户身份数据

报告应使用最小化的合成 fixture，并删除无关数据。如果材料曾经暴露在公开位置，请先撤销或轮换对应凭据，再通过私密渠道说明暴露范围。追溯式 public-readiness 不允许使用全局 ignore 或回显 secret 值。

## 报告所需信息

一份可核对的报告应包含以下信息：

- 受影响的 commit SHA、候选镜像 ID 或文件 checksum
- PostgreSQL、Rust、pgrx、操作系统、架构和 libc 的精确版本
- 最小复现步骤，以及预期结果与实际结果
- 已脱敏的错误类别、崩溃位置或日志片段
- 对认证、完整性、可用性或机密性的影响
- 已知的利用前提，以及问题是否已经公开

不要为了证明影响而访问生产数据、修改生产配置或扩大测试范围。

## 修复与披露原则

维护者会按具体问题协调复现、修复、复验和披露时间，不承诺固定服务等级协议（SLA）。在双方约定公开时间前，请保持报告、补丁和验证材料私密。

公开说明应只包含理解影响和升级所需的信息。说明不得泄露 secret、可识别用户数据或未经修复的额外利用细节。
