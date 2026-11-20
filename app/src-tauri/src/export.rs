//! 書き出し（図の SVG/PNG、Obsidian 用 Markdown）とバックアップのファイル操作。
//!
//! 画面から渡されたファイル名はそのまま使わず、1 段ずつ安全な名前に直してから、
//! ダウンロードフォルダの `Mondeep/` の下にだけ書き込む。
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Deserialize;

/// 1 回に書き出せるファイル数と、1 ファイルの上限（デコード後）。
pub const MAX_FILES: usize = 20_000;
pub const MAX_FILE_BYTES: usize = 20 * 1024 * 1024;
/// 1 回の書き出し全体の上限（デコード後）。
pub const MAX_TOTAL_BYTES: usize = 200 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct ExportFile {
    /// `メモ.md` や `図解/XSS-1.svg` のような相対パス（2 段まで）
    pub path: String,
    pub data_base64: String,
}

/// 書き出し先のルート（ダウンロードフォルダ、なければホームの下の `Mondeep`）。
pub fn export_root() -> Result<PathBuf, String> {
    Ok(dirs::download_dir().or_else(dirs::home_dir).ok_or("保存先のフォルダが分かりません")?.join("Mondeep"))
}

/// ファイル名・フォルダ名 1 段分を安全な名前にする。使えない名前なら None。
///
/// 文字・数字（日本語を含む）と `-_.() ` はそのまま、それ以外は `_` にする。
/// 先頭の `.`（隠しファイル・`..`）は認めない。
pub fn safe_segment(name: &str) -> Option<String> {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '(' | ')' | ' ') { c } else { '_' })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() || s.starts_with('.') || s.contains("..") || s.chars().count() > 120 {
        return None;
    }
    Some(s)
}

/// 相対パス（2 段まで）を安全なパスにする。拡張子は `allowed` のどれか。
pub fn safe_relative(path: &str, allowed: &[&str]) -> Option<PathBuf> {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let mut out = PathBuf::new();
    for p in &parts {
        out.push(safe_segment(p)?);
    }
    let ext = out.extension()?.to_str()?.to_ascii_lowercase();
    allowed.contains(&ext.as_str()).then_some(out)
}

/// `root/folder/` の下にファイルをまとめて書き出す（同名は上書き）。書き出したフォルダを返す。
pub fn write_files(root: &Path, folder: &str, files: &[ExportFile], allowed: &[&str]) -> Result<PathBuf, String> {
    write_files_limited(root, folder, files, allowed, MAX_TOTAL_BYTES)
}

fn write_files_limited(
    root: &Path,
    folder: &str,
    files: &[ExportFile],
    allowed: &[&str],
    max_total: usize,
) -> Result<PathBuf, String> {
    let folder = safe_segment(folder).ok_or("書き出し先のフォルダ名が不正です")?;
    if files.is_empty() {
        return Err("書き出すファイルがありません".into());
    }
    if files.len() > MAX_FILES {
        return Err(format!("一度に書き出せるのは {MAX_FILES} ファイルまでです"));
    }
    // 先にすべて検査してから書く（途中で止まって半端な書き出しにならないように）
    let mut prepared = Vec::with_capacity(files.len());
    let mut total = 0usize;
    for f in files {
        let rel = safe_relative(&f.path, allowed).ok_or_else(|| format!("書き出せないファイル名です: {}", f.path))?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(&f.data_base64).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(format!("{} が大きすぎます", f.path));
        }
        total += bytes.len();
        if total > max_total {
            return Err(format!("書き出すファイルが合計 {} MB を超えます", max_total / 1024 / 1024));
        }
        prepared.push((rel, bytes));
    }
    let dir = root.join(folder);
    for (rel, bytes) in prepared {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> ExportFile {
        ExportFile { path: path.into(), data_base64: base64::engine::general_purpose::STANDARD.encode(text) }
    }

    #[test]
    fn segments_keep_japanese_and_reject_traversal() {
        assert_eq!(safe_segment("クロスサイト スクリプティング (XSS).md").as_deref(), Some("クロスサイト スクリプティング (XSS).md"));
        assert_eq!(safe_segment("a/b:c*?.md").as_deref(), Some("a_b_c__.md"));
        assert_eq!(safe_segment(".."), None);
        assert_eq!(safe_segment(".hidden"), None);
        assert_eq!(safe_segment("   "), None);
        assert_eq!(safe_relative("図解/XSS-1.svg", &["svg"]), Some(PathBuf::from("図解").join("XSS-1.svg")));
        assert_eq!(safe_relative("../x.md", &["md"]), None);
        assert_eq!(safe_relative("a/b/c.md", &["md"]), None);
        assert_eq!(safe_relative("x.exe", &["md", "svg"]), None);
        assert_eq!(safe_relative("/etc/x.md", &["md"]), None);
    }

    #[test]
    fn writes_all_or_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let out = write_files(dir.path(), "Obsidian", &[file("XSS.md", "# XSS"), file("図解/XSS-1.svg", "<svg/>")], &["md", "svg"]).unwrap();
        assert_eq!(std::fs::read_to_string(out.join("XSS.md")).unwrap(), "# XSS");
        assert!(out.join("図解").join("XSS-1.svg").exists());
        // 1 つでも不正なら何も書かない
        let err = write_files(dir.path(), "Other", &[file("ok.md", "x"), file("../bad.md", "x")], &["md"]);
        assert!(err.is_err());
        assert!(!dir.path().join("Other").exists());
        assert!(write_files(dir.path(), "..", &[file("ok.md", "x")], &["md"]).is_err());
        // 合計サイズの上限を超えたら、何も書かない
        let big = write_files_limited(dir.path(), "Big", &[file("a.md", "12345"), file("b.md", "67890")], &["md"], 8);
        assert!(big.is_err());
        assert!(!dir.path().join("Big").exists());
    }
}
