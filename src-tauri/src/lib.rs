//! 「互联」主入口：插件装配、状态托管、托盘、窗口生命周期与命令注册。

pub mod commands;
pub mod config;
pub mod error;
pub mod menu;
pub mod model;
pub mod pds;
pub mod state;
pub mod tray;
pub mod transfer;
pub mod window;

use state::Shared;
use std::sync::Arc;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    // 桌面端专属插件：单实例 + 开机自启
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = window::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ));

    let app = builder
        .setup(|app| {
            let handle = app.handle().clone();
            let shared = Shared::new(handle.clone());
            app.manage(shared.clone());

            // 诊断/自动化入口（供联调、CI、用户排障使用）
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == "--import-cli") {
                handle_cli_import(&handle, &shared);
                return Ok(());
            }

            #[cfg(desktop)]
            {
                // macOS 自定义菜单（Cmd+Q 退出 / Cmd+W 关闭到托盘）
                if let Err(e) = menu::create(&handle) {
                    log::error!("创建应用菜单失败: {e}");
                }
                // 托盘常驻
                if let Err(e) = tray::create(&handle) {
                    log::error!("创建托盘失败: {e}");
                }
                // 同步开机自启状态
                if shared.settings().autostart {
                    let _ = commands::sync_autostart(&handle, true);
                }
                // 开机自启（--hidden）或用户要求时：不显示主窗口，只驻留托盘
                let hidden = std::env::args().any(|a| a == "--hidden");
                if hidden && shared.settings().close_to_tray {
                    // 启动即驻留托盘：桌面端主窗口在平台配置里设为 create=false，
                    // 因此这里什么都不做 —— 进程里根本不会实例化 WebView，空闲占用最低。
                    eprintln!("[hulian] --hidden：只驻留托盘，不创建窗口");
                } else {
                    window::show_main_window(&handle)?;
                }
            }

            if args.iter().any(|a| a == "--selftest") {
                handle_selftest(&handle, shared.clone());
            }

            // 诊断：直接销毁窗口（不经过 close 事件），用于对比验证
            if let Some(idx) = args.iter().position(|a| a == "--destroy-after") {
                if let Some(secs) = args.get(idx + 1).and_then(|v| v.parse::<u64>().ok()) {
                    let h = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                        eprintln!("[hulian] --destroy-after: 直接销毁窗口");
                        window::destroy_main_window(&h);
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        eprintln!("[hulian] --destroy-after: 当前窗口数 = {}", h.webview_windows().len());
                    });
                }
            }

            // 诊断：启动 N 秒后自动关闭主窗口（验证「关闭到托盘」的资源释放）
            if let Some(idx) = args.iter().position(|a| a == "--close-after") {
                if let Some(secs) = args.get(idx + 1).and_then(|v| v.parse::<u64>().ok()) {
                    let h = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                        eprintln!("[hulian] --close-after: 触发关闭窗口（走 CloseRequested 路径）");
                        if let Some(win) = h.get_webview_window(window::MAIN_LABEL) {
                            let _ = win.close();
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::import_local_cli_config,
            commands::test_connection,
            commands::get_drive_quota,
            commands::list_remote,
            commands::list_remote_folders,
            commands::list_folder,
            commands::upload_files,
            commands::pick_and_upload,
            commands::download_files,
            commands::list_transfers,
            commands::cancel_transfer,
            commands::retry_transfer,
            commands::remove_transfer,
            commands::clear_finished_transfers,
            commands::open_path,
            commands::reveal_in_folder,
            commands::open_download_dir,
            commands::get_app_info,
            commands::set_autostart,
            commands::hide_main_window,
            commands::show_main_window,
            commands::quit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle().clone();
                let close_to_tray = app
                    .try_state::<Arc<Shared>>()
                    .map(|s| s.settings().close_to_tray && !s.is_quitting())
                    .unwrap_or(false);
                if close_to_tray {
                    // 关闭 = 销毁窗口（释放 WebView），应用继续驻留托盘
                    api.prevent_close();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                        window::destroy_main_window(&app);
                    });
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build tauri application");

    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { code, api, .. } => {
            // 关闭窗口 / Cmd+Q 时默认不退出，保持托盘常驻；显式退出（code=Some）才放行
            let keep = app_handle
                .try_state::<Arc<Shared>>()
                .map(|shared| {
                    // 只有「关闭到托盘」模式下才阻止退出；关窗即退出模式允许退出
                    code.is_none() && !shared.is_quitting() && shared.settings().close_to_tray
                })
                .unwrap_or(false);
            if keep {
                api.prevent_exit();
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            let _ = window::show_main_window(app_handle);
        }
        _ => {}
    });
}


/// `--import-cli`：把本机 aliyun CLI 的 PDS 配置导入 App 并退出（命令行排障用）
fn handle_cli_import(app: &tauri::AppHandle, shared: &std::sync::Arc<Shared>) {
    use tauri::Manager;
    let result = (|| -> crate::error::Result<()> {
        let mut account = crate::config::import_from_cli()?;
        let async_info = tauri::async_runtime::block_on(async {
            let client = crate::pds::PdsClient::new(&account)?;
            let user = client.user_info().await.ok();
            let drive = if account.drive_id.trim().is_empty() {
                match client.list_drives().await {
                    Ok(drives) => drives.into_iter().next(),
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            Ok::<_, crate::error::AppError>((user, drive))
        })?;
        let (user, drive) = async_info;
        if account.user_id.is_none() || account.user_name.is_none() {
            if let Some((uid, uname)) = user {
                account.user_id = account.user_id.or(uid);
                account.user_name = account.user_name.or(uname);
            }
        }
        if account.drive_id.trim().is_empty() {
            let d = drive
                .ok_or_else(|| crate::error::AppError::msg("该账号下没有云盘空间"))?;
            account.drive_id = d.drive_id;
            account.drive_name = Some(d.drive_name);
        }
        let mut cfg = shared.config.read().unwrap().clone();
        cfg.account = Some(account);
        shared.reconfigure(cfg)?;
        Ok(())
    })();
    match result {
        Ok(()) => log::info!("[hulian] 已从 aliyun CLI 导入云盘配置"),
        Err(e) => log::error!("[hulian] 导入失败: {e}"),
    }
    let _ = app.try_state::<std::sync::Arc<Shared>>();
    app.exit(0);
}

/// `--selftest`：真实上传→下载校验（在真实 App 进程里跑，验证完整链路）
fn handle_selftest(app: &tauri::AppHandle, shared: std::sync::Arc<Shared>) {
    let app = app.clone();
    // 自检只写临时目录，不改用户配置
    {
        let mut cfg = shared.config.write().unwrap();
        cfg.settings.download_dir = std::env::temp_dir()
            .join("hulian-selftest-downloads")
            .to_string_lossy()
            .to_string();
    }
    tauri::async_runtime::spawn(async move {
        let payload: Vec<u8> = (0..1024 * 1024 + 31).map(|i| (i % 251) as u8).collect();
        let src = std::env::temp_dir().join("hulian-selftest-src.bin");
        let _ = std::fs::write(&src, &payload);
        let ids = shared.start_uploads(vec![src.to_string_lossy().to_string()]);
        let mut uploaded = None;
        for _ in 0..900 {
            if let Some(t) = shared.find_task(&ids[0]) {
                let s = t.snapshot();
                if s.state == "success" || s.state == "error" {
                    uploaded = Some(s);
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        match uploaded {
            Some(item) if item.state == "success" => {
                let remote_id = item.remote_file_id.unwrap_or_default();
                let dl_ids = shared.start_downloads(vec![(
                    remote_id,
                    "hulian-selftest-dst.bin".to_string(),
                    payload.len() as u64,
                )]);
                let mut done = None;
                for _ in 0..900 {
                    if let Some(t) = shared.find_task(&dl_ids[0]) {
                        let s = t.snapshot();
                        if s.state == "success" || s.state == "error" {
                            done = Some(s);
                            break;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                match done {
                    Some(dl) if dl.state == "success" => {
                        let ok = std::fs::read(&dl.local_path)
                            .map(|b| b == payload)
                            .unwrap_or(false);
                        println!(
                            "SELFTEST_UPLOAD_DOWNLOAD {}",
                            if ok { "OK" } else { "CONTENT_MISMATCH" }
                        );
                        let _ = std::fs::remove_file(&dl.local_path);
                    }
                    Some(dl) => println!("SELFTEST_DOWNLOAD_FAIL {}", dl.error.unwrap_or_default()),
                    None => println!("SELFTEST_DOWNLOAD_TIMEOUT"),
                }
            }
            Some(item) => println!("SELFTEST_UPLOAD_FAIL {}", item.error.unwrap_or_default()),
            None => println!("SELFTEST_UPLOAD_TIMEOUT"),
        }
        let _ = std::fs::remove_file(&src);
        app.exit(0);
    });
}
