//! Mondeep 共通DB層。
//!
//! 学習アプリ（Tauri）と MCP サーバーの両方が、このクレートを通して同じ SQLite ファイルを読み書きする。
//! 書き込みはすべてパラメータ化クエリで行い、SQLite は WAL モードで開く。

pub mod backup;
pub mod claude_config;
pub mod db;
pub mod diagram;
pub mod error;
pub mod model;
pub mod normalize;
pub mod paths;
pub mod srs;
pub mod store;

pub use error::{Error, Result};
pub use store::Store;
