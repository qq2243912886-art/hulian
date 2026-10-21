//! Tauri 命令层：前端 `src/lib/api.ts` 里的每个 `invoke` 都对应这里一个函数。

use crate::config;
use crate::error::{AppError, Result};
use crate::model::*;
use crate::pds::PdsClient;
use crate::state::Shared;
use crate::transfer;
use crate::window;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

pub const EVT_TRAY_UPLOAD: &str = "app://tray-upload";

/// 下载命令的入参
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadArg {
    pub file_id: String,
    pub name: String,
    #[serde(default)]
    pub size: u64,
}

// ---------------------------------------------------------------- 配置

#[tauri::command]
pub fn get_config(state: State<'_, Arc<Shared>>) -> AppConfig {
    state.config.read().unwrap().clone()
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, Arc<Shared>>,
    config: AppConfig,
) -> Result<AppConfig> {
    let shared = state.inner().clone();
    shared.reconfigure(config.clone())?;
    if let Err(e) = apply_autostart(&app, config.settings.autostart) {
        log::warn!("同步开机自启失败: {e}");
    }
    Ok(config)
}

/// 从本机 aliyun CLI 配置一键导入（自动识别账号与空间）
#[tauri::command]
pub async fn import_local_cli_config() -> Result<ImportResult> {
    let mut account = match config::import_from_cli() {
        Ok(a) => a,
        Err(e) => {
            return Ok(ImportResult {
                ok: false,
                account: None,
                message: Some(e.to_string()),
            })
        }
    };
    let client = PdsClient::new(&account)?;
    let resolved = resolve_account(&client, &mut account).await;
    match resolved {
        Ok(()) => Ok(ImportResult {
            ok: true,
            account: Some(account),
            message: None,
        }),
        Err(e) => Ok(ImportResult {
            ok: false,
            account: None,
            message: Some(e.to_string()),
        }),
    }
}

/// 补全账号信息：用户昵称 + 默认空间
async fn resolve_account(client: &PdsClient, account: &mut PdsAccount) -> Result<()> {
    if let Ok((uid, uname)) = client.user_info().await {
        if account.user_id.is_none() {
            account.user_id = uid;
        }
        if account.user_name.is_none() {
            account.user_name = uname;
        }
    }
    if account.drive_id.trim().is_empty() {
        let drives = client.list_drives().await?;
        let drive = drives
            .into_iter()
            .next()
            .ok_or_else(|| AppError::msg("这个账号下还没有云盘空间"))?;
        account.drive_id = drive.drive_id;
        account.drive_name = Some(drive.drive_name);
    } else if let Ok(d) = client.get_drive().await {
        account.drive_name = Some(d.drive_name);
    }
    Ok(())
}

#[tauri::command]
pub async fn test_connection(
    state: State<'_, Arc<Shared>>,
    account: Option<PdsAccount>,
) -> Result<ConnectionStatus> {
    let shared = state.inner().clone();
    let mut account = match account {
        Some(a) if !a.access_token.trim().is_empty() => a,
        _ => match shared.account() {
            Some(a) => a,
            None => {
                return Ok(ConnectionStatus {
                    ok: false,
                    message: Some("还没有配置阿里云盘账号".into()),
                    user: None,
                    drive: None,
                    quota: None,
                })
            }
        },
    };
    let client = match PdsClient::new(&account) {
        Ok(c) => c,
        Err(e) => {
            return Ok(ConnectionStatus {
                ok: false,
                message: Some(e.to_string()),
                user: None,
                drive: None,
                quota: None,
            })
        }
    };
    match resolve_account(&client, &mut account).await {
        Ok(()) => {
            let drive = client.get_drive().await.ok();
            Ok(ConnectionStatus {
                ok: true,
                message: None,
                user: account.user_name.clone(),
                drive: account.drive_name.clone(),
                quota: drive.map(|d| DriveQuota {
                    drive_name: d.drive_name,
                    used_size: d.used_size,
                    total_size: d.total_size,
                }),
            })
        }
        Err(e) => Ok(ConnectionStatus {
            ok: false,
            message: Some(e.to_string()),
            user: None,
            drive: None,
            quota: None,
        }),
    }
}

#[tauri::command]
pub async fn get_drive_quota(state: State<'_, Arc<Shared>>) -> Result<DriveQuota> {
    let shared = state.inner().clone();
    let client = shared.client()?;
    let d = client.get_drive().await?;
    Ok(DriveQuota {
        drive_name: d.drive_name,
        used_size: d.used_size,
        total_size: d.total_size,
    })
}

// ---------------------------------------------------------------- 云端文件

#[tauri::command]
pub async fn list_remote(state: State<'_, Arc<Shared>>) -> Result<Vec<RemoteFile>> {
    let shared = state.inner().clone();
    let settings = shared.settings();
    let client = shared.client()?;
    let folder = client
        .ensure_transfer_folder(&settings.transfer_folder_name)
        .await?;
    let files = client.list_files(&folder.file_id).await?;
    let mut out: Vec<RemoteFile> = files.into_iter().filter(|f| !f.is_folder).collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

#[tauri::command]
pub async fn list_remote_folders(state: State<'_, Arc<Shared>>) -> Result<Vec<RemoteFile>> {
    let shared = state.inner().clone();
    let settings = shared.settings();
    let client = shared.client()?;
    let folder = client
        .ensure_transfer_folder(&settings.transfer_folder_name)
        .await?;
    let files = client.list_files(&folder.file_id).await?;
    Ok(files.into_iter().filter(|f| f.is_folder).collect())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_folder(
    state: State<'_, Arc<Shared>>,
    parent_file_id: String,
) -> Result<Vec<RemoteFile>> {
    let shared = state.inner().clone();
    let client = shared.client()?;
    let parent = if parent_file_id.trim().is_empty() {
        "root".to_string()
    } else {
        parent_file_id
    };
    client.list_files(&parent).await
}

// ---------------------------------------------------------------- 上传 / 下载

#[tauri::command]
pub fn upload_files(
    state: State<'_, Arc<Shared>>,
    paths: Vec<String>,
) -> Result<Vec<String>> {
    let shared = state.inner().clone();
    shared.client()?; // 未配置直接报错
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    Ok(shared.start_uploads(paths))
}

/// 打开系统文件选择器并上传（桌面端阻塞式对话框，放在阻塞线程池里跑）
#[tauri::command]
pub async fn pick_and_upload(app: AppHandle, state: State<'_, Arc<Shared>>) -> Result<Vec<String>> {
    let shared = state.inner().clone();
    let paths = pick_files(&app).await?;
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    Ok(shared.start_uploads(paths))
}

#[tauri::command]
pub fn download_files(
    state: State<'_, Arc<Shared>>,
    files: Vec<DownloadArg>,
) -> Result<Vec<String>> {
    let shared = state.inner().clone();
    shared.client()?;
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mapped = files
        .into_iter()
        .map(|f| (f.file_id, f.name, f.size))
        .collect();
    Ok(shared.start_downloads(mapped))
}

// ---------------------------------------------------------------- 传输任务

#[tauri::command]
pub fn list_transfers(state: State<'_, Arc<Shared>>) -> Vec<TransferItem> {
    state.snapshot()
}

#[tauri::command]
pub fn cancel_transfer(state: State<'_, Arc<Shared>>, id: String) -> bool {
    state.cancel_transfer(&id)
}

#[tauri::command]
pub fn retry_transfer(state: State<'_, Arc<Shared>>, id: String) -> Result<String> {
    let shared = state.inner().clone();
    shared.retry_transfer(&id)
}

#[tauri::command]
pub fn remove_transfer(state: State<'_, Arc<Shared>>, id: String) {
    state.remove_task(&id);
}

#[tauri::command]
pub fn clear_finished_transfers(state: State<'_, Arc<Shared>>) {
    state.clear_finished();
}

// ---------------------------------------------------------------- 本地文件 / 系统

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| AppError::msg(format!("打开失败：{e}")))
}

#[tauri::command]
pub fn reveal_in_folder(app: AppHandle, path: String) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| AppError::msg(format!("打开失败：{e}")))
}

#[tauri::command]
pub fn open_download_dir(app: AppHandle, state: State<'_, Arc<Shared>>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir: PathBuf = PathBuf::from(state.settings().download_dir);
    std::fs::create_dir_all(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::msg(format!("打开失败：{e}")))
}

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<Shared>>) -> AppInfo {
    let settings = state.settings();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: platform_label().to_string(),
        device_name: settings.device_name,
        download_dir: settings.download_dir,
        is_desktop: !config::is_mobile(),
    }
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<'_, Arc<Shared>>, enabled: bool) -> Result<()> {
    let shared = state.inner().clone();
    let mut cfg = shared.config.read().unwrap().clone();
    cfg.settings.autostart = enabled;
    shared.reconfigure(cfg)?;
    apply_autostart(&app, enabled)
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) {
    window::hide_main_window(&app);
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<()> {
    window::show_main_window(&app)
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, Arc<Shared>>) {
    let shared = state.inner().clone();
    window::quit_app(&app, &shared);
}

// ---------------------------------------------------------------- 辅助

pub fn sync_autostart(app: &AppHandle, enabled: bool) -> Result<()> {
    apply_autostart(app, enabled)
}

fn apply_autostart(_app: &AppHandle, _enabled: bool) -> Result<()> {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let manager = _app.autolaunch();
        let current = manager.is_enabled().unwrap_or(false);
        if _enabled && !current {
            manager
                .enable()
                .map_err(|e| AppError::msg(format!("开启开机自启失败：{e}")))?;
        } else if !_enabled && current {
            manager
                .disable()
                .map_err(|e| AppError::msg(format!("关闭开机自启失败：{e}")))?;
        }
    }
    Ok(())
}

/// 打开文件选择器，返回本地绝对路径列表（移动端返回 content URI 的 app 缓存副本）
pub async fn pick_files(app: &AppHandle) -> Result<Vec<String>> {
    #[cfg(desktop)]
    {
        use tauri_plugin_dialog::DialogExt;
        let app2 = app.clone();
        let picked = tauri::async_runtime::spawn_blocking(move || {
            app2.dialog()
                .file()
                .set_title("选择要上传到云盘的文件")
                .blocking_pick_files()
        })
        .await
        .map_err(|e| AppError::msg(format!("打开文件选择器失败：{e}")))?;
        let mut out = Vec::new();
        if let Some(files) = picked {
            for f in files {
                match f.into_path() {
                    Ok(p) => out.push(p.to_string_lossy().to_string()),
                    Err(_) => {
                        return Err(AppError::msg("这个文件的位置无法读取，请换一个文件"));
                    }
                }
            }
        }
        Ok(out)
    }
    #[cfg(mobile)]
    {
        pick_files_mobile(app).await
    }
}

/// Android / iOS：系统文件选择器返回 content:// URI，经 fs 插件读成真实文件后
/// 复制到 App 缓存目录，再交给上传引擎（缓存目录可被系统或用户清理）。
#[cfg(mobile)]
async fn pick_files_mobile(app: &AppHandle) -> Result<Vec<String>> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    use tauri_plugin_fs::FsExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选择要上传到云盘的文件")
        .pick_files(move |files| {
            let _ = tx.send(files);
        });
    let picked = rx
        .await
        .map_err(|_| AppError::msg("文件选择未完成"))?;
    let Some(files) = picked else {
        return Ok(Vec::new());
    };
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::msg(format!("无法定位缓存目录：{e}")))?
        .join("picked-files");
    std::fs::create_dir_all(&cache_dir)?;
    let mut out = Vec::new();
    for f in files {
        // 普通路径直接用
        if let Ok(p) = f.clone().into_path() {
            if p.exists() {
                out.push(p.to_string_lossy().to_string());
                continue;
            }
        }
        let name = match &f {
            FilePath::Path(p) => p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "picked".into()),
            FilePath::Url(u) => u
                .path_segments()
                .and_then(|mut s| s.next_back().map(|x| x.to_string()))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "picked".into()),
        };
        let dest = cache_dir.join(format!("{}-{}", now_millis(), sanitize_file_name(&name)));
        let mut opts = tauri_plugin_fs::OpenOptions::new();
        opts.read(true);
        let mut src = app
            .fs()
            .open(f.clone(), opts)
            .map_err(|e| AppError::msg(format!("读取所选文件失败：{e}")))?;
        let mut dst = std::fs::File::create(&dest)?;
        std::io::copy(&mut src, &mut dst)
            .map_err(|e| AppError::msg(format!("复制所选文件失败：{e}")))?;
        out.push(dest.to_string_lossy().to_string());
    }
    Ok(out)
}

/// 简单文件名净化（移动端缓存副本用）
#[cfg(mobile)]
fn sanitize_file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' || c == ':' { '_' } else { c })
        .collect();
    if s.trim().is_empty() {
        "file".into()
    } else {
        s
    }
}

/// 供托盘「快速上传」复用
pub async fn quick_upload(app: &AppHandle) {
    let paths = match pick_files(app).await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("托盘快速上传失败: {e}");
            return;
        }
    };
    if paths.is_empty() {
        return;
    }
    let shared = app.state::<Arc<Shared>>().inner().clone();
    if shared.client().is_err() {
        let _ = window::show_main_window(app);
        return;
    }
    let ids = shared.start_uploads(paths);
    if !ids.is_empty() {
        let _ = tauri::Emitter::emit(app, EVT_TRAY_UPLOAD, ());
        let _ = window::show_main_window(app);
    }
}

// 让 transfer 模块的辅助函数在未使用时不触发 dead_code 警告
#[allow(unused_imports)]
use transfer as _transfer;
