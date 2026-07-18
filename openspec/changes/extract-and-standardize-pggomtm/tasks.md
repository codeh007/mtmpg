## 当前基线

`pggomtm` 的OAuth validator主路径已经实现，完整源码已非force进入远端`main`，临时功能分支已删除。Latest-compatible领域、ABI、真实PostgreSQL、final-image及旧candidate/evidence已由run`29646533596`完整通过，但该run使用已废弃的`mtmpg-postgres`名称、run-ID prerelease和定制OCI evidence，只作为重构基线，不完成新的标准SemVer release任务。

任何本地Docker build/run、原生编译、临时PostgreSQL或image检查结果都不能完成以下任务。`main`验证失败时保留源码并通过后续commit修复；没有显式SemVer tag时不得发布image。

## 1. Main与规划基线

- [x] 1.1 将`issue-116-extract-pggomtm`非force fast-forward到远端`main`，把本地工作线切换到同一main，删除workflow临时分支trigger及本地/远端临时分支
- [x] 1.2 确立main-first、Actions-only边界，精确清理旧本地诊断container/image且不执行宽泛prune
- [x] 1.3 按gomtmui #116评论5010761180重写proposal、design、两份delta spec和tasks，并通过严格OpenSpec校验
- [x] 1.4 按gomtmui #116评论5011465794将规划收敛为可复用只读CI、SemVer tag release、`ghcr.io/codeh007/mtmpg`和最小gomtmui消费，并通过严格OpenSpec校验

## 2. 仓库文件与文档精简

- [x] 2.1 删除`SECURITY.md`、`CONTRIBUTING.md`并清理README、MAINTAINERS及其他文件中的全部失效引用
- [x] 2.2 删除`examples/`，裁剪OAuth smoke fixture并迁移到`tests/support/`的test-only target，确认真实PG18 smoke继续使用它
- [x] 2.3 删除`docs/evidence/issue-116/`、历史治理和依赖审计快照，以Git、Actions、Release和attestation保留历史权威
- [x] 2.4 精简README、AGENTS、MAINTAINERS及保留的运行/发布文档，只描述当前产品契约、PG18 major、使用方式和release入口
- [x] 2.5 保留`src/runtime_config.rs`生产逻辑，裁剪`src/runtime_config/tests.rs`重复矩阵并把必要私有单元测试合并回扁平`cfg(test)` module
- [x] 2.6 审查清理后的tracked files与引用图，确认每个保留文件直接服务production、远端验证或发布且不存在历史说明副本

## 3. Latest-compatible工具链与构建

- [x] 3.1 将`rust-toolchain.toml`与builder切换到Rust `stable`，删除rustc/cargo patch、输出文本和builder digest的固定断言
- [x] 3.2 将Cargo依赖从精确`=`改为兼容版本范围，停止提交release用`Cargo.lock`并删除源码/测试中的固定crate patch断言
- [x] 3.3 将PostgreSQL development/runtime切换到PG18稳定通道，删除固定minor、source archive/hash、header hash和runtime digest常量
- [x] 3.4 精简`build.rs`为当前目标PG18 header的bindgen allowlist与必要ABI生成，删除预批准技术栈、bindings hash和复杂内部identity拼装
- [x] 3.5 精简Dockerfile为浮动稳定builder/runtime默认值及CI临时digest参数，只构建production module并复制module、license和最小版本信息
- [x] 3.6 将GitHub Actions改用官方稳定major tag，移除Action commit SHA和自维护scanner/audit archive版本及checksum

## 4. 测试与辅助入口精简

- [x] 4.1 将现有测试逐项映射到validator与supply-chain requirements，标记真实领域/系统风险和仅验证实现形状的测试
- [x] 4.2 删除Dockerfile/workflow/脚本字面量、精确版本/hash、layer/config相等、文件不存在及伪造工具入口自测
- [x] 4.3 裁剪JWT、role、identity、runtime config和失败原因的重复矩阵，保留每个规范边界的最小正负向覆盖
- [x] 4.4 合并官方PG18 header生成、C/Rust layout和导出symbol测试，删除针对固定bindings bytes和ambient formatter的过度provenance矩阵
- [x] 4.5 精简真实PostgreSQL harness，使用本次解析的PG18 minor验证module加载、OAuth allow/deny、role、`system_user`和错误脱敏
- [x] 4.6 删除现有复杂`scripts/`入口；将Cargo编排交给workflow、secret scan交给标准Action、PG/image harness放入`tests/`，只在确有多处复用时保留小型单职责入口
- [x] 4.7 建立精简final-image检查，验证官方entrypoint/initdb、PG18、真实`pkglibdir`、module加载、动态依赖、OAuth smoke及无private/test material

## 5. Latest-compatible核心门禁基线

- [x] 5.1 增加resolve step，生成一次临时Cargo.lock、解析builder/runtime完整digest并输出`resolved-inputs.json`
- [x] 5.2 让fmt、Clippy、Cargo tests、ABI、production module和PG18 integration复用同一lockfile、minor和解析结果，检测任何job漂移
- [x] 5.3 用标准source/secret和dependency/license检查替代自定义历史collector及其自测，保持PR/fork lane完全只读
- [x] 5.4 从同一resolved inputs只构建一次production image，完成final-image验证后物化可传递OCI archive及module identity
- [x] 5.5 将精简实现直接提交到`main`，只依据精确SHA的远端Actions日志前进修复，并由run`29646533596`取得resolve、领域、ABI、PG18和final-image完整成功基线

## 6. 测试去重与标准CI

- [x] 6.1 将Rust领域、真实PG18与final-image现有测试逐项映射到风险边界，识别完整JWT/profile/role/identity矩阵和fixture/client/staging的重复定义
- [x] 6.2 合并重复矩阵与support入口，保留单一Rust领域权威、单一真实PG18 backend harness和最小final-image allow/deny smoke
- [x] 6.3 用支持`pull_request`、`main` push与`workflow_call`的可复用只读`ci.yml`替代复杂`native-ci.yml`，移除candidate、ORAS evidence和跨仓consumer逻辑
- [x] 6.4 让可复用CI在release调用时上传同一run已验证OCI archive、Cargo.lock、resolved inputs与manifest输入，并证明后续publish无需重新运行Cargo或Docker build
- [ ] 6.5 配置required CI与GitHub原生auto-merge，只允许owner、Agent或批准的Dependabot PR自动合并，外部PR保持人工批准且维护者/Agent可直接非force推进`main`
- [ ] 6.6 取得PR与`main`的精确远端成功run，证明两者没有packages、Release、attestation或跨仓写权限，并更新README/维护文档的CI入口

## 7. 首个标准SemVer prerelease

- [ ] 7.1 将Cargo package version设为首个合法prerelease（`0.1.0-rc.1`），并要求去除前导`v`后的Git tag与package version精确一致
- [ ] 7.2 增加只由合法SemVer tag触发的`release.yml`，调用可复用CI并以最小写权限下载和推送同一已验证OCI archive
- [ ] 7.3 发布公开`ghcr.io/codeh007/mtmpg:0.1.0-rc.1`与GitHub prerelease，拒绝既有version、错误source、失败门禁和任何`latest`更新
- [ ] 7.4 生成精简release manifest、Cargo.lock、resolved inputs与checksums作为Release assets，并为同一image digest生成标准SBOM、provenance和GitHub attestation
- [ ] 7.5 匿名拉取versioned image，复验source/version label、module/registry digest、Release assets和attestation，证明Actions artifact不是长期消费权威
- [ ] 7.6 在新命名prerelease完整可读后精确退役`mtmpg-postgres`旧阶段性package versions与孤立attestation referrer，保留Git、Actions run和Issue历史
- [ ] 7.7 更新README、release/compatibility与维护文档，只描述PR/main CI、SemVer tag release、`ghcr.io/codeh007/mtmpg`和标准供应链材料，并完成聚焦检查与严格OpenSpec校验
- [ ] 7.8 完成7.7后在mtmpg #1与gomtmui #116/#117回填阶段性结果，记录prerelease、source、OCI digest、Actions run、Release/attestation和未完成stable事项

## 8. 独立Stable发布与收尾

- [ ] 8.1 将Cargo package version提升为首个stable SemVer并创建对应不可变Git tag，由同一release流程重新解析、完整测试、构建和发布独立stable制品
- [ ] 8.2 发布`ghcr.io/codeh007/mtmpg:<stable>`、stable GitHub Release和标准供应链材料，只在全部成功后把`latest`更新为该stable digest
- [ ] 8.3 复验stable version、source、manifest、Cargo.lock、resolved inputs、module/image digest、Release assets、SBOM、provenance、attestation与`latest`一致且没有覆盖prerelease
- [ ] 8.4 同步确认gomtmui companion OpenSpec只按SemVer消费新image、没有专用native consumer harness或mtmpg release前置的跨仓E2E
- [ ] 8.5 运行全部远端领域、ABI、真实PG18、final-image、release与严格OpenSpec门禁，回填mtmpg #1和gomtmui #116/#117最终结果与已知限制
