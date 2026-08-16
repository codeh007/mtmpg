## MODIFIED Requirements

### Requirement: 每个Release附带宿主产物
mtmpg 的每个 GitHub Release SHALL 在 OCI image 之外，附带对应的裸宿主产物：validator 发布 `pggomtm.so`（PostgreSQL 18 module），executor 发布 `mtmpg-executor` 二进制。宿主产物的 SHA-256 SHALL 与 image 内对应 artifact 一致，并纳入 release 资产与 checksums.txt。

#### Scenario: validator release
- **WHEN** 发布一个 validator `v<semver>` tag
- **THEN** Release 资产 SHALL 包含 `pggomtm.so`，其 sha256 等于 `verified-image.json` 的 `module_sha256`

#### Scenario: executor release
- **WHEN** 发布一个 executor `executor-v<semver>` tag
- **THEN** Release 资产 SHALL 包含 `mtmpg-executor`，其 sha256 等于 `verified-image.json` 的 `binary_sha256`
