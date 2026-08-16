# Host artifact releases evidence

## executor-v0.1.4（失败历史，保留不覆盖）

- Tag `executor-v0.1.4` 指向 `df7f681`，但该 source 的 executor Cargo version 仍为 `0.1.3`，与 tag 不一致；CI Verify 阶段在 `Validator and shared inputs` 的 version 一致性检查失败，Publish 未执行，无 image、无 Release、无附件。
- 按供应链规范（发布错误必须递增 patch 前向修复），tag 保留为失败历史，不删除、不移动、不覆盖。

## Published executor-v0.1.5 evidence

- Source: `ff8a6d598860ab5b54ed09f77b3a9d39e4495061`（main GREEN；Cargo 版本 0.1.5）
- Main CI: run `31962367348`，validator/shared、executor Rust、真实 PostgreSQL 18、final image、product selection 与 aggregate Verify 全部成功。
- Annotated tag: `executor-v0.1.5`，tag object `9bc7529720e0cea645ca1cf8add28f715d6a180f`，唯一指向上述 source。
- Release run: `31963554107`，全部 job 成功。
- GitHub Release: ID `RE_kwDOTao-p84WIxV6`，发布时间 `2026-08-16T18:16:13Z`。
- Host artifact: `mtmpg-executor`，8,116,112 bytes，sha256 `04a7bc9abc04578fa0b61c49b2e021a4c3918583dc2862dda4a9d4a7d2498973`（checksums.txt 已核验）。
- Image: `ghcr.io/codeh007/mtmpg-executor:0.1.5`。

## Published v0.2.1 evidence

- Source: `ff8a6d598860ab5b54ed09f77b3a9d39e4495061`（main GREEN；validator Cargo 版本 0.2.1）
- Annotated tag: `v0.2.1`，tag object `0feebe7bbac0bb463f40ac726de7101f4412b83d`，唯一指向上述 source。
- Release run: `31964122753`，全部 job 成功。
- GitHub Release: ID `RE_kwDOTao-p84WIyCU`，发布时间 `2026-08-16T18:28:56Z`。
- Host artifact: `pggomtm.so`，1,379,832 bytes，sha256 `b9497dff0d4762cc2bdd6393e759d3356d594fde4c9c677e347608b5f6b97708`（checksums.txt 已核验）。
- Image: `ghcr.io/codeh007/mtmpg:0.2.1`；`latest` 已前移。

## gomtm 消费侧

- gomtm `pkg/backend/host_install.go` 常量同步为 `mtmpgValidatorVersion = "0.2.1"`、`mtmpgExecutorVersion = "0.1.5"`（gomtm PR #325，随 v0.9.51 发布）。
- 下载 URL 已验证 HTTP 200：`releases/download/v0.2.1/pggomtm.so` 与 `releases/download/executor-v0.1.5/mtmpg-executor`。
