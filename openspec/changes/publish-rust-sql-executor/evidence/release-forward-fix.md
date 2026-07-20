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
