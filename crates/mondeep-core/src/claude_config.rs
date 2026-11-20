//! Claude Desktop の設定ファイル（claude_desktop_config.json）への MCP 登録。
use serde_json::{json, Map, Value};

use crate::{Error, Result};

pub const SERVER_NAME: &str = "mondeep";

/// mcpServers に入れる Mondeep の登録内容。
pub fn server_entry(command: &str, db_path: &str) -> Value {
    json!({ "command": command, "args": ["--db", db_path] })
}

/// コピー用の設定（設定ファイル全体の形）。
pub fn snippet(command: &str, db_path: &str) -> String {
    let v = json!({ "mcpServers": { SERVER_NAME: server_entry(command, db_path) } });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// 既存の設定に Mondeep の登録を追記（または置き換え）する。他のサーバーや設定は残す。
pub fn merge(existing: Option<&str>, command: &str, db_path: &str) -> Result<String> {
    let mut root = match existing.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => serde_json::from_str::<Value>(s)
            .map_err(|e| Error::Invalid(format!("Claude Desktop の設定ファイルが JSON として読めません: {e}")))?,
        None => Value::Object(Map::new()),
    };
    let obj = root
        .as_object_mut()
        .ok_or_else(|| Error::Invalid("Claude Desktop の設定ファイルの中身がオブジェクトではありません".into()))?;
    let servers = obj.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let servers = servers
        .as_object_mut()
        .ok_or_else(|| Error::Invalid("mcpServers がオブジェクトではありません".into()))?;
    servers.insert(SERVER_NAME.into(), server_entry(command, db_path));
    Ok(serde_json::to_string_pretty(&root)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_without_losing_other_servers() {
        let existing = r#"{"mcpServers":{"obsidian":{"command":"npx","args":["obsidian-mcp"]}},"theme":"dark"}"#;
        let out: Value = serde_json::from_str(&merge(Some(existing), "/Apps/mondeep-mcp", "/data/m.db").unwrap()).unwrap();
        assert_eq!(out["theme"], "dark");
        assert_eq!(out["mcpServers"]["obsidian"]["command"], "npx");
        assert_eq!(out["mcpServers"]["mondeep"]["args"][1], "/data/m.db");
        let fresh: Value = serde_json::from_str(&merge(None, "c", "d").unwrap()).unwrap();
        assert_eq!(fresh["mcpServers"]["mondeep"]["command"], "c");
        assert!(merge(Some("[1]"), "c", "d").is_err());
        assert!(merge(Some("{oops"), "c", "d").is_err());
    }
}
