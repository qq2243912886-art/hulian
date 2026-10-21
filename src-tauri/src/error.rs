//! 统一错误类型：前端拿到的是可读字符串（Tauri 要求 Serialize）。

use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    /// 普通业务错误（直接展示给用户）
    Message(String),
    /// 尚未配置云盘账号
    NotConfigured,
    /// 当前平台不支持
    Unsupported(String),
    /// 用户取消
    Cancelled,
    /// 云盘 API 返回的业务错误
    Api {
        status: u16,
        code: String,
        message: String,
    },
    Io(std::io::Error),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Tauri(tauri::Error),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        AppError::Message(s.into())
    }

    /// 是否是"上传 URL 过期/签名失效"类错误（需要重新换取 URL）
    pub fn is_upload_url_expired(&self) -> bool {
        match self {
            AppError::Api { status, code, .. } => {
                *status == 403 && (code.contains("SignatureDoesNotMatch") || code.contains("AccessDenied"))
            }
            _ => false,
        }
    }

    /// 网络类临时错误（值得重试）
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Http(_) | AppError::Io(_) => true,
            AppError::Api { status, .. } => *status == 429 || *status >= 500,
            _ => false,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Message(m) => write!(f, "{m}"),
            AppError::NotConfigured => write!(f, "还没有配置阿里云盘账号"),
            AppError::Unsupported(m) => write!(f, "当前平台不支持：{m}"),
            AppError::Cancelled => write!(f, "已取消"),
            AppError::Api {
                status,
                code,
                message,
            } => {
                let friendly = friendly_api_message(*status, code, message);
                write!(f, "{friendly}")
            }
            AppError::Io(e) => write!(f, "本地文件操作失败：{e}"),
            AppError::Http(e) => {
                if e.is_timeout() {
                    write!(f, "网络超时，请检查网络后重试")
                } else if e.is_connect() {
                    write!(f, "无法连接云盘服务，请检查网络")
                } else {
                    write!(f, "网络错误：{e}")
                }
            }
            AppError::Json(e) => write!(f, "数据解析失败：{e}"),
            AppError::Tauri(e) => write!(f, "{e}"),
        }
    }
}

fn friendly_api_message(status: u16, code: &str, message: &str) -> String {
    if code.contains("CheckRouterAccessFailed") || (status == 403 && code == "Forbidden") {
        return "当前 API Key 没有该操作的权限（可在网盘控制台的 API Key 设置里调整访问权限）".into();
    }
    if code.contains("SignatureDoesNotMatch") {
        return "上传地址已过期，正在重新获取".into();
    }
    if code.contains("NotFound.UploadId") {
        return "上传会话已失效，需要重新开始".into();
    }
    if status == 401 {
        return "API Key 无效或已过期，请到设置里更新".into();
    }
    if status == 429 {
        return "请求太频繁，请稍后重试".into();
    }
    if message.is_empty() {
        format!("云盘返回错误（HTTP {status}，{code}）")
    } else {
        format!("{message}（{code}）")
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}
impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e)
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}
impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Tauri(e)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
