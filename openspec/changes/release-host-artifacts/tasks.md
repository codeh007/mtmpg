## 1. release 裸宿主产物

- [x] 新增 ARTIFACT_NAME env（executor→mtmpg-executor，validator→pggomtm.so）
- [x] 匿名验证后复制 artifact 到 release 资产并追加 checksum
- [x] 加入 files 数组
- [x] 加入公共资产校验循环
