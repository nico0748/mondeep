// 画面から呼べる操作（コマンド）をここで列挙し、capabilities/default.json で許可したものだけを使えるようにする。
const COMMANDS: &[&str] = &[
    "app_info",
    "data_version",
    "history",
    "session_detail",
    "question_detail",
    "list_terms",
    "term_card",
    "all_tags",
    "review_queue",
    "weak_terms",
    "search",
    "set_needs_review",
    "set_term_tags",
    "diagram_adjustments",
    "set_diagram_adjustment",
    "reset_diagram_adjustments",
    "delete_session",
    "install_mcp_config",
    "open_external",
    "export_file",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
