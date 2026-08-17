> ⚠️ 本 change 的 executor 宿主产物部分（`mtmpg-executor`）已被 gomtm issue #310 硬切取代：executor 产品整个删除。validator 宿主产物（`pggomtm.so`）仍保留。以下 executor 相关任务不再执行，保留本文件作为不可变历史。

## 1. release 裸宿主产物

- [x] 新增 ARTIFACT_NAME env（executor→mtmpg-executor，validator→pggomtm.so）
- [x] 匿名验证后复制 artifact 到 release 资产并追加 checksum
- [x] 加入 files 数组
- [x] 加入公共资产校验循环
