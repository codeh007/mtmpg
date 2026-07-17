# 配置pggomtm validator runtime

本页定义正式`pggomtm` validator唯一允许读取的配置文件与public JSON Web Key Set（JWKS）。配置文件使用版本化JavaScript Object Notation（JSON）schema。平台维护者使用该契约发布非敏感验证材料。Validator不得从环境变量、PostgreSQL Grand Unified Configuration（GUC）、启动参数、网络或Structured Query Language（SQL）读取替代配置。

## 使用固定文件路径

正式module只读取`/etc/pggomtm`下的两个固定文件。容器必须把整个目录以只读bind mount提供给PostgreSQL backend。

| 用途 | 固定容器路径 | 文件要求 |
| --- | --- | --- |
| Validator config | `/etc/pggomtm/validator.json` | 版本化JSON，最大16 KB |
| Public JWKS snapshot | `/etc/pggomtm/jwks.json` | 只含public ES256 key，最大64 KB、最多16个key |

部署不得设置`PGGOMTM_CONFIG_PATH`等路径覆盖，也不得通过GUC或host-based authentication（HBA）参数选择第二个文件。推荐的容器mount形式如下。宿主source目录由平台配置authority管理，不属于signing secret authority。

```text
--mount type=bind,src=/srv/pggomtm-candidate,dst=/etc/pggomtm,readonly
```

两个文件必须满足以下条件：

- 是普通文件，不能是symlink、device、socket、先进先出（FIFO）named pipe或目录
- 文件本身没有owner、group或other写权限；发布后的mode为`0444`
- 容器mount为只读，PostgreSQL backend不能修改文件或目录
- 文件位于同一目录和文件系统，以支持同目录临时文件到固定名称的原子rename
- Config与JWKS都必须完整存在；缺少任一文件时，新OAuth backend拒绝启动

## 使用`pggomtm-validator-config/v1` schema

`/etc/pggomtm/validator.json`必须是UTF-8编码的单个JSON object。下面的完整示例列出v1允许的全部字段。

```json
{
  "schema": "pggomtm-validator-config/v1",
  "issuer": "https://candidate.example.test/oauth/database",
  "audience": "https://candidate.example.test/resources/database/gomtm-test",
  "jwks_path": "/etc/pggomtm/jwks.json"
}
```

字段契约如下：

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `schema` | string | 必须精确等于`pggomtm-validator-config/v1` |
| `issuer` | string | 唯一token issuer；必须是无userinfo、query和fragment的absolute HTTPS resource |
| `audience` | string | 唯一database audience；使用与issuer不同的absolute HTTPS resource |
| `jwks_path` | string | 必须精确等于`/etc/pggomtm/jwks.json` |

四个字段全部必填。Parser必须拒绝重复字段、未知字段、非字符串值、尾随内容、byte order mark（BOM）和非v1 schema。配置是公开部署材料，不得包含credential或secret。

## 保持安全策略不可配置

Config只选择唯一issuer、唯一audience和固定public JWKS文件。以下策略由module与版本化consumer contract固定，不能出现在config中：

- JSON Object Signing and Encryption（JOSE）algorithm、curve、key use或key operation
- Token scope、最小或最大time to live（TTL）、clock skew或claims schema
- Profile、PostgreSQL role、profile-role映射或identity编码
- 第二issuer、第二audience、fallback、兼容token或旧validator
- Hypertext Transfer Protocol（HTTP）URL、JWKS URL、discovery、introspection、Domain Name System（DNS）、SQL或Service Provider Interface（SPI）
- Signing private key、API key、OAuth bearer、database JSON Web Token（JWT）、service credential或连接串
- Signal、reload、共享cache或既有backend重新认证选项

V1固定ES256、P-256、`use=sig`、`key_ops=["verify"]`、`database` scope、30s至300s TTL与closed profile-role集合。改变任一固定策略都需要新的module和consumer contract版本，不能通过修改JSON扩权。

## 原子发布public材料

平台publisher必须按以下顺序更新文件：

1. 在`/etc/pggomtm`对应的宿主source目录创建完整临时文件。
2. 验证内容并把文件mode设置为`0444`。
3. 使用同文件系统`rename`替换固定名称。

Publisher不得原地truncate或分段覆盖active文件。

JWKS轮换先发布同时包含active与retiring public key的完整snapshot。旧key至少保留到其最后token的300s硬上限结束后，再发布不含旧key的新snapshot。每个新OAuth backend只在startup读取一次config与JWKS；既有backend不reload，也不会因文件替换重新认证。

## 拒绝未知配置版本

Module只接受精确v1 discriminator。`pggomtm-validator-config/v2`、缺失schema或任何未知字段都会使新backend启动失败；runtime不得尝试按v1解释、读取旧cache或切换到内置key。

当前仓库已经实现本契约的固定路径文件读取、权限与schema校验、同一真实父目录与设备号检查，以及每个新backend在startup建立独立不可变snapshot并在shutdown释放。轮换测试证明临时文件半写期间active名称仍只暴露旧完整snapshot，原子rename后续backend读取完整active+retiring key集合，既有backend不reload；缺失、损坏或非原子布局会fail closed，且没有旧cache或内置key回退。正式validate callback已经消费该snapshot，并通过真实PG18.4 valid/tampered token smoke与完整actor/claims/signature矩阵。Closed profile-role、config扩权与forbidden-role probe已经取得[精确远端commit的Native CI证据](evidence/issue-116/native-ci-bootstrap.md)。Identity allocator完整往返与脱敏失败reason仍未完成，因此当前artifact仍不能作为稳定可部署validator。
