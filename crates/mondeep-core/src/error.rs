use serde::Serialize;

/// 共通DB層のエラー。MCP サーバーはこのメッセージをそのまま Claude に返すので、
/// Claude が読んで直せる日本語の説明にしておく。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("データベースエラー: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON エラー: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ファイル操作エラー: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} が見つかりません")]
    NotFound(String),
    #[error("入力が不正です: {0}")]
    Invalid(String),
    /// 用語の同一判定で迷ったときに、候補を Claude に返して選ばせる。
    #[error("用語「{input}」に近い既存の用語があります。term_id で既存の用語を指定するか、create_new_term=true で新しい用語として登録してください")]
    AmbiguousTerm {
        input: String,
        candidates: Vec<TermCandidate>,
    },
    /// 図解データが規約に違反している。
    #[error("図解データが規約に違反しています: {}", .0.join(" / "))]
    Diagram(Vec<String>),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TermCandidate {
    pub term_id: i64,
    pub headword: String,
    pub aliases: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Error>;
