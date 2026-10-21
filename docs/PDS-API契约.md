# 互联 · PDS（阿里云盘）API 契约

> 本文记录「互联」实际使用的阿里云盘 HTTP API 契约，内容来自真机探针 [`scripts/pds_probe.py`](../scripts/pds_probe.py)、Python 端到端验证脚本 [`scripts/pds_upload_test.py`](../scripts/pds_upload_test.py)，以及完整的第三方调研报告。
>
> **完整调研证据（认证机制、官方文档引用、秒传/并行上传、限流推断等）见：[PDS-API 调研报告](PDS-API调研报告.md)。** 本文只做「App 实际使用的契约摘要 + 实测状态」，不重复调研报告内容。
>
> 标记：✅ 已实测可用 / ⚠️ 有坑或受限 / 🔲 接口可用但 App 未接入 / ❓ 未验证。

## 目录

1. [命名与版本说明](#命名与版本说明)
2. [基础信息](#基础信息)
3. [鉴权与公共请求头](#鉴权与公共请求头)
4. [通用约定](#通用约定)
5. [接口清单](#接口清单)
6. [账号与空间](#账号与空间)
7. [文件列表与搜索](#文件列表与搜索)
8. [创建目录 / 创建文件](#创建目录--创建文件)
9. [分片上传](#分片上传)
10. [断点续传](#断点续传)
11. [完成上传与校验](#完成上传与校验)
12. [下载与 Range](#下载与-range)
13. [重命名 / 移动 / 复制 / 其他可用接口](#重命名--移动--复制--其他可用接口)
14. [受限接口：删除与回收站](#受限接口删除与回收站)
15. [秒传 / 预秒传（未接入）](#秒传--预秒传未接入)
16. [错误码与排查](#错误码与排查)
17. [限流、配额、URL 与会话时限](#限流配额url-与会话时限)
18. [App 命令 ↔ API 映射](#app-命令--api-映射)
19. [安全与脱敏](#安全与脱敏)
20. [验证状态与未验证项](#验证状态与未验证项)

---

## 命名与版本说明

产品界面与 README 沿用「阿里云盘（个人版）」的通俗叫法；但真机凭据与域名实测表明，本机使用的是**阿里云盘企业版（企业文件管理 / PDS 企业版）**的 **API Key**（`uk-` 开头，静态长效凭据）：

- 企业版域名 `https://{domain_id}.api.aliyunfile.com`，控制台 `https://www.aliyunfile.com`；
- 开发者版域名 `https://{domain_id}.api.aliyunpds.com`，控制台 `https://pds.console.aliyun.com`；
- 两者跑的是同一套 **PDS 2022-03-01 OpenAPI**，差别在域名、控制台与认证方式（企业版主推 API Key，开发者版主推 OAuth/AK）。

> `uk-` **不是** OAuth AccessToken（后者是 JWT `eyJ...`、1–2 小时过期），而是企业版 API Key：创建时需设定过期时间、无 refresh 机制、每个用户最多 2 个。详见[调研报告 §0–1](PDS-API调研报告.md#0-先纠正三个前提很重要)。

## 基础信息

| 项 | 值 |
| --- | --- |
| API Host | `https://{domain_id}.api.aliyunfile.com` |
| 当前账号 Host | `https://bjxxxxx.api.aliyunfile.com` |
| `domain_id` | `bjxxxxx` |
| `drive_id` | `2`（空间「示例空间」，400GiB；`owner_type: group`，团队/企业空间） |
| `store_id` | `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`（上传 URL 的域名前缀，来自 `drive/get`） |
| 协议 | HTTPS；管理接口一律 `POST` + JSON；分片/下载为预签名 URL 的 `PUT` / `GET` |
| 认证 | `Authorization: Bearer uk-...`（API Key） |
| 字段风格 | 请求/响应 JSON 均为 `snake_case`（App 前端契约是 camelCase，Rust 端做映射） |
| 时间格式 | 文件对象为 ISO8601 字符串（`2026-10-20T15:54:20.636Z`）；`user/get` 部分时间为毫秒时间戳 |
| 端到端验证 | 已通过：Python 参考实现 + Rust PDS 客户端 / 传输引擎 live 测试 + App 内置 `--selftest`（见 [验证状态](#验证状态与未验证项)） |

## 鉴权与公共请求头

```http
POST /v2/file/list HTTP/1.1
Host: bjxxxxx.api.aliyunfile.com
Authorization: Bearer uk-xxxxxxxxxxxxxxxx
Content-Type: application/json
User-Agent: hulian/0.1 (macos)
```

- **`Authorization`**：必需，`Bearer` + API Key（`uk-` 开头）。
- **`Content-Type: application/json`**：所有 PDS API 请求必需。
- **`User-Agent`**：可选（App 使用 `hulian/<版本> (<平台>)`）。
- **预签名 URL（upload_url / 下载 url）不走 Bearer**：URL 自带 OSS 签名，`PUT` / `GET` 时**不要**带 `Authorization`，也**不要**带 `Content-Type`（详见 [分片上传](#分片上传)）。
- **API Key 属性**：静态长效凭据；创建时须设置过期时间；无 refresh 机制，到期需在控制台改期或新建；每个用户最多 2 个；一旦泄露等价于该用户权限。
- **获取方式**：控制台「API Key」页面新建；或复用本机 aliyun CLI PDS 插件配置 `~/.aliyun/pds_config.json` → `profiles[0].access_token`（`authentication_type = api_key`、`refresh_token` 为空）。

## 通用约定

1. **统一入口**：管理类接口为 `POST https://{host}/v2/...`，请求体为 JSON（`user/get` 为空对象 `{}`），无 query 参数。
2. **成功状态码**：查询 `200`；创建类（`file/create`、`file/copy`、`share_link/create`）`201`；取消分享等 `204`。
3. **错误结构**：PDS 返回 JSON（OSS 直传失败返回 XML）：

   ```json
   { "code": "Forbidden", "message": "User not authorized to operate on the specified APIs CheckRouterAccessFailed" }
   ```

4. **403 vs 404 的判读（重要）**：
   - `403 Forbidden / CheckRouterAccessFailed`：**凭据权限**没有覆盖该操作（API Key 权限勾选问题），**不是**接口不存在；
   - `404`：该路由在当前域名下不存在（例如 `/v2/drive/get_quota`）。
5. **分页**：`file/list` 使用 `limit` + `marker`，响应 `next_marker`；`next_marker` 为空串表示最后一页。`list_uploaded_parts` 用 `part_number_marker` + `next_part_number_marker`。
6. **名称冲突**：`check_name_mode` 实测有 4 个值 `ignore | auto_rename | refuse | check_name_and_hash`。`auto_rename` 同名自动改名（目录会追加时间戳后缀）；`refuse` 同名直接报错。
7. **类型不一致**：文件对象里 `size` 是 number；`get_download_url` 响应里 `size` / `crc64_hash` 是 string。Rust 端做了兼容解析。

## 接口清单

| # | Method | Path | 用途 | 状态 | App 使用 |
| --- | --- | --- | --- | --- | --- |
| 1 | POST | `/v2/user/get` | 当前用户信息 | ✅ | `test_connection` / 导入回填 |
| 2 | POST | `/v2/drive/get` | 空间信息、配额、store_id | ✅ | `get_drive_quota` / `test_connection` |
| 3 | POST | `/v2/drive/list` | 空间列表 | ✅ | 导入时选默认空间 |
| 4 | POST | `/v2/file/list` | 列目录 / 分页 | ✅ | `list_remote` / `list_folder` / 找互传目录 |
| 5 | POST | `/v2/file/search` | 按条件搜索文件 | ✅ | 可选（当前以 list 为主） |
| 6 | POST | `/v2/file/create` | 创建目录 / 初始化上传 | ✅ | 建互传目录、上传 |
| 7 | POST | `/v2/file/get_upload_url` | 刷新分片上传地址 | ✅ | URL 过期/断点续传 |
| 8 | PUT | `upload_url`（预签名） | 上传分片原始字节 | ✅ ⚠️ | 分片上传 |
| 9 | POST | `/v2/file/list_uploaded_parts` | 查询已上传分片 | ✅ | 断点续传 |
| 10 | POST | `/v2/file/complete` | 完成上传 | ✅ | 上传收尾 + 大小校验 |
| 11 | POST | `/v2/file/get` | 文件元数据（可选 `fields:"*"`） | 🔲 | 未接入 |
| 12 | POST | `/v2/file/get_download_url` | 获取下载 URL | ✅ | 下载 |
| 13 | GET | `下载 url`（预签名） | 下载文件（支持 Range） | ✅ | 下载 / 断点续传 |
| 14 | POST | `/v2/file/update` | 重命名 / 改属性 | 🔲 | 未接入 |
| 15 | POST | `/v2/file/move` | 移动 | 🔲 | 未接入 |
| 16 | POST | `/v2/file/copy` | 复制 | 🔲 | 未接入 |
| 17 | POST | `/v2/share_link/create` / `cancel` | 分享链接（可做 P2P 投递） | 🔲 | 未接入 |
| 18 | POST | `/v2/recyclebin/list` | 回收站列表 | 🔲 | 未接入 |
| 19 | POST | `/v2/file/delete` | 删除（进回收站） | ⚠️ 403 | 不提供 |
| 20 | POST | `/v2/recyclebin/trash` | 删除到回收站 | ⚠️ 403 | 不提供 |
| 21 | POST | `/v2/drive/get_quota` | 旧配额路径 | ❌ 404 | 改用 `drive/get` |

## 账号与空间

### 1）`POST /v2/user/get` ✅

请求（空 body）：

```json
{}
```

响应（脱敏示例，`.cache/probe/user_get.json`）：

```json
{
  "domain_id": "bjxxxxx",
  "user_id": "<redacted>",
  "nick_name": "演示用户",
  "user_name": "<redacted>",
  "phone": "<redacted>",
  "email": "",
  "role": "superadmin",
  "status": "enabled",
  "created_at": 1788789044279,
  "updated_at": 1788789044722,
  "default_drive_id": ""
}
```

App 用途：`test_connection` 校验凭据、用 `nick_name` 展示「已连接：演示用户」；一键导入时回填用户名。

> ⚠️ 响应含手机号/邮箱等个人信息，**不要整包落盘或打印日志**；日志只展示有限字段。

### 2）`POST /v2/drive/get` ✅

请求：

```json
{ "drive_id": "2" }
```

响应（关键字段，脱敏示例）：

```json
{
  "domain_id": "bjxxxxx",
  "drive_id": "2",
  "drive_name": "示例空间",
  "drive_type": "normal",
  "owner_type": "group",
  "status": "enabled",
  "used_size": 2673873767,
  "total_size": 429496729600,
  "store_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "encrypt_mode": "none",
  "created_at": "2026-09-07T13:48:23.390Z"
}
```

- `total_size = 429496729600` 字节 = 400GiB；配额就取 `used_size` / `total_size`（`/v2/drive/get_quota` 实测 404，不要使用）。
- `store_id` 是上传 URL 的域名前缀（形如 `https://{store_id}.data.aliyunfile.com/...`）。
- `owner_type: group` 表明这是团队/企业空间。

### 3）`POST /v2/drive/list` ✅

```json
{ "limit": 100 }
```

响应 `{ "items": [ ... ], "next_marker": "" }`，每个 item 字段与 `drive/get` 一致。

App 用途：一键导入后自动选择**第一个空间**回填 `drive_id` / `drive_name`；单账号单空间场景不做切换 UI（多空间切换在路线图）。

## 文件列表与搜索

### 4）`POST /v2/file/list` ✅

请求：

```json
{
  "drive_id": "2",
  "parent_file_id": "root",
  "limit": 100,
  "marker": "",
  "order_by": "created_at",
  "order_direction": "DESC"
}
```

| 字段 | 说明 |
| --- | --- |
| `parent_file_id` | 根目录固定填 `"root"`；子目录填目录的 `file_id` |
| `limit` | 文档 `[1,100]`；实测 101 仍返回 200，传 0 报 `InvalidParameter.Limit` |
| `marker` | 首页省略或空串，后续回填上一页 `next_marker` |
| `order_by` | `created_at`（默认）/ `updated_at` / `size` / `name` |
| `order_direction` | `ASC`（默认）/ `DESC` |
| `type` | 可选 `file` / `folder` 过滤 |
| `fields` | `"*"` 时返回 `download_url` / `thumbnail` 等扩展字段 |

响应（裁剪示例）：

```json
{
  "items": [
    {
      "drive_id": "2",
      "domain_id": "bjxxxxx",
      "file_id": "<file_id>",
      "name": "big.bin",
      "type": "file",
      "size": 12582912,
      "created_at": "2026-10-20T15:54:20.636Z",
      "updated_at": "2026-10-20T15:54:21.870Z",
      "parent_file_id": "<parent_file_id>",
      "category": "others",
      "content_hash": "<sha1>",
      "content_hash_name": "sha1",
      "crc64_hash": "0",
      "store_id": "<store_id>"
    }
  ],
  "next_marker": ""
}
```

> 响应对象字段很多（`permission`、`action_list`、`download_url` 等），App 只取必要字段。文件对象里 `size` 是 number。

**App 分页策略**：每页 `limit = 200`（文档建议 ≤100，实测 101 可用；200 未单独验证），最多翻 10 页 / 2000 条；`next_marker` 为空即结束。

### 5）`POST /v2/file/search` ✅

```json
{ "drive_id": "2", "query": "name = '文件互传'" }
```

- 类 SQL 条件字符串，响应结构类似 `file/list`（`items[]`）。
- 项目实测可用；调研报告未展开，探针脚本未覆盖该接口。
- App 当前以 `file/list` + 内存查找为主（用于复用互传目录），`search` 作为可选优化。

## 创建目录 / 创建文件

### 6a）创建目录 ✅

```json
{
  "drive_id": "2",
  "parent_file_id": "root",
  "name": "文件互传",
  "type": "folder",
  "check_name_mode": "refuse"
}
```

响应 `201`：`{ parent_file_id, type: "folder", file_id, domain_id, drive_id, file_name, encrypt_mode, created_at, updated_at }`（无 size/status）。

关于 `check_name_mode`（实测）：

- **`refuse`**：同名直接报错。**App 建「文件互传」目录用这个**——先 `file/list` 命中同名目录就复用，不存在才创建；创建失败（并发/已存在）后再查一次兜底，避免生成多余目录。
- **`auto_rename`**：同名自动改名。实测请求 `name = "_互联探针"` 返回的 `file_name` 变成 `"_互联探针_20261020_235420"`（追加时间戳后缀）。若用它建固定目录，会不断产生带后缀的新目录——不要这样用。
- `check_name_and_hash`：需要同时传 `content_hash`，否则 400。

### 6b）初始化文件上传 ✅

请求（3 分片示例）：

```json
{
  "drive_id": "2",
  "parent_file_id": "<互传目录 file_id>",
  "name": "big.bin",
  "type": "file",
  "size": 12582912,
  "part_info_list": [
    { "part_number": 1 },
    { "part_number": 2 },
    { "part_number": 3 }
  ],
  "check_name_mode": "auto_rename"
}
```

响应（`201`，脱敏示例）：

```json
{
  "parent_file_id": "<互传目录 file_id>",
  "file_id": "<file_id>",
  "upload_id": "0C3AA75C2C06413291CF36C056DC56AD",
  "rapid_upload": false,
  "exist": false,
  "type": "file",
  "revision_id": "<revision_id>",
  "domain_id": "bjxxxxx",
  "drive_id": "2",
  "file_name": "big.bin",
  "encrypt_mode": "none",
  "location": "cn-beijing",
  "part_info_list": [
    {
      "part_number": 1,
      "upload_url": "https://{store_id}.data.aliyunfile.com/<path>?<signed>",
      "internal_upload_url": "",
      "content_type": "",
      "signature_info": { "auth_type": "url", "sts_token": "" },
      "upload_form_info": null
    }
  ]
}
```

- `part_info_list` 长度 = 分片数，每片独立预签名 `upload_url`。
- `content_type` 返回**空串** → PUT 时绝不能带 Content-Type（见下节）。
- `upload_id` 必须回传给 `complete`。
- `rapid_upload: true` 表示秒传命中（App 未接入，见 [秒传](#秒传--预秒传未接入)）。
- **0 字节文件**：App 不传 `part_info_list`（`part_count = 0`），`create` 后直接 `complete`。

## 分片上传

### 8）`PUT <upload_url>`（预签名 URL） ✅ ⚠️

```http
PUT <upload_url> HTTP/1.1
Host: {store_id}.data.aliyunfile.com
Content-Length: <该分片字节数>

<分片原始字节>
```

**必须遵守（最容易卡住的一步）：**

1. **完全不要发送 `Content-Type` 头**（连空值也不行）。签名时按「无该 header」计算；实测带 `application/octet-stream` 或空值都会 **403 `SignatureDoesNotMatch`**。
2. **不要发送 `Content-MD5`**（会破坏签名）。
3. **不要发送 `Authorization`**：URL 已包含签名。
4. 只需要 `Content-Length` + 原始字节；**不要用 HEAD 探测 URL**（签名绑定 PUT/GET，HEAD 会 403）。
5. 成功返回 `200` + 响应头 `ETag`；`complete` 不需要提交 ETag。
6. **分片是否允许并行**：官方 JS SDK 为**串行**上传，原因是「服务端使用流式计算 SHA1，单个文件的分片需要串行上传，不支持多个分片并行上传」。并行模式（`parallel_upload: true`）需要每片提交 `parallel_sha1_ctx`（SHA1 中间态），App 未实现。
   → **App 默认单文件串行（`parallel_parts = 1`），不建议调高**；设置里的分片并发属于实验项，调高可能导致上传失败/校验不通过。

**分片规则**

```text
分片数 parts = ceil(size / chunk_size)
除最后一片外每片长度 = chunk_size；最后一片 = size - (parts - 1) * chunk_size
size < chunk_size → 1 片；size = 0 → 0 片（不传 part_info_list，直接 complete）
```

| 约束 | 值 | 说明 |
| --- | --- | --- |
| 单分片最小 | 100 KiB | OSS multipart 规则 |
| 单分片最大 | 5 GiB | 上传文档 |
| `part_info_list` 分片数 | ≤ 10000（官方）/ 9000（SDK 预留 buffer，App 采用） | 超过则自动放大分片 |
| 分片对齐 | 建议 64 字节倍数 | JS SDK 会把 chunk 向上取整到 64 的倍数 |
| 会话时限 | create → complete 必须 10 天内完成 | 超期作废，只能重新 create |
| 半片作废 | 某片传到一半暂停 → 该片作废，续传时整片重传 | — |
| 上传 URL 有效期 | 约 1 小时 | 过期 PUT 返回 403，用 `get_upload_url` 换新 |

**App 实现（`src-tauri/src/pds.rs`）**：

- `calc_chunk`：用户设置 MB → 若 `ceil(size/chunk) > 9000` 自动放大到 `ceil(size/9000)`，再 clamp 到 `[100 KiB, 5 GiB]`；
- `calc_part_count`：`size == 0` 返回 0，否则 `ceil(size/chunk)` 且至少 1；
- 分片读取用 `spawn_blocking` 按 offset/len 精确读，避免一次性把大文件读进内存。

## 断点续传

### 7）`POST /v2/file/get_upload_url` ✅

为若干分片换一批新地址（URL 过期 / 续传时使用，**不要重新 create**，否则会浪费已传分片）：

```json
{
  "drive_id": "2",
  "file_id": "<file_id>",
  "upload_id": "<upload_id>",
  "part_info_list": [{ "part_number": 1 }, { "part_number": 2 }]
}
```

响应 `200`：`{ domain_id, drive_id, file_id, upload_id, created_at, part_info_list: [{ part_number, upload_url, ... }] }`。

### 9）`POST /v2/file/list_uploaded_parts` ✅

```json
{
  "drive_id": "2",
  "file_id": "<file_id>",
  "upload_id": "<upload_id>",
  "limit": 100,
  "part_number_marker": 0
}
```

响应（实测）：

```json
{
  "file_id": "<file_id>",
  "upload_id": "<upload_id>",
  "uploaded_parts": [
    { "part_number": 1, "part_size": 8388608, "etag": "\"...\"", "upload_url": "", "content_type": "" }
  ],
  "next_part_number_marker": "",
  "parallel_upload": false
}
```

- `limit ≤ 100`；分页用 `part_number_marker` ← `next_part_number_marker`；**有 `etag` 即该片已完成**。
- 会话已完成/不存在时返回 `404 NotFound.UploadId`（实测：`complete` 之后再调用）。

**App 断点续传**：

- 上传会话 `file_id/upload_id/chunk/part_count` 保存在内存任务里；失败后点「重试」会用 `list_uploaded_parts` 查已完成分片、`get_upload_url` 刷新剩余片 URL，再补传并 `complete`；
- 会话失效（`NotFound.UploadId` 等）时自动清空会话、重新 create，最多重试 3 轮；
- 单分片 PUT 失败最多重试 4 次（指数退避 400ms → 8s 上限）；URL 过期类 403 直接换新 URL；
- **App 重启后不恢复未完成上传**（会话只存在内存，不写本地断点文件——路线图项）。

## 完成上传与校验

### 10）`POST /v2/file/complete` ✅

```json
{ "drive_id": "2", "file_id": "<file_id>", "upload_id": "<upload_id>" }
```

响应 `200`（实测裁剪）：

```json
{
  "file_id": "<file_id>",
  "name": "probe.txt",
  "size": 20,
  "status": "available",
  "content_hash": "<sha1>",
  "content_hash_name": "sha1",
  "crc64_hash": "11744108994224338685",
  "type": "file",
  "parent_file_id": "root"
}
```

- 若 `create` 时传了 `content_hash` 且内容不匹配，`complete` 直接 400：`InvalidParameter.ContentHash: file content hash is not matched with content_hash in complete file request`。
- 返回 `content_hash` 与本地 SHA1 一致，说明服务端会校验内容。
- **App 在 `complete` 后会比对返回的 `size` 与本地文件大小**，不一致则报错「云端文件大小不一致，请重试」，避免出现「上传成功但文件为空」的假成功。

## 下载与 Range

### 12）`POST /v2/file/get_download_url` ✅

```json
{
  "drive_id": "2",
  "file_id": "<file_id>",
  "expire_sec": 3600
}
```

| 字段 | 说明 |
| --- | --- |
| `expire_sec` | 默认 900（15 分钟），最大 115200（32 小时）；**App 使用 3600（1 小时）** |
| `file_name` | 可选，影响 `Content-Disposition` |

响应（脱敏示例）：

```json
{
  "domain_id": "bjxxxxx",
  "drive_id": "2",
  "file_id": "<file_id>",
  "revision_id": "<revision_id>",
  "method": "GET",
  "url": "https://{store_id}.data.aliyunfile.com/<path>?<signed>",
  "expiration": "2026-10-20T16:11:58.167Z",
  "size": "12582912",
  "content_hash": "<sha1>",
  "content_hash_name": "sha1"
}
```

- `method` 固定 `"GET"`；`size` / `crc64_hash` 在此响应里是**字符串**。
- 链接过期后重新调用本接口；**已经开始下载的传输即使超过有效期也不会被打断**（官方文档明确）。

### 13）`GET <下载 url>` ✅

```http
GET <url> HTTP/1.1
Range: bytes=1048576-
```

- 不带鉴权头，直接 GET；`200` 返回完整内容。
- 带 `Range` → **206 Partial Content** + `Content-Range`（实测 `Range: bytes=0-4` 返回 5 字节）。
- **不要用 HEAD 探测**（403）；大小用文件对象的 `size`。
- 支持并发分段下载后合并；URL 过期重新取。

**App 下载实现（`src-tauri/src/transfer.rs`）**：

- 流式写临时文件 `<目标文件>.hpart`，完成后 `rename` 成目标文件；
- 重试/续传时读临时文件长度作为 `transferred`，发 `Range: bytes=<已写>-`；服务端返回 416 时删掉临时文件从头来；
- 断流/超时最多自动重试 3 轮（退避 400ms → 1.6s 等），仍失败落 `error`，用户可点重试继续；
- 目标同名文件已存在时自动改名：`name (1).ext`、`name (2).ext`……；文件名中的非法字符替换为 `_`；
- 下载尺寸以任务记录的文件大小为准，写完校验 `local >= size` 才成功。

### 11）`POST /v2/file/get` 🔲

获取文件元数据（可 `fields:"*"` 带 `download_url` / `thumbnail`）。App 未接入（列表接口已够用）。

## 重命名 / 移动 / 复制 / 其他可用接口

以下接口**实测可用**，但 App 0.1.0 未接入 UI：

| 接口 | 最小请求体 | 说明 |
| --- | --- | --- |
| `POST /v2/file/update` | `drive_id`、`file_id`，可选 `name` / `description` / `labels` / `starred` / `check_name_mode` | 重命名/改属性；对 `uploading` 状态文件改名返回 **409** |
| `POST /v2/file/move` | `drive_id`、`file_id`、`to_parent_file_id` | 移动；响应 `async_task_id` 非空表示异步任务（需 GetAsyncTask 轮询） |
| `POST /v2/file/copy` | `drive_id`、`file_id`、`to_parent_file_id`，可选 `to_drive_id`/`name`/`check_name_mode` | 复制；缺 `to_parent_file_id` → 400 |
| `POST /v2/share_link/create` | `drive_id`、`file_id_list` 等 | 分享链接（201，返回 `share_id`/`share_url`/`share_pwd`），可作为「不共享 API Key 的 P2P 投递」基础（调研报告 §6.4） |
| `POST /v2/share_link/cancel` | `drive_id`、`share_id` | 取消分享（204） |
| `POST /v2/recyclebin/list` | `drive_id` | 回收站列表（200） |

> 这些能力是「路线图」候选（云端重命名/移动/复制、分享链接投递）；接入前需按当前 API Key 权限再验证一次。

## 受限接口：删除与回收站

### 19/20）`POST /v2/file/delete`、`/v2/recyclebin/trash` ⚠️ 403

```json
{ "drive_id": "2", "file_id": "<file_id>" }
```

实测（当前 API Key）返回 **403**：

```json
{
  "code": "Forbidden",
  "message": "User not authorized to operate on the specified APIs CheckRouterAccessFailed"
}
```

- `/v2/recyclebin/clear` 清空回收站同样返回 403。
- `403 CheckRouterAccessFailed` 的语义是**凭据/网关层没有该路由权限**（控制台里该 API Key 的权限未勾选删除类操作），**不是接口不存在**（不存在的路由是 404，如 `/v2/drive/get_quota`）。

**结论**：

- **App 不提供云端删除功能**（UI 只做「可见 + 可下载」）；
- 删除请到阿里云盘网页控制台 / App 操作；
- 不要用「失败就重试」处理 403——这是权限限制，不是瞬时错误。

## 秒传 / 预秒传（未接入）

> App 当前**不使用**秒传；以下为接口事实，供路线图参考。

- **秒传（rapid upload）**：`file/create` 时带整文件 SHA1：

  ```json
  {
    "drive_id": "2", "parent_file_id": "root", "name": "x.jpg", "type": "file",
    "size": 13381200,
    "content_hash": "<整文件 SHA1 大写 HEX>", "content_hash_name": "sha1",
    "part_info_list": [{ "part_number": 1 }]
  }
  ```

  响应 `"rapid_upload": true` ⇒ 命中，**不返回 part_info_list，直接跳过 PUT/complete**；`false` ⇒ 正常上传。去重范围是 **Domain 级**。

- **预秒传（pre_hash）**：只传前 1KiB 的 SHA1 探路：返回 409 → 云端**可能**存在（再算全量 SHA1 尝试秒传）；返回 201 → 云端**肯定**没有（直接完整上传）。后台异步计算，刚上传的文件分钟级内可能不命中。
- **`proof_code` / `content_md5`**：`proof_code` 是阿里云盘**消费版**（`api.alipan.com`）的秒传概念，PDS 2022-03-01 的 CreateFile **没有**该字段；PDS 侧等价物是 `pre_hash`。`content_md5` 是分片级可选字段（`part_info_list[].content_md5`）。详见[调研报告 §3.4](PDS-API调研报告.md#34-秒传--预秒传)。

## 错误码与排查

| HTTP | `code` 示例 | 场景 | 处理建议 |
| --- | --- | --- | --- |
| 200 | — | 查询/完成成功 | — |
| 201 | — | `file/create` / `file/copy` 成功 | 解析 `file_id` / `upload_id` / `part_info_list` |
| 204 | — | 取消分享等成功 | — |
| 400 | `InvalidParameter.*` | 字段缺失/类型错误/分片数不合法/`expire_sec` 超限 | 报错文案会直接说明缺什么；检查 `drive_id`、`parent_file_id`、`size`、`part_info_list` |
| 401 | `Unauthorized` ❓ | API Key 无效/过期 | 到控制台改期或新建 Key，在设置中更新 |
| 403 | `Forbidden / CheckRouterAccessFailed` | 删除/回收站等 API Key 无权限 | 不重试；App 不提供该操作 |
| 403 | `SignatureDoesNotMatch`（OSS XML） | PUT 带了 Content-Type/Content-MD5，或 URL 过期/被改动、用了 HEAD | 去掉多余 header；URL 过期用 `get_upload_url` 换新 |
| 404 | `NotFound.UploadId` | `upload_id` 不存在/已完成 | 不要对已完成上传调用 `list_uploaded_parts`；重建上传会话 |
| 404 | 路由不存在 | 如 `/v2/drive/get_quota`、`/v2/file/download` | 改用正确路由（`drive/get`） |
| 409 | `InvalidParameter.CheckNameMode` / 改名 `uploading` 文件 | 预秒传探路命中 / 文件仍在传输中 | 预秒传按 409 语义处理；等上传完成再改名 |
| 416 | Range Not Satisfiable | Range 失效（云端文件变化/临时文件不匹配） | 删除临时文件从头下载 |
| 429 | `TooManyRequests` ❓ | 触发限流（未实测） | 指数退避 + 降低并发 |
| 5xx | `InternalError` / `ServiceUnavailable` | 服务端瞬时故障 | 有限次重试 + 指数退避（500ms 起，上限 30s） |

> ❓ 表示未在真机触发验证；App 侧把错误映射成中文提示（`error.rs`），并保留原始 `code` 便于排查。

## 限流、配额、URL 与会话时限

- **配额**：`drive/get` 的 `used_size` / `total_size`（本空间 400GiB，实测时点已用约 2.49GiB）。上传前建议检查剩余空间。
- **限流**：官方「使用限制」页**没有** QPS/并发/带宽数字（只有每账号单 region 最多 3 个 domain 等限制）；平台侧走 API 网关，存在按 API 流控的可能性但当前配置为空。实际瓶颈更可能是单分片 PUT 的 OSS 带宽。App 侧策略：任务并发默认 2（限制 1–6）、单文件分片默认串行、失败指数退避；不做无限重试。
- **上传 URL**：约 1 小时有效；过期 PUT 返回 403，用 `get_upload_url` 换新（不要重新 create）。
- **上传会话**：create → complete 必须 10 天内完成，超期作废。
- **下载 URL**：请求时通过 `expire_sec` 指定（默认 900s，最大 115200s；App 用 3600s），响应 `expiration` 给出实际过期时间；流式中断重试时若已过期，重新 `get_download_url`。
- **大小约束**：单分片 ≤ 5GiB × 分片数 ≤ 10000 → 理论上限约 48TB；实际受空间配额限制。App 用 9000 片上限并自动放大分片。

## App 命令 ↔ API 映射

| App 命令（Tauri） | 调用的 PDS API | 备注 |
| --- | --- | --- |
| `test_connection` | `user/get` + `drive/get` | 返回 `ConnectionStatus`（用户/空间/配额） |
| `import_local_cli_config` | `user/get` + `drive/list` | 导入后自动回填用户名、默认空间（第一个） |
| `get_drive_quota` | `drive/get` | 不调用 `get_quota`（404） |
| `list_remote` | `file/list`（互传目录，`created_at DESC`，自动翻页） | 先确保互传目录存在 |
| `list_remote_folders` | `file/list`（互传目录，过滤 `type=folder`） | 目录浏览 |
| `list_folder` | `file/list`（`parent_file_id`） | 分页合并 |
| `upload_files` / `pick_and_upload` | `file/list`（找目录）→ `file/create`（目录/文件）→（必要时 `get_upload_url` / `list_uploaded_parts`）→ `PUT upload_url` × N → `file/complete`（校验 size） | 默认串行分片，失败重试 |
| `download_files` | `file/get_download_url` → `GET url`（Range 续传） | 流式写盘、临时文件、断点续传 |

## 安全与脱敏

- 探针与 App **不得**把完整 `Authorization`、`upload_url`、下载 URL、手机号/邮箱写入日志；探针 `redact()` 会把 URL query 替换为 `?<signed>`、token 替换为 `<redacted>`。
- `.cache/probe/*.json` 为脱敏响应存档（`.gitignore` 已排除），仅用于开发对照。
- 预签名 URL 是短期凭据：不持久化、不分享；`store_id` 与 URL 中的签名参数都属敏感信息。
- `uk-` API Key 不要提交仓库；`.gitignore` 已排除 `.env`、`.cache/` 等路径。
- App 配置文件中 token 为明文（Unix 权限收紧为 `0600`），未接入系统钥匙串（路线图）。

## 验证状态与未验证项

### 已验证（本机，2026-10）

**Python 参考实现（`scripts/pds_upload_test.py`）**

- 10.5MiB 文件、3 分片（4MiB + 4MiB + 余数）：`create → PUT × 3 → complete` 成功，下载回来 **SHA-256 完全一致**；
- `Range: bytes=...` 下载返回 **206 Partial Content**；
- 断点续传：`list_uploaded_parts` 查到已完成分片 → `get_upload_url` 换新 URL → 补传剩余分片 → `complete`，最终 `size` 正确；
- `PUT` 不带 Content-Type 返回 200；带 Content-Type 返回 403 `SignatureDoesNotMatch`（这正是最早 `.cache/probe` 中文件 `size = 0` 的根因：早期探针的 PUT 带了默认 Content-Type，OSS 拒收分片，而 `complete` 仍创建了 0 字节文件）。

**Rust 客户端 / 传输引擎（`src-tauri`，`#[ignore]` 真机用例）**

- `cargo test --lib -- --ignored live_multipart_roundtrip`：PDS 客户端 5MiB+123B / 2 分片，`create → PUT → complete` 后下载，**字节完全一致**；
- `cargo test --lib -- --ignored live_engine_upload_and_download`：传输引擎 3MiB 上传 → 下载 → **字节完全一致**。

> 另：App 内置 `--selftest`（真实上传→下载→字节比对，输出 `SELFTEST_UPLOAD_DOWNLOAD OK`），作为发布前冒烟入口。

> 以上真机用例默认被 `#[ignore]` 跳过，需要本机 `~/.aliyun/pds_config.json` 凭据；CI 只做构建，不会运行它们。

### 未验证 / 待确认

1. **分片并发**：官方要求串行（服务端流式 SHA1）；App 的 `parallel_parts` 设置调高后是否失败、是否需要 `parallel_sha1_ctx`，未实测；文档建议保持 1（实现上限 4）。
2. **限流/QPS**：无官方数字，未压测；429 未实测。
3. **`file/search`**：项目实测可用，但探针与调研报告均未展开字段细节。
4. **`file/list` 每页 200**：App 实现用 200（文档写 ≤100，实测 101 可用）；200 是否稳定未单独验证。
5. **秒传 / 预秒传**：接口语义已确认，App 未接入。
6. **分享链接**：`share_link/create` 实测 201、`cancel` 204（调研报告），App 未接入。
7. **Android 真机**：上传/下载链路代码已实现，但未在真机验证（见 [使用说明](使用说明.md#已知限制)）；iOS 未适配。
