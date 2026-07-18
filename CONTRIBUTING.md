# 为 pggomtm 贡献变更

本页说明如何限定变更范围、运行 locked Rust 与 Docker 门禁，并让 Agent 管理可审查的 Pull Request（PR）生命周期。所有命令都从本仓库根目录执行，不依赖其他仓库的未跟踪文件。公开的 `main` 是开发主线，不表示 stable 发布状态。

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
3. 稳态开发从最新受保护 `main` 创建名称包含 Issue 编号的短期分支。
4. 只提交该 Issue 与当前 OpenSpec task 所需的文件。

不要把无关重构、依赖升级或发布规则混入同一变更。任务完成且全部仓库工作结束后，再在 Issue 中统一回填 commit、验证结果和已知限制。

当前 `issue-116-extract-pggomtm` 是默认分支尚无完整基线时的一次性 bootstrap ref，不是长期开发分支。它通过追溯审计、cold build 与 whole-branch review 后将非 force fast-forward 到 `main`，且不会创建 tag、Release 或 `latest`。

## 准备 Pull Request

PR 应让 Agent、维护者和外部贡献者能够从 clean checkout 复现结论：

- 关联 Issue 与对应 OpenSpec change，并说明本次完成的 task 范围。
- 说明行为变化、原生应用程序二进制接口（ABI）影响、安全边界和回退方式。
- 列出执行过的命令、exit code、通过数量和未运行项目。
- 附上不含 secret 的证据路径，不粘贴环境变量或完整构建凭据。
- 如果使用人工智能（AI）辅助，说明使用范围与人工核对内容。

不要对维护者正在审查的共享分支执行 `force push`。需要修正时追加清晰 commit，或先与维护者确认历史整理方式。

## 由 Agent 管理 PR 生命周期

公开开发基线进入 `main` 并完成 OpenSpec 任务 7.10 后，Agent 负责普通 PR 的完整生命周期：

1. 从已限定的 mtmpg Issue 创建短期分支和 PR。
2. 保持 PR 范围与 Issue、OpenSpec task 和验证证据一致。
3. 等待 required `Native CI`，读取失败日志并修复根因。
4. 确认讨论已经解决，且没有未审查的高风险变化。
5. 使用 squash auto-merge 合并，并让 GitHub 自动删除源分支。

Ruleset 的 required approving review 数为 `0`，避免单贡献者仓库依赖不存在的第二账号。该设置不取消技术审查：pgrx、JOSE、Rust toolchain、PostgreSQL minor、官方 base/header、Actions source/pin、release workflow 或写权限变化必须在 Issue/PR 中记录上游 diff、风险与独立技术结论，缺少证据时 Agent 不得启用 auto-merge。

Auto-merge 与 ruleset 当前尚未启用；任务 7.10 完成前不得用直接合并模拟稳态流程。公开 fork PR 只能使用 GitHub-hosted 临时 runner、read-only token 和无 secret 上下文，禁止 `pull_request_target` 与发布权限。

## 保持依赖锁定

`Cargo.toml` 与 `Cargo.lock` 共同定义当前依赖图。依赖变更必须保持可审查：

- 使用精确版本，并提交对应 lockfile diff。
- 一个 PR 只处理一组有明确理由的依赖变化。
- 核对上游变更、许可证、安全公告、feature 和原生 ABI 影响。
- 不自动合并 pgrx、pgrx-pg-sys、JSON Object Signing and Encryption（JOSE）、Rust 补丁版本或 PostgreSQL minor 变化。

pgrx、Rust toolchain 或 PostgreSQL minor 变化必须由维护者人工审查，并在精确目标 runtime 重新生成证据。未运行新 minor 的情况下，不得更新支持矩阵。

## 运行可选的本地聚焦诊断

本地原生测试只用于快速定位问题，不能完成OpenSpec task、consumer gate或发布门禁。需要本地诊断时，`pg_config`必须精确指向PostgreSQL 18.4；先确认环境，再运行格式与测试：

```bash
export PGRX_PG_CONFIG_PATH="$(command -v pg_config)"
test "$("$PGRX_PG_CONFIG_PATH" --version)" = "PostgreSQL 18.4"
cargo fmt --check
cargo test --locked --no-default-features --features pg18,abi-gate
cargo test --locked --no-default-features \
  --features pg18,abi-gate,pgx-oauth-gate \
  --test pgx_oauth_gate
```

随后运行与 `Dockerfile` 相同的两组全 target Clippy 检查：

```bash
for feature_set in \
  "pg18,abi-gate" \
  "pg18,abi-gate,pgx-oauth-gate"
do
  cargo clippy --locked --all-targets --no-default-features \
    --features "$feature_set" -- -D warnings
done
```

最后运行与 `Dockerfile` 相同的两组 library Clippy 检查：

```bash
for feature_set in "pg18,abi-runtime-gate" "pg18"
do
  cargo clippy --locked --lib --no-default-features \
    --features "$feature_set" -- -D warnings
done
```

不要删除测试、弱化断言、关闭 warning 或降低检查配置来获得通过结果。

## 取得远端 Native CI 证据

根 `Dockerfile` 是官方 C header、真实 PostgreSQL runtime 和最终制品隔离的唯一 build graph；GitHub Actions 是执行和证据权威。把单一范围 commit 非 force push到短期分支后，等待 `Native CI`：

```bash
branch="$(git branch --show-current)"
gh run list \
  --repo codeh007/mtmpg \
  --workflow native-ci.yml \
  --branch "$branch" \
  --limit 5
run_id=1234567890123
gh run watch "$run_id" --repo codeh007/mtmpg --exit-status
gh run view "$run_id" --repo codeh007/mtmpg --log-failed
```

常规 PR/`main` run 使用 BuildKit/GitHub Actions cache 且无发布权限；不得用本地 tag、终端日志或反复 `--no-cache` 替代远端证据。确需定位 Docker 差异时，可按 README 构建本地 diagnostic image，但不能据此勾选任务。

Workflow 尚未被默认分支识别时，一次性 run 由批准的功能 ref push 启动。进入 `main` 后，普通 cached lane 只服务 PR/`main`；schedule/dispatch cold lane 与 trusted release lane 由后续 OpenSpec task 分别建立。

```bash
gh run rerun 1234567890123 --repo codeh007/mtmpg
```

## 审查原生 ABI 与 PostgreSQL minor

原生边界变更需要独立人工审查。以下变化不能只依赖 Rust 单元测试：

- PostgreSQL OAuth header、struct layout、magic、callback signature 或 loader 行为变化
- pgrx guard、PostgreSQL allocator、panic 或 error 边界变化
- PostgreSQL minor、Debian runtime、target、架构或 libc 变化

对应 PR 必须运行官方 C layout probe、真实 runtime probe、导出 symbol 检查和最终镜像隔离扫描。证据只能说明实际运行过的精确组合。

## 保护 secret 与证据

仓库、Git history、日志、镜像层和证据不得包含真实凭据或运行数据。提交前检查以下内容：

- 不提交私钥、API key、token、连接串、`.env`、session 或 PostgreSQL data。
- 测试只使用确定性合成 fixture，并把 fixture 限定在测试 gate。
- 扫描结果只记录命中文件或脱敏类别，不输出敏感值。
- 不读取或修改生产配置，除非任务明确授权且操作范围经过审查。

发现意外泄露时停止提交，先撤销或轮换凭据，再按[安全政策](SECURITY.md)私密报告。

公开读取的 `mtmpg-postgres` package 不需要部署 credential。Package、Release、tag 与 attestation 写权限只能由受保护 `main` 上的 trusted job 申请；普通 PR、fork、Compose、manifest 和文档示例不得包含这些权限或 credential。

## 更新 OpenSpec 与提交

OpenSpec 是行为和交付范围的权威记录。修改前后按以下顺序核对：

1. 先更新或确认 proposal、design 与 delta spec，再实现行为。
2. 只在实现和验证都完成后更新对应 task checkbox。
3. 通过项目生成命令更新生成文件，不要手写生成结果。
4. 运行可选聚焦诊断和`git diff --check`，推送后取得精确commit的远端`Native CI`成功证据。
5. 提交单一范围的源码、测试、文档与证据。

不要增加第二套源码、兼容 fallback 或包装实现来掩盖根因。
