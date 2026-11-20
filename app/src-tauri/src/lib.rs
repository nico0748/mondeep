//! Mondeep 学習アプリのバックエンド。画面は Tauri のコマンド経由で共通DB層を読む。
//!
//! - DB は OS 標準のアプリデータフォルダに置き、MCP サーバーと共有する（WAL モード）
//! - 起動時に自動バックアップ（直近 7 世代）
//! - 外部通信はしない。外部リンクは http(s) だけを OS のブラウザで開く
use std::path::PathBuf;
use std::sync::Mutex;

use base64::Engine;
use chrono::Local;
use mondeep_core::model::*;
use mondeep_core::store::HistoryGroup;
use mondeep_core::{backup, claude_config, paths, Store};
use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

struct AppState {
    store: Mutex<Store>,
    db_path: PathBuf,
    backup_error: Option<String>,
}

type CmdResult<T> = Result<T, String>;

fn with_store<T>(state: &State<AppState>, f: impl FnOnce(&Store) -> mondeep_core::Result<T>) -> CmdResult<T> {
    let store = state.store.lock().map_err(|_| "DB のロックに失敗しました".to_string())?;
    f(&store).map_err(|e| e.to_string())
}

fn today() -> chrono::NaiveDate {
    Local::now().date_naive()
}

/// MCP サーバーの実行ファイル。アプリと同じフォルダに置く前提（開発時は target/debug に並ぶ）。
fn mcp_binary_path() -> PathBuf {
    let name = if cfg!(windows) { "mondeep-mcp.exe" } else { "mondeep-mcp" };
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    db_path: String,
    backup_dir: String,
    backup_error: Option<String>,
    mcp_binary: String,
    mcp_binary_exists: bool,
    mcp_config_snippet: String,
    claude_config_path: Option<String>,
    today: String,
}

#[tauri::command]
fn app_info(state: State<AppState>) -> AppInfo {
    let bin = mcp_binary_path();
    let db = state.db_path.to_string_lossy().to_string();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        backup_dir: backup::backup_dir(&state.db_path).to_string_lossy().to_string(),
        backup_error: state.backup_error.clone(),
        mcp_binary_exists: bin.exists(),
        mcp_config_snippet: claude_config::snippet(&bin.to_string_lossy(), &db),
        mcp_binary: bin.to_string_lossy().to_string(),
        claude_config_path: paths::claude_desktop_config_path().map(|p| p.to_string_lossy().to_string()),
        db_path: db,
        today: today().to_string(),
    }
}

/// MCP サーバーが書き込むと変わる値。画面は 1 秒ごとにこれを見て、変わったら読み直す。
#[tauri::command]
fn data_version(state: State<AppState>) -> CmdResult<i64> {
    with_store(&state, |s| s.data_version())
}

#[tauri::command]
fn history(state: State<AppState>, filter: HistoryFilter) -> CmdResult<Vec<HistoryGroup>> {
    with_store(&state, |s| s.history(&filter))
}

#[tauri::command]
fn session_detail(state: State<AppState>, id: i64) -> CmdResult<SessionDetail> {
    with_store(&state, |s| s.session_detail(id))
}

#[tauri::command]
fn question_detail(state: State<AppState>, id: i64) -> CmdResult<QuestionDetail> {
    with_store(&state, |s| s.question_detail(id))
}

#[tauri::command]
fn list_terms(state: State<AppState>, tag: Option<String>, query: Option<String>) -> CmdResult<Vec<TermListItem>> {
    with_store(&state, |s| s.list_terms(tag.as_deref(), query.as_deref()))
}

#[tauri::command]
fn term_card(state: State<AppState>, id: i64) -> CmdResult<TermCard> {
    with_store(&state, |s| s.term_card(id))
}

#[tauri::command]
fn all_tags(state: State<AppState>) -> CmdResult<Vec<String>> {
    with_store(&state, |s| s.all_tags())
}

#[tauri::command]
fn review_queue(state: State<AppState>) -> CmdResult<Vec<ReviewItem>> {
    with_store(&state, |s| s.review_queue(200, &[], today()))
}

#[tauri::command]
fn weak_terms(state: State<AppState>) -> CmdResult<Vec<WeakTerm>> {
    with_store(&state, |s| s.weak_terms(100, &[]))
}

#[tauri::command]
fn search(state: State<AppState>, keyword: String) -> CmdResult<Vec<SearchHit>> {
    with_store(&state, |s| s.search(&keyword, 200))
}

#[tauri::command]
fn set_needs_review(state: State<AppState>, term_id: i64, on: bool) -> CmdResult<Term> {
    with_store(&state, |s| s.set_needs_review(term_id, on))
}

#[tauri::command]
fn set_term_tags(state: State<AppState>, term_id: i64, tags: Vec<String>) -> CmdResult<Term> {
    with_store(&state, |s| s.set_term_tags(term_id, &tags))
}

#[tauri::command]
fn diagram_adjustments(state: State<AppState>, term_id: i64) -> CmdResult<Vec<DiagramAdjustment>> {
    with_store(&state, |s| s.diagram_adjustments(term_id))
}

/// 図の部品をドラッグした結果を保存する（部品ごと）。
#[tauri::command]
fn set_diagram_adjustment(
    state: State<AppState>,
    term_id: i64,
    kind: String,
    element_id: String,
    dx: f64,
    dy: f64,
) -> CmdResult<()> {
    with_store(&state, |s| s.set_diagram_adjustment(term_id, &kind, &element_id, dx, dy))
}

#[tauri::command]
fn reset_diagram_adjustments(state: State<AppState>, term_id: i64, kind: String) -> CmdResult<usize> {
    with_store(&state, |s| s.reset_diagram_adjustments(term_id, &kind))
}

/// 削除はアプリ画面からのみ（MCP には公開しない）。
#[tauri::command]
fn delete_session(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_store(&state, |s| s.delete_session(id))
}

/// Claude Desktop の設定ファイルに Mondeep を追記する。元のファイルは .bak に残す。
#[tauri::command]
fn install_mcp_config(state: State<AppState>) -> CmdResult<String> {
    let path = paths::claude_desktop_config_path().ok_or("Claude Desktop の設定ファイルの場所が分かりません")?;
    let existing = std::fs::read_to_string(&path).ok();
    let merged = claude_config::merge(
        existing.as_deref(),
        &mcp_binary_path().to_string_lossy(),
        &state.db_path.to_string_lossy(),
    )
    .map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let Some(old) = &existing {
        std::fs::write(path.with_extension("json.bak"), old).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, merged).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// 外部リンクは http(s) だけを OS のブラウザで開く（アプリ内では開かない）。
#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> CmdResult<()> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err("http(s) のリンクだけ開けます".into());
    }
    app.opener().open_url(url.trim(), None::<&str>).map_err(|e| e.to_string())
}

/// 図の書き出し（SVG / PNG）。ダウンロードフォルダの Mondeep フォルダに保存し、保存先を返す。
#[tauri::command]
fn export_file(file_name: String, data_base64: String) -> CmdResult<String> {
    let ext_ok = file_name.ends_with(".svg") || file_name.ends_with(".png");
    let safe: String = file_name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect();
    if !ext_ok || safe.starts_with('.') || safe.contains("..") {
        return Err("書き出せるのは .svg と .png だけです".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(data_base64).map_err(|e| e.to_string())?;
    let dir = dirs::download_dir().or_else(dirs::home_dir).ok_or("保存先のフォルダが分かりません")?.join("Mondeep");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(safe);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

pub fn run() {
    let db_path = std::env::var_os("MONDEEP_DB").map(PathBuf::from).unwrap_or_else(paths::default_db_path);
    let store = Store::open(&db_path).unwrap_or_else(|e| panic!("DB を開けませんでした（{}）: {e}", db_path.display()));
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_error = backup::backup(store.conn(), &db_path, &stamp).err().map(|e| e.to_string());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { store: Mutex::new(store), db_path, backup_error })
        .invoke_handler(tauri::generate_handler![
            app_info,
            data_version,
            history,
            session_detail,
            question_detail,
            list_terms,
            term_card,
            all_tags,
            review_queue,
            weak_terms,
            search,
            set_needs_review,
            set_term_tags,
            diagram_adjustments,
            set_diagram_adjustment,
            reset_diagram_adjustments,
            delete_session,
            install_mcp_config,
            open_external,
            export_file,
        ])
        .run(tauri::generate_context!())
        .expect("Mondeep の起動に失敗しました");
}
