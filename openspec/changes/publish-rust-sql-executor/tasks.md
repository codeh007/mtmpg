## 1. 固化双产品仓库边界

- [x] 1.1 更新`AGENTS.md`、README与维护文档，明确一个workspace、根validator package、唯一`executor/` package、独立image/tag/version及消费者不构建边界
- [x] 1.2 将根Cargo package改为只含`executor/` member的workspace，建立executor `0.1.0` manifest与只读目录骨架，确保HTTP/libpq依赖不进入validator feature
- [x] 1.3 从validator抽取pgrx无关的database-token profile/role/claims contract并由两个package复用，使用既有validator测试证明contract v2行为不变且不复制mapping/decoder

## 2. 建立有效RED证据

- [x] 2.1 先写Rust HMAC envelope、strict request/response、replay、body/bind边界和30秒ES256 issuer测试，覆盖未知字段、外部credential、role/claims覆盖及credential临期拒绝
- [x] 2.2 先写当前PG18 libpq binding/layout与`PGconn*` token registry测试，覆盖一次取用、未知/重复/失败清理和不同principal并发隔离
- [x] 2.3 先写真实PG18 executor harness，覆盖TLS/OAuth、空bind/参数bind、multi-statement拒绝、CTE/`CALL`/`DO`、read-only/change、rollback、budget与cancel
- [x] 2.4 将只含RED测试/fixture/必要scaffold的精确SHA非force推送`main`，取得因缺少executor生产实现而失败、但既有validator门禁不回归的GitHub Actions run并记录脱敏证据

## 3. 实现HMAC、协议与issuer

- [x] 3.1 实现固定HTTPS path的body上限、HMAC canonical input、constant-time比较、30秒时窗和有界单实例nonce replay store，使对应Rust测试GREEN
- [x] 3.2 实现strict `DelegatedPrincipal`与单statement request/response schema、结构化bind和稳定错误类别，拒绝`statements[]`、未知字段与所有credential/claims覆盖
- [x] 3.3 实现executor-only ES256 key loader/zeroizing issuer、active `kid`、精确30秒contract v2 claims和credential剩余有效期门禁，只复用共享profile-role contract

## 4. 实现libpq OAuth与SQL执行

- [x] 4.1 从本次PG18 `libpq-fe.h`生成最小allowlist bindings并用官方C compiler验证auth hook、request、result和cancel layout，不提交手写或固定minor bindings
- [x] 4.2 实现启动时唯一auth-data hook、`PQconnectStartParams`后`PGconn* -> token`原子registry、cleanup callback与全部失败路径zeroize，取得单元和并发测试GREEN
- [ ] 4.3 实现`postgres`/`gomtm`/同名profile/`require_auth=oauth`/`verify-full`逐请求连接、无password/SCRAM/pool/`SET ROLE` fallback，并验证不同principal并发真实`system_user`
- [ ] 4.4 实现`PQsendQueryParams` extended protocol、结构化参数与一个顶层statement，覆盖空bind、多命令拒绝和合法CTE/`CALL`/`DO`
- [ ] 4.5 实现service-owned read-only/confirmed-change事务、commit前缓冲、columns/rows/command tag/affected rows和SQLSTATE脱敏分类，所有失败rollback或关闭
- [ ] 4.6 固化request/statement/bind/connection/lock/transaction/1000-row/1-MiB/256-KiB/deadline预算，并由connection owner传播cancel且不遗留后台task

## 5. 完成HTTPS服务与远端GREEN

- [ ] 5.1 使用成熟Rust HTTP/TLS runtime组装唯一executor入口、并发semaphore、readiness、correlation审计和统一错误响应，不记录secret、token、SQL、bind或结果
- [ ] 5.2 更新标准CI，使PR/main一次解析共享Cargo.lock与PG18/libpq输入并运行validator、executor Rust、C layout、真实PG18、并发、TLS和cancel门禁
- [ ] 5.3 非force推送最小生产实现到`main`，取得精确SHA的Rustfmt、Clippy、validator、executor与真实PG18完整GREEN run；失败只向前修复且保留历史

## 6. 构建并发布executor image

- [ ] 6.1 增加executor独立image定义，只包含release binary、匹配libpq runtime、CA与MIT license，以固定非root身份运行且不含toolchain/source/test/secret/PostgreSQL server
- [ ] 6.2 扩展标准CI product选择，构建一次executor OCI archive并验证非root HTTPS readiness、动态依赖、最小HMAC/OAuth allow-deny和image secret边界
- [ ] 6.3 增加`executor-v*`独立release workflow，校验annotated tag、executor Cargo version、精确SHA/main ancestry，并只推送同一CI已验证archive及manifest/checksums/SBOM/provenance/attestation
- [ ] 6.4 在精确main GREEN SHA创建annotated `executor-v0.1.0`，等待GitHub Actions成功发布`ghcr.io/codeh007/mtmpg-executor:0.1.0`与独立GitHub Release
- [ ] 6.5 匿名核对versioned image、source、Cargo.lock、resolved PG18/libpq、OCI digest、manifest、checksums、SBOM、provenance和attestation，并证明`mtmpg:v0.2.0` tag/image/Release未变化

## 7. 完成交付证据

- [ ] 7.1 更新README与executor运行/失败/兼容文档，记录固定wire、mount、TLS、单实例nonce与无fallback边界，不包含真实credential或连接串
- [ ] 7.2 运行`openspec validate publish-rust-sql-executor --strict`、secret扫描和Git diff检查，回填精确RED/GREEN/release run、source、image digest与脱敏真实PG18证据
- [ ] 7.3 将已发布executor identity交给gomtmui consumer change；只有gomtmui完整activation与OAuth/API-key E2E完成后，才在Issue #126宣告原始任务完成
