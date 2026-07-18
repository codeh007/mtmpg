# 为 pggomtm 贡献变更

本页说明如何限定变更范围、直接推进开发`main`并通过GitHub Actions取得可审查证据。所有命令都从本仓库根目录执行，不依赖其他仓库的未跟踪文件。公开的`main`是开发主线和唯一CI/CD源码来源，不表示stable发布状态。

## 从 Issue 建立变更范围

每项工作应先关联一个 mtmpg GitHub Issue，并把目标、范围、非目标和验证证据写清楚。跨仓库 Issue 只作为产品总线，实际修改仓库必须有自己的跟踪 Issue：

```bash
gh issue list --repo codeh007/mtmpg --state open
gh issue view 1 --repo codeh007/mtmpg \
  --json number,title,body,url,comments
```

选择并读取目标 Issue 后，按以下顺序限定变更：

1. 阅读 Issue 正文与已有讨论。
2. 阅读相关 OpenSpec proposal、design、spec 和 tasks。
3. 从最新`main`开始编辑，并准备范围单一、可追踪的commit。
4. 只提交该 Issue 与当前 OpenSpec task 所需的文件。

不要把无关重构、依赖升级或发布规则混入同一变更。任务完成且全部仓库工作结束后，再在 Issue 中统一回填 commit、验证结果和已知限制。

## 可选 Pull Request

外部贡献者或需要网页讨论的协作者可以创建Pull Request（PR）。PR不是维护者或Agent更新`main`的前置条件，但仍应让审查者能够从clean checkout复现结论：

- 关联 Issue 与对应 OpenSpec change，并说明本次完成的 task 范围。
- 说明行为变化、原生应用程序二进制接口（ABI）影响、安全边界和回退方式。
- 列出执行过的命令、exit code、通过数量和未运行项目。
- 附上不含 secret 的证据路径，不粘贴环境变量或完整构建凭据。
- 如果使用人工智能（AI）辅助，说明使用范围与人工核对内容。

不要对共享分支执行`force push`。公开fork PR只能运行read-only、无secret的验证，不能取得package、Release或attestation写权限。

## 直接推进 main

维护者或Agent按以下顺序推进普通变更：

1. 保持Issue、OpenSpec task、diff和commit范围一致。
2. 把commit直接非force推送到`main`。
3. 等待该精确SHA的`Native CI`，读取失败日志并定位根因。
4. CI失败时保留commit并追加修复commit，不改写历史。
5. 只有全部门禁成功的SHA才能进入candidate与后续发布状态机。

仓库不要求required PR、branch protection、approving review、squash-only或auto-merge。该治理选择不取消技术审查：pgrx、JOSE、Rust toolchain、PostgreSQL minor、官方base/header、Actions source/pin、release workflow或写权限变化必须记录上游diff、风险与独立技术结论。

## 保持依赖锁定

`Cargo.toml` 与 `Cargo.lock` 共同定义当前依赖图。依赖变更必须保持可审查：

- 使用精确版本，并提交对应 lockfile diff。
- 一个 PR 只处理一组有明确理由的依赖变化。
- 核对上游变更、许可证、安全公告、feature 和原生 ABI 影响。
- pgrx、pgrx-pg-sys、JSON Object Signing and Encryption（JOSE）、Rust补丁版本或PostgreSQL minor变化必须显式审查后再推进。

pgrx、Rust toolchain 或 PostgreSQL minor 变化必须由维护者人工审查，并在精确目标 runtime 重新生成证据。未运行新 minor 的情况下，不得更新支持矩阵。

## 本地工作区边界

本地只执行源码与OpenSpec编辑、Git操作、只读调查、帮助或纯fixture policy命令。不得运行Docker build/run、Cargo或原生编译、临时PostgreSQL cluster和最终image检查；仓库重计算入口会在`GITHUB_ACTIONS!=true`时拒绝。不要通过手工设置该变量绕过边界，也不要用本地tag、终端日志或既有image完成task。

## 取得远端 Native CI 证据

GitHub Actions直接执行官方C header、Rust、真实PostgreSQL runtime、production artifact和最终image隔离门禁；根`Dockerfile`只构建production image。把单一范围commit非force推送到`main`后等待`Native CI`：

```bash
gh run list \
  --repo codeh007/mtmpg \
  --workflow native-ci.yml \
  --branch main \
  --limit 5
run_id=1234567890123
gh run watch "$run_id" --repo codeh007/mtmpg --exit-status
gh run view "$run_id" --repo codeh007/mtmpg --log-failed
```

测试jobs使用GitHub Actions cache且保持只读。仓库自身成功`main` push的candidate job只有在全部前置门禁通过后才取得最小发布权限；失败main、PR和fork不能写GHCR或attestation。

```bash
gh run rerun 1234567890123 --repo codeh007/mtmpg
```

## 审查原生 ABI 与 PostgreSQL minor

原生边界变更需要独立人工审查。以下变化不能只依赖 Rust 单元测试：

- PostgreSQL OAuth header、struct layout、magic、callback signature 或 loader 行为变化
- pgrx guard、PostgreSQL allocator、panic 或 error 边界变化
- PostgreSQL minor、Debian runtime、target、架构或 libc 变化

对应`main` SHA必须运行官方C layout probe、真实runtime probe、导出symbol检查和最终镜像隔离扫描。证据只能说明实际运行过的精确组合。

## 保护 secret 与证据

仓库、Git history、日志、镜像层和证据不得包含真实凭据或运行数据。提交前检查以下内容：

- 不提交私钥、API key、token、连接串、`.env`、session 或 PostgreSQL data。
- 测试只使用确定性合成 fixture，并把 fixture 限定在测试 gate。
- 扫描结果只记录命中文件或脱敏类别，不输出敏感值。
- 不读取或修改生产配置，除非任务明确授权且操作范围经过审查。

发现意外泄露时停止提交，先撤销或轮换凭据，再按[安全政策](SECURITY.md)私密报告。

公开读取的`mtmpg-postgres` package不需要部署credential。Package、Release、tag与attestation写权限只能由仓库自身成功`main` push或受控promotion job申请；普通PR、fork、Compose、manifest和文档示例不得包含这些权限或credential。

## 更新 OpenSpec 与提交

OpenSpec 是行为和交付范围的权威记录。修改前后按以下顺序核对：

1. 先更新或确认 proposal、design 与 delta spec，再实现行为。
2. 只在实现和验证都完成后更新对应 task checkbox。
3. 通过项目生成命令更新生成文件，不要手写生成结果。
4. 运行`git diff --check`等非重计算检查，推送后取得精确commit的远端`Native CI`成功证据。
5. 提交单一范围的源码、测试、文档与证据。

不要增加第二套源码、兼容 fallback 或包装实现来掩盖根因。
