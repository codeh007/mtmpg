# pggomtm 维护规则

公开`main`是唯一源码与CI/CD主线，不表示stable。版本、制品和兼容边界以[发布契约](docs/release-and-compatibility.md)为准。

## 源码与构建权威

- 本仓库维护唯一crate、测试、Dockerfile和release workflow。
- 消费者只使用mtmpg发布的版本化image，不复制源码或现场编译。
- 所有Cargo、PostgreSQL和Docker重计算只在GitHub Actions执行。
- 维护者和Agent可以直接非force推进`main`；失败commit保留并由后续commit修复。
- Pull Request是可选外部贡献入口，始终只读且不得获得package、Release或attestation写权限。
- `Verify`是`main`的required check。只有具备write权限的owner或Agent可以启用GitHub原生auto-merge；Dependabot PR经维护者确认后可以启用，外部贡献者PR必须由维护者人工决定且workflow不得自行合并。

## 依赖与兼容审查

Rust stable、PG18 minor、兼容Cargo依赖、Actions major内版本和标准工具由每次CI自动解析。维护者审查上游变更造成的实际构建、ABI、领域或运行失败，不在源码中长期冻结patch或digest。

以下变化仍需显式技术决策：

- PostgreSQL major、pgrx不兼容major或OAuth ABI变化
- Database token、profile-role、identity或reason-code contract变化
- mtmpg SemVer、release manifest schema和promotion权限变化
- 新平台、架构、libc或runtime发行版

## Candidate与stable

Candidate必须绑定精确source、临时Cargo.lock、resolved inputs、module、OCI digest、SBOM、provenance和attestation。只有仓库自身成功`main` push的publish job可以写入公开GHCR，且只能推送只读job已验证的OCI archive，不得重建。

Gomtmui按SemVer candidate运行真实PG18、OAuth、identity、ACL/RLS和rollback验收。Stable promotion只能给同一digest增加稳定SemVer、`latest`和immutable GitHub Release；任何既有version、tag、asset或Release都不得覆盖。

## 禁止操作

- force push或改写失败历史
- 在本地或生产主机编译module、构建image或启动临时PostgreSQL
- 在backend运行时热覆盖`.so`
- 增加旧validator、备用issuer、network fallback或宽松解析
- 用本地日志、可变tag或另一构建结果替代远端证据

意外凭据暴露时必须先撤销或轮换，再清理公开材料。实现和发布进度通过[mtmpg #1](https://github.com/codeh007/mtmpg/issues/1)与active OpenSpec change追踪。
