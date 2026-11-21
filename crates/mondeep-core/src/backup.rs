//! 起動時の自動バックアップ（直近 7 世代）。
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::Result;

pub const KEEP_GENERATIONS: usize = 7;

/// `backups/` フォルダに `VACUUM INTO` で一貫したコピーを取り、古いものを消して 7 世代に保つ。
pub fn backup(conn: &Connection, db_path: &Path, stamp: &str) -> Result<PathBuf> {
    let dir = backup_dir(db_path);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join(format!("mondeep-{stamp}.db"));
    if target.exists() {
        std::fs::remove_file(&target)?;
    }
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
    prune(&dir, KEEP_GENERATIONS)?;
    Ok(target)
}

/// 指定したファイルに `VACUUM INTO` で一貫したコピーを取る（手動バックアップ、F-13）。既にあれば失敗する。
pub fn backup_to(conn: &Connection, target: &Path) -> Result<()> {
    if target.exists() {
        return Err(crate::Error::Invalid(format!("{} はすでにあります", target.display())));
    }
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
    Ok(())
}

pub fn backup_dir(db_path: &Path) -> PathBuf {
    db_path.parent().unwrap_or(Path::new(".")).join("backups")
}

/// 名前（日時入り）の新しい順に `keep` 個だけ残す。
pub fn prune(dir: &Path, keep: usize) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("mondeep-") && n.ends_with(".db"))
        })
        .collect();
    files.sort();
    let mut removed = Vec::new();
    while files.len() > keep {
        let f = files.remove(0);
        std::fs::remove_file(&f)?;
        removed.push(f);
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_seven_generations() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mondeep.db");
        let conn = crate::db::open(&db).unwrap();
        for i in 0..10 {
            backup(&conn, &db, &format!("20261120-0000{i:02}")).unwrap();
        }
        let mut names: Vec<_> = std::fs::read_dir(backup_dir(&db))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names.len(), KEEP_GENERATIONS);
        assert_eq!(names[0], "mondeep-20261120-000003.db");
        // バックアップは DB として開ける
        let b = Connection::open(backup_dir(&db).join(&names[6])).unwrap();
        let v: i64 = b.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, crate::db::MIGRATIONS.len() as i64);
    }

    #[test]
    fn manual_backup_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::open(&dir.path().join("mondeep.db")).unwrap();
        let target = dir.path().join("out").join("copy.db");
        backup_to(&conn, &target).unwrap();
        assert!(Connection::open(&target).is_ok());
        assert!(backup_to(&conn, &target).is_err());
    }
}
