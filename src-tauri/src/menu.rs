//! macOS 应用菜单：自定义「退出互联 / 关闭窗口」，让 Cmd+Q / Cmd+W 与托盘常驻语义一致。
//!
//! 为什么需要自定义菜单：Tauri 默认菜单的 Quit 会走 ExitRequested，
//! 而我们要在「关闭到托盘」模式下拦截退出；自定义菜单项直接走我们自己的退出逻辑，
//! 同时保留 Edit 菜单以保证输入框里 Cmd+C / Cmd+V 可用。

use crate::error::Result;
use crate::state::Shared;
use crate::window;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
pub fn create(app: &AppHandle) -> Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    let about = PredefinedMenuItem::about(app, Some("关于互联"), None)?;
    let quit = MenuItem::with_id(app, "app-quit", "退出互联", true, Some("CmdOrCtrl+Q"))?;
    let app_menu = Submenu::with_items(
        app,
        "互联",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let close = MenuItem::with_id(app, "app-close", "关闭窗口", true, Some("CmdOrCtrl+W"))?;
    let edit = Submenu::with_items(
        app,
        "编辑",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window_menu = Submenu::with_items(
        app,
        "窗口",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &close,
        ],
    )?;

    let menu = Menu::with_items(app, &[&app_menu, &edit, &window_menu])?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| match event.id().as_ref() {
        "app-quit" => {
            let shared = app.state::<Arc<Shared>>().inner().clone();
            window::quit_app(app, &shared);
        }
        "app-close" => close_window(app),
        _ => {}
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn close_window(app: &AppHandle) {
    let Some(shared) = app.try_state::<Arc<Shared>>() else {
        return;
    };
    let close_to_tray = shared.settings().close_to_tray && !shared.is_quitting();
    if close_to_tray {
        // 与点击窗口红灯一致：销毁窗口，驻留托盘
        window::destroy_main_window(app);
    } else {
        window::quit_app(app, &shared);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn create(_app: &AppHandle) -> Result<()> {
    Ok(())
}
