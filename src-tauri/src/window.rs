//! 主窗口生命周期：关闭到托盘时销毁 WebView（空闲近乎零占用），托盘点击时按需重建。
//!
//! 桌面端：主窗口不在启动时创建（见 `tauri.{macos,windows}.conf.json` 的 `create: false`），
//! 由这里按需创建/销毁。移动端：Tauri 自己管理单窗口，这里的窗口操作是空实现。

use crate::error::Result;
use crate::state::Shared;
use tauri::AppHandle;

pub const MAIN_LABEL: &str = "main";

#[cfg(target_os = "macos")]
fn set_policy(app: &AppHandle, regular: bool) {
    use tauri::ActivationPolicy;
    let _ = app.set_activation_policy(if regular {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    });
}

#[cfg(desktop)]
pub fn build_main_window(app: &AppHandle, visible: bool) -> Result<()> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    let builder = WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::App("index.html".into()))
        .title("互联")
        .inner_size(780.0, 640.0)
        .min_inner_size(420.0, 520.0)
        .visible(visible)
        .theme(Some(tauri::Theme::Dark));
    #[cfg(desktop)]
    let builder = builder.center();
    let win = builder.build()?;
    if visible {
        let _ = win.set_focus();
    }
    Ok(())
}

#[cfg(mobile)]
pub fn build_main_window(_app: &AppHandle, _visible: bool) -> Result<()> {
    // 移动端由 Tauri 创建唯一的 WebView，不需要手动建窗口
    Ok(())
}

pub fn show_main_window(app: &AppHandle) -> Result<()> {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        #[cfg(target_os = "macos")]
        set_policy(app, true);
        if let Some(win) = app.get_webview_window(MAIN_LABEL) {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
            Ok(())
        } else {
            build_main_window(app, true)
        }
    }
    #[cfg(mobile)]
    {
        let _ = app;
        Ok(())
    }
}

/// 隐藏到托盘 = 直接销毁窗口，真正释放 WebView 内存（移动端无托盘，直接无操作）
pub fn hide_main_window(app: &AppHandle) {
    #[cfg(desktop)]
    destroy_main_window(app);
    #[cfg(mobile)]
    {
        let _ = app;
    }
}

pub fn destroy_main_window(app: &AppHandle) {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        match app.get_webview_window(MAIN_LABEL) {
            Some(win) => {
                eprintln!("[hulian] destroy_main_window: 找到窗口，执行 destroy");
                match win.destroy() {
                    Ok(()) => eprintln!("[hulian] destroy_main_window: destroy ok"),
                    Err(e) => eprintln!("[hulian] destroy_main_window: destroy 失败: {e}"),
                }
            }
            None => eprintln!("[hulian] destroy_main_window: 当前没有主窗口"),
        }
    }
    #[cfg(target_os = "macos")]
    set_policy(app, false);
    #[cfg(mobile)]
    {
        let _ = app;
    }
}

#[cfg(desktop)]
pub fn toggle_main_window(app: &AppHandle) {
    use tauri::Manager;
    match app.get_webview_window(MAIN_LABEL) {
        Some(win) => {
            if win.is_visible().unwrap_or(false) {
                destroy_main_window(app);
            } else {
                let _ = show_main_window(app);
            }
        }
        None => {
            let _ = show_main_window(app);
        }
    }
}

#[cfg(mobile)]
pub fn toggle_main_window(_app: &AppHandle) {}

pub fn quit_app(app: &AppHandle, shared: &Shared) {
    shared.mark_quitting();
    app.exit(0);
}
