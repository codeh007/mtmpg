# Executor release forward fix

## Immutable 0.1.0 evidence

- Annotated tag: `executor-v0.1.0`
- Source: `e78070b380834e3ab0539359d633b1c34364cfeb`
- Main CI: `29742964945`，validator、executor Rust、真实PostgreSQL 18与final-image门禁全部成功。
- Release run: `29743753472`
- Published image: `ghcr.io/codeh007/mtmpg-executor:0.1.0`
- OCI digest: `sha256:d5357f8b8ea123fc491b4a29138dd04af41d9839049346d45059885871439c83`
- Release publish job已经成功推送并匿名验证同一OCI image，生成并提交provenance与SBOM attestation；随后在上传GitHub Release assets时失败。

失败原因是仓库启用了GitHub immutable releases，而workflow先以`draft=false`发布Release，再调用asset upload。正式发布后Release立即冻结，GitHub返回`HTTP 422: Cannot upload assets to an immutable release`。`executor-v0.1.0` Release因此保持immutable且assets为空。

按照GitHub官方immutable release发布要求与本change既有前向修复边界，0.1.0 tag、image和Release均不删除、不移动、不覆盖或复用。Workflow改为先创建draft、在draft中上传全部assets，再以API发布并冻结；executor递增patch版本到0.1.1，只有新的精确main GREEN SHA才可创建`executor-v0.1.1`。

## Failed 0.1.1 evidence

- Annotated tag: `executor-v0.1.1`
- Source: `f786945eeb5ab7e42179855c71113e64982c4792`
- Release run: `29763548467`
- Published image: `ghcr.io/codeh007/mtmpg-executor:0.1.1`
- OCI digest: `sha256:9b5862f9a0dc7990e7e22be4fa3240cc8f5b998966ab631de364eaa6a40c6646`

该run的标准CI、真实PG18、final image、image push、provenance、SBOM、attestation、匿名image核验和draft附件上传全部成功；`Create immutable GitHub Release`随后失败。根因是GitHub REST的`releases/tags/<tag>`只解析已发布Release，不能读取刚创建的draft，因此返回HTTP 404。该draft包含九个完整附件但未发布；0.1.1 tag、image、attestation和draft均作为失败历史保留，不删除、不移动、不覆盖或复用。

前向修复把draft查询改为从`releases?per_page=100`分页结果中按tag与`draft=true`唯一选择数值ID，并将executor递增到0.1.2。只有新的精确main GREEN SHA才可创建`executor-v0.1.2`。

## Failed 0.1.2 evidence

- Annotated tag: `executor-v0.1.2`
- Source: `72a3ad3c69d645b5b697228c2f8c1bd3a357010f`
- Main CI: `29765277841`，validator、executor Rust、真实PostgreSQL 18与final-image门禁全部成功。
- Release run: `29766521670`
- Published image: `ghcr.io/codeh007/mtmpg-executor:0.1.2`
- OCI digest: `sha256:8ee0f8fed810bd7ab41b9989f99c70253a2220b27f529ffe8ea307a3fb57d39d`
- Draft Release ID: `356917713`，包含九个完整附件。

该run的标准CI、真实PG18、final image、image push、provenance、SBOM、attestation、匿名image核验和draft附件上传全部成功。`gh release create --draft`返回成功后，紧接着的Release列表查询暂时得到0个匹配项；数秒后同一查询可以唯一找到该draft。根因是GitHub Release列表的最终一致性，而不是tag筛选、附件或数值ID错误。

0.1.2 tag、image、attestation和draft均作为失败历史保留，不删除、不移动、不覆盖或复用。前向修复在Release列表查询中加入最多12次、间隔5秒的有界轮询：0个匹配时等待，1个匹配时继续，多个匹配时立即失败；executor递增到0.1.3，只有新的精确main GREEN SHA才可创建`executor-v0.1.3`。

## Published 0.1.3 evidence

- Source: `c7d60d7c05c751184090affd4692dcea546a3f9d`
- Main CI: `29768561261`，validator/shared、executor Rust、真实PostgreSQL 18、final image、product selection与aggregate Verify全部成功。
- Annotated tag: `executor-v0.1.3`，tag object `4758e160df262244d33dde5cfa1b2d2d5b1ad8a7`唯一指向上述source。
- Release run: `29769822247`，全部八个job成功，包括image push、provenance/SBOM attest、匿名image验证、draft解析、immutable发布和公开附件/latest policy验证。
- Published image: `ghcr.io/codeh007/mtmpg-executor:0.1.3`
- OCI digest: `sha256:d50aefc818018970bf60b1832fffc7159ad1d19a2f20f6ac7ad9fc8bfe66024b`
- GitHub Release: ID `356940003`，`draft=false`、`immutable=true`、九个附件，发布时间`2026-07-20T19:06:21Z`。

匿名Registry V2查询确认versioned image的digest与Release manifest一致，OCI config固定`10001:10001`，revision为上述source且version为`0.1.3`。匿名下载全部九个Release附件后，`checksums.txt`对Cargo.lock、resolved inputs、verified image、release manifest、SPDX SBOM、provenance bundle、SBOM attestation bundle与attestation索引全部校验成功。

Release source job的Gitleaks历史扫描、strict OpenSpec校验、Git diff检查与本地敏感模式扫描均通过；证据只记录合成身份、版本、摘要和公开运行ID，不包含credential、token、private key、连接串或生产数据。

Release material记录Rust `1.97.1`、Cargo `1.97.1`、PostgreSQL `18.4`、匹配PG18 runtime/header摘要、executor binary摘要`4d2012b441c77ab941e8af89d12902c181321c9bd4c5b2f4c0727969ef7450c1`、Cargo.lock摘要`c3980375a8b63e72f70786e0e1cad7abc9bda50e32642489c948bc9f6ac39eb9`及同一OCI digest。Cargo.lock唯一记录`mtmpg-executor 0.1.3`；SPDX 2.3材料包含101个package和3186个file。公开attestation查询返回两个Sigstore bundle，对应provenance ID `36218811`与SBOM ID `36218826`。

Validator隔离复核保持不变：`v0.2.0` Release ID仍为`356495383`且immutable，source仍为`6c5a24a95bc43b4a4ec100f83a609c4af2b4699f`，`ghcr.io/codeh007/mtmpg:0.2.0`与validator `latest`仍共同解析到`sha256:dad813ee85886bf3fa5842ff56d1bf00a46fbc9cbc5d4665c3bc471c295ef16b`。
