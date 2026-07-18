# pggomtm 仓库级 Agent 规则

本文件适用于整个仓库。所有分析、计划、文档和交付回复使用中文；代码、命令、标识符、上游专有名称及完整 MIT 法律文本保留原文。

## 权威与边界

- `src/` 是唯一 production 实现；消费仓库不得保留源码副本或第二条构建链。
- `Cargo.toml` 声明兼容依赖范围，`rust-toolchain.toml` 使用 stable；release用 `Cargo.lock` 只由远端CI解析并保存为发布证据。
- `tests/` 承载Rust领域测试、官方C layout probe、真实PG18和final-image行为测试及最小fixture。
- `Dockerfile` 只构建production module并组装PG18 runtime image。
- `.github/workflows/` 是依赖解析、重计算、candidate和promotion入口。
- `openspec/` 是需求、设计和task状态权威；Git、Actions、Release和attestation保存历史证据。

不要引入Cargo workspace、嵌套crate、生产HTTP服务、第二Dockerfile或本地image fallback。

## 修改前

1. 使用`gh`读取关联Issue，并读取active OpenSpec proposal、design、spec和tasks。
2. 阅读`Cargo.toml`、`rust-toolchain.toml`、`Dockerfile`及相关源码和测试。
3. 修改OAuth边界时追踪`_PG_oauth_validator_module_init`、startup、validate和shutdown调用链。
4. 区分当前行为与计划目标，只有实现和远端验证完成后才能勾选task。

## PostgreSQL与认证

- OAuth ABI权威是本次目标`pg_config --includedir-server/libpq/oauth.h`；bindings必须由allowlist生成并由官方C compiler验证layout，不得手写第二套声明。
- `pgrx`负责module magic、guard、PostgreSQL error和allocator语义，不得用自制兼容层绕过。
- Module由`oauth_validator_libraries`加载，不得增加control、versioned SQL、`CREATE EXTENSION`或`cargo pgrx install/package`交付路径。
- 认证必须fail closed。不得增加备用issuer、旧verifier、network fetch、SQL/SPI、宽松claims或其他fallback。
- Runtime只读取固定只读config/public JWKS；不得读取private key、API key、连接串或生产数据。

## Latest-compatible输入

- Rust跟随stable；PostgreSQL跟随PG18 major内最新稳定minor；Cargo使用兼容版本范围；Actions使用官方稳定major tag。
- 源码不得固定上游patch、base digest、Cargo精确`=`版本、Action commit SHA或手工下载archive hash。
- CI每次只解析一次Cargo.lock和上游image digest，全部测试、build和publish必须复用该结果。
- PostgreSQL major、Cargo不兼容major和产品SemVer升级仍需显式源码变更。

## 验证与本地限制

- 本地只允许源码/规划编辑、Git/OpenSpec操作、只读调查和精确清理已知对象。
- 本地不得运行Cargo、原生编译、Docker build/run、临时PostgreSQL或final-image检查。
- 实现提交到`main`后只使用精确SHA的GitHub Actions结果完成任务；失败历史保留并向前修复。
- 测试验证领域和真实系统行为，不测试Dockerfile/workflow字面量、精确版本/hash、layer/config相等或配置文件不存在。
- 不通过删除必要行为测试、弱化断言、降低lint或扩大权限获得通过。

## 数据与Git

源码、测试、日志、image和发布材料不得包含真实token、private key、JWKS、连接串、`.env`、session、PGDATA或未脱敏身份。测试只使用确定性合成fixture并限定在测试feature。

不得force push、覆盖他人改动、提交ignored产物或修改任务外文件。任务来自Issue时，在该Issue涉及的工作完全结束后回填结果；`extract-and-standardize-pggomtm`须在任务7.7后先回填阶段性结果。
