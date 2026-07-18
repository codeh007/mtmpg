## Context

mtmpg 是一个单一 Rust crate，生产目标是由 PostgreSQL `oauth_validator_libraries` 加载的 `pggomtm` module。OAuth ABI、离线 JWT/JWKS、closed profile-role、版本化 identity、失败脱敏和真实 PG18 OAuth 流程已经实现。

仓库当前只有约 1,281 行非测试 `src` 代码，却有 5,388 行 `tests`、2,359 行 `scripts`、2,986 行文档/规划、510 行 `build.rs` 和 1,635 行锁文件。大量代码用于批准精确 patch、digest、layer、config、脚本命令和历史 GitHub surface，而不是验证用户实际使用的 module 和 image。

完整源码已经非 force 进入 `main`，临时分支已删除，且所有重计算只在 GitHub Actions 执行。新的约束是在保留这两个边界的同时，删除仓库内的历史负担，让每次远端构建主动跟随最新兼容稳定技术栈，并以实际 candidate 行为决定能否发布。

## Goals / Non-Goals

**Goals:**

- 删除不再提供运行或发布价值的文档、例子、脚本、自测和历史证据。
- 让 Rust、PG18 minor、Cargo 依赖、Actions 和标准工具跟随最新兼容稳定版本，而不是在源码中固定 patch 或 digest。
- 在单次 Actions run 中解析一次依赖和上游 image，把同一解析结果用于测试、构建、最终 image 验证和 candidate 发布。
- 以领域规则、C/Rust ABI、真实 PostgreSQL OAuth、identity、module 加载和官方 entrypoint 启动作为主要门禁。
- 以 mtmpg SemVer 表达用户契约，以运行时解析的 OCI digest、lockfile、SBOM 和 attestation 证明发布证据的一致性。

**Non-Goals:**

- 不自动跨越 PostgreSQL major；`latest` 在本 change 中表示 PG18 major 内的最新稳定 minor。
- 不删除生产 `runtime_config`、弱化 JWT/role/identity/fail-closed 契约，或把真实集成测试替换为 mock-only 测试。
- 不恢复本地 Docker build/run、原生编译、临时 PostgreSQL 或 image 检查。
- 不保证同一 source commit 在不同日期解析到相同上游 bytes；不可变性从每个已发布 mtmpg release 自身开始。
- 不修改生产数据库、生产配置或生产流量。

## Decisions

### 1. 仓库只保留当前产品契约

目标结构保持紧凑：

```text
.github/workflows/   远端 CI、candidate 和 promotion
src/                 production module
tests/               领域、ABI、PG18 和 image 行为测试及最小 support
docs/                仅保留运行配置和发布/兼容契约
Cargo.toml
Dockerfile
README.md
LICENSE
```

具体处理如下：

- 无条件删除 `SECURITY.md`、`CONTRIBUTING.md` 及其所有引用。
- 删除 `docs/evidence/issue-116/`、治理说明和依赖审计快照；Git 历史、Actions run、Release asset 和 attestation 是历史权威。
- 删除 `examples/`。现有 OAuth smoke fixture 是测试支持程序，不是用户示例；裁剪后移动到 `tests/support/` 并声明为 test-only target。
- 保留 `src/runtime_config.rs` 的生产加载逻辑。把 `src/runtime_config/tests.rs` 中仍覆盖真实风险的少量测试合并回主文件的 `cfg(test)` module，删除误导性的子目录。
- 默认删除现有 `scripts/`。Cargo 命令由 workflow 直接编排，secret scanning 使用标准 Action，镜像和 PostgreSQL harness 放在 `tests/`；只有被多个 workflow step 真实复用且不能由标准工具表达的小入口才可保留。
- README 只解释产品、当前支持的 PG major、最短使用方式和 release 入口；不复制 Git 历史、CI 实现或阶段性证据。

### 2. 源码声明 latest-compatible，CI 记录实际解析结果

源码只保留人类可维护的兼容边界：

- `rust-toolchain.toml` 使用 `stable`，builder 使用官方稳定 Rust 的 Debian tag，不固定 patch 或 digest。
- PostgreSQL builder development package和 runtime 使用 PG18 稳定通道，例如 `postgresql-server-dev-18` 与 `postgres:18-bookworm`，不固定 minor 或 digest。
- `Cargo.toml` 删除精确 `=`，使用上游兼容语义允许的版本范围；不提交用于 release 解析的 `Cargo.lock`。
- GitHub Actions 使用官方稳定 major tag，例如 `actions/checkout@v7`，不在 workflow 中固定 action commit SHA。
- 标准 scanner/audit 工具使用其稳定 Action、包管理器或 Cargo 安装通道，不维护 archive URL、预批准 checksum 和对应自测。

CI 的 resolve step 负责：

1. 生成一次 Cargo lockfile，并在后续 Cargo 命令中以 `--locked` 复用。
2. 解析 Rust、Cargo、PostgreSQL development/runtime、builder image 和标准工具的实际版本。
3. 把浮动 Docker tag 解析为本次 run 的完整 digest，并将该临时 digest 传给 build，不写回源码。
4. 生成 `resolved-inputs.json`，记录实际版本、Cargo.lock digest、base digest、source SHA 和 workflow run。

版本、header 和 image digest 是观测值，不是预批准常量。测试只验证它们属于声明的兼容边界、彼此一致且被 release evidence 完整记录。

### 3. 单次解析、单次 production image build、同一 bytes 发布

CI 数据流为：

```text
main source
    |
    v
resolve latest-compatible inputs
    |-- Cargo.lock
    |-- resolved-inputs.json
    `-- ephemeral builder/runtime digests
    |
    v
domain + ABI + real PG18 tests
    |
    v
build production image once -> verify that image -> OCI archive
                                             |
                          +------------------+------------------+
                          |                                     |
                          v                                     v
                  publish candidate                    release evidence
                  without rebuild                     lock/SBOM/provenance
                          |                                     |
                          +------------------+------------------+
                                             |
                                             v
                              immutable public OCI evidence
```

只读验证 job 生成并验证 production OCI archive，再把 archive 和 evidence 作为 workflow artifact 交给最小写权限 publish job。Publish job只推送已验证 archive，不运行 Cargo 或 Docker build。这样即使源码允许浮动依赖，测试对象和发布对象仍是同一 bytes。

Workflow artifact只用于本次run内的传递和短期诊断，不是发布权威。GitHub重跑同一run会删除前一attempt的artifact，因此publish job必须把Cargo.lock、resolved inputs、release manifest、SBOM、provenance、attestation和checksums作为同仓库、按candidate SemVer命名且不可覆盖的公开OCI evidence artifact发布。Consumer与stable promotion从该OCI引用取证；重复run在任何写入前同时拒绝既有image tag和evidence tag。

Dockerfile 可以为 builder/runtime `ARG` 提供浮动稳定 tag 默认值；Actions 必须传入 resolve step 得到的临时完整 digest。源码因此不锁 digest，而单次 run 仍避免 build/test 间的上游漂移。

### 4. Build script只生成当前目标 ABI

`build.rs` 继续从本次目标 `pg_config --includedir-server` 的官方 `libpq/oauth.h` 生成最小 allowlist bindings，因为手写 PostgreSQL ABI 会形成第二权威。

删除以下职责：

- 预批准 PostgreSQL source/header/bindings/runtime digest。
- 固定 Rust、pgrx、JOSE patch 和 target 字面值。
- 对 bindgen 输出做与某个 patch 精确相等的文本/hash判断。
- 生成包含预批准技术栈常量的复杂内部 manifest。

保留生成 symbol allowlist、必要 callback ABI override，以及由官方 C compiler执行的 size/offset/layout 测试。实际 header、bindings 和 module digest由 CI evidence 在生成后记录。

### 5. 测试契约而不是实现形状

保留四类门禁：

1. Rust 领域测试：JWT schema/signature/time、closed role、identity codec、runtime config 和失败原因。
2. ABI 测试：官方当前 PG18 header可以生成 bindings，C/Rust size、offset、callback和导出 symbol一致。
3. 真实 PG18 测试：临时 cluster加载 production/test module，覆盖 OAuth allow/deny、requested role、`system_user` 和错误脱敏。
4. 最终 image 测试：官方 entrypoint能够初始化并启动，实际 PostgreSQL 属于 PG18，module位于真实 `pkglibdir` 且能加载，最终 OAuth smoke通过。

删除以下测试模式：

- 检查 Dockerfile、workflow 或脚本必须包含/不包含某个字符串。
- 检查 exact patch、archive hash、base digest、layer数量或完整 Docker `.Config` 相等。
- 为脚本本身伪造 Docker、Cargo、GitHub CLI 和 scanner 的大规模入口自测。
- 重复覆盖同一 fail-closed 分支而没有新增领域风险的组合矩阵。
- 检查配置文件或源码文件不存在。

最终 image 仍需确认不携带私钥、JWT fixture、测试 feature、源码或编译器，但应使用小型内容/运行检查或标准 SBOM policy，不维护完整 base filesystem 镜像和自定义黑名单框架。

### 6. SemVer是产品身份，digest是证据身份

Candidate 使用 mtmpg SemVer prerelease，例如 `0.1.0-rc.<run>`；gomtmui Compose 和用户文档引用该版本化 tag。对应供应链材料使用同版本派生的不可覆盖OCI evidence引用。Consumer workflow在拉取时解析image与evidence完整 OCI digest，并把 `mtmpg version -> source -> image digest -> evidence digest -> module digest -> gomtmui source` 写入验收证据。

Stable promotion只为同一已验收 digest增加稳定 SemVer和`latest`tag并创建 immutable GitHub Release，不重建 image。Workflow必须拒绝覆盖既有 SemVer tag或Release。用户以 mtmpg version交流和升级；digest用于机器校验、attestation和证明 promotion 没有换包。

### 7. Main与发布状态继续分离

`main` 仍允许维护者和 Agent 直接非 force 推进，也允许暂时 CI 失败。任何 resolve、测试、image或evidence失败都只阻止新 candidate/stable，不回退源码，也不改变上一已发布版本。

```text
main失败 -> 保留commit -> 后续commit向前修复
candidate失败 -> 不发布版本 -> 上一版本不变
consumer失败 -> 不promotion -> 发布新candidate重新验收
```

## Risks / Trade-offs

- **相同source在不同日期可能解析到不同依赖** -> 每个 run只解析一次，保存 lockfile和实际digest，并只发布该 run 已验证的 OCI archive。
- **最新上游版本引入不兼容** -> `main` CI fail closed，上一 release 保持可用；通过后续源码修复适配，不回退到永久 pin。
- **自动跨 PostgreSQL major 会破坏 ABI** -> 浮动范围限定 PG18；PG19 需要显式 feature、路径、ABI 和真实运行变更。
- **删减测试可能遗漏真实回归** -> 以领域风险和真实系统边界决定保留项，并在删除前把每个测试映射到仍存在的 requirement；不以行数作为唯一标准。
- **SemVer tag或evidence可能被覆盖** -> publish/promotion workflow在写入前验证image tag、evidence tag、package和Release不存在，并用记录的 OCI digest审计不可变性；不依赖rerun会删除的Actions artifact。
- **删除自定义扫描器降低历史覆盖** -> 标准 secret scanning只保护当前source和提交历史；历史调查按需使用外部工具，不继续维护在产品仓库中。

## Migration Plan

1. 更新并验证本 change 的 proposal、design、delta specs 和 tasks；旧固定版本/image readiness结果不再完成新任务。
2. 删除明确废弃的文档、历史证据和失效引用，迁移最小 test fixture，压缩 runtime config和领域测试。
3. 删除精确版本/hash断言和自定义策略脚本，简化 `build.rs`、Cargo/toolchain、Dockerfile 与 Native CI resolve/test/build流程。
4. 在 `main` 通过远端 Actions取得 latest-compatible domain、ABI、真实 PG18 和最终 image成功结果；不使用本地重计算替代。
5. 从已验证 OCI archive发布首个 SemVer candidate，并把lockfile、resolved inputs、SBOM、provenance和attestation发布为不可覆盖OCI evidence。
6. Gomtmui按 candidate version远端验收并记录实际 digest，完成真实 OAuth、ACL/RLS和rollback。
7. 将同一 digest晋级稳定SemVer，创建immutable Release并回填mtmpg #1、gomtmui #116/#117。

迁移中的任一 `main` 失败保留在历史中并由后续commit修复。删除文件可从Git历史恢复，但只有符合新规格的前进修复可以发布新版本。

## Open Questions

无。用户已经确认“PG18 major内最新稳定minor、SemVer用户身份、digest仅作CI与发布证据”的口径。
