## 1. 规划与基线

- [x] 1.1 读取 AGENTS/MAINTAINERS/docs/openspec 现状，确认 3 个 active change 与 executor 边界
- [x] 1.2 建立 worktree 与分支 slim-validator-only（base=origin/main fba114e，rebase 到 1d407dc）
- [x] 1.3 创建本 openspec change 并完成 proposal/design/specs/tasks

## 2. 删除 executor 产品

- [x] 2.1 删除 executor/ 目录（源码+测试+Dockerfile）
- [x] 2.2 根 Cargo.toml 移除 executor workspace 成员（并移除孤儿 dev-dep sha2）
- [x] 2.3 .github/workflows/ci.yml 移除 executor CI 步骤（executor_domain/executor_pg18/executor_image 及 validator 内 executor 解析/libpq probe）
- [x] 2.4 .github/workflows/release.yml 移除 executor-v* release 入口与 publish 内 executor 分支
- [x] 2.5 根 Dockerfile 与 .dockerignore 移除 executor COPY/白名单
- [x] 2.6 删除 docs/executor-runtime.md；更新 README/MAINTAINERS/AGENTS/docs 中 executor 引用
- [x] 2.7 共享 PG18 harness（postgres_integration*.sh）移除 run-executor 矩阵

## 3. 契约最小化

- [x] 3.1 src/database_auth.rs 改写：最小 claims、identity 编码、枚举与错误路径
- [x] 3.2 src/auth_failure.rs 确认闭集不变（无只属于旧 claims 的 code）
- [x] 3.3 validator Cargo.toml 升 0.3.0

## 4. 测试更新

- [x] 4.1 更新 Rust 领域测试（新 claims 矩阵、拒绝旧字段、身份编码、TTL、profile==role）
- [x] 4.2 更新真实 PG18 harness fixture（新身份编码断言）与 final-image smoke
- [x] 4.3 更新 src/runtime_config.rs 测试 fixture
- [x] 4.4 ABI 测试不动；删除 executor 全部测试

## 5. 文档与 OpenSpec 收敛

- [x] 5.1 同步 README、docs/runtime-configuration.md、docs/authentication-failures.md、docs/release-and-compatibility.md
- [x] 5.2 publish-rust-sql-executor / release-host-artifacts 标注「被 issue-310 硬切取代」保留为历史
- [x] 5.3 standardize-profile-role-contract-v2 标注「被 issue-310 硬切取代」不冲突编辑

## 6. 验证与交付

- [x] 6.1 openspec validate --strict 通过（4 change + 2 spec 全绿）
- [x] 6.2 自审 diff（最小变更、无无关重构）
- [x] 6.3 提交、推送分支、gh pr create（PR #11，label enhancement+rust）
- [x] 6.4 轮询 CI 至全绿（失败只向前修复）

## 验证证据

- PR：https://github.com/codeh007/mtmpg/pull/11
- GREEN run：https://github.com/codeh007/mtmpg/actions/runs/32011254449（head bdbae1d，conclusion success）
- 首次 RED（cargo fmt）：run 32010963197，两处 jwt_identity.rs 格式差异，追加 commit bdbae1d 修复后转绿
- 分支：slim-validator-only（base main 1d407dc，经 rebase 解决 executor modify/delete 冲突，无 force push）
