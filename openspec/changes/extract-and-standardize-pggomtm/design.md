## Context

mtmpg 是一个单一 Rust crate，生产目标是由 PostgreSQL `oauth_validator_libraries` 加载的 `pggomtm` module。OAuth ABI、离线 JWT/JWKS、closed profile-role、版本化 identity、失败脱敏和真实 PG18 OAuth 流程已经实现。

仓库当前只有约 1,281 行非测试 `src` 代码，却有 5,388 行 `tests`、2,359 行 `scripts`、2,986 行文档/规划、510 行 `build.rs` 和 1,635 行锁文件。大量代码用于批准精确 patch、digest、layer、config、脚本命令和历史 GitHub surface，而不是验证用户实际使用的 module 和 image。

完整源码已经非 force 进入 `main`，临时分支已删除，且所有重计算只在 GitHub Actions 执行。`native-ci.yml` 随后增长到 681 行，其中大部分用于每次 `main` 自动 candidate、定制 OCI evidence 和跨仓 promotion；run `29646533596` 证明该探索路径可以工作，也证明它与本仓库体量不相称。新的约束是在保留 Actions-only 和 main-first 边界的同时，把 CI、release 与 gomtmui 平台集成重新分责。

## Goals / Non-Goals

**Goals:**

- 删除不再提供运行或发布价值的文档、例子、脚本、自测和历史证据。
- 让 Rust、PG18 minor、Cargo 依赖、Actions 和标准工具跟随最新兼容稳定版本，而不是在源码中固定 patch 或 digest。
- 让 PR、`main` push 与 release tag 复用一份只读 CI 定义，不重复测试实现。
- 在单次 release run 中解析一次依赖和上游 image，把同一解析结果用于测试、构建、最终 image 验证和发布。
- 以领域规则、C/Rust ABI、真实 PostgreSQL OAuth、identity、module 加载和官方 entrypoint 启动作为主要门禁。
- 以显式 SemVer Git tag 表达发布意图，以 `ghcr.io/codeh007/mtmpg:<semver>`、GitHub Release、运行时解析的 OCI digest、lockfile、SBOM 和 attestation 表达发布结果。
- 删除 gomtmui 专用 consumer workflow 与重复 native harness，把平台 TLS、依赖服务、ACL/RLS 和授权集成留给 gomtmui 自身领域 change。

**Non-Goals:**

- 不自动跨越 PostgreSQL major；`latest` 在本 change 中表示 PG18 major 内的最新稳定 minor。
- 不删除生产 `runtime_config`、弱化 JWT/role/identity/fail-closed 契约，或把真实集成测试替换为 mock-only 测试。
- 不恢复本地 Docker build/run、原生编译、临时 PostgreSQL 或 image 检查。
- 不保证同一 source commit 在不同日期解析到相同上游 bytes；不可变性从每个已发布 mtmpg release 自身开始。
- 不自动合并任意外部 PR，也不以自定义高权限 workflow 代替 GitHub 仓库规则和原生 auto-merge。
- 不让 mtmpg release 等待 gomtmui 跨仓 E2E，也不删除 gomtmui 在真正启用数据库能力时应承担的平台验证。
- 不修改生产数据库、生产配置或生产流量。

## Decisions

### 1. 仓库只保留当前产品契约

目标结构保持紧凑：

```text
.github/workflows/   可复用只读 CI 与 SemVer tag release
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

版本、header 和 image digest 是观测值，不是预批准常量。测试只验证它们属于声明的兼容边界、彼此一致且被 release manifest 与标准供应链材料完整记录。

### 3. CI只验证，SemVer tag才发布

工作流分为两个稳定入口：

```text
pull_request --------------------------+
                                       |
main push -----------------------------+--> reusable read-only CI
                                       |      domain / ABI / real PG18 / image
SemVer tag --> release workflow -------+
                    |
                    v
             verified OCI archive
                    |
                    v
       ghcr.io/codeh007/mtmpg:<semver>
       GitHub Release + manifest + lock
       standard SBOM/provenance/attestation
```

`ci.yml` 同时支持 `pull_request`、`main` push 与 `workflow_call`。它只读 checkout 精确 source，解析一次 latest-compatible inputs，运行领域、ABI、真实 PG18 和最终 image 门禁，并在 release 调用时输出已验证 OCI archive。PR 与 main 不取得 package、Release、attestation 或 tag 写权限。

`release.yml` 只由符合 `vMAJOR.MINOR.PATCH` 或合法 prerelease 的 Git tag 触发。Tag version必须与 `Cargo.toml` 一致；release job复用 `ci.yml` 的同一测试定义，下载该 run 内已验证 archive后推送，不运行第二次 Cargo 或 Docker build。Workflow artifact只负责同一 run 内的短期传递，不是长期发布权威。

发布权威由不可覆盖的 Git tag、版本化 GHCR image、GitHub Release、精简 release manifest、该次 Cargo.lock 和标准 OCI/GitHub SBOM、provenance、attestation共同组成。删除 ORAS client解析、`<version>.evidence` tag、手工 evidence bundle和跨仓 consumer evidence。Dockerfile仍可为 builder/runtime `ARG`提供浮动稳定默认值；Actions传入本次resolve得到的临时完整digest，避免同一run内漂移。

PR auto-merge是仓库治理而不是构建步骤：仓库启用 GitHub 原生 auto-merge，并以required CI和actor范围约束 owner、Agent或批准的Dependabot更新；不得用`pull_request_target`高权限脚本自动合并任意外部代码。维护者与Agent仍可直接非force推送`main`，失败main由后续commit前进修复。

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

每个风险矩阵只能有一个主要权威：完整JWT/profile/role/identity矩阵位于Rust领域测试，真实backend矩阵位于单一PG18 harness，最终image只保留证明打包和启动边界所需的一组最小allow/deny smoke。共享fixture、client和staging入口应复用，不得在另一个仓库复制整套矩阵。

删除以下测试模式：

- 检查 Dockerfile、workflow 或脚本必须包含/不包含某个字符串。
- 检查 exact patch、archive hash、base digest、layer数量或完整 Docker `.Config` 相等。
- 为脚本本身伪造 Docker、Cargo、GitHub CLI 和 scanner 的大规模入口自测。
- 重复覆盖同一 fail-closed 分支而没有新增领域风险的组合矩阵。
- 检查配置文件或源码文件不存在。

最终 image 仍需确认不携带私钥、JWT fixture、测试 feature、源码或编译器，但应使用小型内容/运行检查或标准 SBOM policy，不维护完整 base filesystem 镜像和自定义黑名单框架。

### 6. 每个SemVer tag都是独立release

Prerelease使用普通SemVer，例如`0.1.0-rc.1`；stable使用`0.1.0`。二者都从对应不可变Git tag运行同一release流程，而不是把每个main run ID伪装成产品版本。Prerelease只写自身version tag并创建GitHub prerelease；stable写自身version tag、更新`latest`并创建stable GitHub Release。

Stable不是对某个prerelease digest的跨仓promotion。若stable tag与prerelease tag指向不同source或在不同日期解析到不同兼容依赖，它们就是两个分别经过完整门禁的release，这是标准SemVer语义。任何失败tag不得移动或重写；修复后提升prerelease或patch版本并创建新tag。

用户以mtmpg SemVer交流和升级；OCI digest、source、module digest和attestation用于机器验证该版本本身。Gomtmui Compose引用`ghcr.io/codeh007/mtmpg:<semver>`，平台在实际pull、启动或备份时可以记录resolved digest，但不得为此复制一套release测试或要求mtmpg等待跨仓验收。

### 7. Main、PR与release状态分离

`main`仍允许维护者和Agent直接非force推进，也允许暂时CI失败。PR通过只读CI后可以使用受限原生auto-merge；没有SemVer tag的main commit永远不发布image。Tag release的resolve、测试、image、manifest或attestation任一失败都不得留下可消费Release或移动既有tag。

```text
PR失败      -> 不合并，修复同一PR
main失败    -> 保留commit，后续commit向前修复
tag失败     -> 不重写tag，修复后发布新的SemVer
release成功 -> versioned image + GitHub Release + standard evidence
```

## Risks / Trade-offs

- **相同source在不同日期可能解析到不同依赖** -> 每个 run只解析一次，保存 lockfile和实际digest，并只发布该 run 已验证的 OCI archive。
- **最新上游版本引入不兼容** -> `main` CI fail closed，上一 release 保持可用；通过后续源码修复适配，不回退到永久 pin。
- **自动跨 PostgreSQL major 会破坏 ABI** -> 浮动范围限定 PG18；PG19 需要显式 feature、路径、ABI 和真实运行变更。
- **删减测试可能遗漏真实回归** -> 以领域风险和真实系统边界决定保留项，并在删除前把每个测试映射到仍存在的 requirement；不以行数作为唯一标准。
- **Tag workflow在push image后失败** -> 发布步骤先完成全部只读门禁并检查目标version不存在；失败tag不得复用，修复后发布新SemVer，并精确清理没有Release的孤立目标version。
- **自动合并不受约束会引入外部代码** -> 只使用GitHub原生auto-merge、required CI和明确actor范围；外部PR保持人工批准，直接main权限仅属于现有维护者/Agent身份。
- **删除跨仓consumer workflow降低集成覆盖** -> mtmpg保留自身完整module/image门禁；gomtmui在真正启用TLS、profile role、ACL/RLS和SQL executor时由对应领域change验证，不在抽取change中提前复制。
- **删除自定义扫描器降低历史覆盖** -> 标准 secret scanning只保护当前source和提交历史；历史调查按需使用外部工具，不继续维护在产品仓库中。

## Migration Plan

1. 更新并验证本 change 的 proposal、design、delta specs 和 tasks；旧`mtmpg-postgres` run-ID candidate与OCI evidence只作为历史探索结果。
2. 删除明确废弃的文档、历史证据和失效引用，迁移最小 test fixture，压缩 runtime config和领域测试。
3. 合并重复测试矩阵，保留单一Rust领域权威、单一真实PG18 harness和最小final-image smoke。
4. 用可复用`ci.yml`替代复杂`native-ci.yml`，让PR与main只读运行；配置受限GitHub原生auto-merge而不增加高权限合并脚本。
5. 建立SemVer tag驱动的`release.yml`，发布`ghcr.io/codeh007/mtmpg`、GitHub Release、manifest、Cargo.lock和标准供应链证明。
6. 发布首个新命名prerelease并匿名验证后，精确退役旧`mtmpg-postgres` package中的阶段性version，不删除Git/Actions历史。
7. Gomtmui删除专用consumer workflow与测试目录，重构内测Compose和platform image常量为新SemVer引用；完整授权集成继续由其hard-cut change负责。
8. 运行两仓聚焦检查与严格OpenSpec校验，完成约定阶段后回填mtmpg #1及gomtmui #116/#117。

迁移中的任一 `main` 失败保留在历史中并由后续commit修复。删除文件可从Git历史恢复，但只有符合新规格的前进修复可以发布新版本。

## Open Questions

无。用户已经确认“PG18 major内最新稳定minor、SemVer tag才发布、image名称为mtmpg、digest仅作release机器证据、gomtmui不维护专用consumer harness”的口径。
