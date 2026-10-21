//! 阿里云盘（企业版 / PDS）HTTP 客户端。
//!
//! 契约要点（实测，见 docs/PDS-API契约.md）：
//! - 所有 API：`POST https://{domain}.api.aliyunfile.com/v2/...`，`Authorization: Bearer uk-…`
//! - 上传三步：`/v2/file/create` → `PUT upload_url` × N → `/v2/file/complete`
//! - `PUT upload_url` 时**绝不能带 Content-Type**（OSS 预签名把空 Content-Type 纳入签名）
//! - 分片 URL 约 1 小时过期 → 403 SignatureDoesNotMatch → 用 `/v2/file/get_upload_url` 换新
//! - 断点续传：`/v2/file/list_uploaded_parts` 查询已完成分片

use crate::error::{AppError, Result};
use crate::model::{platform_label, RemoteFile};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const API_TIMEOUT: Duration = Duration::from_secs(60);
const PART_TIMEOUT: Duration = Duration::from_secs(1800);
const LIST_PAGE_SIZE: usize = 200;
const LIST_MAX_PAGES: usize = 10;
/// 单文件分片数上限（PDS 官方 10000，留安全余量）
pub const MAX_PARTS: u32 = 9000;
/// 单片最小（OSS multipart 规则）
pub const MIN_CHUNK: u64 = 100 * 1024;
/// 单片最大
pub const MAX_CHUNK: u64 = 5 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct PdsClient {
    http: Client,
    base: String,
    token: String,
    pub drive_id: String,
    pub domain_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PartInfo {
    #[serde(default)]
    pub part_number: u32,
    #[serde(default)]
    pub upload_url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreateUpload {
    #[serde(default)]
    pub file_id: String,
    #[serde(default)]
    pub upload_id: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub part_info_list: Vec<PartInfo>,
    #[serde(default)]
    pub rapid_upload: bool,
    #[serde(default)]
    pub exist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub drive_id: String,
    pub drive_name: String,
    pub used_size: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    #[serde(default)]
    pub size: Option<u64>,
}

impl PdsClient {
    pub fn new(account: &crate::model::PdsAccount) -> Result<Self> {
        let base = normalize_endpoint(&account.endpoint, &account.domain_id)?;
        let token = account.access_token.trim().to_string();
        if token.is_empty() {
            return Err(AppError::msg("API Key 为空，请先到设置里填写"));
        }
        let ua = format!(
            "hulian/{} ({})",
            env!("CARGO_PKG_VERSION"),
            platform_label()
        );
        let http = Client::builder()
            .user_agent(ua)
            .connect_timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;
        Ok(Self {
            http,
            base,
            token,
            drive_id: account.drive_id.trim().to_string(),
            domain_id: account.domain_id.trim().to_string(),
        })
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    // ---------------------------------------------------------------- 通用请求

    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(&body)
            .timeout(API_TIMEOUT)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(parse_api_error(status.as_u16(), &text));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text)
            .map_err(|e| AppError::msg(format!("解析云盘响应失败（{e}）：{}", truncate(&text, 240))))
    }

    // ---------------------------------------------------------------- 账号 / 空间

    pub async fn get_user(&self) -> Result<Value> {
        self.post("/v2/user/get", json!({})).await
    }

    pub async fn user_info(&self) -> Result<(Option<String>, Option<String>)> {
        let v = self.get_user().await?;
        let id = v
            .get("user_id")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let name = v
            .get("nick_name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        Ok((id, name))
    }

    pub async fn get_drive(&self) -> Result<DriveInfo> {
        let v = self
            .post("/v2/drive/get", json!({ "drive_id": self.drive_id }))
            .await?;
        Ok(parse_drive(&v))
    }

    pub async fn list_drives(&self) -> Result<Vec<DriveInfo>> {
        let v = self.post("/v2/drive/list", json!({ "limit": 100 })).await?;
        let mut out = Vec::new();
        if let Some(items) = v.get("items").and_then(|i| i.as_array()) {
            for it in items {
                out.push(parse_drive(it));
            }
        }
        Ok(out)
    }

    // ---------------------------------------------------------------- 文件列表

    /// 列出某个目录下的全部条目（自动翻页，最多 2000 条）
    pub async fn list_files(&self, parent_file_id: &str) -> Result<Vec<RemoteFile>> {
        let mut out = Vec::new();
        let mut marker = String::new();
        for _ in 0..LIST_MAX_PAGES {
            let mut body = json!({
                "drive_id": self.drive_id,
                "parent_file_id": parent_file_id,
                "limit": LIST_PAGE_SIZE,
                "order_by": "created_at",
                "order_direction": "DESC",
            });
            if !marker.is_empty() {
                body["marker"] = json!(marker);
            }
            let v = self.post("/v2/file/list", body).await?;
            if let Some(items) = v.get("items").and_then(|i| i.as_array()) {
                for it in items {
                    out.push(parse_file(it));
                }
            }
            marker = v
                .get("next_marker")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if marker.is_empty() || out.len() >= 2000 {
                break;
            }
        }
        Ok(out)
    }

    pub async fn find_child(
        &self,
        parent_file_id: &str,
        name: &str,
        folder_only: bool,
    ) -> Result<Option<RemoteFile>> {
        let files = self.list_files(parent_file_id).await?;
        Ok(files
            .into_iter()
            .find(|f| f.name == name && (!folder_only || f.is_folder)))
    }

    pub async fn create_folder(&self, parent_file_id: &str, name: &str) -> Result<RemoteFile> {
        let v = self
            .post(
                "/v2/file/create",
                json!({
                    "drive_id": self.drive_id,
                    "parent_file_id": parent_file_id,
                    "name": name,
                    "type": "folder",
                    "check_name_mode": "refuse",
                }),
            )
            .await?;
        let file_id = v
            .get("file_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if file_id.is_empty() {
            return Err(AppError::msg("创建云端目录失败：响应里没有 file_id"));
        }
        Ok(RemoteFile {
            file_id,
            name: v
                .get("file_name")
                .or_else(|| v.get("name"))
                .and_then(|x| x.as_str())
                .unwrap_or(name)
                .to_string(),
            size: 0,
            is_folder: true,
            created_at: v
                .get("created_at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            updated_at: v
                .get("updated_at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            parent_file_id: parent_file_id.to_string(),
        })
    }

    /// 确保根目录下存在互传目录，返回其文件信息
    pub async fn ensure_transfer_folder(&self, name: &str) -> Result<RemoteFile> {
        let name = {
            let n = name.trim();
            if n.is_empty() {
                "文件互传"
            } else {
                n
            }
        };
        if self.drive_id.is_empty() {
            return Err(AppError::msg("还没有选择云盘空间"));
        }
        let files = self.list_files("root").await?;
        if let Some(f) = files.iter().find(|f| f.is_folder && f.name == name) {
            return Ok(f.clone());
        }
        match self.create_folder("root", name).await {
            Ok(f) => Ok(f),
            Err(e) => {
                // 并发/已存在：重新找一次
                let files = self.list_files("root").await?;
                if let Some(f) = files.iter().find(|f| f.is_folder && f.name == name) {
                    Ok(f.clone())
                } else {
                    Err(e)
                }
            }
        }
    }

    // ---------------------------------------------------------------- 上传

    pub async fn create_upload(
        &self,
        parent_file_id: &str,
        name: &str,
        size: u64,
        part_count: u32,
    ) -> Result<CreateUpload> {
        let mut body = json!({
            "drive_id": self.drive_id,
            "parent_file_id": parent_file_id,
            "name": name,
            "type": "file",
            "size": size,
            "check_name_mode": "auto_rename",
        });
        if size > 0 && part_count > 0 {
            let parts: Vec<Value> = (1..=part_count).map(|n| json!({"part_number": n})).collect();
            body["part_info_list"] = Value::Array(parts);
        }
        let v = self.post("/v2/file/create", body).await?;
        let cu: CreateUpload = serde_json::from_value(v)
            .map_err(|e| AppError::msg(format!("解析上传会话失败：{e}")))?;
        if !cu.rapid_upload && cu.file_id.is_empty() {
            return Err(AppError::msg("创建上传会话失败：响应里没有 file_id"));
        }
        Ok(cu)
    }

    /// 上传单个分片。**不要设置 Content-Type 头**，否则 OSS 签名校验失败。
    pub async fn put_part(&self, upload_url: &str, data: Vec<u8>) -> Result<()> {
        let resp = self
            .http
            .put(upload_url)
            .body(data)
            .timeout(PART_TIMEOUT)
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp.text().await.unwrap_or_default();
        Err(parse_oss_error(status.as_u16(), &text))
    }

    /// 为若干分片换取新的上传地址（URL 过期/续传时使用）
    pub async fn get_upload_urls(
        &self,
        file_id: &str,
        upload_id: &str,
        parts: &[u32],
    ) -> Result<HashMap<u32, String>> {
        let mut out = HashMap::new();
        for batch in parts.chunks(500) {
            let list: Vec<Value> = batch
                .iter()
                .map(|n| json!({ "part_number": n }))
                .collect();
            let v = self
                .post(
                    "/v2/file/get_upload_url",
                    json!({
                        "drive_id": self.drive_id,
                        "file_id": file_id,
                        "upload_id": upload_id,
                        "part_info_list": list,
                    }),
                )
                .await?;
            if let Some(items) = v.get("part_info_list").and_then(|i| i.as_array()) {
                for it in items {
                    let n = it.get("part_number").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                    let url = it
                        .get("upload_url")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    if n > 0 && !url.is_empty() {
                        out.insert(n, url);
                    }
                }
            }
        }
        Ok(out)
    }

    /// 查询已经上传完成的分片编号（断点续传）
    pub async fn list_uploaded_parts(
        &self,
        file_id: &str,
        upload_id: &str,
    ) -> Result<Vec<u32>> {
        let mut out = Vec::new();
        let mut marker = String::new();
        for _ in 0..100 {
            let mut body = json!({
                "drive_id": self.drive_id,
                "file_id": file_id,
                "upload_id": upload_id,
                "limit": 100,
            });
            if !marker.is_empty() {
                body["part_number_marker"] = json!(marker);
            }
            let v = self.post("/v2/file/list_uploaded_parts", body).await?;
            if let Some(items) = v.get("uploaded_parts").and_then(|i| i.as_array()) {
                for it in items {
                    if let Some(n) = it.get("part_number").and_then(|x| x.as_u64()) {
                        out.push(n as u32);
                    }
                }
            }
            marker = v
                .get("next_part_number_marker")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if marker.is_empty() {
                break;
            }
        }
        Ok(out)
    }

    pub async fn complete_upload(&self, file_id: &str, upload_id: &str) -> Result<Value> {
        self.post(
            "/v2/file/complete",
            json!({
                "drive_id": self.drive_id,
                "file_id": file_id,
                "upload_id": upload_id,
            }),
        )
        .await
    }

    /// 取单个文件元信息（下载命令里补全大小用）
    pub async fn get_file(&self, file_id: &str) -> Result<RemoteFile> {
        let v = self
            .post(
                "/v2/file/get",
                json!({ "drive_id": self.drive_id, "file_id": file_id }),
            )
            .await?;
        Ok(parse_file(&v))
    }

    // ---------------------------------------------------------------- 下载

    pub async fn get_download_url(&self, file_id: &str) -> Result<DownloadInfo> {
        let v = self
            .post(
                "/v2/file/get_download_url",
                json!({
                    "drive_id": self.drive_id,
                    "file_id": file_id,
                    "expire_sec": 3600,
                }),
            )
            .await?;
        let url = v
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            return Err(AppError::msg("获取下载地址失败"));
        }
        let size = match v.get("size") {
            Some(Value::String(s)) => s.parse::<u64>().ok(),
            Some(Value::Number(n)) => n.as_u64(),
            _ => None,
        };
        Ok(DownloadInfo { url, size })
    }
}

// ---------------------------------------------------------------- 工具函数

fn parse_drive(v: &Value) -> DriveInfo {
    let get_u64 = |key: &str| -> u64 {
        match v.get(key) {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        }
    };
    DriveInfo {
        drive_id: v
            .get("drive_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        drive_name: v
            .get("drive_name")
            .and_then(|x| x.as_str())
            .unwrap_or("云盘空间")
            .to_string(),
        used_size: get_u64("used_size"),
        total_size: get_u64("total_size"),
    }
}

fn parse_file(v: &Value) -> RemoteFile {
    let kind = v
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("file");
    let size = match v.get("size") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    };
    RemoteFile {
        file_id: v
            .get("file_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        size,
        is_folder: kind == "folder",
        created_at: v
            .get("created_at")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: v
            .get("updated_at")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        parent_file_id: v
            .get("parent_file_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

fn parse_api_error(status: u16, text: &str) -> AppError {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        let code = v
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let message = v
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        return AppError::Api {
            status,
            code,
            message,
        };
    }
    AppError::Api {
        status,
        code: "HttpError".into(),
        message: truncate(text, 300),
    }
}

fn parse_oss_error(status: u16, text: &str) -> AppError {
    let code = extract_tag(text, "Code").unwrap_or_else(|| "OssError".to_string());
    let message = extract_tag(text, "Message").unwrap_or_else(|| truncate(text, 200));
    AppError::Api {
        status,
        code,
        message,
    }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

/// 把用户填写的 endpoint 规范化成完整 host。
/// 支持：完整 URL、`xxx.api.aliyunfile.com`、纯域名 ID（bjxxxxx）。
pub fn normalize_endpoint(endpoint: &str, domain_id: &str) -> Result<String> {
    let e = endpoint.trim().trim_end_matches('/');
    let d = domain_id.trim();
    if e.is_empty() {
        if d.is_empty() {
            return Err(AppError::msg("请填写 endpoint 或 domain_id"));
        }
        return Ok(format!("https://{d}.api.aliyunfile.com"));
    }
    if e.starts_with("http://") || e.starts_with("https://") {
        return Ok(e.to_string());
    }
    if e.contains('.') {
        return Ok(format!("https://{e}"));
    }
    Ok(format!("https://{e}.api.aliyunfile.com"))
}

/// 计算分片大小：在用户设置基础上保证分片数不超上限
pub fn calc_chunk(size: u64, chunk_mb: u32) -> u64 {
    let mut chunk = (chunk_mb.max(1) as u64) * 1024 * 1024;
    if size == 0 {
        return chunk;
    }
    let max_parts = MAX_PARTS as u64;
    if size.div_ceil(chunk) > max_parts {
        chunk = size.div_ceil(max_parts);
    }
    chunk.clamp(MIN_CHUNK, MAX_CHUNK)
}

pub fn calc_part_count(size: u64, chunk: u64) -> u32 {
    if size == 0 {
        0
    } else {
        size.div_ceil(chunk).max(1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalization() {
        assert_eq!(
            normalize_endpoint("", "bjxxxxx").unwrap(),
            "https://bjxxxxx.api.aliyunfile.com"
        );
        assert_eq!(
            normalize_endpoint("bjxxxxx", "").unwrap(),
            "https://bjxxxxx.api.aliyunfile.com"
        );
        assert_eq!(
            normalize_endpoint("bjxxxxx.api.aliyunfile.com", "").unwrap(),
            "https://bjxxxxx.api.aliyunfile.com"
        );
        assert_eq!(
            normalize_endpoint("https://a.example.com/", "").unwrap(),
            "https://a.example.com"
        );
    }

    #[test]
    fn chunk_and_parts() {
        assert_eq!(calc_part_count(0, calc_chunk(0, 8)), 0);
        assert_eq!(calc_part_count(1024, calc_chunk(1024, 8)), 1);
        let size = 10 * 1024 * 1024 + 123;
        assert_eq!(calc_part_count(size, calc_chunk(size, 8)), 2);
        // 超过分片上限时自动放大分片
        let huge = 40u64 * 1024 * 1024 * 1024; // 40GB
        let chunk = calc_chunk(huge, 8);
        assert!(calc_part_count(huge, chunk) <= MAX_PARTS);
        assert!(chunk >= MIN_CHUNK);
    }

    /// 真机联调用例：`cargo test --lib -- --ignored live_`（读取本机 ~/.aliyun/pds_config.json）
    #[tokio::test]
    #[ignore]
    async fn live_multipart_roundtrip() {
        let mut acct = crate::config::import_from_cli().expect("需要本机 aliyun CLI 的 PDS 配置");
        let probe = PdsClient::new(&acct).expect("创建客户端失败");
        if acct.drive_id.trim().is_empty() {
            let drives = probe.list_drives().await.expect("列出空间失败");
            acct.drive_id = drives.first().expect("没有可用空间").drive_id.clone();
        }
        let client = PdsClient::new(&acct).unwrap();
        let folder = client
            .ensure_transfer_folder("文件互传")
            .await
            .expect("确保互传目录失败");
        let size: usize = 5 * 1024 * 1024 + 123;
        let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let chunk = calc_chunk(size as u64, 4);
        let parts = calc_part_count(size as u64, chunk);
        assert!(parts >= 2, "该用例需要至少两个分片");
        let name = format!(
            "_selftest_rust_{}.bin",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let cu = client
            .create_upload(&folder.file_id, &name, size as u64, parts)
            .await
            .expect("创建上传会话失败");
        assert!(!cu.upload_id.is_empty());
        let urls: std::collections::HashMap<u32, String> = cu
            .part_info_list
            .iter()
            .map(|p| (p.part_number, p.upload_url.clone()))
            .collect();
        for n in 1..=parts {
            let start = ((n as u64 - 1) * chunk) as usize;
            let end = std::cmp::min(size, (n as u64 * chunk) as usize);
            client
                .put_part(&urls[&n], payload[start..end].to_vec())
                .await
                .unwrap_or_else(|e| panic!("第 {n} 片上传失败: {e}"));
        }
        let done = client
            .complete_upload(&cu.file_id, &cu.upload_id)
            .await
            .expect("complete 失败");
        let got = done.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(got, size as u64, "云端大小不一致");
        let dl = client
            .get_download_url(&cu.file_id)
            .await
            .expect("获取下载地址失败");
        let bytes = client
            .http()
            .get(&dl.url)
            .send()
            .await
            .expect("下载失败")
            .bytes()
            .await
            .expect("读取下载内容失败");
        assert_eq!(bytes.len(), size, "下载长度不一致");
        assert_eq!(&bytes[..], &payload[..], "下载内容不一致");
        println!("✅ Rust 客户端上传+下载校验通过：{size} 字节 / {parts} 分片 / 文件 {name}");
    }

    #[test]
    fn oss_error_parsing() {
        let xml = "<Error><Code>SignatureDoesNotMatch</Code><Message>bad sig</Message></Error>";
        let e = parse_oss_error(403, xml);
        match e {
            AppError::Api { code, .. } => assert_eq!(code, "SignatureDoesNotMatch"),
            _ => panic!(),
        }
    }
}
