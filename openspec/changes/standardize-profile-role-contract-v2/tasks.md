## 1. 规划与基线

- [x] 1.1 严格校验本change的proposal、design、增量spec和tasks，并确认工作区只包含本change的规划改动
- [x] 1.2 提交并推送完整规划工件到`main`，记录精确source SHA

## 2. 测试先行RED

- [x] 2.1 更新Rust JWT、profile-role、identity与ABI领域测试及共享OAuth fixture，使其要求三个v2下划线名称和`pggomtm:v2`
- [x] 2.2 更新真实PG18与final-image最小行为测试，覆盖v2同名startup role并拒绝v1、项目前缀和阶段前缀名称
- [x] 2.3 对测试先行改动执行静态检查，提交并推送仅含测试与fixture的RED commit到`main`
- [x] 2.4 核验RED commit精确SHA的GitHub Actions因旧production契约产生预期失败，并记录run证据

## 3. Contract v2实现与GREEN

- [x] 3.1 将`DatabaseProfile`、规范字符串和closed PostgreSQL role统一为`ordinary`、`business_admin`和`database_developer`，删除旧名称映射
- [x] 3.2 将identity encoder/decoder升级为只产生并接受`pggomtm:v2`，保持其余claims、安全、ABI和runtime config契约不变
- [x] 3.3 将package version提升为`0.2.0`并更新README、runtime配置及release兼容文档，明确v0.1.x/v1与v0.2.x/v2不可混用
- [x] 3.4 对最小production与文档改动执行静态检查，提交并推送GREEN commit到`main`
- [x] 3.5 核验GREEN commit精确SHA的完整GitHub Actions全部通过，并记录run证据

## 4. V0.2.0发布与核验

- [x] 4.1 在已验证GREEN SHA创建并推送不可变`v0.2.0` tag
- [x] 4.2 核验tag触发的Release workflow完整成功且GitHub Release公开可用
- [x] 4.3 匿名核验`ghcr.io/codeh007/mtmpg:0.2.0`、`latest`、manifest、Cargo.lock、SBOM、provenance与attestation，并记录source、image和module digest
- [x] 4.4 回填发布证据和最终任务状态，严格校验change并推送完成状态到`main`

## 验证证据

- 规划基线：`b0c013e5a1500d9373434579fb704f861ffd6c3d`
- RED候选（rustfmt失败，不计有效RED）：`7cfc65299748af58fb46dd54afc2d5c767708969`，run `29714642271`
- 有效RED：`02ef0fb3c9accd1cbe97973d88f81d588ff48ec2`，run `29714762297`；4个JWT/identity断言因`pggomtm:v1`和旧prefixed role实现失败
- GREEN候选：`6c5a24a95bc43b4a4ec100f83a609c4af2b4699f`
- 有效GREEN：`6c5a24a95bc43b4a4ec100f83a609c4af2b4699f`，run `29715037620`；完整CI成功
- Release tag：`v0.2.0` -> `6c5a24a95bc43b4a4ec100f83a609c4af2b4699f`
- Release：`https://github.com/codeh007/mtmpg/releases/tag/v0.2.0`，run `29715410028`；全部job成功
- OCI：`ghcr.io/codeh007/mtmpg:0.2.0`与`latest` -> `sha256:dad813ee85886bf3fa5842ff56d1bf00a46fbc9cbc5d4665c3bc471c295ef16b`
- Module SHA-256：`746b215bc73a1431cd855e7b1b20393c5f92706c577f4f46fc1fac94842bc789`
- Attestation：provenance `36086737`，SBOM `36086740`；9个公开Release assets的checksums及跨文件身份校验通过
