## 当前基线

`pggomtm` 的OAuth validator主路径已经实现，完整源码已非force进入远端`main`，临时功能分支已删除。旧固定PG18.4/digest方案的Rust、ABI、Cargo、真实PostgreSQL integration和image build曾在远端通过，但final-image gate因比较Docker config失败；该结果不完成本次latest-compatible精简方案。

任何本地Docker build/run、原生编译、临时PostgreSQL或image检查结果都不能完成以下任务。`main`验证失败时保留源码并通过后续commit修复，上一candidate/stable保持不变。

## 1. Main与规划基线

- [x] 1.1 将`issue-116-extract-pggomtm`非force fast-forward到远端`main`，把本地工作线切换到同一main，删除workflow临时分支trigger及本地/远端临时分支
- [x] 1.2 确立main-first、Actions-only边界，精确清理旧本地诊断container/image且不执行宽泛prune
- [x] 1.3 按gomtmui #116评论5010761180重写proposal、design、两份delta spec和tasks，并通过严格OpenSpec校验

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

## 5. 单次解析与只读Native CI

- [x] 5.1 增加resolve step，生成一次临时Cargo.lock、解析builder/runtime完整digest并输出`resolved-inputs.json`
- [x] 5.2 让fmt、Clippy、Cargo tests、ABI、production module和PG18 integration复用同一lockfile、minor和解析结果，检测任何job漂移
- [x] 5.3 用标准source/secret和dependency/license检查替代自定义历史collector及其自测，保持PR/fork lane完全只读
- [x] 5.4 从同一resolved inputs只构建一次production image，完成final-image验证后物化可传递OCI archive及module identity
- [x] 5.5 上传已验证OCI archive、Cargo.lock、resolved inputs和只读测试结果供后续publish job使用，验证artifact与source绑定
- [x] 5.6 将精简实现直接提交到`main`，只依据精确SHA的远端Actions日志前进修复，取得resolve、领域、ABI、PG18和final-image完整成功结果

## 6. SemVer Candidate与供应链证据

- [ ] 6.1 从Cargo package/release输入生成不可覆盖的mtmpg SemVer prerelease candidate，并验证版本、source和workflow run唯一
- [ ] 6.2 增加仅对仓库自身成功`main` push生效的最小写权限publish job，下载并推送已验证OCI archive且不运行Cargo或Docker build
- [ ] 6.3 推送公开`ghcr.io/codeh007/mtmpg-postgres:<semver-prerelease>`，记录registry返回的完整OCI digest并拒绝覆盖既有version
- [ ] 6.4 生成release manifest，绑定mtmpg version、source、Cargo.lock、resolved inputs、PG18、module、OCI archive和registry digest
- [ ] 6.5 为同一candidate生成并验证SBOM、provenance、attestation和checksums，确认全部材料描述实际解析结果而非下一次构建的预批准输入
- [ ] 6.6 验证GHCR匿名公开读取，并证明失败main、PR、fork和重复version无法取得package、Release或attestation写入结果

## 7. Gomtmui远端消费验收

- [ ] 7.1 在gomtmui把`docker-compose.yml`的PostgreSQL image改为明确mtmpg SemVer prerelease tag，不改变既有volume、TLS、healthcheck、environment和command
- [ ] 7.2 在gomtmui Actions拉取candidate version，解析实际OCI digest并验证与mtmpg manifest、module和source一致
- [ ] 7.3 远端验证官方initdb、volume、healthcheck和PG18启动语义，确认最终image没有本地Rust build或fallback
- [ ] 7.4 远端验证现有TLS、sub2api与pgAdmin连接，以及production module从真实`pkglibdir`加载
- [ ] 7.5 远端完成真实database JWT OAuth allow/deny、三类profile、requested role和`system_user`identity矩阵
- [ ] 7.6 远端完成ordinary/business-admin/database-developer的ACL/RLS正负矩阵，确认认证成功不绕过数据库授权
- [ ] 7.7 切回上一已验证mtmpg version完成rollback，并产出绑定mtmpg version/source/manifest/module/OCI digest与gomtmui source的consumer evidence
- [ ] 7.8 完成7.7后在gomtmui #116/#117回填阶段性结果，记录candidate version、实际digest、Actions run、真实验收、rollback和未完成stable事项

## 8. Same-digest Stable发布

- [ ] 8.1 建立promotion workflow，只接受已通过mtmpg与gomtmui证据的candidate version/digest，且不得重新解析依赖或构建module/image
- [ ] 8.2 为同一digest增加稳定mtmpg SemVer与`latest`alias，创建精确source tag和immutable GitHub Release，并拒绝覆盖任何既有发布身份
- [ ] 8.3 验证stable tag、Release assets、Cargo.lock、resolved inputs、manifest、checksums、SBOM、attestation和consumer evidence全部指向同一OCI/module digest
- [ ] 8.4 在gomtmui把已验收prerelease更新为稳定mtmpg SemVer，确认解析digest不变并完成远端启动smoke
- [ ] 8.5 回填mtmpg #1与gomtmui #116/#117最终结果，记录稳定version、source、OCI digest、供应链材料、消费证据、rollback和已知限制
