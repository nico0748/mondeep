//! Mondeep MCP サーバー。
//!
//! Claude Desktop が別プロセスとして起動し、標準入出力で MCP を話す。
//! 触れるのは `--db` で受け取った DB ファイルだけで、外部通信はしない。
//!
//! 使い方: `mondeep-mcp --db <DBファイル> [--knowledge-path <ナレッジDBのフォルダ>] [--obsidian-vault <Vault名>]`
use std::path::PathBuf;

use anyhow::Context;
use rmcp::ServiceExt;

mod guides;
mod server;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub db: PathBuf,
    pub knowledge_path: Option<String>,
    pub obsidian_vault: Option<String>,
}

fn parse_args(args: impl Iterator<Item = String>) -> anyhow::Result<Config> {
    let mut cfg = Config { db: mondeep_core::paths::default_db_path(), ..Default::default() };
    let mut it = args.skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => cfg.db = PathBuf::from(it.next().context("--db の後に DB ファイルのパスを書いてください")?),
            "--knowledge-path" => cfg.knowledge_path = Some(it.next().context("--knowledge-path の値がありません")?),
            "--obsidian-vault" => cfg.obsidian_vault = Some(it.next().context("--obsidian-vault の値がありません")?),
            "-h" | "--help" => {
                eprintln!("mondeep-mcp --db <DBファイル> [--knowledge-path <フォルダ>] [--obsidian-vault <Vault名>]");
                std::process::exit(0);
            }
            other => anyhow::bail!("不明な引数です: {other}"),
        }
    }
    Ok(cfg)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = parse_args(std::env::args())?;
    let store = mondeep_core::Store::open(&cfg.db)
        .with_context(|| format!("DB を開けませんでした: {}", cfg.db.display()))?;
    // 標準出力は MCP の通信に使うので、ログは標準エラーに出す
    eprintln!("mondeep-mcp: DB = {}", cfg.db.display());
    let service = server::MondeepServer::new(store, cfg).serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_args() {
        let c = parse_args(
            ["mondeep-mcp", "--db", "/tmp/x.db", "--knowledge-path", "/kb"].iter().map(|s| s.to_string()),
        )
        .unwrap();
        assert_eq!(c.db, PathBuf::from("/tmp/x.db"));
        assert_eq!(c.knowledge_path.as_deref(), Some("/kb"));
        assert!(parse_args(["m", "--evil"].iter().map(|s| s.to_string())).is_err());
    }
}
