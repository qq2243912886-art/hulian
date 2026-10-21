//! 配置持久化（JSON，位于系统 App 配置目录）与 aliyun CLI 配置导入。

use crate::error::{AppError, Result};
use crate::model::*;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const CONFIG_FILE: &str = "config.json";

pub fn config_dir(app: &AppHandle) -> PathBuf {
    // 便携/排障模式：用环境变量指定配置目录（也方便自动化测试）
    if let Ok(dir) = std::env::var("HULIAN_CONFIG_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join(CONFIG_FILE)
}

/// 从指定配置目录载入（无头模式/测试用）
pub fn load_from(dir: &Path, app: Option<&AppHandle>) -> AppConfig {
    let path = dir.join(CONFIG_FILE);
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
            return AppConfig {
                account: cfg.account,
                settings: fill_settings(app, cfg.settings),
            };
        }
        log::warn!("配置文件解析失败，使用默认配置: {}", path.display());
    }
    defaults_opt(app)
}

/// 写入指定配置目录
pub fn save_to(dir: &Path, cfg: &AppConfig) -> Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(CONFIG_FILE);
    let text = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn defaults_opt(app: Option<&AppHandle>) -> AppConfig {
    AppConfig {
        account: None,
        settings: default_settings_opt(app),
    }
}

pub fn default_settings_opt(app: Option<&AppHandle>) -> AppSettings {
    AppSettings {
        transfer_folder_name: "文件互传".into(),
        download_dir: default_download_dir_opt(app).to_string_lossy().to_string(),
        chunk_size_mb: if is_mobile() { 4 } else { 8 },
        // 阿里云盘 PDS 官方 SDK 为串行上传（服务端流式计算 SHA1），默认串行最稳
        parallel_parts: 1,
        parallel_transfers: 2,
        autostart: false,
        close_to_tray: true,
        device_name: default_device_name(),
        theme: "dark".into(),
    }
}

pub fn default_download_dir_opt(app: Option<&AppHandle>) -> PathBuf {
    #[cfg(desktop)]
    {
        if let Some(a) = app {
            if let Ok(d) = a.path().download_dir() {
                return d.join("互联");
            }
        }
        if let Some(d) = dirs::download_dir() {
            return d.join("互联");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join("Downloads").join("互联");
        }
    }
    if let Some(a) = app {
        if let Ok(d) = a.path().app_data_dir() {
            return d.join("Downloads");
        }
    }
    PathBuf::from("Downloads")
}

pub fn load(app: &AppHandle) -> AppConfig {
    load_from(&config_dir(app), Some(app))
}

fn fill_settings(app: Option<&AppHandle>, s: AppSettings) -> AppSettings {
    let d = default_settings_opt(app);
    AppSettings {
        transfer_folder_name: non_empty(s.transfer_folder_name).unwrap_or(d.transfer_folder_name),
        download_dir: non_empty(s.download_dir).unwrap_or(d.download_dir),
        chunk_size_mb: if (1..=64).contains(&s.chunk_size_mb) {
            s.chunk_size_mb
        } else {
            d.chunk_size_mb
        },
        parallel_parts: if (1..=4).contains(&s.parallel_parts) {
            s.parallel_parts
        } else {
            d.parallel_parts
        },
        parallel_transfers: if (1..=6).contains(&s.parallel_transfers) {
            s.parallel_transfers
        } else {
            d.parallel_transfers
        },
        autostart: s.autostart,
        close_to_tray: s.close_to_tray,
        device_name: non_empty(s.device_name).unwrap_or(d.device_name),
        theme: if s.theme.is_empty() { d.theme } else { s.theme },
    }
}

fn non_empty(s: String) -> Option<String> {
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

pub fn defaults(app: &AppHandle) -> AppConfig {
    defaults_opt(Some(app))
}

pub fn default_settings(app: &AppHandle) -> AppSettings {
    default_settings_opt(Some(app))
}

pub fn default_download_dir(app: &AppHandle) -> PathBuf {
    default_download_dir_opt(Some(app))
}

pub fn default_device_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let sys = match platform_label() {
        "macos" => "Mac",
        "windows" => "Windows",
        "android" => "Android",
        "ios" => "iPhone",
        _ => "本机",
    };
    if user.trim().is_empty() {
        sys.to_string()
    } else {
        format!("{sys} · {}", user.trim())
    }
}

pub fn is_mobile() -> bool {
    cfg!(any(target_os = "android", target_os = "ios"))
}

pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<()> {
    save_to(&config_dir(app), cfg)
}

/// 从本机 aliyun CLI 的 PDS 插件配置导入账号（仅桌面端）。
pub fn import_from_cli() -> Result<PdsAccount> {
    if is_mobile() {
        return Err(AppError::Unsupported("手机端暂不支持自动导入，请手动填写".into()));
    }
    let home = dirs::home_dir().ok_or_else(|| AppError::msg("找不到用户主目录"))?;
    let path = home.join(".aliyun").join("pds_config.json");
    let text = fs::read_to_string(&path).map_err(|_| {
        AppError::msg("没找到本机 aliyun CLI 的 PDS 配置（~/.aliyun/pds_config.json），请手动填写")
    })?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    let profiles = v
        .get("profiles")
        .and_then(|p| p.as_array())
        .ok_or_else(|| AppError::msg("aliyun 配置格式不识别：缺少 profiles"))?;
    let current = v.get("current").and_then(|c| c.as_str()).unwrap_or("default");
    let profile = profiles
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(current))
        .or_else(|| profiles.first())
        .ok_or_else(|| AppError::msg("aliyun 配置里没有任何 profile"))?;

    let endpoint = profile
        .get("endpoint")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let domain_id = profile
        .get("domain_id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    // CLI 的 api_key 认证会把 uk- 令牌写进 access_token 字段
    let token = profile
        .get("access_token")
        .and_then(|x| x.as_str())
        .filter(|s| s.starts_with("uk-"))
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::msg("aliyun 配置里没有可用的 API Key（uk-…）"))?;
    if domain_id.is_empty() {
        return Err(AppError::msg("aliyun 配置缺少 domain_id"));
    }
    Ok(PdsAccount {
        endpoint,
        domain_id,
        drive_id: String::new(),
        access_token: token,
        user_id: profile
            .get("user_id")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        user_name: profile
            .get("nick_name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        drive_name: None,
    })
}
