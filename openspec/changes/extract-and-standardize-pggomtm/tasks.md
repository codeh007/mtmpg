## 当前基线

`pggomtm`已完成独立仓库迁移、官方OAuth ABI生成、PG18.4 loader、离线config/public JWKS、ES256 database JWT、closed role、identity、失败脱敏和无gate production module验证。完整开发基线已经非force进入远端`main`，本地工作线已切换到同一main，临时功能分支已删除。以下任务只跟踪main-first、Actions-only的最终制品与发布闭环，历史过程由Git与`docs/evidence/`保存。

任何本地Docker build/run、原生编译、临时PostgreSQL或image检查结果都不能完成以下任务。`main`验证失败时保留源码并通过后续commit修复，candidate/stable保持不变。

## 1. Main与Actions-only开发线

- [x] 1.1 将`issue-116-extract-pggomtm`非force fast-forward到远端`main`，把本地工作线切换到同一main且保留既有修改；删除workflow中的临时分支trigger并删除本地/远端临时分支
- [x] 1.2 删除AGENTS、README、CONTRIBUTING、MAINTAINERS、release/governance与OpenSpec中的本地Docker、required PR/branch protection/auto-merge和独立cold描述；让重计算入口在非GitHub Actions环境拒绝执行，并精确清理既有`pggomtm-native-refactor`container与`mtmpg-native-ci:refactor-local`image
- [x] 1.3 让`main` push的Native CI以只读默认权限分步运行source/secret、依赖/许可证、fmt/Clippy、Cargo tests、官方ABI provenance、production artifact和专用PG18.4 integration；可选PR/fork只读且无发布权限

## 2. 标准PostgreSQL 18.4 image

- [x] 2.1 精简根Dockerfile和context为固定production build与官方`postgres:18.4-bookworm`完整digest runtime，只复制`pggomtm.so`、MIT license和绑定main source的非敏感metadata，并继承官方runtime配置
- [x] 2.2 在Docker build后独立验证base layer/config、PostgreSQL 18.4、module加载、ELF/动态依赖、filesystem四项增量和metadata，证明image不含gate、fixture、Rust/Cargo/source、JWKS/config、credential或PGDATA
- [ ] 2.3 将职责分离实现直接提交到`main`，只通过精确main SHA的GitHub Actions结果定位并修复失败，取得完整远端Native CI成功结果且不使用本地重计算替代

## 3. Main candidate与供应链

- [ ] 3.1 增加仅对仓库自身成功`main` push生效的最小权限candidate job，从精确`GITHUB_SHA`只构建并推送一次`ghcr.io/codeh007/mtmpg-postgres`，输出不可变完整OCI digest且不创建stable alias或Release
- [ ] 3.2 定义并验证image内build metadata与外部`release-manifest.json`，绑定source、toolchain、PG18.4/header/base、module、`.so`和OCI digest且避免image自引用或重建补写
- [ ] 3.3 为同一candidate生成并验证SBOM、provenance/attestation和checksums，确认GHCR匿名公开读取；验证失败main、PR和fork无法取得package、Release或attestation写权限

## 4. Gomtmui远端消费验收

- [ ] 4.1 在gomtmui把`docker-compose.yml`的`postgres.image`替换为candidate完整digest，不改变现有volume、TLS、healthcheck、environment和command，且不存在本地Rust build/image fallback
- [ ] 4.2 通过gomtmui GitHub Actions验证官方initdb/volume语义、PostgreSQL 18.4、TLS启动、sub2api与pgAdmin连接、module加载和manifest/module/OCI bytes一致
- [ ] 4.3 远端完成真实database JWT OAuth、`system_user`identity、closed role、ACL/RLS正负矩阵和切回上一已验证digest的rollback；consumer evidence绑定mtmpg source/manifest/OCI digest与gomtmui source

## 5. Same-digest stable发布

- [ ] 5.1 建立promotion workflow，只接受已通过mtmpg与gomtmui证据的candidate source/digest，增加SemVer与`latest`alias并创建精确source tag和immutable GitHub Release，workflow不得运行Cargo或Docker build
- [ ] 5.2 验证tag、Release assets、manifest、checksums、SBOM、attestation、consumer evidence和公开GHCR全部指向同一OCI/module digest且不可覆盖
- [ ] 5.3 回填mtmpg #1与gomtmui #116/#117，记录main source、Native CI、OCI digest、供应链材料、Compose消费、rollback、stable结果和已知限制
