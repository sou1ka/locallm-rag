//! rag-ui Tauri application entry point
//!
//! Initializes AppState and registers Tauri commands.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod state;

use state::{AppConfig, AppState, AppStateInner};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

fn main() {
    // config.toml のパスを決定
    // 実行ファイルと同じディレクトリか、カレントディレクトリから探す
    let config_path = find_config();

    // AppState 初期化
    let config = AppConfig::load(&config_path).unwrap_or_else(|e| {
        eprintln!("Failed to load config '{}': {}", config_path, e);
        std::process::exit(1);
    });

    let inner = AppStateInner::init(config).unwrap_or_else(|e| {
        eprintln!("Failed to initialize app state: {}", e);
        std::process::exit(1);
    });

    let app_state = AppState::new(inner);

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", "ウィンドウ表示"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "終了"));

    let tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(app_state)
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::DoubleClick { .. } => {
                show_window(app);
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => show_window(app),
                "quit" => {
                    flush_sessions(app);
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| {
            match event.event() {
                tauri::WindowEvent::CloseRequested { .. } => {
                    flush_sessions(&event.window().app_handle());
                }
                // Tauri v1 に Minimized バリアントはないため Resized で代用
                tauri::WindowEvent::Resized(_) => {
                    if event.window().is_minimized().unwrap_or(false) {
                        let _ = event.window().hide();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::ingest,
            commands::query,
            commands::chat,
            commands::list_sessions,
            commands::get_messages,
            commands::save_session,
            commands::save_all_sessions,
            commands::delete_session,
            commands::index_stats,
            commands::reset_index,
            commands::load_history,
            commands::load_conversation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// メモリ上の全セッションをファイルに書き出す
fn flush_sessions(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let inner = state.0.lock().unwrap();
    let history = match rag_core::history::HistoryManager::new(&inner.config.conversation.history_dir) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to init history manager: {}", e);
            return;
        }
    };
    for conv in inner.sessions.values() {
        if !conv.messages.is_empty() {
            if let Err(e) = history.save(conv) {
                eprintln!("Failed to save session {}: {}", conv.id, e);
            }
        }
    }
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// config.toml を探す
/// 優先順位: 環境変数 > 実行ファイルの隣 > カレントディレクトリ
fn find_config() -> String {
    if let Ok(path) = std::env::var("RAG_CONFIG") {
        return path;
    }

    // 実行ファイルの隣
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));
        let candidate = exe_dir.join("config.toml");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }

    // カレントディレクトリから親を遡って探す
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        let candidate = dir.join("config.toml");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        if !dir.pop() {
            break;
        }
    }

    "config.toml".to_string()
}
