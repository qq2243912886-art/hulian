# 阿里云盘 `*.api.aliyunfile.com` HTTP API 契约与认证调研

> 调研对象：`bjxxxxx.api.aliyunfile.com`（domain_id `bjxxxxx`，drive_id `2`，400GiB 团队空间）
> 方法：官方文档（help.aliyun.com）+ OpenAPI 元数据接口 + `aliyun pds` 插件二进制/`--cli-dry-run` + 对本机凭据的真实调用探测
> 标记：**[已证实]** = 有官方文档或实测支撑；**[推测]** = 无权威来源，给出推断与验证方法

---

## 0. 先纠正三个前提（很重要）

| 你的假设 | 实际情况 | 依据 |
|---|---|---|
| 这是「PDS 个人版（Personal Edition）」 | 这是 **阿里云盘企业版（= PDS 企业版 / 企业文件管理）**。PDS 官方只有两个版本：**企业版**（`drive-and-photo-service-ent`）与**开发者版**（`drive-and-photo-service-dev`），文档里**没有**「个人版」这个产品 | [什么是 PDS](https://help.aliyun.com/zh/pds/product-overview1) |
| `uk-` 是 OAuth token / PDS AccessToken | `uk-` 是**企业版的 API Key**，一种静态长效凭据，**不是** OAuth AccessToken（后者的形态是 JWT `eyJ...`，1~2 小时过期） | [API Key 创建与使用](https://help.aliyun.com/zh/pds/drive-and-photo-service-ent/user-guide/create-and-use-api-keys) |
| 有 `open.aliyunfile.com` 这类「个人版开放平台」控制台 | `open.aliyunfile.com` **不存在**（`*.aliyunfile.com` 是泛解析，TLS 握手直接失败）。企业版入口是 `https://www.aliyunfile.com` | 实测 DNS + TLS；`www.aliyunfile.com` 首页标题为「企业文件管理」 |

企业版域名（`{domain_id}.api.aliyunfile.com`）与开发者版域名（`{domain_id}.api.aliyunpds.com`）**跑的是同一套 PDS 2022-03-01 OpenAPI**，差别在：域名、控制台、以及**鉴权方式**（企业版主推 API Key，开发者版主推 OAuth/AK）。

---

## 1) 认证

### 1.1 结论速览

| 问题 | 结论 | 状态 |
|---|---|---|
| 能力归属 | 阿里云盘企业版（PDS 企业版），API 面 = PDS 2022-03-01 | 已证实 |
| 控制台入口 | 企业版：`https://www.aliyunfile.com` →「企业网盘用户控制台」；开发者版：`https://pds.console.aliyun.com` | 已证实 |
| `uk-` 类型 | 企业版 **API Key**（Bearer 静态凭据） | 已证实 |
| 创建方式 | 控制台左侧「API Key」→「新建 API Key」 | 已证实 |
| 有效期 | 创建时**必填**「过期时间」；无 refresh 机制，到期只能改期或新建 | 已证实 |
| 配额 | **每个用户最多 2 个** API Key | 已证实 |
| refresh_token | API Key **没有** refresh_token；`pds_config.json` 里 `refresh_token:""`、`expires_in:0` 即此意 | 已证实 |
| OAuth 登录 | 路由存在（`GET /v2/oauth/authorize`），但 client_id 只能从**开发者版**控制台创建应用获得 | 已证实（路由）/ 推测（企业版是否开放注册） |
| 个人访问令牌 | **有**，就是 API Key，可在网页控制台直接生成，适合单用户自用 App | 已证实 |

### 1.2 `uk-` token 是什么

官方文档明确写出脱敏形态 `uk-6****995f`，并把它定义为「企业文件管理为外部 Agent 提供的**身份凭证**」：

- 入口：登录企业网盘用户控制台 → 左侧导航「API Key」→「新建 API Key」
- 创建字段：`API Key 名称`（≤50 字符，必填）、`访问权限`（必填）、`过期时间`（必填）
- 访问权限两种模式：
  - **保持与登录用户的空间与权限一致**（动态继承该用户全部空间与权限）
  - **自定义**（在该用户权限范围内手动勾选空间 + 操作权限）
- 状态：已启用 / 已禁用 / 已过期；可禁用、启用、删除
- 配额：每用户 2 个
- 权限上限：「API Key 创建后，权限默认**小于等于**该用户在网盘内的权限范围」

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-ent/user-guide/create-and-use-api-keys>、<https://help.aliyun.com/zh/pds/drive-and-photo-service-ent/user-guide/enable-api-keys>

**传法**（已实测）：`Authorization: Bearer uk-...`。PDS 公共参数文档规定 AccessToken 鉴权就是 `Authorization: Bearer {AccessToken}`；本机 CLI 实测 200。

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/developer-reference/common-parameters>

### 1.3 `aliyun pds config --authentication-type api_key` 里的 api_key 是什么

**就是上面那个 `uk-` API Key。** 实测证据：

1. 本机 `~/.aliyun/pds_config.json` 中 `authentication_type` 为 `api_key`，`access_token` 字段存的就是 `uk-...`（即 api_key 被写进 access_token 字段）
2. `aliyun pds config --help` 的 `--api-key` 参数说明原文：`api key`
3. 插件二进制里 api_key 分支紧邻字符串 `Bearer `，即拼成 `Authorization: Bearer <api_key>`
4. `authentication_type` 只有三种合法值：`ak` / `token` / `api_key`；缺失时报错 `pds profile default api_key not found`

`--authentication-type token` 是「拿 user_id + access_token 走用户身份」，`api_key` 是「拿 API Key 走用户身份」，两者最终都是 `Authorization: Bearer`，差别在 token 来源与生命周期。

### 1.4 `aliyun pds authorize` 的 client_id 从哪来

- CLI 帮助原文：`The client ID assigned when you create an app` / `The ID of the app you create in the Alibaba Cloud PDS console`
- 官方接入文档：先在 **`https://pds.console.aliyun.com`** 创建 Domain（会得到 `https://{domainId}.api.aliyunpds.com`），再在该域下**创建 App（Client）**，选择应用类型（Web服务端 / Web浏览器 / Native / JWT），得到 `AppId(ClientId)` 与 `Secret(ClientSecret)`
- **企业版（aliyunfile）侧没有找到应用注册入口** → 定性为**推测**：企业版不开放自建 OAuth 应用，第三方接入统一走 API Key。
  - 验证方法：用管理员登录 `https://www.aliyunfile.com`，看是否有「开放平台 / 应用管理 / OAuth 应用」菜单；或 `curl 'https://bjxxxxx.api.aliyunfile.com/v2/oauth/authorize?client_id=<你的AppId>&...'` 看是否还能报 `InvalidParameter.ClientId`。

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/user-guide/oauth-2-0-access-process-for-mobile-applications-and-desktop-applications>、<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/user-guide/application-access-details/>

### 1.5 OAuth 流程（开发者版语义，企业版路由同样存在）

**(a) 请求授权**

```
GET /v2/oauth/authorize?client_id=<appId>&redirect_uri=<uri>&scope=<scope>&response_type=code&login_type=<type>&state=<state>&hide_consent=<bool>&lang=zh_CN
Host: {domainId}.api.aliyunpds.com   （企业版为 {domainId}.api.aliyunfile.com）
```

| 参数 | 必选 | 说明 |
|---|---|---|
| `client_id` | 是 | AppId |
| `redirect_uri` | 是 | 必须与建 App 时填的一致 |
| `scope` | 是 | 权限列表，展示在授权页 |
| `response_type` | 是 | 固定 `code` |
| `state` | 推荐 | 原样回传，防 CSRF |
| `login_type` | 是 | 见下 |
| `hide_consent` | 否 | `true` 跳过 consent 页 |
| `lang` | 否 | `zh_CN` / `en_US` |

**redirect_uri 支持自定义 scheme 与 loopback，均已在文档中给出示例 [已证实]：**
- 自定义 Scheme（Android/iOS）：`pdshz001://callback/` → 回调 `pdshz001://callback/?code=xxxx&state=abc`
- 桌面端 Loopback：`http://127.0.0.1:3000/callback` 或 `http://[::1]:3000`

所以 `hulian://callback` 这种写法是**被支持的**（前提：建 App 时把该值登记进 redirect_uri 白名单）。

**login_type 两处口径不一致（要注意）：**

| 来源 | 取值 |
|---|---|
| 官方接入文档（`api.aliyunpds.com` 时代） | `default`、`phone`、`ding`、`ldap`、`wx`、`ram` |
| `aliyun pds authorize --help`（由 2022-03-01 OpenAPI 元数据生成，较新） | `default`、`ding`、`ding_sns`、`ram`、`wechat`、`wechat_app` |

→ 结论：**`default`（聚合登录页）最安全**；`ding`/`ram` 两版都有；`phone`/`ldap`/`wx` 与 `ding_sns`/`wechat`/`wechat_app` 请按实际产品版本实测。**未找到权威来源**说明企业版到底启用哪几种，取决于该 domain 在控制台「账号接入」里开了哪些登录方式。

**(b) code 换 token**

```
POST /v2/oauth/token
Host: {domainId}.api.aliyunfile.com
Content-Type: application/x-www-form-urlencoded

code=xxx&client_id=xxx&redirect_uri=pdshz001://callback&grant_type=authorization_code[&client_secret=xxx]
```

**⚠️ 必须是 form-urlencoded，不能发 JSON [已实测]**：发 `application/json` 会返回
`{"code":"InvalidRequestJSONFormat","message":"The input parameter request body is invalid Json format. Please refer to document."}`

响应（`Token` 结构）：

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIs...",
  "refresh_token": "060e78d36afb4879b51e4264e9541c16",
  "expires_in": 3600,
  "token_type": "Bearer",
  "user_id": "DING-xxxxx", "user_name": "pdsuser", "nick_name": "pdsuser",
  "avatar": "aliyunpds.com/a.jpg",
  "default_drive_id": 1, "default_sbox_drive_id": 1,
  "role": "admin", "status": "enabled",
  "expire_time": "2019-09-01T06:57:48.813Z",
  "is_first_login": false,
  "device_id": "8574ecc3d4904dc095cc49202e74db94",
  "domain_id": "bj1",
  "state": "...", "need_link": false, "need_rp_verify": false, "pin_setup": false
}
```

- Native/桌面端**不要传 client_secret**（文档明确：secret 放客户端不安全，故 Native 应用换取 token 时不需要 secret）
- Native 应用文档另有一句「本次请求的返回值…但不包含 refresh_token」——与上表冲突。**推测**：该限制针对特定 Native 配置；**验证方法**：用真实 client_id 走一次，看响应里有没有 `refresh_token`。
- AccessToken 有效期：`expires_in: 3600`（Token 结构示例）/ 移动端文档写「有效期 2 小时」/ 移动端返回示例 `expire_in: 7200`。**口径不一，以响应实际值为准**。

**(c) 刷新**

```
POST /v2/oauth/token
Content-Type: application/x-www-form-urlencoded

refresh_token=xxx&client_id=xxx&grant_type=refresh_token[&client_secret=xxx]
```

响应结构同上（不含 refresh_token，`expires_in: 3920`）。

证据：
- <https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/user-guide/oauth-2-0-access-process-for-mobile-applications-and-desktop-applications>
- <https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/developer-reference/api-pds-2022-03-01-token>
- CLI 实测 `--cli-dry-run`：`GET /v2/oauth/authorize`（query）+ `POST /v2/oauth/token`（body `{client_id, grant_type, code, redirect_uri, refresh_token, assertion}`）

**(d) 实测到的路由存在性**

```
GET  /v2/oauth/authorize?...&client_id=test  -> 400 {"code":"InvalidParameter.ClientId","message":"The input parameter client_id is not valid. "}
POST /v2/oauth/token  (form)                 -> 400 InvalidParameter.ClientId
POST /v2/oauth/token  (json)                 -> 400 InvalidRequestJSONFormat
```

说明：路由**存在且匿名可访问**，只是没有有效 client_id。

### 1.6 对「第三方跨平台互传 App」的现实建议

1. **单用户自用 / 小范围**：直接用 API Key（`uk-`）+ `Authorization: Bearer`，零 OAuth 成本。缺点：一个 Key 一份权限，最多 2 个，`uk-` 一旦泄露等价于该用户权限。
2. **要分发给别人用**：企业版**没有**公开的 OAuth 应用注册 → 只能每个用户各自在自己控制台生成 API Key 手工填入 App。**这是产品级硬约束，建议尽早验证**（见 1.4 的验证方法）；若确认企业版无 OAuth，跨用户分发要么改用「分享链接」模式，要么换 PDS 开发者版（`api.aliyunpds.com`，有完整 OAuth）。

---

## 2) 文件列表 / 目录

### 2.1 `POST /v2/file/list` — 列举目录

**请求体**（`Content-Type: application/json`）

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `drive_id` | string | 二选一 | 空间 id；与 `share_id` 必须有一个 |
| `share_id` | string | 二选一 | 分享 id；此时需 `x-share-token` 头，`drive_id` 失效 |
| `parent_file_id` | string | **是** | 父目录 id，根目录填 `root` |
| `limit` | int | 否 | 文档写 `[1,100]`；**实测传 101 仍返回 200**，传 0 报 `InvalidParameter.Limit: limit should be positive number` |
| `marker` | string | 否 | 分页游标，传上一页 `next_marker` |
| `status` | string | 否 | `available`(默认) / `uploading` |
| `order_by` | string | 否 | `created_at`(默认) / `updated_at` / `size` / `name` |
| `order_direction` | string | 否 | `ASC`(默认) / `DESC` |
| `category` | string | 否 | `image`/`video`/`audio`/`doc`/`app`/`zip`/`others` |
| `type` | string | 否 | `file` / `folder` |
| `fields` | string | 否 | `*` 或 `url,exif,...`（`*` 才返回 `download_url`/`thumbnail`） |
| `image_thumbnail_process` | string | 否 | 如 `image/resize,w_200`；**实测被接受**（老参数名，仍可用） |
| `video_thumbnail_process` | string | 否 | 如 `video/snapshot,t_1000`；**实测被接受** |
| `thumbnail_processes` | object | 否 | 新式 map；**实测传 JSON 对象报 `InvalidRequestJSONFormat`**，需传字符串化的 JSON |

**响应**

```json
{ "items": [ /* File 对象数组 */ ], "next_marker": "" }
```

`next_marker` 为空串 = 最后一页。**分页用法**：把 `next_marker` 原样塞回 `marker`。（文档对 `marker` 的措辞是「按字母序排在该值之后的第一个」，但对客户端而言就是回填 `next_marker`，实测如此。）

**File 对象主要字段**（实测返回）

```
drive_id, domain_id, file_id, name, type(file|folder), size, content_type, category,
created_at, updated_at, parent_file_id, status(available|uploading), hidden, starred,
content_hash, content_hash_name("sha1"), crc64_hash, file_extension,
revision_id, upload_id, encrypt_mode, creator_type, creator_id, creator_name,
last_modifier_type/id/name, sync_flag, sync_meta,
punish_flag, meta_name_punish_flag, meta_name_investigation_status,
action_list[ "FILE.DELETE","FILE.MOVE","FILE.LISTTRASH","FILE.SHARELINK","FILE.GET",
             "FILE.VISIBLE","FILE.PREVIEW","FILE.CREATE","FILE.EDIT","FILE.COPY",
             "FILE.LIST","FILE.UPDATE","FILE.DOWNLOAD" ],
download_url, thumbnail, thumbnail_urls, image_media_metadata, video_media_metadata,
labels, description, user_tags, id_path, name_path, dir_size_info, trashed_at
```

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/developer-reference/api-pds-2022-03-01-listfile>

### 2.2 其余目录操作（全部 POST + JSON + Bearer）

| 操作 | Path | 必填体字段 | 可选字段 | 实测 |
|---|---|---|---|---|
| 建文件夹 | `/v2/file/create` | `drive_id`,`parent_file_id`,`name`,`type:"folder"` | `check_name_mode`,`file_id`(覆盖) | 201，返回 `file_id`/`domain_id`/`drive_id`/`file_name`/`created_at`（**无 size/status**） |
| 获取元数据 | `/v2/file/get` | `file_id`（`drive_id` 或 `share_id`） | `fields:"*"`,`url_expire_sec`(10~14400),`thumbnail_processes` | 200，`fields:"*"` 时带 `download_url` |
| 重命名/改属性 | `/v2/file/update` | `drive_id`,`file_id` | `name`,`description`,`labels[]`,`starred`,`hidden`,`local_modified_at`,`check_name_mode` | 200，返回完整 File；对 `uploading` 状态文件改名 → **409** |
| 移动 | `/v2/file/move` | `drive_id`,`file_id`,`to_parent_file_id` | `check_name_mode` | 200；`async_task_id` 非空表示异步（需 GetAsyncTask 轮询） |
| 复制 | `/v2/file/copy` | `drive_id`,`file_id`,`to_parent_file_id` | `to_drive_id`,`name`,`check_name_mode` | 201；缺 `to_parent_file_id` → 400 `InvalidParameter.ToParentFileId` |
| 删除 | `/v2/file/delete` | `drive_id`,`file_id` | — | **403 `Forbidden` / `CheckRouterAccessFailed`（本 Key 无删除权限）** |
| 删除到回收站 | `/v2/recyclebin/trash` | 同上 | — | 同样 403 |
| 回收站列表 | `/v2/recyclebin/list` | `drive_id` | — | 200 |

**`check_name_mode` 实测是 4 个值（比文档多一个）：**

```
ignore | auto_rename | refuse | check_name_and_hash
```
`check_name_and_hash` 要求同时传 `content_hash`，否则 → `InvalidParameter.CheckNameMode: content_hash is required`；非法值 → `InvalidParameter.CheckNameMode: check_name_mode should be ignore, auto_rename, refuse or check_name_and_hash`。

**关于 `403 CheckRouterAccessFailed` 的判读（重要）**：本机 API Key 的「访问权限」很可能没勾选删除类权限。这个错误码的语义是**凭据/网关层无该路由权限**，**不是**「接口不存在」（不存在的接口返回 404，例如 `/v2/drive/get_quota`）。排查时优先去控制台看该 API Key 的权限勾选。

---

## 3) 上传（大文件分片 + 断点续传）

### 3.1 标准三步（已端到端实测通过）

```
① POST /v2/file/create    → 拿 file_id / upload_id / part_info_list[].upload_url
② PUT  <upload_url> × N   → 裸 PUT 分片字节（串行！）
③ POST /v2/file/complete  → 收尾，文件变 available
```

#### ① 创建文件

```json
POST /v2/file/create
{
  "drive_id": "2",
  "parent_file_id": "root",
  "name": "probe.txt",
  "type": "file",
  "size": 20,
  "content_hash": "E362E81941CB4C9904338DF688E973557E3FA394",
  "content_hash_name": "sha1",
  "pre_hash": "E362E81941CB4C9904338DF688E973557E3FA394",
  "part_info_list": [{"part_number": 1}, {"part_number": 2}],
  "check_name_mode": "auto_rename"
}
```

**响应 201（实测，字段裁剪）**

```json
{
  "parent_file_id": "6ad78f26...",
  "upload_id": "765C7C890FC74BF09DFAB4D35F9B3D60",
  "rapid_upload": false,
  "type": "file",
  "file_id": "6ad78f262d9cd149463c4eacadfee68d52124f65",
  "revision_id": "6ad78f26b7a1c6d563504493b68c488cfcc497a9",
  "domain_id": "bjxxxxx",
  "drive_id": "2",
  "file_name": "probe.txt",
  "encrypt_mode": "none",
  "location": "cn-beijing",
  "part_info_list": [{
    "part_number": 1,
    "upload_url": "https://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.data.aliyunfile.com/ZxL5mAgL%2F2%2F...?di=bjxxxxx&dr=2&f=...&op=u&partNumber=1&security-token=CAIS...",
    "internal_upload_url": "https://...",
    "content_type": "",
    "signature_info": {"auth_type": "url", "sts_token": ""},
    "upload_form_info": null,
    "internal_upload_form_info": null
  }, ...]
}
```

关键：`upload_url` 的 host 是 **`{store_id}.data.aliyunfile.com`**（与 API 域名不同），是 OSS 预签名直传 URL，**不需要带 Bearer token**。`store_id` 可从 `/v2/drive/get` 拿到（本域为 `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`）。

`semi-official` 字段补充（来自 OpenAPI 元数据，CREATE 响应）：`file_id, upload_id, part_info_list, rapid_upload, type, exist, status, file_name, parent_file_id, domain_id, drive_id`。

#### ② PUT 分片 —— **这里有个必踩的坑（已实测证明）**

| PUT 的 `Content-Type` | 结果 |
|---|---|
| 完全不带该 header | **200 OK** ✅ |
| `Content-Type: application/octet-stream` | **403 SignatureDoesNotMatch** ❌ |
| `Content-Type: ` 空值 | **403 SignatureDoesNotMatch** ❌ |

原因：`part_info_list[].content_type` 返回的是**空串**，而 OSS V1 签名把 `Content-Type` 纳入签名串。空 ⇒ 签名时按「无该 header」计算 ⇒ 真正发 PUT 时必须**完全不发** `Content-Type`。

官方 JS SDK 的做法印证这一点（`lib/loaders/StandardSerialUploader.js:202`）：

```js
'Content-Type': partInfo.content_type || '',   // 空串时 axios 不会发出该 header
```

其它要求：
- 只需 `Content-Length`（= 本分片字节数）+ body 原始字节
- **成功返回 200，响应头带 `ETag`**
- **不要对 upload_url 发 HEAD**：实测 `HEAD` → 403（签名绑定 GET/PUT 方法）
- 分片**串行**上传（默认模式）：「因服务端使用流式计算 SHA1 值，单个文件的分片需要串行上传，不支持多个分片并行上传」
- 分片**不允许覆盖**

#### ③ 完成上传

```json
POST /v2/file/complete
{"drive_id":"2","file_id":"6ad78f3c...","upload_id":"EAB342C484234571865158E6008F30DF"}
```

**响应 200（实测）**

```json
{"file_id":"6ad78f3c8c3d0e4b54bc42c7bbec485448cce1b7","name":"_probe3.txt","size":20,
 "status":"available","content_hash":"E362E81941CB4C9904338DF688E973557E3FA394",
 "crc64_hash":"11744108994224338685","type":"file","parent_file_id":"root"}
```

- 返回的 `content_hash` 与我本地算的 SHA1 **完全一致** → 服务端会校验
- 若 create 时传了 `content_hash` 而内容不匹配，complete 直接 400：
  `InvalidParameter.ContentHash: file content hash is not matched with content_hash in complete file request`
- `crc64_hash` 可选回传（`{"crc64_hash": "..."}`）

### 3.2 分片策略（谁定 part_size？）

**part_info_list 只需要你给 `part_number` 列表，`part_size` 由客户端自己定，服务端不校验你申请了几片**。实测传 `{"part_number":0}` 也能 201（无校验）。

| 约束 | 值 | 来源 |
|---|---|---|
| 单分片最小 | 100 KB | JS SDK 源码注释 |
| 单分片最大 | **5 GB** | 上传文档「文件上传限制」 |
| 分片数上限 | **10000**（`part_info_list` 最多 10000 项） | CreateFile 参数说明 |
| SDK 实际用 | `LIMIT_PART_NUM = 9000`（预留 buffer）、`INIT_CHUNK_SIZE = 10MB` | JS SDK `lib/utils/ChunkUtil.ts:3-4` |
| chunk 对齐 | `chunk_size` 向上取到 **64 的倍数**（`find_64x`） | 同上 |
| CLI 默认 | `--part-size 5242880`（5 MB） | PDS 官方 Skill `references/upload-file.md` |
| 理论最大单文件 | 5GB × 10000 ≈ **48 TB** | JS SDK 注释「理论支持最大单文件大小 48TB」 |

推荐算法（直接照抄 SDK `get_available_size`）：

```js
let chunk = 10 * 1024 * 1024                 // 10 MB 起步
chunk = chunk + (64 - (chunk % 64))          // 对齐 64
let num = Math.ceil(file_size / chunk)
if (num >= 9000) {
  chunk = Math.ceil(file_size / 9000)
  chunk = chunk + (64 - (chunk % 64))
  num = Math.ceil(file_size / chunk)
}
```

> 注意：10MB 分片在 100Mbps 上行下每片约 1 秒，请求开销偏高；**实践值 8~16 MB**，超过 100 GB 的文件按上式自动放大。

### 3.3 断点续传（两把武器，均已实测 200）

**(a) `POST /v2/file/list_uploaded_parts` — 问服务端「哪些片已经收好了」**

```json
{"drive_id":"2","file_id":"...","upload_id":"...","limit":100,"part_number_marker":0}
```

响应（实测）：

```json
{"file_id":"6ad78f3c...","upload_id":"EAB342C484234571865158E6008F30DF",
 "uploaded_parts":[{"part_number":1,"part_size":20,"upload_url":"","internal_upload_url":"",
                    "etag":"\"F25CE3F78F7291D329500F4F444C6863\"","content_type":"",
                    "upload_form_info":null,"internal_upload_form_info":null}],
 "next_part_number_marker":"","parallel_upload":false,"drive_id":"2","next_marker":""}
```

`limit ≤ 100`；分页用 `part_number_marker` ← `next_part_number_marker`。**有 `etag` 即该片已完成**。

**(b) `POST /v2/file/get_upload_url` — 换一批没过期的新 upload_url**

```json
{"drive_id":"2","file_id":"...","upload_id":"...",
 "part_info_list":[{"part_number":1},{"part_number":2}]}
```

响应 200（实测）：`{domain_id, drive_id, file_id, upload_id, create_at, part_info_list[]}`，每个 part 带新 `upload_url`。

**为什么需要它**：分片上传 URL **有效期 1 小时**，过期后 PUT 返回 403，此时调 `get_upload_url` 换新地址再传。官方 JS SDK 就是这套逻辑（`lib/loaders/BaseUploader.js:1064` `if (isOssUrlExpired(e)) await this.getUploadUrl()`）。

**会话时限**：从 create 到 complete 必须 **10 天内**完成，超期作废只能重新 create。
**半片作废**：某片传到一半暂停，该片作废，续传时**从该片起整片重传**。

**App 侧持久化清单**：`drive_id, parent_file_id, file_id, upload_id, size, name, part_size, 已完成 part_number 集合(+etag), content_hash`。恢复流程：
`create 拿到 file_id/upload_id` →（重启后）`list_uploaded_parts` 求已完成集合 → `get_upload_url` 补齐/刷新剩余片 URL → 串行 PUT 未完成片 → `complete`。

### 3.4 秒传 / 预秒传

**秒传（rapid upload）** — 已证实（文档 + CreateFile 参数）

```json
{"drive_id":"2","parent_file_id":"root","name":"x.jpg","type":"file","size":13381200,
 "content_hash":"<整文件 SHA1 大写 HEX>","content_hash_name":"sha1",
 "part_info_list":[{"part_number":1}]}
```
响应 `"rapid_upload": true` ⇒ 命中，**不返回 part_info_list，直接跳过 ②③**。`false` ⇒ 正常走 PUT + complete。
去重范围是 **Domain 级**（同 domain 内相同文件只存一份）。

**预秒传（pre_hash）** — 已证实

```json
{"drive_id":"2","parent_file_id":"root","name":"x.jpg","type":"file","size":13381200,
 "pre_hash":"<前 1KB 的 SHA1>","part_info_list":[{"part_number":1}]}
```
- 返回 **409** ⇒ 云端**可能**已存在 → 再去算整文件 SHA1 尝试秒传
- 返回 **201** ⇒ 云端**肯定**没有 → 直接走完整上传（响应里已带 part_info_list）
- 预秒传是后台异步算的，刚上传的文件分钟级内可能不命中

**关于 `proof_code` / `content_md5`**：
- `proof_code` **未找到权威来源** —— PDS 2022-03-01 的 CreateFile **没有** `proof_code` 字段（元数据 + 文档均无）。它是**阿里云盘消费版**（`api.alipan.com`，`阿里云盘开放平台`）的秒传概念。**PDS 侧的等价物是 `pre_hash`**。[推测] 如果你在别处看到 proof_code + pre_hash 组合，那是 alipan 协议，不能直接套到 aliyunfile。
- `content_md5` 是**分片级**字段（`part_info_list[].content_md5`），用于上传时校验该片 MD5，可选，本次未实测。

### 3.5 并行上传模式（进阶，未实测）

CreateFile 支持 `parallel_upload: true`，此时每个 part 需要带 `parallel_sha1_ctx`：

```json
{"part_number": 2,
 "parallel_sha1_ctx": {"h": [<前 5 个 32bit SHA1 状态字>], "part_offset": <该片之前的字节数，必须是 64 的倍数>},
 "content_md5": "...", "content_type": "..."}
```

JS SDK 对应 `StandardParallelUploader` + `init_chunks_parallel`。**除非你有明确的带宽瓶颈，否则不建议**：串行模式已经能跑满单连接带宽，且并行模式要自己维护 SHA1 中间态（`h[5]` + `part_offset`），实现复杂度高。

### 3.6 覆盖上传

create 时把 `file_id` 设为目标文件的 id 即可覆盖，其余流程一样。覆盖后 **file_id 不变**；并发覆盖以最后一次 complete 为准；要留旧版本需先开多版本（`ListRevision`）。

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/user-guide/upload-file>、<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/developer-reference/api-pds-2022-03-01-createfile>、<https://github.com/aliyun/aliyun-pds-js-sdk>

---

## 4) 下载

### 4.1 `POST /v2/file/get_download_url`（主路径）

**请求**

| 字段 | 必填 | 说明 |
|---|---|---|
| `drive_id` / `share_id` | 二选一 | 分享场景用 `share_id` + `x-share-token` |
| `file_id` | **是** | |
| `file_name` | 否 | ≤1024 字符，影响 `Content-Disposition` |
| `expire_sec` | 否 | 默认 **900**（15 分钟），最大 **115200**（32 小时） |
| `response_content_type` | 否 | 覆写响应 Content-Type |

**响应 200（实测）**

```json
{"domain_id":"bjxxxxx","drive_id":"2","file_id":"6ad78f3c...",
 "revision_id":"6ad78f3c3434955bc279473e915856bae0e77370",
 "method":"GET",
 "url":"https://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.data.aliyunfile.com/G1LU7wGn/2/...?di=...&op=d&x-oss-credential=...&x-oss-signature-version=OSS4-HMAC-SHA256",
 "expiration":"2026-10-20T16:11:58.167Z",
 "size":"20","crc64_hash":"11744108994224338685",
 "content_hash":"E362E81941CB4C9904338DF688E973557E3FA394","content_hash_name":"sha1",
 "punish_flag":"0","meta_name_punish_flag":"0","meta_name_investigation_status":"0"}
```

- 文档另有 `internal_url`（VPC 用）、`cdn_url`
- **`method` 字段实测返回 `"GET"`** —— 这就是「用什么 HTTP 方法下」
- `expire_sec > 115200` → 400 `InvalidParameter.ExpireSec: expire_sec should be less or equal than 115200`
- `expire_sec = 1` 也被接受（不校验下限）
- **注意类型不一致**：`size`/`crc64_hash` 在 get_download_url 里是**字符串**，在 File 对象里是 **number**

### 4.2 下载行为（全部实测）

| 测试 | 结果 |
|---|---|
| `GET <url>` 不带任何鉴权头 | **200**，内容正确（预签名 URL，无需 Bearer） |
| `GET` + `Range: bytes=0-4` | **206 Partial Content**，返回 5 字节 |
| `HEAD <url>` | **403** ← 别用 HEAD 探测大小，直接读元数据里的 `size` |

- **Range 断点续传完全可用**，且支持并发分段下载后合并
- 链接默认 15 分钟；**已经开始下载的传输即使超过有效期也不会被打断**（文档明确）
- 链接过期 → 重新调 `get_download_url`

### 4.3 稳定 URL（缓存友好）

File 对象的 `download_url` 字段（`fields:"*"` 时返回）是**另一个东西**，实测：

```
https://bjxxxxx.api.aliyunfile.com/v2/redirect?id=beaefb64533a46e7922e560d1cc2f71e1792512861075602345
```

对它 `GET` → **302** 跳到 OSS 签名地址（带 `response-content-disposition=attachment; filename*=UTF-8''<name>`、`x-oss-expires=900`）。**实测该 redirect 不校验 Authorization**。

优点：`id` 对同一文件稳定 → 适合做 CDN/图片组件缓存（`get_download_url` 每次返回的签名 URL 都不同，缓存命中不了）。
缺点：**302 必须由客户端跟随**；`id` 本身是承载权限的票据（900s），不要外泄。

> 文档还提到一个 `DownloadFile` 接口（302，query 带 `drive_id`+`file_id`）——**实测本域 `POST /v2/file/download` 返回 404**，即该路由在这个域名下没暴露；用上面 `/v2/redirect?id=` 即可。

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/user-guide/file-download-for-deverloper-edition>、<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/developer-reference/api-pds-2022-03-01-getdownloadurl>

---

## 5) 其他

### 5.1 配额查询

**`/v2/drive/get_quota` 不存在**（实测 404，`/v2/drive/get_used_size` 也 404）。正确做法是 `POST /v2/drive/get` 或 `POST /v2/drive/list`：

```json
POST /v2/drive/get {"drive_id":"2"}
```

实测响应（裁剪）：

```json
{"domain_id":"bjxxxxx","drive_id":"2","drive_name":"示例空间","description":"",
 "creator":"system","owner":"0123456789abcdef0123456789abcdef","owner_type":"group",
 "drive_type":"normal","status":"enabled",
 "used_size":2673873767,"total_size":429496729600,"store_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
 "relative_path":"","encrypt_mode":"none","encrypt_data_access":false,
 "created_at":"2026-09-07T13:48:23.390Z","updated_at":"2026-09-07T13:54:53.672Z",
 "permission":{"*":{...}}}
```

> `429496729600` = 400 GiB 整；`owner_type: "group"` 印证这是**团队/企业空间**（非个人空间）。`store_id` 就是上传域名前缀。
> `POST /v2/drive/list` 返回 `{items:[...], next_marker}`，可列全部空间。

### 5.2 限流 / QPS

**未找到权威来源。** 官方「使用限制」页只写了：每账号单 region 最多 3 个 domain + 支持地域列表，**没有任何 QPS/并发/带宽数字**。

证据：<https://help.aliyun.com/zh/pds/drive-and-photo-service-dev/limits>

**最可能的推断 [推测]**：
- 平台侧走阿里云 API 网关（`*.alicloudapi.com`），存在**按 API 的流控**（OpenAPI 元数据里每个 API 都有 `flowControl` 字段，只是当前为空 `{"flowControlList": []}`，即未按 API 单独配置）
- 实际瓶颈更可能是 **单分片 PUT 的 OSS 带宽**，而非 API QPS（一次上传只有 3 类 API 调用：create/complete + 每片 1 次 PUT）

**验证方法**（你可以直接跑）：
```bash
# 对 /v2/file/list 打 200 并发，看是否出现 429 / Throttling
seq 1 200 | xargs -P 50 -I{} curl -sS -o /dev/null -w "%{http_code}\n" \
  -X POST https://bjxxxxx.api.aliyunfile.com/v2/file/list \
  -H "Authorization: Bearer $TK" -H "Content-Type: application/json" \
  -d '{"drive_id":"2","parent_file_id":"root","limit":1}' | sort | uniq -c
```
判读：出现 `503`/`Throttling`/`RequestLimitExceeded` 即为限流；全是 200 则单机打不到墙。客户端仍应内置**指数退避**（初始 500ms，上限 30s，遇 5xx/429 重试）。

### 5.3 单文件大小上限

- **没有找到「单文件最大 N GB」的权威数字**。约束是间接的：单分片 ≤ 5 GB × 分片数 ≤ 10000 → **理论上限 ≈ 48 TB**
- 实际上限 = 空间配额（本域 400 GiB）。实测单文件 20 B 到分片级都正常
- 秒传/预秒传对大文件很关键：**先算前 1KB 的 SHA1 走 pre_hash，409 再算全量 SHA1**，可以省掉一次几十 GB 的无效 SHA1 计算

### 5.4 SDK

| 语言 | 仓库 | 说明 |
|---|---|---|
| JS/TS（推荐参考） | <https://github.com/aliyun/aliyun-pds-js-sdk> | npm `aliyun-pds-js-sdk` v1.4.0，浏览器 + Node，**分片/断点/秒传/并行全都有**，是读实现的最佳样本 |
| Java | <https://github.com/aliyun/alibabacloud-pds-sdk> | |
| Android | <https://github.com/aliyun/aliyun-pds-android-sdk> | Kotlin |
| C++ | <https://github.com/aliyun/aliyun-pds-cpp-sdk> | |
| Objective-C | <https://github.com/aliyun/aliyun-pds-objc-sdk> | |

**没有官方 Rust / Go / Python 专用 PDS SDK。**
- Go：用 `alibaba-cloud-sdk-go`，或直接吃 OpenAPI 元数据（`aliyun pds` CLI 插件就是 Go 写的、**由 OpenAPI 元数据自动生成**）
- Rust：**直接用 `reqwest` 手写**，这套 API 简单到不需要 SDK（create → PUT × N → complete 三个调用）
- `aliyun pds` 插件本体是 Go 二进制，通过 `https://aliyuncli.alicdn.com/plugins/pkgs/aliyun-cli-pds/...` 分发（`~/.aliyun/plugins/manifest.json` 可查），**没有公开源码仓库**

JS SDK 里最有参考价值的三段：
```
lib/utils/ChunkUtil.ts              # 分片算法（10MB 起步、9000 片上限、64 对齐）
lib/loaders/StandardSerialUploader.js  # 串行上传 + Content-Type 处理 + 403 过期换 URL
lib/loaders/BaseUploader.js         # checkpoint / getUploadUrl / listUploadedParts 恢复逻辑
```

### 5.5 关于「开源项目里用 api.aliyunfile.com 的真实代码」

**未找到权威/公开的示例。** 实话说清楚：
- `aliyunfile.com` 是企业版私有域名，开源项目里基本没人硬编码（都从 `domain_id` 拼）
- 网上大量「阿里云盘 SDK」(`sososdk/aliyunpan-flutter-sdk`、`turbidsoul/n8n-nodes-alipan`、`boxplayer` 等) 都是**阿里云盘消费版 / 开放平台**，域名是 `api.alipan.com` / `openapi.alipan.com`，**和 aliyunfile 不是同一套**（认证是 OAuth2 + `Bearer`，秒传用 `proof_code`，文件列表是 `/adrive/v1.0/...`）。**不要照抄**。
- 唯一官方「在 aliyunfile 域名上工作」的代码痕迹：
  - 官方 PDS Skill 里写到分享页域名默认 `<domain_id>.apps.aliyunfile.com`
    （本机 `~/.agents/skills/alibabacloud-pds-intelligent-workspace/references/share-link.md:100`）
  - `~/.aliyun/pds_config.json` 里 `endpoint: bjxxxxx.api.aliyunfile.com`
- **可行的落地方式**：用 `aliyun-pds-js-sdk`，把 `api_endpoint` 配成 `https://bjxxxxx.api.aliyunfile.com` 即可复用全套上传/断点逻辑（SDK 的 endpoint 是可配置的）。

---

## 6) 给你这个「跨平台互传 App」的落地要点

1. **认证**：短期用 `uk-` API Key 直连（已跑通），token 放系统钥匙串，别写进前端。多用户分发前**先验证企业版有没有 OAuth 应用注册**（1.4）。
2. **上传引擎**（照 3.1 实现，已验证可行）：
   - `part_size` 取 8~16 MB，按 `64 对齐`，超 9000 片自动放大
   - **PUT 时删掉 `Content-Type`**（最容易卡住的一步）
   - 每片成功记 `etag`；失败按 500ms→30s 指数退避
   - 403 且是 URL 过期 → `get_upload_url` 换 URL 重试（**不要重新 create**，那会浪费已传分片）
   - 重启恢复：`list_uploaded_parts` → 补传 → `complete`
   - 上传前先 `pre_hash`（前 1KB SHA1）探路，409 再算全量 SHA1 秒传
   - 全流程持久化 `file_id/upload_id/part_size/已完成片集合`
3. **下载引擎**：`get_download_url`（可选 900s~32h）→ 直接 `GET`，用 `Range` 做多线程分段；**别发 HEAD**；链接过期就重新取。
4. **P2P 互传**：两端各自持 API Key 直连，用**分享链接**（`/v2/share_link/create` 实测 201，返回 `share_id`/`share_url`/`share_pwd`/`share_policy`）做「投递」，接收端用 `x-share-token` 读。这样不需要让对方交出 API Key。
5. **错误判读口诀**：
   - `400 InvalidParameter.*` → 参数问题，报错文案会直接告诉你要什么
   - `403 SignatureDoesNotMatch` → PUT 的 header 不对（99% 是多了 Content-Type）
   - `403 Forbidden / CheckRouterAccessFailed` → **凭据权限**没覆盖该操作，不是接口不存在
   - `404` → 该路由在此域名下不存在（如 `/v2/drive/get_quota`）

---

## 附：本次实测残留（需你在网页端清理）

我的探测在 `drive_id=2` 根目录留下了：
- 目录 `_api_probe_doc`（内含 `_probe3_renamed2.txt`、`_copy_test.txt`）
- `zz.txt`（处于 `uploading` 状态；尝试改名返回 409）

**本 API Key 无删除权限**（`/v2/file/delete`、`/v2/recyclebin/trash` 均 403），所以只能请你在 `https://www.aliyunfile.com` 网页端删除，或给该 Key 补上删除权限后我再清。
（另外根目录原已有 `_互联探针_*`、`hulian-put-test.bin`、`hulian_selftest.txt`，其中 `_互联探针_20261020_235420` 重复出现两次，非我创建。）

我创建的分享链接 `U8GwtT8zHao` 已调用 `/v2/share_link/cancel` 取消（204）。
