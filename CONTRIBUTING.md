# 为 pggomtm 贡献变更

本页说明如何限定变更范围、运行 locked Rust 与 Docker 门禁，并提交可人工审查的 Pull Request（PR）。所有命令都从本仓库根目录执行，不依赖其他仓库的未跟踪文件。

## 从 Issue 建立变更范围

每项工作应先关联一个 GitHub Issue，并把预期行为、非目标和验证证据写清楚。第一条命令列出 mtmpg 自有 Issue；第二条命令读取当前跨仓库任务的真实 Issue：

```bash
gh issue list --repo codeh007/mtmpg --state open
gh issue view https://github.com/codeh007/gomtmui/issues/116 \
  --json number,title,body,url,comments
```

选择并读取目标 Issue 后，按以下顺序限定变更：

1. 阅读 Issue 正文与已有讨论。
2. 阅读相关 OpenSpec proposal、design、spec 和 tasks。
3. 从当前目标分支创建名称包含 Issue 编号的短期分支。
4. 只提交该 Issue 与当前 OpenSpec task 所需的文件。

不要把无关重构、依赖升级或发布规则混入同一变更。任务完成且全部仓库工作结束后，再在 Issue 中统一回填 commit、验证结果和已知限制。

## 准备 Pull Request

PR 应让维护者能够从 clean checkout 复现结论：

- 关联 Issue 与对应 OpenSpec change，并说明本次完成的 task 范围。
- 说明行为变化、原生应用程序二进制接口（ABI）影响、安全边界和回退方式。
- 列出执行过的命令、exit code、通过数量和未运行项目。
- 附上不含 secret 的证据路径，不粘贴环境变量或完整构建凭据。
- 如果使用人工智能（AI）辅助，说明使用范围与人工核对内容。

不要对维护者正在审查的共享分支执行 `force push`。需要修正时追加清晰 commit，或先与维护者确认历史整理方式。

## 保持依赖锁定

`Cargo.toml` 与 `Cargo.lock` 共同定义当前依赖图。依赖变更必须保持可审查：

- 使用精确版本，并提交对应 lockfile diff。
- 一个 PR 只处理一组有明确理由的依赖变化。
- 核对上游变更、许可证、安全公告、feature 和原生 ABI 影响。
- 不自动合并 pgrx、pgrx-pg-sys、JSON Object Signing and Encryption（JOSE）、Rust 补丁版本或 PostgreSQL minor 变化。

pgrx、Rust toolchain 或 PostgreSQL minor 变化必须由维护者人工审查，并在精确目标 runtime 重新生成证据。未运行新 minor 的情况下，不得更新支持矩阵。

## 运行本地 Rust 门禁

本地原生测试要求 `pg_config` 精确指向 PostgreSQL 18.4。先确认环境，再运行格式与测试：

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

## 运行 Docker clean build

Docker 是官方 C header、真实 PostgreSQL runtime 和最终制品隔离门禁的当前权威环境。提交前运行 final clean build：

```bash
DOCKER_BUILDKIT=1 docker build \
  --platform linux/amd64 \
  --no-cache \
  --pull \
  --progress=plain \
  --tag pggomtm-contribution:local \
  .
```

涉及 JSON Web Token（JWT）、role、identity 或 OAuth callback 时，再构建测试专用 gate target：

```bash
DOCKER_BUILDKIT=1 docker build \
  --platform linux/amd64 \
  --no-cache \
  --pull \
  --progress=plain \
  --target pgx-oauth-gate \
  --tag pggomtm-contribution-gate:local \
  .
```

两张镜像都是本地候选，不得把 gate 镜像当作生产制品。

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

## 更新 OpenSpec 与提交

OpenSpec 是行为和交付范围的权威记录。修改前后按以下顺序核对：

1. 先更新或确认 proposal、design 与 delta spec，再实现行为。
2. 只在实现和验证都完成后更新对应 task checkbox。
3. 通过项目生成命令更新生成文件，不要手写生成结果。
4. 运行聚焦测试、Docker 门禁和 `git diff --check`。
5. 提交单一范围的源码、测试、文档与证据。

不要增加第二套源码、兼容 fallback 或包装实现来掩盖根因。
