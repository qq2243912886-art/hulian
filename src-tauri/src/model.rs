//! 数据模型：字段与 `src/lib/types.ts` 一一对应（serde camelCase）。

use serde::{Deserialize, Serialize};

/// 云盘凭据（阿里云盘企业版 / PDS API Key）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdsAccount {
    /// 形如 bjxxxxx.api.aliyunfile.com（也可只填 bjxxxxx）
    pub endpoint: String,
    /// 形如 bjxxxxx
    pub domain_id: String,
    /// 形如 2
    pub drive_id: String,
    /// 形如 uk-xxxxxxxx
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// 云端互传目录名（根目录下自动创建）
    pub transfer_folder_name: String,
    /// 本地下载目录
    pub download_dir: String,
    /// 上传分片大小（MB）
    pub chunk_size_mb: u32,
    /// 单个文件的分片并发数（1 = 串行，推荐）
    pub parallel_parts: u32,
    /// 同时进行的传输任务数
    pub parallel_transfers: u32,
    /// 开机自启（桌面端）
    pub autostart: bool,
    /// 关闭窗口时驻留托盘；false = 退出应用
    pub close_to_tray: bool,
    /// 本机设备名
    pub device_name: String,
    /// dark | light | system
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub account: Option<PdsAccount>,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    pub file_id: String,
    pub name: String,
    pub size: u64,
    pub is_folder: bool,
    pub created_at: String,
    pub updated_at: String,
    pub parent_file_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveQuota {
    pub drive_name: String,
    pub used_size: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferItem {
    pub id: String,
    /// upload | download
    pub kind: String,
    pub name: String,
    pub local_path: String,
    pub size: u64,
    pub transferred: u64,
    /// queued | running | success | error | canceled
    pub state: String,
    pub speed_bps: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_file_id: Option<String>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<DriveQuota>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<PdsAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub device_name: String,
    pub download_dir: String,
    pub is_desktop: bool,
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn platform_label() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "android")]
    {
        "android"
    }
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
}
