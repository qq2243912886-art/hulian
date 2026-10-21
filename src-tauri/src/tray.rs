//! 系统托盘：常驻后台，左键还原窗口，右键菜单（打开/快速上传/下载目录/退出）。

use crate::commands;
use crate::error::Result;
use crate::state::Shared;
use crate::window;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "hulian-tray";

pub fn create(app: &AppHandle) -> Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick-upload", "快速上传…", true, None::<&str>)?;
    let open_dl = MenuItem::with_id(app, "open-downloads", "打开下载文件夹", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出互联", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quick, &open_dl, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("互联 · 文件互传（手动上传/下载）")
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => {
            let _ = window::show_main_window(app);
        }
        "quick-upload" => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                commands::quick_upload(&app2).await;
            });
        }
        "open-downloads" => {
            use tauri_plugin_opener::OpenerExt;
            let shared = app.state::<Arc<Shared>>().inner().clone();
            let dir = shared.settings().download_dir;
            let _ = std::fs::create_dir_all(&dir);
            let _ = app.opener().open_path(dir, None::<&str>);
        }
        "quit" => {
            let shared = app.state::<Arc<Shared>>().inner().clone();
            window::quit_app(app, &shared);
        }
        _ => {}
    }
}
