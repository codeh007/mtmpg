## Why

gomtm backend 已彻底去 Docker（gomtm#310），改为单体主进程直接运行 PostgreSQL、executor 等原生服务。当前 mtmpg 只发布 OCI image，gomtm 宿主化后需要直接下载 `pggomtm.so` 与 `mtmpg-executor` 二进制作为宿主安装产物。

## What Changes

- release workflow 把已验证的 `pggomtm.so`（validator）或 `mtmpg-executor`（executor）裸二进制作为 GitHub Release 资产发布（与 image 并行），供宿主安装下载。
- 新增 `ARTIFACT_NAME` 环境变量；publish 作业在匿名验证后把提取的 artifact 复制进 release 资产并追加 checksum；加入 files 数组与公共资产校验循环。

## Capabilities

### Modified Capabilities

- `pggomtm-release-supply-chain`: 每个 Release 除 OCI image 外，还包含裸 module/binary 宿主产物（文件名 `pggomtm.so` / `mtmpg-executor`），其 sha256 与 image 内 artifact 一致。

## Impact

- `.github/workflows/release.yml`：publish job 增发宿主产物资产。
- 消费：gomtm 可直接下载 versioned 宿主产物，无需拉取 image 提取。
