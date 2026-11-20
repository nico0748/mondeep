//! OS ごとのパス。DB は OS 標準のアプリデータフォルダに置く。
use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "Mondeep";
pub const DB_FILE_NAME: &str = "mondeep.db";

/// 既定の DB ファイルの場所（macOS: ~/Library/Application Support/Mondeep/mondeep.db、
/// Windows: %APPDATA%\Mondeep\mondeep.db、Linux: ~/.local/share/Mondeep/mondeep.db）。
pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
        .join(DB_FILE_NAME)
}

/// Claude Desktop の設定ファイルの場所。OS ごとに切り替える。
pub fn claude_desktop_config_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude_desktop_config.json")
        })
    } else if cfg!(target_os = "windows") {
        dirs::config_dir().map(|c| c.join("Claude").join("claude_desktop_config.json"))
    } else {
        dirs::config_dir().map(|c| c.join("Claude").join("claude_desktop_config.json"))
    }
}
